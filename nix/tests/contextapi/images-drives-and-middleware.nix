# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{
  testers,
  lib,
  debugging ? false,
  context-api-url-version-prefix,
  modules,
  writeText,
}:
let
  store = "/run/context-api/images";
  serverPort = 8080;
  rsd = import ./rsd.nix;
  rhPort = "12345";
in
testers.runNixOSTest {
  name = "image-api-test";
  node.specialArgs = { inherit modules; };
  nodes = {
    gateway = { ... }: {
      imports = [
        ./test-modules/test-config.nix
        ./test-modules/mock-contextapi-satellite-rest-services.nix
        ./test-modules/mock-remote-usb.nix
        modules.contextapi-module
        modules.remote-serial
        modules.remote-power
        modules.remote-usb
        modules.remote-auxiliary
      ]
      ++ lib.optionals debugging [ ./test-modules/debugging.nix ];

      context-api-test-config = {
        enable = true;
        inherit store;
      };

      virtualisation = {
        # Use 4 GiB memory.
        # Needed because we create some fake images to test the upload
        memorySize = 4096;
      };

      services = {
        mock-remote-usb.enable = true;
        mock-contextapi-satellite-rest-services.enable = true;
        http-echo-server.bodyOnly = true;
        contextApi = {
          enable = true;
          openFirewall = true;
          port = serverPort;
        };
        remote-auxiliary = {
          enable = true;
          port = lib.toInt rhPort;
          configFile = toString (writeText "remote-auxiliary.json" (builtins.toJSON { devices = { }; }));
        };
      };

    };
  };

  testScript = ''
    import json
    from time import sleep

    start_all()
    gateway.wait_for_open_port(${builtins.toString serverPort})

    # Create regular image
    imgName="myImageName"
    genImgPath = f"/tmp/{imgName}"
    gateway.succeed(f"dd bs=1024 count=104800 < /dev/urandom > {genImgPath}")
    sha_sum = gateway.succeed(f"sha256sum {genImgPath}").split()[0]
    # Create zstd image
    genImgPathZstd = f"{genImgPath}.zstd"
    gateway.succeed(f"zstd -T0 {genImgPath} -o {genImgPathZstd}")
    sha_sum_zstd = gateway.succeed(f"sha256sum {genImgPathZstd}").split()[0]
    # More test data
    wrong_hash = gateway.succeed("sha256sum <<< foobar").split()[0]
    base_url = "localhost:${builtins.toString serverPort}/${context-api-url-version-prefix}"

    rsd_object_string = json.dumps(${rsd})
    context_uuid = gateway.succeed(f"curl --fail --silent -X POST -H 'Content-Type: application/json' --data '{rsd_object_string}' {base_url}/contexts")
    base_url_ctx = f"{base_url}/contexts/{context_uuid}"
    drive_name = "myDrive"
    machine_name = "abmr" # default from context-api-test-config

    # Helper function to check the number of available images
    def assert_image_count_is(count):
      resp = gateway.succeed(f"curl --fail-with-body --silent -X 'GET' {base_url}/images")
      dict = json.loads(resp)
      assert len(dict) == count, f"Expected {count} images but found {len(dict)}"

    # Helper function to upload a single image
    def upload_image(img_path, should_fail=False, zstd=False):
      url = f"{base_url}/images"
      if zstd:
        url = f"{url}?compression=zstd"

      cmd = f"curl --fail-with-body --silent -X POST -w '%{{http_code}}' -F upload=@{img_path} {url}"
      response = gateway.fail(cmd) if should_fail else gateway.succeed(cmd)
      # last 3 characters are HTTP status code
      return {"body": response[:-3], "status": int(response[-3:])}

    # should store the image
    with subtest("POST image"):
      assert_image_count_is(0)
      response = upload_image(genImgPath)
      assert_image_count_is(1)
      body = json.loads(response["body"])
      assert body["sha256"] == sha_sum, f"Expected sha256sum {sha_sum} but response was {body["sha256"]}"
      assert body["file_name"] == "myImageName", f"Expected filename 'myImageName' but response was {body["file_name"]}"

    # should be idempotent and not create an image duplicate
    with subtest("Upload image a second time"):
      upload_image(genImgPath)
      assert_image_count_is(1)

    # Should decompress to the same image as before, so the same condition as above holds
    # NOTE: The image name is updated, though!
    with subtest("Upload image a third time as zstd"):
      upload_image(genImgPathZstd, zstd = True)
      assert_image_count_is(1)
      assert sha_sum != sha_sum_zstd, "raw and zstd image must have different byte stream checksums"

    with subtest("Returns 404 when requesting non existent image"):
      responseCode = gateway.succeed(f"curl -s -o /dev/null -w %{{http_code}} {base_url}/images/{wrong_hash}")
      assert int(responseCode) == 404, f"Expected 404 but was {responseCode}"

    with subtest("400 when specifying invalid hash"):
      responseCode = gateway.succeed(f"curl -s -o /dev/null -w %{{http_code}} {base_url}/images/foobar")
      assert int(responseCode) == 400, f"Expected 400 but was {responseCode}"

    # should fail when uploading an image that exceeds the maximum allowed image size
    with subtest("Upload limit"):
      gateway.succeed(f"dd bs=2048 count=104800 < /dev/urandom > {genImgPath}")
      response = upload_image(genImgPath, should_fail=True)
      # 413 = Payload Too Large
      assert response["status"] == 413
      # The image must not be in the store afterwards
      assert_image_count_is(1)

    # Test if 'get_image' returns the expected meta data
    with subtest("Get single image"):
      response = gateway.succeed(f"curl --fail-with-body --silent {base_url}/images/{sha_sum}")
      response_json = json.loads(response)
      assert response_json["file_name"] == imgName + ".zstd", f"response had unexpected file name: {response_json}"

    # Test if 'get_images' returns the right amount of images and sizes
    with subtest("Get images"):
      response = gateway.succeed(f"curl --fail-with-body --silent {base_url}/images")
      response_json = json.loads(response)
      assert len(response_json) == 1
      assert response_json[0]["size_bytes"] == 1024 * 104800

    # Test if a drive can be generated from a uploaded image
    with subtest("Create drive"):
      # check if drives are empty
      response = gateway.succeed(f"curl --fail-with-body --silent {base_url_ctx}/drives")
      assert json.loads(response) == [], "expect drives to be a empty list"

      # create a drive
      gateway.succeed(f"curl --fail-with-body --silent -X PUT {base_url_ctx}/drives/{drive_name}?image_hash={sha_sum}")

      # test if our drive exists afterwards
      response = gateway.succeed(f"curl --fail-with-body --silent {base_url_ctx}/drives")
      assert drive_name in json.loads(response), "expect the new drive exist"

    with subtest("drive handler"):
      # use the drive create in the last step for configuration
      usb_conf = [{'type': 'storage', 'luns': [{'path': drive_name}]}]
      usb_functions = json.dumps(usb_conf)

      # test the drive handler by calling the usb functions endpoint
      response = gateway.succeed(f"curl --fail-with-body --silent -X PUT {base_url_ctx}/machines/{machine_name}/usb -H 'Content-Type: application/json' -d '{usb_functions}'")

      # Drive name should not appear at the mock-remote-usb.
      # Otherwise the middleware was not activated.
      gateway.fail(f"systemctl status --full --no-pager echo-server.service | grep {drive_name}")

    with subtest("delete drive"):
      gateway.succeed(f"curl --fail-with-body --silent -X DELETE {base_url_ctx}/drives/{drive_name}")
      response = gateway.succeed(f"curl --fail-with-body --silent {base_url_ctx}/drives")
      assert json.loads(response) == [], "expect drives to be a empty list"

    with subtest("drive cleanup on context deletion"):
      def get_num_drives_fs():
        return int(gateway.succeed("ls -1 ${store}/drives | wc -l"))

      # create exactly one drive for the current reserved context
      gateway.succeed(f"curl --fail-with-body --silent -X PUT {base_url_ctx}/drives/{drive_name}?image_hash={sha_sum}")
      assert get_num_drives_fs() == 1, "expected only one drive to be present"

      # free the context reservation
      gateway.succeed(f"curl --fail-with-body --silent -X DELETE {base_url_ctx}")
      sleep(5)
      assert get_num_drives_fs() == 0, "drive should be gone as the context was terminated"
  '';
}
