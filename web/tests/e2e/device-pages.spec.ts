import type { Route } from "@playwright/test";
import { expect, test } from "@playwright/test";

const corsJsonHeaders = {
  "access-control-allow-headers": "content-type",
  "access-control-allow-methods": "GET,POST,DELETE,OPTIONS",
  "access-control-allow-origin": "*",
  "content-type": "application/json",
};

async function fulfillJson(route: Route, body: unknown) {
  await route.fulfill({
    status: 200,
    headers: corsJsonHeaders,
    body: JSON.stringify(body),
  });
}

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
    const demoDeviceCard = page
      .getByRole("article")
      .filter({ hasText: "mock-001" });
    await expect(
      demoDeviceCard.getByRole("heading", { name: /Demo Device #1/i }),
    ).toBeVisible();

    const openDashboardBtn = demoDeviceCard.getByRole("link", {
      name: /Open Dashboard|打开仪表盘/,
    });

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

  test("should refresh WiFi status after an async WiFi save", async ({
    page,
  }) => {
    await page.getByRole("button", { name: "System" }).click();
    await page.getByRole("link", { name: "Settings" }).click();

    await expect(page.url()).toContain("/settings");
    await expect(page.getByRole("heading", { name: "WiFi" })).toBeVisible();

    await page.getByPlaceholder("SSID").fill("BenchNet");
    await page.getByPlaceholder("PSK").fill("not-shown");
    await page.getByRole("button", { name: "Save WiFi" }).click();

    await expect(page.getByTestId("wifi-status-state")).toHaveText(
      "configured",
    );
    await expect(page.getByTestId("wifi-status-state")).toHaveText(
      "connected",
      { timeout: 5000 },
    );
    await expect(page.getByTestId("wifi-status-ip")).toHaveText("192.0.2.11");
  });

  test("should refresh WiFi status from the USB/devd write target", async ({
    page,
  }) => {
    const directBaseUrl = "http://192.0.2.55";
    const devdBaseUrl = "http://127.0.0.1:39881";
    let devdWifiGetCount = 0;

    const identity = {
      device_id: "loadlynx-test",
      digital_fw_version: "digital test",
      analog_fw_version: "analog test",
      protocol_version: 1,
      uptime_ms: 123000,
      capabilities: {
        cc_supported: true,
        cv_supported: true,
        cp_supported: true,
        presets_supported: true,
        preset_count: 5,
        api_version: "2.0.0-test",
        devd: true,
        dns_sd: true,
        mdns: true,
        usb_cdc_bridge: true,
      },
      network: {
        hostname: "loadlynx-test.local",
        ip: "192.0.2.55",
        mac: "00:00:00:00:00:55",
      },
    };

    await page.route(`${directBaseUrl}/**`, async (route) => {
      const request = route.request();
      if (request.method() === "OPTIONS") {
        await route.fulfill({ status: 204, headers: corsJsonHeaders });
        return;
      }

      const path = new URL(request.url()).pathname;
      if (path === "/api/v1/identity") {
        await fulfillJson(route, identity);
        return;
      }
      if (path === "/api/v1/wifi") {
        await fulfillJson(route, {
          ssid: "OldNet",
          source: "user",
          state: "connecting",
          ip: null,
          last_error: null,
        });
        return;
      }
      if (path === "/api/v1/pd") {
        await fulfillJson(route, {
          fixed: { enabled: false, voltage_mv: 5000, current_ma: 3000 },
          pps: { enabled: false, voltage_mv: 5000, current_ma: 3000 },
        });
        return;
      }

      await route.fulfill({ status: 404, headers: corsJsonHeaders });
    });

    await page.route(`${devdBaseUrl}/**`, async (route) => {
      const request = route.request();
      if (request.method() === "OPTIONS") {
        await route.fulfill({ status: 204, headers: corsJsonHeaders });
        return;
      }

      const path = new URL(request.url()).pathname;
      if (path === "/api/v1/serial/lease") {
        await fulfillJson(route, {
          device_id: "digital-test",
          identity_device_id: "loadlynx-test",
          lease_id: "lease-test",
          heartbeat_interval_ms: 2000,
          lease_ttl_ms: 8000,
        });
        return;
      }
      if (path === "/api/v1/identity") {
        await fulfillJson(route, identity);
        return;
      }
      if (path === "/api/v1/wifi" && request.method() === "POST") {
        await fulfillJson(route, {
          ssid: "BenchNet",
          source: "user",
          state: "connecting",
          ip: null,
          last_error: null,
        });
        return;
      }
      if (path === "/api/v1/wifi") {
        devdWifiGetCount += 1;
        await fulfillJson(route, {
          ssid: "BenchNet",
          source: "user",
          state: devdWifiGetCount >= 2 ? "connected" : "connecting",
          ip: devdWifiGetCount >= 2 ? "192.0.2.11" : null,
          last_error: null,
        });
        return;
      }
      if (path === "/api/v1/pd") {
        await fulfillJson(route, {
          fixed: { enabled: false, voltage_mv: 5000, current_ma: 3000 },
          pps: { enabled: false, voltage_mv: 5000, current_ma: 3000 },
        });
        return;
      }

      await route.fulfill({ status: 404, headers: corsJsonHeaders });
    });

    await page.addInitScript(
      ({ directUrl, devdUrl }) => {
        window.localStorage.setItem("loadlynx.demoMode", "false");
        window.localStorage.setItem(
          "loadlynx.devices",
          JSON.stringify([
            {
              id: "direct-http",
              name: "WiFi Device",
              baseUrl: directUrl,
              connectionMarks: ["usb"],
              devd: {
                baseUrl: devdUrl,
                deviceId: "digital-test",
              },
            },
          ]),
        );
      },
      { directUrl: directBaseUrl, devdUrl: devdBaseUrl },
    );

    await page.goto("/direct-http/settings");
    await expect(page.getByRole("heading", { name: "WiFi" })).toBeVisible();
    await expect(page.getByTestId("wifi-status-state")).toHaveText(
      "connecting",
    );

    await page
      .getByRole("button", { name: /Switch connection|切换连接方式/ })
      .click();
    const switchDialog = page.getByRole("dialog");
    await expect(switchDialog).toBeVisible();
    await expect(switchDialog).toContainText("USB/devd");
    await switchDialog.getByRole("button", { name: "Continue" }).click();
    await expect(page.getByPlaceholder("SSID")).toBeEnabled();

    await page.getByPlaceholder("SSID").fill("BenchNet");
    await page.getByPlaceholder("PSK").fill("not-shown");
    await page.getByRole("button", { name: "Save WiFi" }).click();

    await expect(page.getByTestId("wifi-status-state")).toHaveText(
      "connected",
      { timeout: 5000 },
    );
    await expect(page.getByTestId("wifi-status-ip")).toHaveText("192.0.2.11");
  });

  test("should block WiFi writes over verified LAN without USB/devd", async ({
    page,
  }) => {
    const directBaseUrl = "http://192.0.2.56";
    let directWifiPostCount = 0;

    const identity = {
      device_id: "loadlynx-lan-only",
      digital_fw_version: "digital test",
      analog_fw_version: "analog test",
      protocol_version: 1,
      uptime_ms: 123000,
      capabilities: {
        cc_supported: true,
        cv_supported: true,
        cp_supported: true,
        presets_supported: true,
        preset_count: 5,
        api_version: "2.0.0-test",
        devd: false,
        dns_sd: true,
        mdns: true,
        usb_cdc_bridge: false,
      },
      network: {
        hostname: "loadlynx-lan-only.local",
        ip: "192.0.2.56",
        mac: "00:00:00:00:00:56",
      },
    };

    await page.route(`${directBaseUrl}/**`, async (route) => {
      const request = route.request();
      if (request.method() === "OPTIONS") {
        await route.fulfill({ status: 204, headers: corsJsonHeaders });
        return;
      }

      const path = new URL(request.url()).pathname;
      if (path === "/api/v1/identity") {
        await fulfillJson(route, identity);
        return;
      }
      if (path === "/api/v1/wifi" && request.method() === "POST") {
        directWifiPostCount += 1;
        await fulfillJson(route, {
          ssid: "BenchNet",
          source: "user",
          state: "connecting",
          ip: null,
          last_error: null,
        });
        return;
      }
      if (path === "/api/v1/wifi") {
        await fulfillJson(route, {
          ssid: "OldNet",
          source: "user",
          state: "connected",
          ip: "192.0.2.56",
          last_error: null,
        });
        return;
      }
      if (path === "/api/v1/pd") {
        await fulfillJson(route, {
          fixed: { enabled: false, voltage_mv: 5000, current_ma: 3000 },
          pps: { enabled: false, voltage_mv: 5000, current_ma: 3000 },
        });
        return;
      }

      await route.fulfill({ status: 404, headers: corsJsonHeaders });
    });

    await page.addInitScript((directUrl) => {
      window.localStorage.setItem("loadlynx.demoMode", "false");
      window.localStorage.setItem(
        "loadlynx.devices",
        JSON.stringify([
          {
            id: "lan-only",
            name: "Verified LAN Device",
            baseUrl: directUrl,
            connectionMarks: ["lan"],
          },
        ]),
      );
    }, directBaseUrl);

    await page.goto("/lan-only/settings");
    await expect(page.getByRole("heading", { name: "WiFi" })).toBeVisible();

    await page
      .getByRole("button", { name: /Switch connection|切换连接方式/ })
      .click();

    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toContainText("Bind Connection for WiFi Settings");
    await expect(
      dialog.getByRole("link", { name: "Bind connection" }),
    ).toBeVisible();
    await expect(dialog.getByRole("button", { name: "Continue" })).toHaveCount(
      0,
    );
    expect(directWifiPostCount).toBe(0);
  });
});
