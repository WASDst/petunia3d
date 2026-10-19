use std::collections::HashMap;
use std::sync::Arc;

use petunia_mesh::Mesh;
use petunia_project::{
    PathGenerator, PathGeneratorError, PathGeneratorEvaluationCache, PathGeneratorQuality,
    ProfileError, ProfileResource, ProjectChanges, SplineError, SplineHandleMode, SplinePoint,
    SplineResource, SurfaceAttachment, SurfaceAttachmentError, SurfaceAttachmentStatus,
    SweepGeneratorParameters, detach_surface_attachment_keep_world, evaluate_surface_attachment,
    reproject_surface_attachment_to_target, slide_surface_attachment,
};

use crate::camera::ViewPreset;
use crate::docs::DocsTopic;
use crate::selection::SelectionDomain;
use crate::state::{AppState, EditMode};

/// Taxonomia de erros de comandos da aplicação.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum CommandError {
    #[error("Nenhum asset ativo selecionado")]
    NoActiveAsset,
    #[error("Índice de asset inválido: {0}")]
    InvalidAssetIndex(usize),
    #[error("Comando não suportado no modo de edição atual: {0:?}")]
    InvalidMode(EditMode),
    #[error("Nenhum elemento selecionado")]
    EmptySelection,
    #[error("Primitiva desconhecida: {0}")]
    UnknownPrimitive(String),
    #[error("Comando não registrado: {0}")]
    UnknownCommand(String),
    #[error("Erro ao executar comando: {0}")]
    Execution(String),
    /// O comando rodou mas não alterou nada: o dispatcher desfaz qualquer
    /// efeito colateral e **não** grava entrada de histórico nem avança revisões.
    #[error("Nada a alterar: {0}")]
    NoChange(String),
    /// Referência a um objeto que não existe (mais): nunca cai em outro objeto.
    #[error("Referência inválida ou obsoleta: {0}")]
    StaleReference(String),
    #[error(transparent)]
    Spline(#[from] SplineError),
    #[error(transparent)]
    SurfaceAttachment(#[from] SurfaceAttachmentError),
    #[error(transparent)]
    Profile(#[from] ProfileError),
    #[error(transparent)]
    PathGenerator(#[from] PathGeneratorError),
}

pub use crate::context::{CommandContext, CommandEffects, EditorView, HostRequest, UiRequest};

/// Relatório DTO pós-execução de um comando consolidado (A04).
/// Devolve rótulo, invalidações (ProjectChanges), efeitos colaterais aplicados (CommandEffects),
/// contadores de profundidade de Undo/Redo e a revisão atual do documento.
#[derive(Debug, Clone, PartialEq)]
pub struct CommandExecutionReport {
    pub label: &'static str,
    pub changes: ProjectChanges,
    pub effects: CommandEffects,
    pub undo_depth: usize,
    pub redo_depth: usize,
    pub revision: u64,
}

/// Trait central de comando. Executa contra `CommandContext` (documento, editor e
/// eventos): comandos não enxergam UI nem recursos de render.
pub trait Command: Send + Sync {
    /// Rótulo legível para telemetria e pilha de histórico (Undo/Redo).
    fn label(&self) -> &'static str;

    /// Executa o comando contra o estado do editor.
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError>;

    /// Indica se a operação altera dados do projeto e exige gravação prévia de checkpoint no UndoStack.
    fn is_destructive(&self) -> bool {
        true
    }

    /// Geometry edits freeze live primitives by default. Commands that change
    /// another responsibility must explicitly retain their authored sources.
    fn freezes_active_primitive(&self) -> bool {
        true
    }

    /// Domínios persistentes alterados quando a execução termina com sucesso.
    fn changes(&self) -> ProjectChanges {
        if self.is_destructive() {
            ProjectChanges::GEOMETRY
        } else {
            ProjectChanges::NONE
        }
    }

    /// Valida disponibilidade contextual do comando contra o estado atual (P3D-100).
    /// Retorna Ok(()) se puder ser executado, ou Err("razão legível") quando desabilitado.
    fn can_execute(&self, _state: &EditorView<'_>) -> Result<(), &'static str> {
        Ok(())
    }
}

impl<T: ?Sized + Command> Command for Box<T> {
    fn label(&self) -> &'static str {
        (**self).label()
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        (**self).execute(state)
    }

    fn is_destructive(&self) -> bool {
        (**self).is_destructive()
    }

    fn freezes_active_primitive(&self) -> bool {
        (**self).freezes_active_primitive()
    }

    fn changes(&self) -> ProjectChanges {
        (**self).changes()
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        (**self).can_execute(state)
    }
}

/// Categorias funcionais de comandos da aplicação (P3D-081, P3D-100).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum CommandCategory {
    File,
    Edit,
    Model,
    Select,
    View,
    Tools,
    Window,
    Help,
}

impl CommandCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Edit => "Edit",
            Self::Model => "Model",
            Self::Select => "Select",
            Self::View => "View",
            Self::Tools => "Tools",
            Self::Window => "Window",
            Self::Help => "Help",
        }
    }
}

/// Metadados semânticos de um comando cadastrado na aplicação.
#[derive(Clone)]
pub struct CommandMetadata {
    pub id: String,
    pub label: String,
    pub description: String,
    pub category: CommandCategory,
    pub docs_topic: Option<DocsTopic>,
}

impl CommandMetadata {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
        category: CommandCategory,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            description: description.into(),
            category,
            docs_topic: None,
        }
    }

    pub fn with_docs(mut self, topic: DocsTopic) -> Self {
        self.docs_topic = Some(topic);
        self
    }
}

/// Item de apresentação para a Command Palette (P3D-081) e ferramentas de busca.
#[derive(Debug, Clone)]
pub struct CommandPaletteItem {
    pub id: String,
    pub label: String,
    pub description: String,
    pub category: CommandCategory,
    pub shortcut: Option<String>,
    pub disabled_reason: Option<&'static str>,
    pub is_available: bool,
    pub docs_topic: Option<DocsTopic>,
}

/// Despachante central de comandos com auto-checkpointing de Undo/Redo,
/// sincronização de eventos, catálogo de metadados e suporte a Command Palette.
#[derive(Default, Clone)]
pub struct CommandDispatcher {
    registry: HashMap<String, Arc<dyn Command>>,
    metadata: HashMap<String, CommandMetadata>,
}

impl CommandDispatcher {
    pub fn new() -> Self {
        Self {
            registry: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn register<C: Command + 'static>(&mut self, id: impl Into<String>, cmd: C) {
        let id_str = id.into();
        let cmd_arc: Arc<dyn Command> = Arc::new(cmd);
        let meta = CommandMetadata::new(
            &id_str,
            cmd_arc.label(),
            format!("Action {}", cmd_arc.label()),
            CommandCategory::Model,
        );
        self.metadata.insert(id_str.clone(), meta);
        self.registry.insert(id_str, cmd_arc);
    }

    pub fn register_with_meta<C: Command + 'static>(&mut self, meta: CommandMetadata, cmd: C) {
        let id = meta.id.clone();
        self.metadata.insert(id.clone(), meta);
        self.registry.insert(id, Arc::new(cmd));
    }

    /// Alias for a registered command id (keymap/MCP/Lua dialects).
    ///
    /// The metadata clone gets the alias id: sem isso o catálogo publicado lista
    /// duas linhas com o mesmo `CommandId` (uma para o dono, outra para o slot
    /// do alias) e a contagem de comandos registrados mente.
    pub fn alias(&mut self, from: impl Into<String>, to: &str) {
        let from = from.into();
        if let Some(cmd) = self.registry.get(to).cloned() {
            self.registry.insert(from.clone(), cmd);
        }
        if let Some(meta) = self.metadata.get(to).cloned() {
            let mut meta = meta;
            meta.id = from.clone();
            self.metadata.insert(from, meta);
        }
    }

