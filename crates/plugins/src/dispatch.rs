//! Apply validated Lua intents through the Application command spine.
//!
//! Website/docs/13-extensions/lua-plugins-public-api.md, "API e comandos
//! públicos": um batch de plugin é **atômico** — commit único (uma entrada de
//! Undo rotulada com o plugin) ou nada; IDs obsoletos falham com
//! `StaleReference`, nunca atingem outro objeto.

use petunia_core::{AppState, CommandError, CommandIntent};

use crate::host::{PluginId, ValidatedIntent};

/// Erros estruturados da API pública de plugins.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PluginApiError {
    #[error("invalid id: {0}")]
    InvalidId(String),
    #[error("stale reference: {0}")]
    StaleReference(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("validation failed: {0}")]
    ValidationFailed(String),
    #[error("version mismatch: {0}")]
    VersionMismatch(String),
    #[error("busy")]
    Busy,
    #[error("cancelled")]
    Cancelled,
    #[error("unsupported: {0}")]
    Unsupported(String),
}

impl From<CommandError> for PluginApiError {
    fn from(error: CommandError) -> Self {
        match error {
            CommandError::StaleReference(id) => Self::StaleReference(id),
            CommandError::InvalidAssetIndex(i) => Self::InvalidId(i.to_string()),
            CommandError::UnknownCommand(c) | CommandError::UnknownPrimitive(c) => {
                Self::Unsupported(c)
            }
            CommandError::InvalidMode(mode) => Self::Unsupported(format!("{mode:?}")),
            other => Self::ValidationFailed(other.to_string()),
        }
    }
}

/// Aplica os commands de um plugin como **um** batch atômico. Commands que
/// não mudam nada (`NoChange`) não abortam o batch.
pub fn apply_intents(
    state: &mut AppState,
    plugin: &PluginId,
    intent: &ValidatedIntent,
) -> Result<(), PluginApiError> {
    if intent.commands.is_empty() {
        return Ok(());
    }
    let label = format!("plugin {}", plugin.as_str());
    state
        .transaction(&label, |state| {
            for cmd in &intent.commands {
                let application = CommandIntent {
                    command: cmd.command.clone(),
                    asset_id: cmd.target.clone(),
                    args: cmd.args.clone(),
                };
                match state.dispatch_intent(&application) {
                    Ok(()) | Err(CommandError::NoChange(_)) => {}
                    Err(error) => return Err(error),
                }
            }
            Ok(())
        })
        .map_err(PluginApiError::from)
}

/// Answers a recorded plugin query with a real JSON payload (never `"{}"`).
pub fn answer_query(state: &AppState, name: &str) -> Result<String, PluginApiError> {
    let json = match name {
        "scene.list" | "scene.hierarchy" => serde_json::to_string(&state.query_scene_hierarchy()),
        "scene.selection" => serde_json::to_string(&state.query_selection_details()),
        "scene.tools" => serde_json::to_string(&state.query_tool_status()),
        other => return Err(PluginApiError::Unsupported(other.to_string())),
    };
    json.map_err(|e| PluginApiError::ValidationFailed(e.to_string()))
}

/// Answers a recorded plugin query, with contextual plugin awareness (e.g. `plugin.data`).
pub fn answer_plugin_query(
    state: &AppState,
    plugin: &PluginId,
    name: &str,
) -> Result<String, PluginApiError> {
    if name == "plugin.data" {
        let payload = state.project.plugin_data.get(plugin.as_str());
        let json = serde_json::to_string(&payload)
            .map_err(|e| PluginApiError::ValidationFailed(e.to_string()))?;
        return Ok(json);
    }
    answer_query(state, name)
}

/// Salva dados autorais namespaced de um plugin no Document.
///
/// Valida formato de namespace, schema versionado e limite de tamanho.
pub fn set_plugin_data(
    state: &mut AppState,
    plugin: &PluginId,
    schema_version: u32,
    data: serde_json::Value,
) -> Result<(), PluginApiError> {
    let payload = petunia_project::plugin_data::PluginPayload {
        schema_version,
        data,
    };
    petunia_project::plugin_data::put_payload(
        &mut state.project.plugin_data,
        plugin.as_str(),
        payload,
    )
    .map_err(|e| match e {
        petunia_project::plugin_data::PluginDataError::BadNamespace(s) => {
            PluginApiError::InvalidId(s)
        }
        petunia_project::plugin_data::PluginDataError::TooLarge(size) => {
            PluginApiError::ValidationFailed(format!("plugin payload too large: {size} bytes"))
        }
    })
}

