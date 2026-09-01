# Cookbench Minimal, Top Dock, and macOS Menu Bar Design

Date: 2026-09-02
Status: Approved

## 1. Outcome

Cookbench will add three independent presentation capabilities without changing
its observation-only product boundary:

1. A persisted Minimal Global Bar mode that renders one real, highest-priority
   Stove and keeps the complete Stove list accessible.
2. Cross-platform top-edge docking for the Global Bar, including a three-pixel
   reveal strip and automatic hide/reveal behavior where the desktop environment
   permits it.
3. A macOS-only, variable-width menu bar status item that displays zero to eight
   priority Stove slots, defaults to three, activates individual sessions on
   left click, and exposes the complete Stove list on right click.

The implementation also includes documentation, social promotion images and
vertical videos, a local macOS installation update, GitHub publication, and a
v0.4.0 release.

## 2. Preserved Product Boundaries

- Native Harness Session files remain authoritative.
- Cookbench does not start, stop, prompt, approve, or control an Agent.
- No transcript, prompt, response, source code, command, or tool output is added
  to persistence, logs, fixtures, the menu bar, or promotional assets.
- Stove lifecycle, progress, notification, locator, SSH, and Detached Stove
  behavior remain unchanged.
- `pinned` continues to mean retained discovery and archive protection. It does
  not become a display-priority override.
- The normal Global Bar continues to show every visible Stove. Only the user's
  explicit Minimal mode may reduce the visible Bar to one priority Stove.
- Windows and Linux do not receive a copy of the macOS multi-Stove menu bar
  presentation in v0.4.0.
- No global mouse listener, accessibility permission, or new third-party
  dependency is introduced.

## 3. Attention Selection

A pure presentation selector in `cookbench-core` owns the canonical ordering.
It consumes only the current state, the last accepted event metadata already
held by the reducer, and a bounded Cooked-attention acknowledgement cursor.

Priority, from highest to lowest:

1. Needs Human
2. Failed
3. Disconnected
4. Unacknowledged Cooked
5. Starting, Planning, or Cooking
6. Acknowledged Cooked

Within a priority class, the most recent accepted state event wins. Stable Stove
identity is the deterministic final tie-breaker. Selection changes only after a
real Stove or acknowledgement event; there is no timed carousel.

Activating an unacknowledged Cooked Stove records a presentation-only
acknowledgement for that completed event. The Stove remains Cooked and remains
visible in the full Global Bar until explicitly cleared. If the same native
session relights and completes again, its newer completion is unacknowledged.
Acknowledgements are bounded, metadata-only, and safe to persist.

The desktop snapshot carries the ordered Stove IDs so React, the macOS status
item, tests, and any future presentation surface cannot drift into separate
priority policies.

## 4. Minimal Global Bar

Display settings add `globalBarMode: full | minimal`, defaulting to `full`.
Normal mode retains all existing grouping and responsive Bench behavior, using
the attention order within a flat Bar or within each Harness Bench.

Minimal mode renders one genuine Stove Burner:

- The ring, state, Harness identity, local/remote badge, reduced-motion behavior,
  and active alert treatment remain truthful.
- Left click activates the current Stove through the existing Locator path.
- Right click opens an accessible priority-ordered list of all Stoves.
- Hover or keyboard focus exposes safe details and an Expand command.
- With no Stoves, the Cookbench mark is shown and activates the full Bar.
- Hidden Stoves remain observed and continue to notify normally.

The normal Bar gains a familiar Collapse icon in its brand rail. Settings also
expose the persisted mode. Minimal mode keeps a stable native size so content,
alerts, or labels cannot shift the window.

## 5. Top Dock and Auto Hide

Top docking applies only to the Global Bar, in both Full and Minimal modes.
Detached Stove windows remain freely movable.

- A user drag ending within 12 logical pixels of the current monitor's work-area
  top docks the Bar and preserves its horizontal position.
- Leaving the expanded Bar schedules collapse after 600 ms.
- Collapse moves the Bar so a three-pixel, same-width trigger strip remains at
  the monitor work-area top.
- Pointer entry expands immediately. Active drag, resize, menu, or keyboard
  focus prevents collapse.
- Dragging the expanded Bar at least 24 logical pixels below the work-area top
  undocks it and restores free movement.
- Programmatic dock moves never overwrite the last free-form position.
- Selecting a placement preset explicitly clears the dock state.
- A missing monitor falls back to the primary display and clamps on-screen.

