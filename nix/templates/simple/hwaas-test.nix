# This attribute set defines a HWaaS test. It may contain all the following options/attributes:
#
# - NixOS test attributes, see <https://nixos.org/manual/nixos/unstable/#sec-test-options-reference>
{ user-tooling-hwaasTestVm }:
{
  name = "My first HWaaS Test";

  # === START OF SPECIAL HWAAS TEST ATTRIBUTES ===
  apiUrl = "https://api.hwaas.changeme.example/v5/";
  machines = {
    dummy1 = {
      platform = "test-machine-x86_64";
      image = "my-image.iso";
    };
  };
  nodes.controlVM = { pkgs, ... }: {
    imports = [ user-tooling-hwaasTestVm ];

    hwaas.testVm.enable = true;
  };
  # === END OF SPECIAL HWAAS TEST ATTRIBUTES ===

  testScript = ''
    import sys
    sys.exit(1)
  '';
}
