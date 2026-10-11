//! Application: coordena o editor sem conhecer janela, GPU, plataforma nem filesystem.
//!
//! Contrato de `website/docs/02-application/architecture-decision.md`: a
//! `Application` possui o estado do editor, os módulos internos e a política de
//! autosave; tudo que é efeito do mundo (relógio, variáveis de ambiente, locks,
//! watchers, diálogos) entra por parâmetros ou fica no `DesktopHost`.
//! Construir uma `Application` não lê ambiente, não consulta o relógio, não cria
//! locks e não toca o filesystem.

mod actions;
mod demo;
#[cfg(feature = "lua-plugins")]
pub mod plugins;
mod smoke;

pub use actions::ActionOutcome;
pub use smoke::smoke_test;

use petunia_core::{AppEvent, AppState, AutosaveService, AutosaveStorage, ModuleRegistry};
use petunia_module_assets::AssetsModule;
use petunia_module_model::ToolRegistry;
use petunia_module_paint::PaintModule;
use petunia_module_uv::UvModule;

/// Relógio injetado (segundos desde a época Unix).
pub trait Clock {
    fn now_secs(&self) -> u64;
}

/// Relógio fixo para testes e execuções determinísticas.
#[derive(Debug, Clone, Copy, Default)]
pub struct FixedClock(pub u64);

impl Clock for FixedClock {
    fn now_secs(&self) -> u64 {
        self.0
    }
}

/// Configuração explícita da Application.
#[derive(Debug, Clone)]
pub struct ApplicationConfig {
    /// Idioma da interface (`en`, `pt-BR`).
    pub language: String,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
        }
    }
}

/// Resultado de um tick de autosave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutosaveOutcome {
    /// Snapshot gravado (nome do arquivo).
    Saved(String),
    /// Falha ao gravar (mensagem para o status).
    Failed(String),
}

pub struct Application {
    pub state: AppState,
    pub tools: ToolRegistry,
    pub registry: ModuleRegistry,
    pub autosave: AutosaveService,
    /// Plugins Lua de terceiros (feature `lua-plugins`).
    #[cfg(feature = "lua-plugins")]
    pub plugins: plugins::LuaPlugins,
}

impl Application {
    pub fn new(config: ApplicationConfig) -> Self {
        let mut registry = ModuleRegistry::new();
        registry.register(PaintModule::new());
        registry.register(UvModule::new());
        registry.register(AssetsModule::new());
        Self {
            state: AppState::new(&config.language),
            tools: ToolRegistry::with_defaults(),
            registry,
            autosave: AutosaveService::default(),
            #[cfg(feature = "lua-plugins")]
            plugins: plugins::LuaPlugins::default(),
        }
    }

    /// Política de autosave: decide *quando* salvar (relógio e revisão do documento);
    /// o `storage` injetado decide *como e onde* gravar.
    pub fn tick_autosave(
        &mut self,
        clock: &dyn Clock,
        storage: &dyn AutosaveStorage,
    ) -> Option<AutosaveOutcome> {
        let is_dirty = self.state.is_document_dirty();
        let project_path = self
            .state
            .project
            .project_path
            .as_deref()
            .map(std::path::Path::new);
        let result = self.autosave.tick_with(
            storage,
            clock.now_secs(),
            is_dirty,
            self.state.project.project.revision_id(),
            &self.state.project.project,
            project_path,
        )?;
        let outcome = match result {
            Ok(path) => AutosaveOutcome::Saved(
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("snapshot")
                    .to_string(),
            ),
            Err(error) => AutosaveOutcome::Failed(error.to_string()),
        };
        self.state.set_status(match &outcome {
            AutosaveOutcome::Saved(name) => format!("autosave: {name}"),
            AutosaveOutcome::Failed(error) => format!("autosave err: {error}"),
        });
        Some(outcome)
    }

    /// Entrega um evento (fato já ocorrido) aos módulos internos.
    pub fn dispatch_event(&mut self, event: &AppEvent) {
        self.registry.dispatch(event, &mut self.state);
        #[cfg(feature = "lua-plugins")]
        for failure in self.plugins.process_event(&mut self.state, event) {
            self.state
                .set_status(format!("plugin {}: {}", failure.plugin_id, failure.error));
        }
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new(ApplicationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_is_pure_and_starts_clean() {
        let app = Application::new(ApplicationConfig::default());
        assert!(!app.state.project.assets.is_empty());
        assert!(!app.state.is_document_dirty());
    }

    #[test]
    fn autosave_stays_quiet_for_a_clean_document() {
        let mut app = Application::default();
        assert!(
            app.tick_autosave(&FixedClock(10_000), &petunia_core::FileAutosaveStorage)
                .is_none()
        );
    }
}
