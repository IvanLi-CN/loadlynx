import { expect, test } from "@playwright/test";

// Manual hardware-in-the-loop soft reset test.
// This spec is NOT meant for CI; it assumes a real device is reachable at
// REAL_DEVICE_BASE_URL and will be skipped otherwise.

const REAL_DEVICE_BASE_URL = process.env.REAL_DEVICE_BASE_URL;

test.describe("Manual Soft Reset (HIL)", () => {
  // Skip in CI or when a real-device base URL is not configured.
  test.skip(
    !REAL_DEVICE_BASE_URL || process.env.CI === "true",
    "HIL soft reset test requires REAL_DEVICE_BASE_URL and is not for CI",
  );

  test("manual soft reset via Settings page", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByText("LoadLynx Web Console")).toBeVisible();

    // Add a real device entry.
    const nameInput = page.getByLabel("Device name");
    const baseUrlInput = page.getByLabel("Base URL");

    await nameInput.fill("Real Device (manual)");
    await baseUrlInput.fill(REAL_DEVICE_BASE_URL as string);

    await page.getByRole("button", { name: "Add device" }).click();

    // Wait for the new device row and open CC control.
    await expect(page.getByText("Real Device (manual)")).toBeVisible();
    await page.getByRole("link", { name: "Open CC Control" }).last().click();

    // We should be on the CC control page for this device.
    await expect(
      page.getByRole("heading", { name: "Device control" }),
    ).toBeVisible();

    // Navigate to Settings in the sidebar (avoid matching the route hint code).
    await page.getByRole("link", { name: "Settings" }).click();
    await expect(
      page.getByRole("heading", { name: "Device Settings" }),
    ).toBeVisible();

    // Confirm dialog handler for Soft Reset.
    page.on("dialog", async (dialog) => {
      await dialog.accept();
    });

    const resetBtn = page.getByRole("button", { name: "Soft Reset" });
    await expect(resetBtn).toBeVisible();
    await resetBtn.click();

    // Expect a success alert mentioning soft reset was requested.
    const successAlert = page.locator(".alert-success");
    await expect(successAlert).toBeVisible();
    await expect(successAlert).toContainText(/Soft reset requested/i);
  });
});