    /// True when `id` is a registered command (including aliases).
    pub fn contains(&self, id: &str) -> bool {
        self.registry.contains_key(id)
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn Command>> {
        self.registry.get(id).cloned()
    }

    pub fn get_metadata(&self, id: &str) -> Option<&CommandMetadata> {
        self.metadata.get(id)
    }

    pub fn all_metadata(&self) -> Vec<&CommandMetadata> {
        let mut list: Vec<&CommandMetadata> = self.metadata.values().collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    pub fn can_execute(&self, id: &str, state: &AppState) -> Result<(), &'static str> {
        let cmd = self.registry.get(id).ok_or("Comando não registrado")?;
        cmd.can_execute(&state.view())
    }

    pub fn execute(&self, id: &str, state: &mut AppState) -> Result<(), CommandError> {
        self.execute_detailed(id, state).map(|_| ())
    }

    /// Executa um comando registrado por id devolvendo o relatório DTO pós-commit (A04).
    pub fn execute_detailed(
        &self,
        id: &str,
        state: &mut AppState,
    ) -> Result<CommandExecutionReport, CommandError> {
        let cmd = self
            .registry
            .get(id)
            .ok_or_else(|| CommandError::Execution(format!("Comando '{id}' não registrado")))?
            .clone();
        Self::dispatch_detailed(state, cmd.as_ref())
    }

    pub fn dispatch(state: &mut AppState, cmd: &dyn Command) -> Result<(), CommandError> {
        Self::dispatch_detailed(state, cmd).map(|_| ())
    }

    /// Despacha o comando contra o AppState devolvendo o relatório DTO pós-commit (A04).
    pub fn dispatch_detailed(
        state: &mut AppState,
        cmd: &dyn Command,
    ) -> Result<CommandExecutionReport, CommandError> {
        let mut ctx = state.command_context();
        let report = Self::run_detailed(&mut ctx, cmd)?;
        let effects = ctx.into_effects();
        let host = state.apply_effects(effects);
        host.map(|()| report)
    }

    /// Variante para quem já tem um contexto restrito, devolvendo o relatório DTO pós-commit (A04).
    pub fn run_detailed(
        ctx: &mut CommandContext<'_>,
        cmd: &dyn Command,
    ) -> Result<CommandExecutionReport, CommandError> {
        let label = cmd.label();
        let changes = cmd.changes();
        Self::run(ctx, cmd)?;
        let (undo_depth, redo_depth) = ctx.document.undo.depth();
        let revision = ctx.document.project.revision_id();
        Ok(CommandExecutionReport {
            label,
            changes,
            effects: ctx.effects.clone(),
            undo_depth,
            redo_depth,
            revision,
        })
    }

    /// Fronteira de mutação única: valida (`can_execute`), executa o comando no contexto restrito,
    /// restaura o documento se falhar e registra uma entrada de undo se tiver
    /// sucesso (website/docs/02-application/commands-events-services.md).
    pub fn run(ctx: &mut CommandContext<'_>, cmd: &dyn Command) -> Result<(), CommandError> {
        cmd.can_execute(&ctx.view())
            .map_err(|reason| CommandError::Execution(reason.into()))?;
        if cmd.is_destructive()
            && (ctx.modal.is_some()
                || ctx.mesh_preview.is_some()
                || ctx.paint_stroke.is_some()
                || ctx.uv_transform.is_some())
        {
            return Err(CommandError::Execution(
                "Confirm or cancel the active operation first".into(),
            ));
        }
        ctx.document.normalize_references();
        let changes = cmd.changes();
        ctx.document.project.history_selection = ctx.editor.selection.assets.clone();
        let original_selection = ctx.editor.selection.clone();
        let original = cmd.is_destructive().then(|| ctx.document.project.clone());
        // A sessão de criação de primitiva é consumida ao congelar; guardá-la
        // permite restaurar o estado de sessão junto com o projeto se falhar.
        let original_primitive_session = (
            ctx.editor.primitive_session.clone(),
            ctx.editor.last_primitive,
        );
        if cmd.is_destructive() && cmd.freezes_active_primitive() {
            ctx.freeze_active_primitive_for_command();
        }
        let exec_result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cmd.execute(ctx)));
        let execution_outcome = match exec_result {
            Ok(res) => res,
            Err(panic_payload) => {
                let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown internal panic".to_string()
                };
                tracing::error!(
                    command = cmd.label(),
                    panic = %msg,
                    "Command panicked during execution; document restored to pre-command state"
                );
                Err(CommandError::Execution(format!(
                    "Internal error caught: {msg}"
                )))
            }
        };

        if let Err(error) = execution_outcome {
            if let Some(original) = original {
                ctx.document.project = original;
                ctx.document.sync_reference_mirror();
                ctx.editor.selection = original_selection;
                (ctx.editor.primitive_session, ctx.editor.last_primitive) =
                    original_primitive_session;
                // The project and selection are the same snapshot. Recomputing
                // selection here would mutate the restored document on failure.
                ctx.mark_dirty();
            }
            return Err(error);
        }

        if let Some(original) = original {
            let bytes = original.estimated_bytes();
            ctx.document.undo.push_sized(cmd.label(), original, bytes);
        }
        ctx.document.sync_reference_mirror();
        let sel_changed = ctx.sync_selection();
        let mut final_changes = changes;
        if sel_changed {
            final_changes = final_changes.difference(ProjectChanges::SELECTION);
        }
        ctx.emit_project_changed(final_changes);
        ctx.mark_dirty();
        Ok(())
    }

    /// Variante para quem já tem um contexto restrito (testes e serviços).
    pub fn dispatch_context(
        ctx: &mut CommandContext<'_>,
        cmd: &dyn Command,
    ) -> Result<(), CommandError> {
        Self::run(ctx, cmd)
    }

    /// Consulta a lista de comandos filtrados por busca fuzzy/substring para a Command Palette (P3D-081).
    pub fn query(&self, search_term: &str, state: &AppState) -> Vec<CommandPaletteItem> {
        let term = search_term.trim().to_lowercase();
        let mut results = Vec::new();

        for (id, meta) in &self.metadata {
            let shortcut = state.ui.keybinds.shortcut_for(id);
            let shortcut_lower = shortcut.as_deref().unwrap_or("").to_lowercase();
            let label_lower = meta.label.to_lowercase();
            let desc_lower = meta.description.to_lowercase();
            let cat_lower = meta.category.as_str().to_lowercase();
            let id_lower = id.to_lowercase();

            let matches = term.is_empty()
                || label_lower.contains(&term)
                || desc_lower.contains(&term)
                || cat_lower.contains(&term)
                || id_lower.contains(&term)
                || shortcut_lower.contains(&term);

            if matches {
                let disabled_reason = self.can_execute(id, state).err();
                results.push(CommandPaletteItem {
                    id: id.clone(),
                    label: meta.label.clone(),
                    description: meta.description.clone(),
                    category: meta.category,
                    shortcut,
                    disabled_reason,
                    is_available: disabled_reason.is_none(),
                    docs_topic: meta.docs_topic,
                });
            }
        }

        // Ordena comandos disponíveis primeiro, depois por ordem alfabética de rótulo
        results.sort_by(|a, b| {
            b.is_available
                .cmp(&a.is_available)
                .then_with(|| a.label.cmp(&b.label))
        });

        results
    }

    /// Cria e preenche o dispatcher com todos os comandos canônicos do Petunia3D.
    pub fn canonical() -> Self {
        let mut d = Self::new();

        use crate::origin_commands::{OriginPreset, OriginPresetCmd};
        for (id, preset) in [
            ("model.origin_geometry", OriginPreset::Geometry),
            ("model.origin_bottom", OriginPreset::Bottom),
            ("model.origin_cursor", OriginPreset::Cursor),
            ("model.origin_selection", OriginPreset::Selection),
            ("model.geometry_to_origin", OriginPreset::GeometryToOrigin),
        ] {
            d.register(id, OriginPresetCmd(preset));
        }

        // 1. Arquivo (File)
        d.register_with_meta(
            CommandMetadata::new(
                "file.new",
                "New Project",
                "Create a blank 3D project",
                CommandCategory::File,
            )
            .with_docs(DocsTopic::GettingStarted),
            NewProjectCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "file.save",
                "Save Project",
                "Save active project to disk",
                CommandCategory::File,
            )
            .with_docs(DocsTopic::GettingStarted),
            SaveProjectCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "file.open",
                "Open Project",
                "Open a Petunia3D project from disk",
                CommandCategory::File,
            )
            .with_docs(DocsTopic::GettingStarted),
            OpenProjectCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "file.save_as",
                "Save Project As",
                "Save active project to a new file",
                CommandCategory::File,
            ),
            SaveProjectAsCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "file.import_obj",
                "Import OBJ",
                "Import 3D mesh from Wavefront OBJ file",
                CommandCategory::File,
            )
            .with_docs(DocsTopic::ImportExport),
            ImportObjCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "file.import_gltf",
                "Import glTF / GLB",
                "Import 3D mesh with materials and textures from glTF or GLB file",
                CommandCategory::File,
            )
            .with_docs(DocsTopic::ImportExport),
            ImportGltfCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "file.export_obj",
                "Export OBJ",
                "Export active mesh to Wavefront OBJ format",
                CommandCategory::File,
            )
            .with_docs(DocsTopic::ImportExport),
            ExportObjCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "file.export_glb",
                "Export GLB",
                "Export scene to binary glTF format",
                CommandCategory::File,
            )
            .with_docs(DocsTopic::ImportExport),
            ExportGlbCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "file.save_asset",
                "Save Active Model as Asset",
                "Save active mesh to project asset library",
                CommandCategory::File,
            )
            .with_docs(DocsTopic::Assets),
            SaveActiveAsAssetCmd,
        );

        // 2. Edição (Edit)
        d.register_with_meta(
            CommandMetadata::new(
                "edit.undo",
                "Undo",
                "Undo previous modification",
                CommandCategory::Edit,
            ),
            UndoCmd,
        );
        d.alias("global.undo", "edit.undo");
        d.alias("app.undo", "edit.undo");
        d.register_with_meta(
            CommandMetadata::new(
                "edit.redo",
                "Redo",
                "Redo last undone modification",
                CommandCategory::Edit,
            ),
            RedoCmd,
        );
        d.alias("global.redo", "edit.redo");
        d.alias("app.redo", "edit.redo");
        d.register_with_meta(
            CommandMetadata::new(
                "edit.delete",
                "Delete",
                "Delete selected elements or active object",
                CommandCategory::Edit,
            ),
            DeleteSelectionCmd,
        );
        d.alias("model.delete", "edit.delete");
        d.alias("edit.delete_selected", "edit.delete");
        d.alias("model.delete_selected", "edit.delete");
        d.register_with_meta(
            CommandMetadata::new(
                "edit.duplicate",
                "Duplicate",
                "Duplicate selected elements or active object",
                CommandCategory::Edit,
            ),
            DuplicateSelectionCmd,
        );
        d.alias("model.duplicate", "edit.duplicate");
        d.alias("edit.duplicate_selected", "edit.duplicate");
        d.alias("model.duplicate_selected", "edit.duplicate");

        // 3. Seleção (Select)
        d.register_with_meta(
            CommandMetadata::new(
                "select.all",
                "Select All",
                "Select all geometry elements in active mesh",
                CommandCategory::Select,
            ),
            SelectAllCmd,
        );
        d.alias("model.select_all", "select.all");
        d.register_with_meta(
            CommandMetadata::new(
                "select.none",
                "Deselect All",
                "Clear current geometry selection",
                CommandCategory::Select,
            ),
            ClearSelectionCmd,
        );
        d.alias("model.deselect_all", "select.none");
        d.register_with_meta(
            CommandMetadata::new(
                "select.invert",
                "Invert Selection",
                "Invert geometry selection in active mesh",
                CommandCategory::Select,
            ),
            InvertSelectionCmd,
        );
        d.alias("model.invert_selection", "select.invert");
        d.register_with_meta(
            CommandMetadata::new(
                "select.linked",
                "Select Linked",
                "Select connected geometry elements",
                CommandCategory::Select,
            ),
            SelectLinkedCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "select.domain_object",
                "Select Domain: Object",
                "Switch interaction to Object domain",
                CommandCategory::Select,
            ),
            SetSelectionDomainCmd(SelectionDomain::Object),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "select.domain_vertex",
                "Select Domain: Point",
                "Switch interaction to Point domain",
                CommandCategory::Select,
            ),
            SetSelectionDomainCmd(SelectionDomain::Vertex),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "select.domain_edge",
                "Select Domain: Edge",
                "Switch interaction to Edge domain",
                CommandCategory::Select,
            ),
            SetSelectionDomainCmd(SelectionDomain::Edge),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "select.domain_face",
                "Select Domain: Face",
                "Switch interaction to Face domain",
                CommandCategory::Select,
            ),
            SetSelectionDomainCmd(SelectionDomain::Face),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "select.cycle_domain",
                "Cycle Selection Domain",
                "Toggle between Object and last component domain",
                CommandCategory::Select,
            ),
            CycleSelectionDomainCmd,
        );

        // 4. Modelagem (Model)
        d.register_with_meta(
            CommandMetadata::new(
                "model.add_cube",
                "Add Cube",
                "Add a 3D box primitive",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            AddPrimitiveCmd::cube(true),
        );
        d.alias("add.cube", "model.add_cube");
        d.register_with_meta(
            CommandMetadata::new(
                "model.add_sphere",
                "Add Sphere",
                "Add a low-poly sphere primitive",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            AddPrimitiveCmd::new(PrimitiveKind::Sphere),
        );
        d.alias("add.sphere", "model.add_sphere");
        d.register_with_meta(
            CommandMetadata::new(
                "model.add_cylinder",
                "Add Cylinder",
                "Add a cylinder primitive",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            AddPrimitiveCmd::new(PrimitiveKind::Cylinder),
        );
        d.alias("add.cylinder", "model.add_cylinder");
        d.register_with_meta(
            CommandMetadata::new(
                "model.add_plane",
                "Add Plane",
                "Add a flat plane primitive",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            AddPrimitiveCmd::new(PrimitiveKind::Plane),
        );
        d.alias("add.plane", "model.add_plane");
        d.register_with_meta(
            CommandMetadata::new(
                "model.add_cone",
                "Add Cone",
                "Add a cone primitive",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            AddPrimitiveCmd::new(PrimitiveKind::Cone),
        );
        d.alias("add.cone", "model.add_cone");
        d.register_with_meta(
            CommandMetadata::new(
                "model.add_capsule",
                "Add Capsule",
                "Add a capsule primitive",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            AddPrimitiveCmd::new(PrimitiveKind::Capsule),
        );
        d.alias("add.capsule", "model.add_capsule");
        d.register_with_meta(
            CommandMetadata::new(
                "model.add_wedge",
                "Add Wedge",
                "Add a wedge/ramp primitive",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            AddPrimitiveCmd::new(PrimitiveKind::Wedge),
        );
        d.alias("add.wedge", "model.add_wedge");
        d.register_with_meta(
            CommandMetadata::new(
                "model.add_circle",
                "Add Circle",
                "Add a circle/disc primitive",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            AddPrimitiveCmd::new(PrimitiveKind::Circle),
        );
        d.alias("add.circle", "model.add_circle");
        d.register_with_meta(
            CommandMetadata::new(
                "model.add_torus",
                "Add Torus",
                "Add a torus primitive",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            AddPrimitiveCmd::new(PrimitiveKind::Torus),
        );
        d.alias("add.torus", "model.add_torus");
        d.register_with_meta(
            CommandMetadata::new(
                "model.add_icosphere",
                "Add Icosphere",
                "Add an icosphere primitive",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            AddPrimitiveCmd::new(PrimitiveKind::Icosphere),
        );
        d.alias("add.icosphere", "model.add_icosphere");
        d.register_with_meta(
            CommandMetadata::new(
                "model.revolve",
                "Revolve 360",
                "Revolve selected profile 360 degrees around an axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            RevolveCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.extrude",
                "Extrude",
                "Extrude selected faces along surface normal",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Extrude),
            ExtrudeSelectedCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.extrude_individual",
                "Extrude Individual",
                "Extrude selected faces individually",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Extrude),
            ExtrudeIndividualCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.inset",
                "Inset Faces",
                "Inset selected faces towards interior",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            InsetFacesCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.push_pull",
                "Push/Pull",
                "Push or pull selected faces along the surface normal",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            PushPullToolCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.knife",
                "Knife",
                "Cut the active mesh along edge points picked in the viewport",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Knife),
            KnifeToolCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.join",
                "Join",
                "Merge the other selected objects into the active object keeping every topology",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            JoinObjectsCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "measure.clear",
                "Clear Measurements",
                "Remove every measurement made with the Measure tool",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::measure_commands::ClearMeasurementsCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "scene.group",
                "Group",
                "Gather the selected objects into a new group of the scene structure",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::scene_commands::GroupSelectionCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "scene.ungroup",
                "Ungroup",
                "Dissolve the group of the active object, keeping its objects",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::scene_commands::UngroupCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.shape_builder_solid",
                "Solid Shape Builder",
                "Execute 3D solid partition operations with transient RegionPlan",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::solid_commands::SolidShapeEditCmd::unite(),
        );
        d.alias("solid.edit", "model.shape_builder_solid");
        d.alias("model.solid_edit", "model.shape_builder_solid");

        d.register_with_meta(
            CommandMetadata::new(
                "model.solid_unite",
                "Solid Unite",
                "Unite all partition regions into a single solid mesh",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::solid_commands::SolidShapeEditCmd::unite(),
        );
        d.alias("solid.unite", "model.solid_unite");

        d.register_with_meta(
            CommandMetadata::new(
                "model.solid_remove_overlap",
                "Solid Remove Overlap",
                "Remove overlapping regions from all solid operands",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::solid_commands::SolidShapeEditCmd::remove_overlap(),
        );
        d.alias("solid.remove_overlap", "model.solid_remove_overlap");

        d.register_with_meta(
            CommandMetadata::new(
                "model.solid_extract_overlap",
                "Solid Extract Overlap",
                "Extract overlapping region as a new scene object",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::solid_commands::SolidShapeEditCmd::extract_overlap(),
        );
        d.alias("solid.extract_overlap", "model.solid_extract_overlap");

        d.register_with_meta(
            CommandMetadata::new(
                "model.solid_composition_create",
                "Create Solid Composition",
                "Create a re-editable solid composition from stable source operands",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::solid_commands::CreateSolidCompositionCmd::new(
                Vec::new(),
                crate::solid_commands::SolidOperation::UniteAll,
            ),
        );
        d.alias("solid.composition_create", "model.solid_composition_create");

        d.register_with_meta(
            CommandMetadata::new(
                "model.solid_composition_bake",
                "Bake Solid Composition",
                "Explicitly bake solid composition recipe to editable static mesh",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::solid_commands::BakeSolidCompositionCmd::new(),
        );
        d.alias("solid.composition_bake", "model.solid_composition_bake");
        d.alias("solid.bake", "model.solid_composition_bake");

        d.register_with_meta(
            CommandMetadata::new(
                "model.solid_composition_toggle_step",
                "Toggle Solid Composition Step",
                "Enable or disable a specific step in the solid composition recipe",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::solid_commands::ToggleSolidCompositionStepCmd::new(uuid::Uuid::nil(), true),
        );
        d.alias(
            "solid.composition_toggle",
            "model.solid_composition_toggle_step",
        );

        d.register_with_meta(
            CommandMetadata::new(
                "model.solid_composition_reorder_step",
                "Reorder Solid Composition Steps",
                "Reorder evaluation steps in the solid composition recipe",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::solid_commands::ReorderSolidCompositionStepCmd::new(0, 0),
        );
        d.alias(
            "solid.composition_reorder",
            "model.solid_composition_reorder_step",
        );

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_workplane_to_face",
                "Align Workplane to Face",
                "Align drawing workplane to the specified planar face",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::surface_commands::AlignWorkplaneToFaceCmd::new(0),
        );
        d.alias("workplane.align_face", "model.align_workplane_to_face");

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_workplane_to_selection",
                "Align Workplane to Selection",
                "Align drawing workplane to selected faces or opposing face gap",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::surface_commands::AlignWorkplaneToSelectionCmd,
        );
        d.alias(
            "workplane.align_selection",
            "model.align_workplane_to_selection",
        );

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_objects",
                "Align Objects",
                "Align selected objects in 3D space",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::AlignObjectsCmd::center_x(),
        );
        d.alias("object.align", "model.align_objects");

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_center_x",
                "Align Centers X",
                "Align centers of selected objects along X axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::AlignObjectsCmd::center_x(),
        );
        d.alias("align.center_x", "model.align_center_x");

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_center_y",
                "Align Centers Y",
                "Align centers of selected objects along Y axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::AlignObjectsCmd::center_y(),
        );
        d.alias("align.center_y", "model.align_center_y");

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_center_z",
                "Align Centers Z",
                "Align centers of selected objects along Z axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::AlignObjectsCmd::center_z(),
        );
        d.alias("align.center_z", "model.align_center_z");

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_min_x",
                "Align Left (Min X)",
                "Align minimum edges of selected objects along X axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::AlignObjectsCmd::min_x(),
        );
        d.alias("align.left", "model.align_min_x");

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_max_x",
                "Align Right (Max X)",
                "Align maximum edges of selected objects along X axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::AlignObjectsCmd::max_x(),
        );
        d.alias("align.right", "model.align_max_x");

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_min_y",
                "Align Bottom (Min Y)",
                "Align bottom base of selected objects along Y axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::AlignObjectsCmd::min_y(),
        );
        d.alias("align.bottom", "model.align_min_y");

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_max_y",
                "Align Top (Max Y)",
                "Align top edges of selected objects along Y axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::AlignObjectsCmd::max_y(),
        );
        d.alias("align.top", "model.align_max_y");

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_min_z",
                "Align Back (Min Z)",
                "Align back edges of selected objects along Z axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::AlignObjectsCmd::min_z(),
        );
        d.alias("align.back", "model.align_min_z");

        d.register_with_meta(
            CommandMetadata::new(
                "model.align_max_z",
                "Align Front (Max Z)",
                "Align front edges of selected objects along Z axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::AlignObjectsCmd::max_z(),
        );
        d.alias("align.front", "model.align_max_z");

        d.register_with_meta(
            CommandMetadata::new(
                "model.distribute_objects_x",
                "Distribute Objects X",
                "Distribute spacing between selected objects along X axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::DistributeObjectsCmd::on_x(),
        );
        d.alias("distribute.x", "model.distribute_objects_x");

        d.register_with_meta(
            CommandMetadata::new(
                "model.distribute_objects_y",
                "Distribute Objects Y",
                "Distribute spacing between selected objects along Y axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::DistributeObjectsCmd::on_y(),
        );
        d.alias("distribute.y", "model.distribute_objects_y");

        d.register_with_meta(
            CommandMetadata::new(
                "model.distribute_objects_z",
                "Distribute Objects Z",
                "Distribute spacing between selected objects along Z axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::align_commands::DistributeObjectsCmd::on_z(),
        );
        d.alias("distribute.z", "model.distribute_objects_z");

        d.register_with_meta(
            CommandMetadata::new(
                "model.imprint_profile",
                "Imprint Profile on Surface",
                "Imprint profile contour and holes onto coplanar surface faces",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::surface_commands::ImprintProfileCmd::new(uuid::Uuid::nil()),
        );
        d.alias("surface.imprint", "model.imprint_profile");
        d.alias("model.surface_imprint", "model.imprint_profile");

        d.register_with_meta(
            CommandMetadata::new(
                "model.contextual_push_pull",
                "Contextual Push/Pull",
                "Push or pull surface region: boundary cap stretches or embedded region extrudes",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::surface_commands::ContextualPushPullCmd::new(1.0),
        );
        d.alias("push_pull.contextual", "model.contextual_push_pull");

        d.register_with_meta(
            CommandMetadata::new(
                "model.scale_to_dimension",
                "Scale To Dimension",
                "Scale object along an axis to target dimension in the specified coordinate frame",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::spatial_query::ScaleToDimensionCmd::new(
                uuid::Uuid::nil(),
                0,
                1.0,
                false,
                crate::spatial_query::SpatialFrameKind::World,
            ),
        );
        d.alias("transform.scale_to_dimension", "model.scale_to_dimension");

        d.register_with_meta(
            CommandMetadata::new(
                "model.set_section_plane",
                "Set Section Plane",
                "Set or update active non-destructive section inspection plane",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Modeling),
            crate::section_analysis::SetSectionPlaneCmd::new(
                crate::section_analysis::SectionPlane::default(),
            ),
        );
        d.alias("view.set_section_plane", "model.set_section_plane");

        d.register_with_meta(
            CommandMetadata::new(
                "model.clear_section_plane",
                "Clear Section Plane",
                "Clear active section inspection plane",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Modeling),
            crate::section_analysis::ClearSectionPlaneCmd,
        );
        d.alias("view.clear_section_plane", "model.clear_section_plane");

        d.register_with_meta(
            CommandMetadata::new(
                "model.flip_section_plane",
                "Flip Section Plane",
                "Invert active section inspection plane normal",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Modeling),
            crate::section_analysis::FlipSectionPlaneCmd,
        );
        d.alias("view.flip_section_plane", "model.flip_section_plane");

        d.register_with_meta(
            CommandMetadata::new(
                "model.offset_section_plane",
                "Offset Section Plane",
                "Adjust active section plane offset numerically",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Modeling),
            crate::section_analysis::OffsetSectionPlaneCmd::new(0.0),
        );
        d.alias("view.offset_section_plane", "model.offset_section_plane");

        d.register_with_meta(
            CommandMetadata::new(
                "draw.unite",
                "Unite",
                "Merge the closed shapes of the sketch plane into one",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::shape_builder::ShapeEditCmd::pathfinder(
                crate::shape_builder::PathfinderOp::Unite,
            ),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "draw.subtract",
                "Subtract",
                "Cut the shapes in front out of the shape at the back",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::shape_builder::ShapeEditCmd::pathfinder(
                crate::shape_builder::PathfinderOp::Subtract,
            ),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "draw.intersect",
                "Intersect Shapes",
                "Keep only the area all closed shapes share",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::shape_builder::ShapeEditCmd::pathfinder(
                crate::shape_builder::PathfinderOp::Intersect,
            ),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "draw.exclude",
                "Exclude",
                "Remove the areas where shapes overlap",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            crate::shape_builder::ShapeEditCmd::pathfinder(
                crate::shape_builder::PathfinderOp::Exclude,
            ),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.bevel",
                "Bevel Edges",
                "Bevel selected mesh edges",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Bevel),
            BevelCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.scale_selection",
                "Scale Selection",
                "Scale selected geometry uniformly around its center",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            ScaleSelectionCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.subdivide",
                "Subdivide",
                "Subdivide selected geometry",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::LoopCut),
            SubdivideSelectionCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.loop_cut",
                "Loop Cut",
                "Insert evenly spaced cuts along a quad ring",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::LoopCut),
            LoopCutCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.unwrap_auto",
                "Auto UV",
                "Unwrap the active mesh with the generic UV provider",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UnwrapAutoCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.pack_islands",
                "Pack UV Islands",
                "Pack UV islands into 0..1 without overlaps",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvPackIslandsCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.project_view",
                "Project From View",
                "Project UVs from the current camera view",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvProjectFromViewCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.project_reference",
                "Project From Reference",
                "Project UVs from the active reference image or fallback to camera view",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvProjectFromReferenceCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.stitch",
                "Stitch UV Seams",
                "Weld and align separated UV seams on shared 3D edges",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvStitchCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.relax",
                "Relax UVs",
                "Relax UV islands toward the 3D shape (as-rigid-as-possible), keeping pins and seams",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvRelaxCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.split",
                "Split UVs",
                "Mark seams around the selected UV faces without duplicating vertices",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvSplitCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "paint.bake_reference",
                "Bake Reference to Texture",
                "Project UVs and bake visible reference image into the active texture",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::TexturePainting),
            PaintBakeReferenceCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "paint.set_decal_transform",
                "Set Decal Transform",
                "Update position, scale and rotation of a decal layer",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::TexturePainting),
            SetDecalTransformCmd {
                layer_id: uuid::Uuid::nil(),
                center_uv: [0.5, 0.5],
                scale_uv: [0.25, 0.25],
                rotation_rad: 0.0,
            },
        );
        d.register_with_meta(
            CommandMetadata::new(
                "paint.bake_decal",
                "Bake Decal to Raster",
                "Bake a live decal layer into a static raster layer",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::TexturePainting),
            BakeDecalCmd {
                layer_id: uuid::Uuid::nil(),
            },
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.merge",
                "Merge Center",
                "Merge selected vertices into center point",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            MergeCenterCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.connect",
                "Connect Loops",
                "Connect two faces or boundary loops with quads and triangles",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            ConnectLoopsCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.dissolve",
                "Dissolve",
                "Dissolve selected edges, faces, or vertices cleanly",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            DissolveCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.make_face",
                "Make Face / Edge",
                "Create a new face or edge connecting selected vertices or edges (F)",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            MakeFaceCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.spin",
                "Spin",
                "Extrude and spin selected geometry in an arc around axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            RevolveCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.weld",
                "Merge by Distance",
                "Weld duplicate vertices within distance",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            WeldCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.symmetrize",
                "Symmetrize",
                "Copy one side to the other across an axis",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            SymmetrizeCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.flip_normals",
                "Flip Normals",
                "Reverse orientation of face normals",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            FlipNormalsCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.mark_sharp",
                "Mark Sharp",
                "Mark the selected edges as sharp (limits smooth shading)",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            MarkSharpCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.clear_sharp",
                "Clear Sharp",
                "Remove the sharp mark from the selected edges",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            ClearSharpCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.smooth_by_angle",
                "Smooth by Angle",
                "Shade smooth and mark edges above the angle as sharp",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            SmoothByAngleCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.orient_consistently",
                "Orient Consistently",
                "Make face winding consistent across connected faces",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            OrientConsistentlyCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.orient_outward",
                "Orient Outward",
                "Orient a closed mesh so faces point outside",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            OrientOutwardCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.flip_faces",
                "Flip Faces",
                "Reverse the winding of the selected faces",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            FlipNormalsCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.shade_smooth",
                "Shade Smooth",
                "Interpolate normals across faces of the active object",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            SetShadeSmoothCmd { smooth: true },
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.shade_flat",
                "Shade Flat",
                "Use faceted normals on the active object",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            SetShadeSmoothCmd { smooth: false },
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.flip_diagonal",
                "Flip Diagonal",
                "Flip quad internal diagonal or triangle edge",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            FlipDiagonalCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.separate_selection",
                "Separate Selection",
                "Separate selected geometry into a new object",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            SeparateSelectionCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.mirror",
                "Mirror Mesh",
                "Mirror the active mesh across an axis and optionally weld the plane",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            MirrorMeshCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.push_pull_apply",
                "Apply Push/Pull",
                "Displace selected faces along their normal by a given distance",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            PushPullApplyCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.slice_plane",
                "Slice Plane",
                "Slice the active mesh along a plane through the selection center",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            SlicePlaneCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.triangulate",
                "Triangulate",
                "Triangulate faces of the active mesh",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            TriangulateMeshCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.translate_selection",
                "Translate Selection",
                "Translate selected vertices by a delta vector",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            TranslateSelectionCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.project_planar",
                "Planar UV Projection",
                "Project UVs planarly on the active mesh",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvProjectPlanarCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.project_cube",
                "Cube UV Projection",
                "Project UVs using box mapping on the active mesh",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvProjectCubeCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.project_cylindrical",
                "Cylindrical UV Projection",
                "Project UVs cylindrically around the Y axis on selected faces (or all)",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvProjectCylindricalCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.project_spherical",
                "Spherical UV Projection",
                "Project UVs spherically from the bounding center on selected faces (or all)",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvProjectSphericalCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.equalize_texel_density",
                "Equalize Texel Density",
                "Equalize texel density across UV islands",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvEqualizeTexelDensityCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "uv.normalize_texel_density",
                "Normalize Texel Density",
                "Normalize texel density to a target value for a given texture width",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvNormalizeTexelDensityCmd::default(),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "asset.join_into_active",
                "Join Asset Into Active",
                "Merge another asset's mesh into the active asset and remove the source asset",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            JoinAssetIntoActiveCmd { source_index: 0 },
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.clear_location",
                "Clear Location",
                "Reset object location to [0, 0, 0]",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            ClearLocationCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.clear_rotation",
                "Clear Rotation",
                "Reset object rotation to [0, 0, 0]",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            ClearRotationCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.clear_scale",
                "Clear Scale",
                "Reset object scale to [1, 1, 1]",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            ClearScaleCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.clear_all_transforms",
                "Clear All Transforms",
                "Reset object position, rotation and scale to default",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            ClearAllTransformsCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.set_transform",
                "Set Object Transform",
                "Set absolute position, rotation (Euler XYZ) and scale of an object",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            SetAssetTransformCmd::default(),
        );
        d.alias("object.set_transform", "model.set_transform");
        d.alias("asset.set_transform", "model.set_transform");
        d.register_with_meta(
            CommandMetadata::new(
                "model.instantiate_asset",
                "Instantiate Asset",
                "Instantiate a library asset into the active 3D scene",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Assets),
            InstantiateAssetCmd {
                asset_id: uuid::Uuid::nil(),
                position: None,
            },
        );

        // Materiais (Material & Shading)
        use crate::edit_api::AssignMaterialSlotCmd;
        use crate::material_commands::{
            CreateMaterialCmd, DuplicateMaterialCmd, EditMaterialCmd, RemoveMaterialCmd,
            SetMaterialPropertiesCmd,
        };

        d.register_with_meta(
            CommandMetadata::new(
                "material.create",
                "Create Material",
                "Create a new standard PBR material in the project",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Materials),
            CreateMaterialCmd::default(),
        );
        d.alias("model.create_material", "material.create");

        d.register_with_meta(
            CommandMetadata::new(
                "material.duplicate",
                "Duplicate Material",
                "Duplicate the active or selected material",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Materials),
            DuplicateMaterialCmd::default(),
        );
        d.alias("model.duplicate_material", "material.duplicate");

        d.register_with_meta(
            CommandMetadata::new(
                "material.remove",
                "Remove Material",
                "Remove the specified material from the project",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Materials),
            RemoveMaterialCmd::default(),
        );
        d.alias("model.remove_material", "material.remove");

        d.register_with_meta(
            CommandMetadata::new(
                "material.edit",
                "Edit Material",
                "Edit a scalar or color property of a material",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Materials),
            EditMaterialCmd::default(),
        );

        d.register_with_meta(
            CommandMetadata::new(
                "material.set_properties",
                "Set Material Properties",
                "Set multiple PBR properties of a material in a single atomic transaction",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Materials),
            SetMaterialPropertiesCmd::default(),
        );
        d.alias("model.set_material_properties", "material.set_properties");

        d.register_with_meta(
            CommandMetadata::new(
                "material.assign_slot",
                "Assign Material Slot",
                "Assign a material slot to the active object or selected faces",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Materials),
            AssignMaterialSlotCmd::default(),
        );
        d.alias("model.assign_material", "material.assign_slot");
        d.alias("model.assign_material_slot", "material.assign_slot");

        // Metadados do Objeto (Object Presentation)
        d.register_with_meta(
            CommandMetadata::new(
                "model.set_selection_color",
                "Set Object Selection Color",
                "Set authored custom selection highlight color for an object",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Interface),
            crate::SetAssetSelectionColorCmd::default(),
        );
        d.alias("object.set_selection_color", "model.set_selection_color");

        // Modificadores (Modifiers)
        use crate::modifier_commands::ModifierCmd;
        d.register_with_meta(
            CommandMetadata::new(
                "modifier.apply",
                "Apply Modifier",
                "Bake modifier deformation permanently into base mesh",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            ModifierCmd::default(),
        );
        d.alias("model.apply_modifier", "modifier.apply");

        // 5. Visualização (View)
        d.register_with_meta(
            CommandMetadata::new(
                "view.toggle_wireframe",
                "Toggle Wireframe",
                "Toggle wireframe display on active mesh",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            ToggleWireframeCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.toggle_wire_overlay",
                "Toggle Wire Overlay",
                "Display mesh edges over the active shading mode",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            ToggleWireOverlayCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.toggle_xray",
                "Toggle X-Ray",
                "Toggle semi-transparent see-through mesh display",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            ToggleXRayCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.toggle_face_orientation",
                "Toggle Face Orientation",
                "Display front faces in blue and back faces in red",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            ToggleFaceOrientationCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.toggle_uv_checker",
                "Toggle UV Checker",
                "Display procedural checkerboard pattern for UV inspection",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            ToggleUvCheckerCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "tools.toggle_proportional",
                "Toggle Proportional Editing",
                "Toggle proportional editing with falloff radius",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::Modeling),
            ToggleProportionalEditingCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "tools.toggle_snap",
                "Toggle Magnet Snap",
                "Toggle snapping to grid, vertices, edges or faces",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::Modeling),
            ToggleSnapCmd,
        );
        d.alias("tool.toggle_snap", "tools.toggle_snap");
        d.alias("view.toggle_snap", "tools.toggle_snap");
        d.alias("tool.toggle_proportional", "tools.toggle_proportional");
        d.register_with_meta(
            CommandMetadata::new(
                "view.frame_selection",
                "Frame Selection",
                "Center 3D camera on selected geometry",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            FrameSelectionCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.reset_camera",
                "Reset Camera",
                "Reset 3D camera to default isometric view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            ResetCameraCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.toggle_projection",
                "Toggle Projection",
                "Toggle perspective or orthographic view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            ToggleProjectionCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.frame_all",
                "Frame All",
                "Center 3D camera on all visible scene geometry",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            FrameAllCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.toggle_nav_hud",
                "Toggle Navigation HUD",
                "Toggle display of viewport orientation angle badge",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            ToggleNavHudCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.front",
                "View Front",
                "Align camera to Front orthographic view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::Front),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.back",
                "View Back",
                "Align camera to Back orthographic view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::Back),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.left",
                "View Left",
                "Align camera to Left orthographic view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::Left),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.right",
                "View Right",
                "Align camera to Right orthographic view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::Right),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.top",
                "View Top",
                "Align camera to Top orthographic view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::Top),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.bottom",
                "View Bottom",
                "Align camera to Bottom orthographic view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::Bottom),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.isometric_ne",
                "Isometric NE",
                "Align camera to North-East isometric view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::IsometricNE),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.isometric_nw",
                "Isometric NW",
                "Align camera to North-West isometric view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::IsometricNW),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.isometric_se",
                "Isometric SE",
                "Align camera to South-East isometric view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::IsometricSE),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.isometric_sw",
                "Isometric SW",
                "Align camera to South-West isometric view",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::IsometricSW),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "view.pseudo_isometric_2to1",
                "Pseudo-Isometric 2:1",
                "Align camera to classic 2:1 pseudo-isometric game view (26.565°)",
                CommandCategory::View,
            )
            .with_docs(DocsTopic::Navigation),
            SetViewPresetCmd(ViewPreset::PseudoIsometric2To1),
        );
        d.alias("view.camera_front", "view.front");
        d.alias("view.camera_back", "view.back");
        d.alias("view.camera_left", "view.left");
        d.alias("view.camera_right", "view.right");
        d.alias("view.camera_top", "view.top");
        d.alias("view.camera_bottom", "view.bottom");

        // 6. Janela e Interface (Window)
        d.register_with_meta(
            CommandMetadata::new(
                "window.command_palette",
                "Command Palette",
                "Open rapid search and execute palette",
                CommandCategory::Window,
            )
            .with_docs(DocsTopic::Interface),
            ToggleCommandPaletteCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "window.reference_manager",
                "Reference Set Manager",
                "Open reference images manager window",
                CommandCategory::Window,
            )
            .with_docs(DocsTopic::Interface),
            ToggleReferenceManagerCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "window.settings",
                "Preferences",
                "Open application preferences modal",
                CommandCategory::Window,
            )
            .with_docs(DocsTopic::Themes),
            ToggleSettingsCmd,
        );

        // 7. Ajuda (Help)
        d.register_with_meta(
            CommandMetadata::new(
                "help.documentation",
                "Documentation",
                "Open official documentation online",
                CommandCategory::Help,
            )
            .with_docs(DocsTopic::GettingStarted),
            ToggleHelpCmd,
        );

        d
    }
}

// -------------------------------------------------------------------------------------------------
// Comandos Canônicos
// -------------------------------------------------------------------------------------------------

/// Tipos de primitivas geométricas tridimensionais suportadas (V1: dez).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveKind {
    Cube,
    Sphere,
    Cylinder,
    Plane,
    Cone,
    Capsule,
    Wedge,
    Circle,
    Torus,
    Icosphere,
}

impl PrimitiveKind {
    /// Parses a primitive name. Unknown names are errors — never a silent Cone.
    pub fn parse(name: &str) -> Result<Self, CommandError> {
        match name {
            "Cube" | "cube" => Ok(Self::Cube),
            "Plane" | "plane" => Ok(Self::Plane),
            "Cylinder8" | "Cylinder" | "cylinder" => Ok(Self::Cylinder),
            "Sphere" | "sphere" => Ok(Self::Sphere),
            "Capsule" | "capsule" => Ok(Self::Capsule),
            "Cone" | "cone" => Ok(Self::Cone),
            "Wedge" | "wedge" => Ok(Self::Wedge),
            "Circle" | "circle" => Ok(Self::Circle),
            "Torus" | "torus" => Ok(Self::Torus),
            "Icosphere" | "icosphere" => Ok(Self::Icosphere),
            other => Err(CommandError::UnknownPrimitive(other.to_string())),
        }
    }

    pub fn default_name(&self) -> &'static str {
        match self {
            Self::Cube => "Cube",
            Self::Sphere => "Sphere",
            Self::Cylinder => "Cylinder",
            Self::Plane => "Plane",
            Self::Cone => "Cone",
            Self::Capsule => "Capsule",
            Self::Wedge => "Wedge",
            Self::Circle => "Circle",
            Self::Torus => "Torus",
            Self::Icosphere => "Icosphere",
        }
    }

    /// Nome i18n (`prims.cube`, …).
    pub fn name_key(&self) -> petunia_config::TextId {
        use petunia_config::text_id as T;
        match self {
            Self::Cube => T::PRIMS_CUBE,
            Self::Sphere => T::PRIMS_SPHERE,
            Self::Cylinder => T::PRIMS_CYLINDER,
            Self::Plane => T::PRIMS_PLANE,
            Self::Cone => T::PRIMS_CONE,
            Self::Capsule => T::PRIMS_CAPSULE,
            Self::Wedge => T::PRIMS_WEDGE,
            Self::Circle => T::PRIMS_CIRCLE,
            Self::Torus => T::PRIMS_TORUS,
            Self::Icosphere => T::PRIMS_ICOSPHERE,
        }
    }

    /// Malha padrão: defaults intencionalmente low-poly (§5 do gauntlet).
    pub fn generate_mesh(&self) -> Mesh {
        match self {
            Self::Cube => Mesh::cube(1.0),
            Self::Sphere => Mesh::sphere_low(12, 6, 0.5),
            Self::Cylinder => Mesh::cylinder(8, 0.5, 1.0),
            Self::Plane => Mesh::plane(1.0),
            Self::Cone => Mesh::cone(8, 0.5, 1.0),
            Self::Capsule => Mesh::capsule_profile(8, 0.3, 0.8, 2),
            Self::Wedge => Mesh::wedge(1.0, 1.0, 1.0),
            Self::Circle => Mesh::circle(1.0, 12, true),
            Self::Torus => Mesh::torus(1.0, 0.3, 12, 6),
            Self::Icosphere => Mesh::icosphere(0.5, 1),
        }
    }
}

