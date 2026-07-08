import { expect, test } from "@playwright/test";

test.describe("Simulation device empty state", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.evaluate(() => {
      window.localStorage.clear();
      window.localStorage.setItem("loadlynx.locale", "en");
    });
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
      .filter({ hasText: /Known devices|当前已知设备/ })
      .getByRole("button", { name: /Add sample device|添加示例设备/i })
      .click();

    const demoDeviceCard = page
      .getByRole("article")
      .filter({ hasText: "mock-001" });
    await expect(
      demoDeviceCard.getByRole("heading", { name: /Demo Device #1/i }),
    ).toBeVisible();
    await expect(demoDeviceCard.getByText(/mock-001/i)).toBeVisible();
    await demoDeviceCard
      .getByRole("link", { name: /Open Dashboard|打开仪表盘/ })
      .click();

    await expect(page).toHaveURL(/\/cc/);
    await expect(page.getByText("Mode, output and setpoints")).toBeVisible();
    await expect(
      page.getByRole("region", { name: "Live control" }),
    ).toBeVisible();
    await expect(page.getByRole("button", { name: "Presets" })).toBeVisible();
  });
});
