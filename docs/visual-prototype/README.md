# Cookbench Lightweight Visual Proposal

This proposal defines an original, asset-light visual direction for Cookbench.
It is a review artifact, not the production application.

## Direction

**Bright Precision Stove** combines restrained cooking signals with the bright,
translucent, fluid hierarchy of Apple's current design language. The interface
remains quiet during long sessions and becomes vivid only when a stove changes
state. Light mode is the primary presentation.

This is an Apple-inspired interpretation, not a copy of Apple UI. It uses no
Apple-owned templates, icons, screenshots, or brand assets. The same hierarchy is
translated to native macOS, Windows, and Ubuntu material capabilities.

The brand mark is an open burner ring that also reads as the letter `C`. Its
single ignition point is the only warm accent in the static mark.

## Asset Budget

| Asset | Format | Target production cost |
| --- | --- | --- |
| Brand master | SVG | Under 1 KB minified |
| Tray/menu mark | SVG | Under 1 KB minified |
| UI status visuals | CSS + inline SVG primitives | No downloaded assets |
| Motion | CSS transforms and opacity | No Lottie, GIF, video, or sprite sheets |
| Platform icons | Generated ICO/ICNS/PNG | Generated at build/release time |
| Fonts | System font stack | No bundled web fonts |

The prototype itself has no runtime dependencies, external fonts, images,
network calls, canvas textures, or JavaScript frameworks.

## Material and Color

- The global and detached Bars are the floating functional layer. They use a
  translucent white regular-glass treatment with blur, saturation, a bright edge,
  and a soft neutral shadow.
- Hover details may use the same functional material, but settings/content areas
  remain near-white solid surfaces. Do not stack glass panels inside glass panels.
- Light presentation is the default. A solid high-opacity fallback is mandatory
  when transparency, compositor blur, or backdrop filters are unavailable.
- State colors are deliberately vivid: heat orange, attention amber, cooked green,
  failure red, and disconnected system gray.
- Source accents are cyan for Codex, coral for Claude, and violet for Pi; visible
  source names remain authoritative.
- Motion responds immediately, settles quickly, and avoids decorative looping.
  Reduced-motion, reduced-transparency, and increased-contrast settings must be
  respected.

Official design references:

- [Apple HIG: Materials](https://developer.apple.com/design/human-interface-guidelines/materials)
- [Apple HIG: Color](https://developer.apple.com/design/human-interface-guidelines/color)
- [WWDC25: Meet Liquid Glass](https://developer.apple.com/videos/play/wwdc2025/219/)
- [WWDC25: Get to know the new design system](https://developer.apple.com/videos/play/wwdc2025/356/)

## Status Language

| State | Shape and motion | Color | Meaning |
| --- | --- | --- | --- |
| Cooking | Rotating heat tick and breathing center | Heat orange | Agent is actively working |
| Needs attention | Full ring with a two-beat pulse | Amber | Human input or correction is needed |
| Cooked | Stable full ring and one short finish flare | Green | The latest task finished successfully |
| Failed | Static full ring | Red | The run ended unsuccessfully |
| Disconnected | Static full ring | Gray | Source or SSH connection is unavailable |

Progress arcs appear only while Cooking and only when the source exposes
structured plan/task data. All terminal or attention states use a full ring;
they never use arc length as a proxy for status. Without structured progress,
Cooking uses activity motion without inventing a percentage.

## Source Identity

The first version uses neutral text labels (`Codex`, `Claude`, `Pi`) and compact
letter tokens (`CX`, `CL`, `PI`). It does not bundle or imitate third-party
logos. This remains legible at small sizes and avoids a trademark dependency.

## Interaction Notes

- The global bar always represents every retained stove.
- Hovering or focusing a burner reveals exact project, current action, progress
  provenance, elapsed time, and expected next action.
- Clicking a burner is reserved for returning to the originating tool/session.
- Detached bars use the same burner, color, typography, and state grammar.
- Finished stoves remain visible until the user removes them.

## Files

- `index.html`: self-contained interactive review surface.
- `assets/cookbench-mark.svg`: original full-color brand master.
- `assets/cookbench-tray.svg`: original monochrome tray/menu mark.

Open `index.html` directly in a browser. No development server is required.
