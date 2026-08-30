import { expect, test } from "@playwright/test";

import { E2E_HARNESSES, e2eDriver, stoveFixture } from "./fixtures";

test("global Bar presents sessions from Codex, Claude Code, and Pi together", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  await driver.replaceStoves(E2E_HARNESSES.map((_, index) => stoveFixture(index)));

  const bar = page.getByLabel(/Cookbench global bar with 3 stoves/);
  await expect(bar).toBeVisible();
  await expect(bar.getByTestId("stove")).toHaveCount(3);
  for (const harness of E2E_HARNESSES) {
    await expect(
      bar.locator(`[data-testid="stove"][aria-label^="${harness.label}:"]`),
    ).toBeVisible();
  }
});

test("hovered Stove details stay inside the rendered Bar and close when the pointer leaves", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  await driver.replaceStoves([stoveFixture(0)]);

  const bar = page.getByLabel(/Cookbench global bar with 1 stoves/);
  const stove = bar.getByTestId("stove");
  await stove.hover();

  const tooltip = page.getByRole("tooltip");
  await expect(tooltip).toBeVisible();
  await expect(tooltip).toContainText("Session");
  await expect(bar).toHaveClass(/global-bar--tooltip-open/);
  await page.waitForTimeout(50);
  const [barBounds, tooltipBounds] = await Promise.all([
    bar.evaluate((element) => element.getBoundingClientRect().toJSON()),
    tooltip.evaluate((element) => element.getBoundingClientRect().toJSON()),
  ]);
  expect(tooltipBounds.x).toBeGreaterThanOrEqual(barBounds.x);
  expect(tooltipBounds.x + tooltipBounds.width).toBeLessThanOrEqual(barBounds.x + barBounds.width);
  expect(tooltipBounds.y + tooltipBounds.height).toBeLessThanOrEqual(barBounds.y + barBounds.height);

  await page.mouse.move(0, 0);
  await expect(tooltip).toHaveCount(0);
});
