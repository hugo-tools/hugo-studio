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
        // Code-splitting (M9): the heaviest dependencies live in their
        // own vendor chunks so the initial parse cost is amortised
        // across cached files.
        manualChunks: {
          codemirror: [
            "@uiw/react-codemirror",
            "@codemirror/lang-markdown",
            "@codemirror/state",
            "@codemirror/view",
          ],
          radix: [
            "@radix-ui/react-alert-dialog",
            "@radix-ui/react-dialog",
            "@radix-ui/react-tabs",
            "@radix-ui/react-slot",
          ],
          tanstack: ["@tanstack/react-query"],
          forms: ["react-hook-form", "@hookform/resolvers", "zod"],
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
