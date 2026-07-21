// @ts-check
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";
import mdx from "@astrojs/mdx";
// https://astro.build/config
export default defineConfig({
  site: "https://secunet-ag.github.io/hwaas/",
  integrations: [mdx()],
  vite: {
    plugins: [tailwindcss()],
    server: {
      allowedHosts: true,
    },
  },
  markdown: {
    shikiConfig: {
      theme: "catppuccin-mocha",
    },
  },
});
