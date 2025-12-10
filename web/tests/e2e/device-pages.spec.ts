import { expect, test } from "@playwright/test";

test.describe("Device Pages", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/");
    // Wait for device list to load (mock data usually loads fast but good to be safe)
    await expect(page.locator("text=LoadLynx Web Console")).toBeVisible();

    // Connect to first device
    const openControlBtn = page.locator("text=Open CC Control").first();

    // If no device is present, try to add a demo device
    if ((await openControlBtn.count()) === 0) {
      const addDemoBtn = page.locator("text=Add demo device");
      if (await addDemoBtn.isVisible()) {
        await addDemoBtn.click();
      }
    }

    await expect(openControlBtn).toBeVisible();
    await openControlBtn.click();

    // Should land on CC Control by default
    await expect(page.url()).toContain("/cc");
  });

  test("should navigate to Status page and show content", async ({ page }) => {
    // Click on "Status" in sidebar
    await page.click("text=Status");

    await expect(page.url()).toContain("/status");
    await expect(page.locator("h2")).toContainText("Device Status");

    // Check for key sections
    await expect(page.locator("text=Overview")).toBeVisible();
    await expect(page.locator("text=Temperature & Faults")).toBeVisible();

    // Check for specific data (assuming mock data is present)
    // Voltage unit check
    await expect(page.locator("text=Voltage")).toBeVisible();
  });

  test("should navigate to Settings page and show content", async ({
    page,
  }) => {
    // Click on "Settings" in sidebar
    await page.click("text=Settings");

    await expect(page.url()).toContain("/settings");
    await expect(page.locator("h2")).toContainText("Device Settings");

    // Check for key cards
    await expect(page.locator("text=Device Identity")).toBeVisible();
    await expect(page.getByRole("heading", { name: /Network/i })).toBeVisible();
    await expect(page.locator("text=Capabilities")).toBeVisible();

    // Check soft reset button
    const resetBtn = page.locator("button:text('Soft Reset')");
    await expect(resetBtn).toBeVisible();

    // Handle dialog
    page.on("dialog", async (dialog) => {
      expect(dialog.message()).toContain(
        "Soft reset API is not yet implemented",
      );
      await dialog.accept();
    });
    await resetBtn.click();
  });
});
