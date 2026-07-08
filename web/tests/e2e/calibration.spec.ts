import { expect, test } from "@playwright/test";

async function openCalibrationSection(
  page: Parameters<typeof test>[1] extends (args: infer T) => unknown
    ? T["page"]
    : never,
  name: "电压" | "电流通道1" | "电流通道2",
) {
  await page
    .getByRole("navigation", { name: "系统页导航" })
    .getByRole("link", { name, exact: true })
    .click();
}

async function addSampleDeviceFromDevicesPage(
  page: Parameters<typeof test>[1] extends (args: infer T) => unknown
    ? T["page"]
    : never,
) {
  await page
    .locator("section")
    .filter({ hasText: /当前已知设备|Known devices/ })
    .getByRole("button", { name: /添加示例设备|Add sample device/i })
    .click();
}

async function openSystemPage(
  page: Parameters<typeof test>[1] extends (args: infer T) => unknown
    ? T["page"]
    : never,
  name: "设置" | "状态" | "固件" | "关于",
) {
  await page
    .getByRole("navigation", { name: "系统页导航" })
    .getByRole("link", { name, exact: true })
    .click();
}

async function readStatNumericValue(
  page: Parameters<typeof test>[1] extends (args: infer T) => unknown
    ? T["page"]
    : never,
  title: string,
) {
  const text = await page
    .locator(".ll-stat", { hasText: title })
    .locator(".ll-stat-value")
    .textContent();
  const match = text?.match(/-?\d+(?:\.\d+)?/);
  if (!match) {
    throw new Error(`Could not parse numeric value for stat ${title}`);
  }
  return Number(match[0]);
}

async function waitForStableDacValue(
  page: Parameters<typeof test>[1] extends (args: infer T) => unknown
    ? T["page"]
    : never,
) {
  await expect
    .poll(() => readStatNumericValue(page, "DAC Code"))
    .toBeGreaterThan(0);
  return readStatNumericValue(page, "DAC Code");
}

