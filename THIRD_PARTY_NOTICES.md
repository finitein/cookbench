# Third-Party Notices

Cookbench has not copied third-party source code, artwork, logos, fonts,
session fixtures, or other runtime assets into the project. It does use the
open-source packages declared in `Cargo.toml`, `Cargo.lock`, `package.json`, and
`pnpm-lock.yaml`; those manifests and locks are the authoritative dependency
inventory for each build.

The approved Cookbench mark and tray mark in `docs/visual-prototype/assets/` are
original project assets. The implementation may study the projects listed in
`docs/references/reuse-audit.md`, but an audit entry alone does not authorize
copying. In particular, AgentHUD had no license file when reviewed and is
strictly idea-only: its source must not be copied or ported without explicit
permission or a valid license.

## Direct Dependency Families

The following dependency families are linked or bundled through their normal
package-manager distributions. Cookbench has not modified their source.

| Component | Purpose | License declared upstream | Local modifications | Required notice |
| --- | --- | --- | --- | --- |
| Tauri API, CLI, and Rust crates | Cross-platform desktop shell and packaging | Apache-2.0 OR MIT | None | Preserve upstream license terms in distributions |
| React and React DOM | User interface runtime | MIT | None | Preserve upstream license terms in distributions |
| Tokio, Serde, reqwest, notify, and supporting Rust crates | Async work, schemas, outbound HTTPS, and file observation | See the exact locked crate metadata | None | Preserve each locked crate's license terms |
| keyring | OS credential-store boundary | Apache-2.0 OR MIT | None | Preserve upstream license terms in distributions |
| Vite, TypeScript, Vitest, Testing Library, and Playwright | Build and test tooling; not runtime artwork | See the exact locked package metadata | None | Preserve upstream license terms where redistributed |

No third-party brand artwork or web font is bundled. Provider names are plain
text source labels, not copied logos.

## Future Incorporated Material

Any future copied or adapted material must add a row containing:

| Component | Upstream project and revision | Source files | License | Local modifications | Required notice |
| --- | --- | --- | --- | --- | --- |
| _None_ | _Not applicable_ | _Not applicable_ | _Not applicable_ | _Not applicable_ | _Not applicable_ |
