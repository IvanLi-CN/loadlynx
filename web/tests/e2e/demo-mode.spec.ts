import { expect, test } from "@playwright/test";

test.describe("Demo mode", () => {
  test("uses normal console routes while switching the API data mode", async ({
    page,
  }) => {
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
    await expect(page.getByText("Offline")).toBeVisible();
    realProbeCount = 0;

    await page.goto("/devices?demo=true");

    await expect(page).toHaveURL(/\/devices$/);
    await expect(
      page.evaluate(() => window.localStorage.getItem("loadlynx.demoMode")),
    ).resolves.toBe("true");
    await expect(page.getByText("Demo Device #1")).toBeVisible();
    await expect(page.getByText("mock://demo-1")).toBeVisible();
    await expect(page.getByText("Real Device In Demo")).toHaveCount(0);
    await expect(
      page.getByRole("button", { name: "Add device" }),
    ).toBeDisabled();
    await expect(
      page.getByRole("button", { name: "Scan devd" }),
    ).toBeDisabled();
    await expect(
      page.getByRole("button", { name: "Scan current network..." }),
    ).toBeDisabled();
    await page.waitForTimeout(250);
    expect(realProbeCount).toBe(0);

    await page.getByRole("link", { name: "Open CC Control" }).first().click();
    await expect(page).toHaveURL(/\/mock-001\/cc$/);
    await expect(
      page.getByRole("region", { name: "Mode and output" }),
    ).toBeVisible();
    await expect(page.getByText("profile mock")).toBeVisible();

    await page.goto("/devices?demo=false");
    await expect(page).toHaveURL(/\/devices$/);
    await expect(
      page.evaluate(() => window.localStorage.getItem("loadlynx.demoMode")),
    ).resolves.toBe("false");
  });
});
