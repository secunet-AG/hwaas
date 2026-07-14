// @ts-check
import { defineConfig } from "astro/config";
import scalarAstro from "@scalar/astro";

// https://astro.build/config
export default defineConfig({
  integrations: [
    scalarAstro({
      spec: {
        url: "/openapi.json",
      },
      configuration: {
        title: "HWaaS API",
      },
    }),
  ],
});
