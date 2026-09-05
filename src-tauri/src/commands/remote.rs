//! Configuration commands for read-only zero-install SSH sources.

use cookbench_core::{
    persistence::RemoteSourceConfig,
    remote::{RemoteHost, SessionRoot},
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

use crate::{app_state::AppState, remote::runtime::RemoteRuntimeState};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSourceWire {
    pub id: String,
    pub alias: String,
    pub session_roots: Vec<String>,
    pub enabled: bool,
    pub bridge_enabled: bool,
    pub bridge_binary_path: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteSourceInput {
    pub id: Option<String>,
    pub alias: String,
    pub session_roots: Vec<String>,
    pub enabled: bool,
    pub bridge_enabled: bool,
    pub bridge_binary_path: Option<String>,
}

#[tauri::command]
pub fn get_remote_sources(state: State<'_, AppState>) -> Vec<RemoteSourceWire> {
    wires(&state.persisted_config().remote_sources)
}

#[tauri::command]
pub fn configure_remote_source(
    input: RemoteSourceInput,
    app: AppHandle,
    state: State<'_, AppState>,
    runtime: State<'_, RemoteRuntimeState>,
) -> Result<Vec<RemoteSourceWire>, String> {
    validate_input(&input)?;
    let id = input.id.unwrap_or_else(|| input.alias.clone());
    if id.len() > 256 || id.chars().any(char::is_control) {
        return Err("invalid SSH source identifier".to_owned());
    }
    state.update_persisted_config(|config| {
        config.remote_sources.retain(|source| source.id != id);
        config.remote_sources.push(RemoteSourceConfig {
            id,
            alias: input.alias,
            session_roots: input.session_roots,
            enabled: input.enabled,
            bridge_enabled: input.bridge_enabled,
            bridge_binary_path: input.bridge_binary_path,
        });
        config
            .remote_sources
            .sort_by(|left, right| left.id.cmp(&right.id));
    })?;
    let configured = state.persisted_config().remote_sources;
    runtime.reconfigure(app, &configured)?;
    Ok(wires(&configured))
}

#[tauri::command]
pub fn remove_remote_source(
    id: String,
    app: AppHandle,
    state: State<'_, AppState>,
    runtime: State<'_, RemoteRuntimeState>,
) -> Result<Vec<RemoteSourceWire>, String> {
    if id.len() > 256 || id.chars().any(char::is_control) {
        return Err("invalid SSH source identifier".to_owned());
    }
    state.update_persisted_config(|config| {
        config.remote_sources.retain(|source| source.id != id);
    })?;
    let configured = state.persisted_config().remote_sources;
    runtime.reconfigure(app, &configured)?;
    Ok(wires(&configured))
}

fn validate_input(input: &RemoteSourceInput) -> Result<(), String> {
    if input.bridge_binary_path.as_ref().is_some_and(|path| {
        path.len() > 4_096
            || path.contains('\0')
            || (!path.is_empty() && !std::path::Path::new(path).is_absolute())
    }) {
        return Err("bridge binary path must be an absolute local path".to_owned());
    }
    let roots = input
        .session_roots
        .iter()
        .take(16)
        .map(|root| SessionRoot::new(root.clone()).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    if roots.len() != input.session_roots.len() {
        return Err("too many SSH session roots".to_owned());
    }
    RemoteHost::new(input.alias.clone(), roots)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn wires(configured: &[RemoteSourceConfig]) -> Vec<RemoteSourceWire> {
    configured
        .iter()
        .take(16)
        .map(|source| RemoteSourceWire {
            id: source.id.clone(),
            alias: source.alias.clone(),
            session_roots: source.session_roots.clone(),
            enabled: source.enabled,
            bridge_enabled: source.bridge_enabled,
            bridge_binary_path: source.bridge_binary_path.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{validate_input, RemoteSourceInput};

    fn input(alias: &str, roots: &[&str]) -> RemoteSourceInput {
        RemoteSourceInput {
            id: None,
            alias: alias.to_owned(),
            session_roots: roots.iter().map(|root| (*root).to_owned()).collect(),
            enabled: true,
            bridge_enabled: false,
            bridge_binary_path: None,
        }
    }

    #[test]
    fn rejects_whitespace_aliases_with_an_honest_message() {
        let error = validate_input(&input("bad alias", &[])).unwrap_err();
        assert!(error.contains("whitespace-free"), "{error}");
    }

    #[test]
    fn rejects_relative_session_roots_without_hanging() {
        let error = validate_input(&input("fixture-host", &["relative/root"])).unwrap_err();
        assert!(error.contains("absolute"), "{error}");
    }

    #[test]
    fn accepts_automatic_roots_and_absolute_custom_roots() {
        assert!(validate_input(&input("fixture-host", &[])).is_ok());
        assert!(validate_input(&input("fixture-host", &["/srv/sessions"])).is_ok());
    }

    #[test]
    fn rejects_relative_bridge_binary_paths() {
        let mut value = input("fixture-host", &[]);
        value.bridge_binary_path = Some("bridge".into());
        let error = validate_input(&value).unwrap_err();
        assert!(error.contains("absolute"), "{error}");
    }
}