/// Comando para inserção de primitiva geométrica na cena.
#[derive(Debug, Clone)]
pub struct AddPrimitiveCmd {
    pub kind: PrimitiveKind,
    pub name: Option<String>,
    pub at_cursor: bool,
}

impl AddPrimitiveCmd {
    pub fn new(kind: PrimitiveKind) -> Self {
        Self {
            kind,
            name: None,
            at_cursor: true,
        }
    }

    pub fn cube(at_cursor: bool) -> Self {
        Self {
            kind: PrimitiveKind::Cube,
            name: None,
            at_cursor,
        }
    }
}

impl Command for AddPrimitiveCmd {
    fn label(&self) -> &'static str {
        "add primitive"
    }

    /// Sem checkpoint do dispatcher: `begin_primitive` captura a transação
    /// única da sessão (sem isso, cada criação empilharia dois checkpoints).
    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        // Caminho canônico único: sessão de criação (§82), nunca inserção seca.
        // `at_cursor` é sempre verdadeiro nos chamadores (offset do cursor 3D).
        if state.begin_primitive(self.kind, self.name.clone()) {
            Ok(())
        } else {
            Err(CommandError::Execution("bulk creation failed".to_string()))
        }
    }
}

/// Comando para duplicação do asset ativo ou especificado.
#[derive(Debug, Clone, Default)]
pub struct DuplicateAssetCmd {
    pub asset_index: Option<usize>,
}

impl Command for DuplicateAssetCmd {
    fn label(&self) -> &'static str {
        "duplicate asset"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let idx = self.asset_index.unwrap_or(state.document.active);
        if state.document.assets.get(idx).is_none() {
            Err("No active asset to duplicate")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let idx = self.asset_index.unwrap_or(state.document.active);
        let Some(asset) = state.document.assets.get(idx) else {
            return Err(CommandError::InvalidAssetIndex(idx));
        };

        let copy = asset.duplicate();
        let copy_name = copy.name.clone();
        state.document.assets.push(copy);
        state.document.active = state.document.assets.len() - 1;
        state.set_status(format!("Duplicated {}", copy_name));
        Ok(())
    }
}

/// Reordena um asset na cena por índices, gravando o passo no histórico de desfazer (Undo/Redo).
/// Reorders an asset in the scene by indices, recording the step in the undo/redo history.
#[derive(Debug, Clone)]
pub struct ReorderAssetCmd {
    pub from: usize,
    pub to: usize,
}

impl Command for ReorderAssetCmd {
    fn label(&self) -> &'static str {
        "reorder asset"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let len = state.document.assets.len();
        if self.from >= len || self.to >= len || self.from == self.to {
            Err("Invalid reorder indices")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state.document.reorder_asset(self.from, self.to) {
            state.mark_dirty();
            state.set_status("Objeto reordenado na cena".to_string());
            Ok(())
        } else {
            Err(CommandError::InvalidAssetIndex(self.from))
        }
    }
}

/// Comando para instanciar um asset na cena em uma posição específica ou no 3D Cursor.
#[derive(Debug, Clone)]
pub struct InstantiateAssetCmd {
    pub asset_id: uuid::Uuid,
    pub position: Option<[f32; 3]>,
}

impl Command for InstantiateAssetCmd {
    fn label(&self) -> &'static str {
        "instantiate asset"
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::GEOMETRY | ProjectChanges::TRANSFORMS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.assets.iter().any(|a| a.id == self.asset_id) {
            Ok(())
        } else {
            Err("Asset not found in project library")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(asset) = state.document.assets.iter().find(|a| a.id == self.asset_id) else {
            return Err(CommandError::Execution("Asset not found".to_string()));
        };

        let target = glam::Vec3::from(self.position.unwrap_or(state.editor.cursor_3d));
        let instance = asset
            .duplicate_at_world(target)
            .map_err(|error| CommandError::Execution(error.to_string()))?;
        let name = instance.name.clone();
        state.document.assets.push(instance);
        state.document.active = state.document.assets.len() - 1;
        state.set_selection_domain(SelectionDomain::Object);
        state.set_status(format!("Instantiated '{}'", name));
        Ok(())
    }
}

/// Comando para exclusão segura de um asset da cena.
#[derive(Debug, Clone, Default)]
pub struct DeleteAssetCmd {
    pub asset_index: Option<usize>,
}

impl Command for DeleteAssetCmd {
    fn label(&self) -> &'static str {
        "delete asset"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let idx = self.asset_index.unwrap_or(state.document.active);
        if idx >= state.document.assets.len() {
            Err("No active asset to delete")
        } else if state.document.assets[idx].locked {
            Err("Object is locked")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let idx = self.asset_index.unwrap_or(state.document.active);
        if idx >= state.document.assets.len() {
            return Err(CommandError::InvalidAssetIndex(idx));
        }

        state.document.remove(idx);
        state.document.bump_topology();
        state.document.bump_selection();
        state.mark_dirty();
        state.set_status("Asset deleted");
        Ok(())
    }
}

/// Comando contextual para deletar a seleção (sub-elementos em Edit Mode ou asset em Object Mode).
#[derive(Debug, Clone, Default)]
pub struct DeleteSelectionCmd;

impl Command for DeleteSelectionCmd {
    fn label(&self) -> &'static str {
        "delete"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.edit_mode() == EditMode::Object {
            if state.document.active().is_none() {
                Err("No active asset to delete")
            } else if state.is_active_locked() {
                Err("Object is locked")
            } else {
                Ok(())
            }
        } else if state.selection.is_empty() {
            Err("No elements selected to delete")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state.edit_mode() == EditMode::Object {
            let selected = state.editor.selection.assets.clone();
            if selected.is_empty() {
                return DeleteAssetCmd { asset_index: None }.execute(state);
            }
            state
                .document
                .assets
                .retain(|asset| !selected.contains(&asset.id) || asset.locked);
            state.document.active = usize::MAX;
            state.editor.selection.assets.clear();
            state.document.bump_topology();
            state.document.bump_selection();
            state.mark_dirty();
            state.set_status("Deleted selected objects");
            return Ok(());
        }

        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !state.editor.selection.is_empty()
            && !mesh.faces.iter().any(|f| f.selected)
            && !mesh.verts.iter().any(|v| v.selected)
        {
            state.editor.selection.apply_to_mesh(mesh);
        }
        mesh.delete_selected();
        state.editor.selection.clear();
        state.sync_selection();
        state.set_status("Deleted selection");
        Ok(())
    }
}

/// Comando para duplicar a geometria selecionada dentro da malha ativa em Edit Mode.
#[derive(Debug, Clone, Default)]
pub struct DuplicateSelectionCmd;

impl Command for DuplicateSelectionCmd {
    fn label(&self) -> &'static str {
        "duplicate"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.edit_mode() == EditMode::Object {
            if state.document.active().is_none() {
                Err("No active asset to duplicate")
            } else {
                Ok(())
            }
        } else if state.selection.is_empty() {
            Err("No elements selected to duplicate")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state.edit_mode() == EditMode::Object {
            let selected = &state.editor.selection.assets;
            let copies: Vec<_> = state
                .document
                .assets
                .iter()
                .filter(|asset| selected.contains(&asset.id))
                .map(|asset| asset.duplicate())
                .collect();
            if copies.is_empty() {
                return DuplicateAssetCmd { asset_index: None }.execute(state);
            }
            state.editor.selection.assets = copies.iter().map(|asset| asset.id).collect();
            state.document.assets.extend(copies);
            state.document.active = state.document.assets.len() - 1;
            state.set_status("Duplicated selected objects");
            return Ok(());
        }

        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !state.editor.selection.is_empty()
            && !mesh.faces.iter().any(|f| f.selected)
            && !mesh.verts.iter().any(|v| v.selected)
        {
            state.editor.selection.apply_to_mesh(mesh);
        }
        mesh.duplicate_selected();
        state.sync_selection();
        state.set_status("Duplicated selection");
        Ok(())
    }
}

/// Comando para selecionar todos os elementos da malha ativa.
#[derive(Debug, Clone, Default)]
pub struct SelectAllCmd;

impl Command for SelectAllCmd {
    fn label(&self) -> &'static str {
        "select all"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SELECTION
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.selection_domain() != crate::SelectionDomain::Object
            && state.document.active_mesh().is_none()
        {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state.selection_domain() == crate::SelectionDomain::Object {
            state.editor.selection.assets = state
                .document
                .assets
                .iter()
                .filter(|a| a.visible && !a.locked)
                .map(|a| a.id)
                .collect();
            state.document.active = state
                .editor
                .selection
                .assets
                .last()
                .and_then(|id| state.document.assets.iter().position(|a| a.id == *id))
                .unwrap_or(usize::MAX);
            return Ok(());
        }
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.select_all();
        Ok(())
    }
}

/// Comando para limpar toda a seleção de elementos da malha ativa.
#[derive(Debug, Clone, Default)]
pub struct ClearSelectionCmd;

impl Command for ClearSelectionCmd {
    fn label(&self) -> &'static str {
        "clear selection"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SELECTION
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.selection_domain() != crate::SelectionDomain::Object
            && state.document.active_mesh().is_none()
        {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state.selection_domain() == crate::SelectionDomain::Object {
            state.select_object(None, false);
            return Ok(());
        }
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.deselect_all();
        Ok(())
    }
}

/// Comando para inverter a seleção de elementos da malha ativa.
#[derive(Debug, Clone, Default)]
pub struct InvertSelectionCmd;

impl Command for InvertSelectionCmd {
    fn label(&self) -> &'static str {
        "invert selection"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SELECTION
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.selection_domain() != crate::SelectionDomain::Object
            && state.document.active_mesh().is_none()
        {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state.selection_domain() == crate::SelectionDomain::Object {
            let selected = &state.editor.selection.assets;
            let ids: Vec<_> = state
                .document
                .assets
                .iter()
                .filter(|a| a.visible && !a.locked && !selected.contains(&a.id))
                .map(|a| a.id)
                .collect();
            state.document.active = ids
                .last()
                .and_then(|id| state.document.assets.iter().position(|a| a.id == *id))
                .unwrap_or(usize::MAX);
            state.editor.selection.assets = ids;
            return Ok(());
        }
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.invert_selection();
        Ok(())
    }
}

/// Comando para definir o domínio de seleção ativo (Object, Vertex, Edge, Face) (P3D-015).
#[derive(Debug, Clone, Copy)]
pub struct SetSelectionDomainCmd(pub SelectionDomain);

impl Command for SetSelectionDomainCmd {
    fn label(&self) -> &'static str {
        match self.0 {
            SelectionDomain::Object => "select domain: object",
            SelectionDomain::Vertex => "select domain: vertex",
            SelectionDomain::Edge => "select domain: edge",
            SelectionDomain::Face => "select domain: face",
        }
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SELECTION
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.set_selection_domain(self.0);
        Ok(())
    }
}

/// Comando para alternar entre Object Mode e o último domínio de componente (Tab) (P3D-015).
#[derive(Debug, Clone, Default)]
pub struct CycleSelectionDomainCmd;

impl Command for CycleSelectionDomainCmd {
    fn label(&self) -> &'static str {
        "cycle selection domain"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SELECTION
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.cycle_selection_domain();
        Ok(())
    }
}

/// Comando para separar os elementos selecionados em um novo objeto/asset (P3D-037).
#[derive(Debug, Clone, Default)]
pub struct SeparateSelectionCmd;

