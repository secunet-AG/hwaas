# HWaaS Documentation

This is the HWaaS documentation, to serve locally with the current OpenAPI spec, you can run the following:

```sh
nix run .#serve-docs
```

At the moment, `content/docs` is the source of truth for all documentation pages.

You can add hashed-based, scroll to routing by using H2 tags (## My Tag) to your markdown.

There is not currently any solution implented for sorting these other than tweaking the order in the `docs/[id].astro` component.
