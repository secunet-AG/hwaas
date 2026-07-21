// @ts-check
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";
import mdx from "@astrojs/mdx";
// https://astro.build/config
export default defineConfig({
  site: "https://secunet-ag.github.io",
  base: "/hwaas/",
  integrations: [mdx()],
  vite: {
    plugins: [tailwindcss()],
    server: {
      allowedHosts: true,
    },
  },
  redirects: {
    "/": "/docs/getting-started",
  },
  markdown: {
    shikiConfig: {
      theme: "catppuccin-mocha",
    },
  },
});
