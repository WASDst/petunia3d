//! Frontend de produção do Petunia3D em Slint 1.18.
//!
//! O shell em Slint emite intenções (`UiIntent`); o domínio continua em
//! `petunia_core`, `petunia_commands` e `petunia_project`. A integração de GPU
//! é realizada pelo adapter `WgpuViewport` ou fallback `Software3dViewport`.
//!
//! A UI egui (`crates/ui/`) é legado de transição, acessível via
//! `--legacy-egui` / `PETUNIA_LEGACY_EGUI=1`.

use std::path::PathBuf;

pub mod animate;
mod bridge;
pub use bridge::{ApplicationState, PaintPointerActions};
mod brush_panel;
pub mod color_wheel;
pub mod commands;
mod decal_tool;
mod draw_extensions;
mod draw_shapes;
pub mod files;
mod host;
mod input;
pub mod inspector_layout;
pub mod keymap_edit;
pub mod numeric;
pub mod overlay;
mod paint_surface_tools;
mod paint_view_tools;
mod scene_cache;
mod shape_builder_tool;
pub mod split_view;
pub mod theme;
pub mod thumbnail;
pub mod tr;
pub mod view_layout;
pub mod viewport_gl;
pub use host::{DesktopHost, run};
pub mod viewport_intents;
pub mod viewport_soft;

use viewport_intents::{
    LogicalViewportPoint, PhysicalPointerModifiers, ViewportSelectionIntent,
    ViewportToolPointerIntent,
};
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

/// Fase semântica de um gesto primário na viewport.
///
/// O Slint ainda usa inteiros no callback público por compatibilidade com a
/// API gerada, mas a lógica Rust não deve depender de números mágicos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewportPointerPhase {
    Press,
    Move,
    Release,
    Cancel,
}

impl From<i32> for ViewportPointerPhase {
    fn from(value: i32) -> Self {
        match value {
            0 => Self::Press,
            1 => Self::Move,
            2 => Self::Release,
            _ => Self::Cancel,
        }
    }
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
    /// Matcap procedural no Solid (preferência).
    pub matcap: bool,
    /// Oclusão ambiente (GTAO) no Solid/Material (preferência).
    pub ambient_occlusion: bool,
    /// Aparência das arestas por modo (DRAW = forma, POLY = topologia).
    pub edge_mode: petunia_render::EdgeMode,
    /// Plano de trabalho em destaque (DRAW, ferramenta de desenho).
    pub workplane: Option<petunia_render::WorkplaneOverlay>,
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
            matcap: false,
            ambient_occlusion: false,
            edge_mode: petunia_render::EdgeMode::Overlay,
            workplane: None,
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

impl GizmoModel {
    /// Remove a geometria das alças (o backend GPU a desenha); rótulos,
    /// tripé de navegação e cursor 3D continuam no shell.
    pub fn clear_handle_geometry(&mut self) {
        for commands in [
            &mut self.x_commands,
            &mut self.y_commands,
            &mut self.z_commands,
            &mut self.x_arrow_commands,
            &mut self.y_arrow_commands,
            &mut self.z_arrow_commands,
            &mut self.x_scale_commands,
            &mut self.y_scale_commands,
            &mut self.z_scale_commands,
            &mut self.x_rotate_commands,
            &mut self.y_rotate_commands,
            &mut self.z_rotate_commands,
            &mut self.plane_yz_commands,
            &mut self.plane_xz_commands,
            &mut self.plane_xy_commands,
            &mut self.view_roll_commands,
        ] {
            commands.clear();
        }
    }
}

/// Subcaminhos `M x y L x y … [Z]` de um comando de `Path`: pontos e se fecha.
fn path_polylines(commands: &str) -> Vec<(Vec<[f32; 2]>, bool)> {
    let mut paths: Vec<(Vec<[f32; 2]>, bool)> = Vec::new();
    let mut numbers = Vec::new();
    let flush = |numbers: &mut Vec<f32>, paths: &mut Vec<(Vec<[f32; 2]>, bool)>| {
        if let Some(path) = paths.last_mut() {
            path.0.extend(numbers.as_chunks::<2>().0.iter().copied());
        }
        numbers.clear();
    };
    for token in commands.split_whitespace() {
        match token {
            "M" => {
                flush(&mut numbers, &mut paths);
                paths.push((Vec::new(), false));
            }
            "L" => flush(&mut numbers, &mut paths),
            "Z" | "z" => {
                flush(&mut numbers, &mut paths);
                if let Some(path) = paths.last_mut() {
                    path.1 = true;
                }
            }
            other => {
                if let Ok(value) = other.parse::<f32>() {
                    numbers.push(value);
                }
            }
        }
    }
    flush(&mut numbers, &mut paths);
    paths.retain(|(points, _)| points.len() >= 2);
    paths
}

/// Cores do gizmo desenhado na GPU: eixos dos tokens do shell
/// (`DesignTokens.axis-*`, entregues em `connect_callbacks`); o resto vem do
/// tema ativo no `ThemeRegistry`, a mesma fonte do `apply_theme`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GizmoAxisColors(pub [[f32; 4]; 3]);

impl Default for GizmoAxisColors {
    fn default() -> Self {
        let rgba = |c: [f32; 3]| [c[0], c[1], c[2], 1.0];
        Self([
            rgba(petunia_render::scene::AXIS_X),
            rgba(petunia_render::scene::AXIS_Y),
            rgba(petunia_render::scene::AXIS_Z),
        ])
    }
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

