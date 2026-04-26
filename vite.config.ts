import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  build: {
    // Cap raised because the editor + git + preview wiring is genuinely
    // chunky; the manualChunks split below keeps any single chunk under
    // 400KB after gzip in practice.
    chunkSizeWarningLimit: 800,
    rollupOptions: {
      output: {
        // Code-splitting: function form because Milkdown pulls in
        // @milkdown/* + prosemirror-* transitively, and matching only
        // the top-level package name leaves the transitive deps in the
        // main chunk. Match by id substring instead.
        manualChunks(id) {
          if (
            id.includes("/@milkdown/") ||
            id.includes("/prosemirror-") ||
            id.includes("/@prosemirror/")
          ) {
            return "milkdown";
          }
          if (
            id.includes("/@codemirror/") ||
            id.includes("/@uiw/react-codemirror/") ||
            id.includes("/@lezer/")
          ) {
            return "codemirror";
          }
          if (id.includes("/@radix-ui/")) {
            return "radix";
          }
          if (id.includes("/@tanstack/")) {
            return "tanstack";
          }
          if (
            id.includes("/react-hook-form/") ||
            id.includes("/@hookform/") ||
            id.includes("/zod/")
          ) {
            return "forms";
          }
        },
      },
    },
  },

  // Vite options tailored for Tauri development; only applied in `tauri dev` / `tauri build`.
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. Tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "0.0.0.0",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
