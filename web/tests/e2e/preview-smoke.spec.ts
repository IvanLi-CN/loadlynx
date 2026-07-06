import { expect, test } from "@playwright/test";

test("production preview mounts without runtime crashes @preview-smoke", async ({
  page,
}) => {
  const consoleErrors: string[] = [];
  const pageErrors: string[] = [];

  page.on("console", (message) => {
    if (message.type() === "error") {
      consoleErrors.push(message.text());
    }
  });
  page.on("pageerror", (error) => {
    pageErrors.push(error.message);
  });

  await page.goto("/");

  await expect(
    page.getByRole("link", { name: /Overview|总览/i }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: /Overview|总览/i }).first(),
  ).toBeVisible();
  await expect(pageErrors).toEqual([]);
  await expect(consoleErrors).toEqual([]);
});
