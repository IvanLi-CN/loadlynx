import { expect, test } from "@playwright/test";

test.describe("Device Pages", () => {
  test.beforeEach(async ({ page }) => {
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
    await page.goto("/");
    await expect(page.locator("text=LoadLynx Web Console")).toBeVisible();
    await expect(page.getByText(/Demo Device #1/i)).toBeVisible();

    const openDashboardBtn = page
      .getByRole("link", {
        name: /Open Dashboard|打开仪表盘/,
      })
      .first();

    await expect(openDashboardBtn).toBeVisible();
    await openDashboardBtn.click();
    await expect(page.url()).toContain("/cc");
  });

  test("should navigate to Status page and show content", async ({ page }) => {
    await page.getByRole("button", { name: "System" }).click();
    await page.getByRole("link", { name: "Status" }).click();

    await expect(page.url()).toContain("/status");
    await expect(
      page.getByRole("heading", { name: "Device Status", level: 2 }),
    ).toBeVisible();

    // Check for key sections
    await expect(
      page.getByRole("heading", { name: "Overview", level: 3 }),
    ).toBeVisible();
    await expect(
      page.getByRole("heading", { name: "Temperature & Faults", level: 3 }),
    ).toBeVisible();

    // Check for specific data (assuming mock data is present)
    // Voltage unit check
    await expect(page.getByText("Total Current")).toBeVisible();
  });

  test("should open PD settings from Status page secondary entry", async ({
    page,
  }) => {
    await page.getByRole("button", { name: "System" }).click();
    await page.getByRole("link", { name: "Status" }).click();
    await expect(page.url()).toContain("/status");

    const openPdBtn = page.getByRole("link", { name: "Open PD panel" });
    await expect(openPdBtn).toBeVisible();
    await openPdBtn.click();

    await expect(page.url()).toContain("/cc?panel=pd");
    await expect(
      page.getByRole("heading", { name: "USB-PD", level: 2 }),
    ).toBeVisible();
    await expect(page.getByRole("dialog", { name: "USB-PD" })).toBeVisible();
    await expect(
      page.locator('[aria-label="USB-PD control panel"]'),
    ).toBeVisible();
    await expect(page.getByText("Profile list")).toBeVisible();
    await expect(
      page.getByRole("button", { name: /#1 5\.0 V 3000 mA/ }),
    ).toBeVisible();
    await expect(
      page.getByRole("button", { name: /#3 3\.3–21\.0 V 3000 mA/ }),
    ).toBeVisible();
  });

  test("should navigate to Settings page and show content", async ({
    page,
  }) => {
    await page.getByRole("button", { name: "System" }).click();
    await page.getByRole("link", { name: "Settings" }).click();

    await expect(page.url()).toContain("/settings");
    await expect(
      page.getByRole("heading", { name: "Device Settings", level: 2 }),
    ).toBeVisible();

    // Check for key cards
    await expect(page.locator("text=Device Identity")).toBeVisible();
    await expect(page.getByRole("heading", { name: /Network/i })).toBeVisible();
    await expect(page.locator("text=Capabilities")).toBeVisible();

    // Check soft reset button
    const resetBtn = page.locator("button:text('Soft Reset')");
    await expect(resetBtn).toBeVisible();

    await resetBtn.click();

    // ConfirmDialog (app-level modal), then expect a success alert in the UI.
    const confirmDialog = page.getByRole("dialog");
    await expect(confirmDialog).toBeVisible();
    await expect(confirmDialog).toContainText(/Soft Reset/i);
    await confirmDialog.getByRole("button", { name: "Soft Reset" }).click();

    const successAlert = page.locator(".ll-alert-success");
    await expect(successAlert).toBeVisible();
    await expect(successAlert).toContainText(/Soft reset/i);
  });

  test("should preview and restore backup sections from Settings", async ({
    page,
  }) => {
    await page.getByRole("button", { name: "System" }).click();
    await page.getByRole("link", { name: "Settings" }).click();

    await expect(page.url()).toContain("/settings");
    await expect(page.getByText("Backup & Restore")).toBeVisible();

    const backup = {
      kind: "loadlynx.backup",
      schema_version: 1,
      created_at: "2026-05-31T00:00:00Z",
      sections: {
        settings: {
          wifi: {
            ssid: "BenchNet",
            psk: "not-shown",
            source: "user",
          },
          sound: {
            volume: 2,
          },
        },
      },
    };

    await page.getByLabel("Import backup file").setInputFiles({
      name: "loadlynx-backup.json",
      mimeType: "application/json",
      buffer: Buffer.from(JSON.stringify(backup)),
    });

    await expect(page.getByText("loadlynx-backup.json")).toBeVisible();
    await expect(
      page.getByText(/Unknown section ignored: settings.sound/),
    ).toBeVisible();

    await page.getByRole("button", { name: "Restore Selected" }).click();
    await expect(page.getByText("WiFi OK")).toBeVisible();
  });
});
