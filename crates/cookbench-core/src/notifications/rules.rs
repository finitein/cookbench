use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::domain::{HarnessId, HostIdentity, StoveState};

use super::template::Template;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct DestinationId(String);

impl DestinationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum NotificationEventKind {
    SessionAppeared,
    CookingStarted,
    PhaseChanged,
    NeedsHuman,
    ProgressMilestone,
    Cooked,
    Failed,
    Disconnected,
    ConnectionRestored,
    StoveCleared,
}

impl NotificationEventKind {
    pub const fn is_critical(self) -> bool {
        matches!(self, Self::NeedsHuman | Self::Failed | Self::Disconnected)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotificationContext {
    pub stove_id: String,
    pub project: String,
    pub host: HostIdentity,
    pub harness: HarnessId,
    pub destination: DestinationId,
    pub state: StoveState,
    pub event: NotificationEventKind,
    pub progress_percent: Option<u8>,
    pub milestone: Option<u8>,
    pub task: Option<String>,
    pub agent: Option<String>,
    pub activity: Option<String>,
    pub duration: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RuleScope {
    Project(String),
    Host(String),
    Harness(HarnessId),
    Destination(DestinationId),
    Stove(String),
}

impl RuleScope {
    const fn precedence(&self) -> u8 {
        match self {
            Self::Project(_) => 1,
            Self::Host(_) => 2,
            Self::Harness(_) => 3,
            Self::Destination(_) => 4,
            Self::Stove(_) => 5,
        }
    }

    fn matches(&self, context: &NotificationContext) -> bool {
        match self {
            Self::Project(project) => project == &context.project,
            Self::Host(host) => host == &context.host.id,
            Self::Harness(harness) => harness == &context.harness,
            Self::Destination(destination) => destination == &context.destination,
            Self::Stove(stove) => stove == &context.stove_id,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotificationRule {
    pub scope: Option<RuleScope>,
    pub enabled: Option<bool>,
    pub events: Option<BTreeSet<NotificationEventKind>>,
    pub milestones: Option<BTreeSet<u8>>,
    pub template: Option<Template>,
}

impl Default for NotificationRule {
    fn default() -> Self {
        Self {
            scope: None,
            enabled: Some(false),
            events: Some(BTreeSet::new()),
            milestones: None,
            template: None,
        }
    }
}

impl NotificationRule {
    pub fn enabled_for(events: impl IntoIterator<Item = NotificationEventKind>) -> Self {
        Self {
            enabled: Some(true),
            events: Some(events.into_iter().collect()),
            ..Self::default()
        }
    }

    pub fn for_scope(scope: RuleScope) -> Self {
        Self {
            scope: Some(scope),
            enabled: None,
            events: None,
            milestones: None,
            template: None,
        }
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn with_events(mut self, events: impl IntoIterator<Item = NotificationEventKind>) -> Self {
        self.events = Some(events.into_iter().collect());
        self
    }

    pub fn with_milestones(mut self, milestones: impl IntoIterator<Item = u8>) -> Self {
        self.milestones = Some(milestones.into_iter().collect());
        self
    }

    pub fn with_template(mut self, template: Template) -> Self {
        self.template = Some(template);
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub global: NotificationRule,
    pub rules: Vec<NotificationRule>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuleDecision {
    pub should_notify: bool,
    pub template: Option<Template>,
}

/// Applies scopes from global to stove. Later scopes are more specific and win.
/// It is deliberately pure so senders cannot affect stove processing.
pub fn evaluate(settings: &NotificationSettings, context: &NotificationContext) -> RuleDecision {
    let mut resolved = settings.global.clone();
    let mut matching: Vec<(u8, usize, &NotificationRule)> = settings
        .rules
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| {
            rule.scope
                .as_ref()
                .filter(|scope| scope.matches(context))
                .map(|scope| (scope.precedence(), index, rule))
        })
        .collect();
    matching.sort_by_key(|(precedence, index, _)| (*precedence, *index));
    for (_, _, rule) in matching {
        merge(&mut resolved, rule);
    }

    let is_selected = resolved
        .events
        .as_ref()
        .is_some_and(|events| events.contains(&context.event));
    let milestone_allowed = context.event != NotificationEventKind::ProgressMilestone
        || resolved.milestones.as_ref().is_none_or(|milestones| {
            context
                .milestone
                .is_some_and(|milestone| milestones.contains(&milestone))
        });
    RuleDecision {
        should_notify: resolved.enabled.unwrap_or(false) && is_selected && milestone_allowed,
        template: resolved.template,
    }
}

fn merge(target: &mut NotificationRule, override_rule: &NotificationRule) {
    if override_rule.enabled.is_some() {
        target.enabled = override_rule.enabled;
    }
    if override_rule.events.is_some() {
        target.events = override_rule.events.clone();
    }
    if override_rule.milestones.is_some() {
        target.milestones = override_rule.milestones.clone();
    }
    if override_rule.template.is_some() {
        target.template = override_rule.template.clone();
    }
}
