import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig, type UserConfig } from "vite";
import { VitePWA } from "vite-plugin-pwa";

import { manualChunks } from "./scripts/chunking";
import { resolvePort } from "./scripts/ports";

function isStorybookBuild(): boolean {
  return (
    process.env.STORYBOOK === "true" ||
    process.env.npm_lifecycle_event?.includes("storybook") === true ||
    process.argv.some((arg) => arg.includes("storybook"))
  );
}

export function createViteConfig(): UserConfig {
  const webDevPort = resolvePort("webDev").port;
  const webPreviewPort = resolvePort("webPreview").port;
  const enablePwa = !isStorybookBuild();

  return {
    base: "/",
    plugins: [
      react(),
      tailwindcss(),
      enablePwa
        ? VitePWA({
            injectRegister: null,
            registerType: "prompt",
            manifest: {
              id: "/",
              name: "LoadLynx Web Console",
              short_name: "LoadLynx",
              description:
                "Bench instrument console for LoadLynx device setup, monitoring, calibration and firmware workflows.",
              start_url: "/",
              scope: "/",
              display: "standalone",
              background_color: "#080b14",
              theme_color: "#08111d",
              orientation: "any",
              categories: ["utilities", "productivity"],
              icons: [
                {
                  src: "/favicon.svg",
                  sizes: "any",
                  type: "image/svg+xml",
                  purpose: "any maskable",
                },
              ],
            },
            workbox: {
              cleanupOutdatedCaches: true,
              navigateFallback: "/index.html",
              globPatterns: [
                "**/*.{js,css,html,ico,png,svg,webmanifest,woff2}",
              ],
              globIgnores: ["**/firmware/**", "**/version.json"],
            },
          })
        : null,
    ],
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
