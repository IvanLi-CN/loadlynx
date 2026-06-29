import { expect, test } from "@playwright/test";

test("live control mode selector switches from CC to CV", async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("loadlynx.locale", "en");
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
    window.localStorage.setItem("loadlynx.demoMode", "true");
  });

  await page.goto("/devices?demo=true");

  const openDashboard = page
    .getByRole("link", { name: "Open Dashboard" })
    .first();
  await expect(openDashboard).toBeVisible();
  await openDashboard.click();
  await expect(page).toHaveURL(/\/mock-001\/cc$/);

  await expect(page.getByText("Target current (mA)")).toBeVisible();

  const modeSelector = page.getByRole("radiogroup", {
    name: "Live control mode selector",
  });
  await expect(modeSelector).toBeVisible();

  const cvOption = modeSelector.getByText("CV", { exact: true });
  await expect(cvOption).toBeVisible();
  await cvOption.click();

  await expect(page.getByText("Target voltage (mV)")).toBeVisible();
  await expect(page.getByText("Target current (mA)")).toHaveCount(0);
  await page.waitForTimeout(1000);
  await expect(page.getByText("Target voltage (mV)")).toBeVisible();
  await expect(page.getByText("Target current (mA)")).toHaveCount(0);
});

test("live control mode selector switches when clicking the visible CV segment", async ({
  page,
}) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("loadlynx.locale", "en");
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
    window.localStorage.setItem("loadlynx.demoMode", "true");
  });

  await page.goto("/devices?demo=true");

  const openDashboard = page
    .getByRole("link", { name: "Open Dashboard" })
    .first();
  await expect(openDashboard).toBeVisible();
  await openDashboard.click();
  await expect(page).toHaveURL(/\/mock-001\/cc$/);

  await expect(page.getByText("Target current (mA)")).toBeVisible();

  const cvSegmentBox = await page.evaluate(() => {
    const groups = Array.from(
      document.querySelectorAll('[aria-label="Live control mode selector"]'),
    );
    const group = groups[0];
    if (!group) {
      return null;
    }

    const segments = Array.from(
      group.querySelectorAll(".ll-slider-radio-group__item"),
    );
    const cvSegment = segments.find((segment) => {
      return segment.textContent?.trim() === "CV";
    });

    if (!cvSegment) {
      return null;
    }

    const rect = cvSegment.getBoundingClientRect();
    return {
      x: rect.x,
      y: rect.y,
      width: rect.width,
      height: rect.height,
    };
  });

  expect(cvSegmentBox).not.toBeNull();
  if (!cvSegmentBox) {
    throw new Error("Expected visible CV segment bounding box");
  }

  await page.mouse.click(
    cvSegmentBox.x + cvSegmentBox.width / 2,
    cvSegmentBox.y + cvSegmentBox.height / 2,
  );

  await expect(page.getByText("Target voltage (mV)")).toBeVisible();
  await expect(page.getByText("Target current (mA)")).toHaveCount(0);
  await page.waitForTimeout(1000);
  await expect(page.getByText("Target voltage (mV)")).toBeVisible();
  await expect(page.getByText("Target current (mA)")).toHaveCount(0);
});

test("dragging a live-control limit slider updates and keeps the dragged value", async ({
  page,
}) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("loadlynx.locale", "en");
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
    window.localStorage.setItem("loadlynx.demoMode", "true");
  });

  await page.goto("/devices?demo=true");

  const openDashboard = page
    .getByRole("link", { name: "Open Dashboard" })
    .first();
  await expect(openDashboard).toBeVisible();
  await openDashboard.click();
  await expect(page).toHaveURL(/\/mock-001\/cc$/);

  const maxCurrentInput = page.getByRole("textbox", {
    name: "Max current total (mA)",
  });
  await expect(maxCurrentInput).toHaveValue("4500");

  const maxCurrentSlider = page.getByRole("slider", {
    name: "Max current total (mA)",
  });
  await maxCurrentSlider.scrollIntoViewIfNeeded();

  const box = await maxCurrentSlider.boundingBox();
  expect(box).not.toBeNull();

  if (!box) {
    throw new Error("Expected max current slider bounding box");
  }

  await page.mouse.move(box.x + box.width - 8, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + box.width * 0.35, box.y + box.height / 2, {
    steps: 12,
  });
  await page.mouse.up();

  await page.waitForTimeout(900);

  const nextValue = Number(await maxCurrentInput.inputValue());
  expect(nextValue).toBeGreaterThan(0);
  expect(nextValue).toBeLessThan(4500);
});
