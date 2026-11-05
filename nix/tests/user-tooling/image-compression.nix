# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

# Checks that the HWaaS user tooling is able to bring up a machine from both an uncompressed and a
# zstd compressed image.
{
  testers,
  lib,
  writeText,
  debugging ? false,
  context-api-url-version-prefix,
  modules,
  hwaasPythonDriver,
}:
let
  serverPort = 8080;
  apiUrl = "http://gateway:${toString serverPort}/${context-api-url-version-prefix}";

  bootScript = writeText "boot-hwaas-machine.py" ''
    import sys

    from hwaas_driver import Hwaas

    def main(image_path):
        hwaas = Hwaas(
            {
                "apiUrl": "${apiUrl}",
                "machines": {"abmr": {"platform": "", "image": image_path}},
            }
        )
        machine = hwaas.machines["abmr"]
        assert len(machine.drives) == 1, f"expected exactly one drive, got {machine.drives}"
        machine.power_on()
        print(hwaas.context)
        # Leaving the scope drops the last reference to `hwaas`, which deletes the context.

    main(sys.argv[1])
  '';
in
testers.runNixOSTest {
  name = "user-tooling-image-compression";
  node.specialArgs = { inherit modules; };
  nodes = {
    gateway = _: {
      imports = [
        ../contextapi/test-modules/test-config.nix
        ../contextapi/test-modules/mock-contextapi-satellite-rest-services.nix
        ../contextapi/test-modules/mock-remote-usb.nix
        modules.contextapi-module
      ]
      ++ lib.optionals debugging [ ../contextapi/test-modules/debugging.nix ];

      context-api-test-config.enable = true;

      services = {
        mock-remote-usb.enable = true;
        mock-contextapi-satellite-rest-services.enable = true;
        contextApi = {
          enable = true;
          openFirewall = true;
          port = serverPort;
        };
      };
    };

    client = { pkgs, ... }: {
      environment.systemPackages = [
        pkgs.zstd
        (pkgs.python3.withPackages (_: [ hwaasPythonDriver ]))
      ];
    };
  };

  testScript = ''
    start_all()
    gateway.wait_for_unit("context-api.service")
    gateway.wait_for_open_port(${toString serverPort})
    client.wait_for_unit("multi-user.target")

    images_url = "localhost:${toString serverPort}/${context-api-url-version-prefix}/images"

    def create_image(name):
      """Create a random raw image on the client and return its path and checksum."""
      path = f"/tmp/{name}.img"
      client.succeed(f"dd bs=1M count=16 if=/dev/urandom of={path}")
      return path, client.succeed(f"sha256sum {path}").split()[0]

    def usb_attach_count():
      return int(gateway.succeed(
        "journalctl --no-pager -u mock-remote-usb.service | grep -c \"HTTP PUT '/usb'\" || true"
      ))

    def boot(image_path, raw_sha256):
      """Boot the machine from the given image and check the contextapi received the raw image."""

      attaches_before = usb_attach_count()

      context = client.succeed(f"python3 ${bootScript} {image_path}").strip()
      gateway.succeed(f"curl --fail-with-body --silent {images_url}/{raw_sha256}")
      assert usb_attach_count() > attaches_before, "expected the image to be attached via USB"

      # The context must be released again, so the next boot can reserve the machine
      gateway.wait_until_fails(
        f"curl --fail --silent localhost:${toString serverPort}/${context-api-url-version-prefix}/contexts/{context}"
      )

    with subtest("Boot from uncompressed image"):
      path, sha256 = create_image("raw")
      boot(path, sha256)

    # NOTE(Luke): if these tests take too long, we could just inspect and ensure that the
    # API request is the same for both images, as they take the same path either way.
    for extension in ["zst", "zstd"]:
      with subtest(f"Boot from zstd compressed image with .{extension} extension"):
        path, sha256 = create_image(f"compressed-{extension}")
        compressed_path = f"{path}.{extension}"
        client.succeed(f"zstd -T0 {path} -o {compressed_path}")
        boot(compressed_path, sha256)
        # The API must not have stored the compressed byte stream as an image
        compressed_sha256 = client.succeed(f"sha256sum {compressed_path}").split()[0]
        gateway.fail(f"curl --fail --silent {images_url}/{compressed_sha256}")
  '';
}
