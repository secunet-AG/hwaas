# SPDX-FileCopyrightText: Copyright 2026 secunet Security Networks AG <https://www.secunet.com>
#
# SPDX-License-Identifier: Apache-2.0

{ testers
, lib
, httpie
, context-api-url-version-prefix
, modules
, writeText
, writeShellScript
, curl
, writePython3
, debugging ? false
,
}:
let
  rsd = import ./rsd.nix;
  ctxPort = "8080";
  rhPort = "12345";
  uploadAuxId = "upload-auxiliary";
  uploadAuxPort = "22222";
  uploadserverDir = "/srv/images";
  statusAuxId = "http-sim";
  statusAuxPort = "5042";
in
testers.runNixOSTest {
  name = "remote-hands-aux-device-test";
  node.specialArgs = { inherit modules; };
  nodes = {
    sut =
      { config, ... }:
      {
        imports = [
          modules.contextapi-module
          modules.test-http-sim
          modules.remote-serial
          modules.remote-power
          modules.remote-usb
          modules.remote-auxiliary
          ./test-modules/test-config.nix
          ./test-modules/mock-contextapi-satellite-rest-services.nix
          ./test-modules/mock-remote-usb.nix
        ]
        ++ lib.optionals debugging [
          ./test-modules/debugging.nix
        ];

        environment.systemPackages = [
          httpie
          curl
        ];

        services.contextApi = {
          enable = true;
          openFirewall = true;
          port = lib.toInt ctxPort;
          config = {
            image_api_settings = {
              max_file_size = "20GiB";
              store = "/tmp";
            };
            db_file_path = "/run/context-api/db.sqlite";
            net_ctrl_base_path = "foo";
            network_gateway.ws_gateway_url = "bar";

            request_timeouts = {
              # disable ImageAPI request timeouts
              image_api = null;
              single_context_api = 30000;
            };
          };
        };

        services = {
          mock-remote-usb.enable = true;
          mock-contextapi-satellite-rest-services.enable = true;
          remote-auxiliary = {
            enable = true;
            port = lib.toInt rhPort;
            configFile = toString (
              writeText "remote-auxiliary.json" (
                builtins.toJSON {
                  devices = {
                    ${uploadAuxId}.config = {
                      id = uploadAuxId;
                      url = "http://127.0.0.1:${uploadAuxPort}";
                      cmd = writeShellScript "upload-aux-cmd-script" ":";
                    };
                    ${statusAuxId}.config = {
                      id = statusAuxId;
                      url = "http://127.0.0.1:${statusAuxPort}";
                      cmd = writeShellScript "status-aux-cmd-script" ":";
                    };
                  };
                }
              )
            );
          };
          maintainerCli = {
            enable = true;
            configMachines = [
              {
                id = 1;
                switch_connections = { };
                remote_auxiliary = "http://127.0.0.1:${rhPort}/auxiliaries";
                remote_power = "http://127.0.0.1:${builtins.toString config.services.mock-contextapi-satellite-rest-services.port}/power";
                remote_serial = null;
                remote_usb = "http://127.0.0.1:${builtins.toString config.services.mock-remote-usb.port}/usb";
                platform = "";
              }
            ];
            configNetworks = lib.lists.range 1 2;
          };
        };

        systemd.services = {
          upload-server = {
            description = "HTTP server";
            wantedBy = [ "multi-user.target" ];
            after = [ "network-online.target" ];
            wants = [ "network-online.target" ];
            serviceConfig = {
              ExecStart = writePython3 "upload-server" { } ''
                import http.server
                import os


                class UploadHandler(http.server.SimpleHTTPRequestHandler):
                    def do_PUT(self):
                        # Check for the 64MB limit (64 * 1024 * 1024)
                        content_length = int(self.headers.get('Content-Length', 0))
                        if content_length > 67108864:
                            self.send_error(413, "Payload Too Large")
                            return
                        path = self.translate_path(self.path)
                        os.makedirs(os.path.dirname(path), exist_ok=True)
                        with open(path, 'wb') as f:
                            f.write(self.rfile.read(content_length))
                        self.send_response(201)
                        self.end_headers()


                http.server.test(HandlerClass=UploadHandler, port=${uploadAuxPort})
              '';
              WorkingDirectory = uploadserverDir;
            };
          };
          maintainer-cli-init-service = {
            before = [ "context-api.service" ];
            # Needed to simulate the power device as the maintainer cli performs a power reset
            after = [ "echo-server.service" ];
          };
        };

        systemd.tmpfiles.rules = [ "d ${uploadserverDir} 0755 root wheel" ];
      };
  };

  testScript = ''
    import json
    start_all()
    sut.wait_for_open_port(${uploadAuxPort})
    sut.wait_for_open_port(${rhPort})
    sut.wait_for_open_port(${ctxPort})

    base_url = "localhost:${ctxPort}/${context-api-url-version-prefix}/contexts"
    rsd_object_string = json.dumps(${rsd})
    context_uuid = sut.succeed(f"http --check-status POST http://{base_url} <<<'{rsd_object_string}'")
    auxiliaries_address = f"http://{base_url}/{context_uuid}/machines/abmr/auxiliaries"

    def calc_hash(file):
      sut.succeed(f"sha256sum {file} | awk '{{print $1}}'")

    with subtest("upload test"):
      payload1 = "/tmp/payload1"
      # Limit is 64 MB including HTTP framing overhead
      sut.succeed(f"dd if=/dev/random of={payload1} bs=1M count=63")
      # by default curl sends a `Expect: 100-continue` header.
      # As the upload server does not support this, we remove the header.
      sut.succeed(f"curl -H Expect: --fail-with-body -F files=@{payload1} -X PUT {auxiliaries_address}/${uploadAuxId}/api/upload/payload1")
      assert calc_hash(payload1) == calc_hash("${uploadserverDir}/upload/payload1"), "uploaded payload1 differs from original"

      payload2 = "/tmp/payload2"
      sut.succeed(f"dd if=/dev/random of={payload2} bs=1M count=100")
      res = sut.fail(f"http --check-status --multipart -v PUT {auxiliaries_address}/${uploadAuxId}/api/upload/payload2 files@{payload2} 2>&1")
      assert "HTTP 413 Payload Too Large" in res, "wrong return code"

    with subtest("download test"):
      sut.succeed(f"http --download {auxiliaries_address}/${uploadAuxId}/api/upload/payload1")
      assert calc_hash(payload1) == calc_hash("./payload1"), "downloaded payload1 differs from original"

    with subtest("response header"):
      res = sut.succeed(f"http -h POST {auxiliaries_address}/${statusAuxId}/api/craft <<<'{{\"media_type\": \"image/png\", \"headers\": {{\"X-Foo\": \"Bar\"}} }}'")
      assert "content-type: image/png" in res, "content type mismatch"
      assert "x-foo: Bar" in res, "custom header not found"

    with subtest("error code"):
      sut.wait_for_open_port(${statusAuxPort})
      for code in [ 200, 201, 400, 404, 500 ]:
        res_code = sut.succeed(f"http -h {auxiliaries_address}/${statusAuxId}/api/{code} | grep 'HTTP/' | cut -d ' ' -f 2")
        assert code == int(res_code), f"HTTP status code mismatch: expected {code} got {res_code}"

  '';
}
