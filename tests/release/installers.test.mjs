import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import test from "node:test";

const exec = promisify(execFile);
const root = new URL("../..", import.meta.url);

function manifest(channel = "stable") {
  return {
    schemaVersion: 1,
    product: "Cookbench",
    version: "0.2.1",
    channel,
    signing: channel === "stable" ? "required" : "unsigned-prerelease",
    artifacts: [
      { name: "Cookbench-0.2.1-macos-universal.dmg", sha256: "a".repeat(64) },
      { name: "Cookbench-0.2.1-windows-x64.msi", sha256: "b".repeat(64) },
      { name: "Cookbench-0.2.1-linux-amd64.deb", sha256: "c".repeat(64) },
      { name: "Cookbench-0.2.1-linux-amd64.AppImage", sha256: "d".repeat(64) },
    ],
  };
}

async function withManifest(channel, run) {
  const directory = await mkdtemp(join(tmpdir(), "cookbench-installer-"));
  const path = join(directory, "release-manifest.json");
  await writeFile(path, `${JSON.stringify(manifest(channel), null, 2)}\n`);
  try {
    await run(path);
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
}

test("shell installer selects platform artifacts without installing in dry-run", async () => {
  await withManifest("stable", async (path) => {
    for (const [os, arch, expected] of [
      ["macos", "arm64", "Cookbench-0.2.1-macos-universal.dmg"],
      ["macos", "x64", "Cookbench-0.2.1-macos-universal.dmg"],
      ["linux", "x64", "Cookbench-0.2.1-linux-amd64.AppImage"],
    ]) {
      const { stdout } = await exec("bash", [
        new URL("../../scripts/install.sh", import.meta.url).pathname,
        "--dry-run",
        "--manifest",
        path,
        "--os",
        os,
        "--arch",
        arch,
        "--base-url",
        "https://example.invalid/release",
      ]);
      assert.match(stdout, new RegExp(expected.replaceAll(".", "\\.")));
      assert.match(stdout, /SHA-256/);
    }
  });
});

test("shell installer rejects unsupported architectures and prereleases by default", async () => {
  await withManifest("stable", async (path) => {
    await assert.rejects(
      exec("bash", [new URL("../../scripts/install.sh", import.meta.url).pathname, "--dry-run", "--manifest", path, "--os", "linux", "--arch", "arm64"]),
      /not yet available for linux\/arm64/i,
    );
  });
  await withManifest("prerelease", async (path) => {
    await assert.rejects(
      exec("bash", [new URL("../../scripts/install.sh", import.meta.url).pathname, "--dry-run", "--manifest", path]),
      /requires --allow-prerelease/i,
    );
  });
});

test("installers expose checksum, dry-run, channel, and version controls", async () => {
  const shell = await readFile(new URL("../../scripts/install.sh", import.meta.url), "utf8");
  const powershell = await readFile(new URL("../../scripts/install.ps1", import.meta.url), "utf8");
  for (const contents of [shell, powershell]) {
    assert.match(contents, /release-manifest\.json/);
    assert.match(contents, /sha256/i);
    assert.match(contents, /prerelease/i);
    assert.match(contents, /dry.?run/i);
  }
  assert.match(shell, /hdiutil/);
  assert.match(shell, /AppImage/);
  assert.match(powershell, /msiexec/i);
  assert.match(powershell, /Get-FileHash/);
});

test("release workflow publishes both installers beside the manifest", async () => {
  const workflow = await readFile(new URL("../../.github/workflows/release.yml", import.meta.url), "utf8");
  assert.match(workflow, /scripts\/install\.sh/);
  assert.match(workflow, /scripts\/install\.ps1/);
  assert.match(workflow, /release\/metadata\/install\.sh/);
  assert.match(workflow, /release\/metadata\/install\.ps1/);
});
