import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type UserConfig } from "vite";

import { manualChunks } from "./scripts/chunking";
import { resolvePort } from "./scripts/ports";

export function createViteConfig(): UserConfig {
  const webDevPort = resolvePort("webDev").port;
  const webPreviewPort = resolvePort("webPreview").port;

  return {
    base: "/",
    plugins: [react(), tailwindcss()],
    server: {
      port: webDevPort,
      strictPort: true,
    },
    preview: {
      port: webPreviewPort,
      strictPort: true,
    },
    build: {
      rollupOptions: {
        output: {
          manualChunks,
        },
      },
    },
  };
}

export default defineConfig(createViteConfig);
