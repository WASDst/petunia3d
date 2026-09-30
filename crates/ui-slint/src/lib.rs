//! Frontend de produção do Petunia3D em Slint 1.18.
//!
//! O shell em Slint emite intenções (`UiIntent`); o domínio continua em
//! `petunia_core`, `petunia_commands` e `petunia_project`. A integração de GPU
//! é realizada pelo adapter `WgpuViewport` ou fallback `Software3dViewport`.
//!
//! A UI egui (`crates/ui/`) é legado de transição, acessível via
//! `--legacy-egui` / `PETUNIA_LEGACY_EGUI=1`.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub mod commands;
pub mod files;
mod input;
pub mod numeric;
pub mod overlay;
pub mod theme;
pub mod viewport_gpu;
pub mod viewport_soft;

pub use viewport_soft::Software3dViewport;

use commands::CommandId;
use numeric::NumericFieldState;
use overlay::{OverlayEntry, OverlayId, OverlayKind, OverlayStack};
use petunia_config::keybinds::Mods2;
use petunia_core::PivotPoint;
use petunia_core::PrimitiveDescriptorExt;
use petunia_core::{AppState, Camera, SelectionDomain, Workspace};
use petunia_project::{AlphaMode, Project, ProjectChanges, ShaderProfile};
use slint::ComponentHandle;

slint::include_modules!();

/// Tipo de transformação tridimensional manipulada no Inspector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformKind {
    Position,
    Rotation,
    Scale,
}

/// Gesto do cursor ou toque na viewport 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ViewportGesture {
    Orbit { dx: f32, dy: f32 },
    Pan { dx: f32, dy: f32 },
    Zoom { delta: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewportRenderState {
    pub shading: petunia_render::Shading,
    pub xray: bool,
    pub show_triangulation: bool,
    pub textured: bool,
    pub show_wireframe_overlay: bool,
    pub show_face_orientation: bool,
    pub show_uv_checker: bool,
    /// Domínio de seleção: a camada de seleção precisa saber o que desenhar.
    pub selection_domain: petunia_core::SelectionDomain,
    /// Opacidade da geometria em X-Ray.
    pub xray_opacity: f32,
    pub selection_rgb: [u8; 3],
    pub selection_thickness: f32,
    /// Overlays: grade e wireframe opcional sobre as faces.
    pub show_grid: bool,
    /// Componente sob o cursor (preselection).
    pub hover: petunia_core::HoverTarget,
    pub boolean_operand: Option<uuid::Uuid>,
    /// Luz de estúdio acompanha a câmera (preferência; padrão ligado).
    pub studio_light_follows_camera: bool,
    /// Aparência das arestas por modo (DRAW = forma, POLY = topologia).
    pub edge_mode: petunia_render_wgpu::EdgeMode,
}

impl Default for ViewportRenderState {
    fn default() -> Self {
        Self {
            shading: petunia_render::Shading::Solid,
            xray: false,
            show_triangulation: false,
            textured: false,
            show_wireframe_overlay: true,
            show_face_orientation: false,
            show_uv_checker: false,
            selection_domain: petunia_core::SelectionDomain::Object,
            xray_opacity: 0.42,
            selection_rgb: [233, 106, 0],
            selection_thickness: 2.0,
            show_grid: true,
            hover: petunia_core::HoverTarget::None,
            boolean_operand: None,
            studio_light_follows_camera: true,
            edge_mode: petunia_render_wgpu::EdgeMode::Overlay,
        }
    }
}

fn texture_points(points: &[[f32; 2]], width: u32, height: u32) -> Vec<(u32, u32)> {
    points
        .iter()
        .map(|point| {
            (
                (point[0].round() as i32).clamp(0, width.saturating_sub(1) as i32) as u32,
                (point[1].round() as i32).clamp(0, height.saturating_sub(1) as i32) as u32,
            )
        })
        .collect()
}

/// Gizmo 3D projetado para o overlay da viewport.
///
/// A projeção acontece no bridge; o Slint só desenha as três hastes a partir
/// de origem, comprimento e ângulo em pixels. O overlay não conhece câmera,
/// GPU nem matriz de projeção.
/// Gizmo de transformação e tripé de navegação, tudo em espaço de tela.
//
// O padrão profissional (Blender, C4D, Maya, Plasticity) usa tamanho fixo em
// pixels, nunca escalado pelo mundo: o controle tem sempre o mesmo tamanho
// aparente independente do zoom. Hastes têm setas; o tripé do canto mostra a
// orientação da câmera.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GizmoModel {
    pub visible: bool,
    pub origin_x: f32,
    pub origin_y: f32,
    /// Hastes como comandos `M x y L x y` prontos para o `Path`.
    pub x_commands: String,
    pub y_commands: String,
    pub z_commands: String,
    pub x_end: [f32; 2],
    pub y_end: [f32; 2],
    pub z_end: [f32; 2],
    pub x_label: [f32; 2],
    pub y_label: [f32; 2],
    pub z_label: [f32; 2],
    /// Setas como triângulos preenchidos (`M .. L .. L .. Z`).
    pub x_arrow_commands: String,
    pub y_arrow_commands: String,
    pub z_arrow_commands: String,
    /// Handles adicionais do gizmo combinado (escala e rotação por eixo).
    pub x_scale_commands: String,
    pub y_scale_commands: String,
    pub z_scale_commands: String,
    pub x_rotate_commands: String,
    pub y_rotate_commands: String,
    pub z_rotate_commands: String,
    /// Comandos dos planos (0: YZ normal X, 1: XZ normal Y, 2: XY normal Z).
    pub plane_yz_commands: String,
    pub plane_xz_commands: String,
    pub plane_xy_commands: String,
    /// Anel perimetral de rotação da visão (View Roll) para Rotate.
    pub view_roll_commands: String,
    /// Tripé de navegação no canto inferior esquerdo, em comandos prontos.
    pub view_x_commands: String,
    pub view_y_commands: String,
    pub view_z_commands: String,
    pub view_x_end: [f32; 2],
    pub view_y_end: [f32; 2],
    pub view_z_end: [f32; 2],
    pub view_origin_x: f32,
    pub view_origin_y: f32,
    /// Projeção em tela do 3D Cursor para o overlay da viewport.
    pub cursor_screen: [f32; 2],
    pub cursor_visible: bool,
}

/// Ferramenta paramétrica com preview modal e Tool Properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolModalKind {
    Extrude,
    ExtrudeIndividual,
    Inset,
    Bevel,
    PushPull,
    ScaleSelection,
}

impl ToolModalKind {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "model.extrude" => Some(Self::Extrude),
            "model.extrude_individual" => Some(Self::ExtrudeIndividual),
            "model.inset" => Some(Self::Inset),
            "model.bevel" => Some(Self::Bevel),
            "model.push_pull" => Some(Self::PushPull),
            "model.scale_selection" => Some(Self::ScaleSelection),
            _ => None,
        }
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::Extrude => "model.extrude",
            Self::ExtrudeIndividual => "model.extrude_individual",
            Self::Inset => "model.inset",
            Self::Bevel => "model.bevel",
            Self::PushPull => "model.push_pull",
            Self::ScaleSelection => "model.scale_selection",
        }
    }

    pub const fn title(self) -> &'static str {
        match self {
            Self::Extrude => "Extrude",
            Self::ExtrudeIndividual => "Extrude Individual",
            Self::Inset => "Inset",
            Self::Bevel => "Round Edge",
            Self::PushPull => "Push/Pull",
            Self::ScaleSelection => "Scale",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Extrude | Self::ExtrudeIndividual | Self::PushPull => "Distance",
            Self::Inset => "Amount",
            Self::Bevel => "Width",
            Self::ScaleSelection => "Factor",
        }
    }

    pub const fn modal_kind(self) -> petunia_core::ModalKind {
        match self {
            Self::Extrude => petunia_core::ModalKind::Extrude,
            Self::ExtrudeIndividual => petunia_core::ModalKind::ExtrudeIndividual,
            Self::Inset => petunia_core::ModalKind::Inset,
            Self::Bevel => petunia_core::ModalKind::Bevel,
            Self::PushPull => petunia_core::ModalKind::PushPull,
            Self::ScaleSelection => petunia_core::ModalKind::Scale,
        }
    }

    pub const fn bounds(self) -> (f32, f32) {
        match self {
            Self::Inset => (0.0, 0.95),
            Self::Bevel => (0.0, 100.0),
            Self::ScaleSelection => (0.01, 100.0),
            Self::Extrude | Self::ExtrudeIndividual | Self::PushPull => (-100.0, 100.0),
        }
    }

    pub const fn step(self) -> f32 {
        match self {
            Self::Inset => 0.01,
            _ => 0.1,
        }
    }
}

/// Ferramenta que segue a gramática única (constituição 11, ADR 007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrammarTool {
    /// Move, Rotate e Scale: arrasto livre ("haul") ou pelas alças do gizmo.
    Transform(TransformKind),
    /// Extrude, Inset, Round Edge e Push/Pull: o valor segue o cursor.
    Parametric(ToolModalKind),
    /// Poly Pen: arrastar move o elemento sob o cursor; Ctrl-arrastar aresta
    /// extruda; cliques desenham um polígono; Ctrl-clique derrete o ponto.
    PolyPen,
}

/// Gesto da gramática única em andamento.
#[derive(Debug, Clone, Copy)]
enum ToolGesture {
    Transform {
        gizmo: bool,
    },
    Parametric {
        kind: ToolModalKind,
        frame: petunia_core::DragFrame,
        anchor: [f32; 2],
        start_value: f32,
    },
}

/// Id persistente da ferramenta paramétrica no trilho.
const fn parametric_tool_id(kind: ToolModalKind) -> &'static str {
    match kind {
        ToolModalKind::Extrude | ToolModalKind::ExtrudeIndividual => "extrude",
        ToolModalKind::Inset => "inset",
        ToolModalKind::Bevel => "bevel",
        ToolModalKind::PushPull => "push_pull",
        ToolModalKind::ScaleSelection => "scale",
    }
}

/// Sessão de arrasto transacional iniciada na viewport.
#[derive(Debug, Clone, Copy)]
pub struct ViewportDrag {
    pub kind: TransformKind,
    pub start: [f32; 2],
    pub viewport: [f32; 2],
    pub last_pointer: [f32; 2],
    pub virtual_pointer: [f32; 2],
    pub rotation_angle: f32,
    pub last_angle: f32,
}

/// Ação semântica emitida pelo shell Slint.
#[derive(Debug, Clone, PartialEq)]
pub enum UiIntent {
    SetWorkspace(Workspace),
    SaveProject,
    Undo,
    Redo,
    SetSelectionDomain(SelectionDomain),
    CycleSelectionDomain,
    AddPrimitive(petunia_core::PrimitiveKind),
    FreezeActivePrimitive,
    DeleteActiveAsset,
    SetPaintColor([f32; 3]),
    SetBrushSize(f32),
    SetBrushOpacity(f32),
    SetBrushHardness(f32),
    TogglePaintSymmetryX,
    TogglePaintSymmetryY,
    TogglePaintSymmetryZ,
    SetPaintSymmetryX(bool),
    SetPaintSymmetryY(bool),
    SetPaintSymmetryZ(bool),
    SetPaintTargetVertex(bool),
    TogglePaintMaskSelection,
    SetPaintMaskSelection(bool),
    SetActiveTool(String),
    OpenCommandSearch,
    OpenSettings,
    CloseSettings,
    SetKeymap(String),
    ToggleSceneDrawer,
    ExecuteCommand(CommandId),
    DismissTopOverlay,
    ScrubTransform {
        kind: TransformKind,
        axis: usize,
        delta: f32,
        fine: bool,
    },
    ViewportGesture(ViewportGesture),
    SaveProjectTo(PathBuf),
    OpenProjectFrom(PathBuf),
    ImportModelFrom(PathBuf),
    ExportActiveObjTo(PathBuf),
    ExportSceneGlbTo(PathBuf),
    ImportPalette(PathBuf),
    ExportPalette(PathBuf),
    SelectSceneAsset(String),
    ToggleSceneAssetVisibility(String),
    ToggleSceneAssetLock(String),
    /// Reorders an asset by directional delta (-1: up, +1: down)
    /// Reordena um asset por delta direcional (-1: para cima, +1: para baixo)
    MoveSceneAsset {
        id: String,
        delta: i32,
    },
    /// Reorders an asset from index to index
    /// Reordena um asset de um índice de origem para um de destino
    ReorderSceneAsset {
        from: usize,
        to: usize,
    },
    /// Alternates local isolation mode for the active asset
    /// Alterna o modo de isolamento local para o asset ativo
    ToggleIsolateActiveAsset,
    SetTheme(String),
    DuplicateActiveAsset,
    SelectAll,
    ClearSelection,
    InvertSelection,
    ToggleAssetLibrary,
    ResetCamera,
    ToggleProjection,
    SaveActiveAsAsset,
    AssignMaterialSlot(usize),
    CreateMaterial,
    DuplicateMaterial(usize),
    Paint2dStroke {
        norm_x: f32,
        norm_y: f32,
        phase: i32,
    },
    TogglePaintPixelGrid,
    SetPaintCanvasZoom(i32),
    ProjectFromReference,
    BakeReference,
    ToggleFaceOrientation,
    ToggleUvChecker,
    ToggleProportionalEditing,
    SetProportionalRadius(f32),
    SetProportionalFalloff(String),
    ToggleSnapEnabled,
    SetSnapTarget(String),
    AddProfileRectangle {
        width: f32,
        height: f32,
    },
    AddProfileCircle {
        radius: f32,
        segments: usize,
    },
    AddDecalLayer,
    SetDecalTransform {
        layer_id: String,
        center_u: f32,
        center_v: f32,
        scale_u: f32,
        scale_v: f32,
        rotation_deg: f32,
    },
    BakeActiveDecal,
    SetSectionDocked {
        section: petunia_config::InspectorSectionId,
        docked: bool,
    },
    MoveSectionFloat {
        section: petunia_config::InspectorSectionId,
        x: f32,
        y: f32,
    },
    SetSectionPinOpen {
        section: petunia_config::InspectorSectionId,
        pin_open: bool,
    },
    SetSectionPinnedAsset {
        section: petunia_config::InspectorSectionId,
        asset: Option<String>,
    },
    SetOriginGeometry,
    SetOriginBottom,
    SetOriginCursor,
    SetOriginSelection,
    SetGeometryToOrigin,
    ToggleEditPivot,
    SetColorblindAxes(bool),
    SetReducedMotion(bool),
    SetDoubleTapIntervalMs(u64),
    ToggleMicroInspector,
    TogglePaintUvOverlay,
    ToggleUvShowTexture,
    ProfileSetWorkplaneGround,
    ProfileSetWorkplaneFace,
    ProfileSetWorkplaneView,
    /// Plano automático: face sob o cursor ou plano mais paralelo à vista.
    ProfileSetWorkplaneAuto,
    /// DRAW (forma) ou POLY (componente) dentro do workspace de modelagem.
    SetModelingMode(ModelingMode),
    /// "Olhar para o plano": alinha a câmera ao plano sob comando (cap. 01).
    ProfileLookAtPlane,
}

// Presentation ViewModels and Data Transfer Objects (DTOs) for the Slint shell.
// ViewModels de apresentação e Objetos de Transferência de Dados (DTOs) para o shell Slint.
pub mod section_layout;
pub mod view_model;
pub use view_model::*;

pub mod projection;
pub use projection::*;

pub mod callbacks;
pub(crate) use callbacks::*;

/// Contrato do backend de viewport.
pub trait PetuniaViewport: Send {
    fn resize(&mut self, width: u32, height: u32);
    fn update(&mut self, dt_seconds: f32);
    fn set_workspace(&mut self, workspace: Workspace);
    fn set_selection_domain(&mut self, domain: SelectionDomain);
    /// Entrega invalidações transitórias de textura ao backend antes do frame.
    fn queue_texture_updates(&mut self, _updates: Vec<petunia_core::TextureUpdate>) {}
    /// O backend desenha alvos não selecionados com depth test próprio.
    fn draws_component_guides(&self) -> bool {
        false
    }
    /// O backend renderiza em pixels físicos (nítido em HiDPI e UI scale > 100%).
    /// Backends de CPU permanecem em pixels lógicos para não multiplicar o custo.
    fn uses_physical_pixels(&self) -> bool {
        false
    }
    /// Pixels físicos por pixel lógico, para larguras de linha e pontos.
    fn set_pixel_ratio(&mut self, _ratio: f32) {}
    fn render_frame(
        &mut self,
        _project: &Project,
        _refs: &[petunia_core::ReferenceImage],
        _camera: &Camera,
        _state: ViewportRenderState,
    ) -> Option<slint::Image> {
        None
    }
}

/// Backend inicial usado como fallback seguro quando não há GPU disponível.
#[derive(Debug, Default)]
pub struct PlaceholderViewport {
    pub width: u32,
    pub height: u32,
    pub workspace: Workspace,
    pub selection_domain: SelectionDomain,
    pub last_dt_seconds: f32,
}

impl PetuniaViewport for PlaceholderViewport {
    fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn update(&mut self, dt_seconds: f32) {
        self.last_dt_seconds = dt_seconds.max(0.0);
    }

    fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
    }

    fn set_selection_domain(&mut self, domain: SelectionDomain) {
        self.selection_domain = domain;
    }
}

impl PetuniaViewport for Box<dyn PetuniaViewport> {
    fn resize(&mut self, width: u32, height: u32) {
        (**self).resize(width, height);
    }

    fn update(&mut self, dt_seconds: f32) {
        (**self).update(dt_seconds);
    }

    fn set_workspace(&mut self, workspace: Workspace) {
        (**self).set_workspace(workspace);
    }

    fn set_selection_domain(&mut self, domain: SelectionDomain) {
        (**self).set_selection_domain(domain);
    }

    fn queue_texture_updates(&mut self, updates: Vec<petunia_core::TextureUpdate>) {
        (**self).queue_texture_updates(updates);
    }

    fn draws_component_guides(&self) -> bool {
        (**self).draws_component_guides()
    }

    fn render_frame(
        &mut self,
        project: &Project,
        refs: &[petunia_core::ReferenceImage],
        camera: &Camera,
        state: ViewportRenderState,
    ) -> Option<slint::Image> {
        (**self).render_frame(project, refs, camera, state)
    }
}
/// Sessão de manipulação interativa de decalque 3D: (origem_x, origem_y, center_uv_inicial, scale_uv_inicial, rot_deg_inicial).
pub type DecalDragInitial = (f32, f32, [f32; 2], [f32; 2], f32);

/// Bridge entre callbacks Slint e a aplicação. O bridge só aplica intenção
/// semântica ao `AppState`; algoritmos geométricos permanecem no core/commands.
pub struct SlintUiBridge<V: PetuniaViewport> {
    pub state: AppState,
    pub viewport: V,
    pub overlays: OverlayStack,
    pub scene_drawer_visible: bool,
    pub asset_library_visible: bool,
    pub asset_query: String,
    pub asset_sort_by_name: bool,
    pub parts_query: String,
    pub parts_selected_only: bool,
    pub parts_sort_by_name: bool,
    pub parts_row_height: f32,
    pub command_search_visible: bool,
    pub settings_visible: bool,
    pub position: [NumericFieldState; 3],
    pub rotation: [NumericFieldState; 3],
    pub scale: [NumericFieldState; 3],
    pub drag: Option<ViewportDrag>,
    pub viewport_size: [f32; 2],
    /// Última posição de tela do traço de pintura ativo.
    pub paint_last: Option<[f32; 2]>,
    /// Amostragem incremental do stroke 3D em pixels lógicos da viewport.
    pub paint_sampler: petunia_core::StrokeSampler,
    /// Menu de primitivas aberto (apresentação).
    pub add_menu_open: bool,
    /// Ferramenta paramétrica modal ativa (Extrude, Inset, Bevel, Push/Pull).
    pub tool_modal: Option<ToolModalKind>,
    /// Pixels físicos por pixel lógico da viewport (última medição da janela).
    pub pixel_ratio: f32,
    /// Máquina de estados da gramática única (constituição 11).
    pub tool_session: petunia_core::ToolSession,
    /// Gesto confirmado, ajustável no card "Última operação".
    pub last_operation: Option<petunia_core::LastOperation>,
    /// Variante da ferramenta paramétrica persistente (ex.: Extrude Individual).
    parametric_tool: Option<ToolModalKind>,
    tool_gesture: Option<ToolGesture>,
    tool_press_extend: bool,
    /// Modificador de "ação alternativa" no último press (Ctrl no preset
    /// padrão): no Poly Pen, extruda a aresta ou derrete o ponto.
    tool_press_alternate: bool,
    /// Pontos coletados pelo Poly Pen (estado `Collecting`).
    pub poly_pen_points: Vec<petunia_core::PenPoint>,
    /// Sessão paramétrica aberta por atalho: o movimento do mouse já manipula.
    pub keyboard_tool_modal_active: bool,
    pub tool_modal_value: f32,
    /// Renomeação inline do ativo selecionado (Outliner): `Some(nome em edição)`.
    pub rename_draft: Option<String>,
    /// Menu de contexto do Outliner: posição em px lógicos e ativo alvo.
    pub context_menu: Option<ContextMenuState>,
    /// Menu da barra superior aberto, se houver.
    pub menu_open: Option<MenuKind>,
    pub pivot_menu_open: bool,
    /// Popover de opções de shading aberto.
    pub shading_popover_open: bool,
    pub pointer_position: [f32; 2],
    pub modal_text: String,
    pub instant_transform: bool,
    pub gizmo_hover: Option<GizmoHandle>,
    /// Handle do gizmo sendo arrastado, se houver.
    pub gizmo_drag: Option<GizmoHandle>,
    /// Forma ancorada (Line/Rectangle) em curso: canto inicial em pixels do canvas.
    pub shape_anchor: Option<(u32, u32)>,
    /// Plano de corte (Slice) ativo: âncora em pixels lógicos da viewport.
    pub slice_anchor: Option<[f32; 2]>,
    /// Último clique na viewport com timestamp para detecção de duplo-clique.
    pub last_viewport_click: Option<([f32; 2], std::time::Instant)>,
    /// Sessão de loop cut com slide interativo (P3D-131).
    pub loop_cut: Option<LoopCutSessionState>,
    /// Aresta atualmente sob o cursor para a prévia não destrutiva do Loop Cut.
    pub loop_cut_hover_ring: Option<petunia_core::LoopRing>,
    /// Snapshot usado para a prévia por hover; commit só ocorre após click.
    pub loop_cut_hover_source: Option<petunia_core::Mesh>,
    pub loop_cut_hover_cuts: usize,
    pub loop_cut_balanced: bool,
    pub active_material_slot: i32,
    /// Autosave rotativo do shell (P3D-002). Nunca sobrescreve o arquivo oficial.
    pub autosave: petunia_core::AutosaveService,
    /// Snapshot de recuperação detectado no arranque, aguardando decisão.
    pub pending_recovery: Option<petunia_core::RecoveryInfo>,
    pub paint_pixel_grid: bool,
    pub paint_canvas_zoom: i32,
    pub paint_2d_last: Option<(u32, u32)>,
    /// Amostragem incremental do stroke no espaço de pixels da textura.
    pub paint_2d_sampler: petunia_core::StrokeSampler,
    pub paint_target_vertex: bool,
    /// Sessão de manipulação interativa de decalque 3D: (origem_x, origem_y, center_uv_inicial, scale_uv_inicial, rot_deg_inicial).
    pub decal_drag_initial: Option<DecalDragInitial>,
    /// Runtime dock/float/pin layouts of the six Inspector sections.
    /// Presentation-only: the document and its Undo history never see this.
    /// Layouts de dock/flutuação/pin das seis seções do Inspector.
    /// Só apresentação: o documento e seu histórico de Undo nunca veem isso.
    pub section_layouts: section_layout::SectionLayouts,
    /// Cached preferences backing `section_layouts`; mutated intents persist it.
    /// Preferências em cache por trás de `section_layouts`; intents a persistem.
    pub preferences: petunia_config::UserPreferences,
    /// Test-only override for the preferences file path (hermetic tests).
    /// Production leaves `None` and uses the platform preferences path.
    /// Override de teste do caminho do arquivo de preferências (testes herméticos).
    /// Produção mantém `None` e usa o caminho da plataforma.
    pub preferences_path_override: Option<std::path::PathBuf>,
    /// Rastreamento de duplo toque em atalhos de ferramenta: (nome da ferramenta, instante).
    pub last_tool_press: Option<(String, std::time::Instant)>,
    pub slice_trim: bool,
    pub micro_inspector_open: bool,
    pub micro_inspector_pos: [f32; 2],
    pub paint_show_uv_overlay: bool,
    pub uv_show_texture: bool,
    pub active_profile_id: Option<uuid::Uuid>,
    pub profile_volume_mode: Option<petunia_module_model::ProfileVolumeMode>,
    pub profile_drag_target: Option<ProfileHitTarget>,
    pub profile_selected_point: Option<uuid::Uuid>,
    /// Pré-seleção do Draw: onde o próximo clique cairia (ponto encaixado ou
    /// face que viraria o plano), mostrada com o marcador de snap.
    pub profile_hover_snap: Option<petunia_core::ScreenSnapHit>,
    /// Região de perfil sob o cursor (Push/Pull e Draw), destacada no hover.
    pub region_hover: Option<petunia_core::RegionHit>,
    /// DRAW ou POLY: mesmos documento, seleção, câmera e Inspector; muda o
    /// trilho de ferramentas (ADR 007).
    pub modeling_mode: ModelingMode,
    /// Regiões por plano, recalculadas só quando o documento muda.
    region_planes_cache: Option<([u64; 11], petunia_core::RegionPlanes)>,
    pub profile_preview_asset_id: Option<uuid::Uuid>,
    profile_edit_gesture: Option<ProfileEditGesture>,
    profile_volume_original: Option<Project>,
    pub reference_manager_open: bool,
    pub reference_thumbnails: std::collections::HashMap<String, (u32, u32, Vec<u8>)>,
    pub clipboard: Option<GeometryClipboard>,
}

/// Dados da área de transferência de geometria.
#[derive(Clone, Debug)]
pub enum GeometryClipboard {
    Mesh(petunia_core::Mesh),
    Asset(Box<petunia_project::Asset>),
}

/// Modo do workspace de modelagem (ADR 007): DRAW trabalha no nível de forma,
/// POLY no nível de componente.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ModelingMode {
    Draw,
    #[default]
    Poly,
}

impl ModelingMode {
    /// Rótulo estável usado pela UI para a pílula ativa.
    pub fn id(self) -> &'static str {
        match self {
            Self::Draw => "DRAW",
            Self::Poly => "POLY",
        }
    }

    /// Aparência das arestas do viewport neste modo (capítulo 05).
    pub fn edge_mode(self) -> petunia_render_wgpu::EdgeMode {
        match self {
            Self::Draw => petunia_render_wgpu::EdgeMode::Features,
            Self::Poly => petunia_render_wgpu::EdgeMode::Topology,
        }
    }

    /// A ferramenta persistente pertence ao trilho deste modo.
    pub fn offers_tool(self, tool: &str) -> bool {
        const SHARED: [&str; 5] = ["select", "move", "rotate", "scale", "push_pull"];
        const DRAW: [&str; 1] = ["draw_profile"];
        SHARED.contains(&tool)
            || match self {
                Self::Draw => DRAW.contains(&tool),
                Self::Poly => !DRAW.contains(&tool),
            }
    }
}

/// Alvo de hit-test para manipulação interativa de nós e alças do perfil 2D.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileHitTarget {
    Anchor(uuid::Uuid),
    HandleOut(uuid::Uuid),
    HandleIn(uuid::Uuid),
}

#[derive(Debug, Clone)]
struct ProfileEditGesture {
    spline_id: uuid::Uuid,
    point_id: uuid::Uuid,
    point_before: Option<petunia_core::SplinePoint>,
    revision_before: u64,
}

/// Menu de contexto do Outliner aberto sobre uma linha do painel Parts,
/// ou menu da viewport (verbetes de seleção) aberto com o botão direito
/// sobre o espaço 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextMenuState {
    pub x: f32,
    pub y: f32,
    pub asset: uuid::Uuid,
    /// Verdadeiro no modo viewport (sem alvo de asset): só verbetes de
    /// seleção, nunca rename/visibility/lock.
    pub viewport: bool,
}

/// Menus da barra superior do shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    File,
    Edit,
    View,
    Window,
}

impl MenuKind {
    pub const ALL: [MenuKind; 4] = [Self::File, Self::Edit, Self::View, Self::Window];

    pub const fn id(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Edit => "edit",
            Self::View => "view",
            Self::Window => "window",
        }
    }

    pub const fn title(self) -> petunia_config::TextId {
        use petunia_config::text_id as T;
        match self {
            Self::File => T::MENU_FILE,
            Self::Edit => T::MENU_EDIT,
            Self::View => T::MENU_VIEW,
            Self::Window => T::MENU_WINDOW,
        }
    }

    /// Itens publicados pelo menu, na ordem de exibição.
    ///
    /// Cada id é um comando canônico real ou uma ação de shell que já existe no
    /// roteador — nenhum item decorativo.
    pub const fn items(self) -> &'static [(&'static str, petunia_config::TextId, &'static str)] {
        use petunia_config::text_id as T;
        match self {
            Self::File => &[
                ("file.new", T::FILE_NEW, "Ctrl+N"),
                ("file.open", T::FILE_OPEN_PROJECT, "Ctrl+O"),
                ("file.save", T::FILE_SAVE, "Ctrl+S"),
                ("file.save_as", T::FILE_SAVE_AS, "Ctrl+Shift+S"),
                ("file.import_obj", T::FILE_IMPORT_OBJ, ""),
            ],
            Self::Edit => &[
                ("edit.undo", T::EDIT_UNDO, "Ctrl+Z"),
                ("edit.redo", T::EDIT_REDO, "Ctrl+Shift+Z"),
                ("edit.duplicate", T::UI_DUPLICATE, "Shift+D"),
            ],
            Self::View => &[
                ("view.frame_selection", T::VIEW_FRAME, "F"),
                ("view.frame_all", T::VIEW_FRAME_ALL, "Home"),
                ("view.toggle_projection", T::VIEW_TOGGLE_PROJECTION, "O"),
                ("view.toggle_wireframe", T::VIEW_TOGGLE_WIREFRAME, "Z"),
                ("view.reset_camera", T::VIEW_RESET_CAMERA, "Shift+Home"),
            ],
            Self::Window => &[
                ("window.command_palette", T::MENU_COMMAND_PALETTE, "Ctrl+P"),
                ("window.settings", T::MENU_PREFERENCES, ""),
            ],
        }
    }
}

/// Editor UV 2D: geometria do layout pronta para o `Path` do Slint.
/// Handle do gizmo sob o cursor ou em arrasto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoHandle {
    X,
    Y,
    Z,
    Center,
    Plane(u8), // 0: YZ (normal X), 1: XZ (normal Y), 2: XY (normal Z)
}

impl GizmoHandle {
    /// Índice do eixo para `ModalConstraint::Axis`, se for um eixo individual.
    pub const fn axis(self) -> Option<usize> {
        match self {
            Self::X => Some(0),
            Self::Y => Some(1),
            Self::Z => Some(2),
            Self::Center | Self::Plane(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GizmoTarget {
    handle: GizmoHandle,
    kind: TransformKind,
}

/// Estado da sessão de loop cut ativa no shell.
#[derive(Debug, Clone)]
pub struct LoopCutSessionState {
    pub ring: petunia_core::LoopRing,
    pub cuts: usize,
    pub slide: f32,
    pub balanced: bool,
    /// Malha anterior ao preview: toda reconstrução parte daqui, nunca do preview.
    pub source: petunia_core::Mesh,
}

/// Tema disponível no registry, já marcado como ativo ou não.
/// Normaliza os nomes de teclas do teclado numérico entre plataformas/backends
/// (`KP_1`, `Numpad1`, `Keypad1`, `NumPad1`, `Numpad 1`, ...) para o dígito ou
/// operador que representam: "0".."9", ".", "/".
fn numpad_key(text: &str) -> Option<&'static str> {
    let name = text
        .strip_prefix("KP_")
        .or_else(|| text.strip_prefix("Numpad "))
        .or_else(|| text.strip_prefix("Numpad"))
        .or_else(|| text.strip_prefix("NumPad"))
        .or_else(|| text.strip_prefix("Keypad"))?;
    Some(match name {
        "0" => "0",
        "1" => "1",
        "2" => "2",
        "3" => "3",
        "4" => "4",
        "5" => "5",
        "6" => "6",
        "7" => "7",
        "8" => "8",
        "9" => "9",
        "Decimal" | "." => ".",
        "Divide" | "/" => "/",
        _ => return None,
    })
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn new(state: AppState, viewport: V) -> Self {
        let mut bridge = Self {
            state,
            viewport,
            overlays: OverlayStack::default(),
            scene_drawer_visible: false,
            asset_library_visible: false,
            asset_query: String::new(),
            asset_sort_by_name: false,
            parts_query: String::new(),
            parts_selected_only: false,
            parts_sort_by_name: false,
            parts_row_height: 28.0,
            command_search_visible: false,
            settings_visible: false,
            drag: None,
            viewport_size: [1024.0, 768.0],
            paint_last: None,
            paint_sampler: petunia_core::StrokeSampler::default(),
            add_menu_open: false,
            tool_modal: None,
            pixel_ratio: 1.0,
            tool_session: petunia_core::ToolSession::default(),
            last_operation: None,
            parametric_tool: None,
            tool_gesture: None,
            tool_press_extend: false,
            tool_press_alternate: false,
            poly_pen_points: Vec::new(),
            keyboard_tool_modal_active: false,
            tool_modal_value: 0.0,
            rename_draft: None,
            context_menu: None,
            menu_open: None,
            pivot_menu_open: false,
            shape_anchor: None,
            slice_anchor: None,
            last_viewport_click: None,
            loop_cut: None,
            loop_cut_hover_ring: None,
            loop_cut_hover_source: None,
            loop_cut_hover_cuts: 1,
            loop_cut_balanced: false,
            active_material_slot: 0,
            shading_popover_open: false,
            pointer_position: [512.0, 384.0],
            modal_text: String::new(),
            instant_transform: false,
            gizmo_hover: None,
            gizmo_drag: None,
            autosave: petunia_core::AutosaveService::default(),
            pending_recovery: None,
            paint_pixel_grid: true,
            paint_canvas_zoom: 1,
            paint_2d_last: None,
            paint_2d_sampler: petunia_core::StrokeSampler::default(),
            paint_target_vertex: false,
            decal_drag_initial: None,
            section_layouts: section_layout::default_section_layouts(),
            preferences: petunia_config::UserPreferences::default(),
            preferences_path_override: None,
            last_tool_press: None,
            slice_trim: false,
            micro_inspector_open: false,
            micro_inspector_pos: [512.0, 384.0],
            paint_show_uv_overlay: false,
            uv_show_texture: true,
            active_profile_id: None,
            profile_volume_mode: None,
            profile_drag_target: None,
            profile_selected_point: None,
            profile_hover_snap: None,
            region_hover: None,
            modeling_mode: ModelingMode::default(),
            region_planes_cache: None,
            profile_preview_asset_id: None,
            profile_edit_gesture: None,
            profile_volume_original: None,
            reference_manager_open: false,
            reference_thumbnails: std::collections::HashMap::new(),
            clipboard: None,
            position: [
                NumericFieldState::new(0.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(0.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(0.0, None, None).with_steps(0.1, 0.01),
            ],
            rotation: [
                NumericFieldState::new(0.0, None, None).with_steps(1.0, 0.1),
                NumericFieldState::new(0.0, None, None).with_steps(1.0, 0.1),
                NumericFieldState::new(0.0, None, None).with_steps(1.0, 0.1),
            ],
            scale: [
                NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01),
                NumericFieldState::new(1.0, None, None).with_steps(0.1, 0.01),
            ],
        };
        bridge.sync_viewport_context();
        bridge
    }

    pub fn apply(&mut self, intent: UiIntent) {
        if matches!(
            &intent,
            UiIntent::SetWorkspace(_)
                | UiIntent::SetSelectionDomain(_)
                | UiIntent::SetActiveTool(_)
                | UiIntent::OpenProjectFrom(_)
                | UiIntent::SelectSceneAsset(_)
        ) {
            self.cancel_active_operation();
        }
        match intent {
            UiIntent::SetWorkspace(workspace) => {
                self.state.switch_workspace(workspace);
                // Preselection morta de outro workspace não pode vazar para
                // cá: o hover pertence ao domínio e ao modo onde nasceu.
                self.state.session.tools.hover = petunia_core::HoverTarget::None;
            }
            UiIntent::SaveProject => {
                if let Some(path) = self.state.project.project_path.clone() {
                    self.apply(UiIntent::SaveProjectTo(PathBuf::from(path)));
                } else {
                    self.state.set_status("Save As required");
                }
            }
            UiIntent::OpenCommandSearch => {
                self.command_search_visible = true;
                self.overlays.push(OverlayEntry {
                    id: OverlayId::CommandPalette,
                    kind: OverlayKind::Modal,
                    pinned: false,
                    dismiss_on_escape: true,
                    dismiss_on_click_away: true,
                });
            }
            UiIntent::OpenSettings => {
                self.settings_visible = true;
                self.overlays.push(OverlayEntry {
                    id: OverlayId::Settings,
                    kind: OverlayKind::Modal,
                    pinned: false,
                    dismiss_on_escape: true,
                    dismiss_on_click_away: true,
                });
            }
            UiIntent::CloseSettings => {
                self.settings_visible = false;
                self.overlays.remove(OverlayId::Settings);
            }
            UiIntent::SetKeymap(profile_id) => {
                self.set_keymap_profile(&profile_id);
            }
            UiIntent::ToggleSceneDrawer => {
                self.scene_drawer_visible = !self.scene_drawer_visible;
                if self.scene_drawer_visible {
                    self.overlays.push(OverlayEntry {
                        id: OverlayId::SceneDrawer,
                        kind: OverlayKind::Drawer,
                        pinned: false,
                        dismiss_on_escape: true,
                        dismiss_on_click_away: true,
                    });
                } else {
                    self.overlays.remove(OverlayId::SceneDrawer);
                }
            }
            UiIntent::ExecuteCommand(id) => {
                self.execute_command(id);
            }
            UiIntent::DismissTopOverlay => {
                self.handle_escape();
            }
            UiIntent::ScrubTransform {
                kind,
                axis,
                delta,
                fine,
            } => {
                self.scrub_transform(kind, axis, delta, fine);
            }
            UiIntent::ViewportGesture(gesture) => {
                self.apply_viewport_gesture(gesture);
            }
            UiIntent::SaveProjectTo(path) => {
                if let Err(err) = petunia_project::format::save(&self.state.project.project, &path)
                {
                    self.state.set_status(format!("failed to save: {err}"));
                } else {
                    self.state.project.project_path = Some(path.display().to_string());
                    self.state.mark_document_clean();
                    self.state
                        .set_status(format!("project saved: {}", path.display()));
                }
            }
            UiIntent::OpenProjectFrom(path) => match petunia_project::format::load(&path) {
                Ok(project) => {
                    self.state.project.palette = project.palette.clone();
                    self.state.project.project = project;
                    self.state.project.undo.clear();
                    self.state.project.project_path = Some(path.display().to_string());
                    self.state.mark_document_clean();
                    self.state
                        .set_status(format!("project opened: {}", path.display()));
                }
                Err(err) => {
                    self.state.set_status(format!("failed to open: {err}"));
                }
            },
            UiIntent::ImportModelFrom(path) => {
                match petunia_core::ProjectService::import_file_pipeline(
                    &mut self.state,
                    &path,
                    &petunia_project::pipeline::ImportOptions::default(),
                ) {
                    Ok(names) => self
                        .state
                        .set_status(format!("imported {} asset(s)", names.len())),
                    Err(error) => self.state.set_status(format!("import failed: {error}")),
                }
            }
            UiIntent::ExportActiveObjTo(path) => {
                let index = self.state.project.active;
                match petunia_core::ProjectService::export_obj(&self.state, index, &path) {
                    Ok(()) => self
                        .state
                        .set_status(format!("exported {}", path.display())),
                    Err(error) => self.state.set_status(format!("export failed: {error}")),
                }
            }
            UiIntent::ExportSceneGlbTo(path) => {
                let indices = self.state.project.export_selected_indices();
                match petunia_core::ProjectService::export_glb(&self.state, &indices, &path) {
                    Ok(()) => self
                        .state
                        .set_status(format!("exported {}", path.display())),
                    Err(error) => self.state.set_status(format!("export failed: {error}")),
                }
            }
            UiIntent::ImportPalette(path) => {
                match petunia_module_paint::PaintModule::import_palette_file(&mut self.state, &path)
                {
                    Ok(count) => {
                        self.state
                            .set_status(format!("imported palette ({count} colors)"));
                        self.state.mark_dirty();
                    }
                    Err(error) => {
                        self.state
                            .set_status(format!("palette import failed: {error}"));
                    }
                }
            }
            UiIntent::ExportPalette(path) => {
                match petunia_module_paint::PaintModule::export_palette_file(&self.state, &path) {
                    Ok(()) => {
                        self.state.set_status("palette exported successfully");
                    }
                    Err(error) => {
                        self.state
                            .set_status(format!("palette export failed: {error}"));
                    }
                }
            }
            UiIntent::Undo => {
                if self.cancel_active_operation() {
                    return;
                }
                if self.state.undo() {
                    self.state.set_status("Desfazer executado.");
                }
            }
            UiIntent::Redo => {
                if self.cancel_active_operation() {
                    return;
                }
                if self.state.redo() {
                    self.state.set_status("Refazer executado.");
                }
            }
            UiIntent::SetSelectionDomain(domain) => {
                self.state.set_selection_domain(domain);
                self.state
                    .set_status(format!("Modo de seleção: {:?}", domain));
            }
            UiIntent::CycleSelectionDomain => {
                self.state.cycle_selection_domain();
                let domain = self.state.selection_domain();
                self.state
                    .set_status(format!("Modo de seleção: {:?}", domain));
            }
            UiIntent::AddPrimitive(kind) => {
                self.state.set_selection_domain(SelectionDomain::Object);
                self.sync_viewport_context();
                self.state.begin_primitive(kind, None);
                self.state
                    .set_status(format!("Added {}", kind.default_name()));
            }
            UiIntent::FreezeActivePrimitive => {
                if self.state.freeze_active_primitive() {
                    self.state
                        .set_status("Primitive frozen to editable mesh".to_string());
                }
            }
            UiIntent::DeleteActiveAsset => {
                // `edit.delete` é contextual: em Object remove o asset ativo; em
                // Point/Edge/Face remove os sub-elementos selecionados.
                if let Err(error) = self.state.dispatch_command("edit.delete") {
                    self.state.set_status(error.to_string());
                }
            }
            UiIntent::SetPaintColor(color) => {
                if color.iter().all(|component| component.is_finite()) {
                    let color = color.map(|component| component.clamp(0.0, 1.0));
                    self.state.paint_color = color;
                    self.state.session.tools.paint_color = color;
                }
            }
            UiIntent::SetBrushSize(size) => {
                self.state.session.tools.paint_radius = size.clamp(0.01, 100.0);
            }
            UiIntent::SetBrushOpacity(opacity) => {
                self.state.session.tools.paint_strength = opacity.clamp(0.0, 1.0);
            }
            UiIntent::SetBrushHardness(hardness) => {
                self.state.session.tools.brush_hardness = hardness.clamp(0.0, 1.0);
            }
            UiIntent::TogglePaintSymmetryX => {
                self.state.session.tools.paint_symmetry_x =
                    !self.state.session.tools.paint_symmetry_x;
            }
            UiIntent::TogglePaintSymmetryY => {
                self.state.session.tools.paint_symmetry_y =
                    !self.state.session.tools.paint_symmetry_y;
            }
            UiIntent::TogglePaintSymmetryZ => {
                self.state.session.tools.paint_symmetry_z =
                    !self.state.session.tools.paint_symmetry_z;
            }
            UiIntent::SetPaintSymmetryX(val) => {
                self.state.session.tools.paint_symmetry_x = val;
            }
            UiIntent::SetPaintSymmetryY(val) => {
                self.state.session.tools.paint_symmetry_y = val;
            }
            UiIntent::SetPaintSymmetryZ(val) => {
                self.state.session.tools.paint_symmetry_z = val;
            }
            UiIntent::SetPaintTargetVertex(val) => {
                self.paint_target_vertex = val;
            }
            UiIntent::TogglePaintMaskSelection => {
                self.state.session.tools.paint_isolate_selection =
                    !self.state.session.tools.paint_isolate_selection;
            }
            UiIntent::SetPaintMaskSelection(val) => {
                self.state.session.tools.paint_isolate_selection = val;
            }
            UiIntent::SetActiveTool(tool) => {
                if tool != "poly_pen" {
                    self.poly_pen_points.clear();
                }
                if self.state.session.tools.active_tool == "draw_profile" && tool != "draw_profile"
                {
                    self.profile_pointer_up();
                    self.state.profile.clear();
                    self.active_profile_id = None;
                    self.profile_selected_point = None;
                    self.profile_drag_target = None;
                    self.profile_edit_gesture = None;
                }
                self.state.session.tools.active_tool = tool.clone();
                self.state.set_status(format!("Ferramenta ativa: {tool}"));
                match tool.as_str() {
                    // Transformações são transacionais por arrasto: a sessão
                    // modal abre no pointer-down da viewport, não ao escolher a
                    // ferramenta.
                    "move" | "rotate" | "scale" | "transform" => {
                        self.state.session.tools.gizmo_mode = match tool.as_str() {
                            "rotate" => petunia_core::ModalKind::Rotate,
                            "scale" => petunia_core::ModalKind::Scale,
                            _ => petunia_core::ModalKind::Move,
                        };
                    }
                    "cut" => {
                        if let Some(mesh) = self.state.project.active_mesh().cloned() {
                            self.state.session.tools.cut_session =
                                Some(petunia_core::CutSession::new(mesh));
                            self.state
                                .set_status("Cut: choose two edge points in the viewport");
                        }
                    }
                    "draw_profile" => {
                        self.active_profile_id = None;
                        self.profile_selected_point = None;
                        self.profile_drag_target = None;
                        self.profile_edit_gesture = None;
                        // A câmera nunca se move sozinha (capítulos 01 e 02).
                        petunia_module_model::profile_capture_current(&mut self.state);
                        let plane = self.workplane_display_name();
                        let message = self
                            .state
                            .t_id(petunia_config::text_id::TOOL_GRAMMAR_DRAW_READY)
                            .replace("{plane}", &plane);
                        self.state.set_status(message);
                    }
                    "loop_cut" => {
                        self.loop_cut_hover_ring = None;
                        self.loop_cut_hover_source = None;
                        self.loop_cut_hover_cuts = 1;
                        self.state.session.tools.hover = petunia_core::HoverTarget::None;
                        self.state.set_status("Loop Cut: hover a quad edge ring, click to place, scroll to change cuts");
                    }
                    "slice" => {
                        self.state
                            .set_status("Slice: drag in the viewport to define the cut plane");
                    }
                    "fill" => {
                        let count =
                            petunia_module_paint::PaintModule::fill_selection(&mut self.state);
                        self.state.set_status(format!("Filled {count} points"));
                    }
                    // Formas e pincéis do workspace PAINT compartilham o mesmo
                    // índice de tipo de pincel que o resto do domínio.
                    "line" | "rectangle" => {
                        self.state.session.tools.paint_brush_kind =
                            petunia_core::kind_from_brush_type(if tool == "line" {
                                petunia_core::BrushType::Line
                            } else {
                                petunia_core::BrushType::Rectangle
                            });
                        self.state
                            .set_status("Shape: press on the surface to anchor, release to commit");
                    }
                    "brush" | "eraser" | "picker" | "airbrush" | "pixel" => {
                        self.state.session.tools.paint_brush_kind =
                            petunia_core::kind_from_brush_type(match tool.as_str() {
                                "eraser" => petunia_core::BrushType::Eraser,
                                "picker" => petunia_core::BrushType::Eyedropper,
                                "airbrush" => petunia_core::BrushType::Airbrush,
                                "pixel" => petunia_core::BrushType::Pixel,
                                _ => petunia_core::BrushType::Soft,
                            });
                    }
                    _ => {}
                }
            }
            UiIntent::SelectSceneAsset(id_str) => {
                if let Some(idx) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter().position(|a| a.id == id))
                {
                    self.state.select_object(Some(idx), false);
                    self.state.mark_dirty();
                    self.reset_transform_fields();
                }
            }
            UiIntent::ToggleSceneAssetVisibility(id_str) => {
                if let Some(asset_index) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter().position(|a| a.id == id))
                    && let Err(error) =
                        self.state
                            .dispatch(&petunia_core::ToggleVisibilityAssetCmd {
                                asset_index: Some(asset_index),
                            })
                {
                    self.state.set_status(error.to_string());
                }
            }
            UiIntent::ToggleSceneAssetLock(id_str) => {
                if let Some(asset_index) = uuid::Uuid::parse_str(&id_str)
                    .ok()
                    .and_then(|id| self.state.project.assets.iter().position(|a| a.id == id))
                    && let Err(error) = self.state.dispatch(&petunia_core::ToggleLockAssetCmd {
                        asset_index: Some(asset_index),
                    })
                {
                    self.state.set_status(error.to_string());
                }
            }
            UiIntent::MoveSceneAsset { id, delta } => {
                // Moves asset by delta in scene hierarchy / Move asset por delta na hierarquia da cena
                if let Some(uuid) = uuid::Uuid::parse_str(&id).ok()
                    && let Some(idx) = self.state.project.find(uuid)
                {
                    let len = self.state.project.assets.len();
                    let target = (idx as isize + delta as isize)
                        .clamp(0, (len.saturating_sub(1)) as isize)
                        as usize;
                    if target != idx {
                        let _ = self.state.dispatch(&petunia_core::ReorderAssetCmd {
                            from: idx,
                            to: target,
                        });
                    }
                }
            }
            UiIntent::ReorderSceneAsset { from, to } => {
                // Reorders asset explicitly from index to index / Reordena asset explicitamente entre índices
                let _ = self
                    .state
                    .dispatch(&petunia_core::ReorderAssetCmd { from, to });
            }
            UiIntent::ToggleIsolateActiveAsset => {
                // Toggles isolation of active asset / Alterna isolamento do asset ativo
                self.state.toggle_isolate();
            }
            UiIntent::SetTheme(theme_id) => {
                self.state.ui.active_theme_id = theme_id;
            }
            UiIntent::DuplicateActiveAsset => {
                let _ = self
                    .state
                    .dispatch(&petunia_core::DuplicateAssetCmd { asset_index: None });
                self.state.set_status("Objeto duplicado.");
            }
            UiIntent::SelectAll => {
                let _ = self.state.dispatch(&petunia_core::SelectAllCmd);
                self.state.set_status("Tudo selecionado.");
            }
            UiIntent::ClearSelection => {
                let _ = self.state.dispatch(&petunia_core::ClearSelectionCmd);
                self.state.set_status("Seleção limpa.");
            }
            UiIntent::InvertSelection => {
                let _ = self.state.dispatch(&petunia_core::InvertSelectionCmd);
                self.state.set_status("Seleção invertida.");
            }
            UiIntent::ToggleAssetLibrary => {
                self.asset_library_visible = !self.asset_library_visible;
                if self.asset_library_visible {
                    self.overlays.push(OverlayEntry {
                        id: OverlayId::AssetLibrary,
                        kind: OverlayKind::Drawer,
                        pinned: false,
                        dismiss_on_escape: true,
                        dismiss_on_click_away: true,
                    });
                } else {
                    self.overlays.remove(OverlayId::AssetLibrary);
                }
            }
            UiIntent::ResetCamera => {
                let _ = self.state.dispatch(&petunia_core::ResetCameraCmd);
                self.state.set_status("Câmera redefinida.");
            }
            UiIntent::ToggleProjection => {
                let _ = self.state.dispatch(&petunia_core::ToggleProjectionCmd);
                self.state.set_status(
                    if self.state.session.camera.proj == petunia_core::Projection::Ortho {
                        "Projeção: Ortográfica"
                    } else {
                        "Projeção: Perspectiva"
                    },
                );
            }
            UiIntent::SaveActiveAsAsset => {
                if self.state.save_active_as_asset() {
                    self.state
                        .set_status("Ativo salvo na biblioteca de assets.");
                }
            }
            UiIntent::AssignMaterialSlot(slot) => {
                self.assign_material_slot(slot);
            }
            UiIntent::CreateMaterial => {
                self.create_material();
            }
            UiIntent::DuplicateMaterial(slot) => {
                self.duplicate_material(slot);
            }
            UiIntent::Paint2dStroke {
                norm_x,
                norm_y,
                phase,
            } => {
                self.paint_2d_stroke(norm_x, norm_y, phase);
            }
            UiIntent::TogglePaintPixelGrid => {
                self.paint_pixel_grid = !self.paint_pixel_grid;
            }
            UiIntent::SetPaintCanvasZoom(zoom) => {
                self.paint_canvas_zoom = zoom.clamp(1, 16);
            }
            UiIntent::ProjectFromReference => {
                self.project_from_reference();
            }
            UiIntent::BakeReference => {
                self.bake_reference();
            }
            UiIntent::ToggleFaceOrientation => {
                self.toggle_face_orientation();
            }
            UiIntent::ToggleUvChecker => {
                self.toggle_uv_checker();
            }
            UiIntent::ToggleProportionalEditing => {
                self.toggle_proportional_editing();
            }
            UiIntent::SetProportionalRadius(radius) => {
                self.set_proportional_radius(radius);
            }
            UiIntent::SetProportionalFalloff(falloff) => {
                self.set_proportional_falloff(&falloff);
            }
            UiIntent::ToggleSnapEnabled => {
                self.toggle_snap_enabled();
            }
            UiIntent::SetSnapTarget(target) => {
                self.set_snap_target(&target);
            }
            UiIntent::AddProfileRectangle { width, height } => {
                self.add_profile_rectangle(width, height);
            }
            UiIntent::AddProfileCircle { radius, segments } => {
                self.add_profile_circle(radius, segments);
            }
            UiIntent::AddDecalLayer => {
                self.add_decal_layer();
            }
            UiIntent::SetDecalTransform {
                layer_id,
                center_u,
                center_v,
                scale_u,
                scale_v,
                rotation_deg,
            } => {
                // Dispatches command to update decal transformation (P3D-133)
                // Despacha comando para atualizar transformação do decalque (P3D-133)
                if let Ok(id) = uuid::Uuid::parse_str(&layer_id) {
                    let _ = self.state.dispatch(&petunia_core::SetDecalTransformCmd {
                        layer_id: id,
                        center_uv: [center_u, center_v],
                        scale_uv: [scale_u, scale_v],
                        rotation_rad: rotation_deg.to_radians(),
                    });
                }
            }
            UiIntent::BakeActiveDecal => {
                // Bakes active decal layer to static raster (P3D-160)
                // Rasteriza a camada decal ativa para raster estático (P3D-160)
                if let Some(active_layer) = self
                    .state
                    .project
                    .assets
                    .get(self.state.project.active)
                    .and_then(|asset| asset.paint_stack.as_ref())
                    .and_then(|stack| stack.active())
                {
                    let id = active_layer.id;
                    let _ = self
                        .state
                        .dispatch(&petunia_core::BakeDecalCmd { layer_id: id });
                }
            }
            UiIntent::SetSectionDocked { section, docked } => {
                self.set_section_docked(section, docked);
            }
            UiIntent::MoveSectionFloat { section, x, y } => {
                self.move_section_float(section, x, y);
            }
            UiIntent::SetSectionPinOpen { section, pin_open } => {
                self.set_section_pin_open(section, pin_open);
            }
            UiIntent::SetSectionPinnedAsset { section, asset } => {
                self.set_section_pinned_asset(section, asset);
            }
            UiIntent::SetOriginGeometry => {
                let _ = self.state.set_origin_geometry();
            }
            UiIntent::SetOriginBottom => {
                let _ = self.state.set_origin_bottom();
            }
            UiIntent::SetOriginCursor => {
                let _ = self.state.set_origin_cursor();
            }
            UiIntent::SetOriginSelection => {
                let _ = self.state.set_origin_selection();
            }
            UiIntent::SetGeometryToOrigin => {
                let _ = self.state.set_geometry_to_origin();
            }
            UiIntent::ToggleEditPivot => {
                self.state.toggle_edit_pivot();
            }
            UiIntent::SetColorblindAxes(enabled) => {
                let _ = self.set_colorblind_axes(enabled);
            }
            UiIntent::SetReducedMotion(enabled) => {
                let _ = self.set_reduced_motion(enabled);
            }
            UiIntent::SetDoubleTapIntervalMs(interval) => {
                let _ = self.set_double_tap_interval_ms(interval);
            }
            UiIntent::ToggleMicroInspector => {
                let _ = self.toggle_micro_inspector();
            }
            UiIntent::TogglePaintUvOverlay => {
                self.paint_show_uv_overlay = !self.paint_show_uv_overlay;
                self.state.mark_dirty();
            }
            UiIntent::ToggleUvShowTexture => {
                self.uv_show_texture = !self.uv_show_texture;
                self.state.mark_dirty();
            }
            UiIntent::ProfileSetWorkplaneGround => {
                self.reset_profile_for_workplane();
                self.state.profile.workplane_locked = true;
                petunia_module_model::profile_capture_ground(&mut self.state);
                self.announce_locked_workplane();
            }
            UiIntent::ProfileSetWorkplaneFace => {
                self.reset_profile_for_workplane();
                self.state.profile.workplane_locked = true;
                if petunia_module_model::profile_capture_face(&mut self.state) {
                    self.announce_locked_workplane();
                } else {
                    let message = self
                        .state
                        .t_id(petunia_config::text_id::TOOL_GRAMMAR_NO_FACE_SELECTED);
                    self.state.set_status(message);
                }
            }
            UiIntent::ProfileSetWorkplaneView => {
                self.reset_profile_for_workplane();
                self.state.profile.workplane_locked = true;
                petunia_module_model::profile_capture_view(&mut self.state);
                self.announce_locked_workplane();
            }
            UiIntent::ProfileSetWorkplaneAuto => {
                self.reset_profile_for_workplane();
                self.state.profile.workplane_locked = false;
                petunia_module_model::profile_capture_auto(&mut self.state);
                let message = self
                    .state
                    .t_id(petunia_config::text_id::TOOL_GRAMMAR_WORKPLANE_AUTO);
                self.state.set_status(message);
            }
            UiIntent::SetModelingMode(mode) => {
                if self.state.workspace != Workspace::Model {
                    self.state.switch_workspace(Workspace::Model);
                    self.state.session.tools.hover = petunia_core::HoverTarget::None;
                }
                if self.modeling_mode != mode {
                    self.modeling_mode = mode;
                    // A ferramenta ativa que não existe no trilho novo volta
                    // para Select; nada é convertido ou selecionado sozinho.
                    let tool = self.state.session.tools.active_tool.clone();
                    if !mode.offers_tool(&tool) {
                        self.apply(UiIntent::SetActiveTool("select".into()));
                    }
                }
                let message = self.state.t_id(match mode {
                    ModelingMode::Draw => petunia_config::text_id::WORKSPACE_DRAW_READY,
                    ModelingMode::Poly => petunia_config::text_id::WORKSPACE_POLY_READY,
                });
                self.state.set_status(message);
                self.state.mark_dirty();
            }
            UiIntent::ProfileLookAtPlane => {
                petunia_module_model::profile_align_camera_to_workplane(&mut self.state);
                self.state.mark_dirty();
            }
        }
        self.sync_viewport_context();
    }

    /// Navegação da câmera. Nunca é suspensa por ferramenta; a roda sempre
    /// faz zoom (constituição 11). Contagens e raios usam `viewport_ctrl_scroll`.
    pub fn apply_viewport_gesture(&mut self, gesture: ViewportGesture) {
        match gesture {
            ViewportGesture::Orbit { dx, dy } => {
                self.state.session.camera.orbit(dx, dy);
            }
            ViewportGesture::Pan { dx, dy } => {
                self.state.session.camera.pan(dx, dy);
            }
            ViewportGesture::Zoom { delta } => {
                self.state.session.camera.zoom(delta);
            }
        }
        self.state.mark_dirty();
    }

    pub fn render_viewport(&mut self) -> Option<slint::Image> {
        let render_state = ViewportRenderState {
            shading: self.state.session.shading,
            xray: self.state.session.show_xray,
            show_triangulation: self.state.session.show_triangulation,
            textured: self.state.session.textured,
            show_wireframe_overlay: self.state.session.show_wireframe_overlay,
            show_face_orientation: self.state.session.show_face_orientation,
            show_uv_checker: self.state.session.show_uv_checker,
            selection_domain: self.state.selection_domain(),
            xray_opacity: self.state.session.xray_opacity,
            selection_rgb: self.state.ui.selection_rgb,
            selection_thickness: self.state.ui.selection_thickness,
            show_grid: self.state.session.show_grid,
            hover: self.state.session.tools.hover,
            boolean_operand: self.state.session.tools.boolean_operand,
            studio_light_follows_camera: self.preferences.studio_light_follows_camera,
            edge_mode: self.edge_mode(),
        };
        self.viewport
            .queue_texture_updates(self.state.render.take_texture_updates());
        self.viewport.render_frame(
            &self.state.project,
            &self.state.project.refs,
            &self.state.session.camera,
            render_state,
        )
    }

    /// Cria uma camada de efeito com parâmetros padrão para o tipo pedido.
    pub fn add_paint_effect_layer(&mut self, kind: &str) -> bool {
        use petunia_project::paint_layers::{PaintEffect, PaintLayer};
        let effect = match kind {
            "Pixelate" => PaintEffect::Pixelate { cell_size: 4 },
            "Posterize" => PaintEffect::Posterize { levels: 6 },
            "Invert" => PaintEffect::Invert,
            "Grain" => PaintEffect::Grain {
                intensity: 0.15,
                seed: 1,
            },
            "BrightnessContrast" => PaintEffect::BrightnessContrast {
                brightness: 0.0,
                contrast: 0.0,
            },
            "HueSaturation" => PaintEffect::HueSaturation {
                hue_shift_deg: 0.0,
                saturation: 0.0,
            },
            _ => return false,
        };
        self.mutate_paint_stack("add effect layer", |stack| {
            stack.add_layer(PaintLayer::new_effect(kind.to_string(), effect));
            true
        })
    }

    /// Ajusta um parâmetro do efeito da camada ativa.
    pub fn set_paint_effect_param(&mut self, key: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        self.mutate_paint_stack("effect parameter", |stack| {
            let Some(layer) = stack.active_mut() else {
                return false;
            };
            let petunia_project::paint_layers::LayerKind::Effect(effect) = &mut layer.kind else {
                return false;
            };
            use petunia_project::paint_layers::PaintEffect;
            match (effect, key) {
                (PaintEffect::Pixelate { cell_size }, "cell_size") => {
                    *cell_size = (value.round() as u32).clamp(1, 64);
                }
                (PaintEffect::Posterize { levels }, "levels") => {
                    *levels = (value.round() as u8).clamp(2, 32);
                }
                (PaintEffect::Grain { intensity, .. }, "intensity") => {
                    *intensity = value.clamp(0.0, 1.0);
                }
                (PaintEffect::Grain { seed, .. }, "seed") => {
                    *seed = value.max(0.0).round() as u32;
                }
                (PaintEffect::BrightnessContrast { brightness, .. }, "brightness") => {
                    *brightness = value.clamp(-1.0, 1.0);
                }
                (PaintEffect::BrightnessContrast { contrast, .. }, "contrast") => {
                    *contrast = value.clamp(-1.0, 1.0);
                }
                (PaintEffect::HueSaturation { hue_shift_deg, .. }, "hue_shift_deg") => {
                    *hue_shift_deg = value.clamp(-180.0, 180.0);
                }
                (PaintEffect::HueSaturation { saturation, .. }, "saturation") => {
                    *saturation = value.clamp(-1.0, 1.0);
                }
                _ => return false,
            }
            true
        })
    }

    /// Dimensões do canvas composto do ativo, quando existe.
    pub fn paint_canvas_dimensions(&self) -> Option<(u32, u32)> {
        let asset = self.state.project.assets.get(self.state.project.active)?;
        if let Some(texture) = asset.texture.as_ref() {
            return Some((texture.w, texture.h));
        }
        let stack = asset.paint_stack.as_ref()?;
        let layer = stack.active()?;
        let canvas = layer.canvas()?;
        Some((canvas.w, canvas.h))
    }

    /// Publica a imagem do canvas 2D na janela, quando houver camada.
    pub fn publish_canvas_image(&mut self, window: &PetuniaSlintShell) {
        if let Some(image) = self.render_paint_canvas() {
            window.set_paint_canvas_image(image);
        }
    }

    /// Converte a camada ativa em imagem Slint para o editor 2D.
    ///
    /// A camada ativa é a superfície que o pincel realmente altera; o composto
    /// (`Asset.texture`) é o que vai para o material.
    pub fn render_paint_canvas(&mut self) -> Option<slint::Image> {
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let asset = self.state.project.assets.get(self.state.project.active)?;
        let canvas = asset
            .paint_stack
            .as_ref()
            .and_then(|stack| stack.active())
            .and_then(|layer| layer.canvas())?;
        let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(canvas.w, canvas.h);
        let pixels = buffer.make_mut_bytes();
        let source = &canvas.pixels;
        let length = pixels.len().min(source.len());
        pixels[..length].copy_from_slice(&source[..length]);
        Some(slint::Image::from_rgba8(buffer))
    }

    /// Grade vetorial contextual do editor 2D, nas coordenadas do quadro Slint.
    pub fn paint_canvas_grid_commands(&self) -> String {
        use std::fmt::Write as _;

        const VIEW_SIZE: f32 = 256.0;
        if !self.paint_pixel_grid {
            return String::new();
        }
        let Some((width, height)) = self.paint_canvas_dimensions() else {
            return String::new();
        };
        let zoom = self.paint_canvas_zoom.clamp(1, 16) as f32;
        let scaled_size = VIEW_SIZE * zoom;
        let offset = (VIEW_SIZE - scaled_size) * 0.5;
        let step_x = scaled_size / width.max(1) as f32;
        let step_y = scaled_size / height.max(1) as f32;
        let mut commands = String::new();

        if step_x >= 4.0 {
            let first = ((-offset) / step_x).ceil().max(0.0) as u32;
            let last = ((VIEW_SIZE - offset) / step_x).floor().min(width as f32) as u32;
            for column in first..=last {
                let x = offset + column as f32 * step_x;
                let _ = write!(&mut commands, "M {x:.2} 0 L {x:.2} {VIEW_SIZE:.2} ");
            }
        }
        if step_y >= 4.0 {
            let first = ((-offset) / step_y).ceil().max(0.0) as u32;
            let last = ((VIEW_SIZE - offset) / step_y).floor().min(height as f32) as u32;
            for row in first..=last {
                let y = offset + row as f32 * step_y;
                let _ = write!(&mut commands, "M 0 {y:.2} L {VIEW_SIZE:.2} {y:.2} ");
            }
        }
        commands
    }

    /// Gera comandos SVG de projeção da estampa de decalque sobre a superfície 3D (P3D-133).
    pub fn decal_preview_commands(&self) -> String {
        if self.state.workspace != Workspace::Paint {
            return String::new();
        }
        let Some(decal) = self.active_decal() else {
            return String::new();
        };
        let Some(mesh) = self.state.project.active_mesh() else {
            return String::new();
        };
        let viewport = [
            self.viewport_size[0].max(1.0),
            self.viewport_size[1].max(1.0),
        ];
        let matrix = self.state.session.camera.view_proj();
        let project = |point: glam::Vec3| -> Option<[f32; 2]> {
            let clip = matrix * point.extend(1.0);
            if !clip.is_finite() || clip.w <= 0.05 || clip.z < 0.0 || clip.z > clip.w {
                return None;
            }
            Some([
                (clip.x / clip.w * 0.5 + 0.5) * viewport[0],
                (0.5 - clip.y / clip.w * 0.5) * viewport[1],
            ])
        };

        let cu = decal.center_uv[0];
        let cv = decal.center_uv[1];
        let su = decal.scale_uv[0];
        let sv = decal.scale_uv[1];
        let rot = decal.rotation_rad;
        let cos_r = rot.cos();
        let sin_r = rot.sin();

        // Converte coordenadas locais da estampa [-0.5..0.5] para espaço UV
        let local_to_uv = |lx: f32, ly: f32| -> [f32; 2] {
            let du = lx * su * cos_r - ly * sv * sin_r;
            let dv = lx * su * sin_r + ly * sv * cos_r;
            [(cu + du).clamp(0.0, 1.0), (cv + dv).clamp(0.0, 1.0)]
        };

        // Âncora central e plano tangente de contingência
        let center_hit = mesh.uv_to_world([cu, cv]);
        let (c_pos, c_norm) = center_hit.unwrap_or((glam::Vec3::ZERO, glam::Vec3::Y));
        let up = if c_norm.y.abs() > 0.9 {
            glam::Vec3::X
        } else {
            glam::Vec3::Y
        };
        let tangent = c_norm.cross(up).normalize_or_zero();
        let bitangent = c_norm.cross(tangent).normalize_or_zero();

        let sample_world_pos = |lx: f32, ly: f32| -> glam::Vec3 {
            let uv = local_to_uv(lx, ly);
            if let Some((pos, norm)) = mesh.uv_to_world(uv) {
                pos + norm * 0.003
            } else {
                let du = lx * su * cos_r - ly * sv * sin_r;
                let dv = lx * su * sin_r + ly * sv * cos_r;
                c_pos + (tangent * du + bitangent * dv) * 2.0 + c_norm * 0.003
            }
        };

        use std::fmt::Write as _;
        let mut commands = String::new();

        // 1. Perímetro do decalque com subdivisões para conformação a superfícies curvas
        let mut perimeter_pts = Vec::new();
        const SUBDIVS: usize = 4;
        // Borda superior: (-0.5, -0.5) -> (0.5, -0.5)
        for i in 0..SUBDIVS {
            let t = i as f32 / SUBDIVS as f32;
            perimeter_pts.push((-0.5 + t, -0.5));
        }
        // Borda direita: (0.5, -0.5) -> (0.5, 0.5)
        for i in 0..SUBDIVS {
            let t = i as f32 / SUBDIVS as f32;
            perimeter_pts.push((0.5, -0.5 + t));
        }
        // Borda inferior: (0.5, 0.5) -> (-0.5, 0.5)
        for i in 0..SUBDIVS {
            let t = i as f32 / SUBDIVS as f32;
            perimeter_pts.push((0.5 - t, 0.5));
        }
        // Borda esquerda: (-0.5, 0.5) -> (-0.5, -0.5)
        for i in 0..SUBDIVS {
            let t = i as f32 / SUBDIVS as f32;
            perimeter_pts.push((-0.5, 0.5 - t));
        }

        let mut first = true;
        for (lx, ly) in perimeter_pts {
            let world_pos = sample_world_pos(lx, ly);
            if let Some(screen_pt) = project(world_pos) {
                if first {
                    let _ = write!(commands, "M {:.2} {:.2} ", screen_pt[0], screen_pt[1]);
                    first = false;
                } else {
                    let _ = write!(commands, "L {:.2} {:.2} ", screen_pt[0], screen_pt[1]);
                }
            }
        }
        if !first {
            commands.push_str("Z ");
        }

        // 2. Retículo / mira central
        let center_h0 = sample_world_pos(-0.08, 0.0);
        let center_h1 = sample_world_pos(0.08, 0.0);
        if let (Some(a), Some(b)) = (project(center_h0), project(center_h1)) {
            let _ = write!(
                commands,
                "M {:.2} {:.2} L {:.2} {:.2} ",
                a[0], a[1], b[0], b[1]
            );
        }
        let center_v0 = sample_world_pos(0.0, -0.08);
        let center_v1 = sample_world_pos(0.0, 0.08);
        if let (Some(a), Some(b)) = (project(center_v0), project(center_v1)) {
            let _ = write!(
                commands,
                "M {:.2} {:.2} L {:.2} {:.2} ",
                a[0], a[1], b[0], b[1]
            );
        }

        // 3. Indicador direcional de orientação (topo do decalque)
        let arrow_base = sample_world_pos(0.0, 0.0);
        let arrow_tip = sample_world_pos(0.0, -0.38);
        let arrow_left = sample_world_pos(-0.06, -0.28);
        let arrow_right = sample_world_pos(0.06, -0.28);
        if let (Some(base), Some(tip), Some(left), Some(right)) = (
            project(arrow_base),
            project(arrow_tip),
            project(arrow_left),
            project(arrow_right),
        ) {
            let _ = write!(
                commands,
                "M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} ",
                base[0],
                base[1],
                tip[0],
                tip[1],
                left[0],
                left[1],
                tip[0],
                tip[1],
                right[0],
                right[1]
            );
        }

        commands
    }

    pub fn resize_viewport(&mut self, width: u32, height: u32) {
        self.resize_viewport_scaled(width, height, self.pixel_ratio);
    }

    /// Redimensiona a viewport. `width`/`height` são px lógicos (câmera, picking
    /// e overlays); `pixel_ratio` é px físicos por px lógico (fator de escala da
    /// janela, incluindo a preferência de UI scale).
    pub fn resize_viewport_scaled(&mut self, width: u32, height: u32, pixel_ratio: f32) {
        let width = width.max(1);
        let height = height.max(1);
        let ratio = if pixel_ratio.is_finite() && pixel_ratio > 0.0 {
            pixel_ratio.clamp(0.5, 4.0)
        } else {
            1.0
        };
        self.pixel_ratio = ratio;
        if self.viewport.uses_physical_pixels() {
            self.viewport.set_pixel_ratio(ratio);
            self.viewport.resize(
                (width as f32 * ratio).round().max(1.0) as u32,
                (height as f32 * ratio).round().max(1.0) as u32,
            );
        } else {
            self.viewport.resize(width, height);
        }
        self.viewport_size = [width as f32, height as f32];
        self.state.session.camera.aspect = width as f32 / height as f32;
        self.state.mark_dirty();
    }

    /// Preenche o HUD da operação e a barra de status contextual.
    ///
    /// O HUD diz *o que está acontecendo e com que valor*; a barra diz *como
    /// controlar*. Os dois nunca repetem a mesma informação.
    fn fill_operation_hud(&self, vm: &mut ShellViewModel) {
        let axis_name = |index: usize| ["X", "Y", "Z"][index.min(2)];

        if self.state.session.tools.active_tool == "loop_cut" && self.loop_cut.is_none() {
            vm.operation_hud_active = true;
            vm.operation_hud_title = "Loop Cut".to_string();
            vm.operation_hud_lines = vec![format!("Cuts    {}", self.loop_cut_hover_cuts)];
            vm.operation_hud_hint =
                "Hover a quad edge · Scroll Cuts · Click place · Enter confirm · Esc cancel"
                    .to_string();
            vm.context_hint = vm.operation_hud_hint.clone();
            return;
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            vm.operation_hud_active = true;
            vm.operation_hud_title = "Profile".to_string();
            vm.operation_hud_lines =
                vec![format!("{} point(s)", self.active_profile_point_count())];
            vm.operation_hud_hint = if self.active_profile_closed() {
                "Generate Volume or Revolve · Esc finishes editing".to_string()
            } else {
                "Click to add points · Click first point to close · Esc finishes editing"
                    .to_string()
            };
            vm.context_hint = vm.operation_hud_hint.clone();
            return;
        }

        // Ferramenta paramétrica modal.
        if let Some(kind) = self.tool_modal {
            let (minimum, maximum) = kind.bounds();
            let _ = (minimum, maximum);
            vm.operation_hud_active = true;
            vm.operation_hud_title = kind.title().to_string();
            vm.operation_hud_lines =
                vec![format!("{}   {:.3}", kind.label(), self.tool_modal_value)];
            if !self.modal_text.is_empty() {
                vm.operation_hud_lines
                    .push(format!("Input   {}", self.modal_text));
            }
            vm.operation_hud_subject = self
                .state
                .project
                .active_mesh()
                .map(|mesh| {
                    let faces = mesh.faces.iter().filter(|face| face.selected).count();
                    let verts = mesh.verts.iter().filter(|vert| vert.selected).count();
                    if faces > 0 {
                        format!("{faces} face(s)")
                    } else {
                        format!("{verts} point(s)")
                    }
                })
                .unwrap_or_default();
            vm.operation_hud_hint = format!(
                "{} Confirm   Esc Cancel   Shift Precision",
                if self.is_instant_tool_mode() {
                    "Click"
                } else {
                    "Release"
                }
            );
            vm.context_hint = format!("{} · {}", kind.title(), vm.operation_hud_hint);
            return;
        }

        // Loop Cut.
        if let Some(session) = &self.loop_cut {
            vm.operation_hud_active = true;
            vm.operation_hud_title = "Loop Cut".to_string();
            vm.operation_hud_lines = vec![
                format!("Cuts    {}", session.cuts),
                format!("Slide   {:.3}", session.slide),
            ];
            vm.operation_hud_subject = format!("{} cut(s)", session.cuts);
            vm.operation_hud_hint = "Enter Confirm   Esc Cancel   Drag to slide".to_string();
            vm.context_hint = format!("Loop Cut · {}", vm.operation_hud_hint);
            return;
        }

        if self.state.session.tools.active_tool == "cut"
            && let Some(session) = &self.state.session.tools.cut_session
        {
            vm.operation_hud_active = true;
            vm.operation_hud_title = "Cut".into();
            vm.operation_hud_lines = vec![format!("{} segment(s)", session.segments)];
            vm.operation_hud_hint = "Click edge points · Enter Apply · Esc Cancel".into();
            vm.context_hint = vm.operation_hud_hint.clone();
            if let Some(point) = session.edge_start {
                let clip = self.state.session.camera.view_proj() * point.position.extend(1.0);
                if clip.w > 0.0 {
                    let x = (clip.x / clip.w * 0.5 + 0.5) * self.viewport_size[0];
                    let y = (0.5 - clip.y / clip.w * 0.5) * self.viewport_size[1];
                    vm.operation_preview_commands = format!(
                        "M {x:.2} {y:.2} L {:.2} {:.2}",
                        self.pointer_position[0], self.pointer_position[1]
                    );
                }
            }
            return;
        }

        // Transformação por arrasto (gizmo ou ferramenta ativa).
        if let Some(modal) = self.state.session.tools.modal.as_ref() {
            let title = match modal.kind {
                petunia_core::ModalKind::Move => "Move",
                petunia_core::ModalKind::Rotate => "Rotate",
                petunia_core::ModalKind::Scale => "Scale",
                petunia_core::ModalKind::Extrude => "Extrude",
                petunia_core::ModalKind::ExtrudeIndividual => "Extrude Individual",
                petunia_core::ModalKind::Inset => "Inset",
                petunia_core::ModalKind::Bevel => "Round Edge",
                petunia_core::ModalKind::PushPull => "Push/Pull",
            };
            let mut lines = Vec::new();
            match modal.constraint {
                petunia_core::ModalConstraint::Axis(i) => {
                    lines.push(format!("{}   {:.3}", axis_name(i), modal.value));
                }
                petunia_core::ModalConstraint::Plane(i) => {
                    lines.push(format!(
                        "Plane {}{}   {:.3}",
                        axis_name((i + 1) % 3),
                        axis_name((i + 2) % 3),
                        modal.value
                    ));
                }
                petunia_core::ModalConstraint::Free => {
                    let components = modal.components;
                    if modal.kind == petunia_core::ModalKind::Move {
                        lines.push(format!("X   {:.3}", components.x));
                        lines.push(format!("Y   {:.3}", components.y));
                        lines.push(format!("Z   {:.3}", components.z));
                    } else {
                        lines.push(format!("Value   {:.3}", modal.value));
                    }
                }
            }
            if let Some(fb) = self.state.current_tool_feedback() {
                // ToolFeedback telemetry for magnetic snapping (P3D-131)
                // Telemetria de ToolFeedback para atração magnética (P3D-131)
                if let Some(kind) = fb.snap_kind {
                    lines.push(format!("Snap   {}", self.state.t_id(kind.text_id())));
                }
            }
            vm.operation_hud_active = true;
            vm.operation_hud_title = title.to_string();
            if !self.modal_text.is_empty() {
                lines.push(format!("Input   {}", self.modal_text));
            }
            vm.operation_hud_lines = lines;
            vm.operation_hud_subject = format!(
                "{} · {}",
                if self.state.session.edit_pivot {
                    "pivot"
                } else if self.state.selection_domain() == petunia_core::SelectionDomain::Object {
                    "object"
                } else {
                    "selection"
                },
                match modal.constraint {
                    petunia_core::ModalConstraint::Axis(i) => format!("{} axis", axis_name(i)),
                    petunia_core::ModalConstraint::Plane(i) => {
                        format!("{}{} plane", axis_name((i + 1) % 3), axis_name((i + 2) % 3))
                    }
                    petunia_core::ModalConstraint::Free => "free".to_string(),
                }
            );
            vm.operation_hud_hint = "Enter Confirm   Esc Cancel   Shift Precision".to_string();
            vm.context_hint = format!("{title} · {}", vm.operation_hud_hint);
            return;
        }

        if self.state.session.edit_pivot {
            vm.operation_hud_active = true;
            vm.operation_hud_title = "Edit Pivot Mode".to_string();
            vm.operation_hud_subject = "Pivot".to_string();
            vm.operation_hud_lines = vec!["Moving object pivot point".to_string()];
            vm.operation_hud_hint =
                "D / Insert or Esc to exit · Geometry remains fixed".to_string();
            vm.context_hint =
                "Edit Pivot Mode · D / Insert or Esc to exit · Geometry remains fixed".to_string();
            return;
        }

        if let Some(session) = &self.state.session.primitive_session {
            let name = session.descriptor.kind().default_name();
            vm.operation_hud_active = false;
            vm.context_hint = format!("Add {name} · Enter Confirm · Esc Cancel");
            return;
        }

        // Em repouso: a barra informa o domínio e a navegação.
        vm.operation_hud_active = false;
        vm.context_hint = match self.state.selection_domain() {
            petunia_core::SelectionDomain::Object => {
                "Selection: Object   ·   LMB Select   ·   MMB Orbit   ·   Shift+MMB Pan".to_string()
            }
            petunia_core::SelectionDomain::Vertex => {
                "Selection: Point   ·   LMB Select   ·   MMB Orbit   ·   Shift+MMB Pan".to_string()
            }
            petunia_core::SelectionDomain::Edge => {
                "Selection: Edge   ·   LMB Select   ·   MMB Orbit   ·   Shift+MMB Pan".to_string()
            }
            petunia_core::SelectionDomain::Face => {
                "Selection: Face   ·   LMB Select   ·   MMB Orbit   ·   Shift+MMB Pan".to_string()
            }
        };
    }

    /// Orbita a câmera, usando a seleção como pivô quando existe.
    ///
    /// É o comportamento de Blender/C4D: o usuário orbita em torno do que
    /// está trabalhando, não de um ponto fixo da cena.
    pub fn orbit_viewport(&mut self, dx: f32, dy: f32) -> bool {
        if !dx.is_finite() || !dy.is_finite() {
            return false;
        }
        if self.state.session.tools.active_tool == "cursor"
            || self.state.session.tools.active_tool == "cursor_3d"
            || self.state.session.pivot_point == petunia_core::PivotPoint::Cursor3D
        {
            self.state.session.camera.target = glam::Vec3::from(self.state.session.cursor_3d);
        } else if let Some(center) = self.selection_pivot() {
            self.state.session.camera.target = center;
        }
        self.state.session.camera.orbit(dx, dy);
        self.state.mark_dirty();
        true
    }

    pub fn set_pivot_point(&mut self, id: &str) -> bool {
        let Some(pivot) = pivot_from_id(id) else {
            return false;
        };
        if self.state.session.pivot_point == pivot {
            return false;
        }
        self.state.session.pivot_point = pivot;
        self.state.mark_dirty();
        self.state.set_status(format!("Pivot: {}", pivot.as_str()));
        true
    }

    /// Posiciona o 3D Cursor na viewport dado um ponto normalizado (ou clique na tela).
    pub fn place_cursor_3d(&mut self, norm_x: f32, norm_y: f32) -> bool {
        let nx = norm_x.clamp(0.0, 1.0) * 2.0 - 1.0;
        let ny = 1.0 - norm_y.clamp(0.0, 1.0) * 2.0;

        let (origin, dir) = self.state.session.camera.ray(nx, ny);

        // 1. Snapping magnético a vértices de todas as malhas visíveis
        let mut best_vertex = None;
        let mut min_vert_dist = 0.35_f32;
        for asset in &self.state.project.assets {
            if !asset.visible {
                continue;
            }
            for v in &asset.mesh.verts {
                let p = v.vec();
                let v_rel = p - origin;
                let t = v_rel.dot(dir);
                if t > 0.0 {
                    let proj = origin + dir * t;
                    let dist = (p - proj).length();
                    if dist < min_vert_dist {
                        min_vert_dist = dist;
                        best_vertex = Some(p);
                    }
                }
            }
        }

        let target_pos = if let Some(v_pos) = best_vertex {
            v_pos
        } else if let Some((_face, hit_pos)) = pick_face_hit(&self.state, origin, dir) {
            hit_pos
        } else if dir.y.abs() > 1e-4 {
            let t = -origin.y / dir.y;
            if t > 0.0 {
                origin + dir * t
            } else {
                let plane_t = (self.state.session.cursor_3d[1] - origin.y) / dir.y;
                if plane_t > 0.0 {
                    origin + dir * plane_t
                } else {
                    self.state.session.camera.target
                }
            }
        } else {
            self.state.session.camera.target
        };

        self.state.session.cursor_3d = target_pos.to_array();
        self.state.set_status(format!(
            "3D Cursor: [{:.2}, {:.2}, {:.2}]",
            target_pos.x, target_pos.y, target_pos.z
        ));
        self.state.mark_dirty();
        true
    }

    /// Ajusta uma coordenada individual (X, Y ou Z) do 3D Cursor.
    pub fn set_cursor_3d_coord(&mut self, index: usize, val: f32) -> bool {
        if index < 3 && val.is_finite() {
            self.state.session.cursor_3d[index] = val;
            self.state.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Redefine a posição do 3D Cursor para a origem (0, 0, 0).
    pub fn reset_cursor_3d(&mut self) -> bool {
        self.state.session.cursor_3d = [0.0, 0.0, 0.0];
        self.state
            .set_status("3D Cursor redefinido para a origem (0, 0, 0).");
        self.state.mark_dirty();
        true
    }

    /// Centraliza o alvo da câmera na posição do 3D Cursor.
    pub fn frame_cursor(&mut self) -> bool {
        self.state.session.camera.target = glam::Vec3::from(self.state.session.cursor_3d);
        self.state.set_status("Câmera centralizada no 3D Cursor.");
        self.state.mark_dirty();
        true
    }

    /// Define a origem do objeto ativo para a geometria central.
    pub fn set_origin_geometry(&mut self) -> bool {
        self.state.set_origin_geometry().is_ok()
    }

    /// Define a origem do objeto ativo para o piso / base inferior em Y.
    pub fn set_origin_bottom(&mut self) -> bool {
        self.state.set_origin_bottom().is_ok()
    }

    /// Define a origem do objeto ativo para a coordenada do 3D Cursor.
    pub fn set_origin_cursor(&mut self) -> bool {
        self.state.set_origin_cursor().is_ok()
    }

    /// Define a origem do objeto ativo para o centro dos elementos selecionados.
    pub fn set_origin_selection(&mut self) -> bool {
        self.state.set_origin_selection().is_ok()
    }

    /// Centraliza a geometria do objeto na origem (0, 0, 0) ou origem atual.
    pub fn set_geometry_to_origin(&mut self) -> bool {
        self.state.set_geometry_to_origin().is_ok()
    }

    /// Alterna o modo Ajustar Pivô (Edit Pivot).
    pub fn toggle_edit_pivot(&mut self) -> bool {
        self.state.toggle_edit_pivot()
    }

    /// Distância em mundo que um arrasto de tela representa ao longo de um eixo.
    ///
    /// Projeta a direção do eixo em espaço de tela e mede quanto do movimento
    /// do ponteiro caiu nela, convertendo por `visible_height`.
    fn screen_delta_on_axis(
        &self,
        axis: usize,
        total_x: f32,
        total_y: f32,
        viewport: [f32; 2],
    ) -> f32 {
        let world_axis = match axis {
            0 => glam::Vec3::X,
            1 => glam::Vec3::Y,
            _ => glam::Vec3::Z,
        };
        let view_proj = self.state.session.camera.view_proj();
        let project = |point: glam::Vec3| -> Option<[f32; 2]> {
            let clip = view_proj * glam::Vec4::new(point.x, point.y, point.z, 1.0);
            if clip.w <= 0.05 {
                return None;
            }
            let inv_w = 1.0 / clip.w;
            Some([
                (clip.x * inv_w * 0.5 + 0.5) * viewport[0],
                (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * viewport[1],
            ])
        };
        let origin = self.state.session.tools.modal.as_ref().map_or_else(
            || self.state.calculate_pivot(self.state.session.pivot_point),
            |modal| modal.pivot,
        );
        let Some(a) = project(origin) else {
            return 0.0;
        };
        let Some(b) = project(origin + world_axis) else {
            return 0.0;
        };
        let direction = [b[0] - a[0], b[1] - a[1]];
        let length_squared = direction[0] * direction[0] + direction[1] * direction[1];
        if length_squared <= 1.0e-6 {
            return 0.0;
        }
        // Fração do movimento do ponteiro na direção do eixo, em unidades de
        // mundo (a direção projetada corresponde a 1 unidade do eixo).
        (total_x * direction[0] + total_y * direction[1]) / length_squared
    }

    /// Centro da seleção do ativo, quando há algo selecionado.
    fn selection_pivot(&self) -> Option<glam::Vec3> {
        let mesh = self.state.project.active_mesh()?;
        if self.state.selection_domain() == SelectionDomain::Object {
            return Some(self.state.calculate_pivot(self.state.session.pivot_point));
        }
        if !mesh.has_selection() {
            return None;
        }
        let center = mesh.selection_center();
        if center.iter().all(|value| value.is_finite()) {
            Some(glam::Vec3::from_array(center))
        } else {
            None
        }
    }

    /// Atualiza a preselection de componente sob o cursor.
    ///
    /// Sem X-Ray um componente atrás da geometria não é selecionável, então
    /// também não pode ser destacado: o hover valida profundidade contra a
    /// face frontal antes de aceitar o alvo.
    pub fn hover_component(&mut self, normalized_x: f32, normalized_y: f32) -> bool {
        if normalized_x.is_finite() && normalized_y.is_finite() {
            self.pointer_position = [
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ];
        }
        if self.state.workspace == Workspace::Paint {
            // No Paint o pincel consome o ponteiro: limpar em vez de
            // congelar o último hover do modo Model.
            return self.clear_hover();
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            self.pointer_position = [
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ];
            self.update_profile_preselection(normalized_x, normalized_y);
            // Com um perfil aberto em desenho, o clique adiciona pontos: sem
            // destaque de região para não sugerir outra ação.
            let drawing = self
                .active_profile_resources()
                .is_some_and(|(_, spline)| !spline.closed);
            self.region_hover = if drawing {
                None
            } else {
                self.region_hit_at([
                    normalized_x * self.viewport_size[0],
                    normalized_y * self.viewport_size[1],
                ])
            };
            return true;
        }
        if self.state.session.tools.active_tool == "poly_pen" {
            let pixel = [
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ];
            let next = self.poly_pen_target(pixel);
            let changed = next != self.state.session.tools.hover;
            self.state.session.tools.hover = next;
            // A linha até o cursor acompanha o mouse durante a coleta.
            return changed || !self.poly_pen_points.is_empty();
        }
        if self.state.session.tools.active_tool == "push_pull" && self.tool_modal.is_none() {
            let previous = self.region_hover.take();
            self.region_hover = self.region_hit_at([
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ]);
            if self.region_hover.is_some() {
                self.state.session.tools.hover = petunia_core::HoverTarget::None;
                return previous != self.region_hover;
            }
            if previous.is_some() {
                self.state.session.tools.hover =
                    self.pick_viewport_target(normalized_x, normalized_y);
                return true;
            }
        } else {
            self.region_hover = None;
        }
        if self.state.session.tools.active_tool == "loop_cut" && self.loop_cut.is_none() {
            let previous_edge = match self.state.session.tools.hover {
                petunia_core::HoverTarget::Edge(a, b) => Some((a, b)),
                _ => None,
            };
            let next = self.pick_loop_cut_candidate(normalized_x, normalized_y);
            self.loop_cut_hover_source = next.as_ref().map(|(mesh, _, _)| mesh.clone());
            self.loop_cut_hover_ring = next.as_ref().map(|(_, ring, _)| ring.clone());
            let next_edge = next.map(|(_, _, edge)| edge);
            self.state.session.tools.hover = next_edge
                .map(|(a, b)| petunia_core::HoverTarget::Edge(a, b))
                .unwrap_or_default();
            let changed = self.refresh_loop_cut_hover_preview();
            if let Some(ring) = &self.loop_cut_hover_ring {
                if ring.face_count() > 0 {
                    self.state.set_status("Loop Cut: click to place, scroll to change cuts, Enter to confirm, Esc to cancel");
                }
                return changed || previous_edge != next_edge;
            }
            return changed || previous_edge.is_some();
        }
        let next = self.pick_viewport_target(normalized_x, normalized_y);

        if next == self.state.session.tools.hover {
            return self.state.session.tools.active_tool == "cut";
        }
        self.state.session.tools.hover = next;
        true
    }

    /// Pré-seleção do Draw (constituição 11: "Idle → pré-seleção"). Com o
    /// plano automático e nenhum perfil em curso, destaca a face que o clique
    /// usaria como plano; depois, mostra o ponto encaixado que o clique criaria.
    fn update_profile_preselection(&mut self, normalized_x: f32, normalized_y: f32) {
        if !normalized_x.is_finite() || !normalized_y.is_finite() {
            self.profile_hover_snap = None;
            self.state.session.tools.hover = petunia_core::HoverTarget::None;
            return;
        }
        let ndc = [normalized_x * 2.0 - 1.0, 1.0 - normalized_y * 2.0];
        let choosing_plane =
            !self.state.profile.workplane_locked && self.active_profile_resources().is_none();
        let (hover, snap) = if choosing_plane {
            let hover = match self.pick_target_for_domain(
                SelectionDomain::Face,
                normalized_x,
                normalized_y,
            ) {
                face @ petunia_core::HoverTarget::Face(_) => face,
                _ => petunia_core::HoverTarget::None,
            };
            let mut mask = if self.state.session.snap_enabled || self.state.profile.snap {
                petunia_core::SnapMask::for_target(self.state.session.snap_settings.target)
            } else {
                petunia_core::SnapMask {
                    points: false,
                    edges: false,
                    axes: false,
                    faces: false,
                    grid: false,
                }
            };
            mask.faces = matches!(hover, petunia_core::HoverTarget::Face(_));
            let cursor = glam::Vec2::new(
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            );
            (hover, self.screen_snap(cursor, mask, None, None))
        } else {
            let snap = self
                .profile_screen_to_plane_snapped(ndc[0], ndc[1])
                .and_then(|(_, snap)| snap);
            (petunia_core::HoverTarget::None, snap)
        };
        self.state.session.tools.hover = hover;
        self.profile_hover_snap = snap;
    }

    /// Marcador de snap do quadro: o da operação em curso ou, no Draw, o da
    /// pré-seleção.
    fn snap_marker_model(&self, width: f32, height: f32) -> projection::SnapMarkerModel {
        let marker = compute_snap_marker(&self.state, width, height);
        if marker.visible || self.state.session.tools.active_tool != "draw_profile" {
            return marker;
        }
        self.profile_hover_snap.map_or(marker, |hit| {
            projection::snap_marker_at(&self.state, hit.point, Some(hit.kind), width, height)
        })
    }

    fn pick_loop_cut_candidate(
        &self,
        normalized_x: f32,
        normalized_y: f32,
    ) -> Option<(petunia_core::Mesh, petunia_core::LoopRing, (u32, u32))> {
        let mesh = self.state.project.active_mesh()?;
        let (width, height) = (self.viewport_size[0], self.viewport_size[1]);
        if width <= 1.0 || height <= 1.0 {
            return None;
        }
        let cursor_px = [normalized_x * width, normalized_y * height];
        let camera = &self.state.session.camera;
        let view_proj = camera.view_proj();

        let project_pos = |p: glam::Vec3| -> Option<[f32; 2]> {
            let ndc = view_proj.project_point3(p);
            if !ndc.is_finite() || ndc.z < -1.0 || ndc.z > 1.0 {
                return None;
            }
            Some([(ndc.x * 0.5 + 0.5) * width, (0.5 - ndc.y * 0.5) * height])
        };

        let dist_to_segment = |pt: [f32; 2], a: [f32; 2], b: [f32; 2]| -> f32 {
            let ab = [b[0] - a[0], b[1] - a[1]];
            let ap = [pt[0] - a[0], pt[1] - a[1]];
            let len_sq = ab[0] * ab[0] + ab[1] * ab[1];
            if len_sq <= 1e-4 {
                let dx = pt[0] - a[0];
                let dy = pt[1] - a[1];
                return (dx * dx + dy * dy).sqrt();
            }
            let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / len_sq).clamp(0.0, 1.0);
            let proj = [a[0] + t * ab[0], a[1] + t * ab[1]];
            let dx = pt[0] - proj[0];
            let dy = pt[1] - proj[1];
            (dx * dx + dy * dy).sqrt()
        };

        // 1. Testa se o cursor está sobre uma Face (Blender Quad Selection)
        if let petunia_core::HoverTarget::Face(face_idx) =
            self.pick_target_for_domain(SelectionDomain::Face, normalized_x, normalized_y)
            && let Some(face) = mesh.faces.get(face_idx)
            && face.verts.len() == 4
        {
            let v_indices = [face.verts[0], face.verts[1], face.verts[2], face.verts[3]];
            let mut proj_verts = [None; 4];
            for i in 0..4 {
                if let Some(v) = mesh.verts.get(v_indices[i] as usize) {
                    proj_verts[i] = project_pos(v.vec());
                }
            }
            if proj_verts.iter().all(|p| p.is_some()) {
                let p = [
                    proj_verts[0].unwrap(),
                    proj_verts[1].unwrap(),
                    proj_verts[2].unwrap(),
                    proj_verts[3].unwrap(),
                ];
                let d0 = dist_to_segment(cursor_px, p[0], p[1]);
                let d1 = dist_to_segment(cursor_px, p[1], p[2]);
                let d2 = dist_to_segment(cursor_px, p[2], p[3]);
                let d3 = dist_to_segment(cursor_px, p[3], p[0]);

                let mut candidates = [
                    (d0, (v_indices[0], v_indices[1])),
                    (d1, (v_indices[1], v_indices[2])),
                    (d2, (v_indices[2], v_indices[3])),
                    (d3, (v_indices[3], v_indices[0])),
                ];
                candidates
                    .sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
                for (_, seed) in candidates {
                    if let Ok(ring) = petunia_core::LoopRing::discover(mesh, seed) {
                        return Some((mesh.clone(), ring, seed));
                    }
                }
            }
        }

        // 2. Fallback de proximidade de aresta (24px de tolerância)
        let mut best_candidate: Option<(f32, (u32, u32))> = None;
        for (a, b) in mesh.edges_unique() {
            let (Some(va), Some(vb)) = (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
            else {
                continue;
            };
            let (Some(pa), Some(pb)) = (project_pos(va.vec()), project_pos(vb.vec())) else {
                continue;
            };
            let dist = dist_to_segment(cursor_px, pa, pb);
            if dist < 24.0
                && best_candidate
                    .as_ref()
                    .is_none_or(|(best_d, _)| dist < *best_d)
            {
                best_candidate = Some((dist, (a, b)));
            }
        }

        if let Some((_, seed)) = best_candidate
            && let Ok(ring) = petunia_core::LoopRing::discover(mesh, seed)
        {
            return Some((mesh.clone(), ring, seed));
        }

        None
    }

    fn refresh_loop_cut_hover_preview(&self) -> bool {
        let (Some(ring), Some(source)) = (&self.loop_cut_hover_ring, &self.loop_cut_hover_source)
        else {
            return false;
        };
        let Ok(segments) = ring.preview(source, self.loop_cut_hover_cuts, 0.0) else {
            return false;
        };
        !segments.is_empty()
    }

    fn loop_cut_hover_preview_commands(&self) -> String {
        let (Some(ring), Some(source)) = (&self.loop_cut_hover_ring, &self.loop_cut_hover_source)
        else {
            return String::new();
        };
        let Ok(segments) = (if self.loop_cut_balanced {
            ring.preview_balanced(source, self.loop_cut_hover_cuts, 0.0)
        } else {
            ring.preview(source, self.loop_cut_hover_cuts, 0.0)
        }) else {
            return String::new();
        };
        let mut commands = String::new();
        for [a, b] in segments {
            project_preview_segment(
                &self.state.session.camera,
                self.viewport_size,
                a,
                b,
                &mut commands,
            );
        }
        commands
    }

    /// Atribui o material selecionado ao objeto e às faces selecionadas quando existirem.
    pub fn assign_material_slot(&mut self, slot: usize) -> bool {
        let asset_index = self.state.project.active;
        if asset_index == usize::MAX {
            return false;
        }
        let Some(material_id) = self.state.project.project.materials.get(slot).map(|m| m.id) else {
            return false;
        };
        let selected_faces: Vec<usize> = self
            .state
            .project
            .active_mesh()
            .map(|mesh| {
                mesh.faces
                    .iter()
                    .enumerate()
                    .filter_map(|(index, face)| face.selected.then_some(index))
                    .collect()
            })
            .unwrap_or_default();
        self.state.checkpoint("assign material");
        if let Some(asset) = self.state.project.assets.get_mut(asset_index) {
            asset.material_id = Some(material_id);
        }
        if !selected_faces.is_empty()
            && let Some(mesh) = self.state.project.active_mesh_mut()
        {
            for index in &selected_faces {
                mesh.faces[*index].material_slot = Some(slot);
            }
        }
        let message = if selected_faces.is_empty() {
            format!("Assigned material slot {slot} to object")
        } else {
            format!(
                "Assigned material slot {slot} to object and {} face(s)",
                selected_faces.len()
            )
        };
        self.state.set_status(message);
        self.state.emit_material_changed();
        true
    }

    /// Cria um novo material no projeto e o seleciona como ativo.
    pub fn create_material(&mut self) -> bool {
        let count = self.state.project.project.materials.len();
        let name = format!("Material {}", count + 1);
        self.state.checkpoint("create material");
        self.state
            .project
            .project
            .materials
            .push(petunia_project::Material::new(name));
        self.active_material_slot = count as i32;
        self.state
            .set_status(format!("Created Material {}", count + 1));
        self.state.emit_material_changed();
        true
    }

    /// Duplica o material do slot indicado e o seleciona como ativo.
    pub fn duplicate_material(&mut self, slot: usize) -> bool {
        let Some(mat) = self.state.project.project.materials.get(slot).cloned() else {
            return false;
        };
        let mut dup = mat;
        dup.id = uuid::Uuid::new_v4();
        dup.name = format!("{} Copy", dup.name);
        self.state.checkpoint("duplicate material");
        self.state.project.project.materials.push(dup);
        let new_idx = self.state.project.project.materials.len() - 1;
        self.active_material_slot = new_idx as i32;
        self.state
            .set_status(format!("Duplicated material to slot {new_idx}"));
        self.state.emit_material_changed();
        true
    }

    pub fn select_material_slot(&mut self, slot: i32) -> bool {
        let Ok(slot) = usize::try_from(slot) else {
            return false;
        };
        if slot >= self.state.project.project.materials.len() {
            return false;
        }
        self.active_material_slot = slot as i32;
        true
    }

    pub fn remove_material(&mut self, slot: i32) -> bool {
        let Ok(slot) = usize::try_from(slot) else {
            return false;
        };
        if self.state.project.project.materials.len() <= 1
            || !self.state.project.project.materials.get(slot).is_some()
        {
            return false;
        }
        let material_id = self.state.project.project.materials[slot].id;
        self.state.checkpoint("remove material");
        self.state.project.project.remove_material(material_id);
        self.active_material_slot = (slot as i32 - 1).max(0);
        self.state.emit_material_changed();
        true
    }

    pub fn set_active_material_base_color(&mut self, red: f32, green: f32, blue: f32) -> bool {
        if [red, green, blue].iter().any(|value| !value.is_finite()) {
            return false;
        }
        let slot = self.active_material_slot.max(0) as usize;
        if !self.state.project.project.materials.get(slot).is_some() {
            return false;
        }
        self.state.checkpoint("change material color");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.base_color[0] = red.clamp(0.0, 1.0);
        material.base_color[1] = green.clamp(0.0, 1.0);
        material.base_color[2] = blue.clamp(0.0, 1.0);
        material.base_color[3] = 1.0;
        self.state.emit_material_changed();
        true
    }

    pub fn set_active_material_scalar(&mut self, field: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        let slot = self.active_material_slot.max(0) as usize;
        if !matches!(
            field,
            "roughness" | "metallic" | "normal-scale" | "emission-strength" | "alpha-cutoff"
        ) || !self.state.project.project.materials.get(slot).is_some()
        {
            return false;
        }
        self.state.checkpoint("change material");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        match field {
            "roughness" => material.roughness = value.clamp(0.0, 1.0),
            "metallic" => material.metallic = value.clamp(0.0, 1.0),
            "normal-scale" => material.normal_scale = value.clamp(0.0, 10.0),
            "emission-strength" => material.emission_strength = value.clamp(0.0, 10.0),
            "alpha-cutoff" => material.alpha_cutoff = value.clamp(0.0, 1.0),
            _ => return false,
        }
        material.validate();
        self.state.emit_material_changed();
        true
    }

    pub fn set_active_material_profile(&mut self, profile: i32) -> bool {
        let profile = match profile {
            0 => ShaderProfile::Pbr,
            1 => ShaderProfile::Unlit,
            2 => ShaderProfile::Toon,
            3 => ShaderProfile::Glass,
            4 => ShaderProfile::Emissive,
            _ => return false,
        };
        let slot = self.active_material_slot.max(0) as usize;
        if !self.state.project.project.materials.get(slot).is_some() {
            return false;
        }
        self.state.checkpoint("change material profile");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.profile = profile;
        self.state.emit_material_changed();
        true
    }

    pub fn set_active_material_alpha_mode(&mut self, mode: i32) -> bool {
        let mode = match mode {
            0 => AlphaMode::Opaque,
            1 => AlphaMode::Mask,
            2 => AlphaMode::Blend,
            _ => return false,
        };
        let slot = self.active_material_slot.max(0) as usize;
        if !self.state.project.project.materials.get(slot).is_some() {
            return false;
        }
        self.state.checkpoint("change material alpha");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.alpha_mode = mode;
        self.state.emit_material_changed();
        true
    }

    pub fn create_albedo_texture(&mut self) -> bool {
        let slot = self.active_material_slot.max(0) as usize;
        let Some(material) = self.state.project.project.materials.get(slot) else {
            return false;
        };
        let color = material.base_color;
        self.state.checkpoint("create albedo texture");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.albedo_texture = Some(petunia_project::Canvas::new(
            256,
            256,
            [
                (color[0].clamp(0.0, 1.0) * 255.0) as u8,
                (color[1].clamp(0.0, 1.0) * 255.0) as u8,
                (color[2].clamp(0.0, 1.0) * 255.0) as u8,
                255,
            ],
        ));
        self.state.emit_project_changed(
            petunia_project::ProjectChanges::MATERIALS | petunia_project::ProjectChanges::TEXTURES,
        );
        true
    }

    pub fn clear_albedo_texture(&mut self) -> bool {
        let slot = self.active_material_slot.max(0) as usize;
        if !self
            .state
            .project
            .project
            .materials
            .get(slot)
            .is_some_and(|material| material.albedo_texture.is_some())
        {
            return false;
        }
        self.state.checkpoint("clear albedo texture");
        let Some(material) = self.state.project.project.materials.get_mut(slot) else {
            return false;
        };
        material.albedo_texture = None;
        self.state.emit_project_changed(
            petunia_project::ProjectChanges::MATERIALS | petunia_project::ProjectChanges::TEXTURES,
        );
        true
    }

    pub fn toggle_quick_action(&mut self, id: &str) -> bool {
        let pinned = self
            .state
            .ui
            .model_quick_action_ids()
            .iter()
            .any(|item| item == id);
        self.state.ui.set_model_quick_action_pinned(id, !pinned)
    }

    pub fn reset_quick_actions(&mut self) -> bool {
        self.state.ui.reset_model_quick_actions()
    }

    pub fn execute_quick_action(&mut self, id: &str) -> bool {
        if !self
            .state
            .ui
            .model_quick_action_ids()
            .iter()
            .any(|item| item == id)
        {
            return false;
        }
        match id {
            "model.loop_cut" => {
                self.apply(UiIntent::SetActiveTool("loop_cut".into()));
                true
            }
            other => {
                if let Err(error) = self.execute_core_command(other) {
                    self.state.set_status(error.to_string());
                    false
                } else {
                    true
                }
            }
        }
    }

    /// Asset displayed by a section: the pinned asset when set and resolvable,
    /// otherwise the active selection. Fail-safe fallback, never panics.
    /// Asset exibido por uma seção: o asset fixado quando definido e resolvível,
    /// senão a seleção ativa. Fallback fail-safe, nunca pânico.
    pub fn section_asset(
        &self,
        section: petunia_config::InspectorSectionId,
    ) -> Option<&petunia_project::Asset> {
        self.section_layouts[section_layout::section_index(section)]
            .pinned_asset
            .as_deref()
            .and_then(|id| uuid::Uuid::parse_str(id).ok())
            .and_then(|id| {
                self.state
                    .project
                    .assets
                    .iter()
                    .find(|asset| asset.id == id)
            })
            .or_else(|| self.state.project.active())
    }

    /// Index of the asset owning a modifier id (any asset, not just the active
    /// one) so sections pinned to another asset act on the displayed stack.
    /// Modifier ids are unique; the active path behaves exactly as before.
    /// Índice do asset dono de um modifier (qualquer asset, não só o ativo)
    /// para seções fixadas agirem na pilha exibida. Ids são únicos; o caminho
    /// ativo comporta-se exatamente como antes.
    fn find_modifier_owner(&self, id: uuid::Uuid) -> Option<usize> {
        self.state
            .project
            .assets
            .iter()
            .position(|asset| asset.modifiers.iter().any(|modifier| modifier.id == id))
    }

    pub fn add_modifier(&mut self, kind: &str) -> bool {
        // Creation follows the displayed Modifiers section: pinned asset when
        // set, active selection otherwise. / A criação segue a seção exibida.
        let asset_index = self
            .section_asset(petunia_config::InspectorSectionId::Modifiers)
            .map(|asset| asset.id)
            .and_then(|id| {
                self.state
                    .project
                    .assets
                    .iter()
                    .position(|asset| asset.id == id)
            });
        let Some(asset_index) = asset_index else {
            return false;
        };
        if self.state.project.assets.get(asset_index).is_none() {
            return false;
        }
        let modifier = match kind {
            "mirror" => petunia_project::ModifierInstance::mirror(0, 0.001),
            "symmetry" => petunia_project::ModifierInstance::symmetry(0, true, 0.001),
            _ => return false,
        };
        self.state.checkpoint("add modifier");
        if let Some(asset) = self.state.project.assets.get_mut(asset_index) {
            asset.modifiers.push(modifier);
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn set_modifier_enabled(&mut self, id: &str, enabled: bool) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        let Some(asset) = self.state.project.assets.get_mut(asset_index) else {
            return false;
        };
        let Some(modifier) = asset.modifiers.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        if modifier.enabled == enabled {
            return false;
        }
        self.state.checkpoint("toggle modifier");
        if let Some(modifier) = self.state.project.assets[asset_index]
            .modifiers
            .iter_mut()
            .find(|item| item.id == id)
        {
            modifier.enabled = enabled;
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn remove_modifier(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        if !self.state.project.assets[asset_index]
            .modifiers
            .iter()
            .any(|item| item.id == id)
        {
            return false;
        }
        self.state.checkpoint("remove modifier");
        self.state.project.assets[asset_index]
            .modifiers
            .retain(|item| item.id != id);
        self.state.emit_mesh_changed();
        true
    }

    pub fn move_modifier(&mut self, id: &str, direction: i32) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        let Some(current) = self.state.project.assets[asset_index]
            .modifiers
            .iter()
            .position(|item| item.id == id)
        else {
            return false;
        };
        let target = if direction < 0 {
            if current == 0 {
                return false;
            }
            current - 1
        } else {
            current + 1
        };
        if target >= self.state.project.assets[asset_index].modifiers.len() {
            return false;
        }
        self.state.checkpoint("reorder modifier");
        self.state.project.assets[asset_index]
            .modifiers
            .swap(current, target);
        self.state.emit_mesh_changed();
        true
    }

    pub fn set_modifier_axis(&mut self, id: &str, axis: i32) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        let axis_value = (axis.clamp(0, 2)) as usize;
        self.state.checkpoint("change modifier axis");
        let Some(asset) = self.state.project.assets.get_mut(asset_index) else {
            return false;
        };
        let Some(modifier) = asset.modifiers.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        match &mut modifier.kind {
            petunia_project::ModifierKind::Mirror { axis, .. } => *axis = axis_value,
            petunia_project::ModifierKind::Symmetry { axis, .. } => *axis = axis_value,
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn set_modifier_direction(&mut self, id: &str, direction_value: bool) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        self.state.checkpoint("change modifier direction");
        let Some(asset) = self.state.project.assets.get_mut(asset_index) else {
            return false;
        };
        let Some(modifier) = asset.modifiers.iter_mut().find(|item| item.id == id) else {
            return false;
        };
        match &mut modifier.kind {
            petunia_project::ModifierKind::Symmetry {
                positive_to_negative,
                ..
            } => *positive_to_negative = direction_value,
            petunia_project::ModifierKind::Mirror { .. } => return false,
        }
        self.state.emit_mesh_changed();
        true
    }

    pub fn apply_modifier(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let Some(asset_index) = self.find_modifier_owner(id) else {
            return false;
        };
        let Some(asset) = self.state.project.assets.get(asset_index) else {
            return false;
        };
        if !asset.modifiers.iter().any(|item| item.id == id) {
            return false;
        }
        let evaluated = asset.evaluated_mesh();
        self.state.checkpoint("apply modifier");
        if let Some(asset) = self.state.project.assets.get_mut(asset_index) {
            asset.mesh = evaluated;
            asset.modifiers.retain(|item| item.id != id);
        }
        self.state.emit_mesh_changed();
        true
    }

    fn active_profile_resources(
        &self,
    ) -> Option<(
        &petunia_core::ProfileResource,
        &petunia_core::SplineResource,
    )> {
        let profile = self
            .active_profile_id
            .and_then(|id| self.state.project.project.get_profile(id))?;
        let spline = self.state.project.project.get_spline(profile.spline_id)?;
        Some((profile, spline))
    }

    fn active_profile_state(&self) -> Option<petunia_core::ProfileState> {
        let (profile, spline) = self.active_profile_resources()?;
        let mut state = self.state.profile.clone();
        state.points = spline
            .points
            .iter()
            .map(|point| [point.position[0] as f32, point.position[1] as f32])
            .collect();
        state.nodes = spline
            .points
            .iter()
            .map(|point| {
                let handle_in = [point.handle_in[0] as f32, point.handle_in[1] as f32];
                let handle_out = [point.handle_out[0] as f32, point.handle_out[1] as f32];
                petunia_mesh::curve::BezierNode {
                    point: [point.position[0] as f32, point.position[1] as f32],
                    handle_in: (handle_in != [0.0; 2]).then_some(handle_in),
                    handle_out: (handle_out != [0.0; 2]).then_some(handle_out),
                    kind: match point.handle_mode {
                        petunia_core::SplineHandleMode::Broken => {
                            petunia_mesh::curve::BezierNodeKind::Sharp
                        }
                        petunia_core::SplineHandleMode::Aligned => {
                            petunia_mesh::curve::BezierNodeKind::Smooth
                        }
                        petunia_core::SplineHandleMode::Mirrored => {
                            petunia_mesh::curve::BezierNodeKind::Symmetric
                        }
                    },
                }
            })
            .collect();
        state.origin = profile.workplane.origin.map(|value| value as f32);
        state.right = profile.workplane.right.map(|value| value as f32);
        state.up = profile.workplane.up.map(|value| value as f32);
        state.normal = profile.workplane.normal.map(|value| value as f32);
        state.closed = spline.closed;
        state.wall_thickness = profile.wall_thickness as f32;
        Some(state)
    }

    fn active_profile_point_count(&self) -> usize {
        self.active_profile_resources()
            .map_or(0, |(_, spline)| spline.points.len())
    }

    fn active_profile_closed(&self) -> bool {
        self.active_profile_resources()
            .is_some_and(|(_, spline)| spline.closed)
    }

    fn active_profile_has_curves(&self) -> bool {
        self.active_profile_resources().is_some_and(|(_, spline)| {
            spline.interpolation == petunia_core::SplineInterpolation::CubicBezier
                && spline
                    .points
                    .iter()
                    .any(|point| point.handle_in != [0.0; 3] || point.handle_out != [0.0; 3])
        })
    }

    /// Nome traduzido do plano de trabalho atual (Chão/Face/Vista).
    fn workplane_display_name(&self) -> String {
        use petunia_config::text_id as t;
        let id = match self.state.profile.workplane_kind {
            petunia_core::WorkplaneKind::Ground => t::UI_PROFILE_PLANE_GROUND,
            petunia_core::WorkplaneKind::Face => t::UI_PROFILE_PLANE_FACE,
            petunia_core::WorkplaneKind::View => t::UI_PROFILE_PLANE_VIEW,
        };
        self.state.t_id(id)
    }

    /// Trocar de plano encerra o perfil em edição (um perfil vive num plano).
    fn reset_profile_for_workplane(&mut self) {
        self.profile_pointer_up();
        self.cancel_profile_volume();
        self.active_profile_id = None;
        self.profile_selected_point = None;
        self.profile_hover_snap = None;
        self.state.mark_dirty();
    }

    fn announce_locked_workplane(&mut self) {
        let plane = self.workplane_display_name();
        let message = self
            .state
            .t_id(petunia_config::text_id::TOOL_GRAMMAR_WORKPLANE_SET)
            .replace("{plane}", &plane);
        self.state.set_status(message);
    }

    /// Plano automático no 1º clique de um perfil novo: a face sob o cursor
    /// (se houver) ou o plano do mundo mais paralelo à vista atual. A câmera
    /// não se move. Não faz nada com o plano travado ou com perfil em curso.
    fn resolve_auto_workplane_at(&mut self, ndc_x: f32, ndc_y: f32) {
        if self.state.profile.workplane_locked || self.active_profile_resources().is_some() {
            return;
        }
        let normalized = [(ndc_x + 1.0) * 0.5, (1.0 - ndc_y) * 0.5];
        let face = match self.pick_target_for_domain(
            SelectionDomain::Face,
            normalized[0],
            normalized[1],
        ) {
            petunia_core::HoverTarget::Face(face) => Some(face),
            _ => None,
        };
        let captured = face.is_some_and(|face| {
            petunia_module_model::profile_capture_face_index(&mut self.state, face)
        });
        if !captured {
            petunia_module_model::profile_capture_world_plane_for_view(&mut self.state);
        }
    }

    fn draft_profile_workplane(&self) -> petunia_core::ProfileWorkplane {
        petunia_core::ProfileWorkplane {
            origin: self.state.profile.origin.map(f64::from),
            right: self.state.profile.right.map(f64::from),
            up: self.state.profile.up.map(f64::from),
            normal: self.state.profile.normal.map(f64::from),
        }
        .try_normalized()
        .unwrap_or_default()
    }

    fn profile_screen_to_plane(&self, ndc_x: f32, ndc_y: f32) -> Option<[f64; 2]> {
        self.profile_screen_to_plane_snapped(ndc_x, ndc_y)
            .map(|(point, _)| point)
    }

    /// Ponto no plano do perfil e o alvo de snap que o produziu, se houver.
    fn profile_screen_to_plane_snapped(
        &self,
        ndc_x: f32,
        ndc_y: f32,
    ) -> Option<([f64; 2], Option<petunia_core::ScreenSnapHit>)> {
        let workplane = self.active_profile_resources().map_or_else(
            || self.draft_profile_workplane(),
            |(profile, _)| profile.workplane,
        );
        let (ray_origin, ray_direction) = self.state.session.camera.ray(ndc_x, ndc_y);
        let normal = glam::Vec3::from_array(workplane.normal.map(|value| value as f32));
        let denominator = ray_direction.dot(normal);
        if denominator.abs() < 1.0e-6 {
            return None;
        }
        let origin = glam::Vec3::from_array(workplane.origin.map(|value| value as f32));
        let distance = (origin - ray_origin).dot(normal) / denominator;
        if distance < 0.0 && self.state.session.camera.proj == petunia_core::Projection::Perspective
        {
            return None;
        }
        let right = glam::Vec3::from_array(workplane.right.map(|value| value as f32));
        let up = glam::Vec3::from_array(workplane.up.map(|value| value as f32));
        let mut hit = ray_origin + ray_direction * distance;
        let mut snap_hit = None;
        if self.state.session.snap_enabled || self.state.profile.snap {
            // Mesma passada de snap das transformações (P3D-040): pontos e
            // arestas da malha, guias paralelas aos eixos do plano a partir do
            // último ponto e, por último, a grade do plano de trabalho.
            let to_world = |p: [f64; 3]| origin + right * p[0] as f32 + up * p[1] as f32;
            let anchor = self
                .active_profile_resources()
                .and_then(|(_, spline)| spline.points.last())
                .map(|last| petunia_core::SnapAnchor {
                    point: to_world(last.position),
                    axes: [Some(right), Some(up), None],
                });
            let settings = &self.state.session.snap_settings;
            let cursor = glam::Vec2::new(ndc_x + 1.0, 1.0 - ndc_y)
                * glam::Vec2::from_array(self.viewport_size)
                * 0.5;
            let grid = petunia_core::SnapGrid {
                origin,
                right,
                up,
                spacing: settings.grid_spacing,
            };
            let mut mask = petunia_core::SnapMask::for_target(settings.target);
            mask.grid = true;
            if let Some(snapped) = self.screen_snap(cursor, mask, anchor, Some(grid)) {
                hit = snapped.point;
                snap_hit = Some(snapped);
            }
        }
        // Pontos fora do plano (arestas de outra face) são projetados nele.
        let offset = hit - origin;
        let point = [f64::from(offset.dot(right)), f64::from(offset.dot(up))];
        // O marcador mostra o ponto que realmente entra no perfil (projetado).
        let snap_hit = snap_hit.map(|snapped| petunia_core::ScreenSnapHit {
            point: origin + right * point[0] as f32 + up * point[1] as f32,
            ..snapped
        });
        Some((point, snap_hit))
    }

    fn update_profile_handle(
        &mut self,
        point_id: uuid::Uuid,
        incoming: bool,
        cursor: [f64; 2],
        break_tangent: bool,
        new_node_drag: bool,
    ) -> bool {
        let Some(spline_id) = self
            .profile_edit_gesture
            .as_ref()
            .map(|gesture| gesture.spline_id)
        else {
            return false;
        };
        let Some(spline) = self.state.project.project.get_spline_mut(spline_id) else {
            return false;
        };
        let Some(point) = spline.point(point_id).cloned() else {
            return false;
        };
        let handle = [
            cursor[0] - point.position[0],
            cursor[1] - point.position[1],
            0.0,
        ];
        if handle[0].hypot(handle[1]) <= 0.02 {
            return false;
        }
        let mode = if break_tangent {
            petunia_core::SplineHandleMode::Broken
        } else if new_node_drag {
            petunia_core::SplineHandleMode::Mirrored
        } else {
            point.handle_mode
        };
        let (mut handle_in, mut handle_out) = (point.handle_in, point.handle_out);
        if incoming {
            handle_in = handle;
            match mode {
                petunia_core::SplineHandleMode::Broken => {}
                petunia_core::SplineHandleMode::Mirrored => {
                    handle_out = handle.map(|component| -component);
                }
                petunia_core::SplineHandleMode::Aligned => {
                    let length = glam::DVec3::from_array(handle_out)
                        .length()
                        .max(glam::DVec3::from_array(handle).length());
                    handle_out =
                        (-glam::DVec3::from_array(handle).normalize_or_zero() * length).to_array();
                }
            }
        } else {
            handle_out = handle;
            match mode {
                petunia_core::SplineHandleMode::Broken => {}
                petunia_core::SplineHandleMode::Mirrored => {
                    handle_in = handle.map(|component| -component);
                }
                petunia_core::SplineHandleMode::Aligned => {
                    let length = glam::DVec3::from_array(handle_in)
                        .length()
                        .max(glam::DVec3::from_array(handle).length());
                    handle_in =
                        (-glam::DVec3::from_array(handle).normalize_or_zero() * length).to_array();
                }
            }
        }
        if spline
            .set_handles(point_id, handle_in, handle_out, mode)
            .is_err()
        {
            return false;
        }
        spline.interpolation = petunia_core::SplineInterpolation::CubicBezier;
        true
    }

    fn add_profile_point(&mut self, ndc_x: f32, ndc_y: f32) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        self.resolve_auto_workplane_at(ndc_x, ndc_y);
        let Some(position) = self.profile_screen_to_plane(ndc_x, ndc_y) else {
            return false;
        };
        if self.active_profile_closed() {
            return false;
        }
        if self.active_profile_point_count() >= 512 {
            self.state.set_status("max 512 pts");
            return false;
        }

        let point = petunia_core::SplinePoint::new([position[0], position[1], 0.0]);
        let point_id = point.id;
        let (spline_id, revision_before) =
            if let Some((_, spline)) = self.active_profile_resources() {
                let spline_id = spline.id;
                let revision_before = spline.revision;
                if let Err(error) = self.state.dispatch(&petunia_core::AddSplinePointCmd {
                    spline_id,
                    index: None,
                    point,
                }) {
                    self.state.set_status(error.to_string());
                    return false;
                }
                (spline_id, revision_before)
            } else {
                let mut spline = petunia_core::SplineResource::new(
                    "Profile curve",
                    petunia_core::SplineInterpolation::CubicBezier,
                );
                if spline.add_point(point).is_err() {
                    return false;
                }
                let mut profile = petunia_core::ProfileResource::new(
                    "Profile",
                    spline.id,
                    self.draft_profile_workplane(),
                );
                profile.wall_thickness = f64::from(self.state.profile.wall_thickness.max(0.0));
                let profile_id = profile.id;
                let spline_id = spline.id;
                if let Err(error) = self
                    .state
                    .dispatch(&petunia_core::CreateProfileCmd { spline, profile })
                {
                    self.state.set_status(error.to_string());
                    return false;
                }
                self.active_profile_id = Some(profile_id);
                (spline_id, 0)
            };

        self.profile_selected_point = Some(point_id);
        self.profile_drag_target = Some(ProfileHitTarget::HandleOut(point_id));
        self.profile_edit_gesture = Some(ProfileEditGesture {
            spline_id,
            point_id,
            point_before: None,
            revision_before,
        });
        true
    }

    fn profile_preview_commands(&self) -> String {
        let Some(profile) = self.active_profile_state() else {
            return String::new();
        };
        let points = profile.tessellated_points();
        if points.is_empty() || self.viewport_size[0] <= 1.0 || self.viewport_size[1] <= 1.0 {
            return String::new();
        }
        let matrix = self.state.session.camera.view_proj();
        let project_pt = |p: [f32; 2]| -> Option<[f32; 2]> {
            let pt_3d = profile.to_3d_point(p);
            let point = matrix * pt_3d.extend(1.0);
            if !point.is_finite() || point.w <= 0.05 || point.z < 0.0 || point.z > point.w {
                None
            } else {
                Some([
                    (point.x / point.w * 0.5 + 0.5) * self.viewport_size[0],
                    (0.5 - point.y / point.w * 0.5) * self.viewport_size[1],
                ])
            }
        };

        let mut commands = String::new();
        use std::fmt::Write as _;

        // Loop externo
        let mut first = true;
        for &pt in &points {
            if let Some(screen_pt) = project_pt(pt) {
                let action = if first {
                    first = false;
                    'M'
                } else {
                    'L'
                };
                let _ = write!(
                    commands,
                    "{action} {:.2} {:.2} ",
                    screen_pt[0], screen_pt[1]
                );
            }
        }

        if profile.closed {
            commands.push_str("Z ");

            // Se for oco (wall_thickness > 0), desenha também o laço interno no preview
            if profile.wall_thickness > 0.0 && points.len() >= 3 {
                let inner = profile.inner_points();
                let mut first_inner = true;
                for &pt in &inner {
                    if let Some(screen_pt) = project_pt(pt) {
                        let action = if first_inner {
                            first_inner = false;
                            'M'
                        } else {
                            'L'
                        };
                        let _ = write!(
                            commands,
                            "{action} {:.2} {:.2} ",
                            screen_pt[0], screen_pt[1]
                        );
                    }
                }
                if !first_inner {
                    commands.push_str("Z ");
                }
            }
        } else {
            let [x, y] = self.pointer_position;
            let _ = write!(commands, "L {:.2} {:.2} ", x, y);
        }

        // Alças tangentes e marcadores de nós Bézier
        let Some((_, spline)) = self.active_profile_resources() else {
            return String::new();
        };
        for (node, spline_point) in profile.nodes.iter().zip(&spline.points) {
            let p_anchor = project_pt(node.point);
            let is_selected = self.profile_selected_point == Some(spline_point.id);

            if let (Some(anchor_scr), Some(h_out)) = (p_anchor, node.handle_out) {
                let out_pt = [node.point[0] + h_out[0], node.point[1] + h_out[1]];
                if let Some(out_scr) = project_pt(out_pt) {
                    let _ = write!(
                        commands,
                        "M {:.2} {:.2} L {:.2} {:.2} ",
                        anchor_scr[0], anchor_scr[1], out_scr[0], out_scr[1]
                    );
                    let _ = write!(
                        commands,
                        "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                        out_scr[0] - 2.5,
                        out_scr[1] - 2.5,
                        out_scr[0] + 2.5,
                        out_scr[1] - 2.5,
                        out_scr[0] + 2.5,
                        out_scr[1] + 2.5,
                        out_scr[0] - 2.5,
                        out_scr[1] + 2.5
                    );
                }
            }
            if let (Some(anchor_scr), Some(h_in)) = (p_anchor, node.handle_in) {
                let in_pt = [node.point[0] + h_in[0], node.point[1] + h_in[1]];
                if let Some(in_scr) = project_pt(in_pt) {
                    let _ = write!(
                        commands,
                        "M {:.2} {:.2} L {:.2} {:.2} ",
                        anchor_scr[0], anchor_scr[1], in_scr[0], in_scr[1]
                    );
                    let _ = write!(
                        commands,
                        "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                        in_scr[0] - 2.5,
                        in_scr[1] - 2.5,
                        in_scr[0] + 2.5,
                        in_scr[1] - 2.5,
                        in_scr[0] + 2.5,
                        in_scr[1] + 2.5,
                        in_scr[0] - 2.5,
                        in_scr[1] + 2.5
                    );
                }
            }

            // Marcador do ponto âncora (destacado se selecionado)
            if let Some(anchor_scr) = p_anchor {
                let s = if is_selected { 5.0 } else { 3.0 };
                let _ = write!(
                    commands,
                    "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                    anchor_scr[0] - s,
                    anchor_scr[1] - s,
                    anchor_scr[0] + s,
                    anchor_scr[1] - s,
                    anchor_scr[0] + s,
                    anchor_scr[1] + s,
                    anchor_scr[0] - s,
                    anchor_scr[1] + s
                );
            }
        }

        commands
    }

    pub fn close_profile(&mut self) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" || self.active_profile_closed() {
            return false;
        }
        let Some((_, spline)) = self.active_profile_resources() else {
            self.state
                .set_status("Profile requires at least three points");
            return false;
        };
        if spline.points.len() < 3 {
            self.state
                .set_status("Profile requires at least three points");
            return false;
        }
        let spline_id = spline.id;
        if let Err(error) = self.state.dispatch(&petunia_core::SetSplineClosedCmd {
            spline_id,
            closed: true,
        }) {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state
            .set_status("Profile closed: choose Generate Volume or Revolve");
        true
    }

    pub fn set_pivot_menu_open(&mut self, open: bool) -> bool {
        self.shading_popover_open = false;
        if open {
            self.close_menu();
            self.close_context_menu();
            self.overlays.push(OverlayEntry {
                id: OverlayId::PivotMenu,
                kind: OverlayKind::Popover,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            });
            self.pivot_menu_open = true;
            self.state.set_status("Choose a transform pivot");
        } else if self.overlays.remove(OverlayId::PivotMenu).is_some() {
            self.pivot_menu_open = false;
        }
        self.pivot_menu_open == open
    }

    pub fn set_profile_depth(&mut self, depth: f32) -> bool {
        if !depth.is_finite()
            || depth <= 0.0
            || self.state.session.tools.active_tool != "draw_profile"
        {
            return false;
        }
        self.state.profile.depth = depth.clamp(0.01, 1000.0);
        self.state.mark_dirty();
        if self.active_profile_closed() {
            if self.profile_volume_mode.is_none() {
                self.enter_profile_volume("extrude");
            } else if self.profile_volume_mode
                == Some(petunia_module_model::ProfileVolumeMode::Extrude)
            {
                self.update_profile_volume_preview();
            }
        }
        true
    }

    pub fn set_profile_wall_thickness(&mut self, thickness: f32) -> bool {
        if !thickness.is_finite()
            || thickness < 0.0
            || self.state.session.tools.active_tool != "draw_profile"
        {
            return false;
        }
        let thickness = thickness.clamp(0.0, 1000.0);
        self.state.profile.wall_thickness = thickness;
        if let Some((profile, _)) = self.active_profile_resources() {
            let mut profile = profile.clone();
            if (profile.wall_thickness - f64::from(thickness)).abs() <= f64::EPSILON {
                return false;
            }
            profile.wall_thickness = f64::from(thickness);
            if let Err(error) = self
                .state
                .dispatch(&petunia_core::UpdateProfileCmd { profile })
            {
                self.state.set_status(error.to_string());
                return false;
            }
        } else {
            self.state.mark_dirty();
        }
        if self.profile_volume_mode == Some(petunia_module_model::ProfileVolumeMode::Extrude) {
            self.update_profile_volume_preview();
        }
        true
    }

    pub fn set_profile_revolve_angle(&mut self, angle_deg: f32) -> bool {
        if !angle_deg.is_finite() || angle_deg <= 0.0 {
            return false;
        }
        self.state.profile.revolve_angle = angle_deg.clamp(1.0, 360.0);
        self.state.mark_dirty();
        if self.profile_volume_mode == Some(petunia_module_model::ProfileVolumeMode::Revolve) {
            self.update_profile_volume_preview();
        }
        true
    }

    pub fn set_profile_smoothness(&mut self, smoothness: f32) -> bool {
        if !smoothness.is_finite()
            || smoothness <= 0.0
            || self.state.session.tools.active_tool != "draw_profile"
        {
            return false;
        }
        petunia_module_model::profile_set_curve_smoothness(&mut self.state, smoothness);
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }

    pub fn profile_smooth_curves(&mut self) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        let Some((_, spline)) = self.active_profile_resources() else {
            return false;
        };
        let mut updated = spline.clone();
        let mut path = petunia_mesh::curve::BezierPath {
            nodes: self
                .active_profile_state()
                .map_or_else(Vec::new, |profile| profile.nodes),
            closed: spline.closed,
        };
        path.auto_smooth(0.25);
        updated.interpolation = petunia_core::SplineInterpolation::CubicBezier;
        for (point, node) in updated.points.iter_mut().zip(path.nodes) {
            let handle_in = node.handle_in.unwrap_or([0.0; 2]);
            let handle_out = node.handle_out.unwrap_or([0.0; 2]);
            point.handle_in = [f64::from(handle_in[0]), f64::from(handle_in[1]), 0.0];
            point.handle_out = [f64::from(handle_out[0]), f64::from(handle_out[1]), 0.0];
            point.handle_mode = petunia_core::SplineHandleMode::Aligned;
        }
        if let Err(error) = self
            .state
            .dispatch(&petunia_core::UpdateSplineCmd { spline: updated })
        {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state
            .set_status("Profile curves smoothed (Cubic Bézier)");
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }

    pub fn profile_clear_curves(&mut self) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        let Some((_, spline)) = self.active_profile_resources() else {
            return false;
        };
        let mut updated = spline.clone();
        updated.interpolation = petunia_core::SplineInterpolation::Polyline;
        for point in &mut updated.points {
            point.handle_in = [0.0; 3];
            point.handle_out = [0.0; 3];
            point.handle_mode = petunia_core::SplineHandleMode::Broken;
        }
        if let Err(error) = self
            .state
            .dispatch(&petunia_core::UpdateSplineCmd { spline: updated })
        {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state.set_status("Profile corners sharpened");
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }

    pub fn profile_update_drag_handle(
        &mut self,
        norm_x: f32,
        norm_y: f32,
        break_tangent: bool,
    ) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        let Some(gesture) = self.profile_edit_gesture.as_ref() else {
            return false;
        };
        let point_id = gesture.point_id;
        let ndc_x = norm_x.clamp(0.0, 1.0) * 2.0 - 1.0;
        let ndc_y = 1.0 - norm_y.clamp(0.0, 1.0) * 2.0;
        let Some(point) = self.profile_screen_to_plane(ndc_x, ndc_y) else {
            return false;
        };
        let updated = self.update_profile_handle(point_id, false, point, break_tangent, true);
        if updated {
            self.state.emit_project_changed(ProjectChanges::SPLINES);
            if self.profile_volume_mode.is_some() {
                self.update_profile_volume_preview();
            }
        }
        updated
    }

    /// Realiza hit-testing em screen space contra âncoras e alças Bézier do perfil ativo.
    pub fn hit_test_profile(&self, screen_x: f32, screen_y: f32) -> Option<ProfileHitTarget> {
        if self.state.session.tools.active_tool != "draw_profile" {
            return None;
        }
        let profile = self.active_profile_state()?;
        let (_, spline) = self.active_profile_resources()?;
        if profile.points.is_empty() || self.viewport_size[0] <= 1.0 || self.viewport_size[1] <= 1.0
        {
            return None;
        }
        let matrix = self.state.session.camera.view_proj();
        let project_pt = |p: [f32; 2]| -> Option<[f32; 2]> {
            let pt_3d = profile.to_3d_point(p);
            let point = matrix * pt_3d.extend(1.0);
            if !point.is_finite() || point.w <= 0.05 || point.z < 0.0 || point.z > point.w {
                None
            } else {
                Some([
                    (point.x / point.w * 0.5 + 0.5) * self.viewport_size[0],
                    (0.5 - point.y / point.w * 0.5) * self.viewport_size[1],
                ])
            }
        };

        let hit_radius = 16.0f32;
        let mut best_target = None;
        let mut best_dist = hit_radius;

        for (node, spline_point) in profile.nodes.iter().zip(&spline.points) {
            // Test handle_out
            if let Some(h_out) = node.handle_out {
                let out_pt = [node.point[0] + h_out[0], node.point[1] + h_out[1]];
                if let Some(scr) = project_pt(out_pt) {
                    let dist = (scr[0] - screen_x).hypot(scr[1] - screen_y);
                    if dist < best_dist {
                        best_dist = dist;
                        best_target = Some(ProfileHitTarget::HandleOut(spline_point.id));
                    }
                }
            }
            // Test handle_in
            if let Some(h_in) = node.handle_in {
                let in_pt = [node.point[0] + h_in[0], node.point[1] + h_in[1]];
                if let Some(scr) = project_pt(in_pt) {
                    let dist = (scr[0] - screen_x).hypot(scr[1] - screen_y);
                    if dist < best_dist {
                        best_dist = dist;
                        best_target = Some(ProfileHitTarget::HandleIn(spline_point.id));
                    }
                }
            }
            // Test anchor point
            if let Some(scr) = project_pt(node.point) {
                let dist = (scr[0] - screen_x).hypot(scr[1] - screen_y);
                if dist < best_dist {
                    best_dist = dist;
                    best_target = Some(ProfileHitTarget::Anchor(spline_point.id));
                }
            }
        }

        best_target
    }

    /// Processa clique inicial do ponteiro para selecionar e iniciar arraste de nós/alças do perfil.
    pub fn profile_pointer_down(&mut self, screen_x: f32, screen_y: f32, alt: bool) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        if let Some(target) = self.hit_test_profile(screen_x, screen_y) {
            let point_id = match target {
                ProfileHitTarget::Anchor(id)
                | ProfileHitTarget::HandleOut(id)
                | ProfileHitTarget::HandleIn(id) => id,
            };
            let Some((spline_id, spline_closed, point_count, point_index, point_before, revision)) =
                self.active_profile_resources().and_then(|(_, spline)| {
                    let point_index = spline
                        .points
                        .iter()
                        .position(|point| point.id == point_id)?;
                    Some((
                        spline.id,
                        spline.closed,
                        spline.points.len(),
                        point_index,
                        spline.point(point_id).cloned(),
                        spline.revision,
                    ))
                })
            else {
                return false;
            };
            match target {
                ProfileHitTarget::Anchor(id) => {
                    // Fecha perfil se clicar no primeiro nó de um perfil aberto com >= 3 pontos
                    if !spline_closed && point_count >= 3 && point_index == 0 {
                        self.profile_selected_point = Some(id);
                        self.profile_drag_target = None;
                        return self.close_profile();
                    }
                    self.profile_selected_point = Some(id);
                    if alt {
                        self.profile_drag_target = Some(ProfileHitTarget::HandleOut(id));
                    } else {
                        self.profile_drag_target = Some(ProfileHitTarget::Anchor(id));
                    }
                }
                ProfileHitTarget::HandleOut(id) => {
                    self.profile_selected_point = Some(id);
                    self.profile_drag_target = Some(ProfileHitTarget::HandleOut(id));
                }
                ProfileHitTarget::HandleIn(id) => {
                    self.profile_selected_point = Some(id);
                    self.profile_drag_target = Some(ProfileHitTarget::HandleIn(id));
                }
            }
            self.profile_edit_gesture = Some(ProfileEditGesture {
                spline_id,
                point_id,
                point_before,
                revision_before: revision,
            });
            self.state.mark_dirty();
            return true;
        }
        self.profile_drag_target = None;
        self.profile_selected_point = None;
        self.profile_edit_gesture = None;
        self.state.mark_dirty();
        false
    }

    /// Processa movimento do ponteiro durante o arraste de um nó ou alça.
    pub fn profile_pointer_move(&mut self, screen_x: f32, screen_y: f32, alt: bool) -> bool {
        let Some(target) = self.profile_drag_target else {
            return false;
        };
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        let width = self.viewport_size[0].max(1.0);
        let height = self.viewport_size[1].max(1.0);
        let nx = (screen_x / width).clamp(0.0, 1.0);
        let ny = (screen_y / height).clamp(0.0, 1.0);
        let ndc_x = nx * 2.0 - 1.0;
        let ndc_y = 1.0 - ny * 2.0;

        let Some(point) = self.profile_screen_to_plane(ndc_x, ndc_y) else {
            return false;
        };

        match target {
            ProfileHitTarget::Anchor(point_id) => {
                let Some(gesture) = self.profile_edit_gesture.as_ref() else {
                    return false;
                };
                let Some(spline) = self.state.project.project.get_spline_mut(gesture.spline_id)
                else {
                    return false;
                };
                if spline
                    .move_point(point_id, [point[0], point[1], 0.0])
                    .is_err()
                {
                    return false;
                }
            }
            ProfileHitTarget::HandleOut(point_id) => {
                if !self.update_profile_handle(point_id, false, point, alt, false) {
                    return false;
                }
            }
            ProfileHitTarget::HandleIn(point_id) => {
                if !self.update_profile_handle(point_id, true, point, alt, false) {
                    return false;
                }
            }
        }
        self.state.emit_project_changed(ProjectChanges::SPLINES);
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }

    /// Conclui o arraste de um nó ou alça.
    pub fn profile_pointer_up(&mut self) {
        let target = self.profile_drag_target.take();
        let Some(gesture) = self.profile_edit_gesture.take() else {
            return;
        };
        let Some(point_before) = gesture.point_before else {
            return;
        };
        let Some(point_after) = self
            .state
            .project
            .project
            .get_spline(gesture.spline_id)
            .and_then(|spline| spline.point(gesture.point_id))
            .cloned()
        else {
            return;
        };
        let Some(spline) = self.state.project.project.get_spline_mut(gesture.spline_id) else {
            return;
        };
        if let Some(point) = spline.point_mut(gesture.point_id) {
            *point = point_before.clone();
        }
        spline.revision = gesture.revision_before;

        let result = match target {
            Some(ProfileHitTarget::Anchor(_)) if point_after.position != point_before.position => {
                self.state.dispatch(&petunia_core::MoveSplinePointCmd {
                    spline_id: gesture.spline_id,
                    point_id: gesture.point_id,
                    position: point_after.position,
                })
            }
            Some(ProfileHitTarget::HandleOut(_) | ProfileHitTarget::HandleIn(_))
                if point_after.handle_in != point_before.handle_in
                    || point_after.handle_out != point_before.handle_out
                    || point_after.handle_mode != point_before.handle_mode =>
            {
                self.state.dispatch(&petunia_core::SetSplineHandlesCmd {
                    spline_id: gesture.spline_id,
                    point_id: gesture.point_id,
                    handle_in: point_after.handle_in,
                    handle_out: point_after.handle_out,
                    mode: point_after.handle_mode,
                })
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.state.set_status(error.to_string());
        }
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
    }

    pub fn enter_profile_volume(&mut self, mode_str: &str) -> bool {
        if self.state.session.tools.active_tool != "draw_profile" {
            return false;
        }
        self.profile_pointer_up();
        let Some(profile_state) = self.active_profile_state() else {
            self.state
                .set_status("Draw a profile before generating volume");
            return false;
        };
        let mode = match mode_str {
            "extrude" => {
                if !profile_state.closed && profile_state.points.len() >= 3 && !self.close_profile()
                {
                    return false;
                }
                if !self.active_profile_closed() {
                    self.state
                        .set_status("Extrude requires a closed profile (at least 3 points)");
                    return false;
                }
                petunia_module_model::ProfileVolumeMode::Extrude
            }
            "revolve" => {
                if profile_state.points.len() < 2 {
                    self.state.set_status("Revolve requires at least 2 points");
                    return false;
                }
                petunia_module_model::ProfileVolumeMode::Revolve
            }
            "sweep" => {
                if profile_state.points.len() < 2 {
                    self.state.set_status("Sweep requires at least 2 points");
                    return false;
                }
                petunia_module_model::ProfileVolumeMode::Sweep
            }
            _ => return false,
        };

        if self.profile_volume_mode.is_none() {
            self.state.project.project.history_selection =
                self.state.session.selection.assets.clone();
            self.profile_volume_original = Some(self.state.project.project.clone());
        }
        self.profile_volume_mode = Some(mode);
        self.update_profile_volume_preview();
        self.state.set_status(match mode {
            petunia_module_model::ProfileVolumeMode::Extrude => "Interactive Extrude: drag vertically to adjust depth, Enter to confirm, Esc to cancel",
            petunia_module_model::ProfileVolumeMode::Revolve => "Interactive Revolve: drag horizontally to adjust angle, Enter to confirm, Esc to cancel",
            petunia_module_model::ProfileVolumeMode::Sweep => "Interactive Sweep: Enter to confirm, Esc to cancel",
        });
        true
    }

    pub fn update_profile_volume_preview(&mut self) -> bool {
        let Some(mode) = self.profile_volume_mode else {
            return false;
        };
        let Some(profile) = self.active_profile_state() else {
            return false;
        };

        let mesh_res = match mode {
            petunia_module_model::ProfileVolumeMode::Extrude => {
                petunia_module_model::build_extrude_mesh(&profile)
            }
            petunia_module_model::ProfileVolumeMode::Revolve => {
                petunia_module_model::build_revolve_mesh(&profile)
            }
            petunia_module_model::ProfileVolumeMode::Sweep => {
                petunia_module_model::build_sweep_mesh(&profile, self.state.project.active_mesh())
            }
        };

        match mesh_res {
            Ok(mesh) => {
                if let Some(id) = self.profile_preview_asset_id {
                    if let Some(asset) = self.state.project.assets.iter_mut().find(|a| a.id == id) {
                        asset.mesh = mesh;
                    } else {
                        self.state.project.add("Profile Preview", mesh);
                        self.profile_preview_asset_id =
                            self.state.project.assets.last().map(|a| a.id);
                    }
                } else {
                    self.state.project.add("Profile Preview", mesh);
                    self.profile_preview_asset_id = self.state.project.assets.last().map(|a| a.id);
                }
                self.state.sync_selection();
                self.state.emit_mesh_changed();
                self.state.mark_dirty();
                true
            }
            Err(e) => {
                self.state.set_status(format!("Volume preview error: {e}"));
                false
            }
        }
    }

    pub fn commit_profile_volume(&mut self) -> bool {
        if self.profile_volume_mode.is_none() {
            return false;
        }
        let had_preview = self.profile_preview_asset_id.is_some();
        let Some(original) = self.profile_volume_original.take() else {
            self.state
                .set_status("Profile volume transaction has no initial snapshot");
            return false;
        };

        if let Some(id) = self.profile_preview_asset_id.take() {
            if let Some(asset) = self.state.project.assets.iter_mut().find(|a| a.id == id) {
                asset.name = "Profile".to_string();
            }
        } else if let Some(mode) = self.profile_volume_mode {
            let Some(profile) = self.active_profile_state() else {
                return false;
            };
            let mesh_res = match mode {
                petunia_module_model::ProfileVolumeMode::Extrude => {
                    petunia_module_model::build_extrude_mesh(&profile)
                }
                petunia_module_model::ProfileVolumeMode::Revolve => {
                    petunia_module_model::build_revolve_mesh(&profile)
                }
                petunia_module_model::ProfileVolumeMode::Sweep => {
                    petunia_module_model::build_sweep_mesh(
                        &profile,
                        self.state.project.active_mesh(),
                    )
                }
            };
            if let Ok(mesh) = mesh_res {
                self.state.project.add("Profile", mesh);
            }
        }
        self.state
            .project
            .checkpoint_snapshot("draw profile", &original);

        self.state.profile.clear();
        self.active_profile_id = None;
        self.profile_selected_point = None;
        self.profile_drag_target = None;
        self.profile_edit_gesture = None;
        self.profile_volume_mode = None;
        self.state.set_selection_domain(SelectionDomain::Object);
        self.sync_viewport_context();
        self.state.session.tools.active_tool = "select".to_string();
        self.state.sync_selection();
        if !had_preview {
            self.state.emit_mesh_changed();
        }
        self.state.mark_dirty();
        self.state.set_status(self.state.t("profile.generated"));
        true
    }

    pub fn cancel_profile_volume(&mut self) -> bool {
        let had_preview =
            self.profile_preview_asset_id.is_some() || self.profile_volume_mode.is_some();
        self.profile_preview_asset_id = None;
        if let Some(mut original) = self.profile_volume_original.take() {
            let revision_clock = self.state.project.project.revision_clock();
            original.rebase_revisions_after_restore(revision_clock);
            self.state.session.selection.assets = original.history_selection.clone();
            self.state.project.project = original;
            self.state.sync_selection();
            self.state.emit_mesh_changed();
        }
        self.profile_volume_mode = None;
        if had_preview {
            self.state
                .set_status("Geração de volume cancelada. Perfil 2D mantido.");
        }
        had_preview
    }

    pub fn generate_profile_extrude(&mut self) -> bool {
        if self.enter_profile_volume("extrude") {
            self.commit_profile_volume()
        } else {
            false
        }
    }

    pub fn generate_profile_revolve(&mut self) -> bool {
        if self.enter_profile_volume("revolve") {
            self.commit_profile_volume()
        } else {
            false
        }
    }

    pub fn generate_profile_sweep(&mut self) -> bool {
        if self.enter_profile_volume("sweep") {
            self.commit_profile_volume()
        } else {
            false
        }
    }

    pub fn adjust_loop_cut_hover_count(&mut self, delta: i32) -> bool {
        if self.state.session.tools.active_tool != "loop_cut"
            || self.loop_cut.is_some()
            || self.loop_cut_hover_ring.is_none()
        {
            return false;
        }
        let next = (self.loop_cut_hover_cuts as i32 + delta).clamp(1, 32) as usize;
        if next == self.loop_cut_hover_cuts {
            return false;
        }
        self.loop_cut_hover_cuts = next;
        self.state
            .set_status(format!("Loop Cut: {next} cut(s) · click to place"));
        true
    }

    pub fn place_loop_cut_hover(&mut self) -> bool {
        if self.loop_cut.is_some() || self.state.session.tools.active_tool != "loop_cut" {
            return false;
        }
        let (Some(ring), Some(source)) = (
            self.loop_cut_hover_ring.take(),
            self.loop_cut_hover_source.take(),
        ) else {
            self.state
                .set_status("Loop Cut: move the pointer over a quad edge ring");
            return false;
        };
        self.begin_loop_cut_from_ring(ring, source, self.loop_cut_hover_cuts)
    }

    pub fn update_loop_cut_hover(&mut self, x: f32, y: f32) -> bool {
        if self.state.session.tools.active_tool != "loop_cut" || self.loop_cut.is_some() {
            return false;
        }
        if !x.is_finite()
            || !y.is_finite()
            || self.viewport_size[0] <= 1.0
            || self.viewport_size[1] <= 1.0
        {
            return false;
        }
        self.pointer_position = [x, y];
        self.hover_component(x / self.viewport_size[0], y / self.viewport_size[1])
    }

    /// A preselection e o clique usam exatamente o mesmo hit test. Ponto e
    /// aresta usam alvos em pixels lógicos, independentes da distância da
    /// câmera; objetos e faces usam a superfície real, nunca uma esfera de
    /// bounding que seleciona no vazio.
    fn pick_viewport_target(&self, x: f32, y: f32) -> petunia_core::HoverTarget {
        self.pick_target_for_domain(self.state.selection_domain(), x, y)
    }

    fn pick_target_for_domain(
        &self,
        domain: SelectionDomain,
        x: f32,
        y: f32,
    ) -> petunia_core::HoverTarget {
        use petunia_core::HoverTarget as Target;

        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..=1.0).contains(&x)
            || !(0.0..=1.0).contains(&y)
        {
            return Target::None;
        }
        let (width, height) = (self.viewport_size[0], self.viewport_size[1]);
        if width <= 1.0 || height <= 1.0 {
            return Target::None;
        }
        let camera = &self.state.session.camera;
        let scene =
            petunia_core::viewport_query::ViewportSceneQuery::new(&self.state.project.project);
        let ndc = [x * 2.0 - 1.0, 1.0 - y * 2.0];
        if domain == SelectionDomain::Object {
            return scene
                .nearest_object(camera, ndc)
                .map_or(Target::None, Target::Object);
        }
        let Some(asset) = self
            .state
            .project
            .active()
            .filter(|asset| asset.visible && !asset.locked)
        else {
            return Target::None;
        };
        let mode = match domain {
            SelectionDomain::Vertex => petunia_core::SelectMode::Vertex,
            SelectionDomain::Edge => petunia_core::SelectMode::Edge,
            SelectionDomain::Face => petunia_core::SelectMode::Face,
            SelectionDomain::Object => return Target::None,
        };
        let through = self.state.session.show_xray
            || self.state.session.shading == petunia_core::Shading::Wireframe;
        petunia_core::picking::pick_mesh_filtered(
            &asset.mesh,
            camera,
            glam::Vec2::new(width, height),
            glam::Vec2::from_array(ndc),
            mode,
            through,
            |point| scene.point_visible(camera, point),
        )
        .map_or(Target::None, |hit| match hit.component {
            petunia_core::picking::PickComponent::Vertex(i) => Target::Vertex(i as u32),
            petunia_core::picking::PickComponent::Edge(a, b) => Target::Edge(a, b),
            petunia_core::picking::PickComponent::Face(i) => Target::Face(i),
        })
    }

    /// Limpa a preselection (ponteiro saiu da viewport).
    pub fn clear_hover(&mut self) -> bool {
        let had_preview =
            self.profile_hover_snap.take().is_some() | self.region_hover.take().is_some();
        if !self.state.session.tools.hover.is_some() {
            return had_preview;
        }
        self.state.session.tools.hover = petunia_core::HoverTarget::None;
        true
    }

    /// Oclusão do segmento olho→ponto: a preselection respeita faces, salvo
    /// em X-Ray. `origin`/`direction` documentam o raio que gerou `position`;
    /// o teste usa a mesma cena canônica (`point_visible`) do picking.
    pub fn is_occluded(
        &self,
        _origin: glam::Vec3,
        _direction: glam::Vec3,
        position: glam::Vec3,
    ) -> bool {
        if self.state.session.show_xray {
            return false;
        }
        let scene =
            petunia_core::viewport_query::ViewportSceneQuery::new(&self.state.project.project);
        !scene.point_visible(&self.state.session.camera, position)
    }

    /// Handle do gizmo sob o cursor (preselection, sem clique).
    /// Handle do gizmo sob um ponto de tela, dentro de um raio de tolerância.
    ///
    /// O teste é em espaço de tela porque o gizmo tem tamanho fixo em pixels:
    /// o alvo do mouse precisa ser generoso (12 px) mesmo com a haste fina.
    pub fn gizmo_handle_at(&self, x: f32, y: f32) -> Option<GizmoHandle> {
        self.gizmo_target_at(x, y).map(|target| target.handle)
    }

    fn gizmo_target_at(&self, x: f32, y: f32) -> Option<GizmoTarget> {
        let gizmo = compute_gizmo(&self.state, self.viewport_size[0], self.viewport_size[1]);
        if !gizmo.visible {
            return None;
        }
        let active = self.state.session.tools.active_tool.as_str();
        let dist_from_origin = (x - gizmo.origin_x).hypot(y - gizmo.origin_y);
        // Desambiguação do gizmo universal: zona morta do centro protege contra cliques acidentais
        if active == "transform" && dist_from_origin < 12.0 {
            return None;
        }

        // Centro (Screen-space translation para Move, Uniform Scale para Scale, Trackball para Rotate)
        if dist_from_origin <= 12.0 {
            let kind = match active {
                "scale" => TransformKind::Scale,
                "rotate" => TransformKind::Rotation,
                _ => TransformKind::Position,
            };
            return Some(GizmoTarget {
                handle: GizmoHandle::Center,
                kind,
            });
        }

        // View Roll para a ferramenta Rotate (anel perimetral externo a ~113px)
        if active == "rotate"
            && (dist_from_origin - projection::GIZMO_VIEW_ROLL_RADIUS).abs()
                <= projection::GIZMO_RING_HIT_HALF_WIDTH
        {
            return Some(GizmoTarget {
                handle: GizmoHandle::Center,
                kind: TransformKind::Rotation,
            });
        }

        // Quadrantes de planos (Move e Scale): YZ (normal 0), XZ (normal 1), XY (normal 2)
        if matches!(active, "move" | "scale") {
            let plane_commands = [
                (0u8, &gizmo.plane_yz_commands),
                (1u8, &gizmo.plane_xz_commands),
                (2u8, &gizmo.plane_xy_commands),
            ];
            for (normal, cmd) in plane_commands {
                let numbers: Vec<f32> = cmd
                    .split_whitespace()
                    .filter_map(|token| token.parse::<f32>().ok())
                    .collect();
                if numbers.len() >= 8 {
                    let cx = (numbers[0] + numbers[2] + numbers[4] + numbers[6]) * 0.25;
                    let cy = (numbers[1] + numbers[3] + numbers[5] + numbers[7]) * 0.25;
                    if (x - cx).hypot(y - cy) <= 10.0 {
                        let kind = if active == "scale" {
                            TransformKind::Scale
                        } else {
                            TransformKind::Position
                        };
                        return Some(GizmoTarget {
                            handle: GizmoHandle::Plane(normal),
                            kind,
                        });
                    }
                }
            }
        }

        let families = [
            (
                TransformKind::Scale,
                9.0,
                [
                    &gizmo.x_scale_commands,
                    &gizmo.y_scale_commands,
                    &gizmo.z_scale_commands,
                ],
            ),
            (
                TransformKind::Rotation,
                7.0,
                [
                    &gizmo.x_rotate_commands,
                    &gizmo.y_rotate_commands,
                    &gizmo.z_rotate_commands,
                ],
            ),
            (
                match active {
                    "rotate" => TransformKind::Rotation,
                    "scale" => TransformKind::Scale,
                    _ => TransformKind::Position,
                },
                12.0,
                [&gizmo.x_commands, &gizmo.y_commands, &gizmo.z_commands],
            ),
        ];
        for (kind, hit_radius, commands_by_axis) in families {
            let mut family_best: Option<(GizmoTarget, f32)> = None;
            for (axis, commands) in commands_by_axis.into_iter().enumerate() {
                let handle = [GizmoHandle::X, GizmoHandle::Y, GizmoHandle::Z][axis];
                let numbers: Vec<f32> = commands
                    .split_whitespace()
                    .filter_map(|token| token.parse::<f32>().ok())
                    .collect();
                for segment in numbers.as_chunks::<2>().0.windows(2) {
                    let a = [segment[0][0], segment[0][1]];
                    let b = [segment[1][0], segment[1][1]];
                    if (a[0] - b[0]).hypot(a[1] - b[1]) < 0.25 {
                        continue;
                    }
                    let distance = point_segment_distance([x, y], a, b);
                    if distance <= hit_radius
                        && family_best.is_none_or(|(_, current)| distance < current)
                    {
                        family_best = Some((GizmoTarget { handle, kind }, distance));
                    }
                }
            }
            if let Some((target, _)) = family_best {
                return Some(target);
            }
        }
        None
    }

    /// Atualiza o handle do gizmo sob o cursor (preselection).
    pub fn hover_gizmo(&mut self, x: f32, y: f32) -> bool {
        if self.gizmo_drag.is_some() {
            return false;
        }
        let next = self.gizmo_handle_at(x, y);
        if next == self.gizmo_hover {
            return false;
        }
        self.gizmo_hover = next;
        true
    }

    /// Inicia o arrasto no handle do gizmo, restringindo a transformação ao eixo ou plano.
    pub fn begin_gizmo_drag(&mut self, x: f32, y: f32) -> bool {
        if self.state.primitive_session_valid() {
            self.state.finalize_primitive_session();
        }
        let Some(target) = self.gizmo_target_at(x, y) else {
            return false;
        };
        let handle = target.handle;
        let kind = target.kind;
        if !self.begin_viewport_transform(kind, x, y) {
            return false;
        }
        // A restrição é do domínio: o preview já sai no eixo/plano certo.
        let constraint = match handle {
            GizmoHandle::X => petunia_core::ModalConstraint::Axis(0),
            GizmoHandle::Y => petunia_core::ModalConstraint::Axis(1),
            GizmoHandle::Z => petunia_core::ModalConstraint::Axis(2),
            GizmoHandle::Center => petunia_core::ModalConstraint::Free,
            GizmoHandle::Plane(normal) => petunia_core::ModalConstraint::Plane(normal as usize),
        };
        let _ = self.state.set_modal_constraint(constraint);
        self.gizmo_drag = Some(handle);
        self.state.set_status(format!(
            "{} · {}",
            match kind {
                TransformKind::Position => "Move",
                TransformKind::Rotation => "Rotate",
                TransformKind::Scale => "Scale",
            },
            match handle {
                GizmoHandle::X => "X axis",
                GizmoHandle::Y => "Y axis",
                GizmoHandle::Z => "Z axis",
                GizmoHandle::Center => match kind {
                    TransformKind::Position => "Screen plane",
                    TransformKind::Scale => "Uniform",
                    TransformKind::Rotation => "View roll",
                },
                GizmoHandle::Plane(0) => "YZ plane",
                GizmoHandle::Plane(1) => "XZ plane",
                GizmoHandle::Plane(2) => "XY plane",
                GizmoHandle::Plane(_) => "Plane",
            }
        ));
        true
    }

    /// Encerra o arrasto do gizmo.
    pub fn end_gizmo_drag(&mut self) -> bool {
        if self.gizmo_drag.take().is_none() {
            return false;
        }
        self.end_viewport_transform();
        self.gizmo_hover = None;
        true
    }

    /// Atualiza o plano de corte enquanto o ponteiro se move.
    pub fn update_viewport_slice(&mut self, x: f32, y: f32) -> bool {
        self.update_viewport_slice_modified(x, y, false)
    }

    /// Atualiza o plano de corte com suporte a snap angular (ex.: 15° ao segurar Ctrl/snap).
    pub fn update_viewport_slice_modified(&mut self, x: f32, y: f32, snap: bool) -> bool {
        if self.slice_anchor.is_none() {
            return false;
        }
        self.update_slice_modified(x, y, snap)
    }

    /// Inicia uma transformação modal por arrasto na viewport.
    pub fn begin_viewport_transform(&mut self, kind: TransformKind, x: f32, y: f32) -> bool {
        self.modal_text.clear();
        self.instant_transform = false;
        let modal_kind = match kind {
            TransformKind::Position => petunia_core::ModalKind::Move,
            TransformKind::Rotation => petunia_core::ModalKind::Rotate,
            TransformKind::Scale => petunia_core::ModalKind::Scale,
        };
        match self.state.begin_modal(modal_kind) {
            Ok(()) => {
                self.start_viewport_drag(kind, x, y);
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Arrasto de viewport sobre uma operação já aberta no core.
    fn start_viewport_drag(&mut self, kind: TransformKind, x: f32, y: f32) {
        self.modal_text.clear();
        self.instant_transform = false;
        self.reset_transform_fields();
        self.drag = Some(ViewportDrag {
            kind,
            start: [x, y],
            viewport: self.viewport_size,
            last_pointer: [x, y],
            virtual_pointer: [x, y],
            rotation_angle: 0.0,
            last_angle: 0.0,
        });
    }

    /// Uma única passada de snap em espaço de tela (P3D-040, ADR 007 Onda 3).
    ///
    /// `cursor` em px lógicos da viewport. Durante uma operação, a malha de
    /// origem congelada é o alvo e os vértices que se movem são ignorados;
    /// fora dela, a malha ativa inteira.
    fn screen_snap(
        &self,
        cursor: glam::Vec2,
        mask: petunia_core::SnapMask,
        anchor: Option<petunia_core::SnapAnchor>,
        grid: Option<petunia_core::SnapGrid>,
    ) -> Option<petunia_core::ScreenSnapHit> {
        let modal = self.state.session.tools.modal.as_ref();
        let moving = modal
            .map(|modal| modal.moving_vertices())
            .unwrap_or_default();
        let mesh = modal
            .map(|modal| modal.source_mesh())
            .or_else(|| self.state.project.active_mesh());
        let session = &self.state.session;
        petunia_core::snap_screen(&petunia_core::ScreenSnapQuery {
            camera: &session.camera,
            viewport_pixels: glam::Vec2::from_array(self.viewport_size),
            cursor_pixels: cursor,
            mesh,
            moving: &moving,
            anchor,
            grid,
            radius_pixels: session.snap_settings.radius_pixels,
            mask,
            xray: session.show_xray || session.shading == petunia_core::Shading::Wireframe,
        })
    }

    /// Atualiza a transformação a partir do deslocamento absoluto do ponteiro.
    pub fn update_viewport_transform(&mut self, x: f32, y: f32) -> bool {
        self.update_viewport_transform_modified(x, y, false, false)
    }

    pub fn update_viewport_transform_modified(
        &mut self,
        x: f32,
        y: f32,
        fine: bool,
        snap: bool,
    ) -> bool {
        self.pointer_position = [x, y];
        if !self.modal_text.is_empty() {
            return false;
        }
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        let Some(mut drag) = self.drag else {
            return false;
        };
        let precision = if fine { 0.1 } else { 1.0 };
        for (axis, pointer) in [x, y].into_iter().enumerate() {
            let direction = if axis == 1 && self.state.ui.invert_vertical_drag {
                -1.0
            } else {
                1.0
            };
            drag.virtual_pointer[axis] +=
                (pointer - drag.last_pointer[axis]) * precision * direction;
            drag.last_pointer[axis] = pointer;
        }
        let Some(modal) = self.state.session.tools.modal.as_ref() else {
            return false;
        };
        let (constraint, pivot, normal) = (modal.constraint, modal.pivot, modal.normal);
        let camera = &self.state.session.camera;
        let viewport = glam::Vec2::from_array(drag.viewport);
        let start = glam::Vec2::from_array(drag.start);
        let current = glam::Vec2::from_array(drag.virtual_pointer);
        let delta = current - start;
        let axis = |index| match index {
            0 => glam::Vec3::X,
            1 => glam::Vec3::Y,
            _ => glam::Vec3::Z,
        };
        use petunia_core::{ModalConstraint, transform_projection as projection};
        let result = match drag.kind {
            TransformKind::Position => {
                let (mut translation, mut scalar) = match constraint {
                    ModalConstraint::Axis(index) => {
                        let value = projection::axis_delta(
                            camera,
                            viewport,
                            start,
                            current,
                            pivot,
                            axis(index),
                        )
                        .unwrap_or_else(|| {
                            self.screen_delta_on_axis(index, delta.x, delta.y, drag.viewport)
                        });
                        (axis(index) * value, value)
                    }
                    ModalConstraint::Plane(index) => (
                        projection::plane_delta(
                            camera,
                            viewport,
                            start,
                            current,
                            pivot,
                            axis(index),
                        )
                        .unwrap_or(glam::Vec3::ZERO),
                        0.0,
                    ),
                    ModalConstraint::Free => (
                        projection::plane_delta(
                            camera,
                            viewport,
                            start,
                            current,
                            pivot,
                            camera.forward(),
                        )
                        .unwrap_or(glam::Vec3::ZERO),
                        0.0,
                    ),
                };
                let mut snapped_kind = None;
                if snap || self.state.session.snap_enabled {
                    let target = self.state.session.snap_settings.target;
                    let mut mask = petunia_core::SnapMask::for_target(target);
                    // Move não tem plano de grade inequívoco em 3D: a grade vira
                    // passo relativo ao pivô abaixo, como antes.
                    mask.grid = false;
                    let hit = self.screen_snap(
                        current,
                        mask,
                        Some(petunia_core::SnapAnchor::world(pivot)),
                        None,
                    );
                    if let Some(hit) = hit {
                        translation = hit.point - pivot;
                        match constraint {
                            ModalConstraint::Axis(index) => {
                                let a = axis(index);
                                translation = a * translation.dot(a);
                                scalar = translation.dot(a);
                            }
                            ModalConstraint::Plane(index) => {
                                let a = axis(index);
                                translation = translation - a * translation.dot(a);
                            }
                            ModalConstraint::Free => {}
                        }
                        snapped_kind = Some(hit.kind);
                    } else if matches!(
                        target,
                        petunia_core::SnapTarget::Grid | petunia_core::SnapTarget::Increment
                    ) {
                        let step = self.state.session.snap_settings.grid_spacing.max(0.001);
                        translation = (translation / step).round() * step;
                        scalar = (scalar / step).round() * step;
                    }
                }
                self.state
                    .update_modal_snapped(translation, scalar, snapped_kind)
            }
            TransformKind::Rotation => {
                let normal = match constraint {
                    ModalConstraint::Axis(i) | ModalConstraint::Plane(i) => axis(i),
                    ModalConstraint::Free => normal,
                };
                let angle =
                    projection::rotation_angle(camera, viewport, start, current, pivot, normal)
                        .unwrap_or(delta.x * 0.5);
                let change = (angle - drag.last_angle + 180.0).rem_euclid(360.0) - 180.0;
                drag.rotation_angle += change;
                drag.last_angle = angle;
                let angle = if snap || self.state.session.snap_enabled {
                    (drag.rotation_angle / 15.0).round() * 15.0
                } else {
                    drag.rotation_angle
                };
                self.state.update_modal(glam::Vec3::ZERO, angle)
            }
            TransformKind::Scale => {
                let origin = projection::project_pixel(camera, viewport, pivot).unwrap_or(start);
                let a = start - origin;
                let b = current - origin;
                let mut factor = if a.length() >= 8.0 {
                    b.dot(a) / a.length_squared()
                } else {
                    1.0 + delta.x * 0.005
                };
                if snap || self.state.session.snap_enabled {
                    factor = (factor * 10.0).round() / 10.0;
                }
                if factor.abs() < 0.001 {
                    factor = if factor < 0.0 { -0.001 } else { 0.001 };
                }
                self.state.update_modal(glam::Vec3::ZERO, factor)
            }
        };
        self.drag = Some(drag);
        match result {
            Ok(()) => {
                let components = self
                    .state
                    .session
                    .tools
                    .modal
                    .as_ref()
                    .map(|modal| modal.components);
                if let Some(components) = components {
                    let fields = match drag.kind {
                        TransformKind::Position => &mut self.position,
                        TransformKind::Rotation => &mut self.rotation,
                        TransformKind::Scale => &mut self.scale,
                    };
                    fields[0].set_value(components.x);
                    fields[1].set_value(components.y);
                    fields[2].set_value(components.z);
                }
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Confirma a transformação por arrasto como uma única operação de undo.
    pub fn end_viewport_transform(&mut self) -> bool {
        if self.drag.take().is_none() {
            return false;
        }
        self.commit_transform();
        self.reset_transform_fields();
        true
    }

    pub fn cancel_viewport_transform(&mut self) -> bool {
        if self.drag.take().is_none() {
            return false;
        }
        self.cancel_transform()
    }

    pub fn active_layer_is_decal(&self) -> bool {
        self.state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .map(|layer| {
                matches!(
                    layer.kind,
                    petunia_project::paint_layers::LayerKind::Decal(_)
                )
            })
            .unwrap_or(false)
    }

    pub fn active_decal(&self) -> Option<petunia_project::paint_layers::DecalLayer> {
        self.state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .and_then(|layer| match &layer.kind {
                petunia_project::paint_layers::LayerKind::Decal(decal) => Some(decal.clone()),
                _ => None,
            })
    }

    pub fn set_active_decal_transform(
        &mut self,
        center_uv: [f32; 2],
        scale_uv: [f32; 2],
        rotation_rad: f32,
    ) -> bool {
        let active = self.state.project.active;
        let Some(asset) = self.state.project.assets.get_mut(active) else {
            return false;
        };
        let Some(stack) = asset.paint_stack.as_mut() else {
            return false;
        };
        let Some(layer) = stack.active_mut() else {
            return false;
        };
        let petunia_project::paint_layers::LayerKind::Decal(ref mut decal) = layer.kind else {
            return false;
        };
        decal.center_uv = [center_uv[0].clamp(0.0, 1.0), center_uv[1].clamp(0.0, 1.0)];
        decal.scale_uv = [scale_uv[0].clamp(0.01, 5.0), scale_uv[1].clamp(0.01, 5.0)];
        decal.rotation_rad = rotation_rad;
        petunia_module_paint::PaintModule::composite_active(&mut self.state);
        self.state.mark_dirty();
        true
    }

    pub fn decal_drag_begin(&mut self, x: f32, y: f32, is_shift: bool, is_ctrl: bool) -> bool {
        let Some(decal) = self.active_decal() else {
            return false;
        };
        self.state.begin_paint_stroke();
        self.decal_drag_initial = Some((
            x,
            y,
            decal.center_uv,
            decal.scale_uv,
            decal.rotation_rad.to_degrees(),
        ));
        if self.decal_drag_to(x, y, is_shift, is_ctrl) {
            true
        } else {
            self.decal_drag_initial = None;
            self.state.finish_paint_stroke(true);
            false
        }
    }

    pub fn decal_drag_to(&mut self, x: f32, y: f32, is_shift: bool, is_ctrl: bool) -> bool {
        let Some((init_x, init_y, init_center, init_scale, init_rot)) = self.decal_drag_initial
        else {
            return false;
        };

        if is_shift {
            let delta_y = init_y - y;
            let factor = (1.0 + delta_y * 0.01).max(0.05);
            let new_scale_u = (init_scale[0] * factor).clamp(0.01, 5.0);
            let new_scale_v = (init_scale[1] * factor).clamp(0.01, 5.0);
            self.set_active_decal_transform(
                init_center,
                [new_scale_u, new_scale_v],
                init_rot.to_radians(),
            )
        } else if is_ctrl {
            let delta_x = x - init_x;
            let new_rot_deg = init_rot + delta_x * 0.5;
            self.set_active_decal_transform(init_center, init_scale, new_rot_deg.to_radians())
        } else {
            let width = self.viewport_size[0].max(1.0);
            let height = self.viewport_size[1].max(1.0);
            if !x.is_finite()
                || !y.is_finite()
                || !(0.0..width).contains(&x)
                || !(0.0..height).contains(&y)
            {
                return false;
            }
            let ndc_x = x / width * 2.0 - 1.0;
            let ndc_y = 1.0 - y / height * 2.0;
            let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);
            let uv_opt = pick_face_hit(&self.state, origin, direction).and_then(|(face, hit)| {
                petunia_module_paint::PaintModule::face_hit_uv(&self.state, face, hit, false)
            });
            if let Some(uv) = uv_opt {
                return self.set_active_decal_transform(uv, init_scale, init_rot.to_radians());
            }
            true
        }
    }

    pub fn decal_drag_end(&mut self) -> bool {
        if self.decal_drag_initial.take().is_some() {
            self.state.finish_paint_stroke(false);
            self.state.set_status("Decal transform committed");
            true
        } else {
            false
        }
    }

    pub fn cancel_decal_drag(&mut self) -> bool {
        if self.decal_drag_initial.take().is_some() {
            self.state.finish_paint_stroke(true);
            self.state.set_status("Decal transform cancelled");
            true
        } else {
            false
        }
    }

    /// Inicia um traço de pintura ou manipulação de decalque contínuo.
    pub fn begin_paint_stroke_at(&mut self, x: f32, y: f32) -> bool {
        self.begin_paint_stroke_with_modifiers(x, y, false, false)
    }

    /// Inicia um traço de pintura ou manipulação interativa de decalque com modificadores (Shift: escala, Ctrl: rotação).
    pub fn begin_paint_stroke_with_modifiers(
        &mut self,
        x: f32,
        y: f32,
        is_shift: bool,
        is_ctrl: bool,
    ) -> bool {
        if self.state.workspace != Workspace::Paint {
            return false;
        }
        if self.state.project.active_mesh().is_none() {
            self.state.set_status("No active mesh to paint");
            return false;
        }
        if self.active_layer_is_decal() {
            return self.decal_drag_begin(x, y, is_shift, is_ctrl);
        }
        if self.state.session.tools.active_tool == "picker" {
            return self.pick_paint_color_at(x, y);
        }
        if self.is_shape_tool() {
            return self.begin_paint_shape_at(x, y);
        }
        self.state.begin_paint_stroke();
        let settings = self.viewport_brush_settings();
        let dabs = self.paint_sampler.begin([x, y], settings.dab_step_px());
        self.paint_dabs_at(&dabs, settings);
        self.paint_last = Some([x, y]);
        self.state.mark_dirty();
        true
    }

    /// Ferramentas de forma ancoram no press e confirmam no release.
    pub fn is_shape_tool(&self) -> bool {
        matches!(
            petunia_core::brush_type_from_kind(self.state.session.tools.paint_brush_kind),
            petunia_core::BrushType::Line | petunia_core::BrushType::Rectangle
        ) || self.state.session.tools.active_tool == "gradient"
    }

    /// Inicia uma forma (Line/Rectangle/Gradient) no pixel do canvas sob o cursor.
    pub fn begin_paint_shape_at(&mut self, x: f32, y: f32) -> bool {
        if self.state.workspace != Workspace::Paint {
            return false;
        }
        // A forma precisa de canvas para converter UV em pixel.
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let Some((px, py)) = self.canvas_pixel_at(x, y) else {
            self.state
                .set_status("Shape: point at the surface to anchor the shape");
            return false;
        };
        self.shape_anchor = Some((px, py));
        self.state.set_status("Shape anchored: release to commit");
        true
    }

    /// Confirma a forma como uma única operação de undo.
    pub fn end_paint_shape_at(&mut self, x: f32, y: f32) -> bool {
        let Some((x0, y0)) = self.shape_anchor.take() else {
            return false;
        };
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let Some((x1, y1)) = self.canvas_pixel_at(x, y) else {
            self.state
                .set_status("Shape: release point is off the surface, discarded");
            return false;
        };
        if self.state.session.tools.active_tool == "gradient" {
            let color_start = [
                (self.state.paint_color[0] * 255.0).clamp(0.0, 255.0) as u8,
                (self.state.paint_color[1] * 255.0).clamp(0.0, 255.0) as u8,
                (self.state.paint_color[2] * 255.0).clamp(0.0, 255.0) as u8,
                255,
            ];
            let color_end = [255, 255, 255, 0];
            self.state.checkpoint("paint gradient");
            petunia_module_paint::PaintModule::canvas_gradient_linear(
                &mut self.state,
                x0,
                y0,
                x1,
                y1,
                color_start,
                color_end,
            );
            self.state.emit_texture_changed();
            self.state.mark_dirty();
            self.state.set_status("Gradient committed");
            return true;
        }
        let brush = petunia_core::brush_type_from_kind(self.state.session.tools.paint_brush_kind);
        let color = [
            (self.state.paint_color[0] * 255.0).clamp(0.0, 255.0) as u8,
            (self.state.paint_color[1] * 255.0).clamp(0.0, 255.0) as u8,
            (self.state.paint_color[2] * 255.0).clamp(0.0, 255.0) as u8,
            255,
        ];
        let strength = self.state.session.tools.paint_strength;
        self.state.checkpoint("paint shape");
        petunia_module_paint::PaintModule::commit_shape(
            &mut self.state,
            petunia_module_paint::ShapeStroke {
                x0,
                y0,
                x1,
                y1,
                brush,
                color,
                strength,
            },
        );
        self.state.emit_texture_changed();
        self.state.mark_dirty();
        self.state
            .set_status(format!("Shape committed ({brush:?})"));
        true
    }

    /// Cancela a forma ancorada sem tocar no documento.
    pub fn cancel_paint_shape(&mut self) -> bool {
        if self.shape_anchor.take().is_none() {
            return false;
        }
        self.state.set_status("Shape cancelled");
        true
    }

    fn canvas_pixel_at(&self, x: f32, y: f32) -> Option<(u32, u32)> {
        let width = self.viewport_size[0].max(1.0);
        let height = self.viewport_size[1].max(1.0);
        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..width).contains(&x)
            || !(0.0..height).contains(&y)
        {
            return None;
        }
        let ndc_x = x / width * 2.0 - 1.0;
        let ndc_y = 1.0 - y / height * 2.0;
        let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);
        let (face, hit) = pick_face_hit(&self.state, origin, direction)?;
        let isolate = self.state.session.tools.paint_isolate_selection;
        let uv = petunia_module_paint::PaintModule::face_hit_uv(&self.state, face, hit, isolate)?;
        petunia_module_paint::PaintModule::uv_to_px(&self.state, uv)
    }

    fn pick_paint_color_at(&mut self, x: f32, y: f32) -> bool {
        let Some((px, py)) = self.canvas_pixel_at(x, y) else {
            return false;
        };
        let Some(color) = self
            .state
            .project
            .active()
            .and_then(|asset| asset.texture.as_ref())
            .and_then(|texture| texture.get(px, py))
        else {
            return false;
        };
        let color = [color[0], color[1], color[2]].map(|channel| channel as f32 / 255.0);
        self.state.paint_color = color;
        self.state.session.tools.paint_color = color;
        self.state.mark_dirty();
        self.state.set_status("Color sampled from texture");
        true
    }

    /// Estende o traço interpolando em espaço de tela e pintando cada dab.
    pub fn paint_stroke_to(&mut self, x: f32, y: f32) -> bool {
        self.paint_stroke_to_with_modifiers(x, y, false, false)
    }

    /// Estende o traço de pintura ou manipulação de decalque com suporte a modificadores.
    pub fn paint_stroke_to_with_modifiers(
        &mut self,
        x: f32,
        y: f32,
        is_shift: bool,
        is_ctrl: bool,
    ) -> bool {
        if self.active_layer_is_decal() {
            return self.decal_drag_to(x, y, is_shift, is_ctrl);
        }
        if self.paint_last.is_none() {
            return false;
        }
        let settings = self.viewport_brush_settings();
        let dabs = self.paint_sampler.extend([x, y]);
        self.paint_dabs_at(&dabs, settings);
        self.paint_last = Some([x, y]);
        self.state.mark_dirty();
        true
    }

    /// Confirma o traço de pintura ou manipulação de decalque como uma única entrada de undo.
    pub fn end_paint_stroke_at(&mut self, x: f32, y: f32) -> bool {
        if self.decal_drag_initial.is_some() {
            return self.decal_drag_end();
        }
        if self.shape_anchor.is_some() {
            return self.end_paint_shape_at(x, y);
        }
        if self.paint_last.is_none() {
            return false;
        }
        self.paint_stroke_to(x, y);
        self.paint_last = None;
        self.paint_sampler.reset();
        self.state.finish_paint_stroke(false);
        true
    }

    pub fn cancel_paint_stroke(&mut self) -> bool {
        if self.cancel_decal_drag() {
            return true;
        }
        if self.cancel_paint_shape() {
            return true;
        }
        if self.paint_2d_last.take().is_some() {
            self.paint_2d_sampler.reset();
            self.state.finish_paint_stroke(true);
            self.state.mark_dirty();
            return true;
        }
        if self.paint_last.take().is_none() {
            return false;
        }
        self.paint_sampler.reset();
        self.state.finish_paint_stroke(true);
        true
    }

    /// Tique periódico para acúmulo contínuo de tinta da ferramenta Airbrush (P3D-056).
    pub fn airbrush_tick(&mut self) -> bool {
        let is_airbrush = self.state.session.tools.active_tool == "airbrush"
            || petunia_core::brush_type_from_kind(self.state.session.tools.paint_brush_kind)
                == petunia_core::BrushType::Airbrush;
        if !is_airbrush {
            return false;
        }
        let mut changed = false;
        if let Some([x, y]) = self.paint_last {
            let settings = self.viewport_brush_settings();
            self.paint_dabs_at(&[[x, y]], settings);
            changed = true;
        }
        if let Some((px, py)) = self.paint_2d_last {
            let settings = self.state.brush_settings();
            petunia_module_paint::PaintModule::canvas_brush_with_symmetry(
                &mut self.state,
                px,
                py,
                settings,
            );
            changed = true;
        }
        if changed {
            self.state.mark_dirty();
        }
        changed
    }

    /// Processa interação interativa de desenho no canvas 2D de textura (P3D-057).
    /// phase: 0 = Down, 1 = Move, 2 = Up, outros = Cancel
    pub fn paint_2d_stroke(&mut self, norm_x: f32, norm_y: f32, phase: i32) -> bool {
        if !norm_x.is_finite() || !norm_y.is_finite() {
            return false;
        }
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let (width, height) = match self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|a| a.texture.as_ref())
        {
            Some(t) => (t.w, t.h),
            None => (256, 256),
        };
        let px = ((norm_x * width as f32).floor() as i32).clamp(0, width as i32 - 1) as u32;
        let py = ((norm_y * height as f32).floor() as i32).clamp(0, height as i32 - 1) as u32;

        let tool = self.state.session.tools.active_tool.clone();
        if tool == "gradient" {
            match phase {
                0 => {
                    self.shape_anchor = Some((px, py));
                    self.state
                        .set_status("Gradient: drag to set direction and length");
                    return true;
                }
                1 => {
                    return true;
                }
                2 => {
                    if let Some((x0, y0)) = self.shape_anchor.take() {
                        let color_start = [
                            (self.state.paint_color[0] * 255.0).clamp(0.0, 255.0) as u8,
                            (self.state.paint_color[1] * 255.0).clamp(0.0, 255.0) as u8,
                            (self.state.paint_color[2] * 255.0).clamp(0.0, 255.0) as u8,
                            255,
                        ];
                        let color_end = [255, 255, 255, 0];
                        self.state.checkpoint("paint gradient");
                        petunia_module_paint::PaintModule::canvas_gradient_linear(
                            &mut self.state,
                            x0,
                            y0,
                            px,
                            py,
                            color_start,
                            color_end,
                        );
                        self.state.emit_texture_changed();
                        self.state.mark_dirty();
                        self.state.set_status("Gradient applied");
                        return true;
                    }
                    return false;
                }
                _ => {
                    self.shape_anchor = None;
                    return false;
                }
            }
        }

        match phase {
            0 => {
                let tool = self.state.session.tools.active_tool.clone();
                if tool == "picker" {
                    if let Some(color) = self
                        .state
                        .project
                        .assets
                        .get(self.state.project.active)
                        .and_then(|a| a.texture.as_ref())
                        .and_then(|t| t.get(px, py))
                    {
                        let c = [
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                        ];
                        self.state.paint_color = c;
                        self.state.session.tools.paint_color = c;
                        self.state.mark_dirty();
                        self.state.set_status("Color sampled from canvas");
                    }
                    return true;
                }
                if tool == "fill" {
                    let scope = self.state.session.tools.fill_scope;
                    self.state.begin_paint_stroke();
                    petunia_module_paint::PaintModule::canvas_fill_scoped(
                        &mut self.state,
                        None,
                        Some((px, py)),
                        scope,
                    );
                    self.state.finish_paint_stroke(false);
                    self.state.set_status(format!("Filled canvas ({scope:?})"));
                    return true;
                }
                self.state.begin_paint_stroke();
                let settings = self.state.brush_settings();
                let dabs = self
                    .paint_2d_sampler
                    .begin([px as f32, py as f32], settings.dab_step_px());
                let dabs = texture_points(&dabs, width, height);
                petunia_module_paint::PaintModule::canvas_brush_batch_with_symmetry(
                    &mut self.state,
                    &dabs,
                    settings,
                );
                self.paint_2d_last = Some((px, py));
                self.state.mark_dirty();
                true
            }
            1 => {
                if self.paint_2d_last.is_none() {
                    return false;
                }
                let settings = self.state.brush_settings();
                let dabs = self.paint_2d_sampler.extend([px as f32, py as f32]);
                let dabs = texture_points(&dabs, width, height);
                petunia_module_paint::PaintModule::canvas_brush_batch_with_symmetry(
                    &mut self.state,
                    &dabs,
                    settings,
                );
                self.paint_2d_last = Some((px, py));
                self.state.mark_dirty();
                true
            }
            2 => {
                if self.paint_2d_last.take().is_none() {
                    return false;
                }
                let settings = self.state.brush_settings();
                let dabs = self.paint_2d_sampler.extend([px as f32, py as f32]);
                let dabs = texture_points(&dabs, width, height);
                petunia_module_paint::PaintModule::canvas_brush_batch_with_symmetry(
                    &mut self.state,
                    &dabs,
                    settings,
                );
                self.paint_2d_sampler.reset();
                self.state.finish_paint_stroke(false);
                self.state.mark_dirty();
                true
            }
            _ => {
                if self.paint_2d_last.take().is_none() {
                    return false;
                }
                self.paint_2d_sampler.reset();
                self.state.finish_paint_stroke(true);
                self.state.mark_dirty();
                true
            }
        }
    }

    pub fn project_from_reference(&mut self) -> bool {
        match self.execute_core_command("uv.project_reference") {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn bake_reference(&mut self) -> bool {
        match self.execute_core_command("paint.bake_reference") {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Define o modo de sombreamento da viewport pelo id estável.
    ///
    /// Cada modo corresponde a um pipeline real: Wireframe não preenche,
    /// Solid usa o estúdio da viewport, MaterialPreview amostra a textura e
    /// Rendered usa a luz da cena.
    pub fn set_shading_mode(&mut self, id: &str) -> bool {
        let Some(mode) = petunia_core::Shading::from_id(id) else {
            return false;
        };
        if self.state.shading == mode {
            return false;
        }
        self.state.shading = mode;
        // Material/Rendered dependem de amostrar o material.
        if mode.samples_material() {
            self.state.session.textured = true;
        }
        self.state.set_status(format!("Shading: {}", mode.id()));
        self.state.mark_dirty();
        true
    }

    /// Opacidade da geometria em X-Ray.
    pub fn set_xray_opacity(&mut self, opacity: f32) -> bool {
        if !opacity.is_finite() {
            return false;
        }
        let clamped = opacity.clamp(0.1, 0.9);
        if (clamped - self.state.session.xray_opacity).abs() < f32::EPSILON {
            return false;
        }
        self.state.session.xray_opacity = clamped;
        self.state.mark_dirty();
        true
    }

    /// Define o modo de confirmação das ferramentas paramétricas.
    pub fn set_tool_activation(&mut self, id: &str) -> bool {
        let Some(mode) = petunia_core::ToolActivation::from_id(id) else {
            return false;
        };
        if self.state.session.tools.tool_activation == mode {
            return false;
        }
        self.state.session.tools.tool_activation = mode;
        self.state.set_status(match mode {
            petunia_core::ToolActivation::Drag => "Tools confirm on pointer release (click + drag)",
            petunia_core::ToolActivation::Instant => {
                "Tools follow the pointer; click or Enter confirms, Esc cancels"
            }
        });
        self.state.mark_dirty();
        true
    }

    /// Preferência de input da aplicação, sem criar undo nem sujar o modelo.
    pub fn set_invert_vertical_drag(&mut self, invert: bool) -> bool {
        if self.state.ui.invert_vertical_drag == invert {
            return false;
        }
        self.state.ui.invert_vertical_drag = invert;
        let status_id = if invert {
            petunia_config::text_id::UI_VERTICAL_DRAG_INVERTED
        } else {
            petunia_config::text_id::UI_VERTICAL_DRAG_NORMAL
        };
        self.state.set_status(self.state.t_id(status_id));
        true
    }

    /// Diferenciação não-cromática de eixos para acessibilidade e daltonismo.
    pub fn set_colorblind_axes(&mut self, enabled: bool) -> bool {
        if self.state.ui.colorblind_axes == enabled {
            return false;
        }
        self.state.ui.colorblind_axes = enabled;
        self.preferences.colorblind_axes = enabled;
        self.state.mark_dirty();
        true
    }

    /// Redução de movimento para usuários com sensibilidade vestibular / labirintite.
    pub fn set_reduced_motion(&mut self, enabled: bool) -> bool {
        if self.state.ui.reduced_motion == enabled {
            return false;
        }
        self.state.ui.reduced_motion = enabled;
        self.preferences.reduced_motion = enabled;
        self.state.mark_dirty();
        true
    }

    /// Exibe tag flutuante com a média de medidas na multiseleção de arestas.
    pub fn set_multiselection_measure_tag(&mut self, enabled: bool) -> bool {
        if self.state.ui.multiselection_measure_tag == enabled {
            return false;
        }
        self.state.ui.multiselection_measure_tag = enabled;
        self.preferences.multiselection_measure_tag = enabled;
        self.state.mark_dirty();
        true
    }

    /// Intervalo máximo em milissegundos para duplo toque de tecla de ferramenta.
    pub fn set_double_tap_interval_ms(&mut self, interval_ms: u64) -> bool {
        let clamped = interval_ms.min(2000);
        if self.preferences.double_tap_interval_ms == clamped {
            return false;
        }
        self.preferences.double_tap_interval_ms = clamped;
        self.state.mark_dirty();
        true
    }

    /// Alterna a pílula do Micro-Inspector flutuante sob o cursor (Espaço).
    pub fn toggle_micro_inspector(&mut self) -> bool {
        if self.micro_inspector_open {
            self.micro_inspector_open = false;
            self.overlays.remove(OverlayId::MicroInspector);
        } else {
            self.micro_inspector_open = true;
            self.micro_inspector_pos = self.pointer_position;
            self.overlays.push(OverlayEntry {
                id: OverlayId::MicroInspector,
                kind: OverlayKind::FloatingPanel,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            });
        }
        self.state.mark_dirty();
        true
    }

    pub fn set_asset_query(&mut self, query: &str) -> bool {
        if self.asset_query == query {
            return false;
        }
        self.asset_query = query.to_string();
        true
    }

    pub fn set_parts_query(&mut self, query: &str) -> bool {
        if self.parts_query == query {
            return false;
        }
        self.parts_query = query.to_string();
        true
    }

    pub fn set_parts_selected_only(&mut self, enabled: bool) -> bool {
        if self.parts_selected_only == enabled {
            return false;
        }
        self.parts_selected_only = enabled;
        true
    }

    pub fn set_parts_sort_by_name(&mut self, enabled: bool) -> bool {
        if self.parts_sort_by_name == enabled {
            return false;
        }
        self.parts_sort_by_name = enabled;
        true
    }

    pub fn set_parts_row_height(&mut self, size: f32) -> bool {
        if !size.is_finite() {
            return false;
        }
        let size = size.clamp(28.0, 44.0);
        if (self.parts_row_height - size).abs() < f32::EPSILON {
            return false;
        }
        self.parts_row_height = size;
        true
    }

    pub fn set_asset_sort_by_name(&mut self, sort_by_name: bool) -> bool {
        if self.asset_sort_by_name == sort_by_name {
            return false;
        }
        self.asset_sort_by_name = sort_by_name;
        true
    }

    pub fn set_asset_thumbnail_size(&mut self, size: f32) -> bool {
        if !size.is_finite() {
            return false;
        }
        let size = size.clamp(48.0, 128.0);
        if (self.state.ui.asset_thumbnail_size - size).abs() < f32::EPSILON {
            return false;
        }
        self.state.ui.asset_thumbnail_size = size;
        true
    }

    pub fn set_selection_color_hex(&mut self, value: &str) -> bool {
        let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
        if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            let message = self
                .state
                .t_id(petunia_config::text_id::UI_SELECTION_COLOR_INVALID);
            self.state.set_status(message);
            return false;
        }
        let mut rgb = [0u8; 3];
        for (index, channel) in rgb.iter_mut().enumerate() {
            let Some(piece) = hex.get(index * 2..index * 2 + 2) else {
                return false;
            };
            let Ok(parsed) = u8::from_str_radix(piece, 16) else {
                return false;
            };
            *channel = parsed;
        }
        if !selection_color_has_contrast(rgb) {
            let message = self
                .state
                .t_id(petunia_config::text_id::UI_SELECTION_COLOR_LOW_CONTRAST);
            self.state.set_status(message);
            return false;
        }
        if self.state.ui.selection_rgb == rgb {
            return false;
        }
        self.state.ui.selection_rgb = rgb;
        true
    }

    pub fn set_selection_thickness(&mut self, thickness: f32) -> bool {
        if !thickness.is_finite() {
            return false;
        }
        let thickness = thickness.clamp(1.0, 6.0);
        if (self.state.ui.selection_thickness - thickness).abs() < f32::EPSILON {
            return false;
        }
        self.state.ui.selection_thickness = thickness;
        true
    }

    pub fn set_language(&mut self, lang: &str) -> bool {
        if self.state.ui.i18n.lang == lang {
            return false;
        }
        self.state.ui.i18n.set_lang(lang);
        self.state.mark_dirty();
        true
    }

    pub fn set_ui_scale(&mut self, scale: f32) -> bool {
        if !scale.is_finite() {
            return false;
        }
        let scale = scale.clamp(1.0, 2.0);
        if (self.preferences.ui_scale - scale).abs() < 0.001 {
            return false;
        }
        self.preferences.ui_scale = scale;
        self.state.mark_dirty();
        true
    }

    /// Define o estilo do tema de ícones ("outline" ou "filled").
    pub fn set_icon_theme(&mut self, theme: &str) -> bool {
        let theme_normalized = match theme {
            "filled" => "filled",
            _ => "outline",
        };
        if self.preferences.icon_theme == theme_normalized {
            return false;
        }
        self.preferences.icon_theme = theme_normalized.to_string();
        self.state.mark_dirty();
        true
    }

    /// O modo Instant / Modo Livre com mouse está ativo?
    pub fn is_instant_tool_mode(&self) -> bool {
        self.keyboard_tool_modal_active
            || self.instant_transform
            || self.state.session.tools.tool_activation == petunia_core::ToolActivation::Instant
    }

    /// Alinha a câmera a um eixo a partir do tripé de navegação.
    pub fn snap_view_to_axis(&mut self, axis: &str) -> bool {
        use petunia_core::ViewPreset;
        let preset = match axis {
            "x" => ViewPreset::Right,
            "y" => ViewPreset::Top,
            "z" => ViewPreset::Front,
            _ => return false,
        };
        self.set_view_preset(preset)
    }

    /// Define diretamente o preset de visualização da câmera e notifica o estado.
    pub fn set_view_preset(&mut self, preset: petunia_core::ViewPreset) -> bool {
        self.state.session.camera.set_preset(preset);
        self.state.set_status(format!("View: {}", preset.title()));
        self.state.mark_dirty();
        true
    }

    /// Alterna a visualização para a vista oposta (ex: Front -> Back, Top -> Bottom).
    pub fn toggle_view_opposite(&mut self) -> bool {
        use petunia_core::ViewPreset;
        let next = match self.state.session.camera.view_preset() {
            Some(ViewPreset::Front) => ViewPreset::Back,
            Some(ViewPreset::Back) => ViewPreset::Front,
            Some(ViewPreset::Right) => ViewPreset::Left,
            Some(ViewPreset::Left) => ViewPreset::Right,
            Some(ViewPreset::Top) => ViewPreset::Bottom,
            Some(ViewPreset::Bottom) => ViewPreset::Top,
            _ => {
                self.state.session.camera.yaw = (self.state.session.camera.yaw
                    + std::f32::consts::PI)
                    .rem_euclid(std::f32::consts::TAU);
                self.state.mark_dirty();
                return true;
            }
        };
        self.set_view_preset(next)
    }

    /// Redimensiona o dock de contexto pelo divisor vertical.
    ///
    /// Layout é estado de apresentação: não marca o documento como alterado.
    pub fn set_inspector_width(&mut self, width: f32) -> bool {
        self.state.ui.set_right_width(width)
    }

    /// Redimensiona a Asset Library pelo divisor horizontal.
    pub fn set_asset_library_height(&mut self, height: f32) -> bool {
        self.state.ui.set_shell_asset_library_height(height)
    }

    /// Abre a modal do Gerenciador de Referências (P3D-013).
    pub fn open_reference_manager(&mut self) -> bool {
        self.reference_manager_open = true;
        self.overlays.push(OverlayEntry {
            id: OverlayId::ReferenceManager,
            kind: OverlayKind::Modal,
            pinned: false,
            dismiss_on_escape: true,
            dismiss_on_click_away: true,
        });
        self.state.set_status("Reference Sets · P3D-013");
        true
    }

    /// Fecha a modal do Gerenciador de Referências.
    pub fn close_reference_manager(&mut self) -> bool {
        self.reference_manager_open = false;
        self.overlays.remove(OverlayId::ReferenceManager);
        true
    }

    /// Alterna a visibilidade da modal do Gerenciador de Referências.
    pub fn toggle_reference_manager(&mut self) -> bool {
        if self.reference_manager_open {
            self.close_reference_manager()
        } else {
            self.open_reference_manager()
        }
    }

    /// Carrega uma imagem de referência para o slot correspondente (ou custom).
    pub fn load_reference_slot(
        &mut self,
        axis_str: &str,
        name: String,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    ) -> bool {
        let axis = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::RefAxis::Front,
            "back" => petunia_core::RefAxis::Back,
            "left" => petunia_core::RefAxis::Left,
            "right" => petunia_core::RefAxis::Right,
            "top" => petunia_core::RefAxis::Top,
            "bottom" => petunia_core::RefAxis::Bottom,
            _ => petunia_core::RefAxis::Front,
        };
        let axis_key = axis_str.to_lowercase();
        self.reference_thumbnails
            .insert(axis_key, (width, height, rgba.clone()));
        if axis_str == "custom" {
            petunia_core::project_service::ProjectService::add_reference_image(
                &mut self.state,
                name,
                width,
                height,
                rgba,
            );
        } else {
            petunia_core::project_service::ProjectService::set_reference_slot(
                &mut self.state,
                axis,
                name,
                width,
                height,
                rgba,
            );
        }
        self.state.mark_dirty();
        true
    }

    /// Remove a imagem de referência associada ao slot indicado.
    pub fn remove_reference_slot(&mut self, axis_str: &str) -> bool {
        let axis = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::RefAxis::Front,
            "back" => petunia_core::RefAxis::Back,
            "left" => petunia_core::RefAxis::Left,
            "right" => petunia_core::RefAxis::Right,
            "top" => petunia_core::RefAxis::Top,
            "bottom" => petunia_core::RefAxis::Bottom,
            _ => return false,
        };
        self.reference_thumbnails.remove(&axis_str.to_lowercase());
        if let Some(idx) = self.state.project.refs.iter().position(|r| {
            r.axis == axis
                || (axis == petunia_core::RefAxis::Right && r.axis == petunia_core::RefAxis::Side)
        }) {
            petunia_core::project_service::ProjectService::remove_reference(&mut self.state, idx);
            true
        } else {
            false
        }
    }

    /// Alinha a câmera do viewport à vista ortográfica correspondente ao slot.
    pub fn align_reference_view(&mut self, axis_str: &str) -> bool {
        let preset = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::ViewPreset::Front,
            "back" => petunia_core::ViewPreset::Back,
            "left" => petunia_core::ViewPreset::Left,
            "right" => petunia_core::ViewPreset::Right,
            "top" => petunia_core::ViewPreset::Top,
            "bottom" => petunia_core::ViewPreset::Bottom,
            _ => return false,
        };
        self.state.session.camera.set_preset(preset);
        self.state
            .set_status(format!("View aligned to {}", preset.title()));
        self.state.mark_dirty();
        true
    }

    /// Alterna a visibilidade da imagem de referência no slot.
    pub fn toggle_reference_visible(&mut self, axis_str: &str) -> bool {
        let axis = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::RefAxis::Front,
            "back" => petunia_core::RefAxis::Back,
            "left" => petunia_core::RefAxis::Left,
            "right" => petunia_core::RefAxis::Right,
            "top" => petunia_core::RefAxis::Top,
            "bottom" => petunia_core::RefAxis::Bottom,
            _ => return false,
        };
        if let Some(r) = self.state.project.refs.iter_mut().find(|r| {
            r.axis == axis
                || (axis == petunia_core::RefAxis::Right && r.axis == petunia_core::RefAxis::Side)
        }) {
            r.visible = !r.visible;
            self.state.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Alterna o bloqueio de edição/transformação da imagem de referência.
    pub fn toggle_reference_lock(&mut self, axis_str: &str) -> bool {
        let axis = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::RefAxis::Front,
            "back" => petunia_core::RefAxis::Back,
            "left" => petunia_core::RefAxis::Left,
            "right" => petunia_core::RefAxis::Right,
            "top" => petunia_core::RefAxis::Top,
            "bottom" => petunia_core::RefAxis::Bottom,
            _ => return false,
        };
        if let Some(r) = self.state.project.refs.iter_mut().find(|r| {
            r.axis == axis
                || (axis == petunia_core::RefAxis::Right && r.axis == petunia_core::RefAxis::Side)
        }) {
            r.locked = !r.locked;
            self.state.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Ajusta um parâmetro de calibração fina da referência (opacity, size, offset, rotation).
    pub fn set_reference_param(&mut self, axis_str: &str, param: &str, val: f32) -> bool {
        let axis = match axis_str.to_lowercase().as_str() {
            "front" => petunia_core::RefAxis::Front,
            "back" => petunia_core::RefAxis::Back,
            "left" => petunia_core::RefAxis::Left,
            "right" => petunia_core::RefAxis::Right,
            "top" => petunia_core::RefAxis::Top,
            "bottom" => petunia_core::RefAxis::Bottom,
            _ => return false,
        };
        if let Some(r) = self.state.project.refs.iter_mut().find(|r| {
            r.axis == axis
                || (axis == petunia_core::RefAxis::Right && r.axis == petunia_core::RefAxis::Side)
        }) {
            match param {
                "opacity" => r.opacity = val.clamp(0.0, 1.0),
                "size" => r.size = val.clamp(0.1, 50.0),
                "offset" => r.offset = val.clamp(-50.0, 50.0),
                "rotation" => r.rotation = val.clamp(-180.0, 180.0),
                _ => return false,
            }
            self.state.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Limpa todas as referências da cena.
    pub fn clear_all_references(&mut self) -> bool {
        petunia_core::project_service::ProjectService::clear_references(&mut self.state);
        self.reference_thumbnails.clear();
        true
    }

    /// Abre, troca ou fecha um menu da barra superior.
    pub fn toggle_menu(&mut self, id: &str) -> bool {
        let next = MenuKind::ALL.into_iter().find(|kind| kind.id() == id);
        self.menu_open = match (self.menu_open, next) {
            (Some(current), Some(next)) if current == next => None,
            (_, next) => next,
        };
        match self.menu_open {
            Some(_) => self.overlays.push(OverlayEntry {
                id: OverlayId::MenuBar,
                kind: OverlayKind::Popover,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            }),
            None => {
                self.overlays.remove(OverlayId::MenuBar);
            }
        }
        self.state.mark_dirty();
        self.menu_open.is_some()
    }

    /// Abre diretamente um menu ou troca para ele (rollover entre menus do topo).
    pub fn open_menu(&mut self, id: &str) -> bool {
        let next = MenuKind::ALL.into_iter().find(|kind| kind.id() == id);
        if self.menu_open == next {
            return false;
        }
        self.menu_open = next;
        match self.menu_open {
            Some(_) => self.overlays.push(OverlayEntry {
                id: OverlayId::MenuBar,
                kind: OverlayKind::Popover,
                pinned: false,
                dismiss_on_escape: true,
                dismiss_on_click_away: true,
            }),
            None => {
                self.overlays.remove(OverlayId::MenuBar);
            }
        }
        self.state.mark_dirty();
        self.menu_open.is_some()
    }

    pub fn close_menu(&mut self) -> bool {
        self.overlays.remove(OverlayId::MenuBar);
        self.menu_open.take().is_some()
    }

    /// Executa um item de menu pelo id canônico que ele publica.
    pub fn menu_item_invoked(&mut self, id: &str) -> bool {
        self.close_menu();
        match id {
            // Itens de arquivo que abrem diálogo são despachados pelo mesmo
            // caminho assíncrono do `command-file-action`; aqui só o que resolve
            // de imediato.
            "file.new" => self.execute_core_command("file.new").is_ok(),
            "edit.undo" => {
                self.apply(UiIntent::Undo);
                true
            }
            "edit.redo" => {
                self.apply(UiIntent::Redo);
                true
            }
            "edit.duplicate" => {
                self.apply(UiIntent::DuplicateActiveAsset);
                true
            }
            "window.command_palette" => {
                self.apply(UiIntent::OpenCommandSearch);
                true
            }
            "window.settings" => {
                self.apply(UiIntent::OpenSettings);
                true
            }
            other => self.execute_core_command(other).is_ok(),
        }
    }

    /// Um passo de autosave, respeitando intervalo e dirty state do domínio.
    ///
    /// Retorna `true` quando um snapshot foi gravado. Autosave nunca limpa o
    /// dirty state nem toca no arquivo oficial.
    pub fn autosave_tick(&mut self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let dirty = self.state.is_document_dirty();
        let path = self.state.project.project_path.clone();
        let path_ref = path.as_deref().map(std::path::Path::new);
        matches!(
            self.autosave
                .tick(now, dirty, &self.state.project.project, path_ref),
            Some(Ok(_))
        )
    }

    /// Carrega o snapshot de recuperação detectado no arranque.
    pub fn recover_pending(&mut self) -> bool {
        let Some(info) = self.pending_recovery.take() else {
            return false;
        };
        self.apply(UiIntent::OpenProjectFrom(info.snapshot_path));
        self.state
            .set_status(format!("Recovered snapshot of '{}'", info.project_name));
        true
    }

    /// Mantém o projeto oficial e encerra o aviso de recuperação.
    pub fn keep_saved_project(&mut self) -> bool {
        self.pending_recovery.take().is_some()
    }

    /// Descarta os snapshots de recuperação e o marcador de sessão.
    pub fn discard_pending_recovery(&mut self) -> bool {
        if self.pending_recovery.take().is_none() {
            return false;
        }
        let path = self.state.project.project_path.clone();
        let path_ref = path.as_deref().map(std::path::Path::new);
        match petunia_core::AutosaveService::discard_recovery(path_ref) {
            Ok(()) => self.state.set_status("Recovery snapshots discarded"),
            Err(error) => self
                .state
                .set_status(format!("Failed to discard snapshots: {error}")),
        }
        true
    }

    /// Define o escopo de preenchimento do pincel.
    pub fn set_fill_scope(&mut self, scope: &str) -> bool {
        let parsed = match scope {
            "ConnectedPixels" => petunia_core::FillScope::ConnectedPixels,
            "Face" => petunia_core::FillScope::Face,
            "SelectedFaces" => petunia_core::FillScope::SelectedFaces,
            "UvIsland" => petunia_core::FillScope::UvIsland,
            "Object" => petunia_core::FillScope::Object,
            _ => return false,
        };
        self.state.session.tools.fill_scope = parsed;
        self.state.set_status(format!("Fill scope: {scope}"));
        true
    }

    /// Define o modo de projeção do pincel.
    pub fn set_brush_projection(&mut self, projection: &str) -> bool {
        let parsed = match projection {
            "Surface" => petunia_core::BrushProjectionMode::Surface,
            "ScreenSpace" => petunia_core::BrushProjectionMode::ScreenSpace,
            _ => return false,
        };
        self.state.session.tools.brush_projection = parsed;
        self.state.set_status(format!("Projection: {projection}"));
        true
    }

    /// Define a trava de pincel.
    pub fn set_brush_lock(&mut self, lock: &str) -> bool {
        let parsed = match lock {
            "None" => petunia_core::BrushLock::None,
            "FirstObject" => petunia_core::BrushLock::FirstObject,
            "FirstFace" => petunia_core::BrushLock::FirstFace,
            "SelectedFaces" => petunia_core::BrushLock::SelectedFaces,
            _ => return false,
        };
        self.state.session.tools.brush_lock = parsed;
        self.state.session.tools.paint_lock_face = None;
        self.state.set_status(format!("Brush lock: {lock}"));
        true
    }

    /// Constrói a geometria 2D do editor UV a partir das UVs reais da malha.
    ///
    /// O quadrado 0..1 vira uma caixa de 256 px; cada face contribui um contorno
    /// fechado. Malhas grandes são truncadas e o editor avisa, em vez de
    /// construir uma string gigante em silêncio.
    fn build_uv_editor(&self) -> UvEditorModel {
        const BOX: f32 = 256.0;
        const MAX_FACES: usize = 2_000;
        let Some(mesh) = self.state.project.active_mesh() else {
            return UvEditorModel::default();
        };
        let face_count = mesh.faces.len();
        let mut commands = String::new();
        let mut seam_commands = String::new();
        let mut selected_commands = String::new();
        let mut pinned_commands = String::new();

        for (face_idx, face) in mesh.faces.iter().enumerate().take(MAX_FACES) {
            if face.uv.len() < 3 {
                continue;
            }
            let is_selected = face.selected || self.state.session.uv_selected.contains(&face_idx);

            // Base wireframe path / Caminho da malha de arame base
            for (index, uv) in face.uv.iter().enumerate() {
                if !uv[0].is_finite() || !uv[1].is_finite() {
                    continue;
                }
                let x = uv[0] * BOX;
                let y = (1.0 - uv[1]) * BOX;
                if index == 0 {
                    commands.push_str(&format!("M {x:.2} {y:.2} "));
                } else {
                    commands.push_str(&format!("L {x:.2} {y:.2} "));
                }
            }
            commands.push_str("Z ");

            // Selected face highlight path / Destaque de faces UV selecionadas
            if is_selected {
                for (index, uv) in face.uv.iter().enumerate() {
                    if !uv[0].is_finite() || !uv[1].is_finite() {
                        continue;
                    }
                    let x = uv[0] * BOX;
                    let y = (1.0 - uv[1]) * BOX;
                    if index == 0 {
                        selected_commands.push_str(&format!("M {x:.2} {y:.2} "));
                    } else {
                        selected_commands.push_str(&format!("L {x:.2} {y:.2} "));
                    }
                }
                selected_commands.push_str("Z ");
            }

            // Pinned UV corners / Vértices UV fixados
            for (c_idx, uv) in face.uv.iter().enumerate() {
                if mesh.uv_pinned.contains(&(face_idx, c_idx))
                    && uv[0].is_finite()
                    && uv[1].is_finite()
                {
                    let cx = uv[0] * BOX;
                    let cy = (1.0 - uv[1]) * BOX;
                    pinned_commands.push_str(&format!(
                        "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                        cx - 3.0,
                        cy,
                        cx,
                        cy - 3.0,
                        cx + 3.0,
                        cy,
                        cx,
                        cy + 3.0
                    ));
                }
            }

            // Highlighted seam edges / Arestas de costura destacadas
            let n = face.verts.len();
            for i in 0..n {
                let v0 = face.verts[i];
                let v1 = face.verts[(i + 1) % n];
                let edge = (v0.min(v1), v0.max(v1));
                if mesh.uv_seams.contains(&edge) && i < face.uv.len() && (i + 1) % n < face.uv.len()
                {
                    let uv0 = face.uv[i];
                    let uv1 = face.uv[(i + 1) % n];
                    if uv0[0].is_finite()
                        && uv0[1].is_finite()
                        && uv1[0].is_finite()
                        && uv1[1].is_finite()
                    {
                        let x0 = uv0[0] * BOX;
                        let y0 = (1.0 - uv0[1]) * BOX;
                        let x1 = uv1[0] * BOX;
                        let y1 = (1.0 - uv1[1]) * BOX;
                        seam_commands.push_str(&format!("M {x0:.2} {y0:.2} L {x1:.2} {y1:.2} "));
                    }
                }
            }
        }
        let islands = mesh.uv_islands();
        UvEditorModel {
            layout_commands: commands,
            seam_commands,
            selected_commands,
            pinned_commands,
            pinned_count: mesh.uv_pinned.len(),
            island_count: islands.len(),
            face_count,
            selected_face: mesh
                .faces
                .iter()
                .position(|face| face.selected)
                .map(|index| index as i32)
                .unwrap_or(-1),
            uv_selected_count: self.state.session.uv_selected.len(),
            truncated: face_count > MAX_FACES,
        }
    }

    /// Clique no editor UV 2D: seleciona a face cuja ilha contém o ponto.
    ///
    /// `u`/`v` chegam normalizados em 0..1 com origem embaixo, que é a
    /// convenção do domínio; o editor desenha com origem em cima e converte
    /// antes de chamar.
    pub fn uv_editor_click(&mut self, u: f32, v: f32, extend: bool) -> bool {
        if !u.is_finite() || !v.is_finite() {
            return false;
        }
        let Some(face) = petunia_module_uv::UvModule::uv_hit(&self.state, u, v) else {
            if !extend {
                self.state.session.uv_selected.clear();
                if let Some(mesh) = self.state.project.active_mesh_mut() {
                    for current in &mut mesh.faces {
                        current.selected = false;
                    }
                }
                self.state.sync_selection();
            }
            self.state.set_status("UV: no face under the cursor");
            return false;
        };
        if extend {
            // Com Shift a seleção acumula e alterna; sem Shift ela é substituída,
            // que é o comportamento previsível de um clique simples.
            if !self.state.session.uv_selected.insert(face) {
                self.state.session.uv_selected.remove(&face);
            }
        } else {
            self.state.session.uv_selected.clear();
            self.state.session.uv_selected.insert(face);
        }
        if let Some(mesh) = self.state.project.active_mesh_mut() {
            if !extend {
                for current in &mut mesh.faces {
                    current.selected = false;
                }
            }
            if let Some(target) = mesh.faces.get_mut(face) {
                target.selected = self.state.session.uv_selected.contains(&face);
            }
        }
        self.state.sync_selection();
        self.state.mark_dirty();
        self.state.set_status(format!(
            "UV: face {face} selected ({} total)",
            self.state.session.uv_selected.len()
        ));
        true
    }

    /// Move as UVs selecionadas (ou todas, quando nada está marcado).
    pub fn uv_move_selected(&mut self, du: f32, dv: f32) -> bool {
        if self.state.session.uv_selected.is_empty() || !du.is_finite() || !dv.is_finite() {
            return false;
        }
        if du == 0.0 && dv == 0.0 {
            return false;
        }
        self.state.checkpoint("move uv");
        petunia_module_uv::UvModule::move_selected(&mut self.state, du, dv);
        self.state
            .set_status(format!("UV moved by ({du:.3}, {dv:.3})"));
        true
    }

    /// Escala as UVs selecionadas em torno do centroide.
    pub fn uv_scale_selected(&mut self, factor: f32) -> bool {
        if self.state.session.uv_selected.is_empty() || !factor.is_finite() || factor <= 0.0 {
            return false;
        }
        self.state.checkpoint("scale uv");
        petunia_module_uv::UvModule::scale_selected(&mut self.state, factor);
        self.state.set_status(format!("UV scaled ×{factor:.3}"));
        true
    }

    /// Rotaciona as UVs selecionadas em torno do centroide.
    pub fn uv_rotate_selected(&mut self, degrees: f32) -> bool {
        if self.state.session.uv_selected.is_empty() || !degrees.is_finite() || degrees == 0.0 {
            return false;
        }
        self.state.checkpoint("rotate uv");
        petunia_module_uv::UvModule::rotate_selected(&mut self.state, degrees.to_radians());
        self.state.set_status(format!("UV rotated {degrees:.1}°"));
        true
    }

    /// Marca ou desmarca como costura as arestas selecionadas ou da face UV selecionada.
    pub fn toggle_selected_uv_seams(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh_mut() else {
            return false;
        };
        let selected_edges: Vec<(u32, u32)> = mesh.selected_edges.iter().copied().collect();
        if !selected_edges.is_empty() {
            self.state.checkpoint("toggle uv seams");
            let Some(mesh) = self.state.project.active_mesh_mut() else {
                return false;
            };
            for (a, b) in selected_edges {
                mesh.toggle_seam(a, b);
            }
            let count = self
                .state
                .project
                .active_mesh()
                .map(|mesh| mesh.uv_seams.len())
                .unwrap_or(0);
            self.state.set_status(format!(
                "UV seams on selected edges toggled ({count} total)"
            ));
            self.state.emit_uv_changed();
            return true;
        }

        let Some(face) = mesh.faces.iter().find(|face| face.selected) else {
            self.state
                .set_status("UV: select a face in the viewport first");
            return false;
        };
        let verts = face.verts.clone();
        if verts.len() < 3 {
            return false;
        }
        self.state.checkpoint("toggle uv seams");
        let Some(mesh) = self.state.project.active_mesh_mut() else {
            return false;
        };
        for index in 0..verts.len() {
            let a = verts[index];
            let b = verts[(index + 1) % verts.len()];
            mesh.toggle_seam(a, b);
        }
        let count = self
            .state
            .project
            .active_mesh()
            .map(|mesh| mesh.uv_seams.len())
            .unwrap_or(0);
        self.state.set_status(format!(
            "UV seams on the selected face toggled ({count} total)"
        ));
        self.state.emit_uv_changed();
        true
    }

    /// Limpa todas as costuras da malha ativa.
    pub fn clear_all_uv_seams(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        if mesh.uv_seams.is_empty() {
            self.state.set_status("UV: there are no seams to clear");
            return false;
        }
        self.state.checkpoint("clear uv seams");
        if let Some(mesh) = self.state.project.active_mesh_mut() {
            mesh.uv_seams.clear();
        }
        self.state.set_status("UV: all seams cleared");
        self.state.emit_uv_changed();
        true
    }

    /// Alterna a fixação (pin/unpin) dos vértices UV das faces selecionadas.
    pub fn toggle_selected_uv_pins(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        let has_sel_faces =
            mesh.faces.iter().any(|f| f.selected) || !self.state.session.uv_selected.is_empty();
        if !has_sel_faces {
            self.state.set_status("UV: select face(s) to pin/unpin");
            return false;
        }

        self.state.checkpoint("toggle uv pin");
        let Some(mesh) = self.state.project.active_mesh_mut() else {
            return false;
        };

        let mut faces_to_toggle = self.state.session.uv_selected.clone();
        for (fi, f) in mesh.faces.iter().enumerate() {
            if f.selected {
                faces_to_toggle.insert(fi);
            }
        }

        mesh.toggle_pin_selected_faces_uv(&faces_to_toggle);
        let count = mesh.uv_pinned.len();
        self.state
            .set_status(format!("UV pins toggled ({count} pinned corners total)"));
        self.state.emit_uv_changed();
        true
    }

    /// Limpa todas as fixações de UV da malha ativa.
    pub fn clear_all_uv_pins(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        if mesh.uv_pinned.is_empty() {
            self.state.set_status("UV: there are no pins to clear");
            return false;
        }
        self.state.checkpoint("clear all uv pins");
        if let Some(mesh) = self.state.project.active_mesh_mut() {
            mesh.clear_all_pins();
        }
        self.state.set_status("UV: all pinned vertices cleared");
        self.state.emit_uv_changed();
        true
    }

    /// Mutações do stack de camadas do workspace PAINT.
    ///
    /// Todas passam por `ensure_stack` + checkpoint + recomposição: o raster
    /// canônico é `Asset.paint_stack` e `Asset.texture` é só o cache composto.
    fn mutate_paint_stack(
        &mut self,
        label: &str,
        mutate: impl FnOnce(&mut petunia_project::paint_layers::PaintLayerStack) -> bool,
    ) -> bool {
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let before = self.state.project.project.clone();
        let active = self.state.project.active;
        let Some(asset) = self.state.project.assets.get_mut(active) else {
            return false;
        };
        let Some(stack) = asset.paint_stack.as_mut() else {
            return false;
        };
        if !mutate(stack) {
            return false;
        }
        self.state.project.checkpoint_snapshot(label, &before);
        petunia_module_paint::PaintModule::composite_active(&mut self.state);
        self.state.mark_dirty();
        true
    }

    pub fn add_paint_layer(&mut self) -> bool {
        let (w, h) = self.paint_canvas_dimensions().unwrap_or((256, 256));
        self.mutate_paint_stack("add paint layer", |stack| {
            stack.add_layer(petunia_project::paint_layers::PaintLayer::new(
                format!("Layer {}", stack.layers.len() + 1),
                w,
                h,
                [0, 0, 0, 0],
            ));
            true
        })
    }

    pub fn add_paint_group(&mut self) -> bool {
        self.mutate_paint_stack("add paint group", |stack| {
            stack.add_group(format!("Group {}", stack.layers.len() + 1));
            true
        })
    }

    pub fn add_decal_layer(&mut self) -> bool {
        self.mutate_paint_stack("add decal layer", |stack| {
            let decal_img = petunia_project::Canvas::new(64, 64, [255, 200, 50, 255]);
            let decal = petunia_project::paint_layers::DecalLayer::new(
                decal_img,
                [0.5, 0.5],
                [0.25, 0.25],
                0.0,
            );
            stack.add_layer(petunia_project::paint_layers::PaintLayer::new_decal(
                format!("Decal {}", stack.layers.len() + 1),
                decal,
            ));
            true
        })
    }

    pub fn toggle_face_orientation(&mut self) -> bool {
        self.state.session.show_face_orientation = !self.state.session.show_face_orientation;
        let enabled = self.state.session.show_face_orientation;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "Orientação de faces (azul/vermelho) ativada"
        } else {
            "Orientação de faces desativada"
        });
        enabled
    }

    pub fn toggle_uv_checker(&mut self) -> bool {
        self.state.session.show_uv_checker = !self.state.session.show_uv_checker;
        let enabled = self.state.session.show_uv_checker;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "UV Checkerboard ativado"
        } else {
            "UV Checkerboard desativado"
        });
        enabled
    }

    pub fn toggle_proportional_editing(&mut self) -> bool {
        self.state.session.proportional_editing = !self.state.session.proportional_editing;
        let enabled = self.state.session.proportional_editing;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "Edição proporcional ativada"
        } else {
            "Edição proporcional desativada"
        });
        enabled
    }

    pub fn set_proportional_radius(&mut self, radius: f32) -> bool {
        if !radius.is_finite() {
            return false;
        }
        let r = radius.clamp(0.01, 100.0);
        self.state.session.proportional_settings.radius = r;
        self.state.render.mark_dirty();
        true
    }

    pub fn adjust_proportional_radius(&mut self, delta: f32) -> bool {
        if !delta.is_finite() {
            return false;
        }
        let current = self.state.session.proportional_settings.radius;
        let new_radius = (current + delta).clamp(0.05, 100.0);
        self.set_proportional_radius(new_radius);
        self.state
            .set_status(format!("Raio proporcional: {:.2}", new_radius));
        true
    }

    pub fn set_proportional_falloff(&mut self, falloff_str: &str) -> bool {
        use petunia_core::ProportionalFalloff;
        let falloff = match falloff_str.to_lowercase().as_str() {
            "smooth" => ProportionalFalloff::Smooth,
            "linear" => ProportionalFalloff::Linear,
            "sphere" => ProportionalFalloff::Sphere,
            "sharp" => ProportionalFalloff::Sharp,
            "constant" => ProportionalFalloff::Constant,
            _ => ProportionalFalloff::Smooth,
        };
        self.state.session.proportional_settings.falloff = falloff;
        self.state.render.mark_dirty();
        true
    }

    pub fn toggle_snap_enabled(&mut self) -> bool {
        self.state.session.snap_enabled = !self.state.session.snap_enabled;
        self.state.session.snap_settings.enabled = self.state.session.snap_enabled;
        let enabled = self.state.session.snap_enabled;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "Snap magnético ativado"
        } else {
            "Snap magnético desativado"
        });
        enabled
    }

    /// Overlay persisted layouts onto the runtime section states.
    /// Called once at startup with the loaded preferences; tests pass
    /// hand-built preferences instead of touching disk.
    /// Sobrepõe os layouts persistidos aos estados das seções.
    /// Chamado uma vez na inicialização com as preferências carregadas; testes
    /// passam preferências construídas à mão em vez de tocar o disco.
    pub fn restore_section_layouts(&mut self, preferences: &petunia_config::UserPreferences) {
        self.preferences = preferences.clone();
        self.tool_session
            .set_drag_threshold_px(preferences.drag_threshold_px);
        self.state.session.snap_settings.radius_pixels =
            petunia_core::clamp_snap_radius(preferences.snap_radius_px);
        self.tool_session
            .set_click_move_click(preferences.click_move_click);
        self.state.profile.workplane_prefer_ground = preferences.workplane_prefer_ground;
        if !preferences.active_keymap_id.is_empty() {
            self.state.ui.active_keymap_id = preferences.active_keymap_id.clone();
            self.state.ui.keybinds =
                petunia_config::Keybinds::load_profile(&preferences.active_keymap_id);
        }
        self.section_layouts = section_layout::restore_section_layouts(preferences);
    }

    pub fn set_keymap_profile(&mut self, profile_id: &str) -> bool {
        if self.state.ui.active_keymap_id == profile_id {
            return false;
        }
        self.state.ui.active_keymap_id = profile_id.to_string();
        self.state.ui.keybinds = petunia_config::Keybinds::load_profile(profile_id);
        self.preferences.active_keymap_id = profile_id.to_string();
        self.state.mark_dirty();
        self.state
            .set_status(format!("Perfil de atalhos ativado: {profile_id}"));
        true
    }

    /// Refresh the cached preferences from live UI state (section layouts are
    /// owned by the cache itself and left untouched here).
    /// Atualiza o cache de preferências a partir do estado vivo da UI (os layouts
    /// pertencem ao próprio cache e ficam intocados aqui).
    pub(crate) fn sync_preferences_from_state(&mut self) {
        self.preferences.invert_vertical_drag = self.state.ui.invert_vertical_drag;
        self.preferences.colorblind_axes = self.state.ui.colorblind_axes;
        self.preferences.reduced_motion = self.state.ui.reduced_motion;
        self.preferences.multiselection_measure_tag = self.state.ui.multiselection_measure_tag;
        self.preferences.selection_rgb = self.state.ui.selection_rgb;
        self.preferences.selection_thickness = self.state.ui.selection_thickness;
        self.preferences.model_quick_actions = self.state.ui.model_quick_actions.clone();
        self.preferences.active_keymap_id = self.state.ui.active_keymap_id.clone();
    }

    /// Persist runtime section layouts; failures surface as status, never panic.
    /// Persiste os layouts das seções; falhas viram status, nunca pânico.
    fn persist_section_layouts(&mut self) {
        self.sync_preferences_from_state();
        for id in petunia_config::InspectorSectionId::all() {
            let layout = self.section_layouts[section_layout::section_index(id)].clone();
            self.preferences.set_section_layout(id, layout);
        }
        let path = self
            .preferences_path_override
            .clone()
            .unwrap_or_else(petunia_config::UserPreferences::default_path);
        if let Err(error) = self.preferences.save_to_path(&path) {
            self.state
                .set_status(format!("section layout save failed: {error}"));
        }
    }

    /// Dock or float one Inspector section, then persist.
    /// Ancora ou flutua uma seção do Inspector e persiste.
    pub fn set_section_docked(
        &mut self,
        section: petunia_config::InspectorSectionId,
        docked: bool,
    ) -> bool {
        section_layout::set_docked(&mut self.section_layouts, section, docked);
        self.persist_section_layouts();
        true
    }

    /// Move a floating card in memory (coordinates sanitized), without I/O.
    ///
    /// A drag emits one event per pointer move; persisting here would write the
    /// preferences file dozens of times per second. The UI commits once on
    /// pointer release through [`Self::commit_section_float`].
    /// Move um card flutuante em memória (coordenadas sanitizadas), sem I/O.
    ///
    /// O arraste emite um evento por movimento do ponteiro; persistir aqui
    /// gravaria o arquivo dezenas de vezes por segundo. A UI confirma uma vez
    /// ao soltar o ponteiro via [`Self::commit_section_float`].
    pub fn move_section_float(
        &mut self,
        section: petunia_config::InspectorSectionId,
        x: f32,
        y: f32,
    ) -> bool {
        section_layout::move_floating(&mut self.section_layouts, section, x, y);
        true
    }

    /// Persist section layouts once, after a drag or any other live edit.
    /// Persiste os layouts uma vez, depois do arraste ou de outra edição viva.
    pub fn commit_section_float(&mut self) {
        self.persist_section_layouts();
    }

    /// Pin a section open (ignores collapse-all), then persist.
    /// Fixa uma seção aberta (ignora recolher-tudo) e persiste.
    pub fn set_section_pin_open(
        &mut self,
        section: petunia_config::InspectorSectionId,
        pin_open: bool,
    ) -> bool {
        section_layout::set_pin_open(&mut self.section_layouts, section, pin_open);
        self.persist_section_layouts();
        true
    }

    /// Pin a section to an asset (`None` follows selection), then persist.
    /// Fixa uma seção a um asset (`None` segue a seleção) e persiste.
    pub fn set_section_pinned_asset(
        &mut self,
        section: petunia_config::InspectorSectionId,
        asset: Option<String>,
    ) -> bool {
        section_layout::set_pinned_asset(&mut self.section_layouts, section, asset);
        self.persist_section_layouts();
        true
    }

    /// Collapse-all toggle honoring pinned-open sections: if every section is
    /// open, close the unpinned ones; otherwise open them. Pinned sections
    /// stay open either way. Takes and returns open flags in canonical order.
    /// Alternador de recolher-tudo respeitando pins: se tudo está aberto, fecha
    /// as não-fixadas; senão, abre-as. Fixadas seguem abertas. Recebe e devolve
    /// flags de aberto em ordem canônica.
    pub fn toggle_all_sections(&self, open: [bool; 6]) -> [bool; 6] {
        let close_all = open.iter().all(|flag| *flag);
        let mut next = open;
        for id in petunia_config::InspectorSectionId::all() {
            if !self.section_layouts[section_layout::section_index(id)].pin_open {
                next[section_layout::section_index(id)] = !close_all;
            }
        }
        next
    }

    /// Presentation snapshot of the six section layouts, in canonical order.
    /// Snapshot de apresentação dos seis layouts, em ordem canônica.
    pub fn section_state_models(&self) -> Vec<SectionStateModel> {
        petunia_config::InspectorSectionId::all()
            .iter()
            .map(|id| {
                let layout = &self.section_layouts[section_layout::section_index(*id)];
                SectionStateModel {
                    id: id.as_str().to_string(),
                    pin_open: layout.pin_open,
                    open: layout.open,
                }
            })
            .collect()
    }

    /// Abre ou fecha uma seção do Inspector (persistido).
    pub fn set_section_open(&mut self, section: petunia_config::InspectorSectionId, open: bool) {
        section_layout::set_open(&mut self.section_layouts, section, open);
        self.persist_section_layouts();
        self.state.mark_dirty();
    }

    /// Alterna estado aberto/fechado de uma seção do Inspector (persistido).
    pub fn toggle_section_open(&mut self, section: petunia_config::InspectorSectionId) -> bool {
        let open = section_layout::toggle_open(&mut self.section_layouts, section);
        self.persist_section_layouts();
        self.state.mark_dirty();
        open
    }

    pub fn set_snap_target(&mut self, target_str: &str) -> bool {
        use petunia_core::SnapTarget;
        let target = match target_str.to_lowercase().as_str() {
            "grid" => SnapTarget::Grid,
            "increment" => SnapTarget::Increment,
            "vertex" | "point" => SnapTarget::Vertex,
            "edge" => SnapTarget::Edge,
            "face" => SnapTarget::Face,
            _ => SnapTarget::Grid,
        };
        self.state.session.snap_settings.target = target;
        self.state.render.mark_dirty();
        self.state
            .set_status(format!("Snap target: {}", target.label()));
        true
    }

    fn replace_profile_shape(&mut self, name: &str, points: Vec<[f64; 3]>) -> bool {
        if points.len() < 3 || points.iter().flatten().any(|value| !value.is_finite()) {
            return false;
        }
        if let Some((_, spline)) = self.active_profile_resources() {
            let mut spline = spline.clone();
            spline.points = points
                .into_iter()
                .map(petunia_core::SplinePoint::new)
                .collect();
            spline.closed = true;
            spline.interpolation = petunia_core::SplineInterpolation::Polyline;
            if let Err(error) = self
                .state
                .dispatch(&petunia_core::UpdateSplineCmd { spline })
            {
                self.state.set_status(error.to_string());
                return false;
            }
        } else {
            let spline = petunia_core::SplineResource::from_polyline(name, &points, true);
            let mut profile =
                petunia_core::ProfileResource::new(name, spline.id, self.draft_profile_workplane());
            profile.wall_thickness = f64::from(self.state.profile.wall_thickness.max(0.0));
            let profile_id = profile.id;
            if let Err(error) = self
                .state
                .dispatch(&petunia_core::CreateProfileCmd { spline, profile })
            {
                self.state.set_status(error.to_string());
                return false;
            }
            self.active_profile_id = Some(profile_id);
        }
        self.profile_selected_point = None;
        self.profile_drag_target = None;
        self.profile_edit_gesture = None;
        self.state.session.tools.active_tool = "draw_profile".to_string();
        true
    }

    pub fn add_profile_rectangle(&mut self, width: f32, height: f32) -> bool {
        self.cancel_profile_volume();
        let width = if width.is_finite() && width > 0.0 {
            width
        } else {
            2.0
        };
        let height = if height.is_finite() && height > 0.0 {
            height
        } else {
            1.5
        };
        let half_width = f64::from(width) * 0.5;
        let half_height = f64::from(height) * 0.5;
        if !self.replace_profile_shape(
            "Rectangle Profile",
            vec![
                [-half_width, -half_height, 0.0],
                [half_width, -half_height, 0.0],
                [half_width, half_height, 0.0],
                [-half_width, half_height, 0.0],
            ],
        ) {
            return false;
        }
        self.state.render.mark_dirty();
        self.state.set_status(format!(
            "Perfil retangular ({width:.1} x {height:.1}) criado"
        ));
        true
    }

    pub fn add_profile_circle(&mut self, radius: f32, segments: usize) -> bool {
        self.cancel_profile_volume();
        let radius = if radius.is_finite() && radius > 0.0 {
            radius
        } else {
            1.0
        };
        let segments = segments.clamp(6, 64);
        let points = (0..segments)
            .map(|index| {
                let angle = std::f64::consts::TAU * index as f64 / segments as f64;
                [
                    f64::from(radius) * angle.cos(),
                    f64::from(radius) * angle.sin(),
                    0.0,
                ]
            })
            .collect();
        if !self.replace_profile_shape("Circle Profile", points) {
            return false;
        }
        self.state.render.mark_dirty();
        self.state.set_status(format!(
            "Perfil circular (raio {radius:.1}, {segments} seg) criado"
        ));
        true
    }

    pub fn set_paint_layer_active(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("activate paint layer", |stack| stack.set_active(id))
    }

    pub fn toggle_paint_layer_visibility(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("toggle paint layer visibility", |stack| {
            let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) else {
                return false;
            };
            layer.visible = !layer.visible;
            true
        })
    }

    pub fn toggle_paint_layer_lock(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("toggle paint layer lock", |stack| {
            let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) else {
                return false;
            };
            layer.locked = !layer.locked;
            true
        })
    }

    pub fn remove_paint_layer(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("remove paint layer", |stack| {
            // A última camada é a base do raster: removê-la deixaria o asset sem
            // superfície de pintura.
            if stack.layers.len() <= 1 {
                return false;
            }
            stack.remove_layer(id)
        })
    }

    pub fn merge_down_paint_layer(&mut self, id: &str) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("merge down paint layer", |stack| {
            let Some(pos) = stack.layers.iter().position(|layer| layer.id == id) else {
                return false;
            };
            if pos == 0 {
                return false;
            }
            stack.merge_down(pos)
        })
    }

    /// Move a camada em `delta` posições na ordem de composição.
    pub fn move_paint_layer(&mut self, id: &str, delta: i32) -> bool {
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.mutate_paint_stack("reorder paint layer", |stack| {
            let Some(from) = stack.layers.iter().position(|layer| layer.id == id) else {
                return false;
            };
            let to = from as i32 + delta;
            if to < 0 || to as usize >= stack.layers.len() {
                return false;
            }
            stack.move_layer(from, to as usize)
        })
    }

    pub fn set_paint_layer_opacity(&mut self, id: &str, opacity: f32) -> bool {
        if !opacity.is_finite() {
            return false;
        }
        let Ok(id) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        let opacity = opacity.clamp(0.0, 1.0);
        self.mutate_paint_stack("paint layer opacity", |stack| {
            let Some(layer) = stack.layers.iter_mut().find(|layer| layer.id == id) else {
                return false;
            };
            if (layer.opacity - opacity).abs() < f32::EPSILON {
                return false;
            }
            layer.opacity = opacity;
            true
        })
    }

    /// Abre a sessão de loop cut a partir da aresta selecionada.
    pub fn begin_loop_cut(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh().cloned() else {
            self.state.set_status("Loop Cut: no active mesh");
            return false;
        };
        let Some(seed) = mesh.selected_edges.iter().copied().next() else {
            self.state
                .set_status("Loop Cut: select an edge on a quad ring first");
            return false;
        };
        let Ok(ring) = petunia_core::LoopRing::discover(&mesh, seed) else {
            self.state
                .set_status("Loop Cut: the selected edge is not on a quad ring");
            return false;
        };
        self.begin_loop_cut_from_ring(ring, mesh, 1)
    }

    fn begin_loop_cut_from_ring(
        &mut self,
        ring: petunia_core::LoopRing,
        mesh: petunia_core::Mesh,
        cuts: usize,
    ) -> bool {
        self.state.freeze_active_primitive();
        let initial_cuts = if self.loop_cut_balanced && cuts < 2 {
            2
        } else {
            cuts.clamp(1, 32)
        };
        self.loop_cut = Some(LoopCutSessionState {
            ring,
            cuts: initial_cuts,
            slide: 0.0,
            balanced: self.loop_cut_balanced,
            source: mesh,
        });
        self.state.session.tools.active_tool = "loop_cut".to_string();
        if !self.apply_loop_cut_preview() {
            self.loop_cut = None;
            return false;
        }
        self.state
            .set_status("Loop Cut: drag to slide, Enter confirms, Esc cancels");
        true
    }

    /// Alterna o modo de corte simétrico / equilibrado (Dual Balanced Loop Rings).
    pub fn toggle_loop_cut_balanced(&mut self) -> bool {
        self.loop_cut_balanced = !self.loop_cut_balanced;
        let balanced = self.loop_cut_balanced;
        if balanced && self.loop_cut_hover_cuts < 2 {
            self.loop_cut_hover_cuts = 2;
        }
        if let Some(session) = self.loop_cut.as_mut() {
            session.balanced = balanced;
            if balanced && session.cuts < 2 {
                session.cuts = 2;
            }
        }
        if self.loop_cut.is_some() {
            self.apply_loop_cut_preview();
        }
        self.state.set_status(if balanced {
            "Loop Cut: Dual Balanced ativado"
        } else {
            "Loop Cut: Dual Balanced desativado"
        });
        self.state.mark_dirty();
        balanced
    }

    pub fn set_loop_cut_balanced(&mut self, balanced: bool) -> bool {
        if self.loop_cut_balanced == balanced {
            return false;
        }
        self.loop_cut_balanced = balanced;
        if balanced && self.loop_cut_hover_cuts < 2 {
            self.loop_cut_hover_cuts = 2;
        }
        if let Some(session) = self.loop_cut.as_mut() {
            session.balanced = balanced;
            if balanced && session.cuts < 2 {
                session.cuts = 2;
            }
        }
        if self.loop_cut.is_some() {
            self.apply_loop_cut_preview();
        }
        self.state.mark_dirty();
        true
    }

    /// Ajusta o slide do loop cut em 2D considerando a projeção em tela da direção de corte.
    pub fn scrub_loop_cut_2d(&mut self, delta_x: f32, delta_y: f32, fine: bool) -> bool {
        let step = if fine { 0.0025 } else { 0.01 };
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        let effective_delta = if let Some(first_edge) = session.ring.edges().first() {
            if let (Some(va), Some(vb)) = (
                session.source.verts.get(first_edge.0 as usize),
                session.source.verts.get(first_edge.1 as usize),
            ) {
                let vp = self.state.session.camera.view_proj();
                let pa = vp.project_point3(va.vec());
                let pb = vp.project_point3(vb.vec());
                let screen_dir = glam::Vec2::new(pb.x - pa.x, pb.y - pa.y);
                if screen_dir.length_squared() > 1e-4 {
                    let norm_dir = screen_dir.normalize();
                    let mouse_delta = glam::Vec2::new(delta_x, -delta_y);
                    mouse_delta.dot(norm_dir)
                } else {
                    delta_x - delta_y
                }
            } else {
                delta_x - delta_y
            }
        } else {
            delta_x - delta_y
        };
        session.slide = (session.slide + effective_delta * step).clamp(-1.0, 1.0);
        self.apply_loop_cut_preview()
    }

    /// Ajusta o slide do loop cut e reconstrói a pré-visualização.
    pub fn scrub_loop_cut(&mut self, delta_x: f32, fine: bool) -> bool {
        let step = if fine { 0.0025 } else { 0.01 };
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        session.slide = (session.slide + delta_x * step).clamp(-1.0, 1.0);
        self.apply_loop_cut_preview()
    }

    /// Campo numérico do Slide: não altera a quantidade de cortes.
    pub fn set_loop_cut_slide(&mut self, slide: f32) -> bool {
        if !slide.is_finite() || !(-1.0..=1.0).contains(&slide) {
            self.state
                .set_status("Loop Cut: slide must be between -1 and 1");
            return false;
        }
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        if (session.slide - slide).abs() <= f32::EPSILON {
            return false;
        }
        let previous = session.slide;
        session.slide = slide;
        if self.apply_loop_cut_preview() {
            true
        } else {
            if let Some(session) = self.loop_cut.as_mut() {
                session.slide = previous;
            }
            false
        }
    }

    /// Ajusta a quantidade de cortes paralelos (1..=32).
    pub fn set_loop_cut_count(&mut self, cuts: usize) -> bool {
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        let clamped = cuts.clamp(1, 32);
        if session.cuts == clamped {
            return false;
        }
        session.cuts = clamped;
        self.apply_loop_cut_preview()
    }

    pub fn scroll_loop_cut_count(&mut self, delta_y: f32) -> bool {
        if !delta_y.is_finite() || delta_y.abs() < f32::EPSILON {
            return false;
        }
        let delta = if delta_y > 0.0 { 1 } else { -1 };
        if self.loop_cut.is_some() {
            let current = self.loop_cut.as_ref().map_or(1, |session| session.cuts);
            return self.set_loop_cut_count((current as i32 + delta).clamp(1, 32) as usize);
        }
        self.adjust_loop_cut_hover_count(delta)
    }

    pub fn adjust_loop_cut_count_from_input(&mut self, cuts: usize) -> bool {
        if self.loop_cut.is_some() {
            return self.set_loop_cut_count(cuts);
        }
        let next = cuts.clamp(1, 32);
        let delta = next as i32 - self.loop_cut_hover_cuts as i32;
        self.adjust_loop_cut_hover_count(delta)
    }

    /// Reconstrói a malha a partir do snapshot da sessão, nunca do preview.
    fn apply_loop_cut_preview(&mut self) -> bool {
        let Some(session) = self.loop_cut.as_ref() else {
            return false;
        };
        let res = if session.balanced {
            session
                .ring
                .apply_balanced(&session.source, session.cuts, session.slide)
        } else {
            session
                .ring
                .apply(&session.source, session.cuts, session.slide)
        };
        match res {
            Ok(mesh) => {
                if let Some(active) = self.state.project.active_mesh_mut() {
                    *active = mesh;
                }
                self.state.emit_mesh_changed();
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(format!("Loop Cut: {error}"));
                false
            }
        }
    }

    /// Confirma o loop cut como uma única operação de undo.
    pub fn commit_loop_cut(&mut self) -> bool {
        let Some(session) = self.loop_cut.take() else {
            return false;
        };
        self.state.session.tools.active_tool = "select".to_string();
        // O checkpoint precisa capturar a malha ANTES do corte, então o preview
        // é desfeito primeiro e o resultado final é reaplicado depois.
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = session.source.clone();
        }
        let Ok(cut) = (if session.balanced {
            session
                .ring
                .apply_balanced(&session.source, session.cuts, session.slide)
        } else {
            session
                .ring
                .apply(&session.source, session.cuts, session.slide)
        }) else {
            self.state
                .set_status("Loop Cut: topology refused at commit");
            self.state.emit_mesh_changed();
            return false;
        };
        self.state.freeze_active_primitive();
        self.state.checkpoint("loop cut");
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = cut;
        }
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.set_status(format!(
            "Loop cut ({}{})",
            session.cuts,
            if session.balanced { " balanced" } else { "" }
        ));
        true
    }

    /// Abandona a sessão restaurando a malha original.
    pub fn cancel_loop_cut(&mut self) -> bool {
        let Some(session) = self.loop_cut.take() else {
            return false;
        };
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = session.source;
        }
        self.state.session.tools.active_tool = "select".to_string();
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.set_status("Loop Cut cancelled");
        true
    }

    /// Abre o plano de corte (Slice) ancorado no ponto pressionado.
    pub fn begin_slice(&mut self, x: f32, y: f32) -> bool {
        let mut adjusting_existing = false;
        let mut anchor_pt = [x, y];
        if let Some(old_anchor) = self.slice_anchor {
            let d_anchor = (x - old_anchor[0]).hypot(y - old_anchor[1]);
            let d_end = (x - self.pointer_position[0]).hypot(y - self.pointer_position[1]);
            if d_anchor <= 28.0 {
                adjusting_existing = true;
                anchor_pt = self.pointer_position;
                self.pointer_position = [x, y];
            } else if d_end <= 28.0 {
                adjusting_existing = true;
                anchor_pt = old_anchor;
                self.pointer_position = [x, y];
            } else {
                if self.state.session.tools.cut_session.is_some() {
                    self.commit_slice();
                }
                self.pointer_position = [x, y];
            }
        } else {
            self.pointer_position = [x, y];
        }

        if adjusting_existing {
            if let Some(session) = &self.state.session.tools.cut_session
                && let Some(active) = self.state.project.active_mesh_mut()
            {
                *active = session.source.clone();
            }
            if let Some(session) = self.state.session.tools.cut_session.as_mut() {
                session.fill_cap = self.slice_trim;
            }
        } else {
            let Some(mesh) = self.state.project.active_mesh().cloned() else {
                self.state.set_status("Slice: no active mesh");
                return false;
            };
            let mut session = petunia_core::CutSession::new(mesh);
            session.fill_cap = self.slice_trim;
            self.state.session.tools.cut_session = Some(session);
        }

        self.state.session.tools.active_tool = "slice".to_string();
        self.slice_anchor = Some(anchor_pt);
        self.state.set_status(
            "Slice: arraste para ajustar · T: Trim/Split · Enter/Clique para confirmar · Esc para cancelar",
        );
        true
    }

    /// Atualiza a pré-visualização do plano de corte.
    pub fn update_slice(&mut self, x: f32, y: f32) -> bool {
        self.update_slice_modified(x, y, false)
    }

    /// Atualiza a pré-visualização do plano de corte com restrição angular quando `snap` é verdadeiro.
    pub fn update_slice_modified(&mut self, x: f32, y: f32, snap: bool) -> bool {
        let Some(anchor) = self.slice_anchor else {
            return false;
        };
        let (target_x, target_y) = if snap {
            let dx = x - anchor[0];
            let dy = y - anchor[1];
            let dist = dx.hypot(dy);
            if dist > 1e-4 {
                let angle = dy.atan2(dx);
                let step = std::f32::consts::PI / 12.0; // 15 graus
                let snapped = (angle / step).round() * step;
                (
                    anchor[0] + dist * snapped.cos(),
                    anchor[1] + dist * snapped.sin(),
                )
            } else {
                (x, y)
            }
        } else {
            (x, y)
        };
        self.pointer_position = [target_x, target_y];
        let viewport = petunia_core::LogicalRect::from_min_max(
            [0.0, 0.0],
            [self.viewport_size[0], self.viewport_size[1]],
        );
        if let Some(session) = self.state.session.tools.cut_session.as_mut() {
            session.fill_cap = self.slice_trim;
        }
        let Some(session) = self.state.session.tools.cut_session.as_ref() else {
            return false;
        };
        let Some(sliced) = session.compute_slice(
            &self.state.session.camera,
            anchor,
            [target_x, target_y],
            viewport,
        ) else {
            return false;
        };
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = sliced;
        }
        self.state.emit_mesh_changed();
        self.state.mark_dirty();
        true
    }

    /// Confirma o corte como uma única operação de undo.
    pub fn commit_slice(&mut self) -> bool {
        // A faca compartilha `cut_session`, então o Slice só age quando é ele que
        // está armado — senão um release cancelaria o corte da faca.
        if self.slice_anchor.is_none() && self.state.session.tools.active_tool != "slice" {
            return false;
        }
        if self.slice_anchor.is_none() {
            self.state
                .set_status("Slice: clique e arraste para traçar a linha de corte");
            return false;
        }
        let Some(session) = self.state.session.tools.cut_session.take() else {
            self.slice_anchor = None;
            return false;
        };
        self.slice_anchor = None;
        self.state.session.tools.active_tool = "select".to_string();
        // A pré-visualização já está na malha: restaurar o snapshot, capturar e
        // reaplicar o corte garante que o undo volte ao estado anterior.
        let Some(current) = self.state.project.active_mesh().cloned() else {
            return false;
        };
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = session.source.clone();
        }
        self.state.checkpoint("slice");
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = current;
        }
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.mark_dirty();
        self.state.set_status("Slice applied");
        true
    }

    /// Abandona o plano de corte restaurando a malha original.
    pub fn cancel_slice(&mut self) -> bool {
        if self.slice_anchor.is_none() && self.state.session.tools.active_tool != "slice" {
            return false;
        }
        let Some(session) = self.state.session.tools.cut_session.take() else {
            self.slice_anchor = None;
            if self.state.session.tools.active_tool == "slice" {
                self.state.session.tools.active_tool = "select".to_string();
                self.state.set_status("Slice cancelled");
                return true;
            }
            return false;
        };
        self.slice_anchor = None;
        self.state.session.tools.active_tool = "select".to_string();
        if let Some(active) = self.state.project.active_mesh_mut() {
            *active = session.source;
        }
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.mark_dirty();
        self.state.set_status("Slice cancelled");
        true
    }

    pub fn set_slice_trim(&mut self, trim: bool) {
        self.slice_trim = trim;
        if let Some(session) = self.state.session.tools.cut_session.as_mut() {
            session.fill_cap = trim;
        }
        if let Some(anchor) = self.slice_anchor {
            let viewport = petunia_core::LogicalRect::from_min_max(
                [0.0, 0.0],
                [self.viewport_size[0], self.viewport_size[1]],
            );
            if let Some(session) = self.state.session.tools.cut_session.as_ref()
                && let Some(sliced) = session.compute_slice(
                    &self.state.session.camera,
                    anchor,
                    self.pointer_position,
                    viewport,
                )
            {
                if let Some(active) = self.state.project.active_mesh_mut() {
                    *active = sliced;
                }
                self.state.emit_mesh_changed();
                self.state.mark_dirty();
            }
        }
    }

    pub fn toggle_bevel_clamp_overlap(&mut self) -> bool {
        self.state.tools.bevel_clamp_overlap = !self.state.tools.bevel_clamp_overlap;
        let enabled = self.state.tools.bevel_clamp_overlap;
        self.state.set_status(if enabled {
            "Round Edge: Clamp Overlap ativado"
        } else {
            "Round Edge: Clamp Overlap desativado"
        });
        self.state.render.mark_dirty();
        enabled
    }

    pub fn toggle_bevel_affect_vertices(&mut self) -> bool {
        self.state.tools.bevel_affect_vertices = !self.state.tools.bevel_affect_vertices;
        let enabled = self.state.tools.bevel_affect_vertices;
        self.state.set_status(if enabled {
            "Round Edge: Point (Vertex) Mode ativado"
        } else {
            "Round Edge: Point (Vertex) Mode desativado"
        });
        self.state.render.mark_dirty();
        enabled
    }

    pub fn uv_equalize_texel_density(&mut self) -> bool {
        match petunia_module_uv::UvModule::equalize_texel_density(&mut self.state) {
            Ok(_) => {
                self.state.render.mark_dirty();
                true
            }
            Err(e) => {
                self.state.set_status(format!("Texel density: {e}"));
                false
            }
        }
    }

    pub fn active_primitive_descriptor_or_session(
        &self,
    ) -> Option<petunia_core::PrimitiveDescriptor> {
        if let Some(session) = &self.state.session.primitive_session {
            Some(session.descriptor)
        } else {
            self.state.active_primitive_descriptor()
        }
    }

    pub fn freeze_active_primitive(&mut self) -> bool {
        let frozen = self.state.freeze_active_primitive();
        if frozen {
            self.state.render.mark_dirty();
        }
        frozen
    }

    pub fn update_primitive_param_float(&mut self, param: &str, value: f32) -> bool {
        let Some(desc) = self.active_primitive_descriptor_or_session() else {
            return false;
        };
        use petunia_core::PrimitiveDescriptor::*;
        let new_desc = match desc {
            Box {
                width,
                height,
                depth,
            } => match param {
                "width" => Box {
                    width: value.max(0.01),
                    height,
                    depth,
                },
                "height" => Box {
                    width,
                    height: value.max(0.01),
                    depth,
                },
                "depth" => Box {
                    width,
                    height,
                    depth: value.max(0.01),
                },
                _ => return false,
            },
            Plane { width, height } => match param {
                "width" => Plane {
                    width: value.max(0.01),
                    height,
                },
                "height" => Plane {
                    width,
                    height: value.max(0.01),
                },
                _ => return false,
            },
            Wedge {
                width,
                height,
                depth,
            } => match param {
                "width" => Wedge {
                    width: value.max(0.01),
                    height,
                    depth,
                },
                "height" => Wedge {
                    width,
                    height: value.max(0.01),
                    depth,
                },
                "depth" => Wedge {
                    width,
                    height,
                    depth: value.max(0.01),
                },
                _ => return false,
            },
            Cylinder {
                radius,
                height,
                sides,
                cap_top,
                cap_bottom,
            } => match param {
                "radius" => Cylinder {
                    radius: value.max(0.01),
                    height,
                    sides,
                    cap_top,
                    cap_bottom,
                },
                "height" => Cylinder {
                    radius,
                    height: value.max(0.01),
                    sides,
                    cap_top,
                    cap_bottom,
                },
                "sides" => Cylinder {
                    radius,
                    height,
                    sides: (value.round() as u32).clamp(3, 32),
                    cap_top,
                    cap_bottom,
                },
                _ => return false,
            },
            Cone {
                bottom_radius,
                top_radius,
                height,
                sides,
                cap_bottom,
                cap_top,
            } => match param {
                "radius" => Cone {
                    bottom_radius: value.max(0.0),
                    top_radius,
                    height,
                    sides,
                    cap_bottom,
                    cap_top,
                },
                "radius_b" => Cone {
                    bottom_radius,
                    top_radius: value.max(0.0),
                    height,
                    sides,
                    cap_bottom,
                    cap_top,
                },
                "height" => Cone {
                    bottom_radius,
                    top_radius,
                    height: value.max(0.01),
                    sides,
                    cap_bottom,
                    cap_top,
                },
                "sides" => Cone {
                    bottom_radius,
                    top_radius,
                    height,
                    sides: (value.round() as u32).clamp(3, 32),
                    cap_bottom,
                    cap_top,
                },
                _ => return false,
            },
            Circle {
                radius,
                vertices,
                fill,
            } => match param {
                "radius" => Circle {
                    radius: value.max(0.01),
                    vertices,
                    fill,
                },
                "sides" => Circle {
                    radius,
                    vertices: (value.round() as u32).clamp(3, 64),
                    fill,
                },
                _ => return false,
            },
            Torus {
                major_radius,
                minor_radius,
                major_segments,
                minor_segments,
            } => match param {
                "radius" => Torus {
                    major_radius: value.max(0.01),
                    minor_radius,
                    major_segments,
                    minor_segments,
                },
                "radius_b" => Torus {
                    major_radius,
                    minor_radius: value.max(0.01),
                    major_segments,
                    minor_segments,
                },
                "sides" => Torus {
                    major_radius,
                    minor_radius,
                    major_segments: (value.round() as u32).clamp(3, 64),
                    minor_segments,
                },
                "rings" => Torus {
                    major_radius,
                    minor_radius,
                    major_segments,
                    minor_segments: (value.round() as u32).clamp(3, 32),
                },
                _ => return false,
            },
            LowSphere {
                radius,
                segments,
                rings,
            } => match param {
                "radius" => LowSphere {
                    radius: value.max(0.01),
                    segments,
                    rings,
                },
                "sides" => LowSphere {
                    radius,
                    segments: (value.round() as u32).clamp(3, 32),
                    rings,
                },
                "rings" => LowSphere {
                    radius,
                    segments,
                    rings: (value.round() as u32).clamp(2, 24),
                },
                _ => return false,
            },
            Icosphere { radius, subdiv } => match param {
                "radius" => Icosphere {
                    radius: value.max(0.01),
                    subdiv,
                },
                "subdiv" => Icosphere {
                    radius,
                    subdiv: (value.round() as u32).min(3),
                },
                _ => return false,
            },
            Capsule {
                radius,
                height,
                radial_segments,
                cap_segments,
            } => match param {
                "radius" => Capsule {
                    radius: value.max(0.01),
                    height,
                    radial_segments,
                    cap_segments,
                },
                "height" => Capsule {
                    radius,
                    height: value.max(0.01),
                    radial_segments,
                    cap_segments,
                },
                "sides" => Capsule {
                    radius,
                    height,
                    radial_segments: (value.round() as u32).clamp(3, 32),
                    cap_segments,
                },
                "rings" => Capsule {
                    radius,
                    height,
                    radial_segments,
                    cap_segments: (value.round() as u32).clamp(1, 8),
                },
                _ => return false,
            },
        };
        self.state
            .update_active_primitive(new_desc, !self.state.primitive_session_valid())
    }

    pub fn update_primitive_param_bool(&mut self, param: &str, value: bool) -> bool {
        let Some(desc) = self.active_primitive_descriptor_or_session() else {
            return false;
        };
        use petunia_core::PrimitiveDescriptor::*;
        let new_desc = match desc {
            Cylinder {
                radius,
                height,
                sides,
                cap_top,
                cap_bottom,
            } => match param {
                "cap_top" => Cylinder {
                    radius,
                    height,
                    sides,
                    cap_top: value,
                    cap_bottom,
                },
                "cap_bottom" => Cylinder {
                    radius,
                    height,
                    sides,
                    cap_top,
                    cap_bottom: value,
                },
                _ => return false,
            },
            Cone {
                bottom_radius,
                top_radius,
                height,
                sides,
                cap_bottom,
                cap_top,
            } => match param {
                "cap_top" => Cone {
                    bottom_radius,
                    top_radius,
                    height,
                    sides,
                    cap_bottom,
                    cap_top: value,
                },
                "cap_bottom" => Cone {
                    bottom_radius,
                    top_radius,
                    height,
                    sides,
                    cap_bottom: value,
                    cap_top,
                },
                _ => return false,
            },
            Circle {
                radius, vertices, ..
            } => match param {
                "fill_disc" => Circle {
                    radius,
                    vertices,
                    fill: if value {
                        petunia_core::CircleFill::Disc
                    } else {
                        petunia_core::CircleFill::None
                    },
                },
                _ => return false,
            },
            _ => return false,
        };
        self.state
            .update_active_primitive(new_desc, !self.state.primitive_session_valid())
    }

    pub fn confirm_primitive(&mut self) -> bool {
        self.state.confirm_primitive()
    }

    pub fn cancel_primitive(&mut self) -> bool {
        self.state.cancel_primitive()
    }

    fn delete_selected_profile_point(&mut self) -> bool {
        self.profile_pointer_up();
        let Some(point_id) = self.profile_selected_point else {
            return false;
        };
        let Some((_, spline)) = self.active_profile_resources() else {
            return false;
        };
        let spline_id = spline.id;
        if let Err(error) = self.state.dispatch(&petunia_core::DeleteSplinePointCmd {
            spline_id,
            point_id,
        }) {
            self.state.set_status(error.to_string());
            return false;
        }
        self.profile_selected_point = None;
        self.profile_drag_target = None;
        if self.profile_volume_mode.is_some() {
            self.update_profile_volume_preview();
        }
        true
    }

    /// Exclui ou dissolve elementos selecionados ou o ativo ativo se em Object Mode.
    pub fn delete_or_dissolve_selection(&mut self) -> bool {
        if self.state.session.tools.active_tool == "draw_profile"
            && self.profile_selected_point.is_some()
        {
            return self.delete_selected_profile_point();
        }

        if self.state.selection_domain() != SelectionDomain::Object {
            let hover = self.state.hover;
            let Some(mesh) = self.state.project.active_mesh_mut() else {
                return false;
            };

            let has_selection = mesh.verts.iter().any(|v| v.selected)
                || mesh.faces.iter().any(|f| f.selected)
                || !mesh.selected_edges.is_empty();

            if !has_selection {
                match hover {
                    petunia_core::HoverTarget::Edge(a, b) => {
                        mesh.selected_edges.insert((a.min(b), a.max(b)));
                    }
                    petunia_core::HoverTarget::Vertex(v) => {
                        if let Some(vert) = mesh.verts.get_mut(v as usize) {
                            vert.selected = true;
                        }
                    }
                    petunia_core::HoverTarget::Face(f) => {
                        if let Some(face) = mesh.faces.get_mut(f) {
                            face.selected = true;
                        }
                    }
                    _ => {}
                }
            }

            let has_selection_now = mesh.verts.iter().any(|v| v.selected)
                || mesh.faces.iter().any(|f| f.selected)
                || !mesh.selected_edges.is_empty();

            if !has_selection_now {
                return false;
            }
            self.state.sync_selection();
            if self.state.selection_domain() == SelectionDomain::Face
                || self
                    .state
                    .project
                    .active_mesh()
                    .is_some_and(|m| m.faces.iter().any(|f| f.selected))
            {
                self.state
                    .dispatch(&petunia_core::DeleteSelectionCmd)
                    .is_ok()
            } else {
                self.state
                    .dispatch(&petunia_core::DeleteOrDissolveSelectionCmd)
                    .is_ok()
            }
        } else {
            self.apply(UiIntent::DeleteActiveAsset);
            true
        }
    }

    /// Copia para a área de transferência a geometria selecionada (ou ativo em Object Mode).
    pub fn copy_selection(&mut self) -> bool {
        if self.state.selection_domain() != SelectionDomain::Object {
            let Some(mesh) = self.state.project.active_mesh() else {
                return false;
            };

            let sel_faces: Vec<&petunia_mesh::Face> =
                mesh.faces.iter().filter(|f| f.selected).collect();
            if !sel_faces.is_empty() {
                let mut vert_map: std::collections::HashMap<u32, u32> =
                    std::collections::HashMap::new();
                let mut new_verts = Vec::new();
                for f in &sel_faces {
                    for &v in &f.verts {
                        if !vert_map.contains_key(&v)
                            && let Some(src_v) = mesh.verts.get(v as usize)
                        {
                            let new_idx = new_verts.len() as u32;
                            vert_map.insert(v, new_idx);
                            new_verts.push(src_v.clone());
                        }
                    }
                }
                let mut new_faces = Vec::new();
                for f in &sel_faces {
                    let mut remapped_verts = Vec::new();
                    for &v in &f.verts {
                        if let Some(&new_v) = vert_map.get(&v) {
                            remapped_verts.push(new_v);
                        }
                    }
                    let mut new_face = (*f).clone();
                    new_face.verts = remapped_verts;
                    new_faces.push(new_face);
                }
                let sub_mesh = petunia_mesh::Mesh {
                    verts: new_verts,
                    faces: new_faces,
                    ..Default::default()
                };
                self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                self.state.set_status("Copied selected faces to clipboard");
                return true;
            }

            if !mesh.selected_edges.is_empty() {
                let mut vert_indices = std::collections::HashSet::new();
                for &(a, b) in &mesh.selected_edges {
                    vert_indices.insert(a);
                    vert_indices.insert(b);
                }
                let mut vert_map: std::collections::HashMap<u32, u32> =
                    std::collections::HashMap::new();
                let mut new_verts = Vec::new();
                for &v in &vert_indices {
                    if let Some(src_v) = mesh.verts.get(v as usize) {
                        let new_idx = new_verts.len() as u32;
                        vert_map.insert(v, new_idx);
                        new_verts.push(src_v.clone());
                    }
                }
                let mut new_faces = Vec::new();
                for f in &mesh.faces {
                    if f.verts.iter().all(|v| vert_indices.contains(v)) {
                        let mut remapped = Vec::new();
                        for &v in &f.verts {
                            if let Some(&new_v) = vert_map.get(&v) {
                                remapped.push(new_v);
                            }
                        }
                        let mut new_f = f.clone();
                        new_f.verts = remapped;
                        new_faces.push(new_f);
                    }
                }
                let mut new_selected_edges = std::collections::HashSet::new();
                for &(a, b) in &mesh.selected_edges {
                    if let (Some(&na), Some(&nb)) = (vert_map.get(&a), vert_map.get(&b)) {
                        new_selected_edges.insert((na.min(nb), na.max(nb)));
                    }
                }
                let sub_mesh = petunia_mesh::Mesh {
                    verts: new_verts,
                    faces: new_faces,
                    selected_edges: new_selected_edges,
                    ..Default::default()
                };
                self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                self.state.set_status("Copied selected edges to clipboard");
                return true;
            }

            let sel_verts: Vec<(usize, &petunia_mesh::Vertex)> = mesh
                .verts
                .iter()
                .enumerate()
                .filter(|(_, v)| v.selected)
                .collect();
            if !sel_verts.is_empty() {
                let vert_indices: std::collections::HashSet<u32> =
                    sel_verts.iter().map(|(i, _)| *i as u32).collect();
                let mut vert_map: std::collections::HashMap<u32, u32> =
                    std::collections::HashMap::new();
                let mut new_verts = Vec::new();
                for (i, v) in &sel_verts {
                    let new_idx = new_verts.len() as u32;
                    vert_map.insert(*i as u32, new_idx);
                    new_verts.push((*v).clone());
                }
                let mut new_faces = Vec::new();
                for f in &mesh.faces {
                    if f.verts.iter().all(|v| vert_indices.contains(v)) {
                        let mut remapped = Vec::new();
                        for &v in &f.verts {
                            if let Some(&new_v) = vert_map.get(&v) {
                                remapped.push(new_v);
                            }
                        }
                        let mut new_f = f.clone();
                        new_f.verts = remapped;
                        new_faces.push(new_f);
                    }
                }
                let sub_mesh = petunia_mesh::Mesh {
                    verts: new_verts,
                    faces: new_faces,
                    ..Default::default()
                };
                self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                self.state
                    .set_status("Copied selected vertices to clipboard");
                return true;
            }

            match self.state.hover {
                petunia_core::HoverTarget::Face(f_idx) => {
                    if let Some(f) = mesh.faces.get(f_idx) {
                        let mut new_verts = Vec::new();
                        let mut remapped = Vec::new();
                        for (new_i, &v) in f.verts.iter().enumerate() {
                            if let Some(src_v) = mesh.verts.get(v as usize) {
                                new_verts.push(src_v.clone());
                                remapped.push(new_i as u32);
                            }
                        }
                        let mut new_f = f.clone();
                        new_f.verts = remapped;
                        let sub_mesh = petunia_mesh::Mesh {
                            verts: new_verts,
                            faces: vec![new_f],
                            ..Default::default()
                        };
                        self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                        self.state.set_status("Copied hovered face to clipboard");
                        return true;
                    }
                }
                petunia_core::HoverTarget::Edge(a, b) => {
                    if let (Some(va), Some(vb)) =
                        (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                    {
                        let mut new_edges = std::collections::HashSet::new();
                        new_edges.insert((0, 1));
                        let sub_mesh = petunia_mesh::Mesh {
                            verts: vec![va.clone(), vb.clone()],
                            selected_edges: new_edges,
                            ..Default::default()
                        };
                        self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                        self.state.set_status("Copied hovered edge to clipboard");
                        return true;
                    }
                }
                petunia_core::HoverTarget::Vertex(v_idx) => {
                    if let Some(v) = mesh.verts.get(v_idx as usize) {
                        let sub_mesh = petunia_mesh::Mesh {
                            verts: vec![v.clone()],
                            ..Default::default()
                        };
                        self.clipboard = Some(GeometryClipboard::Mesh(sub_mesh));
                        self.state.set_status("Copied hovered vertex to clipboard");
                        return true;
                    }
                }
                _ => {}
            }

            false
        } else {
            let Some(asset) = self.state.project.active() else {
                return false;
            };
            self.clipboard = Some(GeometryClipboard::Asset(Box::new(asset.clone())));
            self.state
                .set_status(format!("Copied '{}' to clipboard", asset.name));
            true
        }
    }

    /// Cola a geometria da área de transferência como um novo objeto separado.
    pub fn paste_clipboard(&mut self) -> bool {
        let Some(clipboard_data) = self.clipboard.clone() else {
            return false;
        };
        match clipboard_data {
            GeometryClipboard::Mesh(mut mesh) => {
                let base_name = self
                    .state
                    .project
                    .active()
                    .map(|a| a.name.clone())
                    .unwrap_or_else(|| "part".to_string());
                let new_name = format!("{base_name}_part");

                mesh.select_all();

                self.state
                    .checkpoint(&format!("paste geometry: {new_name}"));
                self.state.project.add(&new_name, mesh);
                let new_index = self.state.project.assets.len() - 1;
                self.state.set_selection_domain(SelectionDomain::Object);
                self.state.select_object(Some(new_index), false);
                self.state.emit_mesh_changed();
                self.state.mark_dirty();
                self.state
                    .set_status(format!("Pasted separate object '{new_name}'"));
                true
            }
            GeometryClipboard::Asset(asset) => {
                let mut copy = asset.duplicate();
                copy.name = format!("{}_copy", asset.name);
                for v in &mut copy.mesh.verts {
                    v.pos[0] += 0.5;
                }
                let copy_name = copy.name.clone();
                self.state.checkpoint(&format!("paste asset: {copy_name}"));
                self.state.project.assets.push(copy);
                let new_index = self.state.project.assets.len() - 1;
                self.state.set_selection_domain(SelectionDomain::Object);
                self.state.select_object(Some(new_index), false);
                self.state.emit_mesh_changed();
                self.state.mark_dirty();
                self.state
                    .set_status(format!("Pasted object '{copy_name}'"));
                true
            }
        }
    }

    /// Duplica a seleção: elementos na malha em Edit Mode, ou ativo em Object Mode.
    pub fn duplicate_selection(&mut self) -> bool {
        if self.state.selection_domain() != SelectionDomain::Object {
            let hover = self.state.hover;
            if let Some(mesh) = self.state.project.active_mesh_mut() {
                let has_selection = mesh.verts.iter().any(|v| v.selected)
                    || mesh.faces.iter().any(|f| f.selected)
                    || !mesh.selected_edges.is_empty();
                if !has_selection {
                    match hover {
                        petunia_core::HoverTarget::Face(f) => {
                            if let Some(face) = mesh.faces.get_mut(f) {
                                face.selected = true;
                            }
                        }
                        petunia_core::HoverTarget::Edge(a, b) => {
                            mesh.selected_edges.insert((a.min(b), a.max(b)));
                            if let Some(va) = mesh.verts.get_mut(a as usize) {
                                va.selected = true;
                            }
                            if let Some(vb) = mesh.verts.get_mut(b as usize) {
                                vb.selected = true;
                            }
                        }
                        petunia_core::HoverTarget::Vertex(v) => {
                            if let Some(vert) = mesh.verts.get_mut(v as usize) {
                                vert.selected = true;
                            }
                        }
                        _ => {}
                    }
                }
                mesh.sync_vert_selection_from_faces();
            }
            self.state.sync_selection();

            self.state
                .dispatch(&petunia_core::DuplicateSelectionCmd)
                .is_ok()
        } else {
            self.state
                .dispatch(&petunia_core::DuplicateAssetCmd { asset_index: None })
                .is_ok()
        }
    }

    /// Alterna o modo de extrusão entre Região e Faces Individuais mantendo a distância.
    pub fn switch_extrude_mode(&mut self) -> bool {
        let Some(current) = self.tool_modal else {
            return false;
        };
        let new_kind = match current {
            ToolModalKind::Extrude => ToolModalKind::ExtrudeIndividual,
            ToolModalKind::ExtrudeIndividual => ToolModalKind::Extrude,
            _ => return false,
        };
        let current_value = self.tool_modal_value;
        let _ = self.state.cancel_modal();
        if self.begin_tool_modal(new_kind) {
            self.tool_modal_value = current_value;
            let _ = self.state.update_modal(glam::Vec3::ZERO, current_value);
            self.state.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Um clique de faca na viewport: primeiro ponto ancora, segundo corta.
    ///
    /// O ponto vem de `AppState::pick_edge`, então a faca corta a aresta que o
    /// usuário realmente apontou. O corte é aplicado como uma única transação.
    pub fn knife_click(&mut self, normalized_x: f32, normalized_y: f32) -> bool {
        if self.state.session.tools.cut_session.is_none() {
            return false;
        }
        if !normalized_x.is_finite() || !normalized_y.is_finite() {
            return false;
        }
        let petunia_core::HoverTarget::Edge(a, b) =
            self.pick_target_for_domain(SelectionDomain::Edge, normalized_x, normalized_y)
        else {
            self.state.set_status("Cut: point at a visible edge");
            return false;
        };
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        let vp = self.state.session.camera.view_proj();
        let va = mesh.verts[a as usize].vec();
        let vb = mesh.verts[b as usize].vec();
        let ca = vp * va.extend(1.0);
        let cb = vp * vb.extend(1.0);
        let screen = |p: glam::Vec4| {
            [
                (p.x / p.w * 0.5 + 0.5) * self.viewport_size[0],
                (0.5 - p.y / p.w * 0.5) * self.viewport_size[1],
            ]
        };
        let pa = screen(ca);
        let pb = screen(cb);
        let mouse = [
            normalized_x * self.viewport_size[0],
            normalized_y * self.viewport_size[1],
        ];
        let delta = [pb[0] - pa[0], pb[1] - pa[1]];
        let length = delta[0] * delta[0] + delta[1] * delta[1];
        if length <= 1.0e-6 {
            return false;
        }
        let t = (((mouse[0] - pa[0]) * delta[0] + (mouse[1] - pa[1]) * delta[1]) / length)
            .clamp(0.0, 1.0);
        let t = (t / cb.w) / ((1.0 - t) / ca.w + t / cb.w);
        let edge = (a, b);
        let position = va.lerp(vb, t);
        let point = petunia_core::CutEdgePoint { edge, position };

        let cut_result = {
            let Some(session) = self.state.session.tools.cut_session.as_mut() else {
                return false;
            };
            let Some(start) = session.edge_start else {
                session.edge_start = Some(point);
                session.anchor = Some([normalized_x, normalized_y]);
                self.state.set_status("Knife: pick the second edge point");
                self.state.mark_dirty();
                return true;
            };

            let Some(mesh) = self.state.project.active_mesh().cloned() else {
                return false;
            };
            session.cut_knife_segment(start, point, &mesh)
        };

        match cut_result {
            Ok(cut) => {
                self.state.freeze_active_primitive();
                if let Some(session) = self.state.session.tools.cut_session.as_mut() {
                    session.segments += 1;
                    session.edge_start = None;
                    session.anchor = None;
                }
                if let Some(active) = self.state.project.active_mesh_mut() {
                    *active = cut;
                }
                self.state.session.tools.hover = petunia_core::HoverTarget::None;
                self.state.sync_selection();
                self.state.emit_mesh_changed();
                self.state
                    .set_status("Cut preview: choose another segment, Enter applies, Esc restores");
                true
            }
            Err(error) => {
                self.state.set_status(format!("Cut: {error}"));
                false
            }
        }
    }

    /// Commit all knife segments as one undo record.
    pub fn commit_knife(&mut self) -> bool {
        if self.state.session.tools.active_tool != "cut" {
            return false;
        }
        let Some(session) = self.state.session.tools.cut_session.take() else {
            return false;
        };
        self.state.session.tools.active_tool = "select".into();
        if session.segments > 0 {
            self.state.freeze_active_primitive();
            let Some(result) = self.state.project.active_mesh().cloned() else {
                return false;
            };
            if let Some(mesh) = self.state.project.active_mesh_mut() {
                *mesh = session.source;
            }
            self.state.checkpoint("cut segments");
            if let Some(mesh) = self.state.project.active_mesh_mut() {
                *mesh = result;
            }
        }
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.set_status("Cut applied");
        true
    }

    pub fn cancel_knife(&mut self) -> bool {
        let Some(session) = self.state.session.tools.cut_session.take() else {
            return false;
        };
        if let Some(mesh) = self.state.project.active_mesh_mut() {
            *mesh = session.source;
        }
        self.state.session.tools.active_tool = "select".into();
        self.state.sync_selection();
        self.state.emit_mesh_changed();
        self.state.set_status("Cut cancelled");
        true
    }

    /// Instancia uma cópia do asset da biblioteca no cursor 3D.
    ///
    /// É o caminho que o comando canônico `model.instantiate_asset` não tinha:
    /// ele exige um `asset_id`, então a palette não consegue disparar sozinha.
    pub fn place_asset(&mut self, id: &str) -> bool {
        let Ok(asset) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        self.state.set_selection_domain(SelectionDomain::Object);
        self.sync_viewport_context();
        if !self.state.instantiate_asset_by_id(asset, None) {
            self.state.set_status("Asset not found in project library");
            return false;
        }
        self.state.mark_dirty();
        true
    }

    /// Define o operando B das operações booleanas.
    pub fn set_boolean_operand(&mut self, id: &str) -> bool {
        let Ok(asset) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        if !self.state.project.assets.iter().any(|a| a.id == asset) {
            return false;
        }
        self.state.session.tools.boolean_operand = Some(asset);
        self.state
            .set_status("Boolean operand set: Fuse, Cut or Intersect now applies");
        true
    }

    /// Liga/desliga o modificador **Keep Parts**.
    pub fn set_boolean_keep_parts(&mut self, keep: bool) -> bool {
        if self.state.session.tools.boolean_keep_parts == keep {
            return false;
        }
        self.state.session.tools.boolean_keep_parts = keep;
        self.state.set_status(if keep {
            "Keep Parts on: the operand stays in the scene"
        } else {
            "Keep Parts off: the operand is consumed"
        });
        true
    }

    /// **Join** pelo id canônico.
    pub fn join_operand(&mut self) -> bool {
        match self.execute_core_command("model.join") {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn clear_boolean_operand(&mut self) -> bool {
        if self.state.session.tools.boolean_operand.take().is_none() {
            return false;
        }
        self.state.set_status("Boolean operand cleared");
        true
    }

    /// Executa Fuse/Cut/Intersect pelo id canônico do comando.
    pub fn boolean_op(&mut self, id: &str) -> bool {
        if self.state.session.tools.boolean_operand.is_none()
            && self.state.project.assets.len() == 2
        {
            let active_id = self.state.project.active().map(|a| a.id);
            if let Some(other) = self
                .state
                .project
                .assets
                .iter()
                .find(|a| Some(a.id) != active_id)
            {
                self.state.session.tools.boolean_operand = Some(other.id);
            }
        }
        match self.execute_core_command(id) {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Abre o menu de contexto do Outliner sobre a linha de `id`.
    ///
    /// O alvo é selecionado antes de abrir: as ações do menu operam sobre ele e
    /// um clique-direito em linha não selecionada precisa agir no que o usuário
    /// apontou, não no que estava ativo.
    pub fn open_context_menu(&mut self, id: &str, x: f32, y: f32) -> bool {
        let Ok(asset) = uuid::Uuid::parse_str(id) else {
            return false;
        };
        if !self.state.project.assets.iter().any(|a| a.id == asset) {
            return false;
        }
        self.select_asset_by_id(asset);
        self.context_menu = Some(ContextMenuState {
            x,
            y,
            asset,
            viewport: false,
        });
        self.overlays.push(OverlayEntry {
            id: OverlayId::OutlinerContextMenu,
            kind: OverlayKind::ContextMenu,
            pinned: false,
            dismiss_on_escape: true,
            dismiss_on_click_away: true,
        });
        self.state.mark_dirty();
        true
    }

    /// Menu da viewport (botão direito no 3D): só verbetes de seleção, sem
    /// alvo de asset. Id de overlay próprio para o Escape LIFO fechar o menu
    /// certo quando Outliner e viewport competem.
    pub fn open_viewport_context_menu(&mut self, x: f32, y: f32) -> bool {
        self.context_menu = Some(ContextMenuState {
            x,
            y,
            asset: uuid::Uuid::nil(),
            viewport: true,
        });
        self.overlays.push(OverlayEntry {
            id: OverlayId::ContextMenu,
            kind: OverlayKind::ContextMenu,
            pinned: false,
            dismiss_on_escape: true,
            dismiss_on_click_away: true,
        });
        self.state.mark_dirty();
        true
    }

    /// Botão direito na viewport: abre o menu de contexto e nunca cancela
    /// (constituição 11, ADR 007). Durante um gesto ou operação aberta ele é
    /// ignorado; cancelar é `Esc`. Retorna verdadeiro quando abriu o menu.
    pub fn viewport_context_triage(&mut self, x: f32, y: f32) -> bool {
        let _ = self.cancel_rename();
        if self.tool_session.is_gesture_active()
            || self.drag.is_some()
            || self.tool_modal.is_some()
            || self.state.session.tools.modal.is_some()
        {
            return false;
        }
        self.open_viewport_context_menu(x, y)
    }

    pub fn close_context_menu(&mut self) -> bool {
        self.overlays.remove(OverlayId::OutlinerContextMenu);
        self.overlays.remove(OverlayId::ContextMenu);
        self.context_menu.take().is_some()
    }

    /// Executa uma ação do menu de contexto sobre o alvo apontado.
    pub fn context_menu_action(&mut self, action: &str) -> bool {
        let Some(menu) = self.context_menu else {
            return false;
        };
        // Modo viewport: só verbetes de seleção, sem alvo de asset.
        if menu.viewport {
            self.close_context_menu();
            return match action {
                "select_all" => {
                    self.apply(UiIntent::SelectAll);
                    true
                }
                "clear_selection" => {
                    self.apply(UiIntent::ClearSelection);
                    true
                }
                "invert_selection" => {
                    self.apply(UiIntent::InvertSelection);
                    true
                }
                "frame" => {
                    let _ = self.state.dispatch_command("view.frame_selection");
                    true
                }
                "delete" => {
                    if self.state.session.tools.active_tool == "draw_profile" {
                        self.delete_selected_profile_point()
                    } else if self.state.selection_domain().is_component() {
                        let _ = self.execute_core_command("model.delete");
                        true
                    } else {
                        self.apply(UiIntent::DeleteActiveAsset);
                        true
                    }
                }
                "duplicate" => {
                    self.apply(UiIntent::DuplicateActiveAsset);
                    true
                }
                "extrude" => {
                    if self.state.session.tools.active_tool == "draw_profile"
                        && (self.active_profile_closed() || self.active_profile_point_count() >= 3)
                    {
                        if self.profile_volume_mode
                            == Some(petunia_module_model::ProfileVolumeMode::Extrude)
                        {
                            self.commit_profile_volume();
                        } else {
                            self.enter_profile_volume("extrude");
                        }
                    } else {
                        self.execute_shortcut_tool("model.extrude");
                    }
                    true
                }
                "inset" => {
                    self.execute_shortcut_tool("model.inset");
                    true
                }
                "bevel" => {
                    self.execute_shortcut_tool("model.bevel");
                    true
                }
                "subdivide" => {
                    let _ = self.execute_core_command("model.subdivide");
                    true
                }
                "loop_cut" => {
                    self.apply(UiIntent::SetActiveTool("loop_cut".to_string()));
                    true
                }
                "connect" => {
                    let _ = self.execute_core_command("model.connect");
                    true
                }
                "make_face" => {
                    let _ = self.execute_core_command("model.make_face");
                    true
                }
                "merge" => {
                    let _ = self.execute_core_command("model.merge");
                    true
                }
                "dissolve" => {
                    let _ = self.execute_core_command("model.dissolve");
                    true
                }
                "slice" => {
                    self.apply(UiIntent::SetActiveTool("slice".to_string()));
                    true
                }
                "flip_normals" => {
                    let _ = self.execute_core_command("model.flip_normals");
                    true
                }
                "mark_seam" => self.toggle_selected_uv_seams(),
                "shade_smooth" => self.execute_core_command("model.shade_smooth").is_ok(),
                "shade_flat" => self.execute_core_command("model.shade_flat").is_ok(),
                "boolean_operand" => {
                    if let Some(active) = self.state.project.active() {
                        self.state.session.tools.boolean_operand = Some(active.id);
                        self.state.set_status("Definido como operando booleano");
                        true
                    } else {
                        false
                    }
                }
                "origin_to_geometry" => self.set_origin_geometry(),
                "origin_to_bottom" => self.set_origin_bottom(),
                "origin_to_cursor" => self.set_origin_cursor(),
                "origin_to_selection" => self.set_origin_selection(),
                "geometry_to_origin" => self.set_geometry_to_origin(),
                "toggle_edit_pivot" => self.toggle_edit_pivot(),
                _ => false,
            };
        }
        let id = menu.asset.to_string();
        self.close_context_menu();
        match action {
            "rename" => self.begin_rename(),
            "duplicate" => {
                self.select_asset_by_id(menu.asset);
                self.apply(UiIntent::DuplicateActiveAsset);
                true
            }
            "visibility" => {
                self.toggle_asset_visibility(&id);
                true
            }
            "lock" => {
                self.toggle_asset_lock(&id);
                true
            }
            "isolate" => {
                // Toggles isolation for the targeted asset / Alterna isolamento para o asset alvo
                self.select_asset_by_id(menu.asset);
                self.state.toggle_isolate();
                true
            }
            "move_up" => {
                // Moves target asset up in the scene list / Move o asset alvo para cima na lista da cena
                if let Some(idx) = self.state.project.find(menu.asset)
                    && idx > 0
                {
                    let _ = self.state.dispatch(&petunia_core::ReorderAssetCmd {
                        from: idx,
                        to: idx - 1,
                    });
                    return true;
                }
                false
            }
            "move_down" => {
                // Moves target asset down in the scene list / Move o asset alvo para baixo na lista da cena
                if let Some(idx) = self.state.project.find(menu.asset)
                    && idx + 1 < self.state.project.assets.len()
                {
                    let _ = self.state.dispatch(&petunia_core::ReorderAssetCmd {
                        from: idx,
                        to: idx + 1,
                    });
                    return true;
                }
                false
            }
            "frame" => {
                self.select_asset_by_id(menu.asset);
                let _ = self.state.dispatch_command("view.frame_selection");
                true
            }
            "boolean_operand" => self.set_boolean_operand(&id),
            "origin_to_geometry" => {
                self.select_asset_by_id(menu.asset);
                self.set_origin_geometry()
            }
            "origin_to_bottom" => {
                self.select_asset_by_id(menu.asset);
                self.set_origin_bottom()
            }
            "origin_to_cursor" => {
                self.select_asset_by_id(menu.asset);
                self.set_origin_cursor()
            }
            "origin_to_selection" => {
                self.select_asset_by_id(menu.asset);
                self.set_origin_selection()
            }
            "geometry_to_origin" => {
                self.select_asset_by_id(menu.asset);
                self.set_geometry_to_origin()
            }
            "toggle_edit_pivot" => {
                self.select_asset_by_id(menu.asset);
                self.toggle_edit_pivot()
            }
            "delete" => {
                self.select_asset_by_id(menu.asset);
                self.apply(UiIntent::DeleteActiveAsset);
                true
            }
            _ => false,
        }
    }

    fn select_asset_by_id(&mut self, asset: uuid::Uuid) {
        if let Some(index) = self
            .state
            .project
            .assets
            .iter()
            .position(|candidate| candidate.id == asset)
        {
            self.cancel_active_operation();
            self.state.select_object(Some(index), false);
        }
    }

    fn toggle_asset_visibility(&mut self, id: &str) {
        self.apply(UiIntent::ToggleSceneAssetVisibility(id.to_string()));
    }

    fn toggle_asset_lock(&mut self, id: &str) {
        self.apply(UiIntent::ToggleSceneAssetLock(id.to_string()));
    }

    /// Abre a edição inline do nome do ativo selecionado.
    pub fn begin_rename(&mut self) -> bool {
        if self.rename_draft.is_some() {
            return true;
        }
        let Some(asset) = self.state.project.active() else {
            self.state
                .set_status(petunia_core::AssetRenameError::NoActiveAsset.to_string());
            return false;
        };
        self.rename_draft = Some(asset.name.clone());
        self.state.mark_dirty();
        true
    }

    /// Confirma o nome em edição. O domínio decide validade e histórico.
    pub fn commit_rename(&mut self, name: &str) -> bool {
        if self.rename_draft.take().is_none() {
            return false;
        }
        match self.state.rename_active_asset(name) {
            Ok(_) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Abandona a edição sem tocar no documento.
    pub fn cancel_rename(&mut self) -> bool {
        if self.rename_draft.take().is_none() {
            return false;
        }
        self.state.mark_dirty();
        true
    }

    /// Ferramenta ativa que segue a gramática única, se houver.
    pub fn grammar_tool(&self) -> Option<GrammarTool> {
        if self.state.workspace != Workspace::Model {
            return None;
        }
        let parametric = |fallback: ToolModalKind| {
            let kind = self
                .tool_modal
                .or(self.parametric_tool)
                .filter(|kind| parametric_tool_id(*kind) == parametric_tool_id(fallback))
                .unwrap_or(fallback);
            Some(GrammarTool::Parametric(kind))
        };
        match self.state.session.tools.active_tool.as_str() {
            "move" | "transform" => Some(GrammarTool::Transform(TransformKind::Position)),
            "rotate" => Some(GrammarTool::Transform(TransformKind::Rotation)),
            "scale" => Some(GrammarTool::Transform(TransformKind::Scale)),
            "extrude" => parametric(ToolModalKind::Extrude),
            "inset" => parametric(ToolModalKind::Inset),
            "bevel" => parametric(ToolModalKind::Bevel),
            "push_pull" => parametric(ToolModalKind::PushPull),
            "poly_pen" => Some(GrammarTool::PolyPen),
            _ => None,
        }
    }

    /// O botão principal na viewport é roteado pela gramática única.
    ///
    /// Operações abertas por teclado (modal estilo Blender) e o Loop Cut
    /// continuam nos caminhos próprios até a migração das Ondas 4 e 5.
    pub fn tool_grammar_active(&self) -> bool {
        self.grammar_tool().is_some()
            && !self.keyboard_tool_modal_active
            && !self.instant_transform
            && self.loop_cut.is_none()
    }

    /// Ativa uma ferramenta paramétrica persistente sem abrir operação.
    ///
    /// Em Object, troca de forma visível para o domínio exigido; nunca
    /// seleciona tudo por conta própria (constituição 11).
    pub fn activate_parametric_tool(&mut self, kind: ToolModalKind) {
        if self.tool_modal.is_some() {
            self.cancel_tool_modal();
        }
        self.tool_session.reset();
        self.tool_gesture = None;
        self.parametric_tool = Some(kind);
        self.state.session.tools.active_tool = parametric_tool_id(kind).to_string();
        let domain = if kind == ToolModalKind::Bevel {
            SelectionDomain::Edge
        } else {
            SelectionDomain::Face
        };
        if self.state.selection_domain() == SelectionDomain::Object {
            self.state.set_selection_domain(domain);
            self.sync_viewport_context();
        }
        let message = self
            .state
            .t_id(petunia_config::text_id::TOOL_GRAMMAR_READY)
            .replace("{tool}", kind.title());
        self.state.set_status(message);
        self.state.mark_dirty();
    }

    /// Eventos do botão principal na viewport: 0 = pressionar, 1 = mover,
    /// 2 = soltar. Coordenadas em px lógicos da viewport.
    pub fn tool_pointer(&mut self, phase: i32, x: f32, y: f32, shift: bool, ctrl: bool) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        self.pointer_position = [x, y];
        let effect = match phase {
            0 => {
                let target = if self.gizmo_target_at(x, y).is_some() {
                    petunia_core::PressTarget::Handle(0)
                } else {
                    petunia_core::PressTarget::Surface
                };
                self.tool_press_extend = shift;
                self.tool_press_alternate = ctrl;
                self.tool_session.press([x, y], target)
            }
            1 => self.tool_session.move_to([x, y]),
            2 => self.tool_session.release([x, y]),
            _ => {
                if self.tool_session.is_gesture_active() {
                    self.tool_session.key(petunia_core::ToolKey::Cancel)
                } else {
                    self.tool_session.reset();
                    petunia_core::ToolEffect::Nothing
                }
            }
        };
        self.apply_tool_effect(effect, shift, ctrl)
    }

    fn apply_tool_effect(
        &mut self,
        effect: petunia_core::ToolEffect,
        fine: bool,
        snap: bool,
    ) -> bool {
        use petunia_core::ToolEffect as Effect;
        match effect {
            Effect::Nothing | Effect::TypedValue(_) | Effect::TypedCleared => false,
            Effect::Click { at } if self.grammar_tool() == Some(GrammarTool::PolyPen) => {
                self.poly_pen_click(at)
            }
            Effect::Click { at } => {
                let [width, height] = self.viewport_size;
                if width > 1.0 && height > 1.0 {
                    self.select_viewport_ext(
                        at[0] / width,
                        at[1] / height,
                        self.tool_press_extend,
                        false,
                    );
                }
                true
            }
            Effect::BeginDrag {
                anchor,
                current,
                target,
            } => {
                if !self.begin_tool_gesture(anchor, target) {
                    self.tool_session.reset();
                    return true;
                }
                self.update_tool_gesture(current, fine, snap);
                true
            }
            Effect::UpdateDrag { current, .. } => self.update_tool_gesture(current, fine, snap),
            Effect::Commit => self.commit_tool_gesture(),
            Effect::Cancel => self.cancel_tool_gesture(),
            Effect::ExitTool => self.exit_grammar_tool(),
            Effect::OpenContextMenu { at } => self.open_viewport_context_menu(at[0], at[1]),
        }
    }

    fn begin_tool_gesture(&mut self, anchor: [f32; 2], target: petunia_core::PressTarget) -> bool {
        let Some(tool) = self.grammar_tool() else {
            return false;
        };
        let on_handle = matches!(target, petunia_core::PressTarget::Handle(_));
        match tool {
            GrammarTool::Transform(kind) => {
                let begun = if on_handle {
                    self.begin_gizmo_drag(anchor[0], anchor[1])
                } else {
                    self.begin_viewport_transform(kind, anchor[0], anchor[1])
                };
                if begun {
                    self.tool_gesture = Some(ToolGesture::Transform { gizmo: on_handle });
                }
                begun
            }
            GrammarTool::PolyPen => self.begin_poly_pen_drag(anchor),
            GrammarTool::Parametric(kind) => {
                if self.tool_modal.is_none()
                    && kind == ToolModalKind::PushPull
                    && let Some(hit) = self.region_hit_at(anchor)
                {
                    return self.begin_region_gesture(anchor, &hit);
                }
                if self.tool_modal.is_none() {
                    self.select_under_anchor_for(kind, anchor);
                    if !self.begin_tool_modal(kind) {
                        let id = if kind == ToolModalKind::Bevel {
                            petunia_config::text_id::TOOL_GRAMMAR_NEEDS_EDGE
                        } else {
                            petunia_config::text_id::TOOL_GRAMMAR_NEEDS_FACE
                        };
                        let message = self.state.t_id(id).replace("{tool}", kind.title());
                        self.state.set_status(message);
                        return false;
                    }
                }
                let kind = self.tool_modal.unwrap_or(kind);
                let Some(frame) = self.drag_frame() else {
                    self.cancel_tool_modal();
                    return false;
                };
                self.tool_gesture = Some(ToolGesture::Parametric {
                    kind,
                    frame,
                    anchor,
                    start_value: self.tool_modal_value,
                });
                true
            }
        }
    }

    /// Elemento sob o cursor para o Poly Pen: ponto > aresta > face, com as
    /// mesmas tolerâncias em pixels do picking de seleção.
    fn poly_pen_target(&self, pixel: [f32; 2]) -> petunia_core::HoverTarget {
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 {
            return petunia_core::HoverTarget::None;
        }
        let (x, y) = (pixel[0] / width, pixel[1] / height);
        [
            SelectionDomain::Vertex,
            SelectionDomain::Edge,
            SelectionDomain::Face,
        ]
        .into_iter()
        .map(|domain| self.pick_target_for_domain(domain, x, y))
        .find(|target| target.is_some())
        .unwrap_or_default()
    }

    /// Ponto de mundo para um ponto novo do Poly Pen: snap (se ligado), face
    /// sob o cursor ou plano de frente para a câmera pelo último ponto.
    fn poly_pen_world_point(&self, pixel: [f32; 2]) -> Option<glam::Vec3> {
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 {
            return None;
        }
        let mesh = self.state.project.active_mesh()?;
        let last = self.poly_pen_points.last().map(|point| match *point {
            petunia_core::PenPoint::Existing(index) => mesh
                .verts
                .get(index as usize)
                .map_or(glam::Vec3::ZERO, |v| v.vec()),
            petunia_core::PenPoint::New(position) => glam::Vec3::from(position),
        });
        if self.state.session.snap_enabled {
            let mask = petunia_core::SnapMask::for_target(self.state.session.snap_settings.target);
            if let Some(hit) = self.screen_snap(
                glam::Vec2::from_array(pixel),
                mask,
                last.map(petunia_core::SnapAnchor::world),
                None,
            ) {
                return Some(hit.point);
            }
        }
        let camera = &self.state.session.camera;
        let ndc = glam::Vec2::new(pixel[0] / width * 2.0 - 1.0, 1.0 - pixel[1] / height * 2.0);
        if let Some(hit) = petunia_core::picking::pick_mesh(
            mesh,
            camera,
            glam::Vec2::new(width, height),
            ndc,
            petunia_core::SelectMode::Face,
            false,
        ) {
            return Some(hit.position);
        }
        let anchor = last.unwrap_or_else(|| mesh.center());
        let normal = camera.forward();
        let (origin, direction) = camera.ray(ndc.x, ndc.y);
        let denominator = direction.dot(normal);
        if denominator.abs() < 1.0e-6 {
            return None;
        }
        let distance = (anchor - origin).dot(normal) / denominator;
        let point = origin + direction * distance;
        point.is_finite().then_some(point)
    }

    /// Clique do Poly Pen: Ctrl-clique derrete o ponto; senão coleta um ponto
    /// do polígono (clicar no primeiro ponto fecha).
    fn poly_pen_click(&mut self, at: [f32; 2]) -> bool {
        let target = self.poly_pen_target(at);
        if self.tool_press_alternate {
            if let petunia_core::HoverTarget::Vertex(point) = target {
                if let Err(error) = self.state.poly_pen_melt_point(point) {
                    self.state.set_status(error.to_string());
                }
                return true;
            }
            return false;
        }
        let point = match target {
            petunia_core::HoverTarget::Vertex(index) => petunia_core::PenPoint::Existing(index),
            _ => match self.poly_pen_world_point(at) {
                Some(position) => petunia_core::PenPoint::New(position.to_array()),
                None => return false,
            },
        };
        if self.poly_pen_points.len() >= 3 && self.poly_pen_points.first() == Some(&point) {
            return self.poly_pen_close();
        }
        if self.poly_pen_points.contains(&point) {
            return false;
        }
        self.poly_pen_points.push(point);
        self.state.mark_dirty();
        true
    }

    /// Fecha o polígono coletado (Enter ou clique no primeiro ponto).
    fn poly_pen_close(&mut self) -> bool {
        let points = std::mem::take(&mut self.poly_pen_points);
        match self.state.poly_pen_add_polygon(&points) {
            Ok(_) => {
                self.sync_viewport_context();
                true
            }
            Err(error) => {
                // Mantém os pontos para o usuário corrigir.
                self.poly_pen_points = points;
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Arrasto do Poly Pen: move o elemento sob o cursor (sem selecionar
    /// antes) ou, com o modificador alternativo sobre uma aresta de borda,
    /// extruda a aresta. Durante a coleta de pontos, arrastar não edita.
    fn begin_poly_pen_drag(&mut self, anchor: [f32; 2]) -> bool {
        if !self.poly_pen_points.is_empty() {
            return false;
        }
        let target = self.poly_pen_target(anchor);
        if self.tool_press_alternate
            && let petunia_core::HoverTarget::Edge(a, b) = target
        {
            if let Err(error) = self.state.begin_poly_pen_edge_extrude(a, b) {
                self.state.set_status(error.to_string());
                return false;
            }
            self.start_viewport_drag(TransformKind::Position, anchor[0], anchor[1]);
            self.tool_gesture = Some(ToolGesture::Transform { gizmo: false });
            self.sync_viewport_context();
            return true;
        }
        let domain = match target {
            petunia_core::HoverTarget::Vertex(_) => SelectionDomain::Vertex,
            petunia_core::HoverTarget::Edge(..) => SelectionDomain::Edge,
            petunia_core::HoverTarget::Face(_) => SelectionDomain::Face,
            _ => return false,
        };
        // O elemento arrastado passa a ser a seleção (visível), como no C4D.
        self.state.set_selection_domain(domain);
        let [width, height] = self.viewport_size;
        self.select_viewport_ext(anchor[0] / width, anchor[1] / height, false, false);
        self.sync_viewport_context();
        let begun = self.begin_viewport_transform(TransformKind::Position, anchor[0], anchor[1]);
        if begun {
            self.tool_gesture = Some(ToolGesture::Transform { gizmo: false });
        }
        begun
    }

    /// Contorno dos pontos coletados até o cursor, em px da viewport.
    fn poly_pen_preview_commands(&self) -> String {
        use std::fmt::Write as _;
        let Some(mesh) = self.state.project.active_mesh() else {
            return String::new();
        };
        if self.poly_pen_points.is_empty() {
            return String::new();
        }
        let [width, height] = self.viewport_size;
        let matrix = self.state.session.camera.view_proj();
        let mut commands = String::new();
        for (index, point) in self.poly_pen_points.iter().enumerate() {
            let world = match *point {
                petunia_core::PenPoint::Existing(i) => match mesh.verts.get(i as usize) {
                    Some(vertex) => vertex.vec(),
                    None => return String::new(),
                },
                petunia_core::PenPoint::New(position) => glam::Vec3::from(position),
            };
            let clip = matrix * world.extend(1.0);
            if !clip.is_finite() || clip.w <= 0.05 {
                return String::new();
            }
            let x = (clip.x / clip.w * 0.5 + 0.5) * width;
            let y = (0.5 - clip.y / clip.w * 0.5) * height;
            let action = if index == 0 { 'M' } else { 'L' };
            let _ = write!(commands, "{action} {x:.2} {y:.2} ");
        }
        let [x, y] = self.pointer_position;
        let _ = write!(commands, "L {x:.2} {y:.2}");
        commands
    }

    /// Arestas por workspace: DRAW/POLY seguem o modo; PAINT e UV mantêm as
    /// faces limpas com o overlay opcional.
    fn edge_mode(&self) -> petunia_render_wgpu::EdgeMode {
        if self.state.workspace == Workspace::Model {
            self.modeling_mode.edge_mode()
        } else {
            petunia_render_wgpu::EdgeMode::Overlay
        }
    }

    /// Região de perfil sob o ponto (px lógicos da viewport), se não estiver
    /// escondida atrás da malha ativa. Os planos ficam em cache por revisão.
    fn region_hit_at(&mut self, pixel: [f32; 2]) -> Option<petunia_core::RegionHit> {
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 || !pixel[0].is_finite() || !pixel[1].is_finite() {
            return None;
        }
        let revision = self.state.project.project.revision_clock();
        if self
            .region_planes_cache
            .as_ref()
            .is_none_or(|(cached, _)| *cached != revision)
        {
            self.region_planes_cache = Some((revision, self.state.profile_region_planes()));
        }
        let planes = &self.region_planes_cache.as_ref()?.1;
        if planes.is_empty() {
            return None;
        }
        let ndc = glam::Vec2::new(pixel[0] / width * 2.0 - 1.0, 1.0 - pixel[1] / height * 2.0);
        let camera = &self.state.session.camera;
        let (origin, direction) = camera.ray(ndc.x, ndc.y);
        let hit = petunia_core::region_at_ray(
            planes,
            origin,
            direction,
            camera.proj == petunia_core::Projection::Perspective,
        )?;
        // Uma face da malha ativa claramente à frente esconde a região.
        let occluder = self.state.project.active_mesh().and_then(|mesh| {
            petunia_core::picking::pick_mesh(
                mesh,
                camera,
                glam::Vec2::new(width, height),
                ndc,
                petunia_core::SelectMode::Face,
                false,
            )
        });
        let hidden = occluder.is_some_and(|occluder| {
            let depth = (occluder.position - origin).dot(direction);
            depth < hit.depth - 1.0e-3 * hit.depth.abs().max(1.0)
        });
        (!hidden).then_some(hit)
    }

    /// Push/Pull sobre uma região: prelúdio (imprint ou forma nova) + Extrude
    /// no mesmo gesto, 1 Undo, ajustável na "Última operação".
    fn begin_region_gesture(&mut self, anchor: [f32; 2], hit: &petunia_core::RegionHit) -> bool {
        self.modal_text.clear();
        if self.state.primitive_session_valid() {
            self.state.finalize_primitive_session();
        }
        if let Err(error) = self.state.begin_region_push_pull(hit) {
            self.state.set_status(error.to_string());
            return false;
        }
        self.region_hover = None;
        self.region_planes_cache = None;
        self.tool_modal = Some(ToolModalKind::Extrude);
        self.keyboard_tool_modal_active = false;
        self.tool_modal_value = 0.0;
        self.sync_viewport_context();
        let Some(frame) = self.drag_frame() else {
            self.cancel_tool_modal();
            return false;
        };
        self.tool_gesture = Some(ToolGesture::Parametric {
            kind: ToolModalKind::Extrude,
            frame,
            anchor,
            start_value: 0.0,
        });
        self.state.mark_dirty();
        true
    }

    /// Contorno (e furos) da região em hover, em px da viewport.
    fn region_hover_commands(&self) -> String {
        use std::fmt::Write as _;
        let Some(hit) = &self.region_hover else {
            return String::new();
        };
        let [width, height] = self.viewport_size;
        let matrix = self.state.session.camera.view_proj();
        let mut commands = String::new();
        for ring in std::iter::once(&hit.region.outer).chain(hit.region.holes.iter()) {
            let mut first = true;
            for point in ring {
                let clip = matrix * hit.plane.to_world(*point).extend(1.0);
                if !clip.is_finite() || clip.w <= 0.05 {
                    return String::new();
                }
                let x = (clip.x / clip.w * 0.5 + 0.5) * width;
                let y = (0.5 - clip.y / clip.w * 0.5) * height;
                let action = if std::mem::take(&mut first) { 'M' } else { 'L' };
                let _ = write!(commands, "{action} {x:.2} {y:.2} ");
            }
            commands.push_str("Z ");
        }
        commands
    }

    /// Arrastar sobre um elemento não selecionado o seleciona antes de operar
    /// (como no Cinema 4D e no Modo). Clique sem arrasto continua sendo só
    /// seleção.
    fn select_under_anchor_for(&mut self, kind: ToolModalKind, anchor: [f32; 2]) {
        let domain = match kind {
            ToolModalKind::Bevel => SelectionDomain::Edge,
            ToolModalKind::ScaleSelection => return,
            _ => SelectionDomain::Face,
        };
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 {
            return;
        }
        if self.state.selection_domain() == SelectionDomain::Object {
            self.state.set_selection_domain(domain);
            self.sync_viewport_context();
        }
        let (nx, ny) = (anchor[0] / width, anchor[1] / height);
        let target = self.pick_target_for_domain(self.state.selection_domain(), nx, ny);
        let Some(mesh) = self.state.project.active_mesh() else {
            return;
        };
        let already_selected = match target {
            petunia_core::HoverTarget::Face(index) => {
                mesh.faces.get(index).is_some_and(|face| face.selected)
            }
            petunia_core::HoverTarget::Edge(a, b) => {
                mesh.selected_edges.contains(&(a, b)) || mesh.selected_edges.contains(&(b, a))
            }
            petunia_core::HoverTarget::Vertex(index) => mesh
                .verts
                .get(index as usize)
                .is_some_and(|vertex| vertex.selected),
            // Nada sob o cursor: opera sobre a seleção atual.
            petunia_core::HoverTarget::None | petunia_core::HoverTarget::Object(_) => true,
        };
        if !already_selected {
            self.select_viewport_ext(nx, ny, self.tool_press_extend, false);
        }
    }

    /// Geometria de tela da operação ativa, para o valor seguir o cursor.
    fn drag_frame(&self) -> Option<petunia_core::DragFrame> {
        use petunia_core::transform_projection::project_pixel;
        let modal = self.state.session.tools.modal.as_ref()?;
        let camera = &self.state.session.camera;
        let viewport = glam::Vec2::from_array(self.viewport_size);
        let pivot = project_pixel(camera, viewport, modal.pivot)?;
        let world_per_pixel = project_pixel(camera, viewport, modal.pivot + camera.right())
            .map(|right| (right - pivot).length())
            .filter(|pixels| *pixels > 1.0e-3)
            .map_or(camera.visible_height() / viewport.y.max(1.0), |pixels| {
                1.0 / pixels
            });
        let normal_px_per_unit = project_pixel(camera, viewport, modal.pivot + modal.normal)
            .map(|tip| (tip - pivot).to_array());
        Some(petunia_core::DragFrame {
            pivot_px: pivot.to_array(),
            normal_px_per_unit,
            world_per_pixel,
        })
    }

    fn update_tool_gesture(&mut self, current: [f32; 2], fine: bool, snap: bool) -> bool {
        match self.tool_gesture {
            Some(ToolGesture::Transform { .. }) => {
                self.update_viewport_transform_modified(current[0], current[1], fine, snap)
            }
            Some(ToolGesture::Parametric {
                kind,
                frame,
                anchor,
                start_value,
            }) => {
                if !self.modal_text.is_empty() {
                    // O valor digitado vence o mouse (constituição 11).
                    return false;
                }
                let mut value = petunia_core::drag_value(
                    kind.modal_kind(),
                    frame,
                    anchor,
                    current,
                    start_value,
                );
                if fine {
                    value = start_value + (value - start_value) * 0.1;
                }
                self.set_tool_modal_value(value)
            }
            None => false,
        }
    }

    fn commit_tool_gesture(&mut self) -> bool {
        match self.tool_gesture.take() {
            Some(ToolGesture::Transform { gizmo: true }) => self.end_gizmo_drag(),
            Some(ToolGesture::Transform { gizmo: false }) => self.end_viewport_transform(),
            Some(ToolGesture::Parametric { .. }) => self.commit_tool_modal(),
            None => false,
        }
    }

    fn cancel_tool_gesture(&mut self) -> bool {
        match self.tool_gesture.take() {
            Some(ToolGesture::Transform { gizmo }) => {
                if gizmo {
                    self.gizmo_drag = None;
                }
                self.cancel_viewport_transform()
            }
            Some(ToolGesture::Parametric { .. }) => self.cancel_tool_modal(),
            None => false,
        }
    }

    /// `Esc` sem gesto: a ferramenta volta para Select (escada do Esc).
    fn exit_grammar_tool(&mut self) -> bool {
        if self.grammar_tool().is_none() {
            return false;
        }
        self.tool_session.reset();
        self.tool_gesture = None;
        self.state.session.tools.active_tool = "select".to_string();
        self.state.mark_dirty();
        true
    }

    /// Ajusta a última operação pelo texto do card (mesma entrada de Undo).
    pub fn commit_last_operation_text(&mut self, text: &str) -> bool {
        let Some(last) = self.last_operation.as_ref() else {
            return false;
        };
        match numeric::parse_numeric_with_base(text, last.primary_value()) {
            Ok(value) => self.adjust_last_operation(value),
            Err(_) => false,
        }
    }

    /// Reaplica a última operação com `value`, substituindo o gesto no
    /// histórico sem aumentar a profundidade do Undo.
    pub fn adjust_last_operation(&mut self, value: f32) -> bool {
        let Some(last) = self.last_operation.clone() else {
            return false;
        };
        if !self.state.last_operation_is_current(&last) {
            self.last_operation = None;
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::TOOL_GRAMMAR_EXPIRED),
            );
            return false;
        }
        match self.state.adjust_last_operation(&last, value) {
            Ok(next) => {
                self.last_operation = next;
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::TOOL_GRAMMAR_ADJUSTED),
                );
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// A "Última operação" ainda pode ser ajustada.
    pub fn last_operation_adjustable(&self) -> bool {
        self.tool_modal.is_none()
            && self
                .last_operation
                .as_ref()
                .is_some_and(|last| self.state.last_operation_is_current(last))
    }

    /// Roda com Ctrl: contagens e raios da ferramenta ativa. A roda sem
    /// modificador sempre faz zoom (constituição 11).
    pub fn viewport_ctrl_scroll(&mut self, delta: f32) -> bool {
        if !delta.is_finite() {
            return false;
        }
        if self.state.session.tools.active_tool == "loop_cut" || self.loop_cut.is_some() {
            self.scroll_loop_cut_count(delta);
        } else if self.state.session.tools.modal.is_some()
            && self.state.session.proportional_editing
        {
            let step = if delta > 0.0 { 0.25 } else { -0.25 };
            self.adjust_proportional_radius(step);
            if let Some(drag) = self.drag {
                self.update_viewport_transform_modified(
                    drag.last_pointer[0],
                    drag.last_pointer[1],
                    false,
                    false,
                );
            }
        } else {
            self.state.session.camera.zoom(delta);
        }
        self.state.mark_dirty();
        true
    }

    /// Abre a sessão modal de uma ferramenta paramétrica com preview próprio.
    pub fn begin_tool_modal(&mut self, kind: ToolModalKind) -> bool {
        // Um valor digitado numa operação anterior nunca vaza para a próxima.
        self.modal_text.clear();
        if self.state.primitive_session_valid() {
            self.state.finalize_primitive_session();
        }
        // Troca de domínio visível; nunca seleciona tudo por conta própria
        // (constituição 11). Sem seleção aplicável, o core recusa com erro.
        if self.state.selection_domain() == SelectionDomain::Object {
            match kind {
                ToolModalKind::Extrude
                | ToolModalKind::ExtrudeIndividual
                | ToolModalKind::Inset
                | ToolModalKind::PushPull => {
                    self.state.set_selection_domain(SelectionDomain::Face);
                    self.sync_viewport_context();
                }
                ToolModalKind::Bevel => {
                    self.state.set_selection_domain(SelectionDomain::Edge);
                    self.sync_viewport_context();
                }
                ToolModalKind::ScaleSelection => {}
            }
        }
        match self.state.begin_modal(kind.modal_kind()) {
            Ok(()) => {
                self.tool_modal = Some(kind);
                self.state.session.tools.active_tool = match kind {
                    ToolModalKind::ScaleSelection => "scale",
                    ToolModalKind::Extrude | ToolModalKind::ExtrudeIndividual => "extrude",
                    ToolModalKind::Inset => "inset",
                    ToolModalKind::Bevel => "bevel",
                    ToolModalKind::PushPull => "push_pull",
                }
                .to_string();
                self.keyboard_tool_modal_active = false;
                let initial = match kind {
                    ToolModalKind::Inset => 0.2,
                    ToolModalKind::Bevel => 0.05,
                    ToolModalKind::ScaleSelection => 1.0,
                    _ => 0.0,
                };
                self.tool_modal_value = initial;
                if initial != 0.0 {
                    let _ = self.state.update_modal(glam::Vec3::ZERO, initial);
                }
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Ajusta o preview pelo arrasto vertical na viewport.
    pub fn scrub_tool_modal(&mut self, delta_y: f32, fine: bool) -> bool {
        let Some(kind) = self.tool_modal else {
            return false;
        };
        if !self.modal_text.is_empty() {
            // Enquanto houver texto digitado, ele controla o valor (constituição 11).
            return false;
        }
        let step = kind.step();
        let delta_y = delta_y
            * if fine { 0.1 } else { 1.0 }
            * if self.state.ui.invert_vertical_drag {
                -1.0
            } else {
                1.0
            };
        let world_per_pixel =
            self.state.session.camera.visible_height() / self.viewport_size[1].max(1.0);
        let delta = match kind {
            ToolModalKind::Inset => -delta_y * step * 0.5,
            ToolModalKind::Bevel => -delta_y * world_per_pixel * 0.5,
            ToolModalKind::ScaleSelection => -delta_y * step * 0.5,
            _ => -delta_y * world_per_pixel * 0.5,
        };
        self.set_tool_modal_value(self.tool_modal_value + delta)
    }

    /// Define o valor absoluto do preview (arrasto e campo numérico).
    pub fn set_tool_modal_value(&mut self, value: f32) -> bool {
        let Some(kind) = self.tool_modal else {
            return false;
        };
        if !value.is_finite() {
            return false;
        }
        let (minimum, maximum) = kind.bounds();
        let value = value.clamp(minimum, maximum);
        if self.state.update_modal(glam::Vec3::ZERO, value).is_err() {
            // Topologia recusada (ex.: bevel inválido): mantém o último preview.
            return false;
        }
        self.tool_modal_value = value;
        self.state.mark_dirty();
        true
    }

    /// Confirma a ferramenta paramétrica como uma única operação de undo.
    pub fn commit_tool_modal(&mut self) -> bool {
        if self.tool_modal.take().is_none() {
            return false;
        }
        self.modal_text.clear();
        self.keyboard_tool_modal_active = false;
        self.last_operation = self.state.commit_modal_gesture();
        self.state.mark_dirty();
        true
    }

    pub fn cancel_tool_modal(&mut self) -> bool {
        if self.tool_modal.take().is_none() {
            return false;
        }
        self.modal_text.clear();
        self.keyboard_tool_modal_active = false;
        self.state.cancel_modal();
        self.state.mark_dirty();
        true
    }

    fn viewport_brush_settings(&self) -> petunia_core::BrushSettings {
        let mut settings = self.state.brush_settings();
        settings.kind = match self.state.session.tools.active_tool.as_str() {
            "eraser" => petunia_core::BrushType::Eraser,
            "airbrush" => petunia_core::BrushType::Airbrush,
            "pixel" => petunia_core::BrushType::Pixel,
            _ => settings.kind,
        };
        settings.size_px = (self.state.session.tools.paint_radius * 16.0).max(2.0);
        settings.sanitized()
    }

    fn paint_dabs_at(&mut self, points: &[[f32; 2]], settings: petunia_core::BrushSettings) {
        if points.is_empty() {
            return;
        }
        let width = self.viewport_size[0].max(1.0);
        let height = self.viewport_size[1].max(1.0);
        let mut hits = Vec::with_capacity(points.len());
        for &[x, y] in points {
            if !x.is_finite()
                || !y.is_finite()
                || !(0.0..width).contains(&x)
                || !(0.0..height).contains(&y)
            {
                continue;
            }
            let ndc_x = x / width * 2.0 - 1.0;
            let ndc_y = 1.0 - y / height * 2.0;
            let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);
            if let Some(hit) = pick_face_hit(&self.state, origin, direction) {
                hits.push(hit);
            }
        }
        let Some(&(face, hit)) = hits.first() else {
            return;
        };
        if self.state.session.tools.active_tool == "picker" {
            return;
        }
        if self.state.session.tools.active_tool == "fill" {
            let scope = self.state.session.tools.fill_scope;
            let isolate = self.state.session.tools.paint_isolate_selection;
            let seed =
                petunia_module_paint::PaintModule::face_hit_uv(&self.state, face, hit, isolate)
                    .and_then(|uv| petunia_module_paint::PaintModule::uv_to_px(&self.state, uv));
            petunia_module_paint::PaintModule::canvas_fill_scoped(
                &mut self.state,
                Some(face),
                seed,
                scope,
            );
            self.state.set_status(format!("Filled ({scope:?})"));
            return;
        }
        if self.paint_target_vertex {
            let radius = (self.state.session.tools.paint_radius * 0.5).max(0.01);
            let strength = self.state.session.tools.paint_strength;
            let color = self.state.paint_color;
            let hit_positions: Vec<_> = hits.iter().map(|(_, hit)| *hit).collect();
            petunia_module_paint::PaintModule::paint_vertex_color_3d_batch(
                &mut self.state,
                &hit_positions,
                radius,
                strength,
                color,
            );
        } else {
            let isolate = self.state.session.tools.paint_isolate_selection;
            petunia_module_paint::PaintModule::paint_mesh_3d_batch_with_settings(
                &mut self.state,
                &hits,
                settings,
                isolate,
            );
        }
    }

    pub fn select_viewport(&mut self, normalized_x: f32, normalized_y: f32, extend: bool) {
        self.select_viewport_ext(normalized_x, normalized_y, extend, false);
    }

    pub fn select_viewport_ext(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        extend: bool,
        loop_select: bool,
    ) {
        if !normalized_x.is_finite() || !normalized_y.is_finite() {
            return;
        }
        let now = std::time::Instant::now();
        let max_interval = (self.preferences.double_tap_interval_ms as u128).clamp(300, 800);
        let is_double_click = if let Some((last_pos, last_time)) = self.last_viewport_click {
            let dt = now.duration_since(last_time).as_millis();
            let dx = normalized_x - last_pos[0];
            let dy = normalized_y - last_pos[1];
            (30..=max_interval).contains(&dt) && (dx * dx + dy * dy) < 0.01
        } else {
            false
        };
        self.last_viewport_click = Some(([normalized_x, normalized_y], now));
        let loop_select = loop_select || is_double_click;

        if self.state.session.tools.active_tool == "draw_profile" {
            let ndc_x = normalized_x.clamp(0.0, 1.0) * 2.0 - 1.0;
            let ndc_y = 1.0 - normalized_y.clamp(0.0, 1.0) * 2.0;
            self.add_profile_point(ndc_x, ndc_y);
            return;
        }
        if self.instant_transform && self.state.session.tools.modal.is_some() {
            self.end_viewport_transform();
            return;
        }
        if self.state.session.tools.active_tool == "loop_cut" && self.loop_cut.is_none() {
            let _ = self.place_loop_cut_hover();
            return;
        }
        // No modo Instant um clique confirma a sessão paramétrica em vez de
        // trocar a seleção — é o equivalente ao Enter com o mouse.
        if self.is_instant_tool_mode() && self.tool_modal.is_some() {
            self.commit_tool_modal();
            return;
        }
        // A faca consome o clique antes da seleção: com uma sessão de corte
        // aberta, clicar é escolher ponto de aresta, não selecionar.
        if (self.state.session.tools.active_tool == "cut"
            || self.state.session.tools.active_tool == "knife")
            && self.state.session.tools.cut_session.is_some()
        {
            self.knife_click(normalized_x, normalized_y);
            return;
        }
        if self.state.session.tools.active_tool == "slice" {
            return;
        }
        if self.state.primitive_session_valid() {
            self.state.finalize_primitive_session();
        }
        let ndc_x = normalized_x.clamp(0.0, 1.0) * 2.0 - 1.0;
        let ndc_y = 1.0 - normalized_y.clamp(0.0, 1.0) * 2.0;
        let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);

        if self.state.workspace == Workspace::Paint {
            if let Some((face, hit)) = pick_face_hit(&self.state, origin, direction) {
                let tool = self.state.session.tools.active_tool.as_str();
                let brush = match tool {
                    "eraser" => petunia_core::BrushType::Eraser,
                    "fill" => petunia_core::BrushType::Fill,
                    "picker" => petunia_core::BrushType::Eyedropper,
                    "airbrush" => petunia_core::BrushType::Airbrush,
                    "pixel" => petunia_core::BrushType::Pixel,
                    _ => petunia_core::BrushType::Soft,
                };
                if brush == petunia_core::BrushType::Eyedropper {
                    self.pick_paint_color_at(
                        normalized_x * self.viewport_size[0],
                        normalized_y * self.viewport_size[1],
                    );
                } else if self.paint_target_vertex {
                    let radius = (self.state.session.tools.paint_radius * 0.5).max(0.01);
                    let strength = self.state.session.tools.paint_strength;
                    let color = self.state.paint_color;
                    petunia_module_paint::PaintModule::paint_vertex_color_3d(
                        &mut self.state,
                        hit,
                        radius,
                        strength,
                        color,
                    );
                } else {
                    let radius = (self.state.session.tools.paint_radius * 8.0).max(1.0) as u32;
                    let strength = self.state.session.tools.paint_strength;
                    let isolate = self.state.session.tools.paint_isolate_selection;
                    petunia_module_paint::PaintModule::paint_mesh_3d(
                        &mut self.state,
                        face,
                        hit,
                        brush,
                        radius,
                        strength,
                        isolate,
                    );
                }
            }
            self.state.mark_dirty();
            return;
        }

        use petunia_core::HoverTarget as Target;
        // O clique confirma a preselection: o alvo sob o cursor vira o hover
        // corrente para o destaque e o rótulo aparecerem de imediato, sem
        // esperar o próximo mousemove. Erro limpa em vez de congelar.
        let hit = self.pick_viewport_target(normalized_x, normalized_y);
        match hit {
            Target::Object(index) => {
                self.state.select_object(Some(index), extend);
                let name = self.state.project.assets[index].name.clone();
                self.state.set_status(format!("Selected '{name}'"));
            }
            Target::Vertex(index) => {
                if let Some(mesh) = self.state.project.active_mesh_mut() {
                    if loop_select {
                        let count = mesh.select_vertex_loop(index, extend);
                        self.state
                            .set_status(format!("Selected vertex loop ({count} points)"));
                    } else {
                        if !extend {
                            mesh.deselect_all();
                        }
                        if let Some(vertex) = mesh.verts.get_mut(index as usize) {
                            vertex.selected = !extend || !vertex.selected;
                        }
                        self.state.set_status(format!("Point {index} selected"));
                    }
                }
            }
            Target::Edge(a, b) => {
                if let Some(mesh) = self.state.project.active_mesh_mut() {
                    if loop_select {
                        let count = mesh.select_edge_loop((a, b), extend);
                        self.state
                            .set_status(format!("Selected edge loop ({count} edges)"));
                    } else {
                        if !extend {
                            mesh.deselect_all();
                        }
                        if extend && mesh.selected_edges.contains(&(a, b)) {
                            mesh.selected_edges.remove(&(a, b));
                        } else {
                            mesh.selected_edges.insert((a, b));
                        }
                        // Operações de malha usam os vértices das arestas selecionadas.
                        // Recalcular impede que um Shift-click para desmarcar deixe
                        // vértices invisivelmente selecionados.
                        for vertex in &mut mesh.verts {
                            vertex.selected = false;
                        }
                        for &(start, end) in &mesh.selected_edges {
                            if let Some(vertex) = mesh.verts.get_mut(start as usize) {
                                vertex.selected = true;
                            }
                            if let Some(vertex) = mesh.verts.get_mut(end as usize) {
                                vertex.selected = true;
                            }
                        }
                        self.state.set_status(format!("Edge {a}-{b} selected"));
                    }
                }
            }
            Target::Face(face) => {
                let nearest_edge = if loop_select {
                    match self.pick_target_for_domain(
                        SelectionDomain::Edge,
                        normalized_x,
                        normalized_y,
                    ) {
                        petunia_core::HoverTarget::Edge(ea, eb) => Some((ea, eb)),
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(mesh) = self.state.project.active_mesh_mut() {
                    if loop_select {
                        let count = mesh.select_face_loop(face, nearest_edge, extend);
                        self.state
                            .set_status(format!("Selected face loop ({count} faces)"));
                    } else {
                        if !extend {
                            mesh.deselect_all();
                        }
                        if let Some(current) = mesh.faces.get_mut(face) {
                            current.selected = !extend || !current.selected;
                        }
                        mesh.sync_vert_selection_from_faces();
                        self.state.set_status(format!("Face {face} selected"));
                    }
                }
            }
            Target::None => {
                if self.state.selection_domain() == SelectionDomain::Object {
                    self.state.select_object(None, extend);
                }
                if !extend
                    && self.state.selection_domain().is_component()
                    && let Some(mesh) = self.state.project.active_mesh_mut()
                {
                    mesh.deselect_all();
                }
                self.state.set_status("Nothing under the cursor");
            }
        }
        self.state.session.tools.hover = hit;
        self.state.sync_selection();
        self.state.mark_dirty();
        self.reset_transform_fields();
    }

    fn cancel_active_operation(&mut self) -> bool {
        let mut cancelled = self.cancel_paint_stroke();
        cancelled |= self.cancel_tool_modal();
        cancelled |= self.cancel_loop_cut();
        if self.profile_volume_mode.is_some() {
            cancelled |= self.cancel_profile_volume();
        }
        if self.state.session.tools.active_tool == "loop_cut" {
            self.state.session.tools.active_tool = "select".to_string();
            self.loop_cut_hover_ring = None;
            self.loop_cut_hover_source = None;
            self.state.session.tools.hover = petunia_core::HoverTarget::None;
            cancelled = true;
        }
        cancelled |= self.cancel_slice();
        cancelled |= self.cancel_knife();
        cancelled |= self.cancel_transform();
        if self.micro_inspector_open {
            self.micro_inspector_open = false;
            self.overlays.remove(OverlayId::MicroInspector);
            cancelled = true;
        }
        self.drag = None;
        self.gizmo_drag = None;
        self.modal_text.clear();
        self.instant_transform = false;
        cancelled
    }

    pub fn handle_escape(&mut self) -> bool {
        if self.close_context_menu() {
            return true;
        }
        if self.close_menu() {
            return true;
        }
        if self.micro_inspector_open {
            self.micro_inspector_open = false;
            self.overlays.remove(OverlayId::MicroInspector);
            self.state.mark_dirty();
            return true;
        }
        if self.cancel_rename() {
            return true;
        }
        if self.tool_session.is_gesture_active() {
            let effect = self.tool_session.key(petunia_core::ToolKey::Cancel);
            self.apply_tool_effect(effect, false, false);
            return true;
        }
        // Escada do Esc: pontos coletados antes de sair da ferramenta.
        if !self.poly_pen_points.is_empty() {
            self.poly_pen_points.clear();
            self.state.mark_dirty();
            return true;
        }
        if self.cancel_paint_stroke() {
            return true;
        }
        if self.cancel_tool_modal() {
            return true;
        }
        if self.drag.take().is_some() {
            self.cancel_transform();
            return true;
        }
        if self.cancel_loop_cut() {
            return true;
        }
        if self.state.session.tools.active_tool == "loop_cut" {
            self.state.session.tools.active_tool = "select".to_string();
            self.loop_cut_hover_ring = None;
            self.loop_cut_hover_source = None;
            self.state.session.tools.hover = petunia_core::HoverTarget::None;
            self.state.mark_dirty();
            self.state.set_status("Loop Cut cancelled");
            return true;
        }
        if self.profile_volume_mode.is_some() {
            self.cancel_profile_volume();
            return true;
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            self.profile_pointer_up();
            self.state.profile.clear();
            self.active_profile_id = None;
            self.profile_selected_point = None;
            self.profile_drag_target = None;
            self.profile_edit_gesture = None;
            self.state.session.tools.active_tool = "select".to_string();
            self.state.mark_dirty();
            self.state.set_status("Profile editing finished");
            return true;
        }
        if self.state.primitive_session_valid() {
            // A primitiva criada é um gesto confirmado: Esc fecha o card e a
            // mantém; Undo a remove (constituição 11).
            self.state.finalize_primitive_session();
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::TOOL_GRAMMAR_PRIMITIVE_KEPT),
            );
            self.state.mark_dirty();
            return true;
        }
        if self.cancel_slice() {
            return true;
        }
        if self.cancel_knife() {
            return true;
        }
        if self.cancel_transform() {
            return true;
        }
        if self.state.session.edit_pivot {
            self.state.session.edit_pivot = false;
            self.state.mark_dirty();
            self.state.set_status("Edit Pivot exited");
            return true;
        }
        if let Some(entry) = self.overlays.esc() {
            self.hide_overlay(entry.id);
            return true;
        }
        self.exit_grammar_tool()
    }

    pub fn handle_click_away(&mut self) -> bool {
        let mut handled = false;
        if self.close_context_menu() {
            handled = true;
        }
        if self.close_menu() {
            handled = true;
        }
        if self.add_menu_open {
            self.add_menu_open = false;
            handled = true;
        }
        if self.pivot_menu_open {
            self.pivot_menu_open = false;
            self.overlays.remove(OverlayId::PivotMenu);
            handled = true;
        }
        if let Some(entry) = self.overlays.click_away() {
            self.hide_overlay(entry.id);
            handled = true;
        }
        handled
    }

    pub fn execute_command(&mut self, id: CommandId) {
        self.command_search_visible = false;
        self.overlays.remove(OverlayId::CommandPalette);
        match id {
            CommandId::SaveProject => self.apply(UiIntent::SaveProject),
            CommandId::OpenProject => self.state.set_status("open requested by Slint frontend"),
            CommandId::Undo => self.apply(UiIntent::Undo),
            CommandId::Redo => self.apply(UiIntent::Redo),
            CommandId::AddCube => {
                self.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Cube))
            }
            CommandId::AddSphere => {
                self.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Sphere))
            }
            CommandId::AddCylinder => self.apply(UiIntent::AddPrimitive(
                petunia_core::PrimitiveKind::Cylinder,
            )),
            CommandId::AddPlane => {
                self.apply(UiIntent::AddPrimitive(petunia_core::PrimitiveKind::Plane))
            }
            CommandId::DeleteSelected => self.apply(UiIntent::DeleteActiveAsset),
            CommandId::SelectModeObject => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Object))
            }
            CommandId::SelectModePoint => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex))
            }
            CommandId::SelectModeEdge => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge))
            }
            CommandId::SelectModeFace => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face))
            }
            CommandId::SelectLasso => self.apply(UiIntent::SetActiveTool("lasso_select".into())),
            CommandId::ToolCursor => self.apply(UiIntent::SetActiveTool("cursor".into())),
            CommandId::FrameCursor => {
                self.frame_cursor();
            }
            CommandId::TransformCombined => self.apply(UiIntent::SetActiveTool("transform".into())),
            CommandId::PaintBrush => self.apply(UiIntent::SetActiveTool("brush".to_string())),
            CommandId::PaintEraser => self.apply(UiIntent::SetActiveTool("eraser".to_string())),
            CommandId::PaintFill => self.apply(UiIntent::SetActiveTool("fill".to_string())),
            CommandId::UvUnwrap => {
                if let Err(error) = self.state.dispatch_command("uv.unwrap_auto") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::UvPackIslands => {
                if let Err(error) = self.state.dispatch_command("uv.pack_islands") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::FrameSelection => {
                if let Err(error) = self.state.dispatch_command("view.frame_selection") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::ToggleWireframe => {
                if let Err(error) = self.state.dispatch_command("view.toggle_wireframe") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::ToggleWireOverlay => {
                if let Err(error) = self.state.dispatch_command("view.toggle_wire_overlay") {
                    self.state.set_status(error.to_string());
                }
            }
            CommandId::OpenSettings => self.apply(UiIntent::OpenSettings),
            CommandId::ToggleSceneDrawer => self.apply(UiIntent::ToggleSceneDrawer),
            CommandId::DuplicateSelected => self.apply(UiIntent::DuplicateActiveAsset),
            CommandId::SelectAll => self.apply(UiIntent::SelectAll),
            CommandId::ClearSelection => self.apply(UiIntent::ClearSelection),
            CommandId::InvertSelection => self.apply(UiIntent::InvertSelection),
            CommandId::ToggleAssetLibrary => self.apply(UiIntent::ToggleAssetLibrary),
            CommandId::ResetCamera => self.apply(UiIntent::ResetCamera),
            CommandId::ToggleProjection => self.apply(UiIntent::ToggleProjection),
            CommandId::SaveActiveAsAsset => self.apply(UiIntent::SaveActiveAsAsset),
            CommandId::ToggleFaceOrientation => self.apply(UiIntent::ToggleFaceOrientation),
            CommandId::ToggleUvChecker => self.apply(UiIntent::ToggleUvChecker),
            CommandId::ToggleProportionalEditing => self.apply(UiIntent::ToggleProportionalEditing),
            CommandId::ToggleSnap => self.apply(UiIntent::ToggleSnapEnabled),
            CommandId::OriginToGeometry => self.apply(UiIntent::SetOriginGeometry),
            CommandId::OriginToBottom => self.apply(UiIntent::SetOriginBottom),
            CommandId::OriginToCursor => self.apply(UiIntent::SetOriginCursor),
            CommandId::OriginToSelection => self.apply(UiIntent::SetOriginSelection),
            CommandId::GeometryToOrigin => self.apply(UiIntent::SetGeometryToOrigin),
            CommandId::ToggleEditPivot => self.apply(UiIntent::ToggleEditPivot),
        }
    }

    pub fn search_commands(&self, query: &str) -> Vec<petunia_core::CommandPaletteItem> {
        self.state.commands.query(query, &self.state)
    }

    pub fn execute_core_command(&mut self, id: &str) -> Result<(), petunia_core::CommandError> {
        self.command_search_visible = false;
        self.overlays.remove(OverlayId::CommandPalette);
        if self.state.primitive_session_valid()
            && id != "primitive.confirm"
            && id != "primitive.cancel"
        {
            self.state.finalize_primitive_session();
        }
        if id == "model.delete" {
            self.delete_or_dissolve_selection();
            return Ok(());
        }
        // Ações do card da faca: aplicar todos os segmentos como um Undo ou
        // restaurar a malha original.
        if id == "model.knife_apply" {
            self.commit_knife();
            return Ok(());
        }
        if id == "model.knife_cancel" {
            self.cancel_knife();
            return Ok(());
        }
        if id == "model.duplicate" {
            self.duplicate_selection();
            return Ok(());
        }
        if id == "edit.copy" {
            self.copy_selection();
            return Ok(());
        }
        if id == "edit.paste" {
            self.paste_clipboard();
            return Ok(());
        }
        if id == "model.extrude"
            && self.state.session.tools.active_tool == "draw_profile"
            && (self.active_profile_closed() || self.active_profile_point_count() >= 3)
        {
            if self.profile_volume_mode == Some(petunia_module_model::ProfileVolumeMode::Extrude) {
                self.commit_profile_volume();
            } else {
                self.enter_profile_volume("extrude");
            }
            return Ok(());
        }
        // Ferramentas paramétricas abrem uma sessão modal com Tool Properties
        // próprias em vez de rodar como one-shot de valor fixo.
        if let Some(kind) = ToolModalKind::from_id(id) {
            self.begin_tool_modal(kind);
            return Ok(());
        }
        if id == "model.loop_cut" {
            self.apply(UiIntent::SetActiveTool("loop_cut".to_string()));
            return Ok(());
        }
        match id {
            "uv.unwrap" => self.state.dispatch_command("uv.unwrap_auto"),
            "uv.pack_islands" => self.state.dispatch_command("uv.pack_islands"),
            "uv.stitch" => self.state.dispatch_command("uv.stitch"),
            "uv.relax" => self.state.dispatch_command("uv.relax"),
            "model.connect" => self.state.dispatch_command("model.connect"),
            "model.spin" => self.state.dispatch_command("model.spin"),
            "model.dissolve" => self.state.dispatch_command("model.dissolve"),
            "uv.equalize_texel_density" => {
                self.uv_equalize_texel_density();
                Ok(())
            }
            "model.frame_selection" => self.state.dispatch_command("view.frame_selection"),
            other => self.state.dispatch_command(other),
        }
    }

    fn execute_shortcut_tool(&mut self, id: &str) {
        // Troca de domínio visível; nunca seleciona tudo (constituição 11).
        if self.state.selection_domain() == SelectionDomain::Object {
            match id {
                "model.extrude"
                | "model.extrude_individual"
                | "model.inset"
                | "model.push_pull" => {
                    self.state.set_selection_domain(SelectionDomain::Face);
                    self.sync_viewport_context();
                }
                "model.bevel" => {
                    self.state.set_selection_domain(SelectionDomain::Edge);
                    self.sync_viewport_context();
                }
                "model.loop_cut" => {
                    self.state.set_selection_domain(SelectionDomain::Edge);
                    self.sync_viewport_context();
                }
                _ => {}
            }
        }

        let now = std::time::Instant::now();
        let interval_ms = self.preferences.double_tap_interval_ms;

        let is_double_tap = if interval_ms > 0 {
            if let Some((ref last_tool, last_time)) = self.last_tool_press {
                last_tool == id && now.duration_since(last_time).as_millis() <= interval_ms as u128
            } else {
                false
            }
        } else {
            self.tool_modal.as_ref().is_some_and(|m| m.id() == id)
        };

        let kind = ToolModalKind::from_id(id);
        let is_instant_pref =
            self.state.session.tools.tool_activation == petunia_core::ToolActivation::Instant;

        if is_double_tap || is_instant_pref {
            self.last_tool_press = None;
            if self.tool_modal.is_none() {
                let _ = self.execute_core_command(id);
            }
            if self.tool_modal.is_some() {
                self.keyboard_tool_modal_active = true;
                let label = kind.map_or("Ferramenta", |k| k.title());
                self.state.set_status(format!(
                    "{} (Modo Livre) · Mova o mouse, LMB/Enter para confirmar, RMB/Esc para cancelar",
                    label
                ));
            }
        } else if let Some(kind) = kind.filter(|kind| *kind != ToolModalKind::ScaleSelection) {
            // Gramática única: a tecla seleciona a ferramenta persistente; a
            // operação acontece pelo arrasto (ADR 007). O duplo toque continua
            // abrindo o modo que segue o ponteiro.
            self.last_tool_press = Some((id.to_string(), now));
            self.keyboard_tool_modal_active = false;
            self.activate_parametric_tool(kind);
        } else {
            self.last_tool_press = Some((id.to_string(), now));
            if self.tool_modal.is_none() {
                let _ = self.execute_core_command(id);
            }
            self.keyboard_tool_modal_active = false;
        }
    }

    /// Distância, em px lógicos, que separa clique de arrasto.
    pub fn set_drag_threshold_px(&mut self, pixels: f32) -> bool {
        if !pixels.is_finite() {
            return false;
        }
        let (minimum, maximum) = petunia_core::DRAG_THRESHOLD_RANGE;
        let pixels = pixels.clamp(minimum, maximum);
        if (self.preferences.drag_threshold_px - pixels).abs() < f32::EPSILON {
            return false;
        }
        self.preferences.drag_threshold_px = pixels;
        self.tool_session.set_drag_threshold_px(pixels);
        self.state.mark_dirty();
        true
    }

    /// Raio do snap em px lógicos (acessibilidade motora).
    pub fn set_snap_radius_px(&mut self, pixels: f32) -> bool {
        if !pixels.is_finite() {
            return false;
        }
        let pixels = petunia_core::clamp_snap_radius(pixels);
        if (self.preferences.snap_radius_px - pixels).abs() < f32::EPSILON {
            return false;
        }
        self.preferences.snap_radius_px = pixels;
        self.state.session.snap_settings.radius_pixels = pixels;
        self.state.mark_dirty();
        true
    }

    /// Clicar numa alça a faz seguir o ponteiro até o próximo clique.
    pub fn set_click_move_click(&mut self, enabled: bool) -> bool {
        if self.preferences.click_move_click == enabled {
            return false;
        }
        self.preferences.click_move_click = enabled;
        self.tool_session.set_click_move_click(enabled);
        self.state.mark_dirty();
        true
    }

    /// Luz de estúdio presa à câmera (padrão) ou fixa no mundo.
    pub fn set_studio_light_follows_camera(&mut self, follows: bool) -> bool {
        if self.preferences.studio_light_follows_camera == follows {
            return false;
        }
        self.preferences.studio_light_follows_camera = follows;
        self.state.render.mark_dirty();
        self.state.mark_dirty();
        true
    }

    /// Plano automático do Draw: favorecer o chão (SketchUp) ou a vista (Modo/C4D).
    pub fn set_workplane_prefer_ground(&mut self, enabled: bool) -> bool {
        if self.preferences.workplane_prefer_ground == enabled {
            return false;
        }
        self.preferences.workplane_prefer_ground = enabled;
        self.state.profile.workplane_prefer_ground = enabled;
        self.state.mark_dirty();
        true
    }

    pub fn route_shortcut(&mut self, text: &str, ctrl: bool, shift: bool, alt: bool) -> bool {
        if (text == "Escape" || text == "Esc") && !ctrl && !alt && !shift {
            return self.handle_escape();
        }
        if text == "Enter" && !ctrl && !alt {
            return self.confirm_active_operation();
        }
        if text == "F4" && !ctrl && !alt && !shift {
            return self.toggle_reference_manager();
        }
        if ctrl && !shift && !alt && text.eq_ignore_ascii_case("c") {
            return self.copy_selection();
        }
        if ctrl && !shift && !alt && text.eq_ignore_ascii_case("v") {
            return self.paste_clipboard();
        }
        if ((ctrl && !shift) || (shift && !ctrl)) && !alt && text.eq_ignore_ascii_case("d") {
            return self.duplicate_selection();
        }
        if text == "Backspace" && !ctrl && !alt && !shift && !self.poly_pen_points.is_empty() {
            // Collecting: Backspace remove o último ponto (constituição 11).
            self.poly_pen_points.pop();
            self.state.mark_dirty();
            return true;
        }
        if (text == "Delete" || text == "Backspace") && !ctrl && !alt && !shift {
            if self.state.session.tools.modal.is_some() && !self.modal_text.is_empty() {
                self.modal_text.pop();
                if self.modal_text.is_empty() {
                    let [x, y] = self.pointer_position;
                    self.update_viewport_transform(x, y);
                } else {
                    self.preview_modal_text();
                }
                return true;
            }
            return self.delete_or_dissolve_selection();
        }
        if self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
        {
            // Atalhos de navegação da viewport (estilo Blender / Teclado Numérico)
            if let Some(key) = numpad_key(text) {
                let plain = !ctrl && !alt && !shift;
                let opposite = (ctrl || alt) && key.len() == 1;
                use petunia_core::ViewPreset as V;
                match key {
                    "1" if opposite => return self.set_view_preset(V::Back),
                    "1" if plain => return self.set_view_preset(V::Front),
                    "3" if opposite => return self.set_view_preset(V::Left),
                    "3" if plain => return self.set_view_preset(V::Right),
                    "7" if opposite => return self.set_view_preset(V::Bottom),
                    "7" if plain => return self.set_view_preset(V::Top),
                    "9" if plain => return self.toggle_view_opposite(),
                    "0" if plain => return self.set_view_preset(V::Persp),
                    "5" if plain => {
                        self.apply(UiIntent::ToggleProjection);
                        return true;
                    }
                    "." if plain => {
                        let _ = self.state.dispatch_command("view.frame_selection");
                        return true;
                    }
                    "/" if plain => {
                        self.state.toggle_isolate();
                        self.state.mark_dirty();
                        return true;
                    }
                    _ => {}
                }
            }
        }
        if (text == "Insert" || text.eq_ignore_ascii_case("d"))
            && !ctrl
            && !alt
            && !shift
            && self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
        {
            self.apply(UiIntent::ToggleEditPivot);
            return true;
        }
        if (text == " " || text == "Space") && !ctrl && !alt && !shift {
            if self.state.session.tools.active_tool == "slice"
                && (self.slice_anchor.is_some() || self.state.session.tools.cut_session.is_some())
            {
                return self.commit_slice();
            }
            if (self.state.session.tools.active_tool == "cut"
                || self.state.session.tools.active_tool == "knife")
                && self.state.session.tools.cut_session.is_some()
            {
                return self.commit_knife();
            }
            if self.tool_modal.is_some() || self.loop_cut.is_some() || self.drag.is_some() {
                return self.confirm_active_operation();
            }
            return self.toggle_micro_inspector();
        }
        if alt && !ctrl && !shift {
            match text.to_ascii_lowercase().as_str() {
                "f" => return self.toggle_menu("file"),
                "e" => return self.toggle_menu("edit"),
                "v" => return self.toggle_menu("view"),
                "h" => return self.toggle_menu("help"),
                _ => {}
            }
        }
        if ctrl && !shift && !alt {
            match text {
                "PageUp" => {
                    let next = match self.state.workspace {
                        Workspace::Model => Workspace::Uv,
                        Workspace::Paint => Workspace::Model,
                        Workspace::Uv => Workspace::Paint,
                    };
                    self.apply(UiIntent::SetWorkspace(next));
                    return true;
                }
                "PageDown" => {
                    let next = match self.state.workspace {
                        Workspace::Model => Workspace::Paint,
                        Workspace::Paint => Workspace::Uv,
                        Workspace::Uv => Workspace::Model,
                    };
                    self.apply(UiIntent::SetWorkspace(next));
                    return true;
                }
                _ => {}
            }
        }
        if self.state.session.tools.active_tool == "slice"
            && text.eq_ignore_ascii_case("t")
            && !ctrl
            && !alt
        {
            let next_trim = !self.slice_trim;
            self.set_slice_trim(next_trim);
            self.state.set_status(if next_trim {
                "Slice: Modo Trim (Remove o lado cortado) · Enter para confirmar"
            } else {
                "Slice: Modo Split (Mantém ambos os lados) · Enter para confirmar"
            });
            return true;
        }
        if text.eq_ignore_ascii_case("u")
            && !ctrl
            && !alt
            && !shift
            && self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
            && (self.state.workspace == Workspace::Uv
                || self.state.session.selection_domain == SelectionDomain::Edge)
            && self.toggle_selected_uv_seams()
        {
            return true;
        }
        if text.eq_ignore_ascii_case("p")
            && !ctrl
            && self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
            && self.state.workspace == Workspace::Uv
        {
            if alt {
                if self.clear_all_uv_pins() {
                    return true;
                }
            } else if self.toggle_selected_uv_pins() {
                return true;
            }
        }
        if self.state.workspace == Workspace::Model
            && self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
            && !ctrl
            && !alt
            && !shift
        {
            match text {
                "1" => {
                    self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex));
                    return true;
                }
                "2" => {
                    self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge));
                    return true;
                }
                "3" => {
                    self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face));
                    return true;
                }
                "4" => {
                    self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Object));
                    return true;
                }
                "Tab" => {
                    self.apply(UiIntent::CycleSelectionDomain);
                    return true;
                }
                _ => {}
            }
        }
        if self.state.workspace == Workspace::Paint
            && self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
            && !ctrl
            && !alt
        {
            if text == "[" || text == "{" {
                if shift {
                    self.adjust_brush_hardness(-0.1);
                } else {
                    self.adjust_brush_size(-0.2);
                }
                return true;
            }
            if text == "]" || text == "}" {
                if shift {
                    self.adjust_brush_hardness(0.1);
                } else {
                    self.adjust_brush_size(0.2);
                }
                return true;
            }
        }
        if self.state.session.tools.modal.is_none()
            && self.drag.is_none()
            && self.rename_draft.is_none()
            && !alt
        {
            match text {
                "Left" | "Right" | "Up" | "Down" => {
                    if self.state.workspace == Workspace::Model {
                        let step = if shift {
                            0.01
                        } else if ctrl {
                            1.0
                        } else {
                            0.1
                        };
                        let (dx, dy, dz) = match text {
                            "Left" => (-step, 0.0, 0.0),
                            "Right" => (step, 0.0, 0.0),
                            "Up" => (0.0, step, 0.0),
                            "Down" => (0.0, -step, 0.0),
                            _ => (0.0, 0.0, 0.0),
                        };
                        if self.nudge_selection(dx, dy, dz) {
                            return true;
                        }
                    } else if self.state.workspace == Workspace::Uv {
                        let step = if shift {
                            0.002
                        } else if ctrl {
                            0.05
                        } else {
                            0.01
                        };
                        let (du, dv) = match text {
                            "Left" => (-step, 0.0),
                            "Right" => (step, 0.0),
                            "Up" => (0.0, step),
                            "Down" => (0.0, -step),
                            _ => (0.0, 0.0),
                        };
                        if self.uv_move_selected(du, dv) {
                            return true;
                        }
                    }
                }
                _ => {}
            }
        }
        if self.state.session.tools.modal.is_some() && !ctrl && !alt {
            if let Some(axis) = ["x", "y", "z"]
                .iter()
                .position(|axis| text.eq_ignore_ascii_case(axis))
            {
                let requested = if shift {
                    petunia_core::ModalConstraint::Plane(axis)
                } else {
                    petunia_core::ModalConstraint::Axis(axis)
                };
                let current = self
                    .state
                    .session
                    .tools
                    .modal
                    .as_ref()
                    .map(|op| op.constraint);
                let constraint = if current == Some(requested) {
                    petunia_core::ModalConstraint::Free
                } else {
                    requested
                };
                let _ = self.state.set_modal_constraint(constraint);
                if let Some(drag) = self.drag.as_mut() {
                    drag.rotation_angle = 0.0;
                    drag.last_angle = 0.0;
                }
                if !self.modal_text.is_empty() {
                    self.preview_modal_text();
                } else {
                    let [x, y] = self.pointer_position;
                    self.update_viewport_transform(x, y);
                }
                return true;
            }
            if text == "Backspace" {
                self.modal_text.pop();
                if self.modal_text.is_empty() {
                    let [x, y] = self.pointer_position;
                    self.update_viewport_transform(x, y);
                } else {
                    self.preview_modal_text();
                }
                return true;
            }
            if text.len() == 1
                && text
                    .chars()
                    .all(|c| c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+'))
            {
                self.modal_text.push_str(&text.replace(',', "."));
                self.preview_modal_text();
                return true;
            }
        }
        let Some(key) = input::key_code_from_slint(text) else {
            return false;
        };
        let mods = Mods2 { ctrl, shift, alt };
        let context = match self.state.workspace {
            Workspace::Model => "model",
            Workspace::Paint => "paint",
            Workspace::Uv => "uv",
            #[cfg(feature = "animation-workspace")]
            Workspace::Animate => "animate",
        };
        let Some(action) = self
            .state
            .ui
            .keybinds
            .find_in_context(key, mods, context)
            .map(str::to_owned)
        else {
            if text.eq_ignore_ascii_case("w")
                && !ctrl
                && !alt
                && !shift
                && self.state.session.tools.modal.is_none()
                && self.drag.is_none()
                && self.rename_draft.is_none()
            {
                if self.state.workspace == Workspace::Model {
                    let next = if self.state.session.tools.active_tool == "select" {
                        "box_select"
                    } else {
                        "select"
                    };
                    self.apply(UiIntent::SetActiveTool(next.to_string()));
                    return true;
                } else if self.state.workspace == Workspace::Uv {
                    self.apply(UiIntent::SetActiveTool("select".to_string()));
                    return true;
                }
            }
            return false;
        };
        match action.as_str() {
            "global.undo" => self.apply(UiIntent::Undo),
            "global.redo" => self.apply(UiIntent::Redo),
            "global.reset_camera" => self.apply(UiIntent::ResetCamera),
            "global.toggle_projection" => self.apply(UiIntent::ToggleProjection),
            "model.select_object" => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Object))
            }
            "model.select_vertex" => {
                self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Vertex))
            }
            "model.select_edge" => self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Edge)),
            "model.select_face" => self.apply(UiIntent::SetSelectionDomain(SelectionDomain::Face)),
            "model.tool_select" | "model.select" | "model.select_tool" => {
                if self.state.workspace == Workspace::Model {
                    let next = if self.state.session.tools.active_tool == "select" {
                        "box_select"
                    } else {
                        "select"
                    };
                    self.apply(UiIntent::SetActiveTool(next.to_string()));
                } else if self.state.workspace == Workspace::Uv {
                    self.apply(UiIntent::SetActiveTool("select".to_string()));
                }
            }
            "model.box_select" => self.apply(UiIntent::SetActiveTool("box_select".into())),
            "model.move" | "model.transform" => {
                self.begin_keyboard_transform(TransformKind::Position)
            }
            "model.rotate" => self.begin_keyboard_transform(TransformKind::Rotation),
            "model.scale" => self.begin_keyboard_transform(TransformKind::Scale),
            "model.make_face" => {
                let _ = self.execute_core_command("model.make_face");
            }
            "model.frame_selection" => {
                if self.state.selection_domain().is_component() {
                    let _ = self.execute_core_command("model.make_face");
                } else {
                    let _ = self.execute_core_command("view.frame_selection");
                }
            }
            "model.extrude" => {
                if self.state.session.tools.active_tool == "draw_profile"
                    && (self.active_profile_closed() || self.active_profile_point_count() >= 3)
                {
                    if self.profile_volume_mode
                        == Some(petunia_module_model::ProfileVolumeMode::Extrude)
                    {
                        self.commit_profile_volume();
                    } else {
                        self.enter_profile_volume("extrude");
                    }
                } else {
                    self.execute_shortcut_tool("model.extrude");
                }
            }
            "model.inset" => {
                self.execute_shortcut_tool("model.inset");
            }
            "model.bevel" => {
                self.execute_shortcut_tool("model.bevel");
            }
            "model.delete" => {
                if self.state.session.tools.active_tool == "draw_profile" {
                    self.delete_selected_profile_point();
                } else if self.state.selection_domain().is_component() {
                    self.delete_or_dissolve_selection();
                } else {
                    self.apply(UiIntent::DeleteActiveAsset);
                }
            }
            "model.push_pull" => {
                self.execute_shortcut_tool("model.push_pull");
            }
            "model.knife" => {
                let _ = self.execute_core_command("model.knife");
            }
            "model.extrude_individual" => {
                self.execute_shortcut_tool("model.extrude_individual");
            }
            "model.subdivide" => {
                let _ = self.execute_core_command("model.subdivide");
            }
            "model.merge" => {
                let _ = self.execute_core_command("model.merge");
            }
            "model.loop_cut" => {
                self.apply(UiIntent::SetActiveTool("loop_cut".to_string()));
            }
            "model.measure" => {
                self.apply(UiIntent::SetActiveTool("measure".to_string()));
            }
            "model.primitives" => {
                self.add_menu_open = true;
            }
            "paint.paint" => self.apply(UiIntent::SetActiveTool("brush".into())),
            "paint.size_decrease" => self.adjust_brush_size(-1.0),
            "paint.size_increase" => self.adjust_brush_size(1.0),
            "paint.hardness_decrease" => self.adjust_brush_hardness(-0.1),
            "paint.hardness_increase" => self.adjust_brush_hardness(0.1),
            "global.cycle_mode" => {
                let _ = self.execute_core_command("select.cycle_domain");
            }
            "global.rename" => {
                self.begin_rename();
            }
            "model.slice" => {
                // A ação do keymap só arma a ferramenta; a âncora nasce no
                // pointer-down da viewport.
                self.apply(UiIntent::SetActiveTool("slice".to_string()));
            }
            "model.draw_profile" => {
                self.apply(UiIntent::SetActiveTool("draw_profile".to_string()));
            }
            "global.save_project" => self.apply(UiIntent::SaveProject),
            "global.help" => {
                let _ = self.execute_core_command("help.documentation");
            }
            other
                if other.starts_with("view.")
                    || other.starts_with("model.")
                    || other.starts_with("uv.") =>
            {
                if let Err(error) = self.execute_core_command(other) {
                    self.state.set_status(error.to_string());
                }
            }
            "window.command_palette" => self.apply(UiIntent::OpenCommandSearch),
            _ => return false,
        }
        true
    }

    fn begin_keyboard_transform(&mut self, kind: TransformKind) {
        let tool = match kind {
            TransformKind::Position => "move",
            TransformKind::Rotation => "rotate",
            TransformKind::Scale => "scale",
        };
        let now = std::time::Instant::now();
        let interval_ms = self.preferences.double_tap_interval_ms;

        let is_double_tap = if interval_ms > 0 {
            if let Some((ref last_tool, last_time)) = self.last_tool_press {
                last_tool == tool
                    && now.duration_since(last_time).as_millis() <= interval_ms as u128
            } else {
                false
            }
        } else {
            self.state.session.tools.active_tool == tool
        };

        if is_double_tap {
            self.last_tool_press = None;
            self.apply(UiIntent::SetActiveTool(tool.into()));
            let [x, y] = self.pointer_position;
            if self.begin_viewport_transform(kind, x, y) {
                self.instant_transform = true;
                let label = match kind {
                    TransformKind::Position => "Mover",
                    TransformKind::Rotation => "Rotacionar",
                    TransformKind::Scale => "Escalar",
                };
                self.state.set_status(format!(
                    "{} (Modo Livre) · Mova o mouse, LMB/Enter para confirmar, RMB/Esc para cancelar",
                    label
                ));
            }
        } else {
            self.last_tool_press = Some((tool.to_string(), now));
            self.apply(UiIntent::SetActiveTool(tool.into()));
            self.instant_transform = false;
            let label = match kind {
                TransformKind::Position => "Mover",
                TransformKind::Rotation => "Rotacionar",
                TransformKind::Scale => "Escalar",
            };
            self.state.set_status(format!(
                "Ferramenta {} ativa · Arraste o gizmo, digite o valor ou aperte novamente para Modo Livre",
                label
            ));
        }
    }

    fn preview_modal_text(&mut self) {
        let Ok(value) = numeric::parse_numeric(&self.modal_text) else {
            return;
        };
        if self.tool_modal.is_some() {
            self.set_tool_modal_value(value);
            return;
        }
        let Some(modal) = self.state.session.tools.modal.as_ref() else {
            return;
        };
        let direction = match modal.constraint {
            petunia_core::ModalConstraint::Axis(index) => {
                let mut axis = glam::Vec3::ZERO;
                axis[index] = 1.0;
                axis
            }
            petunia_core::ModalConstraint::Plane(index) => {
                let mut direction = self.state.session.camera.right();
                direction[index] = 0.0;
                direction.normalize_or_zero()
            }
            petunia_core::ModalConstraint::Free => self.state.session.camera.right(),
        };
        if let Err(error) = self.state.update_modal(direction * value, value) {
            self.state.set_status(error.to_string());
        }
    }

    fn confirm_active_operation(&mut self) -> bool {
        if !self.poly_pen_points.is_empty() {
            return self.poly_pen_close();
        }
        if self.tool_modal.is_some() {
            return self.commit_tool_modal();
        }
        if self.loop_cut.is_some() {
            return self.commit_loop_cut();
        }
        if self.state.session.tools.active_tool == "loop_cut" {
            return self.place_loop_cut_hover();
        }
        if self.state.session.tools.active_tool == "slice" {
            return self.commit_slice();
        }
        if self.state.session.tools.active_tool == "cut"
            || self.state.session.tools.active_tool == "knife"
        {
            return self.commit_knife();
        }
        if self.profile_volume_mode.is_some() {
            return self.commit_profile_volume();
        }
        if self.state.session.tools.active_tool == "draw_profile" && self.active_profile_closed() {
            return self.enter_profile_volume("extrude");
        }
        if self.drag.is_some() {
            return self.end_viewport_transform();
        }
        self.commit_transform()
    }

    fn adjust_brush_size(&mut self, steps: f32) {
        let next = (self.state.session.tools.paint_radius + steps).clamp(0.01, 100.0);
        self.apply(UiIntent::SetBrushSize(next));
        self.state.set_status(format!("Brush size: {next:.2}"));
    }

    pub fn set_brush_hardness(&mut self, val: f32) -> bool {
        self.apply(UiIntent::SetBrushHardness(val));
        self.state.mark_dirty();
        self.state.set_status(format!(
            "Brush hardness: {:.0}%",
            self.state.session.tools.brush_hardness * 100.0
        ));
        true
    }

    pub fn adjust_brush_hardness(&mut self, delta: f32) {
        let next = (self.state.session.tools.brush_hardness + delta).clamp(0.0, 1.0);
        self.set_brush_hardness(next);
    }

    pub fn toggle_paint_symmetry_x(&mut self) -> bool {
        self.apply(UiIntent::TogglePaintSymmetryX);
        self.state.mark_dirty();
        self.state.session.tools.paint_symmetry_x
    }

    pub fn toggle_paint_symmetry_y(&mut self) -> bool {
        self.apply(UiIntent::TogglePaintSymmetryY);
        self.state.mark_dirty();
        self.state.session.tools.paint_symmetry_y
    }

    pub fn toggle_paint_symmetry_z(&mut self) -> bool {
        self.apply(UiIntent::TogglePaintSymmetryZ);
        self.state.mark_dirty();
        self.state.session.tools.paint_symmetry_z
    }

    pub fn set_paint_symmetry_x(&mut self, val: bool) {
        self.apply(UiIntent::SetPaintSymmetryX(val));
        self.state.mark_dirty();
    }

    pub fn set_paint_symmetry_y(&mut self, val: bool) {
        self.apply(UiIntent::SetPaintSymmetryY(val));
        self.state.mark_dirty();
    }

    pub fn set_paint_symmetry_z(&mut self, val: bool) {
        self.apply(UiIntent::SetPaintSymmetryZ(val));
        self.state.mark_dirty();
    }

    pub fn nudge_selection(&mut self, dx: f32, dy: f32, dz: f32) -> bool {
        if self.state.workspace == Workspace::Uv {
            return self.uv_move_selected(dx, dy);
        }
        if self.state.workspace != Workspace::Model {
            return false;
        }
        let delta = glam::Vec3::new(dx, dy, dz);
        if delta.length_squared() < 1e-8 {
            return false;
        }
        self.state.checkpoint("nudge");
        if self.state.session.edit_mode() == petunia_core::EditMode::Edit {
            if let Some(mesh) = self.state.project.active_mesh_mut() {
                mesh.translate_selected(delta.to_array());
                self.state.emit_positions_changed();
                self.state
                    .set_status(format!("Nudge [{dx:+.2}, {dy:+.2}, {dz:+.2}]"));
                return true;
            }
        } else if let Some(mesh) = self.state.project.active_mesh_mut() {
            for v in &mut mesh.verts {
                v.pos[0] += dx;
                v.pos[1] += dy;
                v.pos[2] += dz;
            }
            self.state.emit_positions_changed();
            self.state
                .set_status(format!("Nudge [{dx:+.2}, {dy:+.2}, {dz:+.2}]"));
            return true;
        }
        false
    }

    pub fn scrub_transform(
        &mut self,
        kind: TransformKind,
        axis: usize,
        delta: f32,
        fine: bool,
    ) -> f32 {
        if self.state.session.tools.modal.is_none()
            && let Err(error) = self.begin_transform(kind)
        {
            self.state.set_status(error.to_string());
            return self.transform_value(kind, axis.min(2));
        }
        let axis = axis.min(2);
        let field = match kind {
            TransformKind::Position => &mut self.position[axis],
            TransformKind::Rotation => &mut self.rotation[axis],
            TransformKind::Scale => &mut self.scale[axis],
        };
        let new_val = field.scrub(delta, fine);
        let components = match kind {
            TransformKind::Position => self.position.map(|field| field.value()),
            TransformKind::Rotation => self.rotation.map(|field| field.value()),
            TransformKind::Scale => self.scale.map(|field| field.value()),
        };
        if let Err(error) = self
            .state
            .update_modal_components(glam::Vec3::from_array(components))
        {
            self.state.set_status(error.to_string());
        }
        new_val
    }

    pub fn begin_transform(&mut self, kind: TransformKind) -> Result<(), petunia_core::ModalError> {
        let modal_kind = match kind {
            TransformKind::Position => petunia_core::ModalKind::Move,
            TransformKind::Rotation => petunia_core::ModalKind::Rotate,
            TransformKind::Scale => petunia_core::ModalKind::Scale,
        };
        self.state.begin_modal(modal_kind)?;
        self.reset_transform_fields();
        Ok(())
    }

    pub fn commit_transform(&mut self) -> bool {
        let had_operation = self.state.session.tools.modal.is_some();
        self.last_operation = self.state.commit_modal_gesture();
        had_operation
    }

    pub fn commit_transform_text(
        &mut self,
        kind: TransformKind,
        axis: usize,
        text: &str,
    ) -> Result<f32, numeric::NumericInputError> {
        let current_base = match kind {
            TransformKind::Position => self.position[axis.min(2)].value(),
            TransformKind::Rotation => self.rotation[axis.min(2)].value(),
            TransformKind::Scale => self.scale[axis.min(2)].value(),
        };
        // Reject malformed text before opening a transaction.
        let value = numeric::parse_numeric_with_base(text, current_base)?;
        let kind_modal = match kind {
            TransformKind::Position => petunia_core::ModalKind::Move,
            TransformKind::Rotation => petunia_core::ModalKind::Rotate,
            TransformKind::Scale => petunia_core::ModalKind::Scale,
        };
        if self
            .state
            .session
            .tools
            .modal
            .as_ref()
            .is_some_and(|modal| modal.kind != kind_modal)
        {
            return Err(numeric::NumericInputError::Invalid);
        }
        let started = self.state.session.tools.modal.is_none();
        if started && let Err(error) = self.begin_transform(kind) {
            self.state.set_status(error.to_string());
            return Err(numeric::NumericInputError::Invalid);
        }
        let mut components = match kind {
            TransformKind::Position => self.position.map(|field| field.value()),
            TransformKind::Rotation => self.rotation.map(|field| field.value()),
            TransformKind::Scale => self.scale.map(|field| field.value()),
        };
        components[axis.min(2)] = value;
        if let Err(error) = self
            .state
            .update_modal_components(glam::Vec3::from_array(components))
        {
            self.state.set_status(error.to_string());
            if started {
                self.cancel_transform();
            }
            return Err(numeric::NumericInputError::Invalid);
        }
        let fields = match kind {
            TransformKind::Position => &mut self.position,
            TransformKind::Rotation => &mut self.rotation,
            TransformKind::Scale => &mut self.scale,
        };
        for (field, value) in fields.iter_mut().zip(components) {
            field.set_value(value);
        }
        self.state.commit_modal();
        Ok(value)
    }

    pub fn cancel_transform(&mut self) -> bool {
        let cancelled = self.state.cancel_modal();
        if cancelled {
            self.reset_transform_fields();
        }
        cancelled
    }

    pub fn view_model(&self) -> ShellViewModel {
        let mut vm = ShellViewModel::from_state(&self.state);
        vm.paint_target_vertex = self.paint_target_vertex;
        vm.paint_mask_selection = self.state.session.tools.paint_isolate_selection;
        vm.position = [
            self.position[0].value(),
            self.position[1].value(),
            self.position[2].value(),
        ];
        vm.rotation = [
            self.rotation[0].value(),
            self.rotation[1].value(),
            self.rotation[2].value(),
        ];
        vm.scale = [
            self.scale[0].value(),
            self.scale[1].value(),
            self.scale[2].value(),
        ];
        vm.is_orthographic = self.state.session.camera.proj == petunia_core::Projection::Ortho;
        vm.is_wireframe = self.state.session.show_wireframe_overlay;
        vm.asset_library_visible = self.asset_library_visible;
        vm.parts_query = self.parts_query.clone();
        vm.parts_selected_only = self.parts_selected_only;
        vm.parts_sort_by_name = self.parts_sort_by_name;
        vm.parts_row_height = self.parts_row_height;
        let parts_query = self.parts_query.trim().to_lowercase();
        vm.parts_items = vm
            .scene_items
            .iter()
            .filter(|item| {
                (parts_query.is_empty() || item.name.to_lowercase().contains(&parts_query))
                    && (!self.parts_selected_only || item.selected || item.active)
            })
            .cloned()
            .collect();
        if self.parts_sort_by_name {
            vm.parts_items.sort_by(|a, b| {
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        vm.asset_query = self.asset_query.clone();
        vm.asset_sort_by_name = self.asset_sort_by_name;
        vm.asset_thumbnail_size = self.state.ui.asset_thumbnail_size;
        let query = self.asset_query.trim().to_lowercase();
        vm.asset_items = vm
            .scene_items
            .iter()
            .filter(|item| query.is_empty() || item.name.to_lowercase().contains(&query))
            .cloned()
            .collect();
        if self.asset_sort_by_name {
            vm.asset_items.sort_by(|a, b| {
                a.name
                    .to_lowercase()
                    .cmp(&b.name.to_lowercase())
                    .then_with(|| a.id.cmp(&b.id))
            });
        }
        vm.gizmo = compute_gizmo(&self.state, self.viewport_size[0], self.viewport_size[1]);
        vm.selection_overlay = compute_selection_overlay(
            &self.state,
            self.viewport_size[0],
            self.viewport_size[1],
            self.viewport.draws_component_guides(),
        );
        vm.add_menu_open = self.add_menu_open;
        if let Some(draft) = &self.rename_draft {
            vm.rename_active = true;
            vm.rename_value = draft.clone();
        }
        vm.shading_popover_open = self.shading_popover_open;
        vm.section_states = self.section_state_models();
        vm.transform_instant_active =
            self.instant_transform && self.state.session.tools.modal.is_some();
        vm.gizmo_hover_axis = self
            .gizmo_hover
            .and_then(|h| h.axis())
            .map_or(-1, |a| a as i32);
        vm.gizmo_active_axis = self
            .gizmo_drag
            .and_then(|h| h.axis())
            .map_or(-1, |a| a as i32);
        vm.gizmo_has_hover = self.gizmo_hover.is_some();
        vm.gizmo_center_active = matches!(self.gizmo_drag, Some(GizmoHandle::Center));
        vm.gizmo_center_hover = matches!(self.gizmo_hover, Some(GizmoHandle::Center));
        vm.gizmo_constraint_axes = match self
            .state
            .session
            .tools
            .modal
            .as_ref()
            .map(|op| op.constraint)
        {
            Some(petunia_core::ModalConstraint::Axis(axis)) => [axis as i32, -1],
            Some(petunia_core::ModalConstraint::Plane(excluded)) => {
                [((excluded + 1) % 3) as i32, ((excluded + 2) % 3) as i32]
            }
            _ => [-1, -1],
        };
        let link_active = self.drag.is_some()
            || self.tool_modal.is_some()
            || self.state.session.tools.modal.is_some();
        vm.drag_link_commands = compute_drag_link(
            &self.state,
            self.viewport_size[0],
            self.viewport_size[1],
            self.pointer_position,
            link_active,
        );
        vm.proportional_circle_commands = compute_proportional_circle(
            &self.state,
            self.viewport_size[0],
            self.viewport_size[1],
            link_active,
        );
        if let Some(anchor) = self.slice_anchor {
            let dx = self.pointer_position[0] - anchor[0];
            let dy = self.pointer_position[1] - anchor[1];
            let len = dx.hypot(dy);
            if len > 2.0 {
                let dir_x = dx / len;
                let dir_y = dy / len;
                let p1 = [anchor[0] - dir_x * 2000.0, anchor[1] - dir_y * 2000.0];
                let p2 = [anchor[0] + dir_x * 2000.0, anchor[1] + dir_y * 2000.0];
                vm.drag_link_commands =
                    format!("M {:.2} {:.2} L {:.2} {:.2} ", p1[0], p1[1], p2[0], p2[1]);
            }
        }
        vm.hover_label = self.state.session.tools.hover.label();
        self.fill_operation_hud(&mut vm);

        if vm.operation_hud_active {
            vm.hud_pill_visible = true;
            vm.hud_pill_title = vm.operation_hud_title.clone();
            vm.hud_pill_badge = if self.is_instant_tool_mode() {
                "MODO LIVRE".to_string()
            } else if !vm.operation_hud_subject.is_empty() {
                vm.operation_hud_subject.clone()
            } else {
                "CARD".to_string()
            };
            vm.hud_pill_value = vm.operation_hud_lines.join("   ");
            vm.hud_pill_hint = vm.operation_hud_hint.clone();
            if self.pointer_position[0] > 0.0 && self.pointer_position[1] > 0.0 {
                vm.hud_pill_x = (self.pointer_position[0] + 16.0)
                    .clamp(16.0, (self.viewport_size[0] - 260.0).max(16.0));
                vm.hud_pill_y = (self.pointer_position[1] + 16.0)
                    .clamp(16.0, (self.viewport_size[1] - 90.0).max(16.0));
            } else {
                vm.hud_pill_x = 24.0;
                vm.hud_pill_y = 64.0;
            }
        }

        let axis_guide =
            compute_axis_guide(&self.state, self.viewport_size[0], self.viewport_size[1]);
        vm.axis_guide_visible = axis_guide.visible;
        vm.axis_guide_commands = axis_guide.commands;
        vm.axis_guide_color = axis_guide.color;
        vm.axis_guide_label = axis_guide.label;
        vm.axis_guide_label_x = axis_guide.label_x;
        vm.axis_guide_label_y = axis_guide.label_y;

        vm.world_axis_labels =
            compute_world_axis_labels(&self.state, self.viewport_size[0], self.viewport_size[1]);

        let dimension =
            compute_dimension_annotation(&self.state, self.viewport_size[0], self.viewport_size[1]);
        vm.dimension_visible = dimension.visible;
        vm.dimension_commands = dimension.commands;
        vm.dimension_text = dimension.text;
        vm.dimension_x = dimension.label_x;
        vm.dimension_y = dimension.label_y;

        let measure =
            compute_quick_measure(&self.state, self.viewport_size[0], self.viewport_size[1]);
        vm.measure_visible = measure.visible;
        vm.measure_commands = measure.commands;
        vm.measure_text = measure.text;
        vm.measure_x = measure.label_x;
        vm.measure_y = measure.label_y;
        vm.measure_distance = measure.distance;
        vm.measure_dx = measure.dx;
        vm.measure_dy = measure.dy;
        vm.measure_dz = measure.dz;
        vm.measure_angle_deg = measure.angle_deg;
        vm.measure_hud_text = measure.hud_text;
        vm.measure_tags = measure.tags;

        vm.micro_inspector_open = self.micro_inspector_open;
        vm.micro_inspector_x = self.micro_inspector_pos[0];
        vm.micro_inspector_y = self.micro_inspector_pos[1];

        let snap_marker = self.snap_marker_model(self.viewport_size[0], self.viewport_size[1]);
        vm.snap_marker_visible = snap_marker.visible;
        vm.snap_marker_x = snap_marker.x;
        vm.snap_marker_y = snap_marker.y;
        vm.snap_marker_label = snap_marker.label;
        vm.snap_marker_round = snap_marker.round;

        let protractor = compute_protractor(
            &self.state,
            self.viewport_size[0],
            self.viewport_size[1],
            self.drag.as_ref(),
        );
        vm.protractor_visible = protractor.visible;
        vm.protractor_wedge_commands = protractor.wedge_commands;
        vm.protractor_ticks_commands = protractor.ticks_commands;

        if let Some(menu) = self.context_menu {
            vm.context_menu_open = true;
            vm.context_menu_x = menu.x;
            vm.context_menu_y = menu.y;
            if menu.viewport {
                vm.context_menu_mode = "viewport".to_string();
                vm.context_menu_title = "Viewport".to_string();
            } else if let Some(asset) = self
                .state
                .project
                .assets
                .iter()
                .find(|asset| asset.id == menu.asset)
            {
                vm.context_menu_title = asset.name.clone();
                vm.context_menu_visible = asset.visible;
                vm.context_menu_locked = asset.locked;
                vm.context_menu_isolated = self.state.session.isolate_active;
                if let Some(idx) = self.state.project.find(menu.asset) {
                    vm.context_menu_can_move_up = idx > 0;
                    vm.context_menu_can_move_down = idx + 1 < self.state.project.assets.len();
                }
            }
        }
        if let Some(operand) = self.state.session.tools.boolean_operand {
            if let Some(asset) = self
                .state
                .project
                .assets
                .iter()
                .find(|asset| asset.id == operand)
            {
                vm.boolean_operand_name = asset.name.clone();
            } else {
                vm.boolean_operand_name = "missing".to_string();
            }
        }
        vm.boolean_keep_parts = self.state.session.tools.boolean_keep_parts;
        vm.boolean_ready = self.state.session.tools.boolean_operand.is_some()
            && self.state.project.active_mesh().is_some();
        if let Some(kind) = self.menu_open {
            vm.menu_open = kind.id().to_string();
        }
        vm.pivot_menu_open = self.pivot_menu_open;
        let translated = |id: petunia_config::TextId| self.state.t_id(id);
        vm.label_parts = translated(petunia_config::text_id::UI_PARTS);
        vm.label_project_asset_library =
            translated(petunia_config::text_id::UI_PROJECT_ASSET_LIBRARY);
        vm.label_save_active_as_asset =
            translated(petunia_config::text_id::UI_SAVE_ACTIVE_AS_ASSET);
        vm.label_status_hint = translated(petunia_config::text_id::UI_STATUS_HINT);
        vm.label_unwrap_mesh = translated(petunia_config::text_id::UI_UNWRAP_MESH);
        vm.label_pack_islands = translated(petunia_config::text_id::UI_PACK_ISLANDS);
        vm.label_active_brush_color = translated(petunia_config::text_id::UI_ACTIVE_BRUSH_COLOR);
        vm.label_albedo_base_color = translated(petunia_config::text_id::UI_ALBEDO_BASE_COLOR);
        vm.label_theme = translated(petunia_config::text_id::UI_THEME);
        vm.label_place_in_scene = translated(petunia_config::text_id::UI_PLACE_IN_SCENE);
        vm.label_vertical_tool_drag = translated(petunia_config::text_id::UI_VERTICAL_TOOL_DRAG);
        vm.label_invert_vertical_drag =
            translated(petunia_config::text_id::UI_INVERT_VERTICAL_DRAG);
        vm.label_search_assets = translated(petunia_config::text_id::UI_SEARCH_ASSETS);
        vm.label_search_parts = translated(petunia_config::text_id::UI_SEARCH_PARTS);
        vm.label_inspector = translated(petunia_config::text_id::UI_INSPECTOR);
        vm.label_expand_inspector = translated(petunia_config::text_id::UI_EXPAND_INSPECTOR);
        vm.label_collapse_inspector = translated(petunia_config::text_id::UI_COLLAPSE_INSPECTOR);
        vm.label_resize_panel_width = translated(petunia_config::text_id::UI_RESIZE_PANEL_WIDTH);
        vm.label_section_dock = translated(petunia_config::text_id::UI_SECTION_DOCK);
        vm.label_section_drag = translated(petunia_config::text_id::UI_SECTION_DRAG);
        vm.label_section_pin_open = translated(petunia_config::text_id::UI_SECTION_PIN_OPEN);
        vm.label_section_pin_asset = translated(petunia_config::text_id::UI_SECTION_PIN_ASSET);
        vm.label_section_unpin_asset = translated(petunia_config::text_id::UI_SECTION_UNPIN_ASSET);
        vm.label_object_name = translated(petunia_config::text_id::UI_OBJECT_NAME);
        vm.label_object_visibility = translated(petunia_config::text_id::UI_OBJECT_VISIBILITY);
        vm.label_object_lock = translated(petunia_config::text_id::UI_OBJECT_LOCK);
        vm.label_object_no_selection = translated(petunia_config::text_id::UI_OBJECT_NO_SELECTION);
        vm.label_stats_faces = translated(petunia_config::text_id::UI_STATS_FACES);
        vm.label_stats_verts = translated(petunia_config::text_id::UI_STATS_VERTS);
        vm.label_stats_tris = translated(petunia_config::text_id::UI_STATS_TRIS);
        vm.label_stats_selection = translated(petunia_config::text_id::UI_STATS_SELECTION);
        vm.label_tool_options = translated(petunia_config::text_id::UI_TOOL_OPTIONS);
        vm.label_tool_options_expand = translated(petunia_config::text_id::UI_TOOL_OPTIONS_EXPAND);
        vm.label_tool_options_collapse =
            translated(petunia_config::text_id::UI_TOOL_OPTIONS_COLLAPSE);
        vm.label_quick_actions = translated(petunia_config::text_id::UI_QUICK_ACTIONS);
        vm.label_quick_action_customize =
            translated(petunia_config::text_id::UI_QUICK_ACTION_CUSTOMIZE);
        vm.label_quick_action_add = translated(petunia_config::text_id::UI_QUICK_ACTION_ADD);
        vm.label_quick_action_remove = translated(petunia_config::text_id::UI_QUICK_ACTION_REMOVE);
        vm.label_quick_action_reset = translated(petunia_config::text_id::UI_QUICK_ACTION_RESET);
        vm.label_quick_action_done = translated(petunia_config::text_id::UI_QUICK_ACTION_DONE);
        vm.label_action_subdivide = translated(petunia_config::text_id::UI_ACTION_SUBDIVIDE);
        vm.label_action_fuse = translated(petunia_config::text_id::UI_ACTION_FUSE);
        vm.label_action_cut = translated(petunia_config::text_id::UI_ACTION_CUT);
        vm.label_action_intersect = translated(petunia_config::text_id::UI_ACTION_INTERSECT);
        vm.label_action_join = translated(petunia_config::text_id::UI_ACTION_JOIN);
        vm.label_action_merge = translated(petunia_config::text_id::UI_ACTION_MERGE);
        vm.label_action_slice = translated(petunia_config::text_id::UI_ACTION_SLICE);
        vm.label_action_loop_cut = translated(petunia_config::text_id::UI_ACTION_LOOP_CUT);
        vm.label_material_base_color = translated(petunia_config::text_id::UI_MATERIAL_BASE_COLOR);
        vm.label_material_profile = translated(petunia_config::text_id::UI_MATERIAL_PROFILE);
        vm.label_material_roughness = translated(petunia_config::text_id::UI_MATERIAL_ROUGHNESS);
        vm.label_material_metallic = translated(petunia_config::text_id::UI_MATERIAL_METALLIC);
        vm.label_material_normal_scale =
            translated(petunia_config::text_id::UI_MATERIAL_NORMAL_SCALE);
        vm.label_material_advanced = translated(petunia_config::text_id::UI_MATERIAL_ADVANCED);
        vm.label_material_assign = translated(petunia_config::text_id::UI_MATERIAL_ASSIGN);
        vm.label_material_new = translated(petunia_config::text_id::UI_MATERIAL_NEW);
        vm.label_material_duplicate = translated(petunia_config::text_id::UI_MATERIAL_DUPLICATE);
        vm.label_material_remove = translated(petunia_config::text_id::UI_MATERIAL_REMOVE);
        vm.label_material_no_material =
            translated(petunia_config::text_id::UI_MATERIAL_NO_MATERIAL);
        vm.label_material_no_selection =
            translated(petunia_config::text_id::UI_MATERIAL_NO_SELECTION);
        vm.label_material_emission_strength =
            translated(petunia_config::text_id::UI_MATERIAL_EMISSION_STRENGTH);
        vm.label_material_alpha_cutoff =
            translated(petunia_config::text_id::UI_MATERIAL_ALPHA_CUTOFF);
        vm.label_material_texture_albedo =
            translated(petunia_config::text_id::UI_MATERIAL_TEXTURE_ALBEDO);
        vm.label_material_no_texture = translated(petunia_config::text_id::UI_MATERIAL_NO_TEXTURE);
        vm.label_material_create_texture =
            translated(petunia_config::text_id::UI_MATERIAL_CREATE_TEXTURE);
        vm.label_material_clear_texture =
            translated(petunia_config::text_id::UI_MATERIAL_CLEAR_TEXTURE);
        vm.label_material_profile_pbr =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_PBR);
        vm.label_material_profile_unlit =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_UNLIT);
        vm.label_material_profile_toon =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_TOON);
        vm.label_material_profile_glass =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_GLASS);
        vm.label_material_profile_emissive =
            translated(petunia_config::text_id::UI_MATERIAL_PROFILE_EMISSIVE);
        vm.label_material_alpha_opaque =
            translated(petunia_config::text_id::UI_MATERIAL_ALPHA_OPAQUE);
        vm.label_material_alpha_mask = translated(petunia_config::text_id::UI_MATERIAL_ALPHA_MASK);
        vm.label_material_alpha_blend =
            translated(petunia_config::text_id::UI_MATERIAL_ALPHA_BLEND);
        vm.label_modifier_mirror = translated(petunia_config::text_id::UI_MODIFIER_MIRROR);
        vm.label_modifier_symmetry = translated(petunia_config::text_id::UI_MODIFIER_SYMMETRY);
        vm.label_modifiers_empty = translated(petunia_config::text_id::UI_MODIFIERS_EMPTY);
        vm.label_modifier_apply = translated(petunia_config::text_id::UI_MODIFIER_APPLY);
        vm.label_modifier_axis = translated(petunia_config::text_id::UI_MODIFIER_AXIS);
        vm.label_modifier_add_mirror = translated(petunia_config::text_id::UI_MODIFIER_ADD_MIRROR);
        vm.label_modifier_add_symmetry =
            translated(petunia_config::text_id::UI_MODIFIER_ADD_SYMMETRY);
        vm.label_modifier_remove = translated(petunia_config::text_id::UI_MODIFIER_REMOVE);
        vm.label_modifier_move_up = translated(petunia_config::text_id::UI_MODIFIER_MOVE_UP);
        vm.label_modifier_move_down = translated(petunia_config::text_id::UI_MODIFIER_MOVE_DOWN);
        vm.label_modifier_direction = translated(petunia_config::text_id::UI_MODIFIER_DIRECTION);
        vm.label_modifier_positive_to_negative =
            translated(petunia_config::text_id::UI_MODIFIER_POSITIVE_TO_NEGATIVE);
        vm.label_modifier_negative_to_positive =
            translated(petunia_config::text_id::UI_MODIFIER_NEGATIVE_TO_POSITIVE);
        vm.label_tab_parts = translated(petunia_config::text_id::UI_TAB_PARTS);
        vm.label_tab_transform = translated(petunia_config::text_id::UI_TAB_TRANSFORM);
        vm.label_tab_material = translated(petunia_config::text_id::UI_TAB_MATERIAL);
        vm.label_tab_modifiers = translated(petunia_config::text_id::UI_TAB_MODIFIERS);
        vm.material_slots = self
            .state
            .project
            .project
            .materials
            .iter()
            .map(|m| m.name.clone())
            .collect();
        vm.active_material_slot = self
            .active_material_slot
            .clamp(0, vm.material_slots.len().saturating_sub(1) as i32);
        vm.label_tab_object = translated(petunia_config::text_id::UI_TAB_OBJECT);
        vm.label_numeric_field_hint = translated(petunia_config::text_id::UI_NUMERIC_FIELD_HINT);
        vm.label_model_select = translated(petunia_config::text_id::TOOLS_SELECT);
        vm.label_model_position = translated(petunia_config::text_id::TRANSFORM_POSITION);
        vm.label_model_rotate = translated(petunia_config::text_id::TOOLS_ROTATE);
        vm.label_model_scale = translated(petunia_config::text_id::TOOLS_SCALE);
        vm.label_model_transform = translated(petunia_config::text_id::TOOLS_TRANSFORM);
        vm.label_model_lasso = translated(petunia_config::text_id::TOOLS_SELECT_LASSO);
        vm.hint_model_select = translated(petunia_config::text_id::UI_MODEL_SELECT_HINT);
        vm.hint_model_position = translated(petunia_config::text_id::UI_MODEL_POSITION_HINT);
        vm.hint_model_rotate = translated(petunia_config::text_id::UI_MODEL_ROTATE_HINT);
        vm.hint_model_scale = translated(petunia_config::text_id::UI_MODEL_SCALE_HINT);
        vm.hint_model_transform = translated(petunia_config::text_id::UI_MODEL_TRANSFORM_HINT);
        vm.hint_model_lasso = translated(petunia_config::text_id::UI_MODEL_LASSO_HINT);
        vm.label_model_loop_cut = translated(petunia_config::text_id::TOOLS_LOOP_CUT);
        vm.hint_model_loop_cut = translated(petunia_config::text_id::UI_LOOP_CUT_HINT);
        vm.label_model_slice = translated(petunia_config::text_id::TOOLS_SLICE);
        vm.hint_model_slice = translated(petunia_config::text_id::UI_SLICE_HINT);
        vm.label_model_push_pull = translated(petunia_config::text_id::TOOLS_PUSH_PULL);
        vm.label_poly_pen = translated(petunia_config::text_id::TOOLS_POLY_PEN);
        vm.label_poly_pen_hint = translated(petunia_config::text_id::TOOLS_POLY_PEN_HINT);
        vm.poly_pen_preview_commands = self.poly_pen_preview_commands();
        vm.hint_model_push_pull = translated(petunia_config::text_id::UI_PUSH_PULL_HINT);
        vm.label_model_profile = translated(petunia_config::text_id::TOOLS_DRAW_PROFILE);
        vm.hint_model_profile = translated(petunia_config::text_id::UI_PROFILE_HINT);
        vm.label_model_pivot = translated(petunia_config::text_id::TOOLS_PIVOT);
        vm.hint_model_pivot = translated(petunia_config::text_id::UI_PIVOT_HINT);
        vm.label_profile_depth = translated(petunia_config::text_id::UI_PROFILE_DEPTH);
        vm.label_profile_points = translated(petunia_config::text_id::UI_PROFILE_POINTS);
        vm.label_profile_close = translated(petunia_config::text_id::UI_PROFILE_CLOSE);
        vm.label_view_gizmo = translated(petunia_config::text_id::UI_VIEW_GIZMO);
        vm.label_view_gizmo_hint = translated(petunia_config::text_id::UI_VIEW_GIZMO_HINT);
        vm.label_profile_plane = translated(petunia_config::text_id::UI_PROFILE_PLANE);
        vm.label_profile_plane_auto = translated(petunia_config::text_id::UI_PROFILE_PLANE_AUTO);
        vm.label_profile_plane_ground =
            translated(petunia_config::text_id::UI_PROFILE_PLANE_GROUND);
        vm.label_profile_plane_face = translated(petunia_config::text_id::UI_PROFILE_PLANE_FACE);
        vm.label_profile_plane_view = translated(petunia_config::text_id::UI_PROFILE_PLANE_VIEW);
        vm.label_profile_look_at_plane =
            translated(petunia_config::text_id::UI_PROFILE_LOOK_AT_PLANE);
        vm.label_profile_generate = translated(petunia_config::text_id::UI_PROFILE_GENERATE);
        vm.label_profile_revolve = translated(petunia_config::text_id::UI_PROFILE_REVOLVE);
        vm.label_profile_sweep = translated(petunia_config::text_id::UI_PROFILE_SWEEP);
        vm.label_profile_cuts = translated(petunia_config::text_id::UI_PROFILE_CUTS);
        vm.label_profile_presets = translated(petunia_config::text_id::UI_PROFILE_PRESETS);
        vm.label_profile_add_rect = translated(petunia_config::text_id::UI_PROFILE_ADD_RECT);
        vm.label_profile_add_circle = translated(petunia_config::text_id::UI_PROFILE_ADD_CIRCLE);
        vm.label_profile_canvas_hint = translated(petunia_config::text_id::UI_PROFILE_CANVAS_HINT);
        vm.label_profile_wall_thickness =
            translated(petunia_config::text_id::UI_PROFILE_WALL_THICKNESS);
        vm.label_profile_smooth_curves =
            translated(petunia_config::text_id::UI_PROFILE_SMOOTH_CURVES);
        vm.label_profile_sharp_corners =
            translated(petunia_config::text_id::UI_PROFILE_SHARP_CORNERS);
        vm.label_profile_smoothness = translated(petunia_config::text_id::UI_PROFILE_SMOOTHNESS);
        vm.label_decal_transform = translated(petunia_config::text_id::UI_DECAL_TRANSFORM);
        vm.label_decal_position = translated(petunia_config::text_id::UI_DECAL_POSITION);
        vm.label_decal_scale = translated(petunia_config::text_id::UI_DECAL_SCALE);
        vm.label_decal_rotation = translated(petunia_config::text_id::UI_DECAL_ROTATION);
        vm.label_decal_bake = translated(petunia_config::text_id::UI_DECAL_BAKE);
        vm.label_decal_hint = translated(petunia_config::text_id::UI_DECAL_HINT);
        vm.label_parametric_primitive =
            translated(petunia_config::text_id::UI_PRIMITIVE_PARAMETRIC);
        vm.label_freeze_primitive = translated(petunia_config::text_id::UI_PRIMITIVE_FREEZE);
        vm.label_freeze_primitive_hint =
            translated(petunia_config::text_id::UI_PRIMITIVE_FREEZE_HINT);
        vm.label_hide_part = translated(petunia_config::text_id::UI_HIDE_PART);
        vm.label_show_part = translated(petunia_config::text_id::UI_SHOW_PART);
        vm.label_lock_part = translated(petunia_config::text_id::UI_LOCK_PART);
        vm.label_unlock_part = translated(petunia_config::text_id::UI_UNLOCK_PART);
        vm.label_selected_parts_only = translated(petunia_config::text_id::UI_SELECTED_PARTS_ONLY);
        vm.label_sort_parts = translated(petunia_config::text_id::UI_SORT_PARTS);
        vm.label_parts_row_size = translated(petunia_config::text_id::UI_PARTS_ROW_SIZE);
        vm.label_sort_assets = translated(petunia_config::text_id::UI_SORT_ASSETS);
        vm.label_thumbnail_size = translated(petunia_config::text_id::UI_THUMBNAIL_SIZE);
        vm.label_selection_color = translated(petunia_config::text_id::UI_SELECTION_COLOR);
        vm.label_highlight_thickness = translated(petunia_config::text_id::UI_HIGHLIGHT_THICKNESS);
        vm.label_view_wireframe = translated(petunia_config::text_id::UI_VIEW_WIREFRAME);
        vm.label_view_wireframe_hint = translated(petunia_config::text_id::UI_VIEW_WIREFRAME_HINT);
        vm.label_view_solid = translated(petunia_config::text_id::UI_VIEW_SOLID);
        vm.label_view_solid_hint = translated(petunia_config::text_id::UI_VIEW_SOLID_HINT);
        vm.label_view_material = translated(petunia_config::text_id::UI_VIEW_MATERIAL);
        vm.label_view_material_hint = translated(petunia_config::text_id::UI_VIEW_MATERIAL_HINT);
        vm.label_view_lit = translated(petunia_config::text_id::UI_VIEW_LIT);
        vm.label_view_lit_hint = translated(petunia_config::text_id::UI_VIEW_LIT_HINT);
        vm.label_more_model_tools = translated(petunia_config::text_id::UI_MORE_MODEL_TOOLS);
        vm.label_xray_opacity = translated(petunia_config::text_id::UI_XRAY_OPACITY);
        vm.label_wire_overlay = translated(petunia_config::text_id::UI_WIRE_OVERLAY);
        vm.label_wire_overlay_hint = translated(petunia_config::text_id::UI_WIRE_OVERLAY_HINT);
        if let Some(info) = &self.pending_recovery {
            vm.recovery_open = true;
            vm.recovery_title = translated(petunia_config::text_id::UI_RECOVERY_TITLE);
            vm.recovery_body = translated(petunia_config::text_id::UI_RECOVERY_BODY);
            vm.recovery_detail = format!(
                "{}  ·  snapshot {}  ·  {}",
                info.project_name,
                info.snapshot_time,
                info.snapshot_path.display()
            );
            vm.recovery_recover = translated(petunia_config::text_id::UI_RECOVERY_RECOVER);
            vm.recovery_keep = translated(petunia_config::text_id::UI_RECOVERY_KEEP);
            vm.recovery_discard = translated(petunia_config::text_id::UI_RECOVERY_DISCARD);
        }
        vm.label_asset_library = translated(petunia_config::text_id::UI_ASSETS);
        vm.label_preferences = translated(petunia_config::text_id::MENU_PREFERENCES);
        // A linha de rodapé das preferências informa o keymap e o idioma REAIS em uso.
        vm.shell_info = format!(
            "{}: {}  ·  {}: {}",
            translated(petunia_config::text_id::UI_THEME),
            self.state.ui.active_theme_id,
            "Keymap",
            self.state.ui.active_keymap_id,
        );
        vm.label_apply = translated(petunia_config::text_id::ACTIONS_APPLY);
        vm.label_cancel = translated(petunia_config::text_id::ACTIONS_CANCEL);
        vm.label_delete = translated(petunia_config::text_id::ACTIONS_DELETE);
        vm.label_duplicate = translated(petunia_config::text_id::ACTIONS_DUPLICATE);
        vm.themes = petunia_config::theme::ThemeRegistry::global()
            .available()
            .iter()
            .map(|manifest| ThemeEntryModel {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                active: manifest.id == self.state.ui.active_theme_id,
            })
            .collect();
        for kind in MenuKind::ALL {
            let entries: Vec<MenuEntryModel> = kind
                .items()
                .iter()
                .map(|(id, label, shortcut)| MenuEntryModel {
                    id: (*id).to_string(),
                    label: translated(*label),
                    shortcut: (*shortcut).to_string(),
                })
                .collect();
            match kind {
                MenuKind::File => {
                    vm.menu_file_label = translated(kind.title());
                    vm.menu_file_items = entries;
                }
                MenuKind::Edit => {
                    vm.menu_edit_label = translated(kind.title());
                    vm.menu_edit_items = entries;
                }
                MenuKind::View => {
                    vm.menu_view_label = translated(kind.title());
                    vm.menu_view_items = entries;
                }
                MenuKind::Window => {
                    vm.menu_window_label = translated(kind.title());
                    vm.menu_window_items = entries;
                }
            }
        }
        if let Some(effect) = self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .and_then(|layer| match &layer.kind {
                petunia_project::paint_layers::LayerKind::Effect(effect) => Some(*effect),
                _ => None,
            })
        {
            use petunia_project::paint_layers::PaintEffect;
            vm.paint_effect_kind = match &effect {
                PaintEffect::Pixelate { .. } => "Pixelate",
                PaintEffect::Posterize { .. } => "Posterize",
                PaintEffect::Invert => "Invert",
                PaintEffect::Grain { .. } => "Grain",
                PaintEffect::Levels { .. } => "Levels",
                PaintEffect::BrightnessContrast { .. } => "BrightnessContrast",
                PaintEffect::HueSaturation { .. } => "HueSaturation",
            }
            .to_string();
            vm.paint_effect_params = effect_params(&effect);
        }
        if let Some(decal) = self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .and_then(|layer| match &layer.kind {
                petunia_project::paint_layers::LayerKind::Decal(decal) => Some(decal.clone()),
                _ => None,
            })
        {
            // Populates decal transformation fields (P3D-133)
            // Preenche campos de transformação do decalque (P3D-133)
            vm.active_layer_is_decal = true;
            vm.decal_center_u = decal.center_uv[0];
            vm.decal_center_v = decal.center_uv[1];
            vm.decal_scale_u = decal.scale_uv[0];
            vm.decal_scale_v = decal.scale_uv[1];
            vm.decal_rotation_deg = decal.rotation_rad.to_degrees();
            vm.decal_preview_commands = self.decal_preview_commands();
        }
        if let Some((width, height)) = self.paint_canvas_dimensions() {
            vm.paint_canvas_size = format!("{width} × {height}");
            vm.paint_canvas_revision = self.state.project.assets[self.state.project.active]
                .paint_stack
                .as_ref()
                .map(|stack| {
                    stack
                        .layers
                        .iter()
                        .filter_map(|layer| layer.canvas())
                        .map(|canvas| canvas.w as i32 * canvas.h as i32)
                        .sum::<i32>()
                        + stack.layers.len() as i32
                })
                .unwrap_or(0);
        }
        vm.paint_fill_scope = format!("{:?}", self.state.session.tools.fill_scope);
        vm.paint_projection = format!("{:?}", self.state.session.tools.brush_projection);
        vm.paint_lock = format!("{:?}", self.state.session.tools.brush_lock);
        vm.paint_pixel_grid = self.paint_pixel_grid;
        vm.paint_canvas_zoom = self.paint_canvas_zoom;
        vm.paint_canvas_grid_commands = self.paint_canvas_grid_commands();
        vm.paint_show_uv_overlay = self.paint_show_uv_overlay;
        vm.uv_show_texture = self.uv_show_texture;
        vm.uv_editor = if cfg!(test)
            || self.state.workspace == Workspace::Uv
            || (self.state.workspace == Workspace::Paint && self.paint_show_uv_overlay)
        {
            self.build_uv_editor()
        } else {
            UvEditorModel::default()
        };
        if let Some(stack) = self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|asset| asset.paint_stack.as_ref())
        {
            vm.paint_layers = stack
                .layers
                .iter()
                .enumerate()
                .map(|(index, layer)| PaintLayerModel {
                    id: layer.id.to_string(),
                    name: layer.name.clone(),
                    visible: layer.visible,
                    locked: layer.locked,
                    opacity: layer.opacity,
                    active: index == stack.active_layer,
                    is_group: layer.is_group,
                    kind_label: match layer.kind {
                        petunia_project::paint_layers::LayerKind::Raster(_) => "Raster",
                        petunia_project::paint_layers::LayerKind::Decal(_) => "Decal",
                        petunia_project::paint_layers::LayerKind::Effect(_) => "Effect",
                    }
                    .to_string(),
                })
                .collect();
            let (width, height) = self
                .state
                .project
                .assets
                .get(self.state.project.active)
                .and_then(|asset| asset.paint_stack.as_ref())
                .and_then(|stack| stack.active().and_then(|layer| layer.canvas()))
                .map(|canvas| (canvas.w, canvas.h))
                .unwrap_or((0, 0));
            vm.paint_layer_count =
                format!("{} layer(s)  ·  {width} × {height}", stack.layers.len());
        }
        if let Some(session) = &self.loop_cut {
            vm.loop_cut_active = true;
            vm.loop_cut_slide = session.slide;
            vm.loop_cut_cuts = session.cuts as i32;
            vm.loop_cut_balanced = session.balanced;
            let preview_res = if session.balanced {
                session
                    .ring
                    .preview_balanced(&session.source, session.cuts, session.slide)
            } else {
                session
                    .ring
                    .preview(&session.source, session.cuts, session.slide)
            };
            if let Ok(segments) = preview_res {
                let mut commands = String::new();
                for [a, b] in segments {
                    project_preview_segment(
                        &self.state.session.camera,
                        self.viewport_size,
                        a,
                        b,
                        &mut commands,
                    );
                }
                vm.loop_cut_preview_commands = commands;
            }
        } else if self.state.session.tools.active_tool == "loop_cut" {
            vm.loop_cut_armed = true;
            vm.loop_cut_cuts = self.loop_cut_hover_cuts as i32;
            vm.loop_cut_balanced = self.loop_cut_balanced;
            vm.loop_cut_preview_commands = self.loop_cut_hover_preview_commands();
        }
        vm.profile_active = self.state.session.tools.active_tool == "draw_profile";
        vm.profile_point_count = self.active_profile_point_count() as i32;
        vm.profile_closed = self.active_profile_closed();
        vm.profile_depth = self.state.profile.depth;
        vm.profile_wall_thickness = self
            .active_profile_resources()
            .map_or(self.state.profile.wall_thickness, |(profile, _)| {
                profile.wall_thickness as f32
            });
        vm.profile_smoothness = self.state.profile.curve_smoothness;
        vm.profile_has_curves = self.active_profile_has_curves();
        if vm.profile_active {
            vm.profile_preview_commands = self.profile_preview_commands();
        }
        vm.profile_workplane =
            petunia_module_model::profile_workplane_label(&self.state).to_string();
        vm.profile_workplane_locked = self.state.profile.workplane_locked;
        vm.region_hover_commands = self.region_hover_commands();
        vm.modeling_mode = self.modeling_mode.id().to_string();
        vm.label_workspace_draw_title = self
            .state
            .t_id(petunia_config::text_id::WORKSPACE_DRAW_TITLE);
        vm.label_workspace_draw_description = self
            .state
            .t_id(petunia_config::text_id::WORKSPACE_DRAW_DESCRIPTION);
        vm.label_workspace_poly_title = self
            .state
            .t_id(petunia_config::text_id::WORKSPACE_POLY_TITLE);
        vm.label_workspace_poly_description = self
            .state
            .t_id(petunia_config::text_id::WORKSPACE_POLY_DESCRIPTION);
        vm.profile_volume_mode = match self.profile_volume_mode {
            Some(petunia_module_model::ProfileVolumeMode::Extrude) => "extrude".to_string(),
            Some(petunia_module_model::ProfileVolumeMode::Revolve) => "revolve".to_string(),
            Some(petunia_module_model::ProfileVolumeMode::Sweep) => "sweep".to_string(),
            None => "none".to_string(),
        };
        vm.profile_revolve_angle = if self.state.profile.revolve_angle <= 0.0 {
            360.0
        } else {
            self.state.profile.revolve_angle
        };
        vm.slice_trim = self.slice_trim;
        vm.bevel_clamp_overlap = self.state.tools.bevel_clamp_overlap;
        vm.bevel_affect_vertices = self.state.tools.bevel_affect_vertices;
        if let Some(anchor) = self.slice_anchor {
            let mut cmd = format!(
                "M {:.2} {:.2} L {:.2} {:.2}",
                anchor[0], anchor[1], self.pointer_position[0], self.pointer_position[1]
            );
            let r = 5.0;
            cmd.push_str(&format!(
                " M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z",
                anchor[0] - r,
                anchor[1] - r,
                anchor[0] + r,
                anchor[1] - r,
                anchor[0] + r,
                anchor[1] + r,
                anchor[0] - r,
                anchor[1] + r,
            ));
            cmd.push_str(&format!(
                " M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z",
                self.pointer_position[0],
                self.pointer_position[1] - r,
                self.pointer_position[0] + r,
                self.pointer_position[1],
                self.pointer_position[0],
                self.pointer_position[1] + r,
                self.pointer_position[0] - r,
                self.pointer_position[1],
            ));
            if self.slice_trim {
                let dx = self.pointer_position[0] - anchor[0];
                let dy = self.pointer_position[1] - anchor[1];
                let len = dx.hypot(dy);
                if len > 4.0 {
                    let dir_x = dx / len;
                    let dir_y = dy / len;
                    let nx = dir_y;
                    let ny = -dir_x;
                    let mid_x = (anchor[0] + self.pointer_position[0]) * 0.5;
                    let mid_y = (anchor[1] + self.pointer_position[1]) * 0.5;
                    let arrow_x = mid_x + nx * 24.0;
                    let arrow_y = mid_y + ny * 24.0;
                    let w1_x = arrow_x - nx * 7.0 + dir_x * 5.0;
                    let w1_y = arrow_y - ny * 7.0 + dir_y * 5.0;
                    let w2_x = arrow_x - nx * 7.0 - dir_x * 5.0;
                    let w2_y = arrow_y - ny * 7.0 - dir_y * 5.0;
                    cmd.push_str(&format!(
                        " M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2}",
                        mid_x, mid_y, arrow_x, arrow_y,
                        arrow_x, arrow_y, w1_x, w1_y,
                        arrow_x, arrow_y, w2_x, w2_y
                    ));
                }
            }
            vm.operation_preview_commands = cmd.clone();
            vm.slice_preview_visible = true;
            vm.slice_preview_commands = cmd;
        }
        vm.tool_activation = self.state.session.tools.tool_activation.id().to_string();
        vm.keyboard_tool_modal_active =
            self.keyboard_tool_modal_active && self.tool_modal.is_some();
        vm.is_instant_tool_mode = self.is_instant_tool_mode();
        vm.invert_vertical_drag = self.state.ui.invert_vertical_drag;
        vm.colorblind_axes = self.state.ui.colorblind_axes;
        vm.reduced_motion = self.state.ui.reduced_motion;
        vm.multiselection_measure_tag = self.state.ui.multiselection_measure_tag;
        vm.double_tap_interval_ms = self.preferences.double_tap_interval_ms as i32;
        vm.tool_grammar_active = self.tool_grammar_active();
        vm.tool_gesture_latched = self.tool_session.is_latched();
        vm.drag_threshold_px = self.preferences.drag_threshold_px;
        vm.snap_radius_px = self.preferences.snap_radius_px;
        vm.label_snap_radius = translated(petunia_config::text_id::PREFERENCES_SNAP_RADIUS);
        vm.click_move_click = self.preferences.click_move_click;
        vm.workplane_prefer_ground = self.preferences.workplane_prefer_ground;
        vm.studio_light_follows_camera = self.preferences.studio_light_follows_camera;
        vm.label_studio_light_follows_camera =
            translated(petunia_config::text_id::PREFERENCES_STUDIO_LIGHT_FOLLOWS_CAMERA);
        vm.label_studio_light_follows_camera_hint =
            translated(petunia_config::text_id::PREFERENCES_STUDIO_LIGHT_FOLLOWS_CAMERA_HINT);
        vm.label_workplane_prefer_ground =
            translated(petunia_config::text_id::PREFERENCES_WORKPLANE_PREFER_GROUND);
        vm.label_workplane_prefer_ground_hint =
            translated(petunia_config::text_id::PREFERENCES_WORKPLANE_PREFER_GROUND_HINT);
        vm.label_drag_threshold = translated(petunia_config::text_id::PREFERENCES_DRAG_THRESHOLD);
        vm.label_click_move_click =
            translated(petunia_config::text_id::PREFERENCES_CLICK_MOVE_CLICK);
        vm.label_click_move_click_hint =
            translated(petunia_config::text_id::PREFERENCES_CLICK_MOVE_CLICK_HINT);
        vm.label_last_operation = translated(petunia_config::text_id::TOOL_GRAMMAR_LAST_OPERATION);
        vm.label_last_operation_hint =
            translated(petunia_config::text_id::TOOL_GRAMMAR_ADJUST_HINT);
        if self.last_operation_adjustable()
            && let Some(last) = &self.last_operation
        {
            use petunia_core::ModalKind as Kind;
            vm.last_operation_active = true;
            vm.last_operation_title = match last.kind {
                Kind::Move => self.state.t("tools.move"),
                Kind::Rotate => self.state.t_id(petunia_config::text_id::TOOLS_ROTATE),
                Kind::Scale => self.state.t_id(petunia_config::text_id::TOOLS_SCALE),
                Kind::Extrude | Kind::ExtrudeIndividual => self.state.t("tools.extrude"),
                Kind::Inset => self.state.t("tools.inset"),
                Kind::Bevel => self.state.t("tools.bevel"),
                Kind::PushPull => self.state.t_id(petunia_config::text_id::TOOLS_PUSH_PULL),
            };
            vm.last_operation_value = last.primary_value();
            let (step, unit) = match last.kind {
                Kind::Rotate => (5.0, "°"),
                Kind::Scale => (0.1, "×"),
                Kind::Inset => (0.01, ""),
                Kind::Bevel => (0.01, "m"),
                Kind::Move | Kind::Extrude | Kind::ExtrudeIndividual | Kind::PushPull => (0.1, "m"),
            };
            vm.last_operation_step = step;
            vm.last_operation_unit = unit.to_string();
        }
        vm.label_colorblind_axes = translated(petunia_config::text_id::PREFERENCES_COLORBLIND_AXES);
        vm.label_reduced_motion = translated(petunia_config::text_id::PREFERENCES_REDUCED_MOTION);
        vm.label_double_tap_interval =
            translated(petunia_config::text_id::PREFERENCES_DOUBLE_TAP_INTERVAL);
        vm.label_multiselection_measure_tag =
            translated(petunia_config::text_id::PREFERENCES_MULTISELECTION_MEASURE);
        vm.active_language = self.state.ui.i18n.lang.clone();
        vm.ui_scale = self.preferences.ui_scale;
        vm.icon_theme = self.preferences.icon_theme.clone();
        if let Some(kind) = self.tool_modal {
            let (minimum, maximum) = kind.bounds();
            vm.tool_modal_active = true;
            vm.tool_modal_id = kind.id().to_string();
            vm.tool_modal_title = kind.title().to_string();
            vm.tool_modal_label = kind.label().to_string();
            vm.tool_modal_value = self.tool_modal_value;
            vm.tool_modal_step = kind.step();
            vm.tool_modal_min = minimum;
            vm.tool_modal_max = maximum;
        }
        vm.tool_options_active = self.tool_modal.is_some()
            || self.loop_cut.is_some()
            || self.state.session.tools.active_tool == "loop_cut"
            || self.state.session.tools.active_tool == "draw_profile"
            || (self.state.workspace == Workspace::Model
                && !matches!(
                    self.state.session.tools.active_tool.as_str(),
                    "select" | "box_select" | "lasso_select"
                ));
        vm.tool_options_title = if let Some(kind) = self.tool_modal {
            kind.title().to_string()
        } else if self.loop_cut.is_some() || self.state.session.tools.active_tool == "loop_cut" {
            self.state.t_id(petunia_config::text_id::TOOLS_LOOP_CUT)
        } else if self.state.session.tools.active_tool == "draw_profile" {
            self.state.t_id(petunia_config::text_id::TOOLS_DRAW_PROFILE)
        } else {
            match self.state.session.tools.active_tool.as_str() {
                "scale" => self.state.t_id(petunia_config::text_id::TOOLS_SCALE),
                "rotate" => self.state.t_id(petunia_config::text_id::TOOLS_ROTATE),
                "move" => self.state.t_id(petunia_config::text_id::TOOLS_TRANSFORM),
                "slice" => self.state.t_id(petunia_config::text_id::TOOLS_SLICE),
                "knife" => self.state.t("tools.knife"),
                "extrude" => self.state.t("tools.extrude"),
                "extrude_individual" => self.state.t("tools.extrude_individual"),
                "inset" => self.state.t("tools.inset"),
                "bevel" => self.state.t("tools.bevel"),
                "push_pull" => self.state.t_id(petunia_config::text_id::TOOLS_PUSH_PULL),
                "poly_pen" => self.state.t_id(petunia_config::text_id::TOOLS_POLY_PEN),
                "cursor" | "cursor_3d" => "3D Cursor".to_string(),
                "measure" => self.state.t("tools.measure"),
                other => {
                    let key = format!("tools.{other}");
                    let val = self.state.t(&key);
                    if val != key {
                        val
                    } else if !other.is_empty() {
                        let mut chars = other.chars();
                        match chars.next() {
                            Some(first) => {
                                first.to_uppercase().collect::<String>() + chars.as_str()
                            }
                            None => "Tool Options".to_string(),
                        }
                    } else {
                        "Tool Options".to_string()
                    }
                }
            }
        };
        vm.tool_options_hint = if self.tool_modal.is_some() {
            self.state
                .t_id(petunia_config::text_id::UI_NUMERIC_FIELD_HINT)
        } else {
            self.state
                .t_id(petunia_config::text_id::UI_NO_TOOL_PARAMETERS)
        };
        if let Some(asset) = self.section_asset(petunia_config::InspectorSectionId::Object) {
            vm.object_has_selection = true;
            vm.object_id = asset.id.to_string();
            vm.object_name = asset.name.clone();
            vm.object_visible = asset.visible;
            vm.object_locked = asset.locked;
            vm.object_verts = asset.mesh.verts.len() as i32;
            vm.object_faces = asset.mesh.faces.len() as i32;
            vm.object_tris = asset.mesh.tri_count() as i32;
            vm.object_selection = vm.selection_summary.clone();
            vm.object_material = asset
                .material(&self.state.project.project)
                .map(|material| material.name.clone())
                .unwrap_or_else(|| {
                    self.state
                        .t_id(petunia_config::text_id::UI_MATERIAL_NO_MATERIAL)
                });
            vm.object_modifier_count = asset.modifiers.len() as i32;
        }
        // Display follows the pinned asset's material when the Material section
        // is pinned and resolvable; slot actions stay selection-contextual.
        // Exibição segue o material do asset fixado quando resolvível; ações de
        // slot seguem selection-context.
        let material_slot = self
            .section_asset(petunia_config::InspectorSectionId::Material)
            .and_then(|asset| asset.material_id)
            .and_then(|id| {
                self.state
                    .project
                    .project
                    .materials
                    .iter()
                    .position(|material| material.id == id)
            })
            .unwrap_or_else(|| vm.active_material_slot.max(0) as usize);
        if let Some(material) = self.state.project.project.materials.get(material_slot) {
            vm.material_has_selection = true;
            vm.material_id = material.id.to_string();
            vm.material_name = material.name.clone();
            vm.material_profile = match material.profile {
                petunia_project::ShaderProfile::Pbr => "pbr",
                petunia_project::ShaderProfile::Unlit => "unlit",
                petunia_project::ShaderProfile::Toon => "toon",
                petunia_project::ShaderProfile::Glass => "glass",
                petunia_project::ShaderProfile::Emissive => "emissive",
            }
            .to_string();
            vm.material_profile_label = material.profile.label().to_string();
            vm.material_base_color = [
                material.base_color[0],
                material.base_color[1],
                material.base_color[2],
            ];
            vm.material_roughness = material.roughness;
            vm.material_metallic = material.metallic;
            vm.material_normal_scale = material.normal_scale;
            vm.material_emission = material.emission_color;
            vm.material_emission_strength = material.emission_strength;
            vm.material_alpha_mode = match material.alpha_mode {
                petunia_project::AlphaMode::Opaque => "opaque",
                petunia_project::AlphaMode::Mask => "mask",
                petunia_project::AlphaMode::Blend => "blend",
            }
            .to_string();
            vm.material_alpha_cutoff = material.alpha_cutoff;
            vm.material_has_albedo = material.albedo_texture.is_some();
            vm.material_albedo_label = material
                .albedo_texture
                .as_ref()
                .map(|texture| format!("{} × {}", texture.w, texture.h))
                .unwrap_or_else(|| {
                    self.state
                        .t_id(petunia_config::text_id::UI_MATERIAL_NO_TEXTURE)
                });
        }
        vm.material_palette = self.state.project.palette.clone();
        let pinned_ids = self.state.ui.model_quick_action_ids();
        let quick_label = |id: &str| match id {
            "model.subdivide" => self
                .state
                .t_id(petunia_config::text_id::UI_ACTION_SUBDIVIDE),
            "model.fuse" => self.state.t_id(petunia_config::text_id::UI_ACTION_FUSE),
            "model.cut" => self.state.t_id(petunia_config::text_id::UI_ACTION_CUT),
            "model.intersect" => self
                .state
                .t_id(petunia_config::text_id::UI_ACTION_INTERSECT),
            "model.join" => self.state.t_id(petunia_config::text_id::UI_ACTION_JOIN),
            "model.merge" => self.state.t_id(petunia_config::text_id::UI_ACTION_MERGE),
            "model.slice" => self.state.t_id(petunia_config::text_id::UI_ACTION_SLICE),
            "model.loop_cut" => self.state.t_id(petunia_config::text_id::UI_ACTION_LOOP_CUT),
            _ => id.to_string(),
        };
        vm.quick_actions = pinned_ids
            .iter()
            .map(|id| QuickActionModel {
                id: id.clone(),
                label: quick_label(id),
                enabled: self.state.commands.can_execute(id, &self.state).is_ok()
                    || id == "model.slice",
                active: self.state.session.tools.active_tool == id.as_str(),
                pinned: true,
            })
            .collect();
        vm.quick_action_candidates = petunia_core::state::UiState::MODEL_QUICK_ACTION_CANDIDATES
            .iter()
            .map(|id| QuickActionModel {
                id: (*id).to_string(),
                label: quick_label(id),
                enabled: self.state.commands.can_execute(id, &self.state).is_ok()
                    || *id == "model.slice",
                active: self.state.session.tools.active_tool == *id,
                pinned: pinned_ids.iter().any(|pinned| pinned == id),
            })
            .collect();
        if let Some(asset) = self.section_asset(petunia_config::InspectorSectionId::Modifiers) {
            vm.modifier_rows = asset
                .modifiers
                .iter()
                .enumerate()
                .map(|(index, modifier)| {
                    let (kind, axis, positive_to_negative) = match modifier.kind {
                        petunia_project::ModifierKind::Mirror { axis, .. } => {
                            ("mirror", axis, true)
                        }
                        petunia_project::ModifierKind::Symmetry {
                            axis,
                            positive_to_negative,
                            ..
                        } => ("symmetry", axis, positive_to_negative),
                    };
                    let title = if kind == "mirror" {
                        self.state.t_id(petunia_config::text_id::UI_MODIFIER_MIRROR)
                    } else {
                        self.state
                            .t_id(petunia_config::text_id::UI_MODIFIER_SYMMETRY)
                    };
                    ModifierRowModel {
                        id: modifier.id.to_string(),
                        subtitle: format!(
                            "{} {}",
                            self.state.t_id(petunia_config::text_id::UI_MODIFIER_AXIS),
                            ["X", "Y", "Z"][axis.min(2)]
                        ),
                        title,
                        enabled: modifier.enabled,
                        kind: kind.to_string(),
                        axis: axis as i32,
                        positive_to_negative,
                        can_move_up: index > 0,
                        can_move_down: index + 1 < asset.modifiers.len(),
                    }
                })
                .collect();
        }
        vm.settings_visible = self.settings_visible;
        vm.command_search_visible = self.command_search_visible;
        vm.scene_drawer_visible = self.scene_drawer_visible;
        vm.reference_manager_open = self.reference_manager_open;
        for slot in &mut vm.reference_slots {
            if let Some((w, h, rgba)) = self.reference_thumbnails.get(&slot.axis) {
                slot.thumbnail = files::create_thumbnail_image(*w, *h, rgba);
            }
        }
        vm.active_keymap_id = self.state.ui.active_keymap_id.clone();
        vm
    }

    fn sync_viewport_context(&mut self) {
        self.viewport.set_workspace(self.state.workspace);
        self.viewport
            .set_selection_domain(self.state.selection_domain());
    }

    fn reset_transform_fields(&mut self) {
        self.modal_text.clear();
        self.instant_transform = false;
        self.gizmo_drag = None;
        for field in &mut self.position {
            field.set_value(0.0);
        }
        for field in &mut self.rotation {
            field.set_value(0.0);
        }
        for field in &mut self.scale {
            field.set_value(1.0);
        }
    }

    fn transform_value(&self, kind: TransformKind, axis: usize) -> f32 {
        match kind {
            TransformKind::Position => self.position[axis].value(),
            TransformKind::Rotation => self.rotation[axis].value(),
            TransformKind::Scale => self.scale[axis].value(),
        }
    }

    fn hide_overlay(&mut self, id: OverlayId) {
        match id {
            OverlayId::CommandPalette => self.command_search_visible = false,
            OverlayId::Settings => self.settings_visible = false,
            OverlayId::SceneDrawer => self.scene_drawer_visible = false,
            OverlayId::AssetLibrary => self.asset_library_visible = false,
            OverlayId::OutlinerContextMenu => self.context_menu = None,
            OverlayId::ContextMenu => self.context_menu = None,
            OverlayId::MenuBar => self.menu_open = None,
            OverlayId::PivotMenu => self.pivot_menu_open = false,
            OverlayId::MicroInspector => self.micro_inspector_open = false,
            OverlayId::ReferenceManager => self.reference_manager_open = false,
        }
    }
}

/// Executa o frontend de produção com composição WGPU direta ou fallback software.
pub fn run() -> Result<(), slint::PlatformError> {
    let gpu_context = viewport_gpu::WgpuViewport::create_wgpu_context().ok();
    if let Some((instance, adapter, device, queue)) = gpu_context.as_ref() {
        slint::BackendSelector::new()
            .require_wgpu_30(slint::wgpu_30::WGPUConfiguration::Manual {
                instance: instance.clone(),
                adapter: adapter.clone(),
                device: device.clone(),
                queue: queue.clone(),
            })
            .select()?;
    }

    println!("Petunia3D - Slint production frontend");
    let window = PetuniaSlintShell::new()?;
    let mut state = AppState::default();
    let preferences = petunia_config::UserPreferences::load();
    state.ui.invert_vertical_drag = preferences.invert_vertical_drag;
    state.ui.colorblind_axes = preferences.colorblind_axes;
    state.ui.reduced_motion = preferences.reduced_motion;
    state.ui.multiselection_measure_tag = preferences.multiselection_measure_tag;
    state.ui.selection_rgb = if selection_color_has_contrast(preferences.selection_rgb) {
        preferences.selection_rgb
    } else {
        petunia_config::UserPreferences::default().selection_rgb
    };
    state.ui.selection_thickness = preferences.selection_thickness.clamp(1.0, 6.0);
    // Clone (at most 6 short ids): `preferences` stays whole for the section
    // restore below. / Clone (no máximo 6 ids curtos): `preferences` segue
    // íntegro para o restore das seções abaixo.
    state.ui.model_quick_actions = preferences.model_quick_actions.clone();
    if !preferences.active_keymap_id.is_empty() {
        state.ui.active_keymap_id = preferences.active_keymap_id.clone();
        state.ui.keybinds = petunia_config::Keybinds::load_profile(&preferences.active_keymap_id);
    }

    let mut viewport: Box<dyn PetuniaViewport> = if let Some((_, _, device, queue)) = gpu_context {
        println!("Viewport backend: shared WGPU fast path");
        Box::new(viewport_gpu::WgpuViewport::new(
            Arc::new(device),
            Arc::new(queue),
            1024,
            768,
        ))
    } else {
        println!("Viewport backend: software compatibility path");
        Box::new(Software3dViewport::new(1024, 768))
    };

    let render_state = ViewportRenderState {
        shading: state.session.shading,
        xray: state.session.show_xray,
        show_triangulation: state.session.show_triangulation,
        textured: state.session.textured,
        show_wireframe_overlay: state.session.show_wireframe_overlay,
        show_face_orientation: state.session.show_face_orientation,
        show_uv_checker: state.session.show_uv_checker,
        selection_domain: state.selection_domain(),
        xray_opacity: state.session.xray_opacity,
        selection_rgb: state.ui.selection_rgb,
        selection_thickness: state.ui.selection_thickness,
        show_grid: state.session.show_grid,
        hover: state.session.tools.hover,
        boolean_operand: state.session.tools.boolean_operand,
        studio_light_follows_camera: true,
        edge_mode: ModelingMode::default().edge_mode(),
    };
    if let Some(frame) = viewport.render_frame(
        &state.project,
        &state.project.refs,
        &state.session.camera,
        render_state,
    ) {
        window.set_viewport_image(frame);
    }
    window.set_has_gpu_viewport(true);

    let mut startup_bridge = SlintUiBridge::new(state, viewport);
    // Section layouts persist per module: restore them onto the runtime state
    // and seed the preferences cache so later mutations persist everything.
    // Layouts de seção persistem por módulo: restaura no estado runtime e
    // semeia o cache para mutações futuras persistirem tudo.
    startup_bridge.restore_section_layouts(&preferences);
    let bridge = Arc::new(Mutex::new(startup_bridge));

    // Ciclo de vida do autosave (P3D-002): marcador de sessão no arranque,
    // detecção de encerramento sujo e remoção no fechamento limpo.
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    {
        let mut bridge = bridge
            .lock()
            .expect("Slint bridge mutex poisoned during startup");
        bridge.pending_recovery = petunia_core::AutosaveService::detect_recovery(None);
        let project_name = bridge
            .state
            .project
            .project_path
            .clone()
            .unwrap_or_else(|| "Untitled".to_string());
        let path = bridge.state.project.project_path.clone();
        if let Err(error) = petunia_core::AutosaveService::create_session_lock(
            path.as_deref().map(std::path::Path::new),
            &project_name,
            now_secs,
        ) {
            eprintln!("petunia3d: falha ao gravar marcador de sessão: {error}");
        }
    }

    connect_callbacks(&window, Arc::clone(&bridge));
    let vm = bridge
        .lock()
        .expect("Slint bridge mutex poisoned during startup")
        .view_model();
    sync_window_properties(&window, &vm);

    // O primeiro layout pode ocorrer antes da instalação dos callbacks de
    // resize. Use suas dimensões reais antes do primeiro frame interativo.
    let initial_bridge = Arc::clone(&bridge);
    let initial_window = window.as_weak();
    slint::Timer::single_shot(std::time::Duration::ZERO, move || {
        if let Some(window) = initial_window.upgrade()
            && let Ok(mut bridge) = initial_bridge.lock()
        {
            bridge.resize_viewport_scaled(
                window.get_viewport_region_width().round().max(1.0) as u32,
                window.get_viewport_region_height().round().max(1.0) as u32,
                window.window().scale_factor(),
            );
            sync_window_properties(&window, &bridge.view_model());
            if let Some(frame) = bridge.render_viewport() {
                window.set_viewport_image(frame);
            }
        }
    });

    // Um passo de autosave a cada 30s; o intervalo real (120s) e o dirty state
    // são decididos pelo domínio, então o timer só oferece a oportunidade.
    let autosave_bridge = Arc::clone(&bridge);
    let autosave_timer = slint::Timer::default();
    autosave_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_secs(30),
        move || {
            if let Ok(mut bridge) = autosave_bridge.lock() {
                bridge.autosave_tick();
            }
        },
    );

    // Tique de acúmulo contínuo de tinta para Airbrush (P3D-056).
    let airbrush_bridge = Arc::clone(&bridge);
    let airbrush_window = window.as_weak();
    let airbrush_timer = slint::Timer::default();
    airbrush_timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(50),
        move || {
            if let Ok(mut bridge) = airbrush_bridge.lock()
                && bridge.airbrush_tick()
                && let Some(window) = airbrush_window.upgrade()
            {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(canvas_img) = bridge.render_paint_canvas() {
                    window.set_paint_canvas_image(canvas_img);
                }
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        },
    );

    println!("Petunia3D window ready");
    let result = window.run();

    drop(airbrush_timer);
    drop(autosave_timer);
    let path = bridge
        .lock()
        .map(|bridge| bridge.state.project.project_path.clone())
        .unwrap_or(None);
    petunia_core::AutosaveService::remove_session_lock(path.as_deref().map(std::path::Path::new));
    result
}

// Unit and integration test suite for Slint UI bridge and shell interactions.
// Suíte de testes unitários e de integração para o bridge Slint UI e interações do shell.
#[cfg(test)]
mod tests;

#[cfg(test)]
mod numpad_key_tests {
    use super::numpad_key;

    #[test]
    fn numpad_names_normalize_across_backends() {
        for name in ["KP_1", "Numpad1", "Keypad1", "NumPad1", "Numpad 1"] {
            assert_eq!(numpad_key(name), Some("1"), "{name}");
        }
        for name in [
            "KP_Decimal",
            "NumpadDecimal",
            "KeypadDecimal",
            "Numpad .",
            "KP_.",
        ] {
            assert_eq!(numpad_key(name), Some("."), "{name}");
        }
        for name in [
            "KP_Divide",
            "NumpadDivide",
            "KeypadDivide",
            "Numpad /",
            "KP_/",
        ] {
            assert_eq!(numpad_key(name), Some("/"), "{name}");
        }
        assert_eq!(numpad_key("1"), None);
        assert_eq!(numpad_key("Numpad"), None);
        assert_eq!(numpad_key("KP_Enter"), None);
    }
}