impl Command for SeparateSelectionCmd {
    fn label(&self) -> &'static str {
        "separate selection"
    }

    fn is_destructive(&self) -> bool {
        true
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::ALL
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let Some(mesh) = state.document.active_mesh() else {
            return Err("No active mesh");
        };
        if !mesh.has_selection() {
            return Err("Select geometry to separate first");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(active_asset) = state.document.active() else {
            return Err(CommandError::NoActiveAsset);
        };
        let orig_name = active_asset.name.clone();
        let mut orig_mesh = active_asset.mesh.to_owned_value();
        let object_transform = active_asset.transform;
        let legacy_origin = active_asset.origin;
        let mat_id = active_asset.material_id;
        let base_color = active_asset.base_color;
        let collection = active_asset.collection.clone();
        let custom_overlay_color = active_asset.selection_overlay_color;

        let has_selected_faces = orig_mesh.faces.iter().any(|f| f.selected);
        if has_selected_faces {
            orig_mesh.sync_vert_selection_from_faces();
        } else if !orig_mesh.selected_edges.is_empty() {
            for &(a, b) in &orig_mesh.selected_edges {
                if let Some(v) = orig_mesh.verts.get_mut(a as usize) {
                    v.selected = true;
                }
                if let Some(v) = orig_mesh.verts.get_mut(b as usize) {
                    v.selected = true;
                }
            }
        }

        // 1. Constrói a malha separada a partir dos elementos selecionados
        let mut sep_mesh = petunia_mesh::Mesh::default();
        let mut vert_map = std::collections::HashMap::new();

        for (i, v) in orig_mesh.verts.iter().enumerate() {
            if v.selected {
                let new_idx = sep_mesh.verts.len() as u32;
                let mut nv = v.clone();
                nv.selected = false;
                sep_mesh.verts.push(nv);
                vert_map.insert(i as u32, new_idx);
            }
        }

        for f in &orig_mesh.faces {
            let include_face = if has_selected_faces {
                f.selected
            } else {
                f.verts.iter().all(|vi| vert_map.contains_key(vi))
            };
            if include_face
                && let Some(new_verts) = f
                    .verts
                    .iter()
                    .map(|vi| vert_map.get(vi).copied())
                    .collect::<Option<Vec<u32>>>()
            {
                let mut nf = petunia_mesh::Face::with_uv(new_verts, f.uv.clone());
                nf.selected = false;
                nf.material_slot = f.material_slot;
                sep_mesh.push_face(nf);
            }
        }

        if sep_mesh.verts.is_empty() {
            return Err(CommandError::Execution("No geometry separated".into()));
        }
        // Comandos internos nunca auto-reparam: validação estrita detecta bugs
        // do próprio editor em vez de consertar silenciosamente
        // (website/docs/03-geometry/validation-repair-cleanup.md).
        sep_mesh
            .validate_invariants()
            .map_err(|e| CommandError::Execution(format!("Malha separada inválida: {e}")))?;

        // 2. Remove os elementos selecionados da malha original
        if let Some(active_mesh) = state.document.active_mesh_mut() {
            // Aplica a mesma sincronização antes de deletar
            if active_mesh.faces.iter().any(|f| f.selected) {
                active_mesh.sync_vert_selection_from_faces();
            }
            active_mesh.delete_selected();
            active_mesh
                .validate_invariants()
                .map_err(|e| CommandError::Execution(format!("Malha ativa inválida: {e}")))?;
        }

        // 3. Adiciona a malha separada como novo objeto/asset na cena
        let new_name = format!("{}_sep", orig_name);
        let mut new_asset = petunia_project::Asset::new(&new_name, sep_mesh);
        new_asset.set_object_transform(object_transform);
        new_asset.origin = legacy_origin;
        new_asset.material_id = mat_id;
        new_asset.base_color = base_color;
        new_asset.collection = collection;
        new_asset.selection_overlay_color = custom_overlay_color;
        state.document.assets.push(new_asset);
        state.document.active = state.document.assets.len() - 1;
        state.sync_selection();
        state.emit_mesh_changed();
        state.set_status(format!("Separated selection into {}", new_name));
        Ok(())
    }
}

/// Comando para resetar a posição (translação) do objeto ativo para [0.0, 0.0, 0.0] (Blender Alt+G).
#[derive(Debug, Clone, Default)]
pub struct ClearLocationCmd;

impl Command for ClearLocationCmd {
    fn label(&self) -> &'static str {
        "clear location"
    }

    fn is_destructive(&self) -> bool {
        true
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let Some(active) = state.document.active() else {
            return Err("No active object");
        };
        if active.locked {
            return Err("Active object is locked");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        clear_object_frame(state, ClearFramePart::Location)
    }
}

/// Comando para resetar a rotação do objeto ativo para [0.0, 0.0, 0.0] (Blender Alt+R).
#[derive(Debug, Clone, Default)]
pub struct ClearRotationCmd;

impl Command for ClearRotationCmd {
    fn label(&self) -> &'static str {
        "clear rotation"
    }

    fn is_destructive(&self) -> bool {
        true
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let Some(active) = state.document.active() else {
            return Err("No active object");
        };
        if active.locked {
            return Err("Active object is locked");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        clear_object_frame(state, ClearFramePart::Rotation)
    }
}

/// Comando para resetar a escala do objeto ativo para [1.0, 1.0, 1.0] (Blender Alt+S).
#[derive(Debug, Clone, Default)]
pub struct ClearScaleCmd;

impl Command for ClearScaleCmd {
    fn label(&self) -> &'static str {
        "clear scale"
    }

    fn is_destructive(&self) -> bool {
        true
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let Some(active) = state.document.active() else {
            return Err("No active object");
        };
        if active.locked {
            return Err("Active object is locked");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        clear_object_frame(state, ClearFramePart::Scale)
    }
}

/// Comando para resetar todas as transformações (posição, rotação e escala) do objeto ativo.
#[derive(Debug, Clone, Default)]
pub struct ClearAllTransformsCmd;

impl Command for ClearAllTransformsCmd {
    fn label(&self) -> &'static str {
        "clear all transforms"
    }

    fn is_destructive(&self) -> bool {
        true
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let Some(active) = state.document.active() else {
            return Err("No active object");
        };
        if active.locked {
            return Err("Active object is locked");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        clear_object_frame(state, ClearFramePart::All)
    }
}

#[derive(Clone, Copy)]
enum ClearFramePart {
    Location,
    Rotation,
    Scale,
    All,
}

fn clear_object_frame(
    state: &mut CommandContext<'_>,
    part: ClearFramePart,
) -> Result<(), CommandError> {
    let asset = state
        .document
        .active_mut()
        .ok_or(CommandError::NoActiveAsset)?;
    if asset.locked {
        return Err(CommandError::Execution("Asset is locked".into()));
    }
    if asset.has_legacy_baked_transform() {
        return clear_legacy_object_frame(asset, part);
    }
    let mut transform = asset.transform;
    match part {
        ClearFramePart::Location => transform.translation = [0.0; 3],
        ClearFramePart::Rotation => transform.rotation = [0.0, 0.0, 0.0, 1.0],
        ClearFramePart::Scale => transform.scale = [1.0; 3],
        ClearFramePart::All => transform = petunia_project::ObjectTransform::IDENTITY,
    }
    if !transform.is_finite() || !transform.to_mat4().inverse().is_finite() {
        return Err(CommandError::Execution("Invalid object frame".into()));
    }
    if transform == asset.transform {
        return Err(CommandError::NoChange("Object frame already reset".into()));
    }
    asset.set_object_transform(transform);
    state.set_status("Object frame reset");
    Ok(())
}

fn clear_legacy_object_frame(
    asset: &mut petunia_project::Asset,
    part: ClearFramePart,
) -> Result<(), CommandError> {
    use glam::{EulerRot, Mat4, Quat, Vec3};
    let position = Vec3::from(asset.position);
    let rotation = Vec3::from(asset.rotation);
    let scale = Vec3::from(asset.scale);
    let pivot = asset.origin.map_or_else(|| asset.mesh.center(), Vec3::from);
    if !position.is_finite() || !rotation.is_finite() || !scale.is_finite() || !pivot.is_finite() {
        return Err(CommandError::Execution("Invalid legacy transform".into()));
    }
    let world = asset.world_matrix();
    let pivot = world.transform_point3(pivot);
    let around = |matrix| Mat4::from_translation(pivot) * matrix * Mat4::from_translation(-pivot);
    let mut matrix = Mat4::IDENTITY;
    let reset_scale = matches!(part, ClearFramePart::Scale | ClearFramePart::All);
    let reset_rotation = matches!(part, ClearFramePart::Rotation | ClearFramePart::All);
    let reset_location = matches!(part, ClearFramePart::Location | ClearFramePart::All);
    if reset_scale {
        matrix = around(Mat4::from_scale(scale.recip())) * matrix;
    }
    if reset_rotation {
        let q = Quat::from_euler(
            EulerRot::XYZ,
            rotation.x.to_radians(),
            rotation.y.to_radians(),
            rotation.z.to_radians(),
        );
        matrix = around(Mat4::from_quat(q.inverse())) * matrix;
    }
    if reset_location {
        matrix = Mat4::from_translation(-position) * matrix;
    }
    let matrix = world.inverse() * matrix * world;
    if !matrix.is_finite() {
        return Err(CommandError::Execution("Invalid legacy transform".into()));
    }
    if matrix == Mat4::IDENTITY {
        return Err(CommandError::NoChange(
            "Legacy transform already reset".into(),
        ));
    }
    if asset.parametric.is_some() || asset.skeleton_id.is_some() || asset.skin_data.is_some() {
        return Err(CommandError::Execution(
            "Resetting baked procedural/rigged transforms requires explicit source compensation"
                .into(),
        ));
    }
    let mut mesh = (*asset.mesh).clone();
    for vertex in &mut mesh.verts {
        let next = matrix.transform_point3(vertex.vec());
        if !next.is_finite() {
            return Err(CommandError::Execution("Legacy transform overflow".into()));
        }
        vertex.pos = next.to_array();
    }
    let origin = asset
        .origin
        .map(|p| matrix.transform_point3(Vec3::from(p)).to_array());
    if origin.is_some_and(|p| !Vec3::from(p).is_finite()) {
        return Err(CommandError::Execution("Legacy origin overflow".into()));
    }
    asset.mesh = mesh.into();
    asset.origin = origin;
    if reset_location {
        asset.position = [0.0; 3];
    }
    if reset_rotation {
        asset.rotation = [0.0; 3];
    }
    if reset_scale {
        asset.scale = [1.0; 3];
    }
    Ok(())
}

/// Comando para subdividir a geometria selecionada na malha ativa.
/// Cuts come from `state.tools.subdivide_cuts` (default 1) so CLI/FFI/tools
/// share one implementation without changing the unit-struct call sites.
#[derive(Debug, Clone, Default)]
pub struct SubdivideSelectionCmd;

impl Command for SubdivideSelectionCmd {
    fn label(&self) -> &'static str {
        "subdivide"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else if state.active_is_composition() {
            Err("Bake solid composition before direct polygon editing")
        } else if state.selection.is_empty()
            && state
                .document
                .active_mesh()
                .is_none_or(|m| !m.faces.iter().any(|f| f.selected) && m.selected_edges.is_empty())
        {
            Err("Select geometry to subdivide")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let cuts = state.tools.subdivide_cuts.clamp(1, 6);
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !state.editor.selection.is_empty() {
            state.editor.selection.apply_to_mesh(mesh);
        }
        mesh.subdivide_selected_cuts(cuts);
        state.sync_selection();
        state.set_status(format!("Subdivided selection ({cuts} cuts)"));
        Ok(())
    }
}

/// Comando para fundir elementos selecionados no centro na malha ativa.
#[derive(Debug, Clone, Default)]
pub struct MergeCenterCmd;

impl Command for MergeCenterCmd {
    fn label(&self) -> &'static str {
        "merge"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.selection.verts.len() < 2 {
            Err("Select at least 2 vertices")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !state.editor.selection.verts.is_empty() {
            state.editor.selection.apply_to_mesh(mesh);
        }
        mesh.merge_center();
        state.sync_selection();
        state.set_status("Merged selection at center");
        Ok(())
    }
}

/// Comando para fundir vértices duplicados por distância (merge by distance).
#[derive(Debug, Clone)]
pub struct WeldCmd {
    pub eps: f32,
}

impl Default for WeldCmd {
    fn default() -> Self {
        Self { eps: 0.01 }
    }
}

impl Command for WeldCmd {
    fn label(&self) -> &'static str {
        "weld"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let before_verts = mesh.verts.len();
        let result = mesh.weld_with_remap(self.eps.max(0.0));
        let removed = before_verts.saturating_sub(mesh.verts.len());
        state.editor.selection.remap(&result.remap);
        state.sync_selection();
        state.set_status(format!("Welded {removed} vertices (eps {:.4})", self.eps));
        Ok(())
    }
}

/// Comando para tornar a malha simétrica copiando um lado para o outro.
#[derive(Debug, Clone)]
pub struct SymmetrizeCmd {
    pub axis: usize,
    pub positive_to_negative: bool,
    pub eps: f32,
}

impl Default for SymmetrizeCmd {
    fn default() -> Self {
        Self {
            axis: 0,
            positive_to_negative: true,
            eps: 0.001,
        }
    }
}

impl Command for SymmetrizeCmd {
    fn label(&self) -> &'static str {
        "symmetrize"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let mirrored = mesh.symmetrize(self.axis, self.positive_to_negative, self.eps);
        if mirrored == 0 {
            return Err(CommandError::Execution(
                "Nothing to symmetrize on this side".into(),
            ));
        }
        state.set_status(format!("Symmetrized {mirrored} vertices"));
        Ok(())
    }
}

/// Comando para inverter a orientação das normais da malha ativa.
#[derive(Debug, Clone, Default)]
pub struct FlipNormalsCmd;

impl Command for FlipNormalsCmd {
    fn label(&self) -> &'static str {
        "flip_normals"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NORMALS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.flip_normals();
        state.set_status("Flipped normals");
        Ok(())
    }
}

/// Marca como Sharp as arestas selecionadas do objeto ativo (decisão autoral que
/// delimita smoothing islands; não é seam nem feature edge).
#[derive(Debug, Clone, Copy, Default)]
pub struct MarkSharpCmd;

impl Command for MarkSharpCmd {
    fn label(&self) -> &'static str {
        "mark sharp"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NORMALS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let mesh = state.document.active_mesh().ok_or("No active mesh")?;
        if mesh.selected_edges.is_empty() {
            Err("Select edges to mark sharp")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mesh = state
            .document
            .active_mesh_mut()
            .ok_or(CommandError::NoActiveAsset)?;
        let edges: Vec<_> = mesh.selected_edges.iter().copied().collect();
        let changed = mesh.mark_sharp(edges);
        if changed == 0 {
            return Err(CommandError::NoChange("edges are already sharp".into()));
        }
        state.set_status(format!("Marked {changed} sharp edge(s)"));
        Ok(())
    }
}

/// Remove a marca Sharp das arestas selecionadas do objeto ativo.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClearSharpCmd;

impl Command for ClearSharpCmd {
    fn label(&self) -> &'static str {
        "clear sharp"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NORMALS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let mesh = state.document.active_mesh().ok_or("No active mesh")?;
        if mesh.selected_edges.is_empty() {
            Err("Select edges to clear sharp")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mesh = state
            .document
            .active_mesh_mut()
            .ok_or(CommandError::NoActiveAsset)?;
        let edges: Vec<_> = mesh.selected_edges.iter().copied().collect();
        let changed = mesh.clear_sharp(edges);
        if changed == 0 {
            return Err(CommandError::NoChange("edges are not sharp".into()));
        }
        state.set_status(format!("Cleared {changed} sharp edge(s)"));
        Ok(())
    }
}

/// Smooth by Angle: torna o objeto Smooth e grava como Sharp as arestas internas
/// acima do limiar. Não é Auto Smooth permanente: o limiar é só parâmetro do comando.
#[derive(Debug, Clone, Copy)]
pub struct SmoothByAngleCmd {
    pub angle_deg: f32,
}

impl Default for SmoothByAngleCmd {
    fn default() -> Self {
        Self { angle_deg: 30.0 }
    }
}

impl Command for SmoothByAngleCmd {
    fn label(&self) -> &'static str {
        "smooth by angle"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NORMALS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if !self.angle_deg.is_finite() || !(0.0..=180.0).contains(&self.angle_deg) {
            return Err("Angle must be between 0 and 180 degrees");
        }
        state
            .document
            .active_mesh()
            .map(|_| ())
            .ok_or("No active mesh")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let id = state
            .document
            .active()
            .map(|asset| asset.id)
            .ok_or(CommandError::NoActiveAsset)?;
        let marked = state
            .document
            .active_mesh_mut()
            .ok_or(CommandError::NoActiveAsset)?
            .smooth_by_angle(self.angle_deg);
        state.document.project.set_smooth_shaded(id, true);
        state.set_status(format!(
            "Smooth by angle {:.0}°: {marked} sharp edge(s)",
            self.angle_deg
        ));
        Ok(())
    }
}

/// Propaga winding consistente por componente conexo (malhas abertas ou fechadas).
#[derive(Debug, Clone, Copy, Default)]
pub struct OrientConsistentlyCmd;

impl Command for OrientConsistentlyCmd {
    fn label(&self) -> &'static str {
        "orient consistently"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NORMALS | ProjectChanges::GEOMETRY
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        state
            .document
            .active_mesh()
            .map(|_| ())
            .ok_or("No active mesh")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
            .active_mesh_mut()
            .ok_or(CommandError::NoActiveAsset)?
            .orient_consistently();
        state.set_status("Oriented faces consistently");
        Ok(())
    }
}

/// Orienta consistentemente e escolhe o exterior pelo volume com sinal.
/// Recusa malhas abertas.
#[derive(Debug, Clone, Copy, Default)]
pub struct OrientOutwardCmd;

impl Command for OrientOutwardCmd {
    fn label(&self) -> &'static str {
        "orient outward"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NORMALS | ProjectChanges::GEOMETRY
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let mesh = state.document.active_mesh().ok_or("No active mesh")?;
        if mesh.is_closed_manifold() {
            Ok(())
        } else {
            Err("Orient Outward needs a closed manifold mesh")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
            .active_mesh_mut()
            .ok_or(CommandError::NoActiveAsset)?
            .orient_outward()
            .map_err(|error| CommandError::Execution(error.to_string()))?;
        state.set_status("Oriented faces outward");
        Ok(())
    }
}

/// Define sombreamento suave (`smooth = true`) ou facetado do asset ativo.
/// A escolha é persistida por objeto e honrada pelo renderer (cap. 05).
#[derive(Debug, Clone, Copy)]
pub struct SetShadeSmoothCmd {
    pub smooth: bool,
}

impl Command for SetShadeSmoothCmd {
    fn label(&self) -> &'static str {
        if self.smooth {
            "shade smooth"
        } else {
            "shade flat"
        }
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NORMALS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let asset = state.document.active().ok_or("No active mesh")?;
        if state.document.is_smooth_shaded(asset.id) == self.smooth {
            Err(if self.smooth {
                "Object is already smooth shaded"
            } else {
                "Object is already flat shaded"
            })
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let id = state
            .document
            .active()
            .map(|asset| asset.id)
            .ok_or(CommandError::NoActiveAsset)?;
        if !state.document.project.set_smooth_shaded(id, self.smooth) {
            return Err(CommandError::NoChange("shading already set".into()));
        }
        state.set_status(if self.smooth {
            "Smooth shading"
        } else {
            "Flat shading"
        });
        Ok(())
    }
}

/// Comando para inverter a diagonal de triangulação interna de quads selecionados
/// ou executar edge-flip em aresta compartilhada por triângulos.
#[derive(Debug, Clone, Default)]
pub struct FlipDiagonalCmd;

impl Command for FlipDiagonalCmd {
    fn label(&self) -> &'static str {
        "flip diagonal"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.selection.is_empty()
            && !state
                .document
                .active_mesh()
                .is_some_and(|m| m.has_selection())
        {
            Err("Select quad or edge first")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if mesh.flip_diagonal() {
            state.set_status("Flipped quad diagonal / triangle edge");
            Ok(())
        } else {
            Err(CommandError::Execution(
                "No selected quad or edge suitable for diagonal flip".into(),
            ))
        }
    }
}

/// Comando para rotacionar/revolver perfil selecionado ao redor de um eixo coordenado.
#[derive(Debug, Clone)]
pub struct RevolveCmd {
    pub segments: u32,
    pub angle_deg: f32,
    pub axis: usize,
    pub center: [f32; 3],
}

impl Default for RevolveCmd {
    fn default() -> Self {
        Self {
            segments: 16,
            angle_deg: 360.0,
            axis: 1, // Eixo Y
            center: [0.0, 0.0, 0.0],
        }
    }
}

impl Command for RevolveCmd {
    fn label(&self) -> &'static str {
        "revolve"
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if mesh.revolve_selection(self.segments, self.angle_deg, self.axis, self.center) {
            state.set_status(format!(
                "Revolved selection ({} segments, {:.0}°)",
                self.segments, self.angle_deg
            ));
            Ok(())
        } else {
            Err(CommandError::Execution(
                "Failed to revolve selection: require selected connected edges".into(),
            ))
        }
    }
}

/// Comando para extrudar faces selecionadas individualmente (desacopladas).
#[derive(Debug, Clone)]
pub struct ExtrudeIndividualCmd {
    pub dist: f32,
}

impl Default for ExtrudeIndividualCmd {
    fn default() -> Self {
        Self { dist: 0.0 }
    }
}

