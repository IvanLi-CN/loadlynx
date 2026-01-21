import { defineConfig } from "@playwright/test";
import { localhostUrl, resolvePort } from "./scripts/ports";

const webDevPort = resolvePort("webDev").port;

export default defineConfig({
  testDir: "./tests/e2e",
  use: {
    baseURL: localhostUrl(webDevPort),
    trace: "on-first-retry",
  },
  webServer: {
    command: "bun run dev",
    url: localhostUrl(webDevPort),
    reuseExistingServer: false,
    timeout: 120 * 1000,
  },
  projects: [
    {
      name: "desktop",
      use: {
        browserName: "chromium",
        viewport: { width: 1280, height: 800 },
      },
    },
  ],
});
