//! Stable compatibility metadata for external coding-agent harnesses.
//!
//! A catalog entry describes an integration boundary; it does not by itself
//! claim that the harness is installed or that a healthy hook was observed.

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum SupportTier {
    Full,
    Standard,
    Experimental,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HookDialect {
    None,
    CodexNotify,
    ClaudeJson,
    GeminiJson,
    KimiToml,
    QoderJson,
    ZcodeJson,
    FactoryJson,
    CodeBuddyJson,
    CursorJson,
    CopilotJson,
    OpenCodePlugin,
    ClineScripts,
    PiExtension,
    GenericStructured,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReturnSurface {
    CodexDesktopOrTerminal,
    Terminal,
    Ide,
    ApplicationOrTerminal,
    GuardedApplication,
    PresenceOnly,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HarnessProfile {
    pub id: &'static str,
    pub label: &'static str,
    pub tier: SupportTier,
    pub hook_dialect: HookDialect,
    pub return_surface: ReturnSurface,
    pub structured_lifecycle: bool,
    pub executables: &'static [&'static str],
    pub default_roots: &'static [&'static str],
    pub reference: &'static str,
}

macro_rules! profile {
    ($id:literal, $label:literal, $tier:ident, $dialect:ident, $surface:ident,
     $structured:literal, [$($executable:literal),* $(,)?], [$($root:literal),* $(,)?], $reference:literal) => {
        HarnessProfile {
            id: $id,
            label: $label,
            tier: SupportTier::$tier,
            hook_dialect: HookDialect::$dialect,
            return_surface: ReturnSurface::$surface,
            structured_lifecycle: $structured,
            executables: &[$($executable),*],
            default_roots: &[$($root),*],
            reference: $reference,
        }
    };
}

static CATALOG: [HarnessProfile; 27] = [
    profile!(
        "codex",
        "Codex",
        Full,
        CodexNotify,
        CodexDesktopOrTerminal,
        true,
        ["codex"],
        ["~/.codex/sessions"],
        "https://developers.openai.com/codex/"
    ),
    profile!(
        "claude_code",
        "Claude Code",
        Full,
        ClaudeJson,
        Terminal,
        true,
        ["claude"],
        ["~/.claude/projects"],
        "https://docs.anthropic.com/en/docs/claude-code/hooks"
    ),
    profile!(
        "pi",
        "Pi / Oh My Pi",
        Full,
        PiExtension,
        Terminal,
        true,
        ["pi", "omp"],
        ["~/.pi/agent/sessions"],
        "https://github.com/badlogic/pi-mono"
    ),
    profile!(
        "gemini_cli",
        "Gemini CLI",
        Full,
        GeminiJson,
        Terminal,
        true,
        ["gemini"],
        ["~/.gemini/tmp"],
        "https://github.com/google-gemini/gemini-cli/blob/main/docs/hooks/reference.md"
    ),
    profile!(
        "qwen_code",
        "Qwen Code",
        Full,
        GeminiJson,
        Terminal,
        true,
        ["qwen"],
        ["~/.qwen/tmp"],
        "https://qwenlm.github.io/qwen-code-docs/en/users/features/hooks/"
    ),
    profile!(
        "kimi_code",
        "Kimi Code CLI",
        Full,
        KimiToml,
        Terminal,
        true,
        ["kimi"],
        ["~/.kimi-code/sessions"],
        "https://www.kimi.com/code/docs/en/kimi-code-cli/customization/hooks.html"
    ),
    profile!(
        "qoder",
        "Qoder",
        Full,
        QoderJson,
        Ide,
        true,
        ["qoder"],
        ["~/.qoder/projects"],
        "https://docs.qoder.com/cli/hooks"
    ),
    profile!(
        "zcode",
        "ZCode",
        Full,
        ZcodeJson,
        Terminal,
        true,
        ["zcode"],
        ["~/.zcode"],
        "https://zcode.z.ai/en/docs/hooks"
    ),
    profile!(
        "factory_droid",
        "Factory Droid",
        Full,
        FactoryJson,
        ApplicationOrTerminal,
        true,
        ["droid"],
        ["~/.factory"],
        "https://docs.factory.ai/harness/hooks"
    ),
    profile!(
        "codebuddy",
        "CodeBuddy",
        Full,
        CodeBuddyJson,
        ApplicationOrTerminal,
        true,
        ["codebuddy"],
        ["~/.codebuddy"],
        "https://www.codebuddy.ai/docs/cli/README"
    ),
    profile!(
        "cursor",
        "Cursor",
        Full,
        CursorJson,
        Ide,
        true,
        ["cursor", "cursor-agent"],
        ["~/.cursor"],
        "https://prod.cursor.com/docs/hooks"
    ),
    profile!(
        "github_copilot",
        "GitHub Copilot CLI",
        Full,
        CopilotJson,
        Terminal,
        true,
        ["copilot", "gh"],
        ["~/.copilot"],
        "https://docs.github.com/en/copilot/reference/hooks-reference"
    ),
    profile!(
        "opencode",
        "OpenCode",
        Full,
        OpenCodePlugin,
        ApplicationOrTerminal,
        true,
        ["opencode"],
        ["~/.local/share/opencode", "~/.config/opencode"],
        "https://opencode.ai/docs/plugins/"
    ),
    profile!(
        "cline",
        "Cline",
        Full,
        ClineScripts,
        Ide,
        true,
        ["cline"],
        ["~/.cline/data", "~/Documents/Cline"],
        "https://docs.cline.bot/customization/hooks"
    ),
    profile!(
        "trae",
        "Trae / Trae CLI",
        Standard,
        GenericStructured,
        Ide,
        true,
        ["trae", "trae-cli", "traecli"],
        ["~/.trae"],
        "https://www.trae.ai/"
    ),
    profile!(
        "grok_cli",
        "Grok CLI",
        Standard,
        GenericStructured,
        Terminal,
        true,
        ["grok"],
        ["~/.grok"],
        "https://github.com/superagent-ai/grok-cli"
    ),
    profile!(
        "goose",
        "Goose",
        Standard,
        GenericStructured,
        ApplicationOrTerminal,
        true,
        ["goose"],
        ["~/.local/share/goose", "~/.config/goose"],
        "https://block.github.io/goose/"
    ),
    profile!(
        "aider",
        "Aider",
        Standard,
        GenericStructured,
        Terminal,
        true,
        ["aider"],
        ["~/.aider"],
        "https://aider.chat/docs/"
    ),
    profile!(
        "kiro",
        "Kiro",
        Standard,
        GenericStructured,
        Ide,
        true,
        ["kiro", "kiro-cli"],
        ["~/.kiro"],
        "https://kiro.dev/docs/cli/"
    ),
    profile!(
        "amazon_q",
        "Amazon Q Developer",
        Standard,
        GenericStructured,
        Terminal,
        true,
        ["q", "qchat"],
        ["~/.aws/amazonq"],
        "https://docs.aws.amazon.com/amazonq/latest/qdeveloper-ug/command-line.html"
    ),
    profile!(
        "roo_code",
        "Roo Code",
        Standard,
        GenericStructured,
        Ide,
        true,
        ["roo"],
        ["~/.roo"],
        "https://docs.roocode.com/"
    ),
    profile!(
        "continue",
        "Continue",
        Standard,
        GenericStructured,
        Ide,
        true,
        ["cn", "continue"],
        ["~/.continue"],
        "https://docs.continue.dev/"
    ),
    profile!(
        "amp",
        "Amp",
        Standard,
        GenericStructured,
        Terminal,
        true,
        ["amp"],
        ["~/.config/amp"],
        "https://ampcode.com/manual"
    ),
    profile!(
        "mistral_vibe",
        "Mistral Vibe",
        Standard,
        GenericStructured,
        Terminal,
        true,
        ["vibe"],
        ["~/.vibe"],
        "https://github.com/mistralai/mistral-vibe"
    ),
    profile!(
        "crush",
        "Crush",
        Standard,
        GenericStructured,
        Terminal,
        true,
        ["crush"],
        ["~/.config/crush"],
        "https://github.com/charmbracelet/crush"
    ),
    profile!(
        "openhands",
        "OpenHands CLI",
        Standard,
        GenericStructured,
        Terminal,
        true,
        ["openhands"],
        ["~/.openhands"],
        "https://docs.openhands.dev/usage/how-to/cli-mode"
    ),
    profile!(
        "workbuddy",
        "Tencent WorkBuddy",
        Experimental,
        None,
        PresenceOnly,
        false,
        ["workbuddy"],
        [],
        "https://copilot.tencent.com/work/"
    ),
];

pub fn catalog() -> &'static [HarnessProfile] {
    &CATALOG
}

pub fn harness_profile(id: &str) -> Option<&'static HarnessProfile> {
    CATALOG.iter().find(|profile| profile.id == id)
}
