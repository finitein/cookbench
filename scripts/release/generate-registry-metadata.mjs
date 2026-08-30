import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";

function argument(name) {
  const index = process.argv.indexOf(name);
  if (index === -1 || !process.argv[index + 1]) throw new Error(`Missing required ${name} argument`);
  return process.argv[index + 1];
}

function artifact(manifest, suffix) {
  const match = manifest.artifacts.find((candidate) => candidate.name.endsWith(suffix));
  if (!match) throw new Error(`Release manifest is missing ${suffix}`);
  return match;
}

const manifestPath = resolve(argument("--manifest"));
const output = resolve(argument("--output"));
const baseUrl = argument("--base-url").replace(/\/$/, "");
const aptBaseUrl = argument("--apt-base-url").replace(/\/$/, "");
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));

if (manifest.channel !== "stable") {
  throw new Error("Registry metadata is generated only for a stable, signed release manifest");
}

const macos = artifact(manifest, "-macos-universal.dmg");
const windows = artifact(manifest, "-windows-x64.msi");
const deb = artifact(manifest, "-linux-amd64.deb");
const version = manifest.version;

const homebrew = `cask "cookbench" do\n  version "${version}"\n  sha256 "${macos.sha256}"\n\n  url "${baseUrl}/${macos.name}"\n  name "Cookbench"\n  desc "Observe native coding-agent sessions without controlling them"\n  homepage "https://github.com/${process.env.GITHUB_REPOSITORY ?? "OWNER/REPO"}"\n\n  depends_on macos: ">= :sonoma"\n\n  app "Cookbench.app"\nend\n`;

const wingetInstaller = {
  PackageIdentifier: "Cookbench.Cookbench",
  PackageVersion: version,
  InstallerType: "wix",
  Installers: [{
    Architecture: "x64",
    InstallerUrl: `${baseUrl}/${windows.name}`,
    InstallerSha256: windows.sha256.toUpperCase(),
  }],
  ManifestType: "installer",
  ManifestVersion: "1.9.0",
};
const wingetLocale = {
  PackageIdentifier: "Cookbench.Cookbench",
  PackageVersion: version,
  PackageLocale: "en-US",
  Publisher: "Cookbench",
  PackageName: "Cookbench",
  ShortDescription: "Read-only native session observer for coding agents",
  ManifestType: "defaultLocale",
  ManifestVersion: "1.9.0",
};
const wingetVersion = {
  PackageIdentifier: "Cookbench.Cookbench",
  PackageVersion: version,
  DefaultLocale: "en-US",
  ManifestType: "version",
  ManifestVersion: "1.9.0",
};

function yaml(value, depth = 0) {
  const indent = "  ".repeat(depth);
  if (Array.isArray(value)) {
    return value.map((item) => `${indent}- ${typeof item === "object" ? `\n${yaml(item, depth + 1)}` : item}`).join("\n");
  }
  return Object.entries(value).map(([key, item]) => {
    if (Array.isArray(item)) return `${indent}${key}:\n${yaml(item, depth + 1)}`;
    return `${indent}${key}: ${JSON.stringify(item)}`;
  }).join("\n");
}

const debPoolPath = `pool/main/c/cookbench/${deb.name}`;
const packages = `Package: cookbench\nVersion: ${version}\nArchitecture: amd64\nMaintainer: Cookbench\nDescription: Read-only native session observer for coding agents\nFilename: ${debPoolPath}\nSize: ${deb.bytes}\nSHA256: ${deb.sha256}\n\n`;
const packagesGzipDigest = createHash("sha256").update(packages).digest("hex");
const release = `Origin: Cookbench\nLabel: Cookbench\nSuite: stable\nCodename: stable\nArchitectures: amd64\nComponents: main\nDescription: Cookbench APT repository\nSHA256:\n ${packagesGzipDigest} ${Buffer.byteLength(packages)} main/binary-amd64/Packages\n`;
const sources = `Types: deb\nURIs: ${aptBaseUrl}\nSuites: stable\nComponents: main\nArchitectures: amd64\nSigned-By: /usr/share/keyrings/cookbench-archive-keyring.gpg\n`;

const homebrewPath = join(output, "homebrew", "Casks", "cookbench.rb");
const wingetPath = join(output, "winget", "Cookbench", "Cookbench", version);
const aptRoot = join(output, "apt");
await Promise.all([mkdir(dirname(homebrewPath), { recursive: true }), mkdir(wingetPath, { recursive: true }), mkdir(join(aptRoot, "dists/stable/main/binary-amd64"), { recursive: true }), mkdir(join(aptRoot, "pool/main/c/cookbench"), { recursive: true })]);
await Promise.all([
  writeFile(homebrewPath, homebrew),
  writeFile(join(wingetPath, "Cookbench.Cookbench.installer.yaml"), `${yaml(wingetInstaller)}\n`),
  writeFile(join(wingetPath, "Cookbench.Cookbench.locale.en-US.yaml"), `${yaml(wingetLocale)}\n`),
  writeFile(join(wingetPath, "Cookbench.Cookbench.yaml"), `${yaml(wingetVersion)}\n`),
  writeFile(join(aptRoot, "dists/stable/main/binary-amd64/Packages"), packages),
  writeFile(join(aptRoot, "dists/stable/Release"), release),
  writeFile(join(aptRoot, "cookbench.sources"), sources),
  writeFile(join(aptRoot, "README.txt"), "This is unsigned APT repository metadata. Sign Release/InRelease and publish the DEB at the stated pool path before providing it to users.\n"),
]);
