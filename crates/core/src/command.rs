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
    #[error(transparent)]
    Spline(#[from] SplineError),
    #[error(transparent)]
    SurfaceAttachment(#[from] SurfaceAttachmentError),
    #[error(transparent)]
    Profile(#[from] ProfileError),
    #[error(transparent)]
    PathGenerator(#[from] PathGeneratorError),
}

/// Trait central de comando executável contra o estado do editor (`AppState`).
pub trait Command: Send + Sync {
    /// Rótulo legível para telemetria e pilha de histórico (Undo/Redo).
    fn label(&self) -> &'static str;

    /// Executa o comando contra o estado do editor.
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError>;

    /// Indica se a operação altera dados do projeto e exige gravação prévia de checkpoint no UndoStack.
    fn is_destructive(&self) -> bool {
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
    fn can_execute(&self, _state: &AppState) -> Result<(), &'static str> {
        Ok(())
    }
}

impl<T: ?Sized + Command> Command for Box<T> {
    fn label(&self) -> &'static str {
        (**self).label()
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        (**self).execute(state)
    }

    fn is_destructive(&self) -> bool {
        (**self).is_destructive()
    }

    fn changes(&self) -> ProjectChanges {
        (**self).changes()
    }

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
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
        cmd.can_execute(state)
    }

    pub fn execute(&self, id: &str, state: &mut AppState) -> Result<(), CommandError> {
        let cmd = self
            .registry
            .get(id)
            .ok_or_else(|| CommandError::Execution(format!("Comando '{id}' não registrado")))?
            .clone();
        cmd.can_execute(state)
            .map_err(|reason| CommandError::Execution(reason.to_string()))?;
        Self::dispatch(state, cmd.as_ref())
    }

    pub fn dispatch(state: &mut AppState, cmd: &dyn Command) -> Result<(), CommandError> {
        cmd.can_execute(state)
            .map_err(|reason| CommandError::Execution(reason.into()))?;
        if cmd.is_destructive()
            && (state.modal.is_some()
                || state.mesh_preview.is_some()
                || state.paint_stroke.is_some())
        {
            return Err(CommandError::Execution(
                "Confirm or cancel the active operation first".into(),
            ));
        }
        let changes = cmd.changes();
        state.project.project.history_selection = state.session.selection.assets.clone();
        let original_selection = state.session.selection.clone();
        let original = cmd.is_destructive().then(|| state.project.project.clone());
        // A sessão de criação de primitiva é consumida ao congelar; guardá-la
        // permite restaurar o estado de sessão junto com o projeto se falhar.
        let original_primitive_session = (
            state.session.primitive_session.clone(),
            state.session.last_primitive,
        );
        if cmd.is_destructive() {
            state.freeze_active_primitive_for_command();
        }
        if let Err(error) = cmd.execute(state) {
            if let Some(original) = original {
                state.project.project = original;
                state.session.selection = original_selection;
                (
                    state.session.primitive_session,
                    state.session.last_primitive,
                ) = original_primitive_session;
                state.sync_selection();
            }
            return Err(error);
        }

        if let Some(original) = original {
            let bytes = original.estimated_bytes();
            state.project.undo.push_sized(cmd.label(), original, bytes);
            state.mark_document_dirty();
        }
        state.sync_selection();
        state.emit_project_changed(changes);
        state.mark_dirty();
        Ok(())
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
                "model.fuse",
                "Fuse",
                "Combine the active object with the boolean operand into one",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            BooleanOpCmd::new(petunia_mesh::boolean::BooleanOp::Union),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.cut",
                "Cut",
                "Subtract the boolean operand from the active object",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            BooleanOpCmd::new(petunia_mesh::boolean::BooleanOp::Difference),
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.join",
                "Join",
                "Merge the operand into the active object keeping both topologies",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            JoinObjectsCmd,
        );
        d.register_with_meta(
            CommandMetadata::new(
                "model.intersect",
                "Intersect",
                "Keep only the volume shared with the boolean operand",
                CommandCategory::Model,
            )
            .with_docs(DocsTopic::Modeling),
            BooleanOpCmd::new(petunia_mesh::boolean::BooleanOp::Intersection),
        );
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
                "Smooth and relax UV coordinates to reduce stretching",
                CommandCategory::Tools,
            )
            .with_docs(DocsTopic::UvUnwrapping),
            UvRelaxCmd::default(),
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let idx = self.asset_index.unwrap_or(state.project.active);
        if state.project.assets.get(idx).is_none() {
            Err("No active asset to duplicate")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let idx = self.asset_index.unwrap_or(state.project.active);
        let Some(asset) = state.project.assets.get(idx) else {
            return Err(CommandError::InvalidAssetIndex(idx));
        };

        let copy = asset.duplicate();
        let copy_name = copy.name.clone();
        state.project.assets.push(copy);
        state.project.active = state.project.assets.len() - 1;
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let len = state.project.assets.len();
        if self.from >= len || self.to >= len || self.from == self.to {
            Err("Invalid reorder indices")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if state.project.reorder_asset(self.from, self.to) {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.assets.iter().any(|a| a.id == self.asset_id) {
            Ok(())
        } else {
            Err("Asset not found in project library")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(asset) = state
            .project
            .assets
            .iter()
            .find(|a| a.id == self.asset_id)
            .cloned()
        else {
            return Err(CommandError::Execution("Asset not found".to_string()));
        };

        let mut instance = asset.duplicate();
        let target_pos = self.position.unwrap_or(state.session.cursor_3d);
        for v in &mut instance.mesh.verts {
            v.pos[0] += target_pos[0];
            v.pos[1] += target_pos[1];
            v.pos[2] += target_pos[2];
        }
        let name = instance.name.clone();
        state.project.assets.push(instance);
        state.project.active = state.project.assets.len() - 1;
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let idx = self.asset_index.unwrap_or(state.project.active);
        if idx >= state.project.assets.len() {
            Err("No active asset to delete")
        } else if state.project.assets[idx].locked {
            Err("Object is locked")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let idx = self.asset_index.unwrap_or(state.project.active);
        if idx >= state.project.assets.len() {
            return Err(CommandError::InvalidAssetIndex(idx));
        }

        state.project.remove(idx);
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.edit_mode() == EditMode::Object {
            if state.project.active().is_none() {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if state.edit_mode() == EditMode::Object {
            let selected = state.session.selection.assets.clone();
            if selected.is_empty() {
                return DeleteAssetCmd { asset_index: None }.execute(state);
            }
            state
                .project
                .assets
                .retain(|asset| !selected.contains(&asset.id) || asset.locked);
            state.project.active = usize::MAX;
            state.session.selection.assets.clear();
            state.set_status("Deleted selected objects");
            return Ok(());
        }

        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.delete_selected();
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.edit_mode() == EditMode::Object {
            if state.project.active().is_none() {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if state.edit_mode() == EditMode::Object {
            let selected = &state.session.selection.assets;
            let copies: Vec<_> = state
                .project
                .assets
                .iter()
                .filter(|asset| selected.contains(&asset.id))
                .map(|asset| asset.duplicate())
                .collect();
            if copies.is_empty() {
                return DuplicateAssetCmd { asset_index: None }.execute(state);
            }
            state.session.selection.assets = copies.iter().map(|asset| asset.id).collect();
            state.project.assets.extend(copies);
            state.project.active = state.project.assets.len() - 1;
            state.set_status("Duplicated selected objects");
            return Ok(());
        }

        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.duplicate_selected();
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.selection_domain() != crate::SelectionDomain::Object
            && state.project.active_mesh().is_none()
        {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if state.selection_domain() == crate::SelectionDomain::Object {
            state.session.selection.assets = state
                .project
                .assets
                .iter()
                .filter(|a| a.visible && !a.locked)
                .map(|a| a.id)
                .collect();
            state.project.active = state
                .session
                .selection
                .assets
                .last()
                .and_then(|id| state.project.assets.iter().position(|a| a.id == *id))
                .unwrap_or(usize::MAX);
            return Ok(());
        }
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.selection_domain() != crate::SelectionDomain::Object
            && state.project.active_mesh().is_none()
        {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if state.selection_domain() == crate::SelectionDomain::Object {
            state.select_object(None, false);
            return Ok(());
        }
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.selection_domain() != crate::SelectionDomain::Object
            && state.project.active_mesh().is_none()
        {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if state.selection_domain() == crate::SelectionDomain::Object {
            let selected = &state.session.selection.assets;
            let ids: Vec<_> = state
                .project
                .assets
                .iter()
                .filter(|a| a.visible && !a.locked && !selected.contains(&a.id))
                .map(|a| a.id)
                .collect();
            state.project.active = ids
                .last()
                .and_then(|id| state.project.assets.iter().position(|a| a.id == *id))
                .unwrap_or(usize::MAX);
            state.session.selection.assets = ids;
            return Ok(());
        }
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let Some(mesh) = state.project.active_mesh() else {
            return Err("No active mesh");
        };
        if !mesh.has_selection() {
            return Err("Select geometry to separate first");
        }
        Ok(())
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(active_asset) = state.project.active() else {
            return Err(CommandError::NoActiveAsset);
        };
        let orig_name = active_asset.name.clone();
        let orig_mesh = active_asset.mesh.clone();

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
            if f.selected || f.verts.iter().all(|vi| vert_map.contains_key(vi)) {
                let new_verts: Vec<u32> = f.verts.iter().map(|vi| vert_map[vi]).collect();
                let mut nf = petunia_mesh::Face::with_uv(new_verts, f.uv.clone());
                nf.selected = false;
                sep_mesh.push_face(nf);
            }
        }

        if sep_mesh.verts.is_empty() || sep_mesh.faces.is_empty() {
            return Err(CommandError::Execution("No geometry separated".into()));
        }

        // 2. Remove os elementos selecionados da malha original
        if let Some(active_mesh) = state.project.active_mesh_mut() {
            active_mesh.delete_selected();
        }

        // 3. Adiciona a malha separada como novo objeto/asset na cena
        let new_name = format!("{}_sep", orig_name);
        state.project.add(&new_name, sep_mesh);
        state.set_status(format!("Separated selection into {}", new_name));
        Ok(())
    }
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.active_mesh().is_none() {
            Err("No active mesh")
        } else if state.selection.is_empty()
            && state
                .project
                .active_mesh()
                .is_none_or(|m| !m.faces.iter().any(|f| f.selected) && m.selected_edges.is_empty())
        {
            Err("Select geometry to subdivide")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let cuts = state.tools.subdivide_cuts.clamp(1, 6);
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.subdivide_selected_cuts(cuts);
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.selection.verts.len() < 2 {
            Err("Select at least 2 vertices")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.merge_center();
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.project.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let removed = mesh.weld_merged_count(self.eps.max(0.0));
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.project.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.flip_normals();
        state.set_status("Flipped normals");
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let asset = state.project.active().ok_or("No active mesh")?;
        if state.project.is_smooth_shaded(asset.id) == self.smooth {
            Err(if self.smooth {
                "Object is already smooth shaded"
            } else {
                "Object is already flat shaded"
            })
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let id = state
            .project
            .active()
            .map(|asset| asset.id)
            .ok_or(CommandError::NoActiveAsset)?;
        if !state.project.project.set_smooth_shaded(id, self.smooth) {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.selection.is_empty()
            && !state
                .project
                .active_mesh()
                .is_some_and(|m| m.has_selection())
        {
            Err("Select quad or edge first")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if !state.selection.faces.is_empty()
            || state
                .project
                .active_mesh()
                .is_some_and(|m| m.faces.iter().any(|f| f.selected))
        {
            Ok(())
        } else {
            Err("Select faces first")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.selection.is_empty()
            && !state
                .project
                .active_mesh()
                .is_some_and(|m| m.has_selection())
        {
            Err("Select at least one element first")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let idx = self.asset_index.unwrap_or(state.project.active);
        let Some(asset) = state.project.assets.get_mut(idx) else {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let idx = self.asset_index.unwrap_or(state.project.active);
        let Some(asset) = state.project.assets.get_mut(idx) else {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(asset) = state.project.assets.get_mut(self.asset_index) else {
            return Err(CommandError::InvalidAssetIndex(self.asset_index));
        };
        asset.collection = self.collection.clone();
        let name = asset.name.clone();
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let all_vis = state
            .project
            .assets
            .iter()
            .filter(|a| a.collection.as_deref() == Some(&self.collection))
            .all(|a| a.visible);
        for a in &mut state.project.assets {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let all_locked = state
            .project
            .assets
            .iter()
            .filter(|a| a.collection.as_deref() == Some(&self.collection))
            .all(|a| a.locked);
        for a in &mut state.project.assets {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        crate::project_service::ProjectService::new_project(state);
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if !state.is_document_dirty() && state.project.project_path.is_some() {
            Err("Document has no unsaved changes")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if let Some(ref path_str) = state.project.project_path.clone() {
            let path = std::path::PathBuf::from(path_str);
            crate::project_service::ProjectService::save_project(state, &path)
                .map_err(|e| CommandError::Execution(e.to_string()))?;
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.active().is_none() {
            Err("No active model selected")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.undo.can_undo() {
            Ok(())
        } else {
            Err("Nothing to undo")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.undo.can_redo() {
            Ok(())
        } else {
            Err("Nothing to redo")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.project.active_mesh().is_none() {
            Err("No active mesh")
        } else if !state.selection.faces.is_empty()
            || state
                .project
                .active_mesh()
                .is_some_and(|m| m.faces.iter().any(|f| f.selected))
        {
            Ok(())
        } else {
            Err("Select faces first")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let dist = if self.dist != 0.0 {
            self.dist
        } else {
            state.tools.extrude_dist
        };
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !mesh.faces.iter().any(|f| f.selected) {
            return Err(CommandError::EmptySelection);
        }
        mesh.extrude_selected(dist);
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.edit_mode() != EditMode::Edit {
            Err("Requires Edit mode")
        } else if state.project.active_mesh().is_none() {
            Err("No active mesh")
        } else if state
            .project
            .active_mesh()
            .is_some_and(|m| m.faces.iter().any(|f| f.selected))
        {
            Ok(())
        } else {
            Err("Select faces first")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh().cloned() else {
            return Err(CommandError::NoActiveAsset);
        };
        state.session.tools.cut_session = Some(crate::CutSession::new(mesh));
        state.session.tools.active_tool = "cut".to_string();
        state.set_status("Knife: pick two edge points, Enter confirms");
        Ok(())
    }
}

/// Operação booleana entre o ativo e o operando escolhido (Fuse/Cut/Intersect).
#[derive(Debug, Clone, Copy)]
pub struct BooleanOpCmd {
    pub op: petunia_mesh::boolean::BooleanOp,
}

impl BooleanOpCmd {
    pub const fn new(op: petunia_mesh::boolean::BooleanOp) -> Self {
        Self { op }
    }

    const fn label_for(self) -> &'static str {
        match self.op {
            petunia_mesh::boolean::BooleanOp::Union => "fuse",
            petunia_mesh::boolean::BooleanOp::Difference => "cut",
            petunia_mesh::boolean::BooleanOp::Intersection => "intersect",
        }
    }
}

impl Command for BooleanOpCmd {
    fn label(&self) -> &'static str {
        self.label_for()
    }

    /// Sem checkpoint do dispatcher: `apply_boolean` captura a transação única.
    /// Com os dois, cada operação empilharia duas entradas de undo.
    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.active_mesh().is_none() {
            return Err("No active mesh");
        }
        let Some(operand) = state.session.tools.boolean_operand else {
            return Err("Choose a boolean operand first");
        };
        if !state.project.assets.iter().any(|asset| asset.id == operand) {
            return Err("The boolean operand no longer exists");
        }
        if state
            .project
            .active()
            .is_some_and(|asset| asset.id == operand)
        {
            return Err("The boolean operand cannot be the active object");
        }
        Ok(())
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        match state.apply_boolean(self.op) {
            Ok(verts) => {
                let detail = match &state.session.tools.boolean_report {
                    Some(r) if r.fell_back => format!(
                        "{} triangles (cleanup refused: kernel result kept)",
                        r.faces
                    ),
                    Some(r) => format!(
                        "{} faces: {} quads, {} n-gons, {} triangles (was {} triangles)",
                        r.faces, r.quads, r.ngons, r.triangles, r.kernel_triangles
                    ),
                    None => format!("{verts} vertices"),
                };
                state.set_status(format!("{}: {detail}", self.label_for()));
                Ok(())
            }
            Err(error) => Err(CommandError::Execution(error.to_string())),
        }
    }
}

/// **Join**: funde o operando no ativo preservando as duas topologias.
#[derive(Debug, Clone, Copy, Default)]
pub struct JoinObjectsCmd;

impl Command for JoinObjectsCmd {
    fn label(&self) -> &'static str {
        "join"
    }

    /// Sem checkpoint do dispatcher: `join_active_with_operand` captura a única.
    fn is_destructive(&self) -> bool {
        false
    }

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.active_mesh().is_none() {
            return Err("No active mesh");
        }
        let Some(operand) = state.session.tools.boolean_operand else {
            return Err("Choose a boolean operand first");
        };
        if !state.project.assets.iter().any(|asset| asset.id == operand) {
            return Err("The boolean operand no longer exists");
        }
        if state
            .project
            .active()
            .is_some_and(|asset| asset.id == operand)
        {
            return Err("The boolean operand cannot be the active object");
        }
        Ok(())
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        match state.join_active_with_operand() {
            Ok(verts) => {
                state.set_status(format!("Join: {verts} vertices merged"));
                Ok(())
            }
            Err(reason) => Err(CommandError::Execution(reason.to_string())),
        }
    }
}

/// Comando para aplicação de inset nas faces selecionadas.
#[derive(Debug, Clone)]
pub struct InsetFacesCmd {
    pub factor: f32,
}

impl Default for InsetFacesCmd {
    fn default() -> Self {
        Self { factor: 0.2 }
    }
}

impl Command for InsetFacesCmd {
    fn label(&self) -> &'static str {
        "inset"
    }

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.active_mesh().is_none() {
            Err("No active mesh")
        } else if !state.selection.faces.is_empty()
            || state
                .project
                .active_mesh()
                .is_some_and(|m| m.faces.iter().any(|f| f.selected))
        {
            Ok(())
        } else {
            Err("Select faces first")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let factor = if self.factor != 0.0 {
            self.factor
        } else {
            state.tools.inset_factor
        };
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        if !mesh.faces.iter().any(|f| f.selected) {
            return Err(CommandError::EmptySelection);
        }
        mesh.inset_selected(factor);
        state.set_status(format!("Inset faces ({:.2})", factor));
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if let Some(mesh) = state.project.active_mesh() {
            let has_edges = !mesh.selected_edges.is_empty();
            let has_verts = mesh.verts.iter().any(|v| v.selected);
            if !has_edges && !has_verts {
                Err("Select edges or points first")
            } else {
                Ok(())
            }
        } else {
            Err("No active mesh")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let amount = if self.amount != 0.0 {
            self.amount
        } else {
            state.tools.bevel_amount
        };
        let segments = if self.segments == 0 {
            state.tools.bevel_segments.clamp(1, 4)
        } else {
            self.segments.clamp(1, 4)
        };
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let has_edges = !mesh.selected_edges.is_empty();
        let has_verts = mesh.verts.iter().any(|v| v.selected);
        if !has_edges && !has_verts {
            return Err(CommandError::EmptySelection);
        }
        let (v_count, f_count) = if self.affect_vertices && has_verts {
            mesh.bevel_selected_vertex(amount, self.clamp_overlap)
        } else {
            mesh.bevel_selected_full(amount, segments, self.clamp_overlap)
        };
        state.set_status(format!("Beveled (+{} verts, +{} faces)", v_count, f_count));
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.active_mesh().is_none() {
            Err("No active mesh")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let factor = if self.factor == 0.0 {
            state.tools.transform_scale
        } else {
            self.factor
        };
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let center = mesh.selection_center();
        mesh.scale_selected(factor, center);
        state.tools.transform_scale = 1.0;
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.session.show_face_orientation = !state.session.show_face_orientation;
        state.set_status(if state.session.show_face_orientation {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.session.show_uv_checker = !state.session.show_uv_checker;
        state.set_status(if state.session.show_uv_checker {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.session.proportional_editing = !state.session.proportional_editing;
        state.session.proportional_settings.enabled = state.session.proportional_editing;
        state.set_status(if state.session.proportional_editing {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.session.snap_enabled = !state.session.snap_enabled;
        state.set_status(if state.session.snap_enabled {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.session.show_nav_hud = !state.session.show_nav_hud;
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
            ViewPreset::Persp => "view perspective",
        }
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.ui.show_reference_manager = !state.ui.show_reference_manager;
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.ui.show_command_palette = !state.ui.show_command_palette;
        if state.ui.show_command_palette {
            state.ui.command_palette_query.clear();
            state.ui.command_palette_selected_index = 0;
        }
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.ui.show_settings = !state.ui.show_settings;
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.ui.show_help = !state.ui.show_help;
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let mesh = state.project.active_mesh().ok_or("No active mesh")?;
        if mesh.selected_edges.is_empty() {
            Err("Select an edge on a quad ring")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let cuts = self.cuts.clamp(1, 32) as usize;
        let Some(mesh) = state.project.active_mesh() else {
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
        if let Some(dst) = state.project.active_mesh_mut() {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let charts = mesh
            .unwrap_auto()
            .map_err(|e| CommandError::Execution(e.to_string()))?;
        state.set_status(format!("Auto UV ({charts} charts)"));
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct UvPackIslandsCmd {
    pub padding: f32,
}

impl Default for UvPackIslandsCmd {
    fn default() -> Self {
        Self { padding: 0.01 }
    }
}

impl Command for UvPackIslandsCmd {
    fn label(&self) -> &'static str {
        "pack uv"
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::UVS
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let n = mesh.pack_uv_islands(self.padding);
        state.set_status(format!("Packed {n} UV islands"));
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let origin = state.camera.eye();
        let forward = state.camera.forward();
        let up = state.camera.up();
        let right = forward.cross(up).normalize_or_zero();
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.project_from_view(right, up, origin);
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let reference = state.project.refs.iter().find(|r| r.visible).cloned();
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

        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.project_from_view(right, up, origin);
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let count = mesh.stitch_uv(&state.session.uv_selected);
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        let count = mesh.relax_uv(&state.session.uv_selected, self.iterations);
        if count == 0 {
            return Err(CommandError::NoChange("no UV face to relax".into()));
        }
        state.set_status(format!("Relaxed {count} UV face(s)"));
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(ref_img) = state.project.refs.iter().find(|r| r.visible).cloned() else {
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

        let Some(mesh) = state.project.active_mesh_mut() else {
            return Err(CommandError::NoActiveAsset);
        };
        mesh.project_from_view(right, up, origin);

        let active_idx = state.project.active;
        if let Some(asset) = state.project.assets.get_mut(active_idx) {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let mesh = state.project.active_mesh().ok_or("No active mesh")?;
        let n = mesh.faces.iter().filter(|f| f.selected).count();
        if n == 2 || !mesh.selected_edges.is_empty() {
            Ok(())
        } else {
            Err("Select two faces or two boundary edge loops")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if state.selection_domain() == crate::SelectionDomain::Edge {
            let mesh = state
                .project
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
            .project
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
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let mesh = state.project.active_mesh().ok_or("No active mesh")?;
        let has_sel = !mesh.selected_edges.is_empty()
            || mesh.faces.iter().any(|f| f.selected)
            || mesh.verts.iter().any(|v| v.selected);
        if has_sel {
            Ok(())
        } else {
            Err("Select edges, faces or vertices to dissolve")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let mesh = state.project.active_mesh().ok_or("No active mesh")?;
        if !mesh.selected_edges.is_empty()
            || mesh.faces.iter().any(|face| face.selected)
            || mesh.verts.iter().any(|vertex| vertex.selected)
        {
            Ok(())
        } else {
            Err("Select edges, faces, or points to delete")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let mesh = state.project.active_mesh().ok_or("No active mesh")?;
        let vert_count = mesh.verts.iter().filter(|v| v.selected).count();
        let edge_count = mesh.selected_edges.len();
        if edge_count >= 2 || vert_count >= 2 {
            Ok(())
        } else {
            Err("Select at least 3 edges or vertices to make a face (or 2 to make an edge)")
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let Some(mesh) = state.project.active_mesh_mut() else {
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let Some(asset) = state.project.active() else {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let active = state.project.active;
        let changed = if let Some(asset) = state.project.assets.get_mut(active)
            && let Some(stack) = asset.paint_stack.as_mut()
            && stack.set_decal_transform(
                self.layer_id,
                self.center_uv,
                self.scale_uv,
                self.rotation_rad,
            ) {
            true
        } else {
            false
        };
        if !changed {
            return Err(CommandError::Execution(
                "Failed to update decal transform".into(),
            ));
        }
        state.project.project.composite_paint_stack(active);
        state.render.canvas_dirty = true;
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.project.get_spline(self.spline.id).is_some() {
            Err("Spline id already exists")
        } else if self.spline.validate_authoring().is_err() {
            Err("Spline data is invalid")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.project.project.add_spline(self.spline.clone())?;
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let project = &state.project.project;
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let spline = state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let project = &state.project.project;
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.project.project.add_spline(self.spline.clone())?;
        state.project.project.add_profile(self.profile.clone())?;
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let project = &state.project.project;
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let profile = state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let project = &state.project.project;
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let profile = state.project.project.remove_profile(self.profile_id)?;
        state.project.project.remove_spline(profile.spline_id)?;
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let project = &state.project.project;
        if project.get_path_generator(self.generator.id).is_some() {
            return Err("Path generator id already exists");
        }
        project
            .validate_path_generator_candidate(&self.generator)
            .map_err(|_| "Path generator data or dependencies are invalid")
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        self.parameters
            .validate()
            .map_err(|_| "Sweep generator parameters are invalid")?;
        let generator = state
            .project
            .project
            .get_path_generator(self.generator_id)
            .ok_or("Path generator not found")?;
        if generator.sweep_parameters() == self.parameters {
            Err("Sweep generator parameters are unchanged")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        state
            .project
            .project
            .get_path_generator(self.generator_id)
            .map(|_| ())
            .ok_or("Path generator not found")
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        state
            .project
            .project
            .validate_path_generator(self.generator_id)
            .map_err(|_| "Path generator or one of its dependencies is invalid")
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let mut cache = PathGeneratorEvaluationCache::default();
        let evaluation = state.project.project.evaluate_path_generator(
            self.generator_id,
            PathGeneratorQuality::Final,
            &mut cache,
        )?;
        let mesh = evaluation.mesh.clone();
        let vertices = evaluation.diagnostics.vertices;
        let triangles = evaluation.diagnostics.triangles;
        let fallback_name = state
            .project
            .project
            .get_path_generator(self.generator_id)
            .map(|generator| generator.name.clone())
            .ok_or(PathGeneratorError::GeneratorNotFound(self.generator_id))?;
        let asset_name = if self.asset_name.trim().is_empty() {
            fallback_name
        } else {
            self.asset_name.trim().to_string()
        };
        state.project.project.add(&asset_name, mesh);
        state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let project = &state.project.project;
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state.project.project.remove_spline(self.spline_id)?;
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let spline = state
            .project
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
        validate_profile_edit(&state.project.project, self.spline_id, |spline| {
            spline.insert_point(index, self.point.clone())
        })
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let spline = state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let spline = state
            .project
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
            validate_profile_edit(&state.project.project, self.spline_id, |spline| {
                spline.move_point(self.point_id, self.position)
            })
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        state
            .project
            .project
            .get_spline(self.spline_id)
            .ok_or("Spline not found")?
            .point(self.point_id)
            .map(|_| ())
            .ok_or("Spline point not found")
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let point = state
            .project
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
            validate_profile_edit(&state.project.project, self.spline_id, |spline| {
                spline.set_handles(self.point_id, self.handle_in, self.handle_out, self.mode)
            })
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let spline = state
            .project
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let spline = state
            .project
            .project
            .get_spline(self.spline_id)
            .ok_or("Spline not found")?;
        if spline.points.len() < 2 {
            Err("Spline needs at least two points")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let spline = state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let spline = state
            .project
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let mut spline = state.project.project.resolved_spline(self.spline_id)?;
        spline.convert_to_polyline(self.spacing, self.tolerance)?;
        *state
            .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let point = spline_point_for_attachment(state, self.spline_id, self.point_id)?;
        if spline_owned_by_profile(&state.project.project, self.spline_id) {
            return Err("Profile splines cannot be attached to a surface");
        }
        ensure_attachment_target_editable(state, &self.attachment)?;
        let frame = evaluate_surface_attachment(&state.project.project, &self.attachment)
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let position =
            evaluate_surface_attachment(&state.project.project, &self.attachment)?.position;
        let mut attachment = self.attachment;
        attachment.last_world_position = position;
        set_spline_point_attachment(
            &mut state.project.project,
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        spline_point_attachment(state, self.spline_id, self.point_id).map(|_| ())
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let attachment = spline_point_attachment_for_execute(
            &state.project.project,
            self.spline_id,
            self.point_id,
        )?;
        let position = detach_surface_attachment_keep_world(&state.project.project, &attachment)?;
        set_spline_point_attachment(
            &mut state.project.project,
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        ensure_attachment_ray(self.ray_origin, self.ray_direction, self.max_distance)?;
        let attachment = spline_point_attachment(state, self.spline_id, self.point_id)?;
        ensure_attachment_target_editable(state, &attachment)?;
        let projected = slide_surface_attachment(
            &state.project.project,
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let attachment = spline_point_attachment_for_execute(
            &state.project.project,
            self.spline_id,
            self.point_id,
        )?;
        let projected = slide_surface_attachment(
            &state.project.project,
            &attachment,
            self.ray_origin,
            self.ray_direction,
            self.max_distance,
        )?;
        let position = evaluate_surface_attachment(&state.project.project, &projected)?.position;
        set_spline_point_attachment(
            &mut state.project.project,
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
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
            &state.project.project,
            &attachment,
            self.target_id,
            self.ray_origin,
            self.ray_direction,
            self.max_distance,
        )
        .map_err(|_| "Reprojection ray did not hit the requested surface")?;
        let position = evaluate_surface_attachment(&state.project.project, &projected)
            .map_err(|_| "Reprojected attachment cannot be evaluated")?
            .position
            .map(f64::from);
        if projected == attachment && position == point.position {
            Err("Spline point attachment is unchanged")
        } else {
            Ok(())
        }
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let attachment = spline_point_attachment_for_execute(
            &state.project.project,
            self.spline_id,
            self.point_id,
        )?;
        let projected = reproject_surface_attachment_to_target(
            &state.project.project,
            &attachment,
            self.target_id,
            self.ray_origin,
            self.ray_direction,
            self.max_distance,
        )?;
        let position = evaluate_surface_attachment(&state.project.project, &projected)?.position;
        set_spline_point_attachment(
            &mut state.project.project,
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
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
        evaluate_surface_attachment(&state.project.project, &rotated)
            .map(|_| ())
            .map_err(|_| "Rotated attachment cannot be evaluated")
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let attachment = spline_point_attachment_for_execute(
            &state.project.project,
            self.spline_id,
            self.point_id,
        )?;
        let rotated = attachment.rotated(self.delta_radians)?;
        let position = evaluate_surface_attachment(&state.project.project, &rotated)?.position;
        set_spline_point_attachment(
            &mut state.project.project,
            self.spline_id,
            self.point_id,
            Some(rotated),
            position,
        )?;
        Ok(())
    }
}

fn spline_point_for_attachment(
    state: &AppState,
    spline_id: uuid::Uuid,
    point_id: uuid::Uuid,
) -> Result<&SplinePoint, &'static str> {
    state
        .project
        .project
        .get_spline(spline_id)
        .ok_or("Spline not found")?
        .point(point_id)
        .ok_or("Spline point not found")
}

fn spline_point_attachment(
    state: &AppState,
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
    state: &AppState,
    attachment: &SurfaceAttachment,
) -> Result<(), &'static str> {
    match attachment.status(&state.project.project) {
        SurfaceAttachmentStatus::Valid => ensure_target_editable(state, attachment.target),
        SurfaceAttachmentStatus::NeedsReattach => Err("Surface attachment needs reattachment"),
        SurfaceAttachmentStatus::MissingTarget => Err("Surface attachment target is missing"),
        SurfaceAttachmentStatus::Invalid => Err("Surface attachment data is invalid"),
    }
}

fn ensure_target_editable(state: &AppState, target: uuid::Uuid) -> Result<(), &'static str> {
    let asset = state
        .project
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

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        let Some(asset) = state.project.active() else {
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

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        let active = state.project.active;
        let mut baked = false;
        if let Some(asset) = state.project.assets.get_mut(active)
            && let Some(stack) = asset.paint_stack.as_mut()
        {
            let (target_w, target_h) = stack
                .active()
                .and_then(|l| l.canvas())
                .map(|c| (c.w, c.h))
                .unwrap_or((512, 512));
            if stack.bake_decal_to_raster(self.layer_id, target_w, target_h) {
                baked = true;
            }
        }
        if !baked {
            return Err(CommandError::Execution("Failed to bake decal".into()));
        }
        state.project.project.composite_paint_stack(active);
        state.render.canvas_dirty = true;
        state.mark_dirty();
        state.set_status("Decalque convertido para camada raster".to_string());
        Ok(())
    }
}
