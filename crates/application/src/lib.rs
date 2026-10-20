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
pub mod host;
#[cfg(feature = "lua-plugins")]
pub mod plugins;
mod smoke;

pub use actions::ActionOutcome;
pub use host::{DesktopHost, SessionLockGuard};
pub use smoke::smoke_test;

use petunia_core::{AppEvent, AppState, AutosaveService, AutosaveStorage, ModuleRegistry};
use petunia_module_assets::AssetsModule;
use petunia_module_model::ToolRegistry;
use petunia_module_paint::PaintModule;
use petunia_module_uv::UvModule;

/// Relógio injetado: definido no core (única fonte), reexportado aqui.
pub use petunia_core::{Clock, FixedClock, SystemClock};

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
        Self::from_state(AppState::new(&config.language))
    }

    /// Adota uma sessão existente, sem recriar documento, histórico ou relógio.
    ///
    /// O host resolve as preferências e pode injetar serviços no `AppState`
    /// antes da transferência. A construção dos coordenadores internos não lê
    /// ambiente nem filesystem e não executa ações sobre a sessão recebida.
    pub fn from_state(state: AppState) -> Self {
        let mut registry = ModuleRegistry::new();
        registry.register(PaintModule::new());
        registry.register(UvModule::new());
        registry.register(AssetsModule::new());
        Self {
            state,
            tools: ToolRegistry::from_enabled(&std::collections::HashMap::new()),
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

    /// Processa eventos de domínio nativamente na Application, eliminando o acoplamento
    /// a ModuleRegistry dinâmico para subsistemas canônicos (Model, Paint, UV, Assets).
    pub fn process_domain_event(&mut self, event: &AppEvent) {
        match event {
            AppEvent::MeshChanged { .. } | AppEvent::ActiveAssetChanged { .. } => {
                let project = &self.state.project;
                if let Some(o) = project.assets.get(project.active) {
                    self.state.session.tools.uv_selection.retain_valid(&o.mesh);
                }
            }
            AppEvent::SelectionChanged(_) => {
                // Invalidações e sincronizações contextuais se necessário
            }
            _ => {}
        }
    }

    /// Entrega um evento (fato já ocorrido) aos domínios nativos, módulos legados e extensões.
    pub fn dispatch_event(&mut self, event: &AppEvent) {
        // 1. Processamento nativo direto de domínio (aposentadoria gradual de ModuleRegistry)
        self.process_domain_event(event);

        // 2. Despacho compatível para módulos legados
        self.registry.dispatch(event, &mut self.state);

        // 3. Extensões / Plugins Lua
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

    #[test]
    fn adopting_state_preserves_document_history_and_injected_clock() {
        let mut state = AppState::default();
        let clock: std::sync::Arc<dyn Clock> = std::sync::Arc::new(FixedClock(123));
        state.clock = std::sync::Arc::clone(&clock);
        state.dispatch_command("model.add_cube").unwrap();
        let object = state.project.active().unwrap().id;
        let revision = state.project.project.revision_id();
        let depth = state.project.undo.depth();

        let app = Application::from_state(state);

        assert_eq!(app.state.project.active().unwrap().id, object);
        assert_eq!(app.state.project.project.revision_id(), revision);
        assert_eq!(app.state.project.undo.depth(), depth);
        assert!(app.state.is_document_dirty());
        assert!(std::sync::Arc::ptr_eq(&app.state.clock, &clock));
    }

    #[test]
    fn application_command_failure_preserves_history_and_document() {
        let mut app = Application::default();
        let object = app.state.project.active().unwrap().id;
        let revision = app.state.project.project.revision_id();
        let depth = app.state.project.undo.depth();

        assert_eq!(
            app.execute_registered_command("unknown.command"),
            Err(petunia_core::CommandError::UnknownCommand(
                "unknown.command".into()
            ))
        );

        assert_eq!(app.state.project.active().unwrap().id, object);
        assert_eq!(app.state.project.project.revision_id(), revision);
        assert_eq!(app.state.project.undo.depth(), depth);
        assert!(!app.state.is_document_dirty());
    }
}
