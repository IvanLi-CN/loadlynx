import type { StorybookConfig } from "@storybook/react-vite";

import { manualChunks } from "../scripts/chunking.ts";

const config: StorybookConfig = {
  framework: "@storybook/react-vite",
  stories: ["../src/**/*.mdx", "../src/**/*.stories.@(js|jsx|mjs|ts|tsx)"],
  addons: ["@storybook/addon-vitest", "storybook/viewport"],
  async viteFinal(config, options) {
    const { mergeConfig } = await import("vite");

    if (options.configType !== "PRODUCTION") {
      return config;
    }

    return mergeConfig(config, {
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
