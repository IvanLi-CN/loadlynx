import { expect, type Page, test } from "@playwright/test";

async function openFirstDeviceControl(page: Page) {
  await page.addInitScript(() => {
    window.localStorage.setItem("loadlynx.locale", "zh-CN");
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
  await page.goto("/devices?demo=true");
  const demoDeviceCard = page
    .getByRole("article")
    .filter({ hasText: "mock-001" });
  await expect(
    demoDeviceCard.getByRole("heading", { name: "Demo Device #1" }),
  ).toBeVisible();

  const openControlBtn = demoDeviceCard.getByRole("link", {
    name: "打开仪表盘",
  });

  await expect(openControlBtn).toBeVisible();
  await openControlBtn.click();
  await expect(page.url()).toContain("/cc");
}

async function openPresetDrawer(page: Page) {
  const trigger = page.getByRole("button", { name: /预设|Presets/ }).first();
  await expect(trigger).toBeVisible();
  await trigger.click();
  await expect(
    page.getByRole("dialog", { name: /预设|Presets/ }),
  ).toBeVisible();
}

function getPresetDrawer(page: Page) {
  return page.getByRole("dialog", { name: /预设|Presets/ });
}

async function setOutputEnabled(page: Page, enabled: boolean) {
  const outputToggle = page.getByRole("switch", {
    name: /Load output switch|负载主开关/,
  });
  await expect(outputToggle).toBeEnabled();

  const wasChecked =
    (await outputToggle.getAttribute("aria-checked")) === "true";
  if (wasChecked !== enabled) {
    await outputToggle.click();
  }

  await expect(page.getByTestId("control-output-enabled")).toContainText(
    enabled ? "true" : "false",
  );
}

test.describe("Presets + Unified Control (mock://)", () => {
  test.beforeEach(async ({ page }) => {
    await openFirstDeviceControl(page);
  });

  test("presets list loads and has 5 entries", async ({ page }) => {
    await openPresetDrawer(page);
    const drawer = getPresetDrawer(page);
    await expect(drawer.getByRole("button", { name: "#1" })).toBeEnabled();
    await expect(drawer.getByRole("button", { name: "#5" })).toBeEnabled();
    await expect(drawer.getByRole("button", { name: "#6" })).toBeDisabled();
  });

  test("apply preset forces output_enabled=false", async ({ page }) => {
    await setOutputEnabled(page, true);

    await openPresetDrawer(page);
    const drawer = getPresetDrawer(page);
    await drawer.getByRole("button", { name: "#2" }).click();
    await drawer.getByRole("button", { name: /Apply Preset|应用预设/ }).click();
    await expect(drawer.getByText("已应用槽位 #2，输出已关闭。")).toBeVisible();

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
    await openPresetDrawer(page);
    const drawer = getPresetDrawer(page);
    await drawer.getByRole("button", { name: "#3" }).click();

    await drawer.getByLabel("CV").check({ force: true });
    await drawer
      .getByRole("textbox", { name: "Target voltage (mV)" })
      .fill("15000");
    await drawer.getByRole("textbox", { name: "Min voltage (mV)" }).fill("0");
    await drawer
      .getByRole("textbox", { name: "Max current total (mA)" })
      .fill("8000");
    await drawer
      .getByRole("textbox", { name: "Max power (mW)" })
      .fill("120000");
    await drawer.getByRole("button", { name: /Save Slot|保存槽位/ }).click();
    await expect(drawer.getByText("已保存到槽位 #3。")).toBeVisible();
    await drawer.getByRole("button", { name: /Apply Preset|应用预设/ }).click();
    await expect(drawer.getByText("已应用槽位 #3，输出已关闭。")).toBeVisible();

    await expect(page.getByTestId("control-active-preset")).toContainText("3");
    await expect(page.getByTestId("control-active-mode")).toContainText("cv");

    await drawer.getByLabel("CC").check({ force: true });
    await drawer
      .getByRole("textbox", { name: "Target current (mA)" })
      .fill("2500");
    await drawer
      .getByRole("button", { name: /Save Active Slot|保存当前激活槽位/ })
      .click();
    await expect(
      drawer.getByText("已保存槽位 #3，实时控制已同步更新。"),
    ).toBeVisible();

    await expect(page.getByTestId("control-active-mode")).toContainText("cc");
  });

  test("switching a preset to CP and applying reflects in ControlView", async ({
    page,
  }) => {
    await openPresetDrawer(page);
    const drawer = getPresetDrawer(page);
    await drawer.getByRole("button", { name: "#4" }).click();

    await drawer.getByLabel("CP").check({ force: true });
    await drawer
      .getByRole("textbox", { name: "Target power (mW)" })
      .fill("25000");
    await drawer.getByRole("textbox", { name: "Min voltage (mV)" }).fill("0");
    await drawer
      .getByRole("textbox", { name: "Max current total (mA)" })
      .fill("8000");
    await drawer
      .getByRole("textbox", { name: "Max power (mW)" })
      .fill("120000");
    await drawer.getByRole("button", { name: /Save Slot|保存槽位/ }).click();
    await expect(drawer.getByText("已保存到槽位 #4。")).toBeVisible();
    await drawer.getByRole("button", { name: /Apply Preset|应用预设/ }).click();
    await expect(drawer.getByText("已应用槽位 #4，输出已关闭。")).toBeVisible();

    await expect(page.getByTestId("control-active-preset")).toContainText("4");
    await expect(page.getByTestId("control-active-mode")).toContainText("cp");
  });
});
