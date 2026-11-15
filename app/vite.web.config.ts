import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { fileURLToPath } from "node:url";

const here = (f: string) => fileURLToPath(new URL(f, import.meta.url));

// Dactile in the browser: the desktop's interface (src/) as it is, with its
// Tauri calls going to web/shim/ and from there to dactile-server (server/),
// which the dev server passes /api on to.
export default defineConfig({
  root: here("./web"),
  publicDir: here("./public"),
  plugins: [svelte()],
  clearScreen: false,
  resolve: {
    alias: {
      "@tauri-apps/api/core": here("./web/shim/core.ts"),
      "@tauri-apps/api/event": here("./web/shim/event.ts"),
      "@tauri-apps/api/app": here("./web/shim/app.ts"),
      "@tauri-apps/api/window": here("./web/shim/window.ts"),
    },
  },
  server: {
    port: 1421,
    strictPort: true,
    fs: { allow: [here(".")] },
    proxy: { "/api": { target: "http://127.0.0.1:1430", ws: true }, "/giris/google": "http://127.0.0.1:1430" },
  },
  build: { target: "es2022", outDir: here("./dist-web"), emptyOutDir: true, chunkSizeWarningLimit: 2000 },
});