test.describe("Calibration UI", () => {
  test("leaves calibration for sibling system pages from the system page navigation", async ({
    page,
  }) => {
    await page.goto("/devices");
    await addSampleDeviceFromDevicesPage(page);

    await page.goto("/mock-001/calibration?section=current_ch2");
    await expect(page).toHaveURL(
      /\/mock-001\/calibration\?section=current_ch2/,
    );

    await openSystemPage(page, "设置");
    await expect(page).toHaveURL(/\/mock-001\/settings$/);
    await expect(
      page.getByRole("heading", { name: "Device Settings", level: 2 }),
    ).toBeVisible();

    await page.goto("/mock-001/calibration?section=current_ch1");
    await expect(page).toHaveURL(
      /\/mock-001\/calibration\?section=current_ch1/,
    );

    await openSystemPage(page, "状态");
    await expect(page).toHaveURL(/\/mock-001\/status$/);
    await expect(
      page.getByRole("heading", { name: "Device Status", level: 2 }),
    ).toBeVisible();
  });

  test("switches calibration sections from the system page navigation", async ({
    page,
  }) => {
    await page.goto("/devices");
    await addSampleDeviceFromDevicesPage(page);

    await page.goto("/mock-001/calibration");
    await expect(page).toHaveURL(/section=voltage/);
    await expect(page.getByLabel("Measured Voltage (V)")).toBeVisible();

    await openCalibrationSection(page, "电流通道1");
    await expect(page).toHaveURL(/section=current_ch1/);
    await expect(page.getByLabel("Meter Reading (Local) (A)")).toBeVisible();

    await openCalibrationSection(page, "电流通道2");
    await expect(page).toHaveURL(/section=current_ch2/);
    await expect(
      page.getByRole("button", { name: "Copy CH1 → CH2", exact: true }),
    ).toBeVisible();

    await openCalibrationSection(page, "电压");
    await expect(page).toHaveURL(/section=voltage/);
    await expect(page.getByLabel("Measured Voltage (V)")).toBeVisible();
  });

  test("full flow with simulation device", async ({ page }) => {
    await page.goto("/devices");

    await addSampleDeviceFromDevicesPage(page);

    // First simulation device always becomes mock-001.
    const deviceId = "mock-001";

    await page.goto(`/${deviceId}/calibration`);

    await expect(
      page.getByRole("navigation", { name: "主导航" }),
    ).toBeVisible();

    // Voltage tab is default.
    await expect(
      page.getByRole("heading", { name: "Calibration", level: 2 }),
    ).toBeVisible();
    await expect(page).toHaveURL(/section=voltage/);

    // Mock should be online.
    await expect(page.locator(".ll-badge.gap-2")).toHaveText("ONLINE");

    // Wait for raw values to appear (voltage calibration mode).
    const localStat = page.locator(".ll-stat", { hasText: "Local Voltage" });
    const remoteStat = page.locator(".ll-stat", { hasText: "Remote Voltage" });
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
    await openCalibrationSection(page, "电流通道1");
    await expect(page).toHaveURL(/section=current_ch1/);

    // Set target current (1A) and enable output.
    await page.getByRole("button", { name: "1A" }).click();
    await page.getByRole("button", { name: "Set Output" }).click();

    // Wait for raw current to appear.
    const currentStat = page.locator(".ll-stat", { hasText: "Active Current" });
    await expect(currentStat.getByText("Raw:")).not.toContainText("--");
    await expect
      .poll(() => readStatNumericValue(page, "Active Current"))
      .toBeGreaterThan(0.5);
    await waitForStableDacValue(page);

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
    await openCalibrationSection(page, "电流通道2");
    await expect(page).toHaveURL(/section=current_ch2/);

    await expect(draftCurrentTableA).toContainText("No draft points.");
    await page
      .getByRole("button", { name: "Copy CH1 → CH2", exact: true })
      .click();
    await expect(draftCurrentTableA).toContainText("0.900000");
    expect(await draftRowsA.count()).toBe(1);

    // Switch back to CH1.
    await openCalibrationSection(page, "电流通道1");
    await expect(page).toHaveURL(/section=current_ch1/);

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
    await expect
      .poll(() => readStatNumericValue(page, "Active Current"))
      .toBeGreaterThan(1.2);
    await waitForStableDacValue(page);
    await page.getByRole("button", { name: "Capture" }).click();
    expect(await draftRowsMA.count()).toBe(2);

    // Apply and commit.
    const hardwareIoCard = page.locator(".ll-panel", { hasText: "硬件 I/O" });
    await hardwareIoCard
      .getByRole("button", { name: "Apply", exact: true })
      .click();
    await page
      .getByRole("dialog")
      .locator(".ll-modal-action")
      .getByRole("button", { name: "Close" })
      .click();
    await hardwareIoCard.getByRole("button", { name: "Commit" }).click();
    await page
      .getByRole("dialog")
      .locator(".ll-modal-action")
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

    // Navigating away from calibration should keep the header navigation shell.
    await page.goto(`/${deviceId}/cc`);
    await expect(
      page.getByRole("navigation", { name: "主导航" }),
    ).toBeVisible();
    await expect(page.getByText("系统", { exact: true })).toBeVisible();

    await page.goto("/");
  });

  test("restores the saved current tab without mode mismatch", async ({
    page,
  }) => {
    await page.goto("/devices");
    await addSampleDeviceFromDevicesPage(page);

    const deviceId = "mock-001";
    const baseUrl = "mock://demo-1";

    await page.addInitScript(
      ({ seededDeviceId, seededBaseUrl }) => {
        const key = `loadlynx:calibration-draft:v4:${seededDeviceId}:${encodeURIComponent(seededBaseUrl)}`;
        for (const version of [2, 3, 4]) {
          window.localStorage.removeItem(
            `loadlynx:calibration-draft:v${version}:${seededDeviceId}:${encodeURIComponent(seededBaseUrl)}`,
          );
        }
        window.localStorage.setItem(
          key,
          JSON.stringify({
            version: 4,
            saved_at: "2026-04-16T00:00:00.000Z",
            device_id: seededDeviceId,
            base_url: seededBaseUrl,
            active_tab: "current_ch2",
            draft_profile: {
              v_local_points: [],
              v_remote_points: [],
              current_ch1_points: [],
              current_ch2_points: [
                [[5300, 685], 989600],
                [[10680, 1375], 1984700],
              ],
            },
          }),
        );
      },
      { seededDeviceId: deviceId, seededBaseUrl: baseUrl },
    );

    await page.goto(`/${deviceId}/calibration`);
    await expect(page).toHaveURL(/section=current_ch2/);

    const modeBadge = page.locator(".ll-badge", { hasText: "cal_mode:" });
    await expect(modeBadge).toContainText("current_ch2");

    const currentStat = page.locator(".ll-stat", { hasText: "Active Current" });
    await expect(currentStat.getByText("Raw:")).not.toContainText("--");

    await page.reload();

    await expect(page).toHaveURL(/section=current_ch2/);
    const draftCurrentTable = page.locator("table", { hasText: "Value (A)" });
    const draftRows = draftCurrentTable.locator("tbody tr");
    await expect(draftRows).toHaveCount(2);

    await page.getByLabel("Meter Reading (Remote) (A)").fill("2.050000");
    await page.getByRole("button", { name: "Capture" }).click();

    await expect(modeBadge).toContainText("current_ch2");
    await expect(currentStat.getByText("Raw:")).not.toContainText("--");
    await expect(draftRows).toHaveCount(3);
    await expect(
      page.getByText(/正在同步校准模式：等待设备切换到/i),
    ).toHaveCount(0);
  });

  test("leaving calibration tears down calibration mode", async ({ page }) => {
    await page.goto("/devices");
    await addSampleDeviceFromDevicesPage(page);

    const deviceId = "mock-001";
    const baseUrl = "mock://demo-1";

    await page.goto(`/${deviceId}/calibration`);
    await openCalibrationSection(page, "电流通道1");
    await expect(page).toHaveURL(/section=current_ch1/);
    await page.getByRole("button", { name: "1A" }).click();
    await page.getByRole("button", { name: "Set Output" }).click();

    const modeBadge = page.locator(".ll-badge", { hasText: "cal_mode:" });
    await expect(modeBadge).toContainText("current_ch1");

    await page.goto("/devices");

    await expect
      .poll(
        () =>
          page.evaluate(async (targetBaseUrl) => {
            const { getStatus } = await import("/src/api/client.ts");
            const status = await getStatus(targetBaseUrl);
            return status.raw.cal_kind ?? null;
          }, baseUrl),
        { timeout: 5_000 },
      )
      .toBe(null);
  });
});
