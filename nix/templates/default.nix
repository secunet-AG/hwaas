{ ... }: {
  flake.templates = {
    simple = {
      path = ./simple;
      description = "A simple template to get started with HWaaS";
    };
    network-dhcp = {
      path = ./network-dhcp;
      description = "An advanced template for a HWaaS test with local networking via DHCP";
    };
  };
}
