import { expect, test } from "@playwright/test";

import { allStateFixtures, e2eDriver } from "./fixtures";

const scenarios = [
  { name: "desktop-light", width: 1280, height: 720, dark: false, reducedMotion: false },
  { name: "desktop-dark-reduced", width: 1440, height: 900, dark: true, reducedMotion: true },
  { name: "retina-effective", width: 1280, height: 720, dark: false, reducedMotion: true },
  { name: "narrow", width: 390, height: 844, dark: false, reducedMotion: false },
] as const;

test("empty Bar fills the user-sized white surface and retains the Cookbench mark", async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto("/");
  const driver = await e2eDriver(page);
  await driver.replaceStoves([]);

  const bar = page.getByRole("region", { name: "Cookbench global bar with 0 stoves" });
  await expect(bar).toHaveClass(/global-bar--empty/);
  await expect(page.getByRole("img", { name: "Cookbench" })).toBeVisible();
  const bounds = await bar.boundingBox();
  expect(bounds).not.toBeNull();
  expect(bounds!.width).toBeGreaterThanOrEqual(1_200);
  expect(bounds!.height).toBeLessThanOrEqual(100);

  if (process.env.COOKBENCH_CAPTURE_EVIDENCE === "1") {
    await page.screenshot({
      path: "docs/verification/evidence/e2e-empty-bar.png",
      fullPage: true,
    });
  }
});

for (const scenario of scenarios) {
  test(`visual layout remains bounded for ${scenario.name}`, async ({ page }) => {
    await page.setViewportSize({ width: scenario.width, height: scenario.height });
    await page.emulateMedia({
      colorScheme: scenario.dark ? "dark" : "light",
      reducedMotion: scenario.reducedMotion ? "reduce" : "no-preference",
    });
    await page.goto("/");
    await page.evaluate((dark) => {
      document.body.style.background = dark ? "#171a1f" : "#f4f7fb";
    }, scenario.dark);
    const driver = await e2eDriver(page);
    await driver.replaceStoves(allStateFixtures());

    await expect(page.getByTestId("stove")).toHaveCount(7);
    const overflow = await page.evaluate(() => {
      const viewport = { width: window.innerWidth, height: window.innerHeight };
      const elements = [...document.querySelectorAll<HTMLElement>(".global-bar, .stove-burner")]
        .map((element) => ({
          className: element.className,
          rect: element.getBoundingClientRect(),
          intentionallyScrolled: element.matches(".stove-burner")
            && (element.closest(".global-bar__stoves")?.scrollWidth ?? 0)
              > (element.closest(".global-bar__stoves")?.clientWidth ?? 0),
        }))
        .filter(({ rect, intentionallyScrolled }) =>
          !intentionallyScrolled
          && (rect.left < 0 || rect.top < 0 || rect.right > viewport.width || rect.bottom > viewport.height)
        )
        .map(({ className }) => className);
      if (document.documentElement.scrollWidth > viewport.width)
        elements.push("document-horizontal-overflow");
      return elements;
    });
    expect(overflow).toEqual([]);

    if (scenario.reducedMotion) {
      const animationNames = await page.getByTestId("progress-ring").evaluateAll((rings) =>
        rings.map((ring) => getComputedStyle(ring.querySelector(".progress-ring__value")!).animationName),
      );
      expect(animationNames.every((name) => name === "none")).toBe(true);
    }

    if (process.env.COOKBENCH_CAPTURE_EVIDENCE === "1") {
      await page.screenshot({
        path: `docs/verification/evidence/e2e-${scenario.name}.png`,
        fullPage: true,
      });
    }
  });
}
