import { expect, test } from "@playwright/test";

test("app loads and shows the console shell", async ({ page }) => {
  await page.goto("/");

  await expect(
    page.getByRole("link", { name: /Overview|总览/i }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: /Overview|总览/i }).first(),
  ).toBeVisible();
});
