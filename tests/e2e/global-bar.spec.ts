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
