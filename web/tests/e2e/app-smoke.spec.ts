import { expect, test } from "@playwright/test";

test("app loads and shows scaffold title", async ({ page }) => {
  await page.goto("/");

  await expect(
    page.getByRole("heading", { name: /LoadLynx Web Console/i, level: 1 }),
  ).toBeVisible();
});
