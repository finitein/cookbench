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

test("dense harness activity becomes named wrapping benches without scroll containers", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  await page.evaluate(() => { document.documentElement.dataset.cookbenchNative = "true"; });
  const driver = await e2eDriver(page);
  await driver.replaceStoves([
    stoveFixture(0),
    stoveFixture(3),
    stoveFixture(6),
    stoveFixture(9),
    stoveFixture(1),
    stoveFixture(2),
  ]);

  const bar = page.getByLabel(/Cookbench global bar with 6 stoves/);
  await bar.evaluate((element) => { (element as HTMLElement).style.minHeight = "600px"; });
  await expect(bar).toHaveAttribute("data-layout", "grouped");
  await expect(bar.getByRole("region", { name: "Codex" })).toBeVisible();
  await expect(bar.getByRole("region", { name: "Claude Code" })).toBeVisible();
  await expect(bar.getByRole("region", { name: "Pi" })).toBeVisible();
  await expect(bar.getByTestId("stove")).toHaveCount(6);

  const scrollContainers = await bar.locator(".global-bar__bench-stoves").evaluateAll((benches) =>
    benches.filter((bench) => {
      const style = getComputedStyle(bench);
      return style.overflowX !== "visible" || style.overflowY !== "visible" || bench.scrollWidth > bench.clientWidth || bench.scrollHeight > bench.clientHeight;
    }).length,
  );
  expect(scrollContainers).toBe(0);
  const [stoveExtent, brandBox] = await Promise.all([
    bar.locator(".global-bar__item").evaluateAll((items) => {
      const boxes = items.map((item) => item.getBoundingClientRect());
      return { top: Math.min(...boxes.map((box) => box.top)), bottom: Math.max(...boxes.map((box) => box.bottom)) };
    }),
    bar.locator(".global-bar__brand").evaluate((element) => element.getBoundingClientRect().toJSON()),
  ]);
  expect(Math.abs((brandBox.y + brandBox.height / 2) - ((stoveExtent.top + stoveExtent.bottom) / 2))).toBeLessThan(2);
  if (process.env.COOKBENCH_CAPTURE_EVIDENCE === "1") {
    await page.screenshot({
      path: "docs/verification/evidence/e2e-grouped-benches.png",
      fullPage: true,
    });
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
  if (process.env.COOKBENCH_CAPTURE_EVIDENCE === "1") {
    await page.screenshot({
      path: "docs/verification/evidence/e2e-hover-detail.png",
      fullPage: true,
    });
  }

  await page.mouse.move(0, 0);
  await expect(tooltip).toHaveCount(0);
});

test("a local alert emphasizes only its named Stove without changing layout", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  const first = stoveFixture(0);
  const second = stoveFixture(1);
  await driver.replaceStoves([first, second]);

  const bar = page.getByLabel(/Cookbench global bar with 2 stoves/);
  const originalHeight = await bar.evaluate((element) => element.clientHeight);
  await driver.flash(second.id);

  await expect(bar.locator(`[data-stove-id="${second.id}"]`)).toHaveClass(/stove-burner-wrap--alert/);
  await expect(bar.locator(`[data-stove-id="${first.id}"]`)).not.toHaveClass(/stove-burner-wrap--alert/);
  await expect(bar).toHaveJSProperty("clientHeight", originalHeight);
});