The native window controller owns monitor geometry, docking state, movement,
and persistence. The webview reports user interaction and pointer/focus intent;
it does not infer monitor layout. Failure always leaves a visible, on-screen Bar.

macOS, Windows, and Ubuntu X11 target the full behavior. GNOME Wayland reports
best-effort capability honestly: when compositor movement cannot be guaranteed,
Cookbench keeps the Bar visibly placed at the top instead of simulating a pass.

## 6. macOS Combined Status Item

Display settings add `macStatusStoveCount`, clamped to zero through eight and
defaulting to three. The setting is visible only on macOS.

Cookbench reuses its existing single tray/status item. On macOS, the status item
renders a variable-width RGBA image containing the selected Stove slots. The
current Tauri tray implementation already uses a variable-length `NSStatusItem`,
scales rectangular images by aspect ratio, and reports click position and item
rectangle. No AppKit binding or new dependency is needed.

- Each slot is approximately 18 logical pixels high with a small fixed gap.
- Rings and compact Harness marks reuse Cookbench-owned visual language.
- Slot mapping and image replacement are one immutable snapshot.
- Left click maps the event's horizontal coordinate to a Stove and activates it.
- Right click opens a rebuilt native menu containing every Stove in priority
  order plus Show Bar, Hide Bar, Settings, and Quit.
- Retained slots keep their positions when possible. A new higher-priority Stove
  replaces the lowest-priority selected slot without gratuitously shuffling the
  rest.
- A zero count or empty Stove list restores the ordinary Cookbench mark while
  preserving the application entry and native menu.
- Image or update failure also falls back to the ordinary mark and usable menu.

The combined control receives a dynamic overall accessibility label. Individual
Stoves remain keyboard and screen-reader accessible in the native menu.

## 7. Persistence and Compatibility

Layout configuration gains defaulted fields for Global Bar mode, macOS status
count, and optional top-dock layout. Existing monitor identity and relative
position helpers remain authoritative. Presentation state gains bounded Cooked
acknowledgement cursors.

Older config and state files load as Full mode, three macOS slots, undocked, and
no acknowledgements. New values are validated at the Rust command boundary.
Malformed or future values cannot strand the Bar or remove the tray entry.

## 8. Error Handling

- Docking failure leaves the Bar at its last safe visible position.
- Hidden-position persistence is suppressed for programmatic moves.
- Display removal uses existing primary-monitor fallback and clamping.
- Unsupported Wayland operations become a visible top placement, not a hidden
  or unreachable window.
- Missing or removed Stove IDs are rejected before Locator activation.
- Status image failure restores the static Cookbench icon.
- Settings updates are atomic; rejected input restores the previous UI state.

## 9. Verification

Regression tests precede behavior changes. Required evidence includes:

- Pure priority, acknowledgement, stable-slot, migration, and bound tests.
- React component and service tests for Full and Minimal modes and complete-list
  access.
- Native window-controller tests for thresholds, delayed collapse, multi-monitor
  restore, programmatic-move suppression, and Wayland fallback.
- macOS status rendering and click-hit tests for zero, one, three, and eight
  slots.
- Browser E2E for mode switching, priority replacement, restart persistence,
  complete-list reachability, and dock-driver behavior.
- Packaged macOS evidence for light/dark menu bars, Retina, every slot hit,
  right-click menu, fullscreen, multiple monitors, and constrained menu space.
- Native Windows, X11, and Wayland evidence, with unrun checks kept pending.
- `./scripts/verify.sh` passing before release.

The canonical product design and release checklist will change the all-Stoves
rule to: the normal Global Bar displays every visible Stove; Minimal mode shows
one priority Stove while keeping the complete list reachable.

## 10. Promotion and Release

The v0.4.0 announcement uses the existing Cookbench showcase system and
HyperFrames social-video system. It adds a new deterministic feature showcase
page and renders platform-safe 1200x1500 and 1080x1440 promotional images. A
vertical 1080x1920 Chinese video presents Minimal mode, top docking, and the
macOS combined status item using literal product evidence and the existing
light gray, black, orange, thin-rule visual language.

No remote assets, third-party logos, stock photography, heavy runtime media, or
unverified claims are introduced. Promotional source, rendered images, video,
README links, release notes, checksums, and platform caveats ship with v0.4.0.

After verification, the release sequence is: update versions and release notes,
merge/push the reviewed branch, create and push `v0.4.0`, monitor the GitHub
release workflow to completion, verify downloaded artifacts and checksums, then
install the released macOS artifact locally and confirm the running application
reports v0.4.0.
