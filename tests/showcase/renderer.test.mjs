import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../..", import.meta.url);
const names = [
  "01-overview", "02-one-glance", "03-catalog", "04-tiers",
  "05-return", "06-platforms", "07-ssh", "08-privacy",
  "09-hooks", "10-workflow", "11-multibench", "12-install", "13-footprint",
];

test("renderer owns a deterministic thirteen-file 1200x1500 output contract", async () => {
  const renderer = await readFile(new URL("../../scripts/render-showcase.mjs", import.meta.url), "utf8");
  const packageJson = JSON.parse(await readFile(new URL("../../package.json", import.meta.url), "utf8"));
  assert.equal(packageJson.scripts["showcase:render"], "node scripts/render-showcase.mjs");
  for (const name of names) {
    assert.ok(renderer.includes(name), `renderer omits ${name}`);
    const png = await readFile(new URL(`../../docs/showcase/rendered/${name}.png`, import.meta.url));
    assert.equal(png.subarray(1, 4).toString(), "PNG");
    assert.equal(png.readUInt32BE(16), 1200);
    assert.equal(png.readUInt32BE(20), 1500);
  }
});

test("resource footprint claims stay tied to recorded macOS evidence", async () => {
  const source = await readFile(new URL("../../docs/showcase/13-footprint.html", import.meta.url), "utf8");
  const text = source.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ");
  assert.match(text, /约 90 MiB/);
  assert.match(text, /约 18 MiB/);
  assert.match(text, /macOS arm64/);
  assert.match(text, /不同平台与构建版本会有差异/);
});
