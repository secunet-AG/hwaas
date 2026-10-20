{
  perSystem = { ... }: {
    templates = {
      # TODO(hartan): We need a "reference HWaaS setup" that maybe includes only a single x86_64
      # machine for starters. We could tailor all examples and templates here to work with this
      # "reference" HWaaS. The x86_64 machine could get a platform like "test-machine-x86_64", which
      # could easily be kept as part of the HWaaS if desired to onboard new users.
      simple = {
        path = ./simple;
        description = "A simple template to get started with HWaaS";
      };
    };
  };
}
