import { defineConfig } from "@playwright/test";
import { localhostUrl, resolvePort } from "./scripts/ports";

const webPreviewPort = resolvePort("webPreview").port;

export default defineConfig({
  testDir: "./tests/e2e",
  grep: /@preview-smoke/,
  use: {
    baseURL: localhostUrl(webPreviewPort),
    trace: "on-first-retry",
  },
  webServer: {
    command: "bun run preview",
    url: localhostUrl(webPreviewPort),
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
