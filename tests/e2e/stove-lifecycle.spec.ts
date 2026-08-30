import { expect, test } from "@playwright/test";

import { allStateFixtures, e2eDriver, stoveFixture } from "./fixtures";

test("retains Cooked across restart, relights on a prompt, and clears only Cookbench state", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  const cooked = stoveFixture(0, "cooked");
  await driver.replaceStoves([cooked]);
  await expect(page.getByTestId("stove")).toHaveAttribute("data-state", "cooked");

  await driver.restart();
  await expect(page.getByTestId("stove")).toHaveAttribute("data-state", "cooked");

  await driver.replaceStoves([{ ...cooked, state: "cooking", retainedCompletion: false, progress: null }]);
  await expect(page.getByTestId("stove")).toHaveAttribute("data-state", "cooking");
  await expect(page.getByTestId("progress-ring")).toHaveAttribute("data-ring-mode", "indeterminate");

  await driver.clear(cooked.id);
  await expect(page.getByTestId("stove")).toHaveCount(0);
});

test("uses complete terminal rings and a determinate arc only with structured Cooking progress", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  await driver.replaceStoves(allStateFixtures());

  const cookingRing = page.locator('[data-testid="stove"][data-state="cooking"] [data-testid="progress-ring"]');
  await expect(cookingRing).toHaveAttribute("data-ring-mode", "determinate");
  await expect(cookingRing).toHaveAttribute("data-progress", "40");
  for (const state of ["needsHuman", "cooked", "failed", "disconnected"]) {
    const ring = page.locator(`[data-testid="stove"][data-state="${state}"] [data-testid="progress-ring"]`);
    await expect(ring).toHaveAttribute("data-ring-mode", "complete");
    await expect(ring).not.toHaveAttribute("data-progress", /.+/);
  }
});
