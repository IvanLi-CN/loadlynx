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

async function setOutputEnabled(page: Page, enabled: boolean) {
  const outputToggle = page.getByRole("checkbox", { name: "Output enabled" });
  await expect(outputToggle).toBeEnabled();

  const wasChecked = await outputToggle.isChecked();
  if (wasChecked !== enabled) {
    await outputToggle.click();
  }

  await expect(page.getByTestId("control-output-enabled")).toContainText(
    enabled ? "true" : "false",
  );
}

async function expandAdvanced(page: Page) {
  const btn = page.getByRole("button", { name: /Advanced/i }).first();
  await btn.scrollIntoViewIfNeeded();
  await btn.click();
  await expect(page.locator("#preset-mode")).toBeVisible();
}

test.describe("Presets + Unified Control (mock://)", () => {
  test.beforeEach(async ({ page }) => {
    await openFirstDeviceControl(page);
  });

  test("presets list loads and has 5 entries", async ({ page }) => {
    const presets = page.getByRole("region", { name: "Presets" });
    await expect(presets.getByRole("button", { name: "#1" })).toBeEnabled();
    await expect(presets.getByRole("button", { name: "#5" })).toBeEnabled();
    await expect(presets.getByRole("button", { name: "#6" })).toBeDisabled();
  });

  test("apply preset forces output_enabled=false", async ({ page }) => {
    await setOutputEnabled(page, true);

    const presets = page.getByRole("region", { name: "Presets" });
    await presets.getByRole("button", { name: "#2" }).click();
    await presets.getByRole("button", { name: "Apply Preset" }).click();

    await expect(page.getByTestId("control-active-preset")).toContainText("2");
    await expect(page.getByTestId("control-output-enabled")).toContainText(
      "false",
    );
  });

  test("toggling output changes output_enabled", async ({ page }) => {
    await setOutputEnabled(page, false);
    await setOutputEnabled(page, true);
  });

  test("switching a preset CC<->CV and applying reflects in ControlView", async ({
    page,
  }) => {
    const presets = page.getByRole("region", { name: "Presets" });
    await presets.getByRole("button", { name: "#3" }).click();

    await expandAdvanced(page);
    await page.locator("#preset-mode").selectOption("cv");
    await page.locator("#preset-target-v").fill("15000");
    await page.locator("#preset-min-v").fill("0");
    await page.locator("#preset-max-i").fill("8000");
    await page.locator("#preset-max-p").fill("120000");
    await presets.getByRole("button", { name: "Save Draft" }).click();
    await presets.getByRole("button", { name: "Apply Preset" }).click();

    await expect(page.getByTestId("control-active-preset")).toContainText("3");
    await expect(page.getByTestId("control-active-mode")).toContainText("cv");

    await page.locator("#preset-mode").selectOption("cc");
    await page.locator("#preset-target-i").fill("2500");
    await presets.getByRole("button", { name: "Save Draft" }).click();
    await presets.getByRole("button", { name: "Apply Preset" }).click();

    await expect(page.getByTestId("control-active-mode")).toContainText("cc");
  });

  test("switching a preset to CP and applying reflects in ControlView", async ({
    page,
  }) => {
    const presets = page.getByRole("region", { name: "Presets" });
    await presets.getByRole("button", { name: "#4" }).click();

    await expandAdvanced(page);
    await page.locator("#preset-mode").selectOption("cp");
    await page.locator("#preset-target-p").fill("25000");
    await page.locator("#preset-min-v").fill("0");
    await page.locator("#preset-max-i").fill("8000");
    await page.locator("#preset-max-p").fill("120000");
    await presets.getByRole("button", { name: "Save Draft" }).click();
    await presets.getByRole("button", { name: "Apply Preset" }).click();

    await expect(page.getByTestId("control-active-preset")).toContainText("4");
    await expect(page.getByTestId("control-active-mode")).toContainText("cp");
  });
});
