import { expect, type Page, test } from "@playwright/test";

async function openFirstDeviceControl(page: Page) {
  await page.goto("/");
  await expect(page.locator("text=LoadLynx Web Console")).toBeVisible();

  const openControlBtn = page.locator("text=Open CC Control").first();

  if ((await openControlBtn.count()) === 0) {
    const addDemoBtn = page.locator("text=Add demo device");
    if (await addDemoBtn.isVisible()) {
      await addDemoBtn.click();
    }
  }

  await expect(openControlBtn).toBeVisible();
  await openControlBtn.click();
  await expect(page.url()).toContain("/cc");
}

test.describe("Presets + Unified Control (mock://)", () => {
  test.beforeEach(async ({ page }) => {
    await openFirstDeviceControl(page);
  });

  test("presets list loads and has 5 entries", async ({ page }) => {
    await expect(page.getByTestId("preset-row")).toHaveCount(5);
  });

  test("apply preset forces output_enabled=false", async ({ page }) => {
    const outputToggle = page.getByRole("checkbox", { name: "Output enabled" });
    await outputToggle.check();
    await expect(page.getByTestId("control-output-enabled")).toContainText(
      "true",
    );

    await page.getByRole("button", { name: "#2" }).click();
    await page
      .getByRole("button", { name: "Apply preset (forces output off)" })
      .click();

    await expect(page.getByTestId("control-active-preset")).toContainText("2");
    await expect(page.getByTestId("control-output-enabled")).toContainText(
      "false",
    );
  });

  test("toggling output changes output_enabled", async ({ page }) => {
    const outputToggle = page.getByRole("checkbox", { name: "Output enabled" });

    await outputToggle.uncheck();
    await expect(page.getByTestId("control-output-enabled")).toContainText(
      "false",
    );

    await outputToggle.check();
    await expect(page.getByTestId("control-output-enabled")).toContainText(
      "true",
    );
  });

  test("switching a preset CC<->CV and applying reflects in ControlView", async ({
    page,
  }) => {
    await page.getByRole("button", { name: "#3" }).click();

    await page.locator("#preset-mode").selectOption("cv");
    await page.locator("#preset-target-v").fill("15000");
    await page.locator("#preset-min-v").fill("0");
    await page.locator("#preset-max-i").fill("8000");
    await page.locator("#preset-max-p").fill("120000");
    await page.getByRole("button", { name: "Save preset" }).click();

    await page
      .getByRole("button", { name: "Apply preset (forces output off)" })
      .click();

    await expect(page.getByTestId("control-active-preset")).toContainText("3");
    await expect(page.getByTestId("control-active-mode")).toContainText("cv");

    await page.locator("#preset-mode").selectOption("cc");
    await page.locator("#preset-target-i").fill("2500");
    await page.getByRole("button", { name: "Save preset" }).click();
    await page
      .getByRole("button", { name: "Apply preset (forces output off)" })
      .click();

    await expect(page.getByTestId("control-active-mode")).toContainText("cc");
  });
});
