//! Serviço de plugins Lua da Application (feature `lua-plugins`, E01).
//!
//! Qualquer host (Slint, legado, headless) entrega aqui cada evento que
//! drenou do `EventBus`; o serviço traduz o fato para o evento público, entrega
//! aos plugins ativos e aplica os intents de cada plugin como **uma** transação
//! atômica (website/docs/13-extensions/lua-plugins-public-api.md).
//!
//! Proteção contra laço: os eventos gerados pelos batches de plugins vão para
//! a fila normal (módulos internos e UI os veem), mas **não** são entregues de
//! novo a plugins — um plugin que reage a `mesh.changed` mudando a malha não
//! dispara a si mesmo para sempre.

use petunia_core::{AppEvent, AppState};
use petunia_plugins::{HostConfig, PluginApiError, PluginId, PluginRegistry, apply_intents};

/// Plugins Lua instalados e a proteção contra eco de eventos.
#[derive(Default)]
pub struct LuaPlugins {
    pub registry: PluginRegistry,
    /// Eventos na fila que nasceram de batches de plugins (FIFO).
    pending_echoes: usize,
}

/// Falha ao aplicar o batch de um plugin (o documento ficou intacto).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginBatchFailure {
    pub plugin_id: String,
    pub error: PluginApiError,
}

impl LuaPlugins {
    pub fn new(config: HostConfig) -> Self {
        Self {
            registry: PluginRegistry::new(config),
            pending_echoes: 0,
        }
    }

    /// Nome e payload JSON públicos de um fato da Application, ou `None` para
    /// eventos que não são expostos a plugins.
    pub fn public_event(event: &AppEvent) -> Option<(&'static str, String)> {
        let asset = |id: String| serde_json::json!({ "object_id": id }).to_string();
        match event {
            AppEvent::MeshChanged { asset_id } => Some(("mesh.changed", asset(asset_id.to_string()))),
            AppEvent::ActiveAssetChanged { asset_id } => {
                Some(("active_object.changed", asset(asset_id.to_string())))
            }
            AppEvent::TextureChanged { asset_id } => Some(("texture.changed", asset(asset_id.to_string()))),
            AppEvent::SelectionChanged(selection) => Some((
                "selection.changed",
                serde_json::json!({ "faces": selection.faces.len(), "verts": selection.verts.len() })
                    .to_string(),
            )),
            AppEvent::ProjectLoaded => Some(("project.loaded", "{}".to_string())),
            _ => None,
        }
    }

    /// Entrega um evento drenado do `EventBus` aos plugins e aplica os batches.
    /// Devolve as falhas (cada uma já revertida por inteiro).
    pub fn process_event(
        &mut self,
        state: &mut AppState,
        event: &AppEvent,
    ) -> Vec<PluginBatchFailure> {
        if self.pending_echoes > 0 {
            self.pending_echoes -= 1;
            return Vec::new();
        }
        let Some((name, payload)) = Self::public_event(event) else {
            return Vec::new();
        };
        let mut failures = Vec::new();
        for (plugin_id, intents) in self.registry.deliver(name, &payload) {
            let Ok(id) = PluginId::new(&plugin_id) else {
                continue;
            };
            let before = state.events.pending();
            if let Err(error) = apply_intents(state, &id, &intents) {
                failures.push(PluginBatchFailure {
                    plugin_id: plugin_id.clone(),
                    error,
                });
            }
            self.pending_echoes += state.events.pending().saturating_sub(before);
        }
        failures
    }
}
