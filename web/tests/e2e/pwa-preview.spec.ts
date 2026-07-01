import { expect, test } from "@playwright/test";

test("PWA app shell reloads offline without caching API responses @preview-smoke", async ({
  context,
  page,
}) => {
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: /LoadLynx Web Console/i, level: 1 }),
  ).toBeVisible();

  await page.waitForFunction(async () => {
    if (!("serviceWorker" in navigator)) return false;
    await navigator.serviceWorker.ready;
    return true;
  });

  // Reload once while online so the installed worker controls the page.
  await page.reload();
  await page.waitForFunction(() => navigator.serviceWorker.controller !== null);
  await expect(
    page.getByRole("heading", { name: /LoadLynx Web Console/i, level: 1 }),
  ).toBeVisible();

  await context.setOffline(true);
  await page.reload();

  await expect(
    page.getByRole("heading", { name: /LoadLynx Web Console/i, level: 1 }),
  ).toBeVisible();

  const apiFetch = await page.evaluate(async () => {
    try {
      const response = await fetch("/api/v1/status", { cache: "no-store" });
      return { fetched: true, status: response.status };
    } catch (error) {
      return {
        fetched: false,
        message: error instanceof Error ? error.message : String(error),
      };
    }
  });

  expect(apiFetch).toMatchObject({ fetched: false });

  const versionFetch = await page.evaluate(async () => {
    try {
      const response = await fetch("/version.json", { cache: "no-store" });
      return { fetched: true, status: response.status };
    } catch (error) {
      return {
        fetched: false,
        message: error instanceof Error ? error.message : String(error),
      };
    }
  });

  expect(versionFetch).toMatchObject({ fetched: false });

  await context.setOffline(false);
});
