# This attribute set defines a HWaaS test. It may contain all the following options/attributes:
#
# - NixOS test attributes, see <https://nixos.org/manual/nixos/unstable/#sec-test-options-reference>
#
# The `testScript` can use the following Python APIs:
#
# - NixOS Python methods, see <https://nixos.org/manual/nixos/unstable/#ssec-machine-objects>
{
  user-tooling-hwaasTestVm,
  user-tooling-machines-legacyBox,
  pkgs,
}:
let
  # === START OF USER CONFIGURATION ===
  user-config = {
    # NOTE: You MUST adapt these settings to your HWaaS environment
    api-url = "https://api.hwaas.changeme.example/v5";
    bmr-platform = "test-machine-x86_64";
    # NOTE: You MAY change these settings to whatever you wish
    bmr-name = "dummy1";
  };
  # === END OF USER CONFIGURATION ===

  # Disk image booted on the BMR
  disk_image =
    (pkgs.nixos (
      { ... }: {
        imports = [ user-tooling-machines-legacyBox ];
      }
    )).isoImage;
in
{
  name = "My first test with HWaaS";

  # === START OF SPECIAL HWAAS TEST ATTRIBUTES ===
  apiUrl = user-config.api-url;
  machines = {
    "${user-config.bmr-name}" = {
      platform = user-config.bmr-platform;
      image = "${disk_image}/iso/hwaas.iso";
    };
  };
  networks = { "undefined" = []; };

  nodes.controlVM = { ... }: {
    imports = [ user-tooling-hwaasTestVm ];

    hwaas.testVm.enable = true;
  };
  # === END OF SPECIAL HWAAS TEST ATTRIBUTES ===

  testScript = ''
    controlVM.start()
    controlVM.wait_for_unit("multi-user.target")

    hwaas.machines["${user-config.bmr-name}"].power_on()
    print("Machine powered on!")
    hwaas.machines["${user-config.bmr-name}"].power_off()
    print("Machine powered off!")
  '';
}
