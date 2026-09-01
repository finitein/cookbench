import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../..", import.meta.url);
const names = [
  "01-overview", "02-one-glance", "03-catalog", "04-tiers",
  "05-return", "06-platforms", "07-ssh", "08-privacy",
  "09-hooks", "10-workflow", "11-multibench", "12-install", "13-footprint", "14-focus-surfaces",
];

test("renderer owns a deterministic fourteen-file 1200x1500 output contract", async () => {
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
  const social = await readFile(new URL("../../docs/showcase/rendered/14-focus-surfaces-social.png", import.meta.url));
  assert.equal(social.subarray(1, 4).toString(), "PNG");
  assert.equal(social.readUInt32BE(16), 1080);
  assert.equal(social.readUInt32BE(20), 1440);
});

test("resource footprint claims stay tied to recorded macOS evidence", async () => {
  const source = await readFile(new URL("../../docs/showcase/13-footprint.html", import.meta.url), "utf8");
  const text = source.replace(/<[^>]+>/g, " ").replace(/\s+/g, " ");
  assert.match(text, /约 90 MiB/);
  assert.match(text, /约 18 MiB/);
  assert.match(text, /macOS arm64/);
  assert.match(text, /不同平台与构建版本会有差异/);
});

test("focus showcase keeps its attention order and social-safe-zone claims inspectable", async () => {
  const source = await readFile(new URL("../../docs/showcase/14-focus-surfaces.html", import.meta.url), "utf8");
  const readme = await readFile(new URL("../../docs/showcase/README.md", import.meta.url), "utf8");
  assert.match(source, /data-social-safe-zone="72,120,1008,1320"/);
  assert.match(source, /Needs Human[\s\S]*Failed[\s\S]*Disconnected[\s\S]*未确认 Cooked/);
  assert.match(readme, /1080×1440/);
  assert.match(readme, /x=72\.\.1008、y=120\.\.1320/);
});
