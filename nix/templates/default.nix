{ ... }: {
  flake.templates = {
    simple = {
      path = ./simple;
      description = "A simple template to get started with HWaaS";
    };
  };
}
