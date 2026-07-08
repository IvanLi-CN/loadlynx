import { readFileSync } from "node:fs";
import { join } from "node:path";
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

function hydrateLocalBuildVersion() {
  if (process.env.VITE_APP_VERSION?.trim()) {
    return;
  }

  try {
    const payload = JSON.parse(
      readFileSync(join(__dirname, "public", "version.json"), "utf8"),
    ) as { version?: string | null };
    const version = payload.version?.trim();
    if (version) {
      process.env.VITE_APP_VERSION = version;
    }
  } catch {
    // Local builds can still proceed without a hydrated build version.
  }
}

function rewritePwaShellGuardEntry(html: string): string {
  const entryTagMatch = html.match(
    /<script[^>]*type="module"[^>]*src="([^"]+)"[^>]*><\/script>/,
  );

  if (!entryTagMatch) {
    return html;
  }

  return html
    .replace(entryTagMatch[0], "")
    .replace("__LOADLYNX_APP_ENTRY__", entryTagMatch[1]);
}

export function createViteConfig(): UserConfig {
  hydrateLocalBuildVersion();

  const webDevPort = resolvePort("webDev").port;
  const webPreviewPort = resolvePort("webPreview").port;
  const enablePwa = !isStorybookBuild();

  return {
    base: "/",
    plugins: [
      {
        name: "loadlynx-pwa-shell-guard-entry",
        transformIndexHtml(html) {
          return rewritePwaShellGuardEntry(html);
        },
      },
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
