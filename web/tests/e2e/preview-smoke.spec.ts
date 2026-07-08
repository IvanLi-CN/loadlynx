import { readFileSync } from "node:fs";
import { join } from "node:path";
import { expect, type Page, test } from "@playwright/test";

function captureRuntimeErrors(page: Page) {
  const consoleErrors: string[] = [];
  const pageErrors: string[] = [];

  page.on("console", (message) => {
    if (message.type() === "error") {
      consoleErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    pageErrors.push(error.message);
  });

  return { consoleErrors, pageErrors };
}

test("production preview mounts without runtime crashes @preview-smoke", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = captureRuntimeErrors(page);

  await page.goto("/");

  await expect(
    page.getByRole("link", { name: /Overview|总览/i }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: /Overview|总览/i }).first(),
  ).toBeVisible();
  await expect(pageErrors).toEqual([]);
  await expect(consoleErrors).toEqual([]);
});

test("production preview opens the dashboard route without runtime crashes @preview-smoke", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = captureRuntimeErrors(page);

  await page.addInitScript(() => {
    window.localStorage.setItem("loadlynx.locale", "en");
    window.localStorage.setItem("loadlynx.demoMode", "true");
    window.localStorage.setItem(
      "loadlynx.demo.devices",
      JSON.stringify([
        {
          id: "mock-001",
          name: "Demo Device #1",
          baseUrl: "mock://demo-1",
        },
      ]),
    );
  });

  await page.goto("/devices?demo=true");
  const demoDeviceCard = page
    .getByRole("article")
    .filter({ hasText: "mock-001" });
  await demoDeviceCard
    .getByRole("link", { name: /Open Dashboard|打开仪表盘/i })
    .click();

  await expect(page).toHaveURL(/\/mock-001\/cc$/);
  await expect(pageErrors).toEqual([]);
  await expect(consoleErrors).toEqual([]);
  await expect(
    page.locator('[aria-label="Primary dashboard monitor"]'),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: /USB-PD/i })).toBeVisible();
});

test("production preview recovers from a stale HTML shell before loading dead assets @preview-smoke", async ({
  page,
}) => {
  const { consoleErrors, pageErrors } = captureRuntimeErrors(page);
  const currentShell = readFileSync(
    join(process.cwd(), "dist", "index.html"),
    "utf8",
  );
  const staleShell = currentShell
    .replace(
      /data-shell-version="([^"]*)"/,
      'data-shell-version="0.0.0+stale-shell"',
    )
    .replace(
      /data-app-entry="([^"]+)"/,
      'data-app-entry="/assets/definitely-missing-stale-entry.js"',
    );
  let requestCount = 0;

  await page.addInitScript(() => {
    window.localStorage.setItem("loadlynx.locale", "en");
    window.localStorage.setItem("loadlynx.demoMode", "true");
    window.localStorage.setItem(
      "loadlynx.demo.devices",
      JSON.stringify([
        {
          id: "mock-001",
          name: "Demo Device #1",
          baseUrl: "mock://demo-1",
        },
      ]),
    );
  });

  await page.route(/\/mock-001\/cc\?demo=true(?:&.*)?$/, async (route) => {
    requestCount += 1;
    await route.fulfill({
      status: 200,
      contentType: "text/html; charset=utf-8",
      body: requestCount === 1 ? staleShell : currentShell,
    });
  });

  await page.goto("/mock-001/cc?demo=true&stale-shell-test=1");

  await expect(page).toHaveURL(/__ll_sw_recover=/);
  await expect(
    page.locator('[aria-label="Primary dashboard monitor"]'),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: /USB-PD/i })).toBeVisible();
  await expect(pageErrors).toEqual([]);
  await expect(consoleErrors).toEqual([]);
});