    /// Nome traduzível da ferramenta (catálogo `tools.*`).
    pub const fn title_key(self) -> &'static str {
        match self {
            Self::Extrude => "tools.extrude",
            Self::ExtrudeIndividual => "tools.extrude_individual",
            Self::Inset => "tools.inset",
            Self::Bevel => "tools.bevel",
            Self::PushPull => "tools.push_pull",
            Self::ScaleSelection => "tools.scale",
        }
    }

    /// Rótulo traduzível do valor principal.
    pub const fn label_id(self) -> petunia_config::TextId {
        use petunia_config::text_id as t;
        match self {
            Self::Extrude | Self::ExtrudeIndividual | Self::PushPull => t::HUD_LABEL_DISTANCE,
            Self::Inset => t::HUD_LABEL_AMOUNT,
            Self::Bevel => t::HUD_LABEL_WIDTH,
            Self::ScaleSelection => t::HUD_LABEL_FACTOR,
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
            Self::Inset => (0.0, 100.0),
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

/// Alvo de press da alça paramétrica (`PressTarget::Handle`).
const PARAMETRIC_HANDLE_TARGET: u8 = 4;

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
    /// Shape Builder (DRAW): arrastar sobre faces as funde; Ctrl-arrastar as apaga;
    /// clique extrai a face.
    ShapeBuilder,
    SurfacePaint,
    /// Decalque ativo no PAINT: clique posiciona, arrastar move, alças
    /// escalam e giram (`decal_tool`).
    Decal,
    /// Draw Profile (DRAW): clique adiciona ponto; arrastar no vazio puxa a
    /// alça do ponto novo; arrastar um nó/alça o move; a alça de profundidade
    /// e o volume em edição seguem o arrasto.
    DrawProfile,
    /// Knife: cliques em arestas definem os segmentos; Enter aplica.
    Knife,
    /// Loop Cut: clique posiciona sobre o anel; arrastar desliza; soltar aplica.
    LoopCut,
    /// Slice: arrastar define o plano; clique confirma.
    Slice,
}

/// Modo do Poly Pen (cap. 46 §4, Polygon Pen do Cinema 4D): que elemento o
/// hover destaca e o que clique e arrasto fazem.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PolyPenMode {
    /// Ponto > aresta > polígono sob o cursor.
    #[default]
    Auto,
    /// Só pontos: arrastar move; cliques desenham; alternativo derrete.
    Points,
    /// Só arestas: clique insere ponto; arrastar move; alternativo extruda.
    Edges,
    /// Polígonos e arestas de borda: arrastar a partir da borda pinta quads.
    Polygons,
}

impl PolyPenMode {
    pub const ALL: [Self; 4] = [Self::Auto, Self::Points, Self::Edges, Self::Polygons];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Points => "points",
            Self::Edges => "edges",
            Self::Polygons => "polygons",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.id() == id)
    }

    /// Domínios que o hover considera, em ordem de prioridade.
    const fn domains(self) -> &'static [SelectionDomain] {
        match self {
            Self::Auto => &[
                SelectionDomain::Vertex,
                SelectionDomain::Edge,
                SelectionDomain::Face,
            ],
            Self::Points => &[SelectionDomain::Vertex],
            Self::Edges => &[SelectionDomain::Edge],
            Self::Polygons => &[SelectionDomain::Edge, SelectionDomain::Face],
        }
    }

    const fn hint_id(self) -> petunia_config::TextId {
        use petunia_config::text_id as t;
        match self {
            Self::Auto => t::TOOLS_POLY_PEN_MODE_AUTO_HINT,
            Self::Points => t::TOOLS_POLY_PEN_MODE_POINTS_HINT,
            Self::Edges => t::TOOLS_POLY_PEN_MODE_EDGES_HINT,
            Self::Polygons => t::TOOLS_POLY_PEN_MODE_POLYGONS_HINT,
        }
    }
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
    /// O estado vive em `SlintUiBridge::shape_builder`.
    ShapeBuilder,
    SurfacePaint,
    /// O estado vive em `SlintUiBridge::decal_gesture`.
    Decal,
    /// Nó ou alça do perfil (estado em `profile_edit_gesture`).
    ProfilePoint,
    /// Ponto novo com a alça puxada pelo arrasto.
    ProfileHandlePull,
    /// Profundidade do perfil fechado (alça ou arrasto vertical).
    ProfileDepth {
        start: f32,
        anchor_y: f32,
    },
    /// Ângulo do Revolve em edição.
    ProfileRevolve {
        start: f32,
        anchor_x: f32,
    },
    /// Deslize do Loop Cut a partir do último ponto do ponteiro.
    LoopSlide {
        last: [f32; 2],
    },
    /// Plano do Slice em definição.
    Slice,
    /// Pintura de faces do Poly Pen (estado em `poly_pen_strip`).
    PolyPenStrip,
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
    ViewportSelection(ViewportSelectionIntent),
    /// Ação do workspace Animate (cap. 45 F2).
    Animate(animate::AnimateIntent),
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
    /// Modo do Poly Pen pelo id (`auto`, `points`, `edges`, `polygons`).
    SetPolyPenMode(String),
    OpenCommandSearch,
    OpenSettings,
    CloseSettings,
    SetKeymap(String),
    ToggleSceneDrawer,
    /// Mostra/esconde a coluna direita inteira (sem trilha de pílulas).
    ToggleInspector,
    /// DRAW: vista planar ↔ câmera livre (só a vista muda).
    TogglePlanarNavigation,
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
    /// Importa uma imagem (PNG/JPEG/WebP/BMP) como camada de decalque.
    ImportDecalFrom(PathBuf),
    /// Acrescenta a imagem como variante do decalque ativo (Decal Set).
    AddDecalVariantFrom(PathBuf),
    /// Exporta a textura com cada variante do decalque ativo e a trilha.
    ExportDecalFramesTo(PathBuf),
    SetDecalVariant(usize),
    RemoveDecalVariant,
    SetDecalKey,
    RemoveDecalKey,
    SetDecalFrame(i32),
    SetDecalTrackFps(f32),
    SetDecalTrackLength(i32),
    ToggleDecalPlayback,
    ImportSvgProfiles(PathBuf),
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
    /// Resolução (lado em px) da textura de pintura do ativo.
    SetPaintTextureSide(u32),
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
    ProfileSetWorkplaneSelection,
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

