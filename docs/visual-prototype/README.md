# Cookbench Lightweight Visual Proposal

This proposal defines an original, asset-light visual direction for Cookbench.
It is a review artifact, not the production application.

## Direction

**Precision stove** combines an instrument-panel feel with restrained cooking
signals. The interface remains quiet during long sessions and becomes expressive
only when a stove changes state.

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

## Status Language

| State | Shape and motion | Color | Meaning |
| --- | --- | --- | --- |
| Cooking | Rotating heat tick and breathing center | Heat orange | Agent is actively working |
| Needs attention | Two-beat pulse | Amber | Human input or correction is needed |
| Cooked | Stable full ring and one short finish flare | Green | The latest task finished successfully |
| Failed | Broken ring | Red | The run ended unsuccessfully |
| Disconnected | Dashed, static ring | Gray | Source or SSH connection is unavailable |

Progress arcs appear only when the source exposes structured plan/task data.
Otherwise, the ring communicates activity without inventing a percentage.

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
