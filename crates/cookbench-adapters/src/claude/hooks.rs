use serde_json::{json, Map, Value};

const HOOK_COMMAND: &str = "cookbench-hook --harness claude-code";
const MAX_HOOK_ARGUMENT_BYTES: usize = 512;
const HOOK_EVENTS: &[&str] = &[
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "Stop",
    "SubagentStart",
    "SubagentStop",
    "Notification",
    "SessionEnd",
];

/// Filesystem code must create a backup before applying a changed transform.
/// This pure model makes that requirement explicit without giving the adapter
/// authority to edit Claude configuration itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HookBackupIntent {
    pub required: bool,
    pub reason: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HookMutation {
    pub configuration: Value,
    pub changed: bool,
    pub backup: HookBackupIntent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HookMutationError {
    RootNotObject,
    HooksNotObject,
    EventNotArray(String),
    HookEntryNotObject(String),
    HookListNotArray(String),
    UnsafeAmbiguousEmptyHooks,
    UnsafeCommand,
}

impl std::fmt::Display for HookMutationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RootNotObject => formatter.write_str("Claude settings root must be an object"),
            Self::HooksNotObject => formatter.write_str("Claude hooks must be an object"),
            Self::EventNotArray(event) => {
                write!(formatter, "Claude hook event {event} is not an array")
            }
            Self::HookEntryNotObject(event) => {
                write!(formatter, "Claude hook entry for {event} is not an object")
            }
            Self::HookListNotArray(event) => {
                write!(formatter, "Claude hook list for {event} is not an array")
            }
            Self::UnsafeAmbiguousEmptyHooks => {
                formatter.write_str("refusing to mutate an ambiguous empty Claude hooks object")
            }
            Self::UnsafeCommand => formatter.write_str("hook command is not a bounded safe argv"),
        }
    }
}

impl std::error::Error for HookMutationError {}

pub fn install_hooks(configuration: &Value) -> Result<HookMutation, HookMutationError> {
    install_hooks_with_group(configuration, managed_shell_group())
}

pub fn install_hooks_with_command(
    configuration: &Value,
    command: &str,
    args: &[&str],
) -> Result<HookMutation, HookMutationError> {
    validate_command(command, args)?;
    install_hooks_with_group(configuration, managed_exec_group(command, args))
}

fn install_hooks_with_group(
    configuration: &Value,
    managed_group: Value,
) -> Result<HookMutation, HookMutationError> {
    let mut next = configuration.clone();
    validate_configuration(&next)?;
    if next
        .get("hooks")
        .is_some_and(|hooks| hooks.as_object().is_some_and(Map::is_empty))
    {
        return Err(HookMutationError::UnsafeAmbiguousEmptyHooks);
    }
    let root = next.as_object_mut().expect("validated root object");
    let hooks = root
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()));
    let hooks = hooks.as_object_mut().expect("validated hooks object");
    let mut changed = false;
    for event in HOOK_EVENTS {
        let groups = hooks
            .entry(*event)
            .or_insert_with(|| Value::Array(Vec::new()));
        let groups = groups.as_array_mut().expect("validated hook event array");
        if !contains_managed_hook(groups, &managed_group) {
            groups.push(managed_group.clone());
            changed = true;
        }
    }
    Ok(HookMutation {
        configuration: next,
        changed,
        backup: backup_intent(changed),
    })
}

pub fn uninstall_hooks(configuration: &Value) -> Result<HookMutation, HookMutationError> {
    uninstall_hooks_with_group(configuration, managed_shell_group())
}

pub fn uninstall_hooks_with_command(
    configuration: &Value,
    command: &str,
    args: &[&str],
) -> Result<HookMutation, HookMutationError> {
    validate_command(command, args)?;
    uninstall_hooks_with_group(configuration, managed_exec_group(command, args))
}

/// Removes every Cookbench-owned Claude hook shape across upgrades while
/// preserving unrelated command groups byte-for-byte in the JSON model.
pub fn uninstall_all_cookbench_hooks(
    configuration: &Value,
) -> Result<HookMutation, HookMutationError> {
    let mut next = configuration.clone();
    validate_configuration(&next)?;
    let root = next.as_object_mut().expect("validated root object");
    let Some(hooks) = root.get_mut("hooks") else {
        return Ok(HookMutation {
            configuration: next,
            changed: false,
            backup: backup_intent(false),
        });
    };
    let hooks = hooks.as_object_mut().expect("validated hooks object");
    let mut changed = false;
    let mut emptied_events = Vec::new();
    for event in HOOK_EVENTS {
        let Some(groups) = hooks.get_mut(*event) else {
            continue;
        };
        let groups = groups.as_array_mut().expect("validated hook event array");
        let original_len = groups.len();
        groups.retain(|group| !is_cookbench_managed_group(group));
        changed |= groups.len() != original_len;
        if groups.is_empty() && original_len > 0 {
            emptied_events.push(*event);
        }
    }
    for event in emptied_events {
        hooks.remove(event);
    }
    if hooks.is_empty() {
        root.remove("hooks");
    }
    Ok(HookMutation {
        configuration: next,
        changed,
        backup: backup_intent(changed),
    })
}

