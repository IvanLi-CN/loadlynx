import path from "node:path";
import { fileURLToPath } from "node:url";

import { storybookTest } from "@storybook/addon-vitest/vitest-plugin";
import { defineConfig, mergeConfig } from "vitest/config";

import { createViteConfig } from "./vite.config";

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default mergeConfig(
  createViteConfig(),
  defineConfig({
    test: {
      projects: [
        {
          test: {
            name: "unit",
            environment: "node",
            include: ["src/**/*.test.ts", "tests/unit/**/*.test.ts"],
            exclude: ["tests/e2e/**"],
          },
        },
        {
          extends: true,
          plugins: [
            storybookTest({
              configDir: path.join(dirname, ".storybook"),
              storybookScript: "bun run storybook --ci",
            }),
          ],
          test: {
            name: "storybook",
            browser: {
              enabled: true,
              provider: "playwright",
              headless: true,
              instances: [{ browser: "chromium" }],
            },
            setupFiles: ["./.storybook/vitest.setup.ts"],
          },
        },
      ],
    },
  }),
);
