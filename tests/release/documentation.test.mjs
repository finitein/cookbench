import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../..", import.meta.url);
const read = async (path) => readFile(new URL(path, root), "utf8");

const readmes = ["README.md", "README.zh-CN.md", "README.ja.md", "README.ko.md"];

test("all language landing pages share the compatibility and safety contract", async () => {
  for (const path of readmes) {
    const contents = await read(path);
    assert.match(contents, /27/);
    assert.match(contents, /docs\/harness-compatibility\.md/);
    assert.match(contents, /Full/i);
    assert.match(contents, /Standard/i);
    assert.match(contents, /Experimental/i);
    assert.match(contents, /install\.sh/);
    assert.match(contents, /install\.ps1/);
    assert.match(contents, /docs\/showcase\/README\.md/);
    assert.match(contents, /SQLite/i);
    assert.match(contents, /control/i);
  }
});

test("language navigation is complete and reciprocal", async () => {
  for (const path of readmes) {
    const contents = await read(path);
    for (const target of readmes) {
      assert.ok(contents.includes(target), `${path} does not link ${target}`);
    }
  }
});

test("canonical compatibility document lists every catalog id exactly once", async () => {
  const catalog = await read("crates/cookbench-adapters/src/catalog.rs");
  const compatibility = await read("docs/harness-compatibility.md");
  const ids = [...catalog.matchAll(/profile!\(\s*"([a-z0-9_]+)"/g)].map((match) => match[1]);
  assert.equal(ids.length, 27);
  for (const id of ids) {
    const count = compatibility.match(new RegExp("\\| `" + id + "` \\|", "g"))?.length ?? 0;
    assert.equal(count, 1, `${id} must appear in exactly one compatibility row`);
  }
  assert.match(compatibility, /absence of activity.+never.+Cooked/is);
  assert.match(compatibility, /WorkBuddy.+presence.+only/is);
});
