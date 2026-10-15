# This attribute set defines a HWaaS test. It may contain all the following options/attributes:
#
# - NixOS test attributes, see <https://nixos.org/manual/nixos/unstable/#sec-test-options-reference>
{
  user-tooling-hwaasTestVm,
  user-tooling-machines-legacyBox,
  pkgs,
}:
let
  machine_image =
    (pkgs.nixos (
      { ... }: {
        imports = [ user-tooling-machines-legacyBox ];
      }
    )).isoImage;
in
{
  name = "My first HWaaS Test";

  # === START OF SPECIAL HWAAS TEST ATTRIBUTES ===
  apiUrl = "https://api.hwaas.changeme.example/v5/";
  machines = {
    dummy1 = {
      platform = "test-machine-x86_64";
      image = "${machine_image}/iso/hwaas.iso";
    };
  };
  nodes.controlVM = { ... }: {
    imports = [ user-tooling-hwaasTestVm ];

    hwaas.testVm.enable = true;
  };
  # === END OF SPECIAL HWAAS TEST ATTRIBUTES ===

  testScript = ''
    controlVM.start()
    controlVM.wait_for_unit("default.target")
    import sys
    sys.exit(1)
  '';
}
