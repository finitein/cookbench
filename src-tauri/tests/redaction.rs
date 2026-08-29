use cookbench_core::diagnostics::{
    AdapterDiagnostic, AdapterId, CapabilitySummary, FallbackReason, Health,
};
use cookbench_desktop_lib::diagnostics::collect_diagnostics;

#[test]
fn diagnostics_never_serialize_user_content_or_secrets() {
    let prompt = "PROMPT_DO_NOT_LEAK: implement the private feature";
    let code = "CODE_DO_NOT_LEAK: const apiKey = 'private';";
    let token = "TOKEN_DO_NOT_LEAK: bearer-secret-value";
    let webhook = "https://hooks.example.invalid/secret-webhook";
    let credential = "/Users/alex/.config/credentials.json";
    let ssh_secret = "/Users/alex/.ssh/id_ed25519";

    let diagnostics = collect_diagnostics(
        vec![AdapterDiagnostic {
            adapter: AdapterId::Codex,
            health: Health::Degraded,
            capabilities: CapabilitySummary {
                discovery: true,
                watch_events: true,
                structured_progress: true,
                locator: false,
            },
            parser_error_count: 3,
        }],
        [
            "/Users/alex/.codex/sessions".to_owned(),
            webhook.to_owned(),
            credential.to_owned(),
            ssh_secret.to_owned(),
        ],
        vec![FallbackReason::WaylandBestEffortOverlay],
        "/Users/alex",
    );

    let output = serde_json::to_string(&diagnostics).expect("diagnostics serialize");
    for forbidden in [prompt, code, token, webhook, credential, ssh_secret, "alex"] {
        assert!(
            !output.contains(forbidden),
            "diagnostics leaked {forbidden}"
        );
    }
    assert!(output.contains("parserErrorCount"));
    assert!(output.contains("waylandBestEffortOverlay"));
    assert!(output.contains("~/.codex/sessions"));
}

#[test]
fn malformed_path_corpus_isolated_from_diagnostics_output() {
    let malformed = [
        "\0not-a-path",
        "https://token.example.invalid/secret",
        "C:\\Users\\alex\\.ssh\\id_rsa",
        "/home/alex/passwords.txt",
        "/home/alex/project\nPROMPT_DO_NOT_LEAK",
    ];
    let diagnostics =
        collect_diagnostics(vec![], malformed.map(str::to_owned), vec![], "/home/alex");
    let output = serde_json::to_string(&diagnostics).expect("diagnostics serialize");

    for forbidden in ["token.example", ".ssh", "password", "PROMPT_DO_NOT_LEAK"] {
        assert!(
            !output.contains(forbidden),
            "diagnostics leaked {forbidden}"
        );
    }
}
