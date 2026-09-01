import { expect, test } from "@playwright/test";
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const cases = [
  ["01-overview.html", "overview"],
  ["02-one-glance.html", "concurrency"],
  ["03-catalog.html", "compatibility"],
  ["04-tiers.html", "tiers"],
  ["05-return.html", "return"],
  ["06-platforms.html", "platforms"],
  ["07-ssh.html", "ssh"],
  ["08-privacy.html", "privacy"],
  ["09-hooks.html", "hooks"],
  ["10-workflow.html", "workflow"],
  ["11-multibench.html", "multibench"],
  ["12-install.html", "install"],
  ["13-footprint.html", "footprint"],
  ["14-focus-surfaces.html", "focus-surfaces"],
] as const;

for (const [filename, topic] of cases) {
  test(`${filename} is an offline 1200x1500 composition without overflow`, async ({ page }) => {
    const url = pathToFileURL(resolve("docs/showcase", filename)).href;
    await page.goto(url);
    await expect(page.locator("main.poster")).toHaveAttribute("data-topic", topic);
    await expect(page.locator("h1")).toBeVisible();
    const metrics = await page.evaluate(() => ({
      width: document.documentElement.scrollWidth,
      height: document.documentElement.scrollHeight,
      bodyWidth: document.body.scrollWidth,
      bodyHeight: document.body.scrollHeight,
      remoteAssets: [...document.querySelectorAll("[src], [href]")].filter((node) => {
        const value = node.getAttribute("src") ?? node.getAttribute("href") ?? "";
        return /^https?:/i.test(value);
      }).length,
    }));
    expect(metrics).toEqual({
      width: 1200,
      height: 1500,
      bodyWidth: 1200,
      bodyHeight: 1500,
      remoteAssets: 0,
    });
  });
}

test("14-focus-surfaces social layout reserves a channel-safe 1080x1440 composition", async ({ page }) => {
  const url = pathToFileURL(resolve("docs/showcase", "14-focus-surfaces.html")).href;
  await page.setViewportSize({ width: 1080, height: 1440 });
  await page.goto(url);
  await page.locator("html").evaluate((node) => node.classList.add("social"));
  await expect(page.locator("main.poster")).toHaveAttribute("data-social-safe-zone", "72,120,1008,1320");
  const metrics = await page.evaluate(() => ({
    width: document.documentElement.scrollWidth,
    height: document.documentElement.scrollHeight,
    bodyWidth: document.body.scrollWidth,
    bodyHeight: document.body.scrollHeight,
  }));
  expect(metrics).toEqual({ width: 1080, height: 1440, bodyWidth: 1080, bodyHeight: 1440 });
});
