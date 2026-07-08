import { expect, test } from "@playwright/test";

test.describe("Demo mode", () => {
  test("uses normal console routes while switching the API data mode", async ({
    page,
  }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem("loadlynx.locale", "en");
    });
    let realProbeCount = 0;
    await page.route("http://192.0.2.10/**", async (route) => {
      realProbeCount += 1;
      await route.abort();
    });
    await page.addInitScript(() => {
      window.localStorage.setItem(
        "loadlynx.demo.devices",
        JSON.stringify([
          {
            id: "real-demo",
            name: "Real Device In Demo",
            baseUrl: "http://192.0.2.10",
          },
        ]),
      );
      window.localStorage.setItem(
        "loadlynx.devices",
        JSON.stringify([
          {
            id: "cached-real",
            name: "Cached Real Device",
            baseUrl: "http://192.0.2.10",
          },
        ]),
      );
    });

    await page.goto("/devices?demo=false");
    await expect(page.getByText("Cached Real Device")).toBeVisible();
    await expect(
      page
        .locator("article")
        .filter({ hasText: "Cached Real Device" })
        .getByText("Offline", { exact: true }),
    ).toBeVisible();
    realProbeCount = 0;

    await page.goto("/devices?demo=true");

    await expect(page).toHaveURL(/\/devices$/);
    await expect(
      page.evaluate(() => window.localStorage.getItem("loadlynx.demoMode")),
    ).resolves.toBe("true");
    const demoDeviceCard = page
      .getByRole("article")
      .filter({ hasText: "mock-001" });
    await expect(
      demoDeviceCard.getByRole("heading", { name: "Demo Device #1" }),
    ).toBeVisible();
    await expect(demoDeviceCard.getByText("mock-001")).toBeVisible();
    await expect(page.getByText("Real Device In Demo")).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Add device" }),
    ).toBeDisabled();
    await expect(page.getByRole("button", { name: "Refresh" })).toBeDisabled();
    await expect(
      page.getByRole("button", { name: "Scan network..." }),
    ).toBeDisabled();
    realProbeCount = 0;
    await page.waitForTimeout(250);
    await expect.poll(() => realProbeCount).toBe(0);

    await demoDeviceCard.getByRole("link", { name: "Open Dashboard" }).click();
    await expect(page).toHaveURL(/\/mock-001\/cc$/);
    await expect(page.getByText("Mode, output and setpoints")).toBeVisible();
    await expect(
      page.getByRole("region", { name: "Live control" }),
    ).toBeVisible();
    await expect(page.getByRole("button", { name: "Presets" })).toBeVisible();

    await page.goto("/devices?demo=false");
    await expect(page).toHaveURL(/\/devices$/);
    await expect(
      page.evaluate(() => window.localStorage.getItem("loadlynx.demoMode")),
    ).resolves.toBe("false");
  });
});
