import { cpSync, existsSync } from "node:fs";
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const host = process.env.TAURI_DEV_HOST;

// The licenses dialog loads the notices on demand instead of bundling them.
if (existsSync("THIRD_PARTY_NOTICES.md")) cpSync("THIRD_PARTY_NOTICES.md", "public/THIRD_PARTY_NOTICES.md");

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  build: {
    target: ["safari15", "chrome105"],
    chunkSizeWarningLimit: 1500,
  },
  server: {
    port: 1424,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host ? { protocol: "ws", host, port: 1425 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
