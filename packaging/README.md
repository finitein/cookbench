# Release Packaging

`release-matrix.json` is the CI contract for artifacts built on their native
platform runners. It is not evidence that a platform was manually verified.

The tag workflow always creates a reviewable draft. `prerelease` accepts
unsigned artifacts but labels them as such and never generates registry
submission metadata. A `stable` workflow dispatch requires the signing and
notarization secret set, then validates macOS stapling and Windows Authenticode
before it can create stable registry candidates.

`scripts/release/generate-registry-metadata.mjs` emits three candidate sets:

- a Homebrew Cask with a fixed version, URL, and SHA-256;
- three winget manifests with a fixed MSI URL and SHA-256;
- APT `Packages`, `Release`, and source-entry files.

Those files are submissions/bootstrap material only. Homebrew and winget need
their own review/merge process. The APT metadata must be signed and uploaded to
an HTTPS repository with the generated DEB at its declared pool path before
`sudo apt install cookbench` is a valid user instruction.