/// Copia um view-model para o shell, como o bridge faz a cada quadro. Público
/// só para testes headless que precisam dos rótulos reais (auditoria de
/// acessibilidade em `tests/ui_metrics.rs`).
#[doc(hidden)]
pub fn sync_shell_for_tests(window: &PetuniaSlintShell, vm: &ShellViewModel) {
    callbacks::sync_window_properties(window, vm);
}

pub mod projection;
pub use decal_tool::{DecalHandles, DecalMode, DecalParam};
pub use projection::*;

pub mod callbacks;
mod perf;
mod refresh;
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
    /// Malhas deformadas por skin (preview do Animate) que substituem, só ao
    /// desenhar, as malhas em repouso do documento. `None` volta ao repouso.
    fn set_pose_override(&mut self, _pose: Option<std::sync::Arc<petunia_project::PoseOverride>>) {}
    /// Objetos com contorno de seleção (domínio Object) e o ativo.
    fn set_outlined_objects(&mut self, _selected: &[uuid::Uuid], _active: Option<uuid::Uuid>) {}
    /// Objetos em transformação rígida neste quadro (modo objeto): o backend
    /// pode reaproveitar a geometria triangulada aplicando a matriz.
    fn set_rigid_previews(&mut self, _previews: &[(uuid::Uuid, glam::Mat4)]) {}
    /// O backend desenha o gizmo de transformação num passo próprio (GPU,
    /// tamanho constante em pixels, sem depth test); o shell então não o
    /// desenha em 2D. O hit test continua no bridge, sobre a mesma geometria.
    fn draws_gizmo(&self) -> bool {
        false
    }
    /// Overlay de tela em px lógicos (gizmo) para backends que o desenham.
    fn set_screen_overlay(&mut self, _shapes: Vec<petunia_render::OverlayShape>) {}
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

    fn uses_physical_pixels(&self) -> bool {
        (**self).uses_physical_pixels()
    }

    fn set_pixel_ratio(&mut self, ratio: f32) {
        (**self).set_pixel_ratio(ratio);
    }

    fn draws_gizmo(&self) -> bool {
        (**self).draws_gizmo()
    }

    fn set_screen_overlay(&mut self, shapes: Vec<petunia_render::OverlayShape>) {
        (**self).set_screen_overlay(shapes);
    }

    fn set_pose_override(&mut self, pose: Option<std::sync::Arc<petunia_project::PoseOverride>>) {
        (**self).set_pose_override(pose);
    }

    fn set_outlined_objects(&mut self, selected: &[uuid::Uuid], active: Option<uuid::Uuid>) {
        (**self).set_outlined_objects(selected, active);
    }

    fn set_rigid_previews(&mut self, previews: &[(uuid::Uuid, glam::Mat4)]) {
        (**self).set_rigid_previews(previews);
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

/// Bridge entre callbacks Slint e a aplicação. A Application possui a sessão
/// única; o adapter `state` preserva os consumidores durante a migração dos
/// intents. Comandos e histórico passam pela Application, que reutiliza o
/// dispatcher canônico do Core.
/// Miniaturas RGBA por (prefab, revisão); `None` = prefab sem geometria.
type PrefabThumbCache =
    std::collections::HashMap<(uuid::Uuid, u32), Option<std::sync::Arc<Vec<u8>>>>;

pub struct SlintUiBridge<V: PetuniaViewport> {
    pub state: ApplicationState,
    pub viewport: V,
    pub overlays: OverlayStack,
    pub scene_drawer_visible: bool,
    /// Coluna direita visível. Escondida, a viewport ocupa a largura toda;
    /// o acordeão das seções é independente disto.
    pub inspector_visible: bool,
    pub asset_library_visible: bool,
    pub asset_query: String,
    pub asset_sort_by_name: bool,
    /// Biblioteca: mostra só prefabs favoritos.
    pub asset_only_favorites: bool,
    /// Vista 3D secundária opcional (viewport dividida).
    pub split: split_view::SplitView,
    /// PiP do Paint: canvas 2D grande + inset 3D com auto-enquadramento.
    pub paint_pip: bool,
    /// Miniaturas renderizadas por (prefab, revisão); `None` = sem geometria.
    prefab_thumbs: std::cell::RefCell<PrefabThumbCache>,
    inspector_layout: inspector_layout::InspectorLayoutMemory,
    pub parts_query: String,
    pub parts_selected_only: bool,
    pub parts_sort_by_name: bool,
    pub parts_row_height: f32,
    pub command_search_visible: bool,
    pub settings_visible: bool,
    pub position: [NumericFieldState; 3],
    pub rotation: [NumericFieldState; 3],
    pub scale: [NumericFieldState; 3],
    pub transform_initial: [f32; 3],
    pub drag: Option<ViewportDrag>,
    pub viewport_size: [f32; 2],
    /// Última posição de tela do traço de pintura ativo.
    pub paint_last: Option<[f32; 2]>,
    /// Amostragem incremental do stroke 3D em pixels lógicos da viewport.
    pub paint_sampler: petunia_core::StrokeSampler,
    /// Estabilizador do cursor no traço 3D / 2D (suavização do pincel).
    paint_stabilizer: petunia_core::PointStabilizer,
    paint_2d_stabilizer: petunia_core::PointStabilizer,
    /// Presets de pincel do usuário (carregados sob demanda).
    pub brush_presets: Vec<petunia_core::BrushPreset>,
    pub brush_presets_loaded: bool,
    pub brush_presets_path_override: Option<std::path::PathBuf>,
    /// Preset aplicado por último (índice em `all_brush_presets`).
    pub active_brush_preset: Option<usize>,
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
    /// Alt no último evento da gramática (quebrar tangente no perfil).
    tool_pointer_alt: bool,
    /// Modificador de "ação alternativa" no último press (Ctrl no preset
    /// padrão): no Poly Pen, extruda a aresta ou derrete o ponto.
    tool_press_alternate: bool,
    /// Pontos coletados pelo Poly Pen (estado `Collecting`).
    pub poly_pen_points: Vec<petunia_core::PenPoint>,
    /// Modo do Poly Pen (Auto/Points/Edges/Polygons).
    pub poly_pen_mode: PolyPenMode,
    /// Largura em px lógicos em que a textura aparece no canvas 2D (0 =
    /// desconhecida: o pincel 2D fica em texels).
    pub paint_2d_raster_px: f32,
    /// Pintura de faces em andamento (modo Polygons).
    poly_pen_strip: Option<petunia_core::PenStrip>,
    /// Cores dos eixos do gizmo GPU (tokens do shell).
    pub gizmo_axis_colors: GizmoAxisColors,
    /// Cursor sobre a alça da ferramenta paramétrica (destaque).
    pub parametric_handle_hover: bool,
    /// O último press caiu na alça paramétrica (o clique não seleciona).
    tool_press_parametric_handle: bool,
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
    /// Prévia da ferramenta de malha ativa (Loop Cut, Slice): só o viewport a
    /// vê; o documento só muda no commit (`bridge/tool_preview.rs`).
    pub(crate) tool_preview: Option<std::sync::Arc<petunia_project::PoseOverride>>,
    pub(crate) tool_preview_revision: u64,
    pub active_material_slot: i32,
    /// Snapshot de recuperação detectado no arranque, aguardando decisão.
    pub pending_recovery: Option<petunia_core::RecoveryInfo>,
    pub paint_pixel_grid: bool,
    pub paint_canvas_zoom: i32,
    pub paint_2d_last: Option<(u32, u32)>,
    surface_paint: paint_surface_tools::SurfacePaintDraft,
    draw_parameters: draw_extensions::DrawParameters,
    /// Amostragem incremental do stroke no espaço de pixels da textura.
    pub paint_2d_sampler: petunia_core::StrokeSampler,
    pub paint_target_vertex: bool,
    /// Gesto da ferramenta de decalque em andamento (`decal_tool`).
    pub(crate) decal_gesture: Option<decal_tool::DecalGesture>,
    /// Último gesto de decalque confirmado (card "Última operação").
    pub decal_last_operation: Option<decal_tool::DecalLastOperation>,
    /// Quadro atual da trilha de variantes do decalque ativo.
    pub decal_frame: u32,
    /// A trilha de variantes está tocando.
    pub decal_playing: bool,
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
    /// Home (cap. 23) aberta: só no arranque sem projeto, se a preferência pedir.
    pub home_open: bool,
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
    /// Domínio Curve do DRAW: segmento selecionado do perfil ativo.
    pub profile_selected_segment: Option<usize>,
    /// Domínio Region do DRAW: região fechada selecionada (alvo do Push/Pull).
    pub region_selected: Option<petunia_core::RegionHit>,
    /// Pré-seleção do Draw: onde o próximo clique cairia (ponto encaixado ou
    /// face que viraria o plano), mostrada com o marcador de snap.
    pub profile_hover_snap: Option<petunia_core::ScreenSnapHit>,
    /// Região de perfil sob o cursor (Push/Pull e Draw), destacada no hover.
    pub region_hover: Option<petunia_core::RegionHit>,
    /// DRAW ou POLY: mesmos documento, seleção, câmera e Inspector; muda o
    /// trilho de ferramentas (ADR 007).
    pub modeling_mode: ModelingMode,
    /// DRAW usa navegação planar (ortográfica, normal ao Workplane, sem
    /// orbit) enquanto isto for verdadeiro; desligar volta à câmera livre.
    pub draw_planar_navigation: bool,
    /// Câmera de antes da navegação planar, restaurada ao sair dela
    /// (estado de ViewportSession; nunca vai para o documento).
    pub(crate) planar_saved_camera: Option<petunia_core::Camera>,
    /// Regiões por plano, recalculadas só quando o documento muda.
    region_planes_cache: std::cell::RefCell<Option<draw_shapes::ShapeCache>>,
    /// Planos editáveis do Shape Builder, por revisão (perfis em uso por geradores ficam de fora).
    shape_planes_cache: std::cell::RefCell<Option<([u64; 4], Vec<petunia_core::ShapePlane>)>>,
    /// Gesto do Shape Builder em andamento.
    shape_builder: Option<shape_builder_tool::ShapeBuilderGesture>,
    /// Último clique num nó do perfil (duplo clique alterna reto ↔ curva).
    profile_last_anchor_click: Option<(uuid::Uuid, std::time::Instant)>,
    /// Ferramenta usada antes da atual; `Space` alterna entre as duas.
    previous_tool: String,
    /// Cena de consultas da viewport (picking/oclusão), por geometria.
    scene_query_cache: scene_cache::SceneCache,
    /// Geometria de snap (todos os objetos visíveis), por gesto ou revisão.
    snap_accel_cache: std::cell::RefCell<Option<(u64, std::sync::Arc<petunia_core::SnapAccel>)>>,
    /// Edição de atalhos (perfis do usuário, captura de tecla).
    pub keymap_editor: keymap_edit::KeymapEditor,
    /// Id do objeto de prévia do volume de perfil (só desenho; ver
    /// `show_new_object_preview`). `Some` enquanto há prévia.
    pub profile_preview_id: Option<uuid::Uuid>,
    profile_edit_gesture: Option<ProfileEditGesture>,
    profile_transform_gesture: Option<ProfileTransformGesture>,
    profile_dimension_gesture: Option<(
        petunia_project::ProfileResource,
        petunia_project::SplineResource,
    )>,
    pub reference_manager_open: bool,
    pub reference_thumbnails: std::collections::HashMap<String, (u32, u32, Vec<u8>)>,
    pub clipboard: Option<GeometryClipboard>,
    /// Runtime do workspace Animate (cache do avaliador, relógio de reprodução).
    pub animate: animate::AnimateRuntime,
}

/// Dados da área de transferência de geometria.
#[derive(Clone, Debug)]
pub enum GeometryClipboard {
    Mesh(Box<petunia_core::Mesh>),
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
    pub fn edge_mode(self) -> petunia_render::EdgeMode {
        match self {
            Self::Draw => petunia_render::EdgeMode::Features,
            Self::Poly => petunia_render::EdgeMode::Topology,
        }
    }

    /// A ferramenta persistente pertence ao trilho deste modo.
    pub fn offers_tool(self, tool: &str) -> bool {
        const SHARED: [&str; 6] = [
            "select",
            "move",
            "rotate",
            "scale",
            "transform",
            "push_pull",
        ];
        const DRAW: [&str; 2] = ["draw_profile", "shape_builder"];
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

#[derive(Debug, Clone)]
struct ProfileTransformGesture {
    project_before: Project,
    profile_id: uuid::Uuid,
    spline: petunia_core::SplineResource,
    profile: petunia_core::ProfileResource,
    kind: TransformKind,
    anchor: [f32; 2],
    center: [f64; 2],
    constraint: petunia_core::ModalConstraint,
    components: glam::Vec3,
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
    /// Aresta sob o cursor no momento do clique direito (semente dos loops).
    pub edge: Option<(u32, u32)>,
    /// Face sob o cursor no momento do clique direito (semente do Face Loop).
    pub face: Option<usize>,
}

/// Id reservado que representa um divisor entre grupos de itens de menu.
pub const MENU_SEPARATOR: &str = "-";

/// Passo das guias angulares ao desenhar perfis no DRAW (graus).
const DRAW_ANGLE_STEP_DEGREES: f32 = 15.0;

/// Menus da barra superior do shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    File,
    Edit,
    View,
    Window,
}

/// Rótulo do item "Imagens de referência" (chave existente do catálogo `[sl]`).
const MENU_REFERENCE_IMAGES: petunia_config::TextId =
    petunia_config::TextId::new("sl.reference_images");

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
                (MENU_SEPARATOR, T::UI_CLOSE, ""),
                ("file.save", T::FILE_SAVE, "Ctrl+S"),
                ("file.save_as", T::FILE_SAVE_AS, "Ctrl+Shift+S"),
                (MENU_SEPARATOR, T::UI_CLOSE, ""),
                ("file.import_obj", T::FILE_IMPORT_OBJ, ""),
            ],
            Self::Edit => &[
                ("edit.undo", T::EDIT_UNDO, "Ctrl+Z"),
                ("edit.redo", T::EDIT_REDO, "Ctrl+Shift+Z"),
                (MENU_SEPARATOR, T::UI_CLOSE, ""),
                ("edit.duplicate", T::UI_DUPLICATE, "Shift+D"),
            ],
            Self::View => &[
                ("view.frame_selection", T::VIEW_FRAME, "F"),
                ("view.frame_all", T::VIEW_FRAME_ALL, "Home"),
                ("view.reset_camera", T::VIEW_RESET_CAMERA, "Shift+Home"),
                (MENU_SEPARATOR, T::UI_CLOSE, ""),
                ("view.toggle_projection", T::VIEW_TOGGLE_PROJECTION, "O"),
                (
                    "view.pseudo_isometric_2to1",
                    T::VIEW_PSEUDO_ISOMETRIC_2TO1,
                    "Numpad 2",
                ),
                ("view.toggle_wireframe", T::VIEW_TOGGLE_WIREFRAME, "Z"),
                ("view.toggle_split", T::VIEW_TOGGLE_SPLIT, ""),
                (MENU_SEPARATOR, T::UI_CLOSE, ""),
                // Saiu da top bar (plano de UI 2026-10-04, F3): o menu é o
                // caminho visível; F4 continua sendo o atalho.
                ("view.reference_images", MENU_REFERENCE_IMAGES, "F4"),
            ],
            Self::Window => &[
                ("window.parts", T::UI_PARTS, ""),
                ("window.command_palette", T::MENU_COMMAND_PALETTE, "Ctrl+P"),
                (MENU_SEPARATOR, T::UI_CLOSE, ""),
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

// Unit and integration test suite for Slint UI bridge and shell interactions.
// Suíte de testes unitários e de integração para o bridge Slint UI e interações do shell.
/// `#RRGGBB` (com ou sem `#`) para RGB.
fn parse_hex_rgb(value: &str) -> Option<[u8; 3]> {
    let hex = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let mut rgb = [0u8; 3];
    for (index, channel) in rgb.iter_mut().enumerate() {
        *channel = u8::from_str_radix(hex.get(index * 2..index * 2 + 2)?, 16).ok()?;
    }
    Some(rgb)
}

/// Destaque e seleção precisam ser distinguíveis à primeira vista (D5).
/// Distância euclidiana em RGB; 90 separa o violeta padrão de um lilás.
fn colors_too_close(a: [u8; 3], b: [u8; 3]) -> bool {
    let distance_sq: i32 = a
        .iter()
        .zip(b)
        .map(|(&x, y)| (x as i32 - y as i32).pow(2))
        .sum();
    distance_sq < 90 * 90
}

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
