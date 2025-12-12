import { expect, test } from "@playwright/test";

test.describe("Calibration UI", () => {
  test("full flow with simulation device", async ({ page }) => {
    await page.goto("/devices");

    await page.getByRole("button", { name: "Add simulation device" }).click();

    // First simulation device always becomes mock-001.
    const deviceId = "mock-001";

    await page.goto(`/${deviceId}/calibration`);

    // Voltage tab is default.
    await expect(page.getByRole("heading", { level: 2 })).toHaveText(
      "Calibration",
    );
    await expect(page.getByRole("tab", { name: "Voltage" })).toHaveClass(
      /tab-active/,
    );

    // Mock should be online.
    await expect(page.locator(".badge")).toHaveText("ONLINE");

    // Wait for raw values to appear (voltage calibration mode).
    const localStat = page.locator(".stat", { hasText: "Local Voltage" });
    const remoteStat = page.locator(".stat", { hasText: "Remote Voltage" });
    await expect(localStat.getByText("Raw:")).not.toContainText("--");
    await expect(remoteStat.getByText("Raw:")).not.toContainText("--");

    // Capture a voltage point at 12V.
    await page.getByLabel("Measured Voltage (V)").fill("12.00");
    await page.getByRole("button", { name: "Capture Point" }).click();
    await expect(page.locator("table").first()).toContainText("12000");

    // Switch to current tab.
    await page.getByRole("tab", { name: "Current" }).click();
    await expect(page.getByRole("tab", { name: "Current" })).toHaveClass(
      /tab-active/,
    );

    // Set target current (1A) and enable output.
    await page.getByRole("button", { name: "1A" }).click();
    await page.getByRole("button", { name: "Set Output" }).click();

    // Wait for raw current to appear.
    const currentStat = page.locator(".stat", { hasText: "Active Current" });
    await expect(currentStat.getByText("Raw:")).not.toContainText("--");

    // Capture current point based on meter reading.
    await page.getByLabel("Meter Reading (Local) (A)").fill("0.950");
    await page.getByRole("button", { name: "Capture" }).click();

    await expect(page.locator("table")).toContainText("950");
    await expect(page.locator("tbody tr")).toHaveCount(3);

    // Apply and commit.
    await page.getByRole("button", { name: "Apply" }).click();
    await page.getByRole("button", { name: "Commit" }).click();

    // Reset back to initial profile.
    await page.getByRole("button", { name: "Reset" }).click();
    await expect(page.locator("table")).not.toContainText("950");
    await expect(page.locator("tbody tr")).toHaveCount(2);

    await page.goto("/");
  });
});
