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
    await expect(page.getByRole("tab", { name: "电压" })).toHaveClass(
      /tab-active/,
    );

    // Mock should be online.
    await expect(page.locator(".badge.gap-2")).toHaveText("ONLINE");

    // Wait for raw values to appear (voltage calibration mode).
    const localStat = page.locator(".stat", { hasText: "Local Voltage" });
    const remoteStat = page.locator(".stat", { hasText: "Remote Voltage" });
    await expect(localStat.getByText("Raw:")).not.toContainText("--");
    await expect(remoteStat.getByText("Raw:")).not.toContainText("--");

    // Draft starts empty (no user calibration points).
    const localDraftCardVoltage = page.locator(".card", {
      hasText: "Local Draft",
    });
    await expect(localDraftCardVoltage.locator("table")).toContainText(
      "No draft points.",
    );

    // Capture a voltage point at 12V.
    await page.getByLabel("Measured Voltage (V)").fill("12.00");
    await page.getByRole("button", { name: "Capture" }).click();
    await expect(localDraftCardVoltage.locator("table")).toContainText("12000");

    // Switch to current tab (CH1).
    await page.getByRole("tab", { name: "电流通道1" }).click();
    await expect(page.getByRole("tab", { name: "电流通道1" })).toHaveClass(
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

    const localDraftCardCurrent = page.locator(".card", {
      hasText: "Local Draft",
    });
    await expect(localDraftCardCurrent.locator("table")).toContainText("950");
    const draftRows = localDraftCardCurrent.locator("tbody tr");
    expect(await draftRows.count()).toBeGreaterThan(0);

    // Apply and commit.
    const deviceSyncCard = page.locator(".card", { hasText: "Device Sync" });
    await deviceSyncCard.getByRole("button", { name: "Apply" }).click();
    await deviceSyncCard.getByRole("button", { name: "Commit" }).click();

    // Reset back to initial profile.
    await deviceSyncCard
      .getByRole("button", { name: "Reset", exact: true })
      .click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Reset", exact: true })
      .click();

    await expect(localDraftCardCurrent.locator("table")).not.toContainText(
      "950",
    );
    await expect(localDraftCardCurrent.locator("table")).toContainText(
      "No draft points.",
    );

    await page.goto("/");
  });
});
