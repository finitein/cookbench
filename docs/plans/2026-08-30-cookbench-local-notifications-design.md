# Cookbench Local Notifications Design

## Goal

Add four independent local alert channels to Cookbench: sound, an operating-system notification banner, a brief Stove flash, and a system attention request. Users choose which channels and lifecycle events are enabled in Settings. A fresh or migrated installation enables sound only.

Local alerts remain an observation feature. They never send input to an Agent, modify a native Session, approve a request, or change a Stove state.

## User Experience

Settings gains a **Local alerts** section above outbound notification destinations.

The four channel toggles are:

- **Sound**: enabled by default.
- **System notification**: disabled by default. Enabling or testing it may ask for operating-system notification permission.
- **Flash Stove**: disabled by default. The affected Stove receives a short visual emphasis; reduced-motion settings replace animation with a static emphasis.
- **Request attention**: disabled by default. Cookbench asks the desktop environment to draw attention to the application, such as a Dock bounce, taskbar flash, or urgency hint.

All four channels share one event selection. Defaults are Needs Human, Cooked, Failed, and Disconnected. The existing ten lifecycle event choices remain available. Each channel has a test action so a user can verify it without waiting for an Agent transition.

## Architecture

`PersistedConfig.preferences` receives a serde-defaulted local notification preference object. The object stores four booleans plus a bounded set of lifecycle event kinds. Its custom default enables only sound and selects the four default events. Existing config files migrate without a version bump because the field is additive and defaulted.

The desktop notification runtime receives only an already-accepted effective Stove transition. It filters by event, applies a short `(stove, event)` duplicate guard, and independently attempts each enabled channel:

- Sound uses a bounded, non-shell system sound command selected per platform.
- System notification uses the official Tauri 2 notification plugin from Rust.
- Stove flash emits a metadata-only Tauri event consumed by the main and detached React views.
- System attention uses Tauri's native window attention API.

The local runtime is separate from outbound Telegram, Slack, Discord, Lark, and generic webhook delivery. A failure in either path does not suppress the other.

## Data And Privacy

Local notification payloads contain only:

- Stable Stove ID
- Selected lifecycle event
- Bounded project display label
- Sanitized state label

They never contain transcript text, prompt text, tool input, commands, credentials, full native paths, or task/action summaries. Operating-system notification history therefore receives no conversation content.

The visual flash event carries only Stove ID and lifecycle event. Sound commands use fixed executables and fixed arguments; no user-derived value enters a shell.

## Permissions And Failure Behavior

Cookbench never opens a notification permission prompt during background observation. Permission is requested only after an explicit user action to enable or test system notifications. If permission is denied, the setting remains understandable and testing returns a visible, bounded error.

Sound, banner, flash, and attention delivery are best effort. Missing Linux sound utilities, denied notification permission, or unsupported desktop attention behavior are recorded as redacted diagnostics and never change Agent or Stove state. Test feedback disappears through the existing transient notice behavior.

## Visual Behavior

The flash emphasizes the affected Stove rather than the entire transparent window. It lasts approximately 1.2 seconds, does not resize the Bar, and works in both the global Bar and a detached Stove window. Under `prefers-reduced-motion: reduce`, the same duration uses a static outline/highlight with no pulsing or sweeping animation.

No new image, font, animation asset, or third-party logo is added.

## Verification

Automated coverage must prove:

- New and legacy config files default to sound only.
- Event selection is bounded and round-trips through persistence.
- Stale or superseded observations do not alert.
- Each platform sound driver uses fixed arguments and times out safely.
- Banner permission denial does not trigger background prompts or other state changes.
- Flash targets only the matching Stove and honors reduced motion.
- Settings loads, saves, and tests every channel.
- Existing outbound notification behavior is unchanged.

Release verification includes the full Rust and frontend suites, lint, clippy, production build, Tauri build, and a macOS packaged-app smoke test. Windows and graphical Linux behavior must be verified on their native runners or recorded honestly as unverified; a compile-only result is not a desktop notification proof.
