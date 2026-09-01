import { expect, test } from "@playwright/test";

import { e2eDriver, stoveFixture } from "./fixtures";

test("browser dock fixture collapses behind an exact 3px trigger and hover reveals it", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  await driver.replaceStoves([stoveFixture(0)]);
  await driver.setDockState("dockedCollapsed");

  const shell = page.getByLabel("Cookbench E2E presentation");
  const trigger = page.getByTestId("e2e-dock-trigger");
  await expect(shell).toHaveAttribute("data-dock-phase", "dockedCollapsed");
  await expect(trigger).toHaveCSS("height", "3px");
  await expect(page.getByLabel(/Cookbench global bar/)).not.toBeVisible();
  await trigger.hover();
  await expect(shell).toHaveAttribute("data-dock-phase", "dockedExpanded");
  await expect(page.getByLabel(/Cookbench global bar/)).toBeVisible();
});

test("browser dock fixture supports explicit undock and retains best-effort expansion", async ({ page }) => {
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  await driver.replaceStoves([stoveFixture(0)]);
  await driver.setDockState("dockedExpanded", true);
  const shell = page.getByLabel("Cookbench E2E presentation");
  await page.mouse.move(0, 0);
  await page.waitForTimeout(650);
  await expect(shell).toHaveAttribute("data-dock-phase", "dockedExpanded");
  await expect(shell).toHaveAttribute("data-dock-best-effort", "true");

  await page.getByRole("button", { name: "Undock fixture" }).click();
  await expect(shell).toHaveAttribute("data-dock-phase", "undocked");
  await expect(shell).toHaveAttribute("data-dock-best-effort", "false");
});

test("reduced-motion dock fixture has no presentation transitions", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto(process.env.COOKBENCH_E2E_URL ?? "http://127.0.0.1:1420");
  const driver = await e2eDriver(page);
  await driver.replaceStoves([stoveFixture(0)]);
  await driver.setDockState("dockedCollapsed");
  const trigger = page.getByTestId("e2e-dock-trigger");
  await expect(trigger).toHaveCSS("transition-duration", "0s");
});