impl Command for ExtrudeIndividualCmd {
    fn label(&self) -> &'static str {
        "extrude individual"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if !state.selection.faces.is_empty()
            || state
                .document
                .active_mesh()
                .is_some_and(|m| m.faces.iter().any(|f| f.selected))
        {
            Ok(())
        } else {
            Err("Select faces first")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !mesh.faces.iter().any(|f| f.selected) {
            return Err(CommandError::EmptySelection);
        }
        mesh.extrude_individual(self.dist);
        state.set_status(format!("Extruded individual faces ({:.2})", self.dist));
        Ok(())
    }
}

/// Comando para selecionar todos os elementos conectados à seleção atual (Select Linked).
#[derive(Debug, Clone, Default)]
pub struct SelectLinkedCmd;

impl Command for SelectLinkedCmd {
    fn label(&self) -> &'static str {
        "select linked"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.selection.is_empty()
            && !state
                .document
                .active_mesh()
                .is_some_and(|m| m.has_selection())
        {
            Err("Select at least one element first")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.select_linked();
        Ok(())
    }
}

/// Comando para seleção por área (retângulo 2D projetado via matriz de visão/projeção).
#[derive(Debug, Clone)]
pub struct BoxSelectCmd {
    pub p0: [f32; 2],
    pub p1: [f32; 2],
    pub view_proj: [f32; 16],
    pub add: bool,
}

impl Command for BoxSelectCmd {
    fn label(&self) -> &'static str {
        "box select"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.box_select(self.p0, self.p1, &self.view_proj, self.add);
        Ok(())
    }
}

/// Comando para alternar o bloqueio (lock) do asset ativo ou especificado.
#[derive(Debug, Clone, Default)]
pub struct ToggleLockAssetCmd {
    pub asset_index: Option<usize>,
}

impl Command for ToggleLockAssetCmd {
    fn label(&self) -> &'static str {
        "toggle lock"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let idx = self.asset_index.unwrap_or(state.document.active);
        let Some(asset) = state.document.assets.get_mut(idx) else {
            return Err(CommandError::InvalidAssetIndex(idx));
        };
        asset.locked = !asset.locked;
        let name = asset.name.clone();
        let locked = asset.locked;
        state.set_status(if locked {
            format!("Locked {}", name)
        } else {
            format!("Unlocked {}", name)
        });
        Ok(())
    }
}

/// Comando para alternar a visibilidade do asset ativo ou especificado.
#[derive(Debug, Clone, Default)]
pub struct ToggleVisibilityAssetCmd {
    pub asset_index: Option<usize>,
}

impl Command for ToggleVisibilityAssetCmd {
    fn label(&self) -> &'static str {
        "toggle visibility"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let idx = self.asset_index.unwrap_or(state.document.active);
        let Some(asset) = state.document.assets.get_mut(idx) else {
            return Err(CommandError::InvalidAssetIndex(idx));
        };
        asset.visible = !asset.visible;
        let name = asset.name.clone();
        let visible = asset.visible;
        state.set_status(if visible {
            format!("Showed {}", name)
        } else {
            format!("Hid {}", name)
        });
        Ok(())
    }
}

/// Comando para definir as transformações absolutas (posição, rotação Euler XYZ em graus, escala) de um asset.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SetAssetTransformCmd {
    pub asset_index: Option<usize>,
    pub translation: Option<[f32; 3]>,
    pub rotation_euler_deg: Option<[f32; 3]>,
    pub scale: Option<[f32; 3]>,
}

pub type SetObjectTransformCmd = SetAssetTransformCmd;

impl Command for SetAssetTransformCmd {
    fn label(&self) -> &'static str {
        "set object transform"
    }

    fn is_destructive(&self) -> bool {
        true
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let idx = self.asset_index.unwrap_or(state.document.active);
        let Some(asset) = state.document.assets.get(idx) else {
            return Err("No active object");
        };
        if asset.locked {
            return Err("Active object is locked");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let idx = self.asset_index.unwrap_or(state.document.active);
        let Some(asset) = state.document.assets.get_mut(idx) else {
            return Err(CommandError::InvalidAssetIndex(idx));
        };
        if asset.locked {
            return Err(CommandError::Execution("Asset is locked".into()));
        }

        if let Some(pos) = self.translation
            && !pos.iter().all(|v| v.is_finite())
        {
            return Err(CommandError::Execution(
                "Translation contains non-finite values".into(),
            ));
        }
        if let Some(rot) = self.rotation_euler_deg
            && !rot.iter().all(|v| v.is_finite())
        {
            return Err(CommandError::Execution(
                "Rotation contains non-finite values".into(),
            ));
        }
        if let Some(scale) = self.scale {
            if !scale.iter().all(|v| v.is_finite()) {
                return Err(CommandError::Execution(
                    "Scale contains non-finite values".into(),
                ));
            }
            if scale.iter().any(|v| v.abs() < 1e-6) {
                return Err(CommandError::Execution("Scale cannot be zero".into()));
            }
        }

        let target_pos = self.translation.unwrap_or(asset.position);
        let target_rot = self.rotation_euler_deg.unwrap_or(asset.rotation);
        let target_scale = self.scale.unwrap_or(asset.scale);
        if let Some((channel, axis)) = asset.transform_locks.first_violation(
            [asset.position, asset.rotation, asset.scale],
            [target_pos, target_rot, target_scale],
        ) {
            return Err(CommandError::Execution(format!(
                "{} {} está travada",
                channel.label(),
                petunia_project::transform_locks::axis_label(axis)
            )));
        }

        if target_pos == asset.position
            && target_rot == asset.rotation
            && target_scale == asset.scale
        {
            return Err(CommandError::NoChange("Transform unchanged".into()));
        }

        let quat = glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            target_rot[0].to_radians(),
            target_rot[1].to_radians(),
            target_rot[2].to_radians(),
        );
        let transform = petunia_project::ObjectTransform::from_trs(
            glam::Vec3::from_array(target_pos),
            quat,
            glam::Vec3::from_array(target_scale),
        );
        if !transform.is_finite() || !transform.to_mat4().inverse().is_finite() {
            return Err(CommandError::Execution("Invalid object transform".into()));
        }

        asset.set_object_transform_with_euler(transform, target_rot);
        state.set_status("Object transform updated");
        Ok(())
    }
}

/// Comando para definir a coleção organizadora de um asset.
#[derive(Debug, Clone)]
pub struct SetAssetCollectionCmd {
    pub asset_index: usize,
    pub collection: Option<String>,
}

impl Command for SetAssetCollectionCmd {
    fn label(&self) -> &'static str {
        "set collection"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(asset) = state.document.assets.get_mut(self.asset_index) else {
            return Err(CommandError::InvalidAssetIndex(self.asset_index));
        };
        let left = std::mem::replace(&mut asset.collection, self.collection.clone());
        let name = asset.name.clone();
        if let Some(group) = left.as_deref()
            && self.collection.as_deref() != Some(group)
        {
            state.document.project.prune_collection_if_empty(group);
        }
        if let Some(ref col) = self.collection {
            state.set_status(format!("Moved {} to collection {}", name, col));
        } else {
            state.set_status(format!("Removed {} from collections", name));
        }
        Ok(())
    }
}

/// Comando para alternar a visibilidade de todos os assets de uma coleção.
#[derive(Debug, Clone)]
pub struct ToggleCollectionVisibilityCmd {
    pub collection: String,
}

impl Command for ToggleCollectionVisibilityCmd {
    fn label(&self) -> &'static str {
        "toggle collection visibility"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let all_vis = state
            .document
            .assets
            .iter()
            .filter(|a| a.collection.as_deref() == Some(&self.collection))
            .all(|a| a.visible);
        for a in &mut state.document.assets {
            if a.collection.as_deref() == Some(&self.collection) {
                a.visible = !all_vis;
            }
        }
        state.set_status(format!(
            "Collection {}: {}",
            self.collection,
            if !all_vis { "visible" } else { "hidden" }
        ));
        Ok(())
    }
}

/// Comando para alternar o bloqueio (lock) de todos os assets de uma coleção.
#[derive(Debug, Clone)]
pub struct ToggleCollectionLockCmd {
    pub collection: String,
}

impl Command for ToggleCollectionLockCmd {
    fn label(&self) -> &'static str {
        "toggle collection lock"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let all_locked = state
            .document
            .assets
            .iter()
            .filter(|a| a.collection.as_deref() == Some(&self.collection))
            .all(|a| a.locked);
        for a in &mut state.document.assets {
            if a.collection.as_deref() == Some(&self.collection) {
                a.locked = !all_locked;
            }
        }
        state.set_status(format!(
            "Collection {}: {}",
            self.collection,
            if !all_locked { "locked" } else { "unlocked" }
        ));
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------
// Comandos Adicionais do Catálogo Canônico
// -------------------------------------------------------------------------------------------------

/// Comando para criar um novo projeto limpo.
#[derive(Debug, Clone, Default)]
pub struct NewProjectCmd;

impl Command for NewProjectCmd {
    fn label(&self) -> &'static str {
        "new project"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.request_host(HostRequest::NewProject);
        Ok(())
    }
}

/// Comando para salvar o projeto ativo no caminho atual.
#[derive(Debug, Clone, Default)]
pub struct SaveProjectCmd;

impl Command for SaveProjectCmd {
    fn label(&self) -> &'static str {
        "save project"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if !state.is_document_dirty() && state.document.project_path.is_some() {
            Err("Document has no unsaved changes")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if let Some(ref path_str) = state.document.project_path.clone() {
            state.request_host(HostRequest::SaveProject(std::path::PathBuf::from(path_str)));
            Ok(())
        } else {
            state.set_status("Save As required (no destination file)");
            Ok(())
        }
    }
}

/// Comando de boundary para solicitar a abertura de um projeto.
#[derive(Debug, Clone, Default)]
pub struct OpenProjectCmd;

impl Command for OpenProjectCmd {
    fn label(&self) -> &'static str {
        "open project"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.set_status("Open Project requested");
        Ok(())
    }
}

/// Comando para salvar o projeto em novo arquivo.
#[derive(Debug, Clone, Default)]
pub struct SaveProjectAsCmd;

impl Command for SaveProjectAsCmd {
    fn label(&self) -> &'static str {
        "save project as"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.set_status("Save As requested");
        Ok(())
    }
}

/// Comando para importar arquivo Wavefront OBJ.
#[derive(Debug, Clone, Default)]
pub struct ImportObjCmd;

impl Command for ImportObjCmd {
    fn label(&self) -> &'static str {
        "import obj"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.set_status("Import OBJ requested");
        Ok(())
    }
}

/// Comando para importar malha glTF/GLB (com materiais e texturas).
#[derive(Debug, Clone, Default)]
pub struct ImportGltfCmd;

impl Command for ImportGltfCmd {
    fn label(&self) -> &'static str {
        "import gltf/glb"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.set_status("Import glTF/GLB requested");
        Ok(())
    }
}

/// Comando para exportar malha ativa em formato Wavefront OBJ.
#[derive(Debug, Clone, Default)]
pub struct ExportObjCmd;

impl Command for ExportObjCmd {
    fn label(&self) -> &'static str {
        "export obj"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.set_status("Export OBJ requested");
        Ok(())
    }
}

/// Comando para exportar cena em formato glTF binário (GLB).
#[derive(Debug, Clone, Default)]
pub struct ExportGlbCmd;

impl Command for ExportGlbCmd {
    fn label(&self) -> &'static str {
        "export glb"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.set_status("Export GLB requested");
        Ok(())
    }
}

/// Comando para salvar o modelo ativo na biblioteca permanente do projeto.
#[derive(Debug, Clone, Default)]
pub struct SaveActiveAsAssetCmd;

impl Command for SaveActiveAsAssetCmd {
    fn label(&self) -> &'static str {
        "save active as asset"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active().is_none() {
            Err("No active model selected")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state.save_active_as_asset() {
            Ok(())
        } else {
            Err(CommandError::NoActiveAsset)
        }
    }
}

/// Comando para desfazer última modificação (Undo).
#[derive(Debug, Clone, Default)]
pub struct UndoCmd;

impl Command for UndoCmd {
    fn label(&self) -> &'static str {
        "undo"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.undo.can_undo() {
            Ok(())
        } else {
            Err("Nothing to undo")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.undo();
        Ok(())
    }
}

/// Comando para refazer modificação previamente desfeita (Redo).
#[derive(Debug, Clone, Default)]
pub struct RedoCmd;

impl Command for RedoCmd {
    fn label(&self) -> &'static str {
        "redo"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.undo.can_redo() {
            Ok(())
        } else {
            Err("Nothing to redo")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.redo();
        Ok(())
    }
}

/// Comando para extrusão conectada da seleção de faces.
#[derive(Debug, Clone)]
pub struct ExtrudeSelectedCmd {
    pub dist: f32,
}

impl Default for ExtrudeSelectedCmd {
    fn default() -> Self {
        Self { dist: 0.5 }
    }
}

impl Command for ExtrudeSelectedCmd {
    fn label(&self) -> &'static str {
        "extrude"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else if state.active_is_composition() {
            Err("Bake solid composition before direct polygon editing")
        } else if !state.selection.faces.is_empty()
            || state
                .document
                .active_mesh()
                .is_some_and(|m| m.faces.iter().any(|f| f.selected))
        {
            Ok(())
        } else {
            Err("Select faces first")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let dist = if self.dist != 0.0 {
            self.dist
        } else {
            state.tools.extrude_dist
        };
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !state.editor.selection.faces.is_empty() {
            state.editor.selection.apply_to_mesh(mesh);
        }
        if !mesh.faces.iter().any(|f| f.selected) {
            return Err(CommandError::EmptySelection);
        }
        mesh.extrude_selected(dist);
        state.sync_selection();
        state.set_status(format!("Extruded faces ({:.2})", dist));
        Ok(())
    }
}

/// Ativa a sessão modal de Push/Pull. O deslocamento vem do arrasto na
/// viewport ou da entrada numérica do inspector.
#[derive(Debug, Clone, Copy, Default)]
pub struct PushPullToolCmd;

impl Command for PushPullToolCmd {
    fn label(&self) -> &'static str {
        "push/pull"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else if state.active_is_composition() {
            Err("Bake solid composition before direct polygon editing")
        } else if state
            .document
            .active_mesh()
            .is_some_and(|m| m.faces.iter().any(|f| f.selected))
        {
            Ok(())
        } else {
            Err("Select faces first")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .begin_modal(crate::ModalKind::PushPull)
            .map_err(|error| CommandError::Execution(error.to_string()))
    }
}

/// Abre uma sessão de corte (Knife) sobre a malha ativa.
#[derive(Debug, Clone, Copy, Default)]
pub struct KnifeToolCmd;

impl Command for KnifeToolCmd {
    fn label(&self) -> &'static str {
        "knife"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(asset) = state.document.active() else {
            return Err(CommandError::NoActiveAsset);
        };
        let session = crate::CutSession::new(asset.mesh.to_owned_value())
            .with_object_to_world(asset.world_matrix());
        state.editor.tools.cut_session = Some(session);
        state.editor.tools.active_tool = "cut".to_string();
        state.set_status("Knife: pick two edge points, Enter confirms");
        Ok(())
    }
}

/// **Join**: funde os demais objetos selecionados no ativo preservando todas as
/// topologias lado a lado (sem kernel booleano; booleanas são só por nós).
#[derive(Debug, Clone, Copy, Default)]
pub struct JoinObjectsCmd;

impl Command for JoinObjectsCmd {
    fn label(&self) -> &'static str {
        "join"
    }

    /// Sem checkpoint do dispatcher: `join_selected_into_active` captura a única.
    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let Some(active) = state.document.active() else {
            return Err("No active mesh");
        };
        let others = state
            .selection
            .assets
            .iter()
            .filter(|id| **id != active.id)
            .filter(|id| state.document.assets.iter().any(|asset| asset.id == **id))
            .count();
        if others == 0 {
            return Err("Select the objects to join and keep one active");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        match state.join_selected_into_active() {
            Ok(verts) => {
                state.set_status(format!("Join: {verts} vertices merged"));
                Ok(())
            }
            Err(reason) => Err(CommandError::Execution(reason.to_string())),
        }
    }
}

/// Inset métrico das faces selecionadas (website/docs/03-geometry/
/// direct-modeling-semantics.md, "Inset"): Region por padrão, distância em
/// unidades de mundo; o clamp aparece no status, nunca silencioso.
#[derive(Debug, Clone)]
pub struct InsetFacesCmd {
    /// Distância em unidades de mundo; `0.0` usa a da ferramenta.
    pub distance: f32,
    /// `None` usa o modo da ferramenta.
    pub mode: Option<petunia_mesh::InsetMode>,
}

impl Default for InsetFacesCmd {
    fn default() -> Self {
        Self {
            distance: 0.0,
            mode: None,
        }
    }
}

impl Command for InsetFacesCmd {
    fn label(&self) -> &'static str {
        "inset"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else if !state.selection.faces.is_empty()
            || state
                .document
                .active_mesh()
                .is_some_and(|m| m.faces.iter().any(|f| f.selected))
        {
            Ok(())
        } else {
            Err("Select faces first")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let distance = if self.distance != 0.0 {
            self.distance
        } else {
            state.tools.inset_distance
        };
        let mode = self.mode.unwrap_or(state.tools.inset_mode);
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !state.editor.selection.faces.is_empty() {
            state.editor.selection.apply_to_mesh(mesh);
        }
        let faces: Vec<usize> = mesh
            .faces
            .iter()
            .enumerate()
            .filter(|(_, f)| f.selected)
            .map(|(i, _)| i)
            .collect();
        if faces.is_empty() {
            return Err(CommandError::EmptySelection);
        }
        let report = mesh.inset_faces(&faces, distance, mode);
        if report.applied <= 0.0 {
            return Err(CommandError::NoChange(format!(
                "inset {distance:.2} cannot be applied to this selection"
            )));
        }
        state.sync_selection();
        let mut status = if report.limited {
            format!(
                "Inset requested {:.2} · applied {:.2} · limit reached",
                report.requested, report.applied
            )
        } else {
            format!("Inset {:.2}", report.applied)
        };
        if report.skipped_regions > 0 {
            status.push_str(&format!(
                " · {} region(s) with unsupported topology skipped",
                report.skipped_regions
            ));
        }
        state.set_status(status);
        Ok(())
    }
}

/// Comando para chanfrar (bevel) arestas selecionadas.
#[derive(Debug, Clone)]
pub struct BevelCmd {
    pub amount: f32,
    pub segments: u32,
    pub clamp_overlap: bool,
    pub affect_vertices: bool,
}

impl Default for BevelCmd {
    fn default() -> Self {
        Self {
            amount: 0.1,
            segments: 1,
            clamp_overlap: true,
            affect_vertices: false,
        }
    }
}

impl Command for BevelCmd {
    fn label(&self) -> &'static str {
        "bevel"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.active_is_composition() {
            Err("Bake solid composition before direct polygon editing")
        } else if let Some(mesh) = state.document.active_mesh() {
            let has_edges = !state.selection.edges.is_empty() || !mesh.selected_edges.is_empty();
            let has_verts =
                !state.selection.verts.is_empty() || mesh.verts.iter().any(|v| v.selected);
            if !has_edges && !has_verts {
                Err("Select edges or points first")
            } else {
                Ok(())
            }
        } else {
            Err("No active mesh")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let amount = if self.amount != 0.0 {
            self.amount
        } else {
            state.tools.bevel_amount
        };
        // UI comum oferece 1–8 (website/docs/03-geometry, "Bevel / Segments").
        let segments = if self.segments == 0 {
            state.tools.bevel_segments
        } else {
            self.segments
        }
        .clamp(1, 8);
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let has_app_sel =
            !state.editor.selection.edges.is_empty() || !state.editor.selection.verts.is_empty();
        let has_mesh_sel = !mesh.selected_edges.is_empty() || mesh.verts.iter().any(|v| v.selected);
        if !has_app_sel && !has_mesh_sel {
            return Err(CommandError::EmptySelection);
        }
        if has_app_sel {
            state.editor.selection.apply_to_mesh(mesh);
        }
        let has_verts = mesh.verts.iter().any(|v| v.selected);
        if (self.affect_vertices && has_verts) || mesh.selected_edges.is_empty() {
            let (applied, unsupported) = mesh.bevel_selected_vertex(amount, self.clamp_overlap);
            if applied == 0 {
                return Err(CommandError::NoChange(format!(
                    "no selected point can be beveled ({unsupported} unsupported)"
                )));
            }
            state.sync_selection();
            state.set_status(format!("Bevel {applied} point(s)"));
            return Ok(());
        }
        // Conjunto inteiro validado antes: qualquer aresta sem solução = nada muda.
        let edges: Vec<(u32, u32)> = mesh.selected_edges.iter().copied().collect();
        let report = mesh.bevel_edges(&edges, amount, segments, self.clamp_overlap);
        if !report.unsupported_edges.is_empty() {
            let listed: Vec<String> = report
                .unsupported_edges
                .iter()
                .take(6)
                .map(|(a, b)| format!("{a}-{b}"))
                .collect();
            return Err(CommandError::Execution(format!(
                "Bevel not applied: {} of {} selected edge(s) have unsupported topology ({}{})",
                report.unsupported_edges.len(),
                edges.len(),
                listed.join(", "),
                if report.unsupported_edges.len() > listed.len() {
                    ", …"
                } else {
                    ""
                }
            )));
        }
        state.sync_selection();
        let mut status = format!(
            "Bevel {} edge(s) · {} segment(s)",
            report.beveled_edges, report.segments
        );
        if report.clamped_edges > 0 {
            status.push_str(&format!(
                " · {} clamped by overlap limit",
                report.clamped_edges
            ));
        }
        state.set_status(status);
        Ok(())
    }
}