fn uninstall_hooks_with_group(
    configuration: &Value,
    managed_group: Value,
) -> Result<HookMutation, HookMutationError> {
    let mut next = configuration.clone();
    validate_configuration(&next)?;
    let root = next.as_object_mut().expect("validated root object");
    let Some(hooks) = root.get_mut("hooks") else {
        return Ok(HookMutation {
            configuration: next,
            changed: false,
            backup: backup_intent(false),
        });
    };
    let hooks = hooks.as_object_mut().expect("validated hooks object");
    let mut changed = false;
    let mut empty_managed_events = Vec::new();
    for event in HOOK_EVENTS {
        let Some(groups) = hooks.get_mut(*event) else {
            continue;
        };
        let groups = groups.as_array_mut().expect("validated hook event array");
        let original_len = groups.len();
        let only_managed = groups.iter().all(|group| group == &managed_group);
        groups.retain(|group| group != &managed_group);
        changed |= groups.len() != original_len;
        if only_managed {
            empty_managed_events.push(*event);
        }
    }
    // Empty event lists created solely by Cookbench are harmless but removing
    // them makes uninstall deterministic without touching unrelated events.
    for event in empty_managed_events {
        hooks.remove(event);
    }
    if hooks.is_empty() {
        root.remove("hooks");
    }
    Ok(HookMutation {
        configuration: next,
        changed,
        backup: backup_intent(changed),
    })
}

fn backup_intent(changed: bool) -> HookBackupIntent {
    HookBackupIntent {
        required: changed,
        reason: "preserve the exact native settings before a Cookbench hook mutation",
    }
}

fn managed_shell_group() -> Value {
    json!({ "matcher": "*", "hooks": [{ "type": "command", "command": HOOK_COMMAND }] })
}

fn managed_exec_group(command: &str, args: &[&str]) -> Value {
    json!({ "matcher": "*", "hooks": [{ "type": "command", "command": command, "args": args }] })
}

fn contains_managed_hook(groups: &[Value], expected: &Value) -> bool {
    groups.iter().any(|group| group == expected)
}

fn is_cookbench_managed_group(group: &Value) -> bool {
    let Some(group) = group.as_object() else {
        return false;
    };
    if group.get("matcher").and_then(Value::as_str) != Some("*") {
        return false;
    }
    let Some([handler]) = group
        .get("hooks")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
    else {
        return false;
    };
    let Some(handler) = handler.as_object() else {
        return false;
    };
    if handler.get("type").and_then(Value::as_str) != Some("command") {
        return false;
    }
    let Some(command) = handler.get("command").and_then(Value::as_str) else {
        return false;
    };
    if command == HOOK_COMMAND && handler.get("args").is_none() {
        return true;
    }
    let executable = command
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(command)
        .to_ascii_lowercase();
    let expected_executable = executable == "cookbench-hook" || executable == "cookbench-hook.exe";
    let expected_args = handler
        .get("args")
        .and_then(Value::as_array)
        .is_some_and(|args| {
            args.as_slice()
                == [
                    Value::String("--harness".into()),
                    Value::String("claude-code".into()),
                ]
        });
    expected_executable && expected_args
}

fn validate_command(command: &str, args: &[&str]) -> Result<(), HookMutationError> {
    let safe = !command.is_empty()
        && command.len() <= MAX_HOOK_ARGUMENT_BYTES
        && !command.chars().any(char::is_control)
        && args.len() <= 8
        && args.iter().all(|argument| {
            !argument.is_empty()
                && argument.len() <= MAX_HOOK_ARGUMENT_BYTES
                && !argument.chars().any(char::is_control)
        });
    if safe {
        Ok(())
    } else {
        Err(HookMutationError::UnsafeCommand)
    }
}

fn validate_configuration(configuration: &Value) -> Result<(), HookMutationError> {
    let root = configuration
        .as_object()
        .ok_or(HookMutationError::RootNotObject)?;
    let Some(hooks) = root.get("hooks") else {
        return Ok(());
    };
    let hooks = hooks.as_object().ok_or(HookMutationError::HooksNotObject)?;
    for (event, groups) in hooks {
        let groups = groups
            .as_array()
            .ok_or_else(|| HookMutationError::EventNotArray(event.clone()))?;
        for group in groups {
            let group = group
                .as_object()
                .ok_or_else(|| HookMutationError::HookEntryNotObject(event.clone()))?;
            if let Some(hook_list) = group.get("hooks") {
                hook_list
                    .as_array()
                    .ok_or_else(|| HookMutationError::HookListNotArray(event.clone()))?;
            }
        }
    }
    Ok(())
}
