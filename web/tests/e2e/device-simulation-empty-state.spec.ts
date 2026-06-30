import { expect, test } from "@playwright/test";

test.describe("Simulation device empty state", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem("loadlynx.locale", "en");
    });
    await page.goto("/");
    await page.evaluate(() => window.localStorage.clear());
    await page.reload();
    await expect(
      page.getByRole("heading", { name: /Overview|总览/i }).first(),
    ).toBeVisible();
  });

  test("adds simulation device from empty state and opens dashboard", async ({
    page,
  }) => {
    await expect(
      page.getByText(
        /No devices yet\. Add a LoadLynx device to get started\./i,
      ),
    ).toBeVisible();
    await expect(page.getByText(/Add a sample device/i)).toBeVisible();

    await page
      .locator("section")
      .filter({ hasText: "当前已知设备" })
      .getByRole("button", { name: /Add sample device/i })
      .click();

    await expect(page.getByText(/Demo Device #1/i)).toBeVisible();
    await expect(page.getByText(/mock-001/i)).toBeVisible();
    await page.getByRole("link", { name: /Open Dashboard|打开仪表盘/ }).click();

    await expect(page).toHaveURL(/\/cc/);
    await expect(page.getByText("Mode, output and setpoints")).toBeVisible();
    await expect(
      page.getByRole("region", { name: "Live control" }),
    ).toBeVisible();
    await expect(page.getByRole("button", { name: "Presets" })).toBeVisible();
  });
});