/// Uniform scale of the current selection around its center.
#[derive(Debug, Clone)]
pub struct ScaleSelectionCmd {
    pub factor: f32,
}

impl Default for ScaleSelectionCmd {
    fn default() -> Self {
        Self { factor: 1.0 }
    }
}

impl Command for ScaleSelectionCmd {
    fn label(&self) -> &'static str {
        "scale"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::POSITIONS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let factor = if self.factor == 0.0 {
            state.tools.transform_scale
        } else {
            self.factor
        };
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !state.editor.selection.verts.is_empty() {
            let center = mesh.vertices_center(&state.editor.selection.verts);
            mesh.scale_vertices(
                &state.editor.selection.verts,
                [factor, factor, factor],
                center,
            );
        } else {
            let center = mesh.selection_center();
            mesh.scale_selected(factor, center);
        }
        state.tools.transform_scale = 1.0;
        state.sync_selection();
        state.set_status(format!("Scaled selection ({factor:.2})"));
        Ok(())
    }
}

/// Comando para alternar entre sombreamento sólido e aramado (Wireframe).
#[derive(Debug, Clone, Default)]
pub struct ToggleWireframeCmd;

impl Command for ToggleWireframeCmd {
    fn label(&self) -> &'static str {
        "toggle wireframe"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.shading = match state.shading {
            crate::Shading::Wireframe => crate::Shading::Solid,
            _ => crate::Shading::Wireframe,
        };
        state.mark_dirty();
        Ok(())
    }
}

/// Alterna somente o overlay de arestas; não altera o modo-base de shading.
#[derive(Debug, Clone, Default)]
pub struct ToggleWireOverlayCmd;

impl Command for ToggleWireOverlayCmd {
    fn label(&self) -> &'static str {
        "toggle wire overlay"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.show_wireframe_overlay = !state.show_wireframe_overlay;
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para alternar exibição de Raio-X (X-Ray).
#[derive(Debug, Clone, Default)]
pub struct ToggleXRayCmd;

impl Command for ToggleXRayCmd {
    fn label(&self) -> &'static str {
        "toggle xray"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.show_xray = !state.show_xray;
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para alternar o overlay visual de orientação de faces (azuis para fora, vermelhas para dentro).
#[derive(Debug, Clone, Default)]
pub struct ToggleFaceOrientationCmd;

impl Command for ToggleFaceOrientationCmd {
    fn label(&self) -> &'static str {
        "toggle face orientation"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.editor.show_face_orientation = !state.editor.show_face_orientation;
        state.set_status(if state.editor.show_face_orientation {
            "Face orientation overlay enabled"
        } else {
            "Face orientation overlay disabled"
        });
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para alternar o shader/overlay de mapa xadrez UV Checkerboard.
#[derive(Debug, Clone, Default)]
pub struct ToggleUvCheckerCmd;

impl Command for ToggleUvCheckerCmd {
    fn label(&self) -> &'static str {
        "toggle uv checker"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.editor.show_uv_checker = !state.editor.show_uv_checker;
        state.set_status(if state.editor.show_uv_checker {
            "UV checker overlay enabled"
        } else {
            "UV checker overlay disabled"
        });
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para alternar o modo de edição proporcional.
#[derive(Debug, Clone, Default)]
pub struct ToggleProportionalEditingCmd;

impl Command for ToggleProportionalEditingCmd {
    fn label(&self) -> &'static str {
        "toggle proportional editing"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.editor.proportional_editing = !state.editor.proportional_editing;
        state.editor.proportional_settings.enabled = state.editor.proportional_editing;
        state.set_status(if state.editor.proportional_editing {
            "Proportional editing enabled"
        } else {
            "Proportional editing disabled"
        });
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para alternar o snap magnético.
#[derive(Debug, Clone, Default)]
pub struct ToggleSnapCmd;

impl Command for ToggleSnapCmd {
    fn label(&self) -> &'static str {
        "toggle snap"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.editor.snap_enabled = !state.editor.snap_enabled;
        state.set_status(if state.editor.snap_enabled {
            "Magnet snap enabled"
        } else {
            "Magnet snap disabled"
        });
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para centralizar a câmera 3D na geometria selecionada (Frame Selection).
#[derive(Debug, Clone, Default)]
pub struct FrameSelectionCmd;

impl Command for FrameSelectionCmd {
    fn label(&self) -> &'static str {
        "frame selection"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.frame_selection();
        Ok(())
    }
}

/// Comando para resetar a câmera para vista padrão.
#[derive(Debug, Clone, Default)]
pub struct ResetCameraCmd;

impl Command for ResetCameraCmd {
    fn label(&self) -> &'static str {
        "reset camera"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.camera_frame = None;
        state.camera.reset();
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para alternar entre projeção em perspectiva e ortográfica.
#[derive(Debug, Clone, Default)]
pub struct ToggleProjectionCmd;

impl Command for ToggleProjectionCmd {
    fn label(&self) -> &'static str {
        "toggle projection"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.camera_frame = None;
        state.camera.toggle_projection();
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para enquadrar todo o conteúdo relevante da cena (Frame All, P3D-008).
#[derive(Debug, Clone, Default)]
pub struct FrameAllCmd;

impl Command for FrameAllCmd {
    fn label(&self) -> &'static str {
        "frame all"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.frame_all();
        Ok(())
    }
}

/// Comando para alternar exibição do Navigation HUD no viewport (P3D-005).
#[derive(Debug, Clone, Default)]
pub struct ToggleNavHudCmd;

impl Command for ToggleNavHudCmd {
    fn label(&self) -> &'static str {
        "toggle nav hud"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.editor.show_nav_hud = !state.editor.show_nav_hud;
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para alinhar a câmera a um preset canônico ou isométrico (P3D-004, P3D-006).
#[derive(Debug, Clone)]
pub struct SetViewPresetCmd(pub ViewPreset);

impl Command for SetViewPresetCmd {
    fn label(&self) -> &'static str {
        match self.0 {
            ViewPreset::Front => "view front",
            ViewPreset::Back => "view back",
            ViewPreset::Left => "view left",
            ViewPreset::Right => "view right",
            ViewPreset::Top => "view top",
            ViewPreset::Bottom => "view bottom",
            ViewPreset::IsometricNE => "view isometric ne",
            ViewPreset::IsometricNW => "view isometric nw",
            ViewPreset::IsometricSE => "view isometric se",
            ViewPreset::IsometricSW => "view isometric sw",
            ViewPreset::PseudoIsometric2To1 => "view pseudo-isometric 2:1",
            ViewPreset::Persp => "view perspective",
        }
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.camera_frame = None;
        state.camera.set_preset(self.0);
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para alternar exibição do Reference Set Manager (P3D-013).
#[derive(Debug, Clone, Default)]
pub struct ToggleReferenceManagerCmd;

impl Command for ToggleReferenceManagerCmd {
    fn label(&self) -> &'static str {
        "reference manager"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.request_ui(UiRequest::ToggleReferenceManager);
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para alternar exibição da Command Palette (P3D-081).
#[derive(Debug, Clone, Default)]
pub struct ToggleCommandPaletteCmd;

impl Command for ToggleCommandPaletteCmd {
    fn label(&self) -> &'static str {
        "command palette"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.request_ui(UiRequest::ToggleCommandPalette);
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para abrir janela de Preferências/Configurações.
#[derive(Debug, Clone, Default)]
pub struct ToggleSettingsCmd;

impl Command for ToggleSettingsCmd {
    fn label(&self) -> &'static str {
        "preferences"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.request_ui(UiRequest::ToggleSettings);
        state.mark_dirty();
        Ok(())
    }
}

/// Comando para abrir a documentação oficial.
#[derive(Debug, Clone, Default)]
pub struct ToggleHelpCmd;

impl Command for ToggleHelpCmd {
    fn label(&self) -> &'static str {
        "documentation"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.request_ui(UiRequest::ToggleHelp);
        state.mark_dirty();
        Ok(())
    }
}

/// Even Loop Cut on the first selected edge (uniform spacing when `even`, symmetric when `balanced`).
#[derive(Debug, Clone)]
pub struct LoopCutCmd {
    pub cuts: u32,
    pub even: bool,
    pub slide: f32,
    pub balanced: bool,
}

impl Default for LoopCutCmd {
    fn default() -> Self {
        Self {
            cuts: 1,
            even: true,
            slide: 0.0,
            balanced: false,
        }
    }
}

impl Command for LoopCutCmd {
    fn label(&self) -> &'static str {
        "loop cut"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let mesh = state.document.active_mesh().ok_or("No active mesh")?;
        if mesh.selected_edges.is_empty() {
            Err("Select an edge on a quad ring")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let cuts = self.cuts.clamp(1, 32) as usize;
        let Some(mesh) = state.document.active_mesh() else {
            return Err(CommandError::NoActiveAsset);
        };
        let seed = mesh
            .selected_edges
            .iter()
            .copied()
            .next()
            .ok_or(CommandError::EmptySelection)?;
        let ring = petunia_mesh::loop_cut::LoopRing::discover(mesh, seed)
            .map_err(|e| CommandError::Execution(e.to_string()))?;
        let next = if self.balanced {
            ring.apply_balanced(mesh, cuts, self.slide)
        } else {
            ring.apply_even(mesh, cuts, self.slide, self.even)
        }
        .map_err(|e| CommandError::Execution(e.to_string()))?;
        if let Some(dst) = state.document.active_mesh_mut() {
            *dst = next;
        }
        state.set_status(format!(
            "Loop cut ({cuts}{})",
            if self.balanced {
                " balanced"
            } else if self.even {
                " even"
            } else {
                ""
            }
        ));
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct UnwrapAutoCmd;

impl Command for UnwrapAutoCmd {
    fn label(&self) -> &'static str {
        "auto uv"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let charts = mesh
            .unwrap_auto()
            .map_err(|e| CommandError::Execution(e.to_string()))?;
        state.set_status(format!("Auto UV ({charts} charts)"));
        Ok(())
    }
}

/// Pack de ilhas UV (website/docs/07-uv/architecture-decision.md, "Pack",
/// "Padding"). A UI fala em pixels; a conversão para UV usa a resolução da
/// textura ativa do asset (`Asset::texture_size`). Com seleção UV,
/// só as ilhas que a tocam se movem; as demais viram obstáculos.
#[derive(Debug, Clone)]
pub struct UvPackIslandsCmd {
    /// Distância mínima entre ilhas e até a borda, em pixels.
    pub padding_px: f32,
    /// Permite girar 90° ilhas mais altas que largas.
    pub allow_rotation: bool,
    /// Ilhas com pin ficam onde estão.
    pub keep_pinned_islands: bool,
}

impl Default for UvPackIslandsCmd {
    fn default() -> Self {
        Self {
            padding_px: 4.0,
            allow_rotation: true,
            keep_pinned_islands: false,
        }
    }
}

impl Command for UvPackIslandsCmd {
    fn label(&self) -> &'static str {
        "pack uv"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(asset) = state.document.active_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let texture_size = asset.texture_size();
        let options = petunia_mesh::UvPackOptions {
            padding_px: self.padding_px,
            texture_size,
            allow_rotation: self.allow_rotation,
            keep_pinned_islands: self.keep_pinned_islands,
        };
        let mesh = &mut *asset.mesh;
        let faces = state.editor.uv_selection.faces(mesh);
        let report = mesh
            .pack_uv_islands_with(&faces, &options)
            .map_err(|e| CommandError::Execution(e.to_string()))?;
        if report.packed == 0 {
            return Err(CommandError::NoChange("no UV island to pack".into()));
        }
        state.set_status(format!("Packed {} UV islands", report.packed));
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct UvProjectFromViewCmd;

impl Command for UvProjectFromViewCmd {
    fn label(&self) -> &'static str {
        "project from view"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let origin = state.camera.eye();
        let forward = state.camera.forward();
        let up = state.camera.up();
        let right = forward.cross(up).normalize_or_zero();
        let Some(asset) = state.document.active_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        // A vista está em mundo: projeta as posições em mundo do objeto.
        let positions = asset.world_positions();
        asset
            .mesh
            .project_from_view_positions(&positions, right, up, origin);
        state.set_status("Projected UVs from view");
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct UvProjectFromReferenceCmd;

impl Command for UvProjectFromReferenceCmd {
    fn label(&self) -> &'static str {
        "project from reference"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let reference = state.document.refs.iter().find(|r| r.visible).cloned();
        let (right, up, origin) = if let Some(ref_img) = reference {
            let (vx, vy, vz) = match ref_img.axis {
                crate::RefAxis::Front => (glam::Vec3::X, glam::Vec3::Y, glam::Vec3::Z),
                crate::RefAxis::Back => (-glam::Vec3::X, glam::Vec3::Y, -glam::Vec3::Z),
                crate::RefAxis::Left => (-glam::Vec3::Z, glam::Vec3::Y, -glam::Vec3::X),
                crate::RefAxis::Right | crate::RefAxis::Side => {
                    (glam::Vec3::Z, glam::Vec3::Y, glam::Vec3::X)
                }
                crate::RefAxis::Top => (glam::Vec3::X, -glam::Vec3::Z, glam::Vec3::Y),
                crate::RefAxis::Bottom => (glam::Vec3::X, glam::Vec3::Z, -glam::Vec3::Y),
            };
            let rot = glam::Quat::from_axis_angle(vz, ref_img.rotation.to_radians());
            let right = rot * vx;
            let up = rot * vy;
            let origin = vz * ref_img.offset;
            (right, up, origin)
        } else {
            let origin = state.camera.eye();
            let forward = state.camera.forward();
            let up = state.camera.up();
            let right = forward.cross(up).normalize_or_zero();
            (right, up, origin)
        };

        let Some(asset) = state.document.active_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let positions = asset.world_positions();
        asset
            .mesh
            .project_from_view_positions(&positions, right, up, origin);
        state.set_status("Projected UVs from reference");
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct UvStitchCmd;

impl Command for UvStitchCmd {
    fn label(&self) -> &'static str {
        "uv stitch"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let faces = state.editor.uv_selection.faces(mesh);
        let count = mesh.stitch_uv(&faces);
        if count == 0 {
            return Err(CommandError::NoChange("no UV seam edge to stitch".into()));
        }
        state.set_status(format!("Stitched {count} UV seam edge(s)"));
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct UvRelaxCmd {
    pub iterations: usize,
}

impl Default for UvRelaxCmd {
    fn default() -> Self {
        Self { iterations: 5 }
    }
}

impl Command for UvRelaxCmd {
    fn label(&self) -> &'static str {
        "uv relax"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let faces = state.editor.uv_selection.faces(mesh);
        let count = mesh.relax_uv(&faces, self.iterations);
        if count == 0 {
            return Err(CommandError::NoChange("no UV face to relax".into()));
        }
        state.set_status(format!("Relaxed {count} UV face(s)"));
        Ok(())
    }
}

/// Move/Rotate/Scale direto dos cantos UV selecionados: um gesto = um Undo
/// (website/docs/07-uv/architecture-decision.md, "UV Transform", "Pins").
/// Pins não bloqueiam o gesto explícito; o canto continua fixado.
#[derive(Debug, Clone)]
pub struct UvTransformCmd {
    pub transform: petunia_mesh::UvTransform,
}

impl Command for UvTransformCmd {
    fn label(&self) -> &'static str {
        match self.transform {
            petunia_mesh::UvTransform::Translate(_) => "move uv",
            petunia_mesh::UvTransform::Rotate(_) => "rotate uv",
            petunia_mesh::UvTransform::Scale(_) => "scale uv",
        }
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if !self.transform.is_finite() {
            return Err(CommandError::Execution("non-finite UV transform".into()));
        }
        if state.editor.uv_selection.is_empty() {
            return Err(CommandError::EmptySelection);
        }
        let corners = state.editor.uv_selection.corner_set();
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if mesh.transform_uv_corners(&corners, self.transform) == 0 {
            return Err(CommandError::NoChange("no UV corner to transform".into()));
        }
        Ok(())
    }
}

/// Set Density: leva as ilhas selecionadas no UV (ou todas) a `px_per_unit`
/// px/unidade com a resolução da textura ativa
/// (website/docs/07-uv/architecture-decision.md, "Texel Density").
#[derive(Debug, Clone)]
pub struct UvSetTexelDensityCmd {
    pub px_per_unit: f32,
}

impl Command for UvSetTexelDensityCmd {
    fn label(&self) -> &'static str {
        "set texel density"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if !self.px_per_unit.is_finite() || self.px_per_unit <= 0.0 {
            return Err(CommandError::Execution(
                "texel density must be a positive number".into(),
            ));
        }
        let Some(asset) = state.document.active_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let texture_size = asset.texture_size();
        let mesh = &mut *asset.mesh;
        let faces = state.editor.uv_selection.faces(mesh);
        let islands = mesh.set_texel_density(&faces, texture_size, self.px_per_unit);
        if islands == 0 {
            return Err(CommandError::NoChange("density already matches".into()));
        }
        state.set_status(format!(
            "Set {:.1} px/unit on {islands} UV island(s)",
            self.px_per_unit
        ));
        Ok(())
    }
}

/// Split UV: seam na fronteira das faces totalmente selecionadas no UV, sem
/// duplicar vértice 3D (website/docs/07-uv/architecture-decision.md, "Split UV").
#[derive(Debug, Clone, Default)]
pub struct UvSplitCmd;

impl Command for UvSplitCmd {
    fn label(&self) -> &'static str {
        "uv split"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let faces = state.editor.uv_selection.faces(mesh);
        if faces.is_empty() {
            return Err(CommandError::EmptySelection);
        }
        let count = mesh.split_uv(&faces);
        if count == 0 {
            return Err(CommandError::NoChange("selection is already split".into()));
        }
        state.set_status(format!("Split UV along {count} edge(s)"));
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct PaintBakeReferenceCmd;

impl Command for PaintBakeReferenceCmd {
    fn label(&self) -> &'static str {
        "bake reference"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS | ProjectChanges::TEXTURES
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(ref_img) = state.document.refs.iter().find(|r| r.visible).cloned() else {
            return Err(CommandError::Execution(
                "No visible reference image to bake".into(),
            ));
        };
        if ref_img.width == 0 || ref_img.height == 0 || ref_img.rgba.is_empty() {
            return Err(CommandError::Execution(
                "Reference image has no pixel data".into(),
            ));
        }

        let (vx, vy, vz) = match ref_img.axis {
            crate::RefAxis::Front => (glam::Vec3::X, glam::Vec3::Y, glam::Vec3::Z),
            crate::RefAxis::Back => (-glam::Vec3::X, glam::Vec3::Y, -glam::Vec3::Z),
            crate::RefAxis::Left => (-glam::Vec3::Z, glam::Vec3::Y, -glam::Vec3::X),
            crate::RefAxis::Right | crate::RefAxis::Side => {
                (glam::Vec3::Z, glam::Vec3::Y, glam::Vec3::X)
            }
            crate::RefAxis::Top => (glam::Vec3::X, -glam::Vec3::Z, glam::Vec3::Y),
            crate::RefAxis::Bottom => (glam::Vec3::X, glam::Vec3::Z, -glam::Vec3::Y),
        };
        let rot = glam::Quat::from_axis_angle(vz, ref_img.rotation.to_radians());
        let right = rot * vx;
        let up = rot * vy;
        let origin = vz * ref_img.offset;

        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.project_from_view(right, up, origin);

        let active_idx = state.document.active;
        if let Some(asset) = state.document.assets.get_mut(active_idx) {
            let tex = asset.texture.get_or_insert_with(|| {
                petunia_project::Canvas::new(ref_img.width, ref_img.height, [255, 255, 255, 255])
            });
            let tw = tex.w as usize;
            let th = tex.h as usize;
            for y in 0..th {
                for x in 0..tw {
                    let rx = (x * ref_img.width as usize / tw).min(ref_img.width as usize - 1);
                    let ry = (y * ref_img.height as usize / th).min(ref_img.height as usize - 1);
                    let src_idx = (ry * ref_img.width as usize + rx) * 4;
                    let dst_idx = (y * tw + x) * 4;
                    if src_idx + 3 < ref_img.rgba.len() && dst_idx + 3 < tex.pixels.len() {
                        let a = ref_img.rgba[src_idx + 3] as f32 / 255.0 * ref_img.opacity;
                        if a > 0.01 {
                            for c in 0..3 {
                                let orig = tex.pixels[dst_idx + c] as f32;
                                let new_val = ref_img.rgba[src_idx + c] as f32;
                                tex.pixels[dst_idx + c] =
                                    (orig * (1.0 - a) + new_val * a).round() as u8;
                            }
                            tex.pixels[dst_idx + 3] =
                                (tex.pixels[dst_idx + 3] as f32 * (1.0 - a) + 255.0 * a).round()
                                    as u8;
                        }
                    }
                }
            }
        }

