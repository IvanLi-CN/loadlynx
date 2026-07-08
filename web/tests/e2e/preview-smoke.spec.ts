import { expect, test, type Page } from "@playwright/test";

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
  await expect(
    page.getByRole("button", { name: /USB-PD/i }),
  ).toBeVisible();
});
