import { sveltekit } from "@sveltejs/kit/vite";
import { defineConfig } from "vite";
import UnoCSS from "@unocss/svelte-scoped/vite";
import Icons from "unplugin-icons/vite";

export default defineConfig({
  server: {
    host: true,
    proxy: {
      "/api/v1": {
        target: "http://127.0.0.1:8080",
      },
      "/oauth/token": {
        target: "http://127.0.0.1:8080/api/internal",
      },
    },
  },

  plugins: [
    UnoCSS({}),
    sveltekit(),
    Icons({
      compiler: "svelte",
    }),
  ],

  css: {
    preprocessorOptions: {
      scss: {
        additionalData: '@use "src/variables.scss" as *;',
      },
    },
  },
});
