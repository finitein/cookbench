import { createHash } from "node:crypto";
import { mkdir, readdir, readFile, stat, writeFile } from "node:fs/promises";
import { basename, extname, join, relative, resolve } from "node:path";

function argument(name) {
  const index = process.argv.indexOf(name);
  if (index === -1 || !process.argv[index + 1]) {
    throw new Error(`Missing required ${name} argument`);
  }
  return process.argv[index + 1];
}

async function collectArtifacts(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = await Promise.all(entries.map(async (entry) => {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) return collectArtifacts(path);
    return [path];
  }));
  return files.flat();
}

function sha256(contents) {
  return createHash("sha256").update(contents).digest("hex");
}

const artifactsDirectory = resolve(argument("--artifacts"));
const outputDirectory = resolve(argument("--output"));
const version = argument("--version");
const channel = argument("--channel");
const supportedExtensions = new Set([".dmg", ".zip", ".msi", ".deb", ".appimage"]);
const artifactPaths = (await collectArtifacts(artifactsDirectory))
  .filter((path) => supportedExtensions.has(extname(path).toLowerCase()))
  .sort();

if (artifactPaths.length === 0) {
  throw new Error(`No release artifacts found under ${artifactsDirectory}`);
}

const artifacts = await Promise.all(artifactPaths.map(async (path) => {
  const contents = await readFile(path);
  const metadata = await stat(path);
  return {
    name: basename(path),
    path: relative(artifactsDirectory, path),
    bytes: metadata.size,
    sha256: sha256(contents),
  };
}));

const namespace = `https://cookbench.app/spdx/${version}/${artifacts.map((artifact) => artifact.sha256.slice(0, 12)).join("-")}`;
const packages = artifacts.map((artifact, index) => ({
  SPDXID: `SPDXRef-Artifact-${index + 1}`,
  name: artifact.name,
  versionInfo: version,
  downloadLocation: "NOASSERTION",
  filesAnalyzed: false,
  checksums: [{ algorithm: "SHA256", checksumValue: artifact.sha256 }],
  licenseConcluded: "NOASSERTION",
  licenseDeclared: "NOASSERTION",
  copyrightText: "NOASSERTION",
  primaryPackagePurpose: "APPLICATION",
  comment: "First-party release artifact. This SBOM intentionally does not claim transitive dependency completeness.",
}));

const sbom = {
  spdxVersion: "SPDX-2.3",
  dataLicense: "CC0-1.0",
  SPDXID: "SPDXRef-DOCUMENT",
  name: `Cookbench ${version} first-party artifact SBOM`,
  documentNamespace: namespace,
  creationInfo: {
    creators: ["Tool: Cookbench release manifest generator"],
    created: new Date().toISOString(),
  },
  documentDescribes: packages.map((pkg) => pkg.SPDXID),
  packages,
};

const manifest = {
  schemaVersion: 1,
  product: "Cookbench",
  version,
  channel,
  signing: channel === "stable" ? "required" : "unsigned-prerelease",
  sbom: {
    filename: "sbom.spdx.json",
    scope: "first-party release artifacts only",
  },
  artifacts,
};

await mkdir(outputDirectory, { recursive: true });
await writeFile(join(outputDirectory, "release-manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
await writeFile(join(outputDirectory, "sbom.spdx.json"), `${JSON.stringify(sbom, null, 2)}\n`);
