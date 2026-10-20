//! Contexto restrito de execução da camada Application.
//!
//! `CommandContext` enxerga somente o documento, a sessão de edição e a fila de
//! eventos (website/docs/02-application/commands-events-services.md). Ele **não**
//! alcança `UiState`, `RenderResources` nem o dispatcher: tudo o que um comando
//! precisa dizer à interface ou ao renderer sai como [`CommandEffects`], aplicado
//! pelo `AppState` depois que o comando termina.

use uuid::Uuid;

use crate::events::EventBus;
use crate::selection::Workspace;
use crate::state::{AppState, DirtyReason, EditorSession, ProjectState, TextureDirtyRect, UiState};

/// Pedidos de interface emitidos por comandos (intents, não fatos).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiRequest {
    ToggleReferenceManager,
    ToggleCommandPalette,
    ToggleSettings,
    ToggleHelp,
}

/// Operação de ciclo de vida do projeto que o host executa depois do comando
/// (I/O e recentes não pertencem ao contexto restrito).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostRequest {
    NewProject,
    SaveProject(std::path::PathBuf),
}

/// Invalidação de textura pedida por um comando.
#[derive(Debug, Clone, PartialEq)]
pub enum TextureOp {
    Full {
        asset_id: Uuid,
        width: u32,
        height: u32,
    },
    Regions {
        asset_id: Uuid,
        width: u32,
        height: u32,
        regions: Vec<TextureDirtyRect>,
    },
}

/// Saídas de um comando para fora do documento/editor.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct CommandEffects {
    /// Mensagem de status (última vence).
    pub status: Option<String>,
    /// Mensagens de diagnóstico opcionais da execução (A04).
    pub diagnostics: Vec<String>,
    /// Redesenho pedido, com o motivo mais recente quando conhecido.
    pub redraw: bool,
    pub redraw_reason: Option<DirtyReason>,
    pub textures: Vec<TextureOp>,
    /// Troca de workspace (primeiro anterior, último destino).
    pub workspace_change: Option<(Workspace, Workspace)>,
    pub ui: Vec<UiRequest>,
    pub host: Vec<HostRequest>,
    pub canvas_dirty: bool,
}

/// Fonte de textos traduzidos injetada no contexto.
pub trait TextSource {
    fn text(&self, key: &str) -> String;
    fn text_id(&self, id: petunia_config::TextId) -> String;
}

/// Fonte de texto neutra: devolve a própria chave (sem catálogo de idiomas).
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyText;

impl TextSource for KeyText {
    fn text(&self, key: &str) -> String {
        key.to_string()
    }
    fn text_id(&self, id: petunia_config::TextId) -> String {
        id.key().to_string()
    }
}

impl TextSource for UiState {
    fn text(&self, key: &str) -> String {
        self.t(key)
    }
    fn text_id(&self, id: petunia_config::TextId) -> String {
        self.t_id(id)
    }
}

/// Documento + editor + eventos, sem UI nem render.
pub struct CommandContext<'a> {
    pub document: &'a mut ProjectState,
    pub editor: &'a mut EditorSession,
    pub events: &'a mut EventBus,
    pub text: &'a dyn TextSource,
    pub effects: CommandEffects,
}

impl std::ops::Deref for CommandContext<'_> {
    type Target = EditorSession;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.editor
    }
}

impl std::ops::DerefMut for CommandContext<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.editor
    }
}

impl<'a> CommandContext<'a> {
    /// Monta um contexto a partir de partes independentes (serviços e testes).
    pub fn new(
        document: &'a mut ProjectState,
        editor: &'a mut EditorSession,
        events: &'a mut EventBus,
        text: &'a dyn TextSource,
    ) -> Self {
        Self {
            document,
            editor,
            events,
            text,
            effects: CommandEffects::default(),
        }
    }
}

