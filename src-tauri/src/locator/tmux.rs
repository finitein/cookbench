use super::JumpAction;

/// Generates a direct tmux invocation. The target is an argv element, not
/// interpolated shell text. tmux is the only V1 terminal integration that can
/// truthfully claim pane-level focus.
pub fn exact_pane_action(pane: Option<&str>) -> Option<JumpAction> {
    let pane = pane?;
    if pane.is_empty() || pane.len() > 256 || pane.chars().any(char::is_control) {
        return None;
    }

    Some(JumpAction::ExactPane {
        program: "tmux",
        args: vec!["select-pane".to_owned(), "-t".to_owned(), pane.to_owned()],
    })
}
