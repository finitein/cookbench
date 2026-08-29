mod dedupe;
pub mod event;
mod queue;
mod rules;
mod template;

pub use dedupe::{DedupeKey, DedupeWindow};
pub use queue::{BoundedQueue, QueueItem, QueuePushResult};
pub use rules::{
    evaluate, DestinationId, NotificationContext, NotificationEventKind, NotificationRule,
    NotificationSettings, RuleDecision, RuleScope,
};
pub use template::{Template, TemplateContext, TemplateError, MAX_RENDERED_MESSAGE_BYTES};
