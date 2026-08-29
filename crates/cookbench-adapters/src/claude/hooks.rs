use serde_json::{json, Map, Value};

const HOOK_COMMAND: &str = "cookbench-hook --harness claude-code";
const HOOK_EVENTS: &[&str] = &[
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "Stop",
    "SubagentStart",
    "SubagentStop",
    "Notification",
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
        }
    }
}

impl std::error::Error for HookMutationError {}

pub fn install_hooks(configuration: &Value) -> Result<HookMutation, HookMutationError> {
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
        if !contains_managed_hook(groups) {
            groups.push(managed_group());
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
        let only_managed = groups.iter().all(is_managed_group);
        groups.retain(|group| !is_managed_group(group));
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

fn managed_group() -> Value {
    json!({ "matcher": "*", "hooks": [{ "type": "command", "command": HOOK_COMMAND }] })
}

fn contains_managed_hook(groups: &[Value]) -> bool {
    groups.iter().any(is_managed_group)
}

fn is_managed_group(group: &Value) -> bool {
    group.get("matcher").and_then(Value::as_str) == Some("*")
        && group
            .get("hooks")
            .and_then(Value::as_array)
            .is_some_and(|hooks| {
                hooks.len() == 1
                    && hooks[0].get("type").and_then(Value::as_str) == Some("command")
                    && hooks[0].get("command").and_then(Value::as_str) == Some(HOOK_COMMAND)
            })
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
