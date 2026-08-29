# Privacy

Cookbench is local-first. It reads the minimum native harness metadata needed to
show session state and return the user to their original tool. The original
harness remains the place where conversation and work occur.

## Stored Data

Cookbench stores only its own display configuration and retained Stove state in
small atomic JSON files. Retained state identifies a Stove and its concise,
sanitized presentation summary; it does not duplicate a full conversation.
Clearing a Stove removes Cookbench presentation state only and never deletes
native harness history.

## Diagnostics

Diagnostics contain adapter health, capability status, parser-error counts,
home-redacted non-sensitive paths, and platform fallback reasons. They omit
prompts, code, commands, session text, tokens, passwords, SSH secrets,
credential data, webhook URLs, and notification destinations. Sensitive paths
are omitted rather than redacted into a recoverable value.

## Network Use

Cookbench has no cloud synchronization or team account service in V1. SSH
inspection is user-selected and read-only. Optional notification delivery sends
only the user-selected, bounded state summary to configured outbound
destinations; Cookbench does not receive IM messages or accept remote commands.

## Verification Baselines

The repository includes deterministic bounds for diagnostics in the 1,000
historical-session and 30-active-Stove release scenarios. These prove bounded
diagnostic output but are not a substitute for recorded CPU, memory, and
hook-to-UI latency measurements on macOS, Windows, and Ubuntu release builds.
Those platform measurements remain required before public beta.
