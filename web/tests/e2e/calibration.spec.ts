import { expect, test } from "@playwright/test";

test.describe("Calibration UI", () => {
  test("full flow with simulation device", async ({ page }) => {
    await page.goto("/devices");

    await page.getByRole("button", { name: "Add simulation device" }).click();

    // First simulation device always becomes mock-001.
    const deviceId = "mock-001";

    await page.goto(`/${deviceId}/calibration`);

    await expect(page.locator("aside")).toHaveCount(0);

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
    const draftVoltageTable = page.locator("table", { hasText: "Value (mV)" });
    await expect(draftVoltageTable).toContainText("No draft points.");

    // Capture voltage points (1µV input precision, stored as integer mV).
    const measuredVoltage = page.getByLabel("Measured Voltage (V)");
    await expect(measuredVoltage).toHaveAttribute("step", "0.000001");

    await measuredVoltage.fill("12.00");
    await page.getByRole("button", { name: "Capture" }).click();
    await expect(draftVoltageTable).toContainText("12000");

    // 12.000500 V = 12000.5 mV -> rounds half-up to 12001 mV.
    await measuredVoltage.fill("12.000500");
    await page.getByRole("button", { name: "Capture" }).click();
    await expect(draftVoltageTable).toContainText("12001");

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

    // Advanced: subtract baseline current (e.g., adapters/fixtures).
    await page.locator("summary", { hasText: "高级选项" }).click();
    await page.getByLabel("基础电流扣除 (Local) (A)").fill("0.050");

    // Capture current point based on meter reading.
    await page.getByLabel("Meter Reading (Local) (A)").fill("0.950");
    await page.getByRole("button", { name: "Capture" }).click();

    const draftCurrentTableA = page.locator("table", { hasText: "Value (A)" });
    await expect(draftCurrentTableA).toContainText("0.900000");
    const draftRowsA = draftCurrentTableA.locator("tbody tr");
    expect(await draftRowsA.count()).toBe(1);

    // Switch to current tab (CH2) and copy CH1 calibration into CH2 draft.
    await page.getByRole("tab", { name: "电流通道2" }).click();
    await expect(page.getByRole("tab", { name: "电流通道2" })).toHaveClass(
      /tab-active/,
    );

    await expect(draftCurrentTableA).toContainText("No draft points.");
    await page
      .getByRole("button", { name: "Copy CH1 → CH2", exact: true })
      .click();
    await expect(draftCurrentTableA).toContainText("0.900000");
    expect(await draftRowsA.count()).toBe(1);

    // Switch back to CH1.
    await page.getByRole("tab", { name: "电流通道1" }).click();
    await expect(page.getByRole("tab", { name: "电流通道1" })).toHaveClass(
      /tab-active/,
    );

    // Unit toggle + precision: in mA mode, inputs are 1µA steps (0.001mA).
    await page.getByRole("button", { name: "mA", exact: true }).click();
    const baselineInput = page.getByLabel("基础电流扣除 (Local) (mA)");
    await baselineInput.fill("0.952");
    await page.getByLabel("Meter Reading (Local) (mA)").click(); // blur
    await expect(baselineInput).toHaveValue("0.952");

    const draftCurrentTableMA = page.locator("table", {
      hasText: "Value (mA)",
    });
    const draftRowsMA = draftCurrentTableMA.locator("tbody tr");

    // Restore baseline/meter to keep the duplicate-measurement warning path.
    await baselineInput.fill("50.000");
    await page.getByLabel("Meter Reading (Local) (mA)").fill("950.000");

    // Re-capture the same meter reading after changing the output. Draft should
    // allow duplicate samples; apply/commit will later clean them (mode/median)
    // and show a warning.
    await page.getByRole("button", { name: "2A" }).click();
    await page.getByRole("button", { name: "Set Output" }).click();
    await expect(currentStat.locator(".stat-value")).toContainText("1.7100 A");
    await page.getByRole("button", { name: "Capture" }).click();
    expect(await draftRowsMA.count()).toBe(2);

    // Apply and commit.
    const hardwareIoCard = page.locator(".card", { hasText: "硬件 I/O" });
    await hardwareIoCard
      .getByRole("button", { name: "Apply", exact: true })
      .click();
    await page
      .getByRole("dialog")
      .locator(".modal-action")
      .getByRole("button", { name: "Close" })
      .click();
    await hardwareIoCard.getByRole("button", { name: "Commit" }).click();
    await page
      .getByRole("dialog")
      .locator(".modal-action")
      .getByRole("button", { name: "Close" })
      .click();

    // Reset back to initial profile.
    await page.getByRole("tab", { name: "设备数据" }).click();
    await page.getByRole("button", { name: "Reset", exact: true }).click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Reset", exact: true })
      .click();
    await page.getByRole("tab", { name: "本地草稿" }).click();

    await expect(draftCurrentTableMA).not.toContainText("900.000");
    await expect(draftCurrentTableMA).toContainText("No draft points.");

    await page.goto("/");
  });
});
