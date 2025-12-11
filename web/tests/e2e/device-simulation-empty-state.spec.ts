import { expect, test } from "@playwright/test";

test.describe("Simulation device empty state", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    await page.evaluate(() => window.localStorage.clear());
    await page.reload();
    await expect(
      page.getByRole("heading", { name: /LoadLynx Web Console/i, level: 1 }),
    ).toBeVisible();
  });

  test("adds simulation device from empty state and opens CC", async ({
    page,
  }) => {
    await expect(
      page.getByText(/No devices yet\. Add a LoadLynx device/i),
    ).toBeVisible();
    await expect(page.getByText(/Add a simulation device/i)).toBeVisible();

    await page.getByRole("button", { name: /Add simulation device/i }).click();

    await expect(
      page.getByRole("cell", { name: /mock:\/\/demo-1/i }),
    ).toBeVisible();
    await page.getByRole("link", { name: /Open CC Control/i }).click();

    await expect(page).toHaveURL(/\/cc/);
    await expect(page.getByText(/Target current/i)).toBeVisible();
  });
});
