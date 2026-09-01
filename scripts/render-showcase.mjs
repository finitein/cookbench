import { chromium } from "@playwright/test";
import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const root = dirname(fileURLToPath(new URL("../package.json", import.meta.url)));
const source = join(root, "docs", "showcase");
const output = join(source, "rendered");
const names = [
  "01-overview",
  "02-one-glance",
  "03-catalog",
  "04-tiers",
  "05-return",
  "06-platforms",
  "07-ssh",
  "08-privacy",
  "09-hooks",
  "10-workflow",
  "11-multibench",
  "12-install",
  "13-footprint",
  "14-focus-surfaces",
];
const socialName = "14-focus-surfaces";

await mkdir(output, { recursive: true });
const browser = await chromium.launch();
try {
  const context = await browser.newContext({
    viewport: { width: 1200, height: 1500 },
    deviceScaleFactor: 1,
    reducedMotion: "reduce",
  });
  const page = await context.newPage();
  for (const name of names) {
    await page.setViewportSize({ width: 1200, height: 1500 });
    await page.goto(pathToFileURL(join(source, `${name}.html`)).href, { waitUntil: "load" });
    await page.evaluate(() => document.fonts.ready);
    const dimensions = await page.evaluate(() => ({
      width: document.documentElement.scrollWidth,
      height: document.documentElement.scrollHeight,
    }));
    if (dimensions.width !== 1200 || dimensions.height !== 1500) {
      throw new Error(`${name} rendered at ${dimensions.width}x${dimensions.height}`);
    }
    await page.screenshot({
      path: join(output, `${name}.png`),
      animations: "disabled",
      caret: "hide",
    });

    if (name === socialName) {
      await page.setViewportSize({ width: 1080, height: 1440 });
      await page.evaluate(() => document.documentElement.classList.add("social"));
      const socialDimensions = await page.evaluate(() => ({
        width: document.documentElement.scrollWidth,
        height: document.documentElement.scrollHeight,
      }));
      if (socialDimensions.width !== 1080 || socialDimensions.height !== 1440) {
        throw new Error(`${name} social rendered at ${socialDimensions.width}x${socialDimensions.height}`);
      }
      await page.screenshot({
        path: join(output, `${name}-social.png`),
        animations: "disabled",
        caret: "hide",
      });
    }
  }
  await context.close();
} finally {
  await browser.close();
}
