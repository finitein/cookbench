use std::fmt;

use serde::{Deserialize, Serialize};

use super::rules::NotificationContext;

pub const MAX_RENDERED_MESSAGE_BYTES: usize = 4_096;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Template(String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemplateError {
    UnknownPlaceholder(String),
    UnclosedPlaceholder,
    EmptyOutputLimit,
}

impl fmt::Display for TemplateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPlaceholder(name) => {
                write!(formatter, "unknown notification placeholder {{{name}}}")
            }
            Self::UnclosedPlaceholder => formatter.write_str("unclosed notification placeholder"),
            Self::EmptyOutputLimit => {
                formatter.write_str("notification output limit must be positive")
            }
        }
    }
}

impl std::error::Error for TemplateError {}

impl Template {
    pub fn parse(source: impl Into<String>) -> Result<Self, TemplateError> {
        let source = source.into();
        validate(&source)?;
        Ok(Self(source))
    }

    pub fn render(
        &self,
        context: &TemplateContext,
        max_bytes: usize,
    ) -> Result<String, TemplateError> {
        if max_bytes == 0 {
            return Err(TemplateError::EmptyOutputLimit);
        }
        let limit = max_bytes.min(MAX_RENDERED_MESSAGE_BYTES);
        let mut result = String::new();
        let mut rest = self.0.as_str();
        while let Some(start) = rest.find('{') {
            append_bounded(&mut result, &rest[..start], limit);
            let placeholder = &rest[start + 1..];
            let Some(end) = placeholder.find('}') else {
                return Err(TemplateError::UnclosedPlaceholder);
            };
            append_bounded(&mut result, context.value(&placeholder[..end]), limit);
            rest = &placeholder[end + 1..];
        }
        append_bounded(&mut result, rest, limit);
        Ok(result)
    }
}

fn validate(source: &str) -> Result<(), TemplateError> {
    let mut rest = source;
    while let Some(start) = rest.find('{') {
        let placeholder = &rest[start + 1..];
        let Some(end) = placeholder.find('}') else {
            return Err(TemplateError::UnclosedPlaceholder);
        };
        if !matches!(
            &placeholder[..end],
            "project"
                | "task"
                | "agent"
                | "state"
                | "progress"
                | "activity"
                | "host"
                | "duration"
                | "completed_at"
        ) {
            return Err(TemplateError::UnknownPlaceholder(
                placeholder[..end].to_owned(),
            ));
        }
        rest = &placeholder[end + 1..];
    }
    Ok(())
}

fn append_bounded(output: &mut String, value: &str, limit: usize) {
    let available = limit.saturating_sub(output.len());
    if available == 0 {
        return;
    }
    let mut end = value.len().min(available);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    output.push_str(&value[..end]);
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateContext {
    project: String,
    task: String,
    agent: String,
    state: String,
    progress: String,
    activity: String,
    host: String,
    duration: String,
    completed_at: String,
}

impl From<&NotificationContext> for TemplateContext {
    fn from(context: &NotificationContext) -> Self {
        Self {
            project: context.project.clone(),
            task: context.task.clone().unwrap_or_default(),
            agent: context.agent.clone().unwrap_or_default(),
            state: format!("{:?}", context.state),
            progress: context
                .progress_percent
                .map(|value| format!("{value}%"))
                .unwrap_or_default(),
            activity: context.activity.clone().unwrap_or_default(),
            host: context.host.id.clone(),
            duration: context.duration.clone().unwrap_or_default(),
            completed_at: context.completed_at.clone().unwrap_or_default(),
        }
    }
}

impl TemplateContext {
    fn value(&self, name: &str) -> &str {
        match name {
            "project" => &self.project,
            "task" => &self.task,
            "agent" => &self.agent,
            "state" => &self.state,
            "progress" => &self.progress,
            "activity" => &self.activity,
            "host" => &self.host,
            "duration" => &self.duration,
            "completed_at" => &self.completed_at,
            _ => "",
        }
    }
}