        state.set_status("Baked reference image into texture");
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ConnectLoopsCmd {
    pub segments: usize,
    pub twist: i32,
}

impl Default for ConnectLoopsCmd {
    fn default() -> Self {
        Self {
            segments: 1,
            twist: 0,
        }
    }
}

impl Command for ConnectLoopsCmd {
    fn label(&self) -> &'static str {
        "connect"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let mesh = state.document.active_mesh().ok_or("No active mesh")?;
        let n = mesh.faces.iter().filter(|f| f.selected).count();
        if n == 2 || !mesh.selected_edges.is_empty() {
            Ok(())
        } else {
            Err("Select two faces or two boundary edge loops")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state.selection_domain() == crate::SelectionDomain::Edge {
            let mesh = state
                .document
                .active_mesh_mut()
                .ok_or(CommandError::NoActiveAsset)?;
            mesh.connect_selected_edges_with_options(self.segments, self.twist)
                .map_err(CommandError::Execution)?;
            state.set_status(format!(
                "Connected selected boundaries ({} segments, {} twist)",
                self.segments, self.twist
            ));
            return Ok(());
        }
        let faces: Vec<usize> = state
            .document
            .active_mesh()
            .map(|m| {
                m.faces
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.selected)
                    .map(|(i, _)| i)
                    .collect()
            })
            .unwrap_or_default();
        if faces.len() != 2 {
            return Err(CommandError::EmptySelection);
        }
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.connect_loops_with_options(faces[0], faces[1], self.segments, self.twist)
            .map_err(CommandError::Execution)?;
        state.set_status(format!(
            "Connected loops ({} segments, {} twist)",
            self.segments, self.twist
        ));
        Ok(())
    }
}

/// Comando para dissolver seleção (arestas, faces ou vértices) sem deixar buracos na malha.
#[derive(Debug, Clone, Default)]
pub struct DissolveCmd;

impl Command for DissolveCmd {
    fn label(&self) -> &'static str {
        "dissolve"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let mesh = state.document.active_mesh().ok_or("No active mesh")?;
        let has_sel = !mesh.selected_edges.is_empty()
            || mesh.faces.iter().any(|f| f.selected)
            || mesh.verts.iter().any(|v| v.selected);
        if has_sel {
            Ok(())
        } else {
            Err("Select edges, faces or vertices to dissolve")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let (verts, faces) = (mesh.verts.len(), mesh.faces.len());
        mesh.dissolve_selected();
        if mesh.verts.len() == verts && mesh.faces.len() == faces {
            return Err(CommandError::NoChange(
                "selection could not be dissolved".into(),
            ));
        }
        state.set_status("Dissolved selected geometry");
        Ok(())
    }
}

/// Comando contextual usado pelo Delete da viewport em domínio de componente.
/// Tenta preservar a superfície por dissolve; quando a topologia não muda,
/// aplica delete explícito como fallback na mesma transação.
#[derive(Debug, Clone, Default)]
pub struct DeleteOrDissolveSelectionCmd;

impl Command for DeleteOrDissolveSelectionCmd {
    fn label(&self) -> &'static str {
        "delete/dissolve selection"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let mesh = state.document.active_mesh().ok_or("No active mesh")?;
        if !mesh.selected_edges.is_empty()
            || mesh.faces.iter().any(|face| face.selected)
            || mesh.verts.iter().any(|vertex| vertex.selected)
        {
            Ok(())
        } else {
            Err("Select edges, faces, or points to delete")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        // Se houver faces selecionadas, o usuário quer deletar as faces!
        // Deleta diretamente a face selecionada sem alterar a topologia das faces vizinhas.
        if mesh.faces.iter().any(|face| face.selected) {
            mesh.delete_selected();
            state.set_status("Deleted selected faces");
            return Ok(());
        }
        let counts = (
            mesh.verts.len(),
            mesh.faces.len(),
            mesh.selected_edges.len(),
        );
        mesh.dissolve_selected();
        if counts
            == (
                mesh.verts.len(),
                mesh.faces.len(),
                mesh.selected_edges.len(),
            )
        {
            mesh.delete_selected();
        }
        state.set_status("Dissolved/deleted selection");
        Ok(())
    }
}

/// Comando para criar uma nova face a partir de 3 ou mais arestas/vértices selecionados (atalho 'F').
#[derive(Debug, Clone, Default)]
pub struct MakeFaceCmd;

impl Command for MakeFaceCmd {
    fn label(&self) -> &'static str {
        "make_face"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let mesh = state.document.active_mesh().ok_or("No active mesh")?;
        let vert_count = mesh.verts.iter().filter(|v| v.selected).count();
        let edge_count = mesh.selected_edges.len();
        if edge_count >= 2 || vert_count >= 2 {
            Ok(())
        } else {
            Err("Select at least 3 edges or vertices to make a face (or 2 to make an edge)")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        match mesh.make_face_from_selection() {
            Ok(()) => {
                state.set_status("Face created from selection");
                Ok(())
            }
            Err(err) => {
                state.set_status(format!("Make face: {err}"));
                Err(CommandError::Execution(err.into()))
            }
        }
    }
}

/// Altera a posição, escala e rotação de uma camada Decal na pilha de pintura ativa (P3D-133).
/// Changes the position, scale, and rotation of a Decal layer in the active paint stack (P3D-133).
#[derive(Debug, Clone)]
pub struct SetDecalTransformCmd {
    pub layer_id: uuid::Uuid,
    pub center_uv: [f32; 2],
    pub scale_uv: [f32; 2],
    pub rotation_rad: f32,
}

impl Command for SetDecalTransformCmd {
    fn label(&self) -> &'static str {
        "set decal transform"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::TEXTURES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let Some(asset) = state.document.active() else {
            return Err("No active asset");
        };
        let Some(stack) = asset.paint_stack.as_ref() else {
            return Err("No paint stack");
        };
        if stack.layers.iter().any(|l| {
            l.id == self.layer_id
                && matches!(l.kind, petunia_project::paint_layers::LayerKind::Decal(_))
        }) {
            Ok(())
        } else {
            Err("Decal layer not found")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let active = state.document.active;
        let changed = if let Some(asset) = state.document.assets.get_mut(active)
            && let Some(stack) = asset.paint_stack.as_mut()
            && stack.set_decal_transform(
                self.layer_id,
                self.center_uv,
                self.scale_uv,
                self.rotation_rad,
            ) {
            // SVG re-rasterizado na resolução que a nova área pede.
            let (w, h) = asset.texture.as_ref().map_or((0, 0), |c| (c.w, c.h));
            let _ = stack.fit_decal_svg(self.layer_id, Some(&*asset.mesh), w, h);
            true
        } else {
            false
        };
        if !changed {
            return Err(CommandError::Execution(
                "Failed to update decal transform".into(),
            ));
        }
        state.document.project.composite_paint_stack(active);
        state.mark_canvas_dirty();
        state.mark_dirty();
        Ok(())
    }
}

/// Converte uma camada Decal em uma camada Raster estática por rasterização da projeção (P3D-160).
/// Converts a Decal layer into a static Raster layer by baking its projection (P3D-160).
#[derive(Debug, Clone)]
pub struct BakeDecalCmd {
    pub layer_id: uuid::Uuid,
}

/// Adiciona um recurso de path persistente sem criar mesh derivada (P3D-161).
#[derive(Debug, Clone)]
pub struct CreateSplineCmd {
    pub spline: SplineResource,
}

impl Command for CreateSplineCmd {
    fn label(&self) -> &'static str {
        "create spline"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.project.get_spline(self.spline.id).is_some() {
            Err("Spline id already exists")
        } else if self.spline.validate_authoring().is_err() {
            Err("Spline data is invalid")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.document.project.add_spline(self.spline.clone())?;
        Ok(())
    }
}

/// Substitui o conteúdo autoral de uma spline preservando seu identificador.
///
/// É usado por edições compostas (primitivas 2D e suavização em lote) para que
/// uma ação do usuário produza exatamente uma entrada de Undo.
#[derive(Debug, Clone)]
pub struct UpdateSplineCmd {
    pub spline: SplineResource,
}

impl Command for UpdateSplineCmd {
    fn label(&self) -> &'static str {
        "update spline"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        let current = project
            .get_spline(self.spline.id)
            .ok_or("Spline not found")?;
        if current == &self.spline {
            return Err("Spline data is unchanged");
        }
        self.spline
            .validate_authoring()
            .map_err(|_| "Spline data is invalid")?;
        if let Some(profile) = project
            .profiles
            .iter()
            .find(|profile| profile.spline_id == self.spline.id)
        {
            profile
                .validate_authoring(&self.spline)
                .map_err(|_| "Profile spline data is invalid")?;
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let spline = state
            .document
            .project
            .get_spline_mut(self.spline.id)
            .ok_or(SplineError::SplineNotFound(self.spline.id))?;
        // A revisão pertence ao recurso, não ao chamador: sempre avança uma vez.
        let next_revision = spline.revision.wrapping_add(1);
        *spline = self.spline.clone();
        spline.revision = next_revision;
        Ok(())
    }
}

/// Cria a curva planar e seu Profile persistente em uma única transação.
#[derive(Debug, Clone)]
pub struct CreateProfileCmd {
    pub spline: SplineResource,
    pub profile: ProfileResource,
}

impl Command for CreateProfileCmd {
    fn label(&self) -> &'static str {
        "create profile"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES | ProjectChanges::PROCEDURAL
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        if project.get_spline(self.spline.id).is_some() {
            return Err("Profile spline id already exists");
        }
        if project.get_profile(self.profile.id).is_some() {
            return Err("Profile id already exists");
        }
        if project
            .profiles
            .iter()
            .any(|profile| profile.spline_id == self.spline.id)
        {
            return Err("Profile spline is already owned");
        }
        self.profile
            .validate_authoring(&self.spline)
            .map_err(|_| "Profile data is invalid")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.document.project.add_spline(self.spline.clone())?;
        state.document.project.add_profile(self.profile.clone())?;
        Ok(())
    }
}

/// Atualiza metadados e workplane de um Profile sem trocar sua spline.
#[derive(Debug, Clone)]
pub struct UpdateProfileCmd {
    pub profile: ProfileResource,
}

impl Command for UpdateProfileCmd {
    fn label(&self) -> &'static str {
        "update profile"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::PROCEDURAL
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        let current = project
            .get_profile(self.profile.id)
            .ok_or("Profile not found")?;
        if current.spline_id != self.profile.spline_id {
            return Err("Profile spline cannot be replaced");
        }
        if current == &self.profile {
            return Err("Profile data is unchanged");
        }
        let spline = project
            .get_spline(self.profile.spline_id)
            .ok_or("Profile spline not found")?;
        self.profile
            .validate_authoring(spline)
            .map_err(|_| "Profile data is invalid")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let profile = state
            .document
            .project
            .get_profile_mut(self.profile.id)
            .ok_or(ProfileError::ProfileNotFound(self.profile.id))?;
        let next_revision = profile.revision.wrapping_add(1);
        *profile = self.profile.clone();
        profile.revision = next_revision;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeleteProfileCmd {
    pub profile_id: uuid::Uuid,
}

impl Command for DeleteProfileCmd {
    fn label(&self) -> &'static str {
        "delete profile"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES | ProjectChanges::PROCEDURAL
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        let profile = project
            .get_profile(self.profile_id)
            .ok_or("Profile not found")?;
        if project
            .path_generators
            .iter()
            .any(|generator| generator.profile_id == self.profile_id)
        {
            return Err("Profile is used by a path generator");
        }
        if project
            .path_generators
            .iter()
            .any(|generator| generator.path_id == profile.spline_id)
        {
            return Err("Profile spline is used as a generator path");
        }
        if project
            .profiles
            .iter()
            .any(|other| other.id != profile.id && other.spline_id == profile.spline_id)
        {
            return Err("Profile spline has another owner");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let profile = state.document.project.remove_profile(self.profile_id)?;
        state.document.project.remove_spline(profile.spline_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CreatePathGeneratorCmd {
    pub generator: PathGenerator,
}

impl Command for CreatePathGeneratorCmd {
    fn label(&self) -> &'static str {
        "create path generator"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::PROCEDURAL
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        if project.get_path_generator(self.generator.id).is_some() {
            return Err("Path generator id already exists");
        }
        project
            .validate_path_generator_candidate(&self.generator)
            .map_err(|_| "Path generator data or dependencies are invalid")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
            .project
            .add_path_generator(self.generator.clone())?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct UpdateSweepGeneratorCmd {
    pub generator_id: uuid::Uuid,
    pub parameters: SweepGeneratorParameters,
}

impl Command for UpdateSweepGeneratorCmd {
    fn label(&self) -> &'static str {
        "update sweep generator"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::PROCEDURAL
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        self.parameters
            .validate()
            .map_err(|_| "Sweep generator parameters are invalid")?;
        let generator = state
            .document
            .project
            .get_path_generator(self.generator_id)
            .ok_or("Path generator not found")?;
        if generator.sweep_parameters() == self.parameters {
            Err("Sweep generator parameters are unchanged")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if state
            .document
            .project
            .set_sweep_generator_parameters(self.generator_id, self.parameters)?
        {
            Ok(())
        } else {
            Err(CommandError::Execution(
                "Sweep generator parameters are unchanged".into(),
            ))
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeletePathGeneratorCmd {
    pub generator_id: uuid::Uuid,
}

impl Command for DeletePathGeneratorCmd {
    fn label(&self) -> &'static str {
        "delete path generator"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::PROCEDURAL
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        state
            .document
            .project
            .get_path_generator(self.generator_id)
            .map(|_| ())
            .ok_or("Path generator not found")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
            .project
            .remove_path_generator(self.generator_id)?;
        Ok(())
    }
}

/// Materializa a saída procedural em uma malha estática e remove o gerador vivo.
#[derive(Debug, Clone)]
pub struct BakePathGeneratorCmd {
    pub generator_id: uuid::Uuid,
    pub asset_name: String,
}

impl Command for BakePathGeneratorCmd {
    fn label(&self) -> &'static str {
        "bake path generator"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::GEOMETRY | ProjectChanges::PROCEDURAL
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        state
            .document
            .project
            .validate_path_generator(self.generator_id)
            .map_err(|_| "Path generator or one of its dependencies is invalid")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mut cache = PathGeneratorEvaluationCache::default();
        let evaluation = state.document.project.evaluate_path_generator(
            self.generator_id,
            PathGeneratorQuality::Final,
            &mut cache,
        )?;
        let mesh = evaluation.mesh.clone();
        let vertices = evaluation.diagnostics.vertices;
        let triangles = evaluation.diagnostics.triangles;
        let fallback_name = state
            .document
            .project
            .get_path_generator(self.generator_id)
            .map(|generator| generator.name.clone())
            .ok_or(PathGeneratorError::GeneratorNotFound(self.generator_id))?;
        let asset_name = if self.asset_name.trim().is_empty() {
            fallback_name
        } else {
            self.asset_name.trim().to_string()
        };
        state.document.project.add(&asset_name, mesh);
        state
            .document
            .project
            .remove_path_generator(self.generator_id)?;
        state.set_status(format!(
            "Baked {asset_name}: {vertices} vertices, {triangles} triangles"
        ));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeleteSplineCmd {
    pub spline_id: uuid::Uuid,
}

impl Command for DeleteSplineCmd {
    fn label(&self) -> &'static str {
        "delete spline"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let project = &state.document.project;
        project
            .get_spline(self.spline_id)
            .map(|_| ())
            .ok_or("Spline not found")?;
        if project
            .profiles
            .iter()
            .any(|profile| profile.spline_id == self.spline_id)
        {
            return Err("Spline is owned by a profile");
        }
        if project
            .path_generators
            .iter()
            .any(|generator| generator.path_id == self.spline_id)
        {
            return Err("Spline is used by a path generator");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.document.project.remove_spline(self.spline_id)?;
        Ok(())
    }
}

/// Um Profile exige spline planar e sem attachment. Comandos que editam uma
/// spline pertencente a um Profile aplicam a edição numa cópia e validam o
/// resultado, para que nenhum caminho (não só `UpdateSplineCmd`) quebre o generator.
fn validate_profile_edit(
    project: &petunia_project::Project,
    spline_id: uuid::Uuid,
    apply: impl FnOnce(&mut SplineResource) -> Result<(), SplineError>,
) -> Result<(), &'static str> {
    let Some(profile) = project
        .profiles
        .iter()
        .find(|profile| profile.spline_id == spline_id)
    else {
        return Ok(());
    };
    let mut spline = project
        .get_spline(spline_id)
        .ok_or("Spline not found")?
        .clone();
    apply(&mut spline).map_err(|_| "Spline edit is invalid")?;
    profile
        .validate_authoring(&spline)
        .map_err(|_| "Edit would break the Profile (must stay planar and unattached)")
}

fn spline_owned_by_profile(project: &petunia_project::Project, spline_id: uuid::Uuid) -> bool {
    project
        .profiles
        .iter()
        .any(|profile| profile.spline_id == spline_id)
}

#[derive(Debug, Clone)]
pub struct AddSplinePointCmd {
    pub spline_id: uuid::Uuid,
    pub index: Option<usize>,
    pub point: SplinePoint,
}

impl Command for AddSplinePointCmd {
    fn label(&self) -> &'static str {
        "add spline point"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let spline = state
            .document
            .project
            .get_spline(self.spline_id)
            .ok_or("Spline not found")?;
        let index = self.index.unwrap_or(spline.points.len());
        if index > spline.points.len() {
            return Err("Spline point index is invalid");
        }
        if spline.point(self.point.id).is_some() {
            return Err("Spline point id already exists");
        }
        if self
            .point
            .position
            .iter()
            .chain(&self.point.handle_in)
            .chain(&self.point.handle_out)
            .any(|component| !component.is_finite())
        {
            return Err("Spline point data is not finite");
        }
        validate_profile_edit(&state.document.project, self.spline_id, |spline| {
            spline.insert_point(index, self.point.clone())
        })
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let spline = state
            .document
            .project
            .get_spline_mut(self.spline_id)
            .ok_or(SplineError::SplineNotFound(self.spline_id))?;
        spline.insert_point(
            self.index.unwrap_or(spline.points.len()),
            self.point.clone(),
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MoveSplinePointCmd {
    pub spline_id: uuid::Uuid,
    pub point_id: uuid::Uuid,
    pub position: [f64; 3],
}

impl Command for MoveSplinePointCmd {
    fn label(&self) -> &'static str {
        "move spline point"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let spline = state
            .document
            .project
            .get_spline(self.spline_id)
            .ok_or("Spline not found")?;
        let point = spline
            .point(self.point_id)
            .ok_or("Spline point not found")?;
        if point.attachment.is_some() {
            Err("Attached points must be edited with surface attachment commands")
        } else if self.position.iter().any(|component| !component.is_finite()) {
            Err("Spline point position is not finite")
        } else if point.position == self.position {
            Err("Spline point position is unchanged")
        } else {
            validate_profile_edit(&state.document.project, self.spline_id, |spline| {
                spline.move_point(self.point_id, self.position)
            })
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
            .project
            .get_spline_mut(self.spline_id)
            .ok_or(SplineError::SplineNotFound(self.spline_id))?
            .move_point(self.point_id, self.position)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DeleteSplinePointCmd {
    pub spline_id: uuid::Uuid,
    pub point_id: uuid::Uuid,
}

impl Command for DeleteSplinePointCmd {
    fn label(&self) -> &'static str {
        "delete spline point"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        state
            .document
            .project
            .get_spline(self.spline_id)
            .ok_or("Spline not found")?
            .point(self.point_id)
            .map(|_| ())
            .ok_or("Spline point not found")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
            .project
            .get_spline_mut(self.spline_id)
            .ok_or(SplineError::SplineNotFound(self.spline_id))?
            .delete_point(self.point_id)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SetSplineHandlesCmd {
    pub spline_id: uuid::Uuid,
    pub point_id: uuid::Uuid,
    pub handle_in: [f64; 3],
    pub handle_out: [f64; 3],
    pub mode: SplineHandleMode,
}

impl Command for SetSplineHandlesCmd {
    fn label(&self) -> &'static str {
        "set spline handles"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let point = state
            .document
            .project
            .get_spline(self.spline_id)
            .ok_or("Spline not found")?
            .point(self.point_id)
            .ok_or("Spline point not found")?;
        let (handle_in, handle_out) =
            SplinePoint::resolved_handles(self.handle_in, self.handle_out, self.mode)
                .map_err(|_| "Spline handle data is not finite")?;
        if point.handle_in == handle_in
            && point.handle_out == handle_out
            && point.handle_mode == self.mode
        {
            Err("Spline handles are unchanged")
        } else {
            validate_profile_edit(&state.document.project, self.spline_id, |spline| {
                spline.set_handles(self.point_id, self.handle_in, self.handle_out, self.mode)
            })
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
            .project
            .get_spline_mut(self.spline_id)
            .ok_or(SplineError::SplineNotFound(self.spline_id))?
            .set_handles(self.point_id, self.handle_in, self.handle_out, self.mode)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SetSplineClosedCmd {
    pub spline_id: uuid::Uuid,
    pub closed: bool,
}

impl Command for SetSplineClosedCmd {
    fn label(&self) -> &'static str {
        "set spline closed"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let spline = state
            .document
            .project
            .get_spline(self.spline_id)
            .ok_or("Spline not found")?;
        if spline.closed == self.closed {
            Err("Spline closed state is unchanged")
        } else if self.closed && spline.points.len() < 3 {
            Err("A closed spline needs at least three points")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state
            .document
            .project
            .get_spline_mut(self.spline_id)
            .ok_or(SplineError::SplineNotFound(self.spline_id))?
            .set_closed(self.closed)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReverseSplineCmd {
    pub spline_id: uuid::Uuid,
}

impl Command for ReverseSplineCmd {
    fn label(&self) -> &'static str {
        "reverse spline"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let spline = state
            .document
            .project
            .get_spline(self.spline_id)
            .ok_or("Spline not found")?;
        if spline.points.len() < 2 {
            Err("Spline needs at least two points")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let spline = state
            .document
            .project
            .get_spline_mut(self.spline_id)
            .ok_or(SplineError::SplineNotFound(self.spline_id))?;
        if spline.points.len() < 2 {
            return Err(SplineError::NeedsTwoPoints.into());
        }
        spline.reverse();
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConvertSplineToPolylineCmd {
    pub spline_id: uuid::Uuid,
    pub spacing: f64,
    pub tolerance: f64,
}

impl Command for ConvertSplineToPolylineCmd {
    fn label(&self) -> &'static str {
        "convert spline to polyline"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let spline = state
            .document
            .project
            .resolved_spline(self.spline_id)
            .map_err(|_| "Spline or one of its surface attachments is invalid")?;
        if !self.spacing.is_finite() || self.spacing <= 0.0 {
            Err("Spline sample spacing is invalid")
        } else if !self.tolerance.is_finite() || self.tolerance <= 0.0 {
            Err("Spline sampling tolerance is invalid")
        } else if spline.segment_count() == 0 {
            Err("Spline needs at least two points")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mut spline = state.document.project.resolved_spline(self.spline_id)?;
        spline.convert_to_polyline(self.spacing, self.tolerance)?;
        *state
            .document
            .project
            .get_spline_mut(self.spline_id)
            .ok_or(SplineError::SplineNotFound(self.spline_id))? = spline;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AttachSplinePointCmd {
    pub spline_id: uuid::Uuid,
    pub point_id: uuid::Uuid,
    pub attachment: SurfaceAttachment,
}

impl Command for AttachSplinePointCmd {
    fn label(&self) -> &'static str {
        "attach spline point"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let point = spline_point_for_attachment(state, self.spline_id, self.point_id)?;
        if spline_owned_by_profile(&state.document.project, self.spline_id) {
            return Err("Profile splines cannot be attached to a surface");
        }
        ensure_attachment_target_editable(state, &self.attachment)?;
        let frame = evaluate_surface_attachment(&state.document.project, &self.attachment)
            .map_err(|_| "Surface attachment cannot be evaluated")?;
        let mut attachment = self.attachment;
        attachment.last_world_position = frame.position;
        let position = frame.position.map(f64::from);
        if point.attachment == Some(attachment) && point.position == position {
            Err("Spline point attachment is unchanged")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let position =
            evaluate_surface_attachment(&state.document.project, &self.attachment)?.position;
        let mut attachment = self.attachment;
        attachment.last_world_position = position;
        set_spline_point_attachment(
            &mut state.document.project,
            self.spline_id,
            self.point_id,
            Some(attachment),
            position,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DetachSplinePointCmd {
    pub spline_id: uuid::Uuid,
    pub point_id: uuid::Uuid,
}

impl Command for DetachSplinePointCmd {
    fn label(&self) -> &'static str {
        "detach spline point"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        spline_point_attachment(state, self.spline_id, self.point_id).map(|_| ())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let attachment = spline_point_attachment_for_execute(
            &state.document.project,
            self.spline_id,
            self.point_id,
        )?;
        let position = detach_surface_attachment_keep_world(&state.document.project, &attachment)?;
        set_spline_point_attachment(
            &mut state.document.project,
            self.spline_id,
            self.point_id,
            None,
            position,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SlideSplinePointAttachmentCmd {
    pub spline_id: uuid::Uuid,
    pub point_id: uuid::Uuid,
    pub ray_origin: [f32; 3],
    pub ray_direction: [f32; 3],
    pub max_distance: f32,
}

impl Command for SlideSplinePointAttachmentCmd {
    fn label(&self) -> &'static str {
        "slide spline point attachment"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        ensure_attachment_ray(self.ray_origin, self.ray_direction, self.max_distance)?;
        let attachment = spline_point_attachment(state, self.spline_id, self.point_id)?;
        ensure_attachment_target_editable(state, &attachment)?;
        let projected = slide_surface_attachment(
            &state.document.project,
            &attachment,
            self.ray_origin,
            self.ray_direction,
            self.max_distance,
        )
        .map_err(|_| "Slide ray did not hit the attached surface")?;
        if projected == attachment {
            Err("Spline point attachment is unchanged")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let attachment = spline_point_attachment_for_execute(
            &state.document.project,
            self.spline_id,
            self.point_id,
        )?;
        let projected = slide_surface_attachment(
            &state.document.project,
            &attachment,
            self.ray_origin,
            self.ray_direction,
            self.max_distance,
        )?;
        let position = evaluate_surface_attachment(&state.document.project, &projected)?.position;
        set_spline_point_attachment(
            &mut state.document.project,
            self.spline_id,
            self.point_id,
            Some(projected),
            position,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReprojectSplinePointAttachmentCmd {
    pub spline_id: uuid::Uuid,
    pub point_id: uuid::Uuid,
    pub target_id: uuid::Uuid,
    pub ray_origin: [f32; 3],
    pub ray_direction: [f32; 3],
    pub max_distance: f32,
}

impl Command for ReprojectSplinePointAttachmentCmd {
    fn label(&self) -> &'static str {
        "reproject spline point attachment"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        ensure_attachment_ray(self.ray_origin, self.ray_direction, self.max_distance)?;
        let point = spline_point_for_attachment(state, self.spline_id, self.point_id)?;
        let attachment = point
            .attachment
            .ok_or("Spline point is not surface-attached")?;
        if !attachment.is_well_formed() {
            return Err("Surface attachment data is invalid");
        }
        ensure_target_editable(state, self.target_id)?;
        let projected = reproject_surface_attachment_to_target(
            &state.document.project,
            &attachment,
            self.target_id,
            self.ray_origin,
            self.ray_direction,
            self.max_distance,
        )
        .map_err(|_| "Reprojection ray did not hit the requested surface")?;
        let position = evaluate_surface_attachment(&state.document.project, &projected)
            .map_err(|_| "Reprojected attachment cannot be evaluated")?
            .position
            .map(f64::from);
        if projected == attachment && position == point.position {
            Err("Spline point attachment is unchanged")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let attachment = spline_point_attachment_for_execute(
            &state.document.project,
            self.spline_id,
            self.point_id,
        )?;
        let projected = reproject_surface_attachment_to_target(
            &state.document.project,
            &attachment,
            self.target_id,
            self.ray_origin,
            self.ray_direction,
            self.max_distance,
        )?;
        let position = evaluate_surface_attachment(&state.document.project, &projected)?.position;
        set_spline_point_attachment(
            &mut state.document.project,
            self.spline_id,
            self.point_id,
            Some(projected),
            position,
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RotateSplinePointAttachmentCmd {
    pub spline_id: uuid::Uuid,
    pub point_id: uuid::Uuid,
    pub delta_radians: f32,
}

impl Command for RotateSplinePointAttachmentCmd {
    fn label(&self) -> &'static str {
        "rotate spline point attachment"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if !self.delta_radians.is_finite() {
            return Err("Attachment rotation is not finite");
        }
        let attachment = spline_point_attachment(state, self.spline_id, self.point_id)?;
        ensure_attachment_target_editable(state, &attachment)?;
        let rotated = attachment
            .rotated(self.delta_radians)
            .map_err(|_| "Attachment rotation is invalid")?;
        if rotated == attachment {
            return Err("Attachment rotation is unchanged");
        }
        evaluate_surface_attachment(&state.document.project, &rotated)
            .map(|_| ())
            .map_err(|_| "Rotated attachment cannot be evaluated")
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let attachment = spline_point_attachment_for_execute(
            &state.document.project,
            self.spline_id,
            self.point_id,
        )?;
        let rotated = attachment.rotated(self.delta_radians)?;
        let position = evaluate_surface_attachment(&state.document.project, &rotated)?.position;
        set_spline_point_attachment(
            &mut state.document.project,
            self.spline_id,
            self.point_id,
            Some(rotated),
            position,
        )?;
        Ok(())
    }
}

fn spline_point_for_attachment<'a>(
    state: &EditorView<'a>,
    spline_id: uuid::Uuid,
    point_id: uuid::Uuid,
) -> Result<&'a SplinePoint, &'static str> {
    state
        .document
        .project
        .get_spline(spline_id)
        .ok_or("Spline not found")?
        .point(point_id)
        .ok_or("Spline point not found")
}

fn spline_point_attachment(
    state: &EditorView<'_>,
    spline_id: uuid::Uuid,
    point_id: uuid::Uuid,
) -> Result<SurfaceAttachment, &'static str> {
    spline_point_for_attachment(state, spline_id, point_id)?
        .attachment
        .ok_or("Spline point is not surface-attached")
}

fn spline_point_attachment_for_execute(
    project: &petunia_project::Project,
    spline_id: uuid::Uuid,
    point_id: uuid::Uuid,
) -> Result<SurfaceAttachment, SplineError> {
    project
        .get_spline(spline_id)
        .ok_or(SplineError::SplineNotFound(spline_id))?
        .point(point_id)
        .ok_or(SplineError::PointNotFound(point_id))?
        .attachment
        .ok_or(SplineError::PointNotAttached(point_id))
}

fn set_spline_point_attachment(
    project: &mut petunia_project::Project,
    spline_id: uuid::Uuid,
    point_id: uuid::Uuid,
    attachment: Option<SurfaceAttachment>,
    position: [f32; 3],
) -> Result<(), SplineError> {
    project
        .get_spline_mut(spline_id)
        .ok_or(SplineError::SplineNotFound(spline_id))?
        .set_attachment(point_id, attachment, position.map(f64::from))
}

fn ensure_attachment_target_editable(
    state: &EditorView<'_>,
    attachment: &SurfaceAttachment,
) -> Result<(), &'static str> {
    match attachment.status(&state.document.project) {
        SurfaceAttachmentStatus::Valid => ensure_target_editable(state, attachment.target),
        SurfaceAttachmentStatus::NeedsReattach => Err("Surface attachment needs reattachment"),
        SurfaceAttachmentStatus::MissingTarget => Err("Surface attachment target is missing"),
        SurfaceAttachmentStatus::Invalid => Err("Surface attachment data is invalid"),
    }
}

fn ensure_target_editable(state: &EditorView<'_>, target: uuid::Uuid) -> Result<(), &'static str> {
    let asset = state
        .document
        .project
        .assets
        .iter()
        .find(|asset| asset.id == target)
        .ok_or("Surface attachment target is missing")?;
    if asset.locked {
        Err("Surface attachment target is locked")
    } else {
        Ok(())
    }
}

fn ensure_attachment_ray(
    origin: [f32; 3],
    direction: [f32; 3],
    max_distance: f32,
) -> Result<(), &'static str> {
    let direction_length_squared = direction
        .iter()
        .map(|component| component * component)
        .sum::<f32>();
    if origin
        .iter()
        .chain(&direction)
        .any(|component| !component.is_finite())
        || direction_length_squared <= 1.0e-16
        || !max_distance.is_finite()
        || max_distance <= 0.0
    {
        Err("Surface attachment ray is invalid")
    } else {
        Ok(())
    }
}

impl Command for BakeDecalCmd {
    fn label(&self) -> &'static str {
        "bake decal to raster"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::TEXTURES
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let Some(asset) = state.document.active() else {
            return Err("No active asset");
        };
        let Some(stack) = asset.paint_stack.as_ref() else {
            return Err("No paint stack");
        };
        if stack.layers.iter().any(|l| {
            l.id == self.layer_id
                && matches!(l.kind, petunia_project::paint_layers::LayerKind::Decal(_))
        }) {
            Ok(())
        } else {
            Err("Decal layer not found")
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let active = state.document.active;
        let mut baked = false;
        if let Some(asset) = state.document.assets.get_mut(active)
            && let Some(stack) = asset.paint_stack.as_mut()
        {
            // O raster assado tem o tamanho da textura do objeto (a camada
            // ativa é o próprio decalque, que não tem canvas).
            let (target_w, target_h) = asset
                .texture
                .as_ref()
                .map(|c| (c.w, c.h))
                .or_else(|| {
                    stack
                        .layers
                        .iter()
                        .find_map(|l| l.canvas())
                        .map(|c| (c.w, c.h))
                })
                .unwrap_or((512, 512));
            if stack.bake_decal_to_raster_on(self.layer_id, target_w, target_h, Some(&*asset.mesh))
            {
                baked = true;
            }
        }
        if !baked {
            return Err(CommandError::Execution("Failed to bake decal".into()));
        }
        state.document.project.composite_paint_stack(active);
        state.mark_canvas_dirty();
        state.mark_dirty();
        state.set_status("Decalque convertido para camada raster".to_string());
        Ok(())
    }
}

/// Espelha a malha ativa ao longo de um eixo com soldagem opcional no plano.
#[derive(Debug, Clone)]
pub struct MirrorMeshCmd {
    pub axis: usize,
    pub weld: f32,
}

impl Default for MirrorMeshCmd {
    fn default() -> Self {
        Self {
            axis: 0,
            weld: 0.001,
        }
    }
}

impl Command for MirrorMeshCmd {
    fn label(&self) -> &'static str {
        "mirror"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else if self.axis > 2 || !self.weld.is_finite() {
            Err("Invalid mirror parameters")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.mirror(self.axis, self.weld);
        state.sync_selection();
        Ok(())
    }
}

/// Aplica deslocamento Push/Pull imediato nas faces selecionadas.
#[derive(Debug, Clone, Default)]
pub struct PushPullApplyCmd {
    pub dist: f32,
}

impl Command for PushPullApplyCmd {
    fn label(&self) -> &'static str {
        "push/pull"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else if !self.dist.is_finite() {
            Err("Push/pull distance must be finite")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let dist = if self.dist != 0.0 {
            self.dist
        } else {
            state.tools.push_dist
        };
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !state.editor.selection.faces.is_empty() {
            state.editor.selection.apply_to_mesh(mesh);
        }
        mesh.push_pull(dist);
        state.sync_selection();
        Ok(())
    }
}

/// Fatia a malha ativa ao longo de um plano passando pelo centro da seleção.
#[derive(Debug, Clone)]
pub struct SlicePlaneCmd {
    pub normal: [f32; 3],
    pub cap: bool,
}

impl Default for SlicePlaneCmd {
    fn default() -> Self {
        Self {
            normal: [0.0, 1.0, 0.0],
            cap: true,
        }
    }
}

impl Command for SlicePlaneCmd {
    fn label(&self) -> &'static str {
        "slice"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        let n = glam::Vec3::from_array(self.normal);
        if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else if !n.is_finite() || n.length_squared() <= 1.0e-12 {
            Err("Slice plane normal is invalid")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let center = glam::Vec3::from(mesh.selection_center());
        let normal = glam::Vec3::from_array(self.normal);
        mesh.slice_plane(center, normal, self.cap);
        state.sync_selection();
        Ok(())
    }
}

/// Triangula as faces da malha ativa.
#[derive(Debug, Clone, Copy, Default)]
pub struct TriangulateMeshCmd;

impl Command for TriangulateMeshCmd {
    fn label(&self) -> &'static str {
        "triangulate"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.triangulate();
        state.sync_selection();
        Ok(())
    }
}

/// Translada os elementos selecionados da malha ativa por `delta`.
#[derive(Debug, Clone, Default)]
pub struct TranslateSelectionCmd {
    pub delta: [f32; 3],
}

impl Command for TranslateSelectionCmd {
    fn label(&self) -> &'static str {
        "move"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::POSITIONS
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.document.active_mesh().is_none() {
            Err("No active mesh")
        } else if !glam::Vec3::from_array(self.delta).is_finite() {
            Err("Move delta must be finite")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let delta = self.delta;
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !state.editor.selection.is_empty() {
            state.editor.selection.apply_to_mesh(mesh);
        }
        mesh.translate_selected(delta);
        state.tools.transform_delta = [0.0; 3];
        state.sync_selection();
        state.set_status(format!("move {delta:?}"));
        Ok(())
    }
}

/// Projeção UV planar sobre a malha ativa.
#[derive(Debug, Clone, Copy, Default)]
pub struct UvProjectPlanarCmd;

impl Command for UvProjectPlanarCmd {
    fn label(&self) -> &'static str {
        "uv planar"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.project_planar();
        state.editor.tools.uv_selection.clear();
        Ok(())
    }
}

/// Projeção UV cúbica (Box Mapping) sobre a malha ativa.
#[derive(Debug, Clone, Copy, Default)]
pub struct UvProjectCubeCmd;

impl Command for UvProjectCubeCmd {
    fn label(&self) -> &'static str {
        "uv cube projection"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.project_cube();
        state.editor.tools.uv_selection.clear();
        Ok(())
    }
}

/// Projeção UV cilíndrica sobre as faces selecionadas (ou toda a malha se vazio).
#[derive(Debug, Clone, Copy, Default)]
pub struct UvProjectCylindricalCmd;

impl Command for UvProjectCylindricalCmd {
    fn label(&self) -> &'static str {
        "uv cylindrical projection"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let faces = {
            let Some(mesh) = state.document.active_mesh() else {
                return Err(CommandError::NoActiveAsset);
            };
            state.editor.uv_selection.faces(mesh)
        };
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.project_cylindrical(&faces);
        state.editor.tools.uv_selection.clear();
        Ok(())
    }
}

/// Projeção UV esférica sobre as faces selecionadas (ou toda a malha se vazio).
#[derive(Debug, Clone, Copy, Default)]
pub struct UvProjectSphericalCmd;

impl Command for UvProjectSphericalCmd {
    fn label(&self) -> &'static str {
        "uv spherical projection"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let faces = {
            let Some(mesh) = state.document.active_mesh() else {
                return Err(CommandError::NoActiveAsset);
            };
            state.editor.uv_selection.faces(mesh)
        };
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.project_spherical(&faces);
        state.editor.tools.uv_selection.clear();
        Ok(())
    }
}

/// Equaliza a densidade de texels entre as ilhas UV.
#[derive(Debug, Clone, Copy, Default)]
pub struct UvEqualizeTexelDensityCmd;

impl Command for UvEqualizeTexelDensityCmd {
    fn label(&self) -> &'static str {
        "equalize texel density"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let faces = match state.document.active_mesh() {
            Some(mesh) => state.editor.tools.uv_selection.faces(mesh),
            None => return Err(CommandError::NoActiveAsset),
        };
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let modified = mesh.equalize_texel_density(&faces);
        state.set_status(format!("Equalized texel density on {modified} island(s)"));
        Ok(())
    }
}

/// Normaliza a densidade de texels para um alvo específico dada a largura da textura.
#[derive(Debug, Clone)]
pub struct UvNormalizeTexelDensityCmd {
    pub texture_w: u32,
    pub target: f32,
}

impl Default for UvNormalizeTexelDensityCmd {
    fn default() -> Self {
        Self {
            texture_w: 1024,
            target: 128.0,
        }
    }
}

impl Command for UvNormalizeTexelDensityCmd {
    fn label(&self) -> &'static str {
        "normalize texel density"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn can_execute(&self, _state: &EditorView<'_>) -> Result<(), &'static str> {
        if self.texture_w == 0 || !self.target.is_finite() || self.target <= 0.0 {
            Err("Invalid texel density parameters")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(mesh) = state.document.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.normalize_texel_density(self.texture_w, self.target);
        Ok(())
    }
}

/// Mescla a malha de `source_index` no asset ativo e remove o asset de origem.
#[derive(Debug, Clone)]
pub struct JoinAssetIntoActiveCmd {
    pub source_index: usize,
}

impl Command for JoinAssetIntoActiveCmd {
    fn label(&self) -> &'static str {
        "join asset"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if self.source_index == state.document.active
            || self.source_index >= state.document.assets.len()
        {
            Err("Invalid source asset index for join")
        } else if state.document.active_mesh().is_none() {
            Err("No active asset")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if self.source_index == state.document.active
            || self.source_index >= state.document.assets.len()
        {
            return Err(CommandError::InvalidAssetIndex(self.source_index));
        }
        let source_mesh = state.document.assets[self.source_index].mesh.clone();
        let Some(active_asset) = state.document.active_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        active_asset.mesh.join(&source_mesh);
        state.document.remove(self.source_index);
        state.sync_selection();
        state.set_status("asset joined into active");
        Ok(())
    }
}
