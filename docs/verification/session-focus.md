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
| Codex Desktop | Exact `codex://threads/<native-id>` navigation request; OS dispatch is asynchronous | Bundle-ID activation fallback | Project root | URL escaping/action coverage; real selected-thread check pending |
| VS Code | Not claimed in V1 | Visual Studio Code activation | IDE workspace or working directory | Automated fallback coverage; manual host check pending |
| macOS Terminal | Unique TTY with selected-tab postcondition | Terminal activation | Yes | Automated driver/postcondition coverage; real Automation permission check pending |
| iTerm2 | Unique TTY with current-session postcondition | iTerm activation | Yes | Automated driver/postcondition coverage; real iTerm check pending |
| Ghostty | Native terminal ID with selected-terminal postcondition | Ghostty activation | Yes | Automated driver/postcondition coverage; real Ghostty check pending |
| WezTerm | Native pane ID with list/list-clients postcondition | WezTerm activation | Yes | Automated driver/postcondition coverage; real platform checks pending |
| Zellij | Native session and pane IDs with list-panes postcondition | None | Yes | Automated driver/postcondition coverage; real host check pending |
| cmux | Native panel ID with list-panels postcondition | cmux activation | Yes | Automated driver/postcondition coverage; real host check pending |
| Windows Terminal | Not claimed in V1 | Windows Terminal activation, subject to elevation | Yes | Automated elevated-target fallback coverage; Windows manual check pending |
| Ubuntu GNOME Terminal | Not claimed in V1 | GNOME Terminal activation, subject to compositor policy | Yes | Automated fallback coverage; Ubuntu X11/Wayland manual checks pending |
| Ubuntu Konsole / Xfce Terminal | Not claimed in V1 | Application activation, subject to compositor policy | Yes | Automated fallback coverage; Ubuntu manual checks pending |

## Current Host Evidence

The current development host is macOS. `cargo test -p cookbench-desktop --test
locator_fallback` verifies action ordering, bounded external commands,
permission-denied and elevated-target fallbacks, selector pre/postconditions,
tmux argv separation, and unsupported-terminal honesty. It does not prove
macOS Automation permission grants, a real Codex selected-thread transition,
Windows foreground rules, or Ubuntu X11/Wayland focus behavior; those checks
remain release-host manual verification work.
