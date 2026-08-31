# Harness Compatibility

Cookbench describes integration capability instead of using one undifferentiated
“supported” badge. Native files remain authoritative. Hooks contribute bounded
lifecycle and locator metadata only; prompt, response, command, tool input/output,
token, credential, and secret content is discarded.

## Tier Definitions

- **Full:** a structured identity and lifecycle contract exists. Cookbench can
  return exactly only when the host also supplies a unique verified locator.
- **Standard:** structured observation is available, while return may be a guarded
  application, project, IDE, or terminal fallback.
- **Experimental:** presence or an explicitly configured generic surface is visible,
  but completion is not inferred.

Automatic Hook setup means Cookbench can preview, install, repair, and uninstall
only its own entries while preserving unrelated configuration. Manual means the
profile and sanitized helper exist, but Cookbench does not rewrite that harness's
configuration yet.

| Stable ID | Harness | Tier | Observation path | Hook setup | Return surface |
| --- | --- | --- | --- | --- | --- |
| `codex` | Codex CLI/Desktop | Full | Native sessions + structured notify | Automatic | Verified Codex Desktop or terminal |
| `claude_code` | Claude Code | Full | Native JSONL + structured hooks | Automatic | Verified terminal |
| `pi` | Pi / Oh My Pi | Full | Native sessions + extension | Automatic | Verified terminal |
| `gemini_cli` | Gemini CLI | Full | Structured official hook | Manual | Verified terminal when uniquely correlated |
| `qwen_code` | Qwen Code | Full | Structured official hook | Manual | Verified terminal when uniquely correlated |
| `kimi_code` | Kimi Code CLI | Full | Structured TOML hook | Automatic | Verified terminal when uniquely correlated |
| `qoder` | Qoder | Full | Structured official hook | Manual | Guarded IDE target |
| `zcode` | ZCode | Full | Structured JSON hook | Automatic | Verified terminal when uniquely correlated |
| `factory_droid` | Factory Droid | Full | Structured official hook | Manual | Guarded app or verified terminal |
| `codebuddy` | CodeBuddy | Full | Structured official hook | Manual | Guarded app or verified terminal |
| `cursor` | Cursor | Full | Structured official hook | Manual | Guarded IDE target |
| `github_copilot` | GitHub Copilot CLI | Full | Structured official hook | Manual | Verified terminal when uniquely correlated |
| `opencode` | OpenCode | Full | Structured plugin/event surface | Manual | Guarded app or verified terminal |
| `cline` | Cline | Full | Structured task hooks | Manual | Guarded IDE target |
| `trae` | Trae / Trae CLI | Standard | Allowlisted structured metadata | Manual | Guarded IDE or terminal |
| `grok_cli` | Grok CLI | Standard | Allowlisted structured metadata | Manual | Verified terminal when uniquely correlated |
| `goose` | Goose | Standard | Allowlisted structured metadata | Manual | Guarded app or terminal |
| `aider` | Aider | Standard | Allowlisted structured metadata | Manual | Verified terminal when uniquely correlated |
| `kiro` | Kiro | Standard | Allowlisted structured metadata | Manual | Guarded IDE target |
| `amazon_q` | Amazon Q Developer | Standard | Allowlisted structured metadata | Manual | Verified terminal when uniquely correlated |
| `roo_code` | Roo Code | Standard | Allowlisted structured metadata | Manual | Guarded IDE target |
| `continue` | Continue | Standard | Allowlisted structured metadata | Manual | Guarded IDE target |
| `amp` | Amp | Standard | Allowlisted structured metadata | Manual | Verified terminal when uniquely correlated |
| `mistral_vibe` | Mistral Vibe | Standard | Allowlisted structured metadata | Manual | Verified terminal when uniquely correlated |
| `crush` | Crush | Standard | Allowlisted structured metadata | Manual | Verified terminal when uniquely correlated |
| `openhands` | OpenHands CLI | Standard | Allowlisted structured metadata | Manual | Verified terminal when uniquely correlated |
| `workbuddy` | Tencent WorkBuddy | Experimental | Process presence only | Unavailable | Presence only |

## State Safety

Absence of activity is never converted into `Cooked`. Full-circle Attention,
Cooked, Failed, and Disconnected states require corresponding structured or native
evidence. A partial Cooking arc appears only when a harness provides reliable
structured progress. WorkBuddy remains presence-only and cannot emit Needs Human,
Cooked, or Failed until a public structured contract is available.

Subagent-start and subagent-stop events are ignored so a parent harness's internal
workers do not flood the Bar with extra Stoves.

## References

Important contracts include [Codex](https://developers.openai.com/codex/),
[Claude Code](https://docs.anthropic.com/en/docs/claude-code/hooks),
[Qwen Code](https://qwenlm.github.io/qwen-code-docs/en/users/features/hooks/),
[Kimi Code](https://www.kimi.com/code/docs/en/kimi-code-cli/customization/hooks.html),
[Qoder](https://docs.qoder.com/cli/hooks), [ZCode](https://zcode.z.ai/en/docs/hooks),
[Factory Droid](https://docs.factory.ai/harness/hooks),
[Cursor](https://prod.cursor.com/docs/hooks),
[GitHub Copilot](https://docs.github.com/en/copilot/reference/hooks-reference), and
[Cline](https://docs.cline.bot/customization/hooks). The static source of truth is
[`catalog.rs`](../crates/cookbench-adapters/src/catalog.rs).