/// Lê os dados autorais namespaced de um plugin no Document.
pub fn get_plugin_data<'a>(
    state: &'a AppState,
    plugin: &PluginId,
) -> Option<&'a petunia_project::plugin_data::PluginPayload> {
    state.project.plugin_data.get(plugin.as_str())
}

/// Identifica plugins cujos dados estão presentes no Document, mas o plugin
/// não está instalado no registry (produzindo o diagnóstico canônico sem descartar o payload).
pub fn detect_missing_plugins(
    state: &AppState,
    registry: &crate::registry::PluginRegistry,
) -> Vec<String> {
    petunia_project::plugin_data::missing_plugins(&state.project.plugin_data, |id| {
        registry.is_installed(id)
    })
    .map(ToString::to_string)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::host::{Host, LuaCommand, PLUGIN_API_VERSION, PluginCapabilities};
    use petunia_core::ProjectService;

    fn id() -> PluginId {
        PluginId::new("test.add").unwrap()
    }

    #[test]
    fn lua_intent_reaches_application_commands() {
        let mut state = AppState::new("en");
        ProjectService::new_project(&mut state);
        let before = state.project.assets.len();
        let host = Host::new();
        let caps = PluginCapabilities {
            api_version: PLUGIN_API_VERSION,
            commands: vec!["model.add_primitive".to_string()],
            queries: vec!["scene.list".to_string()],
            filesystem_roots: vec![],
            allow_network: false,
            allow_destructive: false,
        };
        let intent = host
            .execute(&id(), &caps, r#"petunia.command("model.add_primitive")"#)
            .unwrap();
        apply_intents(&mut state, &id(), &intent).unwrap();
        assert_eq!(state.project.assets.len(), before + 1);
        let json = answer_query(&state, "scene.list").unwrap();
        assert!(json.contains("Cube"));
        assert_ne!(json, "{}");
    }

    #[test]
    fn a_failing_command_aborts_the_whole_batch() {
        let mut state = AppState::new("en");
        ProjectService::new_project(&mut state);
        let before = state.project.assets.len();
        let undo_before = state.project.undo.depth();
        let intent = ValidatedIntent {
            commands: vec![
                LuaCommand {
                    command: "model.add_primitive".into(),
                    args: vec![],
                    target: None,
                },
                LuaCommand {
                    command: "model.subdivide".into(),
                    args: vec![1.0],
                    target: Some(uuid_like_stale()),
                },
            ],
            queries: vec![],
        };
        let error = apply_intents(&mut state, &id(), &intent).unwrap_err();
        assert!(matches!(error, PluginApiError::StaleReference(_)));
        assert_eq!(state.project.assets.len(), before);
        assert_eq!(state.project.undo.depth(), undo_before);
    }

    #[test]
    fn a_successful_batch_is_one_labelled_undo_step() {
        let mut state = AppState::new("en");
        ProjectService::new_project(&mut state);
        let (undo_before, _) = state.project.undo.depth();
        let add = LuaCommand {
            command: "model.add_primitive".into(),
            args: vec![],
            target: None,
        };
        let intent = ValidatedIntent {
            commands: vec![add.clone(), add],
            queries: vec![],
        };
        apply_intents(&mut state, &id(), &intent).unwrap();
        assert_eq!(state.project.undo.depth().0, undo_before + 1);
        assert_eq!(state.project.undo.undo_label(), Some("plugin test.add"));
    }

    fn uuid_like_stale() -> String {
        "00000000-0000-4000-8000-000000000000".to_string()
    }

    #[test]
    fn plugin_data_round_trip_and_missing_plugins_detection() {
        let mut state = AppState::new("en");
        ProjectService::new_project(&mut state);
        let pid = id();

        // 1. Initial query before data is set -> null json
        let initial_query = answer_plugin_query(&state, &pid, "plugin.data").unwrap();
        assert_eq!(initial_query, "null");

        // 2. Set namespaced plugin data
        let sample_payload = serde_json::json!({
            "mode": "auto_relax",
            "threshold": 0.42
        });
        set_plugin_data(&mut state, &pid, 1, sample_payload.clone()).unwrap();

        // 3. Query after data is set
        let query_after = answer_plugin_query(&state, &pid, "plugin.data").unwrap();
        assert!(query_after.contains("\"threshold\":0.42"));
        assert!(query_after.contains("\"schema_version\":1"));

        // 4. In-memory check
        let stored = get_plugin_data(&state, &pid).expect("payload must be stored");
        assert_eq!(stored.schema_version, 1);
        assert_eq!(stored.data, sample_payload);

        // 5. Missing plugins detection
        let registry = crate::registry::PluginRegistry::default();
        let missing = detect_missing_plugins(&state, &registry);
        assert_eq!(missing, vec!["test.add"]);
    }
}
