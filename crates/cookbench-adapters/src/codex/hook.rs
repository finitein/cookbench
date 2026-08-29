use serde_json::Value;

/// A read-only assessment of a pre-existing Codex `notify` configuration.
/// Cookbench never applies this plan or writes a Codex config file.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotifyHookPlan {
    NotConfigured,
    Chain {
        existing_command: Vec<String>,
        cookbench_command: Vec<String>,
    },
    ReadOnlyFallback {
        reason: &'static str,
    },
}

/// Inspects a TOML-shaped config text for a top-level `notify = [..]` command.
/// Only a plain argv array can be safely chained without invoking a shell.
pub fn inspect_notify_hook(config: &str, cookbench_command: &[String]) -> NotifyHookPlan {
    let Some(raw) = config.lines().map(str::trim).find_map(|line| {
        let rest = line.strip_prefix("notify")?;
        (rest.is_empty()
            || rest.chars().next().is_some_and(char::is_whitespace)
            || rest.starts_with('='))
        .then_some(rest)
        .and_then(|rest| rest.trim_start().strip_prefix('=').map(str::trim))
    }) else {
        return NotifyHookPlan::NotConfigured;
    };
    let Ok(Value::Array(items)) = serde_json::from_str::<Value>(raw) else {
        return NotifyHookPlan::ReadOnlyFallback {
            reason: "notify is not a simple argv array",
        };
    };
    let command: Option<Vec<String>> = items
        .into_iter()
        .map(|item| item.as_str().map(ToOwned::to_owned))
        .collect();
    let Some(command) = command.filter(|command| {
        !command.is_empty()
            && command
                .iter()
                .all(|part| !part.is_empty() && !part.contains(['\n', '\r', ';', '|', '&']))
    }) else {
        return NotifyHookPlan::ReadOnlyFallback {
            reason: "notify command requires shell interpretation",
        };
    };
    if cookbench_command.is_empty()
        || cookbench_command
            .iter()
            .any(|part| part.is_empty() || part.contains(['\n', '\r']))
    {
        return NotifyHookPlan::ReadOnlyFallback {
            reason: "Cookbench hook command is invalid",
        };
    }
    NotifyHookPlan::Chain {
        existing_command: command,
        cookbench_command: cookbench_command.to_vec(),
    }
}
