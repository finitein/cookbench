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

  await bar.locator(`[data-stove-id="${second.id}"]`).getByTestId("stove").click();
  await expect(bar.locator(`[data-stove-id="${second.id}"]`)).not.toHaveClass(/stove-burner-wrap--alert/);
});

test("reduced motion keeps local alert emphasis static", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  const stove = stoveFixture(0);
  await driver.replaceStoves([stove]);
  await driver.flash(stove.id);

  const burner = page.locator(`[data-stove-id="${stove.id}"] .stove-burner`);
  await expect(burner).toHaveCSS("animation-name", "none");
  await expect(burner).not.toHaveCSS("box-shadow", "none");
});

test("minimal mode uses the canonical attention order rather than source order", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  const first = stoveFixture(0, "cooking");
  const attention = stoveFixture(1, "needsHuman");
  const third = stoveFixture(2, "failed");
  await driver.replaceSnapshot({
    stoves: [first, attention, third],
    attentionOrder: [attention.id, "unknown", attention.id, third.id],
    globalBarMode: "minimal",
  });

  const bar = page.getByTestId("minimal-global-bar");
  await expect(bar).toBeVisible();
  await expect(bar.getByTestId("stove")).toHaveCount(1);
  await expect(bar.locator(`[data-stove-id="${attention.id}"]`)).toBeVisible();

  await driver.replaceSnapshot({
    stoves: [first, attention, third],
    attentionOrder: [third.id, attention.id, first.id],
  });
  await expect(bar.locator(`[data-stove-id="${third.id}"]`)).toBeVisible();
  await expect(bar.locator(`[data-stove-id="${attention.id}"]`)).toHaveCount(0);
});

test("cooked acknowledgement preserves state, supplied order, and minimal preference after restart", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  const cooked = stoveFixture(0, "cooked");
  const active = stoveFixture(1, "cooking");
  await driver.replaceSnapshot({ stoves: [cooked, active], attentionOrder: [cooked.id, active.id], globalBarMode: "minimal" });
  await driver.acknowledgeCooked(cooked.id, [active.id, cooked.id]);
  await driver.restart();

  const bar = page.getByTestId("minimal-global-bar");
  await expect(bar.locator(`[data-stove-id="${active.id}"]`)).toBeVisible();
  await driver.setGlobalBarMode("full");
  await expect(page.locator(`[data-stove-id="${cooked.id}"]`).getByTestId("stove")).toHaveAttribute("data-state", "cooked");
  const orderedIds = await page.locator(".global-bar__item [data-stove-id]").evaluateAll((items) => items.map((item) => item.getAttribute("data-stove-id")));
  expect(orderedIds).toEqual([active.id, cooked.id]);
});

test("expanding minimal mode restores every stove in canonical order", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  const source = [stoveFixture(0), stoveFixture(1), stoveFixture(2)];
  await driver.replaceSnapshot({ stoves: source, attentionOrder: [source[2].id, source[0].id, source[1].id], globalBarMode: "minimal" });
  await page.getByTestId("minimal-global-bar").getByRole("button", { name: "Use full Bar" }).click();
  await expect(page.getByTestId("stove")).toHaveCount(3);
  const orderedIds = await page.locator(".global-bar__item [data-stove-id]").evaluateAll((items) => items.map((item) => item.getAttribute("data-stove-id")));
  expect(orderedIds).toEqual([source[2].id, source[0].id, source[1].id]);
});

test("mac-status fixture marker exposes only canonical selected IDs", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  const stoves = [stoveFixture(0), stoveFixture(1), stoveFixture(2), stoveFixture(3)];
  await driver.replaceSnapshot({ stoves, attentionOrder: [stoves[2].id, stoves[0].id, stoves[3].id, stoves[1].id] });
  await driver.setMacStatusFixture(true, 3);
  const marker = page.getByTestId("e2e-mac-status-fixture");
  await expect(marker).toHaveAttribute("data-available", "true");
  await expect(marker).toHaveAttribute("data-count", "3");
  await expect(marker).toHaveAttribute("data-stove-ids", [stoves[2].id, stoves[0].id, stoves[3].id].join(","));
});