impl CommandContext<'_> {
    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.effects.status = Some(msg.into());
    }

    /// Adiciona uma mensagem de diagnóstico pós-commit (A04).
    pub fn add_diagnostic(&mut self, msg: impl Into<String>) {
        self.effects.diagnostics.push(msg.into());
    }

    /// Pede redesenho (render-on-demand).
    pub fn mark_dirty(&mut self) {
        self.effects.redraw = true;
    }

    pub fn mark_dirty_reason(&mut self, reason: DirtyReason) {
        self.effects.redraw = true;
        self.effects.redraw_reason = Some(reason);
    }

    pub fn mark_texture_full(&mut self, asset_id: Uuid, width: u32, height: u32) {
        self.effects.textures.push(TextureOp::Full {
            asset_id,
            width,
            height,
        });
    }

    pub fn mark_texture_regions(
        &mut self,
        asset_id: Uuid,
        width: u32,
        height: u32,
        regions: impl IntoIterator<Item = TextureDirtyRect>,
    ) {
        self.effects.textures.push(TextureOp::Regions {
            asset_id,
            width,
            height,
            regions: regions.into_iter().collect(),
        });
    }

    pub(crate) fn note_workspace_change(&mut self, previous: Workspace, next: Workspace) {
        let first = self
            .effects
            .workspace_change
            .map_or(previous, |(first, _)| first);
        self.effects.workspace_change = Some((first, next));
    }

    /// Despacha outro comando dentro da mesma transação de contexto.
    pub fn dispatch(
        &mut self,
        cmd: &dyn crate::command::Command,
    ) -> Result<(), crate::command::CommandError> {
        crate::command::CommandDispatcher::run(self, cmd)
    }

    pub fn request_ui(&mut self, request: UiRequest) {
        self.effects.ui.push(request);
    }

    pub fn request_host(&mut self, request: HostRequest) {
        self.effects.host.push(request);
    }

    /// O canvas 2D precisa ser recomposto.
    pub fn mark_canvas_dirty(&mut self) {
        self.effects.canvas_dirty = true;
    }

    pub fn t(&self, key: &str) -> String {
        self.text.text(key)
    }

    pub fn t_id(&self, id: petunia_config::TextId) -> String {
        self.text.text_id(id)
    }

    /// Visão somente leitura do documento e do editor.
    pub fn view(&self) -> EditorView<'_> {
        EditorView {
            document: self.document,
            editor: self.editor,
        }
    }

    /// Consome o contexto devolvendo o que o comando pediu ao host.
    pub fn into_effects(self) -> CommandEffects {
        self.effects
    }
}

/// Consultas somente leitura sobre documento e editor (`Copy`: retornos podem
/// emprestar do documento sem depender da vida da própria visão).
#[derive(Clone, Copy)]
pub struct EditorView<'a> {
    pub document: &'a ProjectState,
    pub editor: &'a EditorSession,
}

impl std::ops::Deref for EditorView<'_> {
    type Target = EditorSession;
    #[inline]
    fn deref(&self) -> &Self::Target {
        self.editor
    }
}

impl AppState {
    /// Contexto restrito para executar comandos.
    pub fn command_context(&mut self) -> CommandContext<'_> {
        CommandContext {
            document: &mut self.project,
            editor: &mut self.session,
            events: &mut self.events,
            text: &self.ui,
            effects: CommandEffects::default(),
        }
    }

    /// Executa `f` no contexto restrito e aplica os efeitos ao terminar.
    pub fn in_context<R>(&mut self, f: impl FnOnce(&mut CommandContext<'_>) -> R) -> R {
        let mut ctx = self.command_context();
        let result = f(&mut ctx);
        let effects = ctx.into_effects();
        // `in_context` não tem canal de erro: falhas do host viram status.
        if let Err(error) = self.apply_effects(effects) {
            self.ui.set_status(error.to_string());
        }
        result
    }

    pub fn view(&self) -> EditorView<'_> {
        EditorView {
            document: &self.project,
            editor: &self.session,
        }
    }

    /// Aplica a UI, ao renderer e ao host o que o contexto pediu. O primeiro
    /// erro de I/O do host é devolvido ao chamador do comando.
    pub fn apply_effects(
        &mut self,
        effects: CommandEffects,
    ) -> Result<(), crate::command::CommandError> {
        if let Some(status) = effects.status {
            self.ui.set_status(status);
        }
        if let Some((previous, next)) = effects.workspace_change {
            self.ui.swap_workspace_memory(previous, next);
        }
        for request in effects.ui {
            match request {
                UiRequest::ToggleReferenceManager => {
                    self.ui.show_reference_manager = !self.ui.show_reference_manager;
                }
                UiRequest::ToggleCommandPalette => {
                    self.ui.show_command_palette = !self.ui.show_command_palette;
                    if self.ui.show_command_palette {
                        self.ui.command_palette_query.clear();
                        self.ui.command_palette_selected_index = 0;
                    }
                }
                UiRequest::ToggleSettings => {
                    self.ui.show_settings = !self.ui.show_settings;
                }
                UiRequest::ToggleHelp => {
                    self.ui.show_help = !self.ui.show_help;
                }
            }
        }
        for op in effects.textures {
            match op {
                TextureOp::Full {
                    asset_id,
                    width,
                    height,
                } => self.render.mark_texture_full(asset_id, width, height),
                TextureOp::Regions {
                    asset_id,
                    width,
                    height,
                    regions,
                } => self
                    .render
                    .mark_texture_regions(asset_id, width, height, regions),
            }
        }
        if effects.canvas_dirty {
            self.render.canvas_dirty = true;
        }
        match (effects.redraw, effects.redraw_reason) {
            (_, Some(reason)) => self.render.mark_dirty_reason(reason),
            (true, None) => self.render.mark_dirty(),
            _ => {}
        }
        let mut outcome = Ok(());
        for request in effects.host {
            let result = match request {
                HostRequest::NewProject => {
                    crate::project_service::ProjectService::new_project(self);
                    Ok(())
                }
                HostRequest::SaveProject(path) => {
                    crate::project_service::ProjectService::save_project(self, &path)
                        .map_err(|e| crate::command::CommandError::Execution(e.to_string()))
                }
            };
            if outcome.is_ok() {
                outcome = result;
            }
        }
        outcome
    }
}
