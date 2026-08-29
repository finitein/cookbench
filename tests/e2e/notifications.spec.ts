import { expect, test } from "@playwright/test";

import { e2eDriver, stoveFixture } from "./fixtures";

test("SSH disconnection never becomes Cooked and filtered outbound notifications remain outbound-only", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  await driver.replaceStoves([
    stoveFixture(0, "disconnected", { host: { kind: "ssh", id: "fixture-ssh" } }),
  ]);

  await expect(page.getByTestId("stove")).toHaveAttribute("data-state", "disconnected");
  await expect(page.getByTestId("progress-ring")).toHaveAttribute("data-ring-mode", "complete");
  const notifications = await driver.notifications();
  expect(notifications).toContainEqual({ destination: "e2e-enabled", event: "disconnected" });
  expect(notifications).not.toContainEqual({ destination: "e2e-filtered", event: "disconnected" });
});
