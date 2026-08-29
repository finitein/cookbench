# Session Focus Verification

Date: 2026-08-30

Cookbench only generates bounded, first-party focus actions. It does not invoke
a shell, interpolate locator values into command text, control an agent, or
embed a session transcript. The fallback order is always exact pane, application
window, project directory, then visible resume instructions. Permission denial,
elevation, and desktop focus policy continue to the next fallback.

## Capability Matrix

| Surface | Exact session/pane | Application window | Project directory | Status |
| --- | --- | --- | --- | --- |
| tmux | `tmux select-pane -t <pane>` argv action | Host application if recorded | Yes | Automated action-generation coverage; manual host check pending |
| VS Code | Not claimed in V1 | Visual Studio Code activation | IDE workspace or working directory | Automated fallback coverage; manual host check pending |
| macOS Terminal | Not claimed in V1 | Terminal activation | Yes | Automated fallback coverage; macOS automation/accessibility check pending |
| iTerm2 | Not claimed in V1 | iTerm activation | Yes | Automated fallback coverage; macOS automation/accessibility check pending |
| Windows Terminal | Not claimed in V1 | Windows Terminal activation, subject to elevation | Yes | Automated elevated-target fallback coverage; Windows manual check pending |
| Ubuntu GNOME Terminal | Not claimed in V1 | GNOME Terminal activation, subject to compositor policy | Yes | Automated fallback coverage; Ubuntu X11/Wayland manual checks pending |
| Ubuntu Konsole / Xfce Terminal | Not claimed in V1 | Application activation, subject to compositor policy | Yes | Automated fallback coverage; Ubuntu manual checks pending |

## Current Host Evidence

The current development host is macOS. `cargo test -p cookbench-desktop --test
locator_fallback` verifies action ordering, permission-denied and elevated-target
fallbacks, tmux argv separation, and unsupported-terminal honesty. It does not
prove macOS Automation permission grants, real terminal tab selection, Windows
foreground rules, or Ubuntu X11/Wayland focus behavior; those checks remain
release-host manual verification work.
