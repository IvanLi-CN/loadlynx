import { expect, test } from "@playwright/test";

test("dashboard live control top controls stay aligned and the load switch is distinct", async ({
  page,
}) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("loadlynx.locale", "en");
  });

  await page.goto("/devices?demo=true");

  const openDashboard = page.locator('a[href$="/mock-001/cc"]').first();
  await expect(openDashboard).toBeVisible();
  await openDashboard.click();

  await expect(page).toHaveURL(/\/mock-001\/cc$/);

  const modeSelector = page.getByRole("radiogroup", {
    name: "Live control mode selector",
  });
  await expect(modeSelector).toBeVisible();
  const loadSwitch = page.getByRole("switch", {
    name: "Load output switch",
  });
  await expect(loadSwitch).toBeVisible();

  const [modeBounds, switchBounds] = await Promise.all([
    modeSelector.boundingBox(),
    loadSwitch.boundingBox(),
  ]);
  expect(modeBounds).not.toBeNull();
  expect(switchBounds).not.toBeNull();
  expect(modeBounds?.height).toBeLessThan(80);
  expect(switchBounds?.height).toBeLessThan(80);
  expect(
    Math.abs((modeBounds?.height ?? 0) - (switchBounds?.height ?? 0)),
  ).toBeLessThanOrEqual(6);
});
