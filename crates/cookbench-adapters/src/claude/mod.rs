//! Read-only Claude Code session discovery and event normalization.
//!
//! Claude's native transcript files remain authoritative. This module parses
//! only bounded structural fields and exposes no process-control operations.

mod discovery;
mod hooks;
mod parser;
mod tasks;

pub use discovery::{
    decode_project_path, default_projects_root, discover_session, encode_project_path,
};
pub use hooks::{
    install_hooks, install_hooks_with_command, uninstall_all_cookbench_hooks, uninstall_hooks,
    uninstall_hooks_with_command, HookBackupIntent, HookMutation, HookMutationError,
};
pub use parser::{parse_record, ParsedRecord};
pub use tasks::{extract_task_progress, TaskProgress};

use std::path::PathBuf;

use async_trait::async_trait;
use cookbench_core::domain::HarnessId;

use crate::{
    AdapterCapabilities, AdapterError, EventSink, HarnessAdapter, HostSource, NativeSession,
    ResumeAction, SessionLocator, WatchHandle,
};

#[derive(Clone, Debug)]
pub struct ClaudeAdapter {
    projects_root: PathBuf,
}

impl ClaudeAdapter {
    pub fn new(projects_root: PathBuf) -> Self {
        Self { projects_root }
    }

    pub fn from_environment() -> Result<Self, AdapterError> {
        Ok(Self::new(default_projects_root()?))
    }

    pub fn projects_root(&self) -> &std::path::Path {
        &self.projects_root
    }
}

#[async_trait]
impl HarnessAdapter for ClaudeAdapter {
    fn id(&self) -> HarnessId {
        HarnessId::ClaudeCode
    }

    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            discovery: true,
            // Hooks are handled by cookbench-hook. The adapter never claims a
            // watcher it cannot independently run.
            watch_events: false,
            structured_progress: true,
            locator: true,
            resume: true,
        }
    }

    async fn discover(&self, source: &HostSource) -> Result<Vec<NativeSession>, AdapterError> {
        discovery::discover_sessions(&self.projects_root, source)
    }

    async fn watch(&self, _sink: EventSink) -> Result<WatchHandle, AdapterError> {
        Err(AdapterError::UnsupportedCapability(
            "Claude Code watch events",
        ))
    }

    fn locate(&self, session: &NativeSession) -> Option<SessionLocator> {
        (session.harness == self.id()).then(|| session.locator.clone())
    }

    fn resume(&self, session: &NativeSession) -> Vec<ResumeAction> {
        if session.harness != self.id() {
            return Vec::new();
        }
        vec![
            ResumeAction::OpenSessionLocation(session.locator.clone()),
            ResumeAction::SuggestedCommand {
                program: "claude".to_owned(),
                args: vec!["--resume".to_owned(), session.native_session_id.clone()],
            },
        ]
    }
}
