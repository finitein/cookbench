import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const root = new URL("../..", import.meta.url);

async function read(relativePath) {
  const contents = await readFile(new URL(relativePath, root), "utf8");
  return contents.replaceAll("\r\n", "\n");
}

test("tag release builds the declared macOS, Windows, and Ubuntu artifact matrix", async () => {
  const workflow = await read(".github/workflows/release.yml");
  const staging = await read("scripts/release/stage-artifacts.sh");
  const macSidecars = await read("scripts/release/prepare-macos-universal-sidecars.sh");

  for (const required of [
    "macos_universal",
    "windows_x64",
    "ubuntu_amd64",
    "universal-apple-darwin",
    "x86_64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
    "Cookbench-${VERSION}-macos-universal.dmg",
    "Cookbench-${VERSION}-windows-x64.msi",
    "Cookbench-${VERSION}-linux-amd64.deb",
    "Cookbench-${VERSION}-linux-amd64.AppImage",
  ]) {
    assert.ok(`${workflow}\n${staging}`.includes(required), `missing ${required}`);
  }

  assert.match(workflow, /ref: \$\{\{ inputs\.tag \|\| github\.ref \}\}/);
  assert.ok(
    workflow.match(/ref: \$\{\{ needs\.release_context\.outputs\.tag \}\}/g)?.length >= 4,
    "every dependent build/metadata job must checkout the resolved tag",
  );
  assert.match(workflow, /commit_sha:/);
  assert.match(workflow, /git rev-parse HEAD/);
  assert.doesNotMatch(workflow, /TAG="\$\{\{ inputs\.tag/);
  assert.match(workflow, /permissions:\n  contents: read/);
  assert.match(workflow, /publish_release:[\s\S]*permissions:\n      contents: write/);
  assert.match(workflow, /if \[\[ "\$RELEASE_CHANNEL" == "prerelease" \]\]/);
  assert.match(workflow, /args\+=\(--draft\)/);
  assert.match(workflow, /COOKBENCH_TARGET: universal-apple-darwin/);
  assert.match(macSidecars, /for helper in cookbench-bridge cookbench-hook/);
  assert.match(macSidecars, /src-tauri\/binaries\/\$helper-\$target/);
});

test("release output is checksummed, described, and kept distinct from stable publishing", async () => {
  const workflow = await read(".github/workflows/release.yml");
  const manifest = await read("scripts/release/build-manifest.mjs");
  const gate = await read("scripts/release/check-release-channel.sh");

  assert.match(workflow, /sha256sum/);
  assert.match(workflow, /build-manifest\.mjs/);
  assert.match(manifest, /sbom\.spdx\.json/);
  assert.match(manifest, /release-manifest\.json/);
  assert.match(workflow, /check-release-channel\.sh/);
  assert.match(manifest, /SPDX-2\.3/);
  assert.match(manifest, /sha256/);
  assert.match(gate, /stable/);
  assert.match(gate, /APPLE_CERTIFICATE/);
  assert.match(gate, /WINDOWS_SIGNING_CERTIFICATE/);
  assert.match(gate, /notarization/);
});

test("registry submissions and installation instructions are generated rather than pretended", async () => {
  const installer = await read("docs/installing.md");
  const registry = await read("scripts/release/generate-registry-metadata.mjs");

  assert.match(installer, /brew install --cask cookbench/);
  assert.match(installer, /winget install Cookbench\.Cookbench/);
  assert.match(installer, /sudo apt install cookbench/);
  assert.match(installer, /not yet published/i);
  assert.match(registry, /homebrew/);
  assert.match(registry, /winget/);
  assert.match(registry, /apt/);
  assert.match(registry, /sha256/i);
});
