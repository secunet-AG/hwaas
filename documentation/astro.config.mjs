// @ts-check
import { defineConfig } from "astro/config";
import tailwindcss from "@tailwindcss/vite";
import mdx from "@astrojs/mdx";

const isProd = process.env.NODE_ENV === "production";

// https://astro.build/config
export default defineConfig({
  site: isProd ? "https://secunet-ag.github.io" : "http://localhost:4321",
  base: isProd ? "/hwaas/" : "/",
  trailingSlash: "never",
  build: { format: "file" },
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
