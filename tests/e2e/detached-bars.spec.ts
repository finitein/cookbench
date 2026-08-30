import { expect, test } from "@playwright/test";

import { e2eDriver, stoveFixture } from "./fixtures";

test("a detached stove is restored with the global Bar and is removed when cleared", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  const stove = stoveFixture(0, "cooked");
  await driver.replaceStoves([stove]);

  await driver.detach(stove.id);
  const detached = page.getByLabel(/Detached Stove bar for Codex/);
  await expect(detached).toBeVisible();
  await expect(detached.locator(".detached-stove-bar__harness .harness-mark")).toHaveText("CX");
  await expect(detached).not.toContainText("Codex");
  await expect(detached.getByRole("button", { name: "Close detached Stove" })).toBeVisible();
  const overflow = await detached.evaluate((element) => ({
    x: getComputedStyle(element).overflowX,
    y: getComputedStyle(element).overflowY,
    width: element.scrollWidth - element.clientWidth,
    height: element.scrollHeight - element.clientHeight,
  }));
  expect(overflow).toEqual({ x: "hidden", y: "hidden", width: 0, height: 0 });
  if (process.env.COOKBENCH_CAPTURE_EVIDENCE === "1") {
    await detached.screenshot({
      path: "docs/verification/evidence/e2e-detached-stove.png",
    });
  }
  await driver.moveDetached(stove.id, 180, 120);
  await driver.restoreDetached();
  await expect(page.getByLabel(/Detached Stove bar for Codex/)).toBeVisible();
  await expect(page.getByTestId(`detached-window-${stove.id}`)).toHaveCSS("left", "180px");
  await expect(page.getByTestId(`detached-window-${stove.id}`)).toHaveCSS("top", "120px");

  await driver.clear(stove.id);
  await expect(page.getByLabel(/Detached Stove bar for Codex/)).toHaveCount(0);
});
