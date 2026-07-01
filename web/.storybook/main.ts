import path from "node:path";
import { fileURLToPath } from "node:url";
import type { StorybookConfig } from "@storybook/react-vite";

import { manualChunks } from "../scripts/chunking.ts";

const dirname = path.dirname(fileURLToPath(import.meta.url));

const config: StorybookConfig = {
  framework: "@storybook/react-vite",
  stories: ["../src/**/*.stories.@(js|jsx|mjs|ts|tsx)"],
  addons: ["@storybook/addon-vitest", "storybook/viewport"],
  core: {
    disableTelemetry: true,
  },
  async viteFinal(config, options) {
    const { mergeConfig } = await import("vite");
    const baseConfig = mergeConfig(config, {
      resolve: {
        alias: {
          "virtual:pwa-register/react": path.resolve(
            dirname,
            "../src/pwa/pwa-register-storybook.ts",
          ),
        },
      },
    });

    if (options.configType !== "PRODUCTION") {
      return baseConfig;
    }

    return mergeConfig(baseConfig, {
      build: {
        // Storybook injects a framework-owned Vitest mocker runtime; bundle
        // budgets are enforced separately in scripts/check-bundles.ts.
        chunkSizeWarningLimit: 1200,
        rollupOptions: {
          output: {
            manualChunks,
          },
        },
      },
    });
  },
};

export default config;
