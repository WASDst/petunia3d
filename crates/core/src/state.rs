//! Estado global do editor (vive em `core`, sem conhecer backends).
//! Renderers/módulos/UI operam sobre este estado + eventos.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use glam::Vec3;
use petunia_commands::UndoStack;
use petunia_config::{I18n, Keybinds};
use petunia_project::{Project, ProjectChanges};
use uuid::Uuid;

use super::camera::Camera;
use super::events::{AppEvent, EventBus};
use super::selection::{SelectMode, Selection, SelectionDomain, Workspace};

/// Modo de sombreamento da viewport.
///
/// Nomeia o que o usuário vê, não o modelo de iluminação: cada variante
/// corresponde a um pipeline real no renderer. Vive no core para o estado de
/// sessão não depender de crate de render (cap. 28).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shading {
    /// Só topologia: sem preenchimento de face, arestas neutras.
    Wireframe,
    /// Faces preenchidas com iluminação de estúdio da viewport, cor do objeto.
    #[default]
    Solid,
    /// Faces preenchidas com a textura do material amostrada por UV.
    MaterialPreview,
    /// Faces preenchidas com o material sob a luz da cena.
    Rendered,
}

impl Shading {
    pub const ALL: [Shading; 4] = [
        Shading::Wireframe,
        Shading::Solid,
        Shading::MaterialPreview,
        Shading::Rendered,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Wireframe => "wireframe",
            Self::Solid => "solid",
            Self::MaterialPreview => "material",
            Self::Rendered => "rendered",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "wireframe" => Some(Self::Wireframe),
            "solid" => Some(Self::Solid),
            "material" => Some(Self::MaterialPreview),
            "rendered" => Some(Self::Rendered),
            _ => None,
        }
    }

    /// A viewport desenha faces preenchidas neste modo?
    pub const fn fills_faces(self) -> bool {
        !matches!(self, Self::Wireframe)
    }

    /// A textura do material é amostrada neste modo?
    pub const fn samples_material(self) -> bool {
        matches!(self, Self::MaterialPreview | Self::Rendered)
    }

    /// A iluminação vem da cena (não do estúdio fixo da viewport)?
    pub const fn uses_scene_light(self) -> bool {
        matches!(self, Self::Rendered)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefAxis {
    Front,
    Back,
    Left,
    Right,
    Side,
    Top,
    Bottom,
}

impl RefAxis {
    pub fn key(&self) -> &'static str {
        match self {
            RefAxis::Front => "refs.front",
            RefAxis::Back => "refs.back",
            RefAxis::Left => "refs.left",
            RefAxis::Right | RefAxis::Side => "refs.right",
            RefAxis::Top => "refs.top",
            RefAxis::Bottom => "refs.bottom",
        }
    }
}

/// Imagem de referência: pixels RGBA + posicionamento no espaço.
#[derive(Debug, Clone)]
pub struct ReferenceImage {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub axis: RefAxis,
    pub offset: f32,
    pub size: f32,
    pub opacity: f32,
    pub visible: bool,
    pub locked: bool,
    pub rotation: f32,
    pub xray: bool,
    /// Identidade monotônica do conteúdo de `rgba` (nunca 0). Trocar os pixels
    /// exige `bump_revision()`; o renderer usa isto em vez de varrer os bytes.
    pub revision: u64,
}

impl ReferenceImage {
    /// Próxima revisão global de conteúdo (única por processo, começa em 1).
    pub fn next_revision() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        NEXT.fetch_add(1, Ordering::Relaxed)
    }

    /// Marca que `rgba` foi substituído.
    pub fn bump_revision(&mut self) {
        self.revision = Self::next_revision();
    }

    pub fn from_rgba(name: String, width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            revision: Self::next_revision(),
            name,
            width,
            height,
            rgba,
            axis: RefAxis::Front,
            offset: -3.0,
            size: 4.0,
            opacity: 0.6,
            visible: true,
            locked: false,
            rotation: 0.0,
            xray: false,
        }
    }
}

/// Origem do plano de trabalho do desenho (ADR 007, Onda 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkplaneKind {
    /// Chão do mundo (XZ, normal +Y).
    #[default]
    Ground,
    /// Face plana da malha (sob o cursor ou selecionada).
    Face,
    /// Plano da câmera ou plano do mundo mais paralelo à vista.
    View,
}

/// Perfil 2D do Draw Profile (spec §9), num frame right/up/origin conhecido.
#[derive(Debug, Clone, Default)]
pub struct ProfileState {
    pub points: Vec<[f32; 2]>,
    pub holes: Vec<Vec<[f64; 2]>>,
    pub nodes: Vec<petunia_mesh::curve::BezierNode>,
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub origin: [f32; 3],
    pub normal: [f32; 3],
    pub closed: bool,
    pub depth: f32,
    pub revolve_segments: u32,
    pub revolve_angle: f32,
    pub wall_thickness: f32,
    pub curve_smoothness: f32,
    pub snap: bool,
    /// De onde veio o frame atual (rótulo público do plano).
    pub workplane_kind: WorkplaneKind,
    /// `false` = automático: o 1º clique de um perfil novo escolhe a face sob
    /// o cursor ou o plano do mundo mais paralelo à vista. Escolha explícita
    /// (Chão/Face/Vista) trava o plano até voltar para Auto.
    pub workplane_locked: bool,
    /// Preferência do usuário: no plano automático sem face, favorecer o chão.
    pub workplane_prefer_ground: bool,
}

impl ProfileState {
    pub fn clear(&mut self) {
        let depth = if self.depth == 0.0 { 1.0 } else { self.depth };
        let revolve_segments = if self.revolve_segments == 0 {
            12
        } else {
            self.revolve_segments
        };
        let revolve_angle = if self.revolve_angle <= 0.0 {
            360.0
        } else {
            self.revolve_angle
        };
        let wall_thickness = self.wall_thickness;
        let (workplane_kind, workplane_locked, workplane_prefer_ground) = (
            self.workplane_kind,
            self.workplane_locked,
            self.workplane_prefer_ground,
        );
        let curve_smoothness = if self.curve_smoothness <= 0.0 {
            0.02
        } else {
            self.curve_smoothness
        };
        *self = Self {
            depth,
            revolve_segments,
            revolve_angle,
            wall_thickness,
            curve_smoothness,
            workplane_kind,
            workplane_locked,
            workplane_prefer_ground,
            ..Default::default()
        };
    }

    pub fn to_3d(&self, i: usize) -> Vec3 {
        let p = self.points[i];
        Vec3::from(self.origin) + Vec3::from(self.right) * p[0] + Vec3::from(self.up) * p[1]
    }

    pub fn to_3d_point(&self, p: [f32; 2]) -> Vec3 {
        Vec3::from(self.origin) + Vec3::from(self.right) * p[0] + Vec3::from(self.up) * p[1]
    }

    /// Retorna os pontos amostrados/tesselados avaliando curvas Bézier caso haja alças.
    pub fn tessellated_points(&self) -> Vec<[f32; 2]> {
        if self.nodes.is_empty() {
            return self.points.clone();
        }
        let has_curves = self
            .nodes
            .iter()
            .any(|n| n.handle_in.is_some() || n.handle_out.is_some());
        if has_curves {
            let path = petunia_mesh::curve::BezierPath {
                nodes: self.nodes.clone(),
                closed: self.closed,
            };
            path.tessellate(self.curve_smoothness.max(0.005))
        } else if !self.points.is_empty() {
            self.points.clone()
        } else {
            self.nodes.iter().map(|n| n.point).collect()
        }
    }

    /// Retorna os pontos efetivos para geração de geometria (com espessura de parede oca se configurada).
    pub fn effective_points(&self) -> Vec<[f32; 2]> {
        let pts = self.tessellated_points();
        if self.wall_thickness > 0.0 && self.closed && pts.len() >= 3 {
            petunia_mesh::curve::create_hollow_profile(&pts, self.wall_thickness)
        } else {
            pts
        }
    }

    /// Retorna os pontos do contorno interno se houver espessura de parede configurada.
    pub fn inner_points(&self) -> Vec<[f32; 2]> {
        let pts = self.tessellated_points();
        if self.wall_thickness > 0.0 && self.closed && pts.len() >= 3 {
            petunia_mesh::curve::offset_polygon(&pts, -self.wall_thickness)
        } else {
            Vec::new()
        }
    }
}

/// Estatísticas do último frame (overlay §45).
#[derive(Debug, Clone, Copy, Default)]
pub struct RenderStats {
    pub fps: f32,
    pub frame_ms: f32,
    pub tris: usize,
    pub verts: usize,
    pub draws: usize,
}

/// Visão **derivada** do estado de edição (P3D-015).
///
/// Não é estado próprio e não pode ser escrita: a autoridade semântica é
/// [`SelectionDomain`] (Object / Point / Edge / Face) e o estado de pintura é
/// [`Workspace::Paint`]. Este enum existe apenas como leitura conveniente para
/// renderer, consultas e telemetria — trocá-lo é o mesmo que trocar o domínio de
/// seleção ou o workspace (ver `EditorSession::edit_mode` e
/// `AppState::set_edit_mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum EditMode {
    #[default]
    Object,
    Edit,
    TexturePaint,
}

impl EditMode {
    pub fn key(&self) -> &'static str {
        match self {
            EditMode::Object => "modes.object",
            EditMode::Edit => "modes.edit",
            EditMode::TexturePaint => "modes.paint",
        }
    }
}

pub use petunia_project::{AnnotationItem, AnnotationStroke, MeasurementItem};
pub type Measurement = MeasurementItem;

/// 1. DOMÍNIO E PERSISTÊNCIA: estado puro do projeto e histórico (independe de interface e GPU).
pub struct ProjectState {
    pub project: Project,
    pub undo: UndoStack<Project>,
    pub project_path: Option<String>,
    pub is_dirty: bool,
    pub refs: Vec<ReferenceImage>,
    pub palette: Vec<[f32; 3]>,
    pub export_selected: Vec<Uuid>,
    pub export_gltf: bool,
    pub recent_projects: crate::RecentProjects,
}

pub type DomainState = ProjectState;

impl std::ops::Deref for ProjectState {
    type Target = Project;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.project
    }
}

impl std::ops::DerefMut for ProjectState {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.project
    }
}

impl Default for ProjectState {
    fn default() -> Self {
        Self::new()
    }
}

impl ProjectState {
    pub fn new() -> Self {
        let mut p = Project::new();
        // Nunca deixar todas as revisões em 0: o fingerprint da cena recorre a
        // um hash O(V+F) por frame enquanto o projeto estiver "sem revisão".
        p.bump_changes(ProjectChanges::ALL);
        let pal = p.palette.clone();
        Self {
            project: p,
            undo: UndoStack::new(),
            project_path: None,
            is_dirty: false,
            refs: Vec::new(),
            palette: pal,
            export_selected: Vec::new(),
            export_gltf: true,
            recent_projects: crate::RecentProjects::load(),
        }
    }

    pub fn reset(&mut self) {
        let mut p = Project::new();
        p.bump_changes(ProjectChanges::ALL);
        self.palette = p.palette.clone();
        self.project = p;
        self.undo.clear();
        self.refs.clear();
        self.project_path = None;
        self.is_dirty = false;
        self.export_selected.clear();
        self.export_gltf = true;
    }

    /// Verifica se há alterações não salvas no documento (via flag direta ou histórico).
    pub fn is_dirty(&self) -> bool {
        self.is_dirty || self.undo.is_dirty()
    }

    /// Marca o documento como alterado/não salvo.
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
    }

    /// Marca o documento como sincronizado com o arquivo salvo em disco.
    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
        self.undo.mark_clean();
    }

    pub fn checkpoint(&mut self, label: &str) {
        let snap = self.project.clone();
        let bytes = snap.estimated_bytes();
        self.undo.push_sized(label, snap, bytes);
        self.is_dirty = true;
    }

    pub fn checkpoint_snapshot(&mut self, label: &str, snapshot: &Project) {
        self.undo
            .checkpoint_sized(label, snapshot, snapshot.estimated_bytes());
        self.is_dirty = true;
    }

    pub fn history_metrics(&self) -> petunia_commands::HistoryMetrics {
        self.undo.metrics()
    }

    /// Retorna os índices resolvidos válidos dos assets selecionados para exportação.
    pub fn export_selected_indices(&self) -> Vec<usize> {
        if self.export_selected.is_empty() {
            (0..self.project.assets.len()).collect()
        } else {
            self.export_selected
                .iter()
                .filter_map(|id| self.project.find(*id))
                .collect()
        }
    }

    pub fn scene_tris(&self) -> usize {
        self.project.assets.iter().map(|a| a.mesh.tri_count()).sum()
    }

    pub fn scene_verts(&self) -> usize {
        self.project.assets.iter().map(|a| a.mesh.verts.len()).sum()
    }

    pub fn is_active_locked(&self) -> bool {
        self.project.active().map(|a| a.locked).unwrap_or(false)
    }

    pub fn toggle_lock_active(&mut self) -> Option<(String, bool)> {
        let asset = self.project.active_mut()?;
        asset.locked = !asset.locked;
        Some((asset.name.clone(), asset.locked))
    }

    pub fn pick_vertex(&self, origin: Vec3, dir: Vec3) -> Option<(usize, Vec3)> {
        let obj = self.project.assets.get(self.project.active)?;
        let mut best: Option<(usize, f32, Vec3)> = None;
        for (i, v) in obj.mesh.verts.iter().enumerate() {
            let p = v.vec();
            let to = p - origin;
            let t = to.dot(dir);
            if t < 0.0 {
                continue;
            }
            let proj = origin + dir * t;
            let d = (p - proj).length();
            let tol = 0.12 * (1.0 + t * 0.15);
            if d < tol && best.map(|(_, bt, _)| t < bt).unwrap_or(true) {
                best = Some((i, t, p));
            }
        }
        best.map(|(i, _, p)| (i, p))
    }

    pub fn pick_edge(&self, origin: Vec3, dir: Vec3) -> Option<((u32, u32), Vec3)> {
        let obj = self.project.assets.get(self.project.active)?;
        let mut best: Option<((u32, u32), f32, f32, Vec3)> = None;
        for (a, b) in obj.mesh.edges_unique() {
            let pa = obj.mesh.verts[a as usize].vec();
            let pb = obj.mesh.verts[b as usize].vec();
            let mut bd = f32::MAX;
            let mut bp = pa;
            for k in 0..=8 {
                let p = pa.lerp(pb, k as f32 / 8.0);
                let t = (p - origin).dot(dir);
                if t < 0.0 {
                    continue;
                }
                let d = (p - (origin + dir * t)).length();
                if d < bd {
                    bd = d;
                    bp = p;
                }
            }
            let t = (bp - origin).dot(dir);
            let tol = 0.15 * (1.0 + t.max(0.0) * 0.15);
            if bd < tol && best.map(|(_, bt, _, _)| t < bt).unwrap_or(true) {
                best = Some(((a, b), t, bd, bp));
            }
        }
        best.map(|(e, _, _, p)| (e, p))
    }
}

/// 2. FERRAMENTAS E SESSÕES INTERATIVAS: contexto operacional de modelagem.
///
/// Como uma ferramenta paramétrica recebe o gesto de confirmação:
///
/// - `Drag`: a sessão abre no atalho e o valor vem do arrasto do ponteiro na
///   viewport; soltar confirma.
/// - `Instant`: a sessão abre no atalho e segue o movimento do mouse sem
///   botão pressionado (estilo Blender); clicar ou Enter confirma, Esc cancela.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ToolActivation {
    #[default]
    Drag,
    Instant,
}

impl ToolActivation {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Drag => "drag",
            Self::Instant => "instant",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "drag" => Some(Self::Drag),
            "instant" => Some(Self::Instant),
            _ => None,
        }
    }
}

/// Componente sob o cursor (preselection).
///
/// Vive na sessão, nunca no documento: passar o mouse não pode alterar o
/// projeto nem entrar no histórico. É o que responde "o que eu vou clicar"
/// antes do clique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HoverTarget {
    #[default]
    None,
    Vertex(u32),
    Edge(u32, u32),
    Face(usize),
    /// Índice do asset ativo sob o cursor, no domínio Object.
    Object(usize),
}

impl HoverTarget {
    pub const fn is_some(self) -> bool {
        !matches!(self, Self::None)
    }

    /// Descrição curta para a barra de status.
    pub fn label(self) -> String {
        match self {
            Self::None => String::new(),
            Self::Vertex(index) => format!("Point {index}"),
            Self::Edge(a, b) => format!("Edge {a}-{b}"),
            Self::Face(index) => format!("Face {index}"),
            Self::Object(index) => format!("Object {index}"),
        }
    }
}

pub struct ToolState {
    pub active_tool: String,
    pub gizmo_mode: crate::ModalKind,
    pub modal: Option<crate::modal::ModalOp>,
    pub pending_modal: Option<crate::modal::ModalKind>,
    pub pointer_session: Option<crate::modal::PointerSession>,
    pub cut_session: Option<crate::cutting_session::CutSession>,
    /// Operando B das operações booleanas (Fuse/Cut/Intersect): o outro ativo
    /// escolhido pelo usuário, distinto do ativo atual (A).
    pub boolean_operand: Option<uuid::Uuid>,
    /// Modificador **Keep Parts**: mantém o operando na cena depois da operação,
    /// em vez de consumi-lo.
    pub boolean_keep_parts: bool,
    /// Cleanup seguro do resultado (cap. 04): regiões coplanares viram quads,
    /// n-gons e faixas. Desligado, mantém a triangulação crua do kernel.
    pub boolean_cleanup: bool,
    /// Relatório da última operação booleana (para status e testes).
    pub boolean_report: Option<petunia_mesh::boolean_cleanup::CleanupReport>,
    pub mesh_preview: Option<crate::mesh_preview::MeshPreview>,
    pub paint_color: [f32; 3],
    pub paint_radius: f32,
    pub paint_strength: f32,
    pub paint_stroke: Option<Project>,
    pub canvas_brush: u32,
    pub paint_brush_kind: usize,
    /// Dureza do pincel (0..=1) — iniciativa Paint (P3D-057, `BrushSettings`).
    pub brush_hardness: f32,
    /// Fluxo de tinta por dab (0..=1) — Airbrush (iniciativa Paint).
    pub brush_flow: f32,
    /// Espaçamento entre dabs como fração do diâmetro (0.01..=1.0).
    pub brush_spacing: f32,
    pub paint_isolate_selection: bool,
    /// Estilo do traço: ponta, mistura, estabilizador, jitter, spray (2E).
    pub brush_style: crate::brush::BrushStyle,
    /// Origem do Clone em texels (Ctrl+clique define).
    pub clone_source: Option<[f32; 2]>,
    pub paint_stencil: Option<crate::brush::SurfaceStencil>,
    pub brush_projection: crate::brush::BrushProjectionMode,
    pub brush_lock: crate::brush::BrushLock,
    /// Face travada pelo `BrushLock` no primeiro toque do traço atual.
    pub paint_lock_face: Option<Option<usize>>,
    /// Faces elegíveis no traço atual (P3D-132). Camada externa `None` = ainda
    /// não resolvido; interna `None` = sem restrição. Vale por traço.
    pub paint_restriction: Option<Option<std::sync::Arc<crate::brush::PaintRestriction>>>,
    /// Cobertura acumulada do traço atual (opacidade × fluxo).
    pub paint_buffer: Option<crate::brush::StrokeBuffer>,
    pub fill_scope: crate::brush::FillScope,
    /// Canal de textura alvo da pintura (P3D-062). V1: só Albedo opera;
    /// demais canais ficam desabilitados na UI até V1.x.
    pub paint_channel: petunia_project::TextureChannel,
    /// Grade de pixels no canvas 2D (contextual: só com zoom suficiente).
    pub paint_pixel_grid: bool,
    /// Simetria de pintura em tempo real nos eixos X, Y e Z.
    pub paint_symmetry_x: bool,
    pub paint_symmetry_y: bool,
    pub paint_symmetry_z: bool,
    pub transform_delta: [f32; 3],
    pub transform_rotation: [f32; 3],
    pub transform_scale: f32,
    pub extrude_dist: f32,
    pub inset_factor: f32,
    pub bevel_amount: f32,
    pub bevel_segments: u32,
    pub bevel_clamp_overlap: bool,
    pub bevel_affect_vertices: bool,
    pub subdivide_cuts: u32,
    pub revolve_segments: u32,
    pub revolve_angle: f32,
    pub revolve_axis: usize,
    pub mirror_axis: usize,
    pub mirror_weld: f32,
    /// Escala ligada (uniforme) vs eixos independentes no inspector.
    pub scale_linked: bool,
    /// Fatores de escala por eixo (rascunho relativo do inspector).
    pub scale_factors: [f32; 3],
    pub merge_dist: f32,
    pub symmetrize_axis: usize,
    pub symmetrize_pos_to_neg: bool,
    pub push_dist: f32,
    pub profile: ProfileState,
    pub uv_selected: HashSet<usize>,
    pub selected_annotation: Option<uuid::Uuid>,
    pub selected_measurement: Option<uuid::Uuid>,
    pub active_measurement: Option<MeasurementItem>,
    pub active_annotation: Option<AnnotationStroke>,

    /// Modo de confirmação das ferramentas paramétricas.
    pub tool_activation: ToolActivation,
    /// Componente sob o cursor (preselection).
    pub hover: HoverTarget,
}

impl Default for ToolState {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolState {
    pub fn new() -> Self {
        Self {
            active_tool: "select".to_string(),
            gizmo_mode: crate::ModalKind::Move,
            modal: None,
            pending_modal: None,
            pointer_session: None,
            cut_session: None,
            boolean_operand: None,
            boolean_keep_parts: false,
            boolean_cleanup: true,
            boolean_report: None,
            mesh_preview: None,
            paint_color: [1.0, 0.2, 0.2],
            paint_radius: 0.8,
            paint_strength: 1.0,
            paint_stroke: None,
            canvas_brush: 13,
            paint_brush_kind: 0,
            // 0.0 = falloff quadrático legado do Soft (semântica de duas zonas
            // do motor: hardness é a fração de núcleo sólido). A UI migra para
            // valores explícitos via `BrushSettings`.
            brush_hardness: 0.0,
            brush_flow: 1.0,
            brush_spacing: 0.15,
            paint_isolate_selection: false,
            brush_style: crate::brush::BrushStyle::default(),
            clone_source: None,
            paint_stencil: None,
            brush_projection: crate::brush::BrushProjectionMode::Surface,
            brush_lock: crate::brush::BrushLock::None,
            paint_lock_face: None,
            paint_restriction: None,
            paint_buffer: None,
            tool_activation: ToolActivation::Drag,
            hover: HoverTarget::None,
            fill_scope: crate::brush::FillScope::ConnectedPixels,
            paint_channel: petunia_project::TextureChannel::Albedo,
            paint_pixel_grid: true,
            paint_symmetry_x: false,
            paint_symmetry_y: false,
            paint_symmetry_z: false,
            transform_delta: [0.0; 3],
            transform_rotation: [0.0; 3],
            transform_scale: 1.0,
            extrude_dist: 0.5,
            inset_factor: 0.3,
            bevel_amount: 0.15,
            bevel_segments: 1,
            bevel_clamp_overlap: true,
            bevel_affect_vertices: false,
            subdivide_cuts: 1,
            revolve_segments: 16,
            revolve_angle: 360.0,
            revolve_axis: 1,
            mirror_axis: 0,
            mirror_weld: 0.001,
            scale_linked: true,
            scale_factors: [1.0; 3],
            merge_dist: 0.01,
            symmetrize_axis: 0,
            symmetrize_pos_to_neg: true,
            push_dist: 0.5,
            profile: ProfileState {
                depth: 1.0,
                revolve_segments: 12,
                ..Default::default()
            },
            uv_selected: HashSet::new(),
            selected_annotation: None,
            selected_measurement: None,
            active_measurement: None,
            active_annotation: None,
        }
    }
}

/// Orientação de coordenadas para transformações (P3D-026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum TransformOrientation {
    #[default]
    Global,
    Local,
}

impl TransformOrientation {
    pub fn all() -> [Self; 2] {
        [Self::Global, Self::Local]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Global => "Global",
            Self::Local => "Local",
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Global => "orientation.global",
            Self::Local => "orientation.local",
        }
    }
}

/// Centro de pivô para transformações (P3D-027).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum PivotPoint {
    #[default]
    MedianPoint,
    BoundingBoxCenter,
    Cursor3D,
    IndividualOrigins,
}

impl PivotPoint {
    pub fn all() -> [Self; 4] {
        [
            Self::MedianPoint,
            Self::BoundingBoxCenter,
            Self::Cursor3D,
            Self::IndividualOrigins,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MedianPoint => "Median Point",
            Self::BoundingBoxCenter => "Bounding Box",
            Self::Cursor3D => "3D Cursor",
            Self::IndividualOrigins => "Individual Origins",
        }
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::MedianPoint => "pivot.median",
            Self::BoundingBoxCenter => "pivot.bbox",
            Self::Cursor3D => "pivot.cursor",
            Self::IndividualOrigins => "pivot.individual",
        }
    }
}

/// Configurações do Grid 3D e guias axiais/isométricos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GridSettings {
    pub size: f32,
    pub subdivisions: f32,
    pub opacity: f32,
    pub show_isometric_guide: bool,
    pub isometric_angle_deg: f32,
}

impl Default for GridSettings {
    fn default() -> Self {
        Self {
            size: 20.0,
            subdivisions: 1.0,
            opacity: 0.4,
            show_isometric_guide: false,
            isometric_angle_deg: 30.0,
        }
    }
}

/// 3. SESSÃO DO EDITOR: câmera, seleção, modos e viewport settings.
pub struct EditorSession {
    pub selection: Selection,
    pub workspace: Workspace,
    pub select_mode: SelectMode,
    pub selection_domain: SelectionDomain,
    pub last_component_domain: SelectionDomain,
    pub shading: Shading,
    pub textured: bool,
    pub camera: Camera,
    pub camera_frame: Option<(Camera, Camera, f32)>,
    pub cursor_3d: [f32; 3],
    pub locked_axes: [bool; 3],
    pub isolate_active: bool,
    pub isolate_prev_visibilities: Option<Vec<bool>>,
    pub snap_enabled: bool,
    pub snap_settings: crate::snap::SnapSettings,
    pub proportional_editing: bool,
    pub proportional_settings: crate::proportional::ProportionalSettings,
    pub transform_orientation: TransformOrientation,
    pub pivot_point: PivotPoint,
    pub edit_pivot: bool,
    pub show_overlays: bool,
    pub show_xray: bool,
    /// Opacidade da geometria em X-Ray (0.1..=0.9). Ajustável pelo popover.
    pub xray_opacity: f32,
    pub show_triangulation: bool,
    pub show_nav_hud: bool,
    pub show_grid: bool,
    pub grid_settings: GridSettings,
    pub show_axes: bool,
    pub show_wireframe_overlay: bool,
    pub show_cursor: bool,
    pub show_face_orientation: bool,
    pub show_uv_checker: bool,
    pub tools: ToolState,
    /// Criação de primitiva em andamento (Wave 8: cartão Last Operation).
    pub primitive_session: Option<crate::primitive_session::PrimitiveCreationSession>,
    /// Último descritor confirmado (reabertura explícita).
    pub last_primitive: Option<crate::primitive_session::PrimitiveDescriptor>,
    /// Criatura, Motion e playhead do workspace Animate (não persistido).
    pub animate: crate::animate_session::AnimateSession,
}

impl std::ops::Deref for EditorSession {
    type Target = ToolState;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.tools
    }
}

impl std::ops::DerefMut for EditorSession {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tools
    }
}

impl Default for EditorSession {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorSession {
    pub fn new() -> Self {
        Self {
            selection: Selection::default(),
            workspace: Workspace::Model,
            select_mode: SelectMode::Vertex,
            selection_domain: SelectionDomain::Object,
            last_component_domain: SelectionDomain::Vertex,
            shading: Shading::Solid,
            textured: false,
            camera: Camera::default(),
            camera_frame: None,
            cursor_3d: [0.0, 0.0, 0.0],
            locked_axes: [false; 3],
            isolate_active: false,
            isolate_prev_visibilities: None,
            snap_enabled: false,
            snap_settings: crate::snap::SnapSettings::default(),
            proportional_editing: false,
            proportional_settings: crate::proportional::ProportionalSettings::default(),
            transform_orientation: TransformOrientation::Global,
            pivot_point: PivotPoint::MedianPoint,
            edit_pivot: false,
            show_overlays: true,
            show_xray: false,
            xray_opacity: 0.42,
            show_triangulation: false,
            show_nav_hud: true,
            show_grid: true,
            grid_settings: GridSettings::default(),
            show_axes: true,
            show_wireframe_overlay: true,
            show_cursor: true,
            show_face_orientation: false,
            show_uv_checker: false,
            tools: ToolState::new(),
            primitive_session: None,
            last_primitive: None,
            animate: crate::animate_session::AnimateSession::default(),
        }
    }

    /// Visão derivada do estado de edição (P3D-015).
    ///
    /// Não existe campo `mode` em `EditorSession`: o valor é calculado a partir
    /// do domínio de seleção e do workspace ativo, eliminando o estado duplicado
    /// que competia com `SelectionDomain`.
    pub fn edit_mode(&self) -> EditMode {
        if self.workspace == Workspace::Paint {
            EditMode::TexturePaint
        } else if self.selection_domain.is_component() {
            EditMode::Edit
        } else {
            EditMode::Object
        }
    }

    pub fn is_axis_locked(&self, axis: usize, modal: Option<&crate::modal::ModalOp>) -> bool {
        if let Some(modal) = modal {
            match modal.constraint {
                crate::modal::ModalConstraint::Axis(i) => i == axis,
                crate::modal::ModalConstraint::Plane(i) => i != axis,
                crate::modal::ModalConstraint::Free => false,
            }
        } else {
            self.locked_axes.get(axis).copied().unwrap_or(false)
        }
    }

    pub fn active_axis_constraint_label(
        &self,
        modal: Option<&crate::modal::ModalOp>,
    ) -> Option<(&'static str, [u8; 3])> {
        if let Some(modal) = modal {
            match modal.constraint {
                crate::modal::ModalConstraint::Axis(0) => Some(("Eixo X", [235, 75, 75])),
                crate::modal::ModalConstraint::Axis(1) => Some(("Eixo Y", [85, 195, 100])),
                crate::modal::ModalConstraint::Axis(2) => Some(("Eixo Z", [70, 130, 245])),
                crate::modal::ModalConstraint::Plane(0) => Some(("Plano YZ", [30, 144, 180])),
                crate::modal::ModalConstraint::Plane(1) => Some(("Plano XZ", [142, 68, 173])),
                crate::modal::ModalConstraint::Plane(2) => Some(("Plano XY", [211, 84, 0])),
                _ => None,
            }
        } else {
            let [x, y, z] = self.locked_axes;
            match (x, y, z) {
                (true, false, false) => Some(("Eixo X", [235, 75, 75])),
                (false, true, false) => Some(("Eixo Y", [85, 195, 100])),
                (false, false, true) => Some(("Eixo Z", [70, 130, 245])),
                (false, true, true) => Some(("Plano YZ", [30, 144, 180])),
                (true, false, true) => Some(("Plano XZ", [142, 68, 173])),
                (true, true, false) => Some(("Plano XY", [211, 84, 0])),
                _ => None,
            }
        }
    }

    pub fn sync_selection(&mut self, project: &Project, events: &mut EventBus) -> bool {
        let mut sel = Selection::default();
        if let Some(a) = project.assets.get(project.active) {
            sel.asset = Some(a.id);
            sel.assets = self
                .selection
                .assets
                .iter()
                .copied()
                .filter(|id| project.assets.iter().any(|asset| asset.id == *id))
                .collect();
            if !sel.assets.contains(&a.id) {
                sel.assets = vec![a.id];
            }
            sel.verts = a
                .mesh
                .verts
                .iter()
                .enumerate()
                .filter(|(_, v)| v.selected)
                .map(|(i, _)| i as u32)
                .collect();
            sel.faces = a
                .mesh
                .faces
                .iter()
                .enumerate()
                .filter(|(_, f)| f.selected)
                .map(|(i, _)| i)
                .collect();
            sel.edges = a.mesh.selected_edges.iter().copied().collect();
        }
        if self.selection == sel {
            return false;
        }
        self.selection = sel.clone();
        events.emit(AppEvent::SelectionChanged(sel));
        true
    }
}

/// Largura inicial da coluna de ferramentas (logical px).
///
/// Espelha `petunia_ui::tokens::TOOLBAR_MIN_WIDTH` — o design system continua
/// dono dos tokens, e um teste na UI garante que os dois valores não divergem.
///
/// O valor é derivado (ícone + vão + seta + moldura): com o piso herdado de
/// 48px a seta do menu da família saía do paine e o menu ficava inalcançável
/// pelo mouse.
pub const TOOLBAR_DEFAULT_WIDTH: f32 = 74.0;
/// Largura inicial do dock de contexto (logical px).
///
/// Largura padrão do Inspector redimensionável (logical px).
pub const PROPERTIES_DEFAULT_WIDTH: f32 = 336.0;
/// Piso do dock de contexto: abaixo disso os campos numéricos do inspetor
/// deixam de caber lado a lado com o rótulo.
pub const PROPERTIES_MIN_WIDTH: f32 = 280.0;
/// Teto do dock de contexto: a viewport precisa manter área útil.
pub const PROPERTIES_MAX_WIDTH: f32 = 460.0;
/// Altura inicial da Asset Library do shell Slint (logical px).
pub const SHELL_ASSET_LIBRARY_DEFAULT_HEIGHT: f32 = 264.0;
/// Piso da Asset Library: cabe uma fileira de cartões com o cabeçalho.
pub const SHELL_ASSET_LIBRARY_MIN_HEIGHT: f32 = 132.0;
/// Teto da Asset Library: nunca cobre mais que isso da viewport.
pub const SHELL_ASSET_LIBRARY_MAX_HEIGHT: f32 = 520.0;
/// Limite canônico do nome de ativo (Outliner, inspetor e biblioteca).
pub const ASSET_NAME_MAX_LEN: usize = 64;

/// Recusa de renomeação de ativo.
///
/// A mensagem é o texto de status que o shell mostra: o domínio decide o
/// motivo, a UI só apresenta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum AssetRenameError {
    #[error("No active asset to rename")]
    NoActiveAsset,
    #[error("Asset name cannot be empty")]
    EmptyName,
    #[error("Asset name is limited to 64 characters")]
    NameTooLong,
}

/// 4. ESTADO DE APRESENTAÇÃO E WIDGETS UI: campos visuais, abas, pesquisas e preferências.
pub struct UiState {
    pub viewport_rect: Option<crate::viewport::LogicalRect>,
    pub viewport_pixels_per_point: f32,
    pub pending_pick: Option<(f32, f32)>,
    pub box_select_start: Option<[f32; 2]>,
    pub context_menu_pos: Option<[f32; 2]>,
    pub outliner_search: String,
    pub properties_tab: String,
    pub show_settings: bool,
    pub settings_tab: String,
    pub show_asset_library: bool,
    pub show_asset_browser: bool,
    pub show_help: bool,
    pub show_perf: bool,
    pub show_command_palette: bool,
    pub show_reference_manager: bool,
    pub command_palette_query: String,
    pub command_palette_selected_index: usize,
    pub active_theme_id: String,
    pub active_icon_pack_id: String,
    pub active_keymap_id: String,
    pub asset_thumbnail_size: f32,
    /// Preferência de entrada: inverte o arrasto vertical das ferramentas,
    /// sem alterar a navegação da câmera nem os dados do documento.
    pub invert_vertical_drag: bool,
    /// Diferenciação não-cromática de eixos para acessibilidade e daltonismo.
    pub colorblind_axes: bool,
    /// Redução de movimento para usuários com sensibilidade vestibular / labirintite.
    pub reduced_motion: bool,
    /// Exibe tag flutuante com a média de medidas na multiseleção de arestas.
    pub multiselection_measure_tag: bool,
    pub selection_rgb: [u8; 3],
    pub selection_thickness: f32,
    pub inspector_detached: bool,
    /// Fração da altura do dock direito ocupada pelo Outliner (Wave 2).
    /// Persistente na sessão; ajustada pelo divisor arrastável (0.25..=0.75).
    pub right_dock_split: f32,
    /// Colapso independente dos painéis do dock direito (Wave 2).
    pub outliner_collapsed: bool,
    pub inspector_collapsed: bool,
    /// Memória de layout por workspace (Wave 3): voltar a um workspace restaura
    /// seu split, aba do inspector e colapsos. Indexada por `workspace_index`.
    pub workspace_memory: [WorkspaceUiMemory; Workspace::COUNT],
    /// No workspace UV estreito, alterna entre editor UV e prévia 3D (Wave 3).
    pub uv_show_preview: bool,
    /// Exibe a shelf contextual sobre a viewport (Wave 5: preferência real).
    pub show_shelf: bool,
    /// Densidade global da UI (linhas, espaçamentos, hitboxes).
    pub density: UiDensity,
    /// Altura do painel Scene automática (conteúdo) vs manual (divisor).
    pub scene_split_auto: bool,
    /// Altura da Asset Library do shell Slint (logical px), dona do valor que o
    /// divisor horizontal ajusta. Distinta da shelf do shell egui legado.
    pub shell_asset_library_height: f32,
    /// Busca do Scene: aberta (campo expandido) e foco pendente (Ctrl+F).
    pub scene_search_open: bool,
    pub scene_search_focus_request: bool,
    /// Filtros do Scene (menu do funil; texto vive em `outliner_search`).
    pub scene_filter: SceneFilter,
    /// Ativo inspecionado fixado (pin); `None` = segue a seleção.
    pub inspector_pinned: Option<Uuid>,
    /// Busca de propriedades do inspector (aberta + consulta).
    pub inspector_search_open: bool,
    pub inspector_search: String,
    /// Ordem dos atalhos da toolbar esquerda (ids de ferramenta).
    /// Vazio = ordem canônica de `canonical_toolbar_order`.
    pub toolbar_order: Vec<String>,
    /// Atalhos ocultos da toolbar esquerda (ids de ferramenta).
    pub toolbar_hidden: Vec<String>,
    /// Colunas de atalhos da toolbar esquerda (1 ou 2).
    pub toolbar_columns: u8,
    /// Ações rápidas do Inspector MODEL como ids canônicos de comando.
    /// Vazio = conjunto padrão; nunca altera semântica do documento.
    pub model_quick_actions: Vec<String>,
    /// Lado do dock Outliner/Inspector.
    pub dock_side: DockSide,
    /// Disposição dos painéis do dock (empilhados ou lado a lado).
    pub dock_orientation: DockOrientation,
    /// Largura da coluna de ferramentas (Wave 5b): a geometria do shell vive no
    /// `PetuniaShellLayout` e o dono do valor persistido é este campo.
    pub left_width: f32,
    /// Largura do dock de contexto (Wave 5b). Ver [`Self::left_width`].
    pub right_width: f32,
    pub timeline_frame: i32,
    pub timeline_start: i32,
    pub timeline_end: i32,
    pub timeline_playing: bool,
    pub status: String,
    pub i18n: I18n,
    pub keybinds: Keybinds,
}

impl UiState {
    pub fn new(lang: &str) -> Self {
        Self {
            viewport_rect: None,
            viewport_pixels_per_point: 1.0,
            pending_pick: None,
            box_select_start: None,
            context_menu_pos: None,
            outliner_search: String::new(),
            properties_tab: "object".to_string(),
            show_settings: false,
            settings_tab: "appearance".to_string(),
            show_asset_library: false,
            show_asset_browser: false,
            show_help: false,
            show_perf: false,
            show_command_palette: false,
            show_reference_manager: false,
            command_palette_query: String::new(),
            command_palette_selected_index: 0,
            active_theme_id: "petunia-dark".to_string(),
            active_icon_pack_id: "petunia".to_string(),
            active_keymap_id: "petunia-default".to_string(),
            asset_thumbnail_size: 64.0,
            invert_vertical_drag: false,
            colorblind_axes: false,
            reduced_motion: false,
            multiselection_measure_tag: true,
            selection_rgb: [233, 106, 0],
            selection_thickness: 2.0,
            inspector_detached: false,
            right_dock_split: 0.42,
            outliner_collapsed: false,
            inspector_collapsed: false,
            workspace_memory: Default::default(),
            uv_show_preview: true,
            show_shelf: true,
            density: UiDensity::Comfortable,
            scene_split_auto: true,
            shell_asset_library_height: SHELL_ASSET_LIBRARY_DEFAULT_HEIGHT,
            scene_search_open: false,
            scene_search_focus_request: false,
            scene_filter: SceneFilter::default(),
            inspector_pinned: None,
            inspector_search_open: false,
            inspector_search: String::new(),
            toolbar_order: Vec::new(),
            toolbar_hidden: Vec::new(),
            toolbar_columns: 1,
            model_quick_actions: Vec::new(),
            dock_side: DockSide::Right,
            dock_orientation: DockOrientation::Stacked,
            left_width: TOOLBAR_DEFAULT_WIDTH,
            right_width: PROPERTIES_DEFAULT_WIDTH,
            timeline_frame: 1,
            timeline_start: 1,
            timeline_end: 250,
            timeline_playing: false,
            status: String::new(),
            i18n: I18n::load(lang),
            keybinds: Keybinds::load(),
        }
    }

    pub fn t(&self, key: &str) -> String {
        self.i18n.t(key)
    }

    /// Traduz um [`TextId`](petunia_config::TextId) tipado (Wave 7).
    pub fn t_id(&self, id: petunia_config::TextId) -> String {
        self.i18n.t_id(id)
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status = msg.into();
    }

    /// Largura do dock de contexto, limitada à faixa utilizável.
    ///
    /// Retorna `true` quando o valor mudou — o divisor arrastável usa isso para
    /// evitar trabalho de renderização em arrastos que já bateram no limite.
    /// Entradas não finitas são recusadas em vez de virarem um chute.
    pub fn set_right_width(&mut self, width: f32) -> bool {
        if !width.is_finite() {
            return false;
        }
        let clamped = width.clamp(PROPERTIES_MIN_WIDTH, PROPERTIES_MAX_WIDTH);
        if (clamped - self.right_width).abs() < f32::EPSILON {
            return false;
        }
        self.right_width = clamped;
        true
    }

    /// Altura da Asset Library do shell Slint, limitada à faixa utilizável.
    pub fn set_shell_asset_library_height(&mut self, height: f32) -> bool {
        if !height.is_finite() {
            return false;
        }
        let clamped = height.clamp(
            SHELL_ASSET_LIBRARY_MIN_HEIGHT,
            SHELL_ASSET_LIBRARY_MAX_HEIGHT,
        );
        if (clamped - self.shell_asset_library_height).abs() < f32::EPSILON {
            return false;
        }
        self.shell_asset_library_height = clamped;
        true
    }

    /// Catálogo controlado de ações rápidas do Inspector MODEL.
    pub const MODEL_QUICK_ACTION_CANDIDATES: [&'static str; 8] = [
        "model.subdivide",
        "model.fuse",
        "model.cut",
        "model.join",
        "model.intersect",
        "model.merge",
        "model.slice",
        "model.loop_cut",
    ];

    /// Conjunto padrão de ações rápidas, sem esconder capacidade essencial.
    pub const MODEL_QUICK_ACTION_DEFAULTS: [&'static str; 4] =
        ["model.subdivide", "model.fuse", "model.cut", "model.join"];

    /// Máximo de ações rápidas para preservar contexto em vez de volume.
    pub const MODEL_QUICK_ACTION_LIMIT: usize = 6;

    /// Ids efetivos, sanitizados contra o catálogo controlado.
    pub fn model_quick_action_ids(&self) -> Vec<String> {
        if self.model_quick_actions.is_empty() {
            return Self::MODEL_QUICK_ACTION_DEFAULTS
                .iter()
                .map(|id| id.to_string())
                .collect();
        }
        let mut ids = Vec::new();
        for id in &self.model_quick_actions {
            if Self::MODEL_QUICK_ACTION_CANDIDATES.contains(&id.as_str())
                && !ids.iter().any(|kept| kept == id)
            {
                ids.push(id.clone());
            }
            if ids.len() >= Self::MODEL_QUICK_ACTION_LIMIT {
                break;
            }
        }
        if ids.is_empty() {
            return Self::MODEL_QUICK_ACTION_DEFAULTS
                .iter()
                .map(|id| id.to_string())
                .collect();
        }
        ids
    }

    /// Fixa ou solta uma ação rápida. Retorna `true` quando mudou.
    pub fn set_model_quick_action_pinned(&mut self, id: &str, pinned: bool) -> bool {
        if !Self::MODEL_QUICK_ACTION_CANDIDATES.contains(&id) {
            return false;
        }
        let mut ids = self.model_quick_action_ids();
        if pinned {
            if ids.iter().any(|kept| kept == id) {
                return false;
            }
            if ids.len() >= Self::MODEL_QUICK_ACTION_LIMIT {
                return false;
            }
            ids.push(id.to_string());
        } else {
            let before = ids.len();
            ids.retain(|kept| kept != id);
            if ids.len() == before {
                return false;
            }
        }
        self.model_quick_actions = ids;
        true
    }

    /// Restaura o conjunto padrão de ações rápidas.
    pub fn reset_model_quick_actions(&mut self) -> bool {
        if self.model_quick_actions.is_empty() {
            return false;
        }
        self.model_quick_actions.clear();
        true
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new("en")
    }
}

/// Densidade da interface (preferência global de UI).
///
/// Consome os tokens de densidade da UI (altura de linha, espaçamento,
/// hitbox); não cria segundo sistema de settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum UiDensity {
    Compact,
    #[default]
    Comfortable,
    Spacious,
}

impl UiDensity {
    pub fn key(self) -> &'static str {
        match self {
            UiDensity::Compact => "density.compact",
            UiDensity::Comfortable => "density.comfortable",
            UiDensity::Spacious => "density.spacious",
        }
    }

    pub fn all() -> [UiDensity; 3] {
        [
            UiDensity::Compact,
            UiDensity::Comfortable,
            UiDensity::Spacious,
        ]
    }
}

/// Filtro de estado de objeto no painel Scene (extensível por tipo futuro).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SceneObjectState {
    #[default]
    All,
    VisibleOnly,
    UnlockedOnly,
}

impl SceneObjectState {
    pub fn key(self) -> &'static str {
        match self {
            SceneObjectState::All => "scene_filter.all",
            SceneObjectState::VisibleOnly => "scene_filter.visible",
            SceneObjectState::UnlockedOnly => "scene_filter.unlocked",
        }
    }

    pub fn all() -> [SceneObjectState; 3] {
        [
            SceneObjectState::All,
            SceneObjectState::VisibleOnly,
            SceneObjectState::UnlockedOnly,
        ]
    }
}

/// Filtros do painel Scene (menu do funil; busca textual vive em
/// `UiState::outliner_search`). Tudo default-visível.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneFilter {
    pub show_collections: bool,
    pub show_annotations: bool,
    pub show_measurements: bool,
    pub show_refs: bool,
    pub object_state: SceneObjectState,
}

impl Default for SceneFilter {
    fn default() -> Self {
        Self {
            show_collections: true,
            show_annotations: true,
            show_measurements: true,
            show_refs: true,
            object_state: SceneObjectState::All,
        }
    }
}
/// Motivo do dirty para rastreio de loops de repaint contínuo (Wave 1 — §16.6).
/// Usar `mark_dirty_reason` em vez de `mark_dirty` quando o motivo é conhecido;
/// builds de desenvolvimento registram via `tracing` para identificar poluição.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtyReason {
    CameraOrbit,
    CameraPan,
    CameraZoom,
    Selection,
    GeometryEdit,
    CurveEdit,
    MaterialEdit,
    TransformModal,
    TimelinePlayback,
    UiInteraction,
    FileEvent,
    Unknown,
}

/// Lado do dock Outliner/Inspector no shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockSide {
    Left,
    #[default]
    Right,
}

/// Disposição dos painéis Outliner/Inspector dentro do dock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DockOrientation {
    /// Empilhados na vertical com divisor arrastável (padrão).
    #[default]
    Stacked,
    /// Lado a lado na horizontal com divisor arrastável.
    SideBySide,
}

/// Memória de layout do dock por workspace (Wave 3 — §6.5).
#[derive(Debug, Clone)]
pub struct WorkspaceUiMemory {
    pub dock_split: f32,
    pub inspector_tab: String,
    pub outliner_collapsed: bool,
    pub inspector_collapsed: bool,
    /// Larguras do shell por workspace (Wave 5b, §47): voltar a um workspace
    /// restaura a composição, não só a divisão do dock.
    pub left_width: f32,
    pub right_width: f32,
    /// Altura da Asset Library do shell Slint por workspace.
    pub shell_asset_library_height: f32,
}

impl Default for WorkspaceUiMemory {
    fn default() -> Self {
        Self {
            dock_split: 0.42,
            inspector_tab: "object".to_string(),
            outliner_collapsed: false,
            inspector_collapsed: false,
            left_width: TOOLBAR_DEFAULT_WIDTH,
            right_width: PROPERTIES_DEFAULT_WIDTH,
            shell_asset_library_height: SHELL_ASSET_LIBRARY_DEFAULT_HEIGHT,
        }
    }
}

/// Índice da memória de layout para um workspace (enum sem payload: `0..Workspace::COUNT`).
pub fn workspace_index(workspace: Workspace) -> usize {
    workspace as usize
}

/// 5. RECURSOS E TELEMETRIA DA GPU: contadores de renderização e sinalizadores de buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureDirtyRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl TextureDirtyRect {
    pub fn clipped(self, canvas_width: u32, canvas_height: u32) -> Option<Self> {
        let x = self.x.min(canvas_width);
        let y = self.y.min(canvas_height);
        let right = self.x.saturating_add(self.width).min(canvas_width);
        let bottom = self.y.saturating_add(self.height).min(canvas_height);
        (x < right && y < bottom).then_some(Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }

    pub fn byte_len(self) -> u64 {
        u64::from(self.width) * u64::from(self.height) * 4
    }
}

/// Invalidação transitória de textura; nunca é serializada no documento.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextureUpdate {
    pub asset_id: Uuid,
    pub canvas_width: u32,
    pub canvas_height: u32,
    /// `None` exige upload integral; `Some` contém regiões parciais.
    pub regions: Option<Vec<TextureDirtyRect>>,
}

impl TextureUpdate {
    pub fn is_full(&self) -> bool {
        self.regions.is_none()
    }
}

fn normalize_texture_regions(
    regions: impl IntoIterator<Item = TextureDirtyRect>,
    canvas_width: u32,
    canvas_height: u32,
) -> Vec<TextureDirtyRect> {
    let mut horizontal: Vec<_> = regions
        .into_iter()
        .filter_map(|region| region.clipped(canvas_width, canvas_height))
        .collect();
    horizontal.sort_unstable_by_key(|region| (region.y, region.height, region.x, region.width));
    let mut merged: Vec<TextureDirtyRect> = Vec::with_capacity(horizontal.len());
    for region in horizontal {
        if let Some(previous) = merged.last_mut()
            && previous.y == region.y
            && previous.height == region.height
            && region.x <= previous.x.saturating_add(previous.width)
        {
            let right = previous
                .x
                .saturating_add(previous.width)
                .max(region.x.saturating_add(region.width));
            previous.width = right - previous.x;
        } else {
            merged.push(region);
        }
    }

    merged.sort_unstable_by_key(|region| (region.x, region.width, region.y, region.height));
    let mut vertical: Vec<TextureDirtyRect> = Vec::with_capacity(merged.len());
    for region in merged {
        if let Some(previous) = vertical.last_mut()
            && previous.x == region.x
            && previous.width == region.width
            && region.y <= previous.y.saturating_add(previous.height)
        {
            let bottom = previous
                .y
                .saturating_add(previous.height)
                .max(region.y.saturating_add(region.height));
            previous.height = bottom - previous.y;
        } else {
            vertical.push(region);
        }
    }
    vertical
}

#[derive(Debug, Clone)]
pub struct RenderResources {
    pub dirty: bool,
    pub canvas_dirty: bool,
    texture_updates: Vec<TextureUpdate>,
    pub backend_name: String,
    pub stats: RenderStats,
    /// Último motivo registrado (diagnóstico dev; não afeta decisão de redraw).
    pub last_dirty_reason: Option<DirtyReason>,
}

impl Default for RenderResources {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderResources {
    pub fn new() -> Self {
        Self {
            dirty: true,
            canvas_dirty: true,
            texture_updates: Vec::new(),
            backend_name: String::new(),
            stats: RenderStats::default(),
            last_dirty_reason: None,
        }
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn mark_dirty_reason(&mut self, reason: DirtyReason) {
        self.dirty = true;
        self.last_dirty_reason = Some(reason);
        tracing::trace!(?reason, "mark_dirty");
    }

    pub fn consume_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    pub fn mark_texture_full(&mut self, asset_id: Uuid, canvas_width: u32, canvas_height: u32) {
        if let Some(update) = self
            .texture_updates
            .iter_mut()
            .find(|update| update.asset_id == asset_id)
        {
            update.canvas_width = canvas_width;
            update.canvas_height = canvas_height;
            update.regions = None;
            return;
        }
        self.texture_updates.push(TextureUpdate {
            asset_id,
            canvas_width,
            canvas_height,
            regions: None,
        });
    }

    pub fn mark_texture_regions(
        &mut self,
        asset_id: Uuid,
        canvas_width: u32,
        canvas_height: u32,
        regions: impl IntoIterator<Item = TextureDirtyRect>,
    ) {
        let regions = normalize_texture_regions(regions, canvas_width, canvas_height);
        if regions.is_empty() {
            return;
        }
        if let Some(update) = self
            .texture_updates
            .iter_mut()
            .find(|update| update.asset_id == asset_id)
        {
            if update.canvas_width != canvas_width || update.canvas_height != canvas_height {
                update.canvas_width = canvas_width;
                update.canvas_height = canvas_height;
                update.regions = None;
                return;
            }
            if let Some(existing) = update.regions.as_mut() {
                existing.extend(regions);
                *existing = normalize_texture_regions(
                    std::mem::take(existing),
                    canvas_width,
                    canvas_height,
                );
                if existing.as_slice()
                    == [TextureDirtyRect {
                        x: 0,
                        y: 0,
                        width: canvas_width,
                        height: canvas_height,
                    }]
                {
                    update.regions = None;
                }
            }
            return;
        }
        let full_canvas = regions.as_slice()
            == [TextureDirtyRect {
                x: 0,
                y: 0,
                width: canvas_width,
                height: canvas_height,
            }];
        self.texture_updates.push(TextureUpdate {
            asset_id,
            canvas_width,
            canvas_height,
            regions: (!full_canvas).then_some(regions),
        });
    }

    pub fn take_texture_updates(&mut self) -> Vec<TextureUpdate> {
        std::mem::take(&mut self.texture_updates)
    }
}

/// Coordenador global da aplicação agregando os subsistemas segregados.
pub struct AppState {
    pub project: ProjectState,
    pub session: EditorSession,
    pub ui: UiState,
    pub render: RenderResources,
    pub events: EventBus,
    pub commands: crate::command::CommandDispatcher,
}

impl std::ops::Deref for AppState {
    type Target = EditorSession;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl std::ops::DerefMut for AppState {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}

impl AppState {
    pub fn new(lang: &str) -> Self {
        Self {
            project: ProjectState::new(),
            session: EditorSession::new(),
            ui: UiState::new(lang),
            render: RenderResources::new(),
            events: EventBus::new(),
            commands: crate::command::CommandDispatcher::canonical(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new("en")
    }
}

impl AppState {
    pub fn scene_tris(&self) -> usize {
        self.project.scene_tris()
    }

    pub fn scene_verts(&self) -> usize {
        self.project.scene_verts()
    }

    pub fn t(&self, key: &str) -> String {
        self.ui.t(key)
    }

    pub fn t_id(&self, id: petunia_config::TextId) -> String {
        self.ui.t_id(id)
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.ui.set_status(msg);
    }

    /// Marca para render-on-demand (§33).
    pub fn mark_dirty(&mut self) {
        self.render.mark_dirty();
    }

    /// Sessão de criação válida? Ativo + asset presente + checkpoint no topo.
    ///
    /// Qualquer outra operação com checkpoint no meio invalida (a primitiva vira
    /// malha comum e o cartão some): Esc posterior não remove nada.
    pub fn primitive_session_valid(&self) -> bool {
        match &self.session.primitive_session {
            None => false,
            Some(session) => {
                self.project.assets.iter().any(|a| a.id == session.asset_id)
                    && self.project.undo.undo_label() == Some(session.undo_label.as_str())
            }
        }
    }

    /// Inicia uma criação com transação única de undo (Wave 8 — §10).
    ///
    /// Uma sessão anterior válida é confirmada silenciosamente (malha mantida).
    pub fn begin_primitive(
        &mut self,
        kind: crate::command::PrimitiveKind,
        name: Option<String>,
    ) -> bool {
        use crate::primitive_session::{PrimitiveDescriptor, PrimitiveDescriptorExt};
        self.finalize_primitive_session();
        let descriptor = PrimitiveDescriptor::default_for(kind);
        self.begin_primitive_with(descriptor, name)
    }

    /// Inicia uma criação a partir de um descritor (reabertura Last Operation).
    pub fn begin_primitive_with(
        &mut self,
        descriptor: crate::primitive_session::PrimitiveDescriptor,
        name: Option<String>,
    ) -> bool {
        use crate::primitive_session::PrimitiveDescriptorExt;
        self.set_selection_domain(SelectionDomain::Object);
        self.finalize_primitive_session();
        let original_selection = self.session.selection.clone();
        let cursor_offset = self.session.cursor_3d;
        let asset_name = name.unwrap_or_else(|| descriptor.kind().default_name().to_string());
        let undo_label = format!("add primitive: {asset_name}");
        self.checkpoint(&undo_label);
        let mut mesh = descriptor.build();
        for v in &mut mesh.verts {
            v.pos[0] += cursor_offset[0];
            v.pos[1] += cursor_offset[1];
            v.pos[2] += cursor_offset[2];
        }
        self.project.add(&asset_name, mesh);
        let Some(asset) = self.project.assets.last_mut() else {
            return false;
        };
        asset.parametric = Some(descriptor);
        asset.origin = Some(cursor_offset);
        let asset_id = asset.id;
        self.session.primitive_session = Some(crate::primitive_session::PrimitiveCreationSession {
            asset_id,
            descriptor,
            cursor_offset,
            undo_label,
            original_selection,
        });
        self.sync_selection();
        self.emit_mesh_changed();
        self.set_status(format!("Added {asset_name}"));
        true
    }

    /// Regenera a malha a partir de novos parâmetros (sem checkpoint).
    pub fn update_primitive(
        &mut self,
        descriptor: crate::primitive_session::PrimitiveDescriptor,
    ) -> bool {
        if !self.primitive_session_valid() {
            self.session.primitive_session = None;
            return false;
        }
        let (asset_id, cursor_offset) = match &self.session.primitive_session {
            Some(session) => (session.asset_id, session.cursor_offset),
            None => return false,
        };
        let Some(asset) = self.project.assets.iter_mut().find(|a| a.id == asset_id) else {
            self.session.primitive_session = None;
            return false;
        };
        let mut mesh = descriptor.build();
        for v in &mut mesh.verts {
            v.pos[0] += cursor_offset[0];
            v.pos[1] += cursor_offset[1];
            v.pos[2] += cursor_offset[2];
        }
        asset.mesh = mesh;
        asset.parametric = Some(descriptor);
        if let Some(session) = self.session.primitive_session.as_mut() {
            session.descriptor = descriptor;
        }
        self.emit_mesh_changed();
        true
    }

    /// O asset ativo é uma primitiva paramétrica viva?
    pub fn active_is_parametric(&self) -> bool {
        self.project.active().is_some_and(|a| a.is_parametric())
    }

    /// Retorna o descritor paramétrico do asset ativo, se houver.
    pub fn active_primitive_descriptor(
        &self,
    ) -> Option<crate::primitive_session::PrimitiveDescriptor> {
        self.project.active().and_then(|a| a.parametric)
    }

    /// Congela transparentemente a primitiva paramétrica do asset ativo em malha estática.
    pub fn freeze_active_primitive(&mut self) -> bool {
        if self.freeze_active_primitive_for_command() {
            self.emit_mesh_changed();
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub(crate) fn freeze_active_primitive_for_command(&mut self) -> bool {
        let Some(asset) = self.project.active_mut() else {
            return false;
        };
        if asset.freeze_parametric() {
            if let Some(session) = self.session.primitive_session.take() {
                self.session.last_primitive = Some(session.descriptor);
            }
            true
        } else {
            false
        }
    }

    /// Atualiza os parâmetros da primitiva do asset ativo e regenera a malha.
    ///
    /// Se a sessão de criação estiver ativa, atualiza diretamente sem novo checkpoint.
    /// Caso contrário, se `create_checkpoint` for verdadeiro, registra checkpoint no undo.
    pub fn update_active_primitive(
        &mut self,
        descriptor: crate::primitive_session::PrimitiveDescriptor,
        create_checkpoint: bool,
    ) -> bool {
        if self.primitive_session_valid() {
            return self.update_primitive(descriptor);
        }

        let Some(asset) = self.project.active_mut() else {
            return false;
        };
        if !asset.is_parametric() {
            return false;
        }

        let origin = asset.origin.unwrap_or_else(|| {
            let c = asset.mesh.center();
            [c.x, c.y, c.z]
        });

        if create_checkpoint {
            let label = format!("edit primitive: {}", descriptor.kind_name());
            self.checkpoint(&label);
        }

        let mut mesh = descriptor.build();
        for v in &mut mesh.verts {
            v.pos[0] += origin[0];
            v.pos[1] += origin[1];
            v.pos[2] += origin[2];
        }

        let Some(asset) = self.project.active_mut() else {
            return false;
        };
        asset.mesh = mesh;
        asset.parametric = Some(descriptor);
        self.session.last_primitive = Some(descriptor);
        self.emit_mesh_changed();
        self.mark_dirty();
        true
    }

    /// Confirma a criação (uma transação no undo; guarda para reabrir).
    pub fn confirm_primitive(&mut self) -> bool {
        if !self.primitive_session_valid() {
            self.session.primitive_session = None;
            return false;
        }
        if let Some(session) = self.session.primitive_session.take() {
            self.session.last_primitive = Some(session.descriptor);
        }
        self.mark_dirty();
        true
    }

    /// Cancela a criação: desfaz até o ponto de inserção e restaura a seleção.
    pub fn cancel_primitive(&mut self) -> bool {
        let Some(session) = self.session.primitive_session.take() else {
            return false;
        };
        if !(self.project.assets.iter().any(|a| a.id == session.asset_id)
            && self.project.undo.undo_label() == Some(session.undo_label.as_str()))
        {
            return false;
        }
        // Desempilha até o checkpoint de inserção (normalmente um pop; o laço
        // tolera modais/strokes que o primeiro undo() só finaliza).
        for _ in 0..4 {
            if self.project.undo.undo_label() != Some(session.undo_label.as_str()) {
                break;
            }
            if !self.undo() {
                break;
            }
        }
        self.restore_selection_snapshot(session.original_selection);
        self.mark_dirty();
        true
    }

    /// Reabre a última criação confirmada como nova sessão (Last Operation).
    pub fn reopen_last_primitive(&mut self) -> bool {
        if self.session.primitive_session.is_some() {
            return false;
        }
        let Some(descriptor) = self.session.last_primitive else {
            return false;
        };
        self.begin_primitive_with(descriptor, None)
    }

    /// Finaliza silenciosamente (malha mantida, vira edição comum).
    pub fn finalize_primitive_session(&mut self) {
        if let Some(session) = self.session.primitive_session.take()
            && self.project.assets.iter().any(|a| a.id == session.asset_id)
        {
            self.session.last_primitive = Some(session.descriptor);
        }
    }

    /// Reaplica um snapshot de seleção com guarda de limites.
    fn restore_selection_snapshot(&mut self, snapshot: Selection) {
        for asset in &mut self.project.assets {
            for v in &mut asset.mesh.verts {
                v.selected = false;
            }
            for f in &mut asset.mesh.faces {
                f.selected = false;
            }
            asset.mesh.selected_edges.clear();
        }
        if let Some(id) = snapshot.asset
            && let Some(idx) = self.project.assets.iter().position(|a| a.id == id)
        {
            self.project.active = idx;
            if let Some(asset) = self.project.assets.get_mut(idx) {
                for v in snapshot.verts {
                    if let Some(vert) = asset.mesh.verts.get_mut(v as usize) {
                        vert.selected = true;
                    }
                }
                for f in snapshot.faces {
                    if let Some(face) = asset.mesh.faces.get_mut(f) {
                        face.selected = true;
                    }
                }
                asset.mesh.selected_edges = snapshot
                    .edges
                    .into_iter()
                    .filter(|(a, b)| {
                        (*a as usize) < asset.mesh.verts.len()
                            && (*b as usize) < asset.mesh.verts.len()
                            && a != b
                    })
                    .map(|(a, b)| (a.min(b), a.max(b)))
                    .collect();
            }
        }
        self.sync_selection();
    }

    /// Troca de workspace com memória de layout por workspace (Wave 3 — §6.5).
    ///
    /// Dono único da transição (§3.2): salva split/aba/colapsos do workspace
    /// atual e restaura os do destino. Projeto, seleção e undo são
    /// compartilhados — só a composição do shell muda.
    pub fn switch_workspace(&mut self, next: Workspace) {
        let prev = self.session.workspace;
        if prev == next {
            return;
        }
        // Troca de workspace finaliza a criação (vira malha comum).
        self.finalize_primitive_session();
        self.ui.workspace_memory[workspace_index(prev)] = WorkspaceUiMemory {
            dock_split: self.ui.right_dock_split,
            inspector_tab: self.ui.properties_tab.clone(),
            outliner_collapsed: self.ui.outliner_collapsed,
            inspector_collapsed: self.ui.inspector_collapsed,
            left_width: self.ui.left_width,
            right_width: self.ui.right_width,
            shell_asset_library_height: self.ui.shell_asset_library_height,
        };
        let restored = self.ui.workspace_memory[workspace_index(next)].clone();
        self.ui.right_dock_split = restored.dock_split;
        self.ui.properties_tab = restored.inspector_tab;
        self.ui.outliner_collapsed = restored.outliner_collapsed;
        self.ui.inspector_collapsed = restored.inspector_collapsed;
        self.ui.left_width = restored.left_width;
        self.ui.right_width = restored.right_width;
        self.ui.shell_asset_library_height = restored.shell_asset_library_height;
        self.session.workspace = next;
        #[cfg(feature = "animation-workspace")]
        if next == Workspace::Animate {
            self.animate_resolve();
        }
        self.mark_dirty();
    }

    /// Retorna se o eixo especificado (0 = X, 1 = Y, 2 = Z) está travado na atividade de edição atual.
    pub fn is_axis_locked(&self, axis: usize) -> bool {
        self.session
            .is_axis_locked(axis, self.session.tools.modal.as_ref())
    }

    /// Retorna rótulo amigável e cor RGB do eixo ou plano travado atualmente, se houver.
    pub fn active_axis_constraint_label(&self) -> Option<(&'static str, [u8; 3])> {
        self.session
            .active_axis_constraint_label(self.session.tools.modal.as_ref())
    }

    /// Alterna o travamento de um eixo específico (0 = X, 1 = Y, 2 = Z).
    pub fn toggle_axis_lock(&mut self, axis: usize) {
        if axis > 2 {
            return;
        }
        if self.session.tools.modal.is_some() {
            let current_locked = self.is_axis_locked(axis);
            let next_constraint = if current_locked {
                crate::modal::ModalConstraint::Free
            } else {
                crate::modal::ModalConstraint::Axis(axis)
            };
            if let Err(e) = self.set_modal_constraint(next_constraint) {
                self.set_status(e.to_string());
            }
        } else {
            self.session.locked_axes[axis] = !self.session.locked_axes[axis];
            self.mark_dirty();
        }
    }

    pub fn consume_dirty(&mut self) -> bool {
        self.render.consume_dirty()
    }

    /// Retorna se o documento/projeto possui modificações não salvas (P3D-001 §Dirty State).
    pub fn is_document_dirty(&self) -> bool {
        self.project.is_dirty()
    }

    /// Marca o documento como alterado/não salvo.
    pub fn mark_document_dirty(&mut self) {
        self.project.mark_dirty();
    }

    /// Marca o documento como limpo e sincronizado com o arquivo salvo em disco.
    pub fn mark_document_clean(&mut self) {
        self.project.mark_clean();
    }

    /// Despacha um comando através do CommandDispatcher com auto-checkpoint e propagação de eventos.
    pub fn dispatch(
        &mut self,
        cmd: &dyn crate::command::Command,
    ) -> Result<(), crate::command::CommandError> {
        crate::command::CommandDispatcher::dispatch(self, cmd)
    }

    /// Checkpoint de undo ANTES de mutar o projeto + evento.
    pub fn checkpoint(&mut self, label: &str) {
        self.project.project.history_selection = self.session.selection.assets.clone();
        self.project.checkpoint(label);
        self.mark_dirty();
    }

    pub fn undo(&mut self) -> bool {
        if self.session.tools.mesh_preview.is_some() {
            self.finish_mesh_preview(true);
            return true;
        }
        if self.session.tools.paint_stroke.is_some() {
            self.finish_paint_stroke(true);
            return true;
        }
        if self.cancel_modal() {
            return true;
        }
        let cur = self.project.project.clone();
        let bytes = cur.estimated_bytes();
        let revision_clock = cur.revision_clock();
        if let Some(mut prev) = self.project.undo.undo_sized(cur, bytes) {
            prev.rebase_revisions_after_restore(revision_clock);
            self.session.selection.assets = prev.history_selection.clone();
            self.project.palette = prev.palette.clone();
            self.project.project = prev;
            self.project.is_dirty = !self.project.undo.is_clean();
            self.sync_selection();
            self.session.tools.uv_selected.clear();
            self.emit_project_changed(ProjectChanges::ALL);
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if self.session.tools.mesh_preview.is_some() {
            self.finish_mesh_preview(true);
            return true;
        }
        if self.session.tools.paint_stroke.is_some() {
            self.finish_paint_stroke(true);
            return true;
        }
        if self.cancel_modal() {
            return true;
        }
        let cur = self.project.project.clone();
        let bytes = cur.estimated_bytes();
        let revision_clock = cur.revision_clock();
        if let Some(mut next) = self.project.undo.redo_sized(cur, bytes) {
            next.rebase_revisions_after_restore(revision_clock);
            self.session.selection.assets = next.history_selection.clone();
            self.project.palette = next.palette.clone();
            self.project.project = next;
            self.project.is_dirty = !self.project.undo.is_clean();
            self.sync_selection();
            self.session.tools.uv_selected.clear();
            self.emit_project_changed(ProjectChanges::ALL);
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub fn emit_mesh_changed(&mut self) {
        self.emit_project_changed(ProjectChanges::GEOMETRY);
    }

    pub fn emit_positions_changed(&mut self) {
        self.emit_project_changed(ProjectChanges::POSITIONS);
    }

    pub fn emit_normals_changed(&mut self) {
        self.emit_project_changed(ProjectChanges::NORMALS);
    }

    pub fn emit_uv_changed(&mut self) {
        self.emit_project_changed(ProjectChanges::UVS);
    }

    pub fn emit_color_changed(&mut self) {
        self.emit_project_changed(ProjectChanges::COLORS);
    }

    pub fn emit_material_changed(&mut self) {
        self.emit_project_changed(ProjectChanges::MATERIALS);
    }

    pub fn emit_texture_changed(&mut self) {
        self.emit_project_changed(ProjectChanges::TEXTURES);
    }

    pub fn emit_texture_changed_regions(&mut self, regions: Vec<TextureDirtyRect>) {
        if regions.is_empty() {
            return;
        }
        self.project.project.bump_changes(ProjectChanges::TEXTURES);
        self.notify_project_changed_with_texture_regions(ProjectChanges::TEXTURES, Some(regions));
    }

    pub fn emit_transform_changed(&mut self) {
        self.emit_project_changed(ProjectChanges::TRANSFORMS);
    }

    pub fn emit_project_changed(&mut self, changes: ProjectChanges) {
        if changes.is_empty() {
            return;
        }
        self.project.project.bump_changes(changes);
        self.notify_project_changed(changes);
    }

    fn notify_project_changed(&mut self, changes: ProjectChanges) {
        self.notify_project_changed_with_texture_regions(changes, None);
    }

    fn notify_project_changed_with_texture_regions(
        &mut self,
        changes: ProjectChanges,
        texture_regions: Option<Vec<TextureDirtyRect>>,
    ) {
        let id = self.project.assets.get(self.project.active).map(|a| a.id);
        if changes.intersects(
            ProjectChanges::GEOMETRY
                | ProjectChanges::UVS
                | ProjectChanges::TRANSFORMS
                | ProjectChanges::MATERIALS,
        ) && let Some(asset_id) = id
        {
            self.events.emit(AppEvent::MeshChanged { asset_id });
        }
        if changes.contains(ProjectChanges::TEXTURES)
            && let Some(asset_id) = id
        {
            self.events.emit(AppEvent::TextureChanged { asset_id });
        }
        if changes.contains(ProjectChanges::TEXTURES) {
            if let Some(regions) = texture_regions {
                let active_asset = self.project.assets.get(self.project.active);
                let active_id = active_asset.map(|asset| asset.id);
                let active_material_id = active_asset.and_then(|asset| asset.material_id);
                let textures: Vec<_> = self
                    .project
                    .assets
                    .iter()
                    .filter(|asset| {
                        Some(asset.id) == active_id
                            || (active_material_id.is_some()
                                && asset.material_id == active_material_id
                                && asset.texture.is_none())
                    })
                    .filter_map(|asset| {
                        asset
                            .texture
                            .as_ref()
                            .or_else(|| {
                                asset
                                    .material(&self.project.project)
                                    .and_then(|material| material.albedo_texture.as_ref())
                            })
                            .map(|canvas| (asset.id, canvas.w, canvas.h))
                    })
                    .collect();
                for (asset_id, width, height) in textures {
                    self.render.mark_texture_regions(
                        asset_id,
                        width,
                        height,
                        regions.iter().copied(),
                    );
                }
            } else {
                let textures: Vec<_> = self
                    .project
                    .assets
                    .iter()
                    .filter_map(|asset| {
                        asset
                            .texture
                            .as_ref()
                            .or_else(|| {
                                asset
                                    .material(&self.project.project)
                                    .and_then(|material| material.albedo_texture.as_ref())
                            })
                            .map(|canvas| (asset.id, canvas.w, canvas.h))
                    })
                    .collect();
                for (asset_id, width, height) in textures {
                    self.render.mark_texture_full(asset_id, width, height);
                }
            }
        }
        let reason = if changes == ProjectChanges::SPLINES
            || changes == ProjectChanges::PROCEDURAL
            || changes == (ProjectChanges::SPLINES | ProjectChanges::PROCEDURAL)
        {
            DirtyReason::CurveEdit
        } else if changes.intersects(ProjectChanges::TEXTURES | ProjectChanges::MATERIALS) {
            DirtyReason::MaterialEdit
        } else if changes.contains(ProjectChanges::TRANSFORMS) {
            DirtyReason::TransformModal
        } else {
            DirtyReason::GeometryEdit
        };
        self.render.mark_dirty_reason(reason);
    }

    pub fn sync_selection(&mut self) -> bool {
        let changed = self
            .session
            .sync_selection(&self.project.project, &mut self.events);
        self.project.project.history_selection = self.session.selection.assets.clone();
        if changed {
            self.project.project.bump_selection();
            self.render.mark_dirty_reason(DirtyReason::Selection);
        }
        changed
    }

    /// Shared object selection authority for Parts, viewport and automation.
    pub fn select_object(&mut self, index: Option<usize>, extend: bool) {
        if self.modal.is_some() || self.mesh_preview.is_some() || self.paint_stroke.is_some() {
            return;
        }
        let Some(index) = index.filter(|&i| {
            self.project
                .assets
                .get(i)
                .is_some_and(|a| a.visible && !a.locked)
        }) else {
            if !extend {
                self.project.active = usize::MAX;
                self.session.selection = Selection::default();
                self.sync_selection();
            }
            return;
        };
        let id = self.project.assets[index].id;
        if !extend {
            self.session.selection.assets.clear();
        }
        if extend && self.session.selection.assets.contains(&id) {
            self.session
                .selection
                .assets
                .retain(|selected| *selected != id);
            self.project.active = self
                .session
                .selection
                .assets
                .last()
                .and_then(|last| self.project.assets.iter().position(|a| a.id == *last))
                .unwrap_or(usize::MAX);
        } else {
            self.session.selection.assets.push(id);
            self.project.active = index;
        }
        self.session.selection.asset = self.project.active().map(|a| a.id);
        self.sync_selection();
    }

    /// Retorna o domínio de seleção e interação ativo (P3D-015).
    pub fn selection_domain(&self) -> SelectionDomain {
        self.session.selection_domain
    }

    /// Define o estado de edição por via semântica (P3D-015).
    ///
    /// `EditMode` é derivado, então escrever nele significa:
    /// `Object` → domínio `Object`; `Edit` → último domínio de componente;
    /// `TexturePaint` → workspace PAINT. Não há estado paralelo a manter.
    pub fn set_edit_mode(&mut self, mode: EditMode) {
        match mode {
            EditMode::Object => self.set_selection_domain(SelectionDomain::Object),
            EditMode::Edit => {
                let target = self.session.last_component_domain;
                self.set_selection_domain(if target.is_component() {
                    target
                } else {
                    SelectionDomain::Vertex
                });
            }
            EditMode::TexturePaint => self.switch_workspace(Workspace::Paint),
        }
    }

    /// Define o domínio de seleção (Object, Point, Edge, Face) e sincroniza sub-estados.
    ///
    /// `EditMode` é derivado deste estado — não existe escrita separada de modo.
    pub fn set_selection_domain(&mut self, domain: SelectionDomain) {
        let previous = self.session.selection_domain;
        let source = if previous == SelectionDomain::Object {
            self.session.last_component_domain
        } else {
            previous
        };
        if domain.is_component()
            && source != domain
            && let Some(mesh) = self.project.active_mesh_mut()
        {
            match source {
                SelectionDomain::Vertex => match domain {
                    SelectionDomain::Edge => {
                        mesh.selected_edges = mesh
                            .edges_unique()
                            .into_iter()
                            .filter(|&(a, b)| {
                                mesh.verts[a as usize].selected && mesh.verts[b as usize].selected
                            })
                            .collect();
                        mesh.faces.iter_mut().for_each(|face| face.selected = false);
                        for vertex in &mut mesh.verts {
                            vertex.selected = false;
                        }
                        let selected_edges: Vec<_> = mesh.selected_edges.iter().copied().collect();
                        for (a, b) in selected_edges {
                            mesh.verts[a as usize].selected = true;
                            mesh.verts[b as usize].selected = true;
                        }
                    }
                    SelectionDomain::Face => {
                        mesh.sync_face_selection_from_verts();
                        mesh.selected_edges.clear();
                        mesh.sync_vert_selection_from_faces();
                    }
                    _ => {}
                },
                SelectionDomain::Edge => match domain {
                    SelectionDomain::Vertex => {
                        mesh.faces.iter_mut().for_each(|face| face.selected = false);
                        mesh.selected_edges.clear();
                    }
                    SelectionDomain::Face => {
                        for face in &mut mesh.faces {
                            face.selected = !face.verts.is_empty()
                                && (0..face.verts.len()).all(|i| {
                                    let a = face.verts[i];
                                    let b = face.verts[(i + 1) % face.verts.len()];
                                    mesh.selected_edges.contains(&(a.min(b), a.max(b)))
                                });
                        }
                        mesh.selected_edges.clear();
                        mesh.sync_vert_selection_from_faces();
                    }
                    _ => {}
                },
                SelectionDomain::Face => match domain {
                    SelectionDomain::Vertex => {
                        mesh.sync_vert_selection_from_faces();
                        mesh.faces.iter_mut().for_each(|face| face.selected = false);
                        mesh.selected_edges.clear();
                    }
                    SelectionDomain::Edge => {
                        mesh.selected_edges.clear();
                        for face in mesh.faces.iter().filter(|face| face.selected) {
                            for i in 0..face.verts.len() {
                                let a = face.verts[i];
                                let b = face.verts[(i + 1) % face.verts.len()];
                                mesh.selected_edges.insert((a.min(b), a.max(b)));
                            }
                        }
                        mesh.faces.iter_mut().for_each(|face| face.selected = false);
                        for vertex in &mut mesh.verts {
                            vertex.selected = false;
                        }
                        for &(a, b) in &mesh.selected_edges {
                            mesh.verts[a as usize].selected = true;
                            mesh.verts[b as usize].selected = true;
                        }
                    }
                    _ => {}
                },
                SelectionDomain::Object => {}
            }
        }
        self.session.tools.hover = HoverTarget::None;
        self.session.selection_domain = domain;
        if domain.is_component() {
            self.session.last_component_domain = domain;
            if let Some(mode) = domain.as_select_mode() {
                self.select_mode = mode;
            }
        }
        self.sync_selection();
        self.mark_dirty();
    }

    /// Alterna entre o domínio Object e o último domínio de componente usado (Tab, P3D-015).
    pub fn cycle_selection_domain(&mut self) {
        if self.session.selection_domain == SelectionDomain::Object {
            let target = self.session.last_component_domain;
            self.set_selection_domain(if target.is_component() {
                target
            } else {
                SelectionDomain::Vertex
            });
        } else {
            self.set_selection_domain(SelectionDomain::Object);
        }
    }

    /// Calcula a posição no espaço de mundo do pivô selecionado (P3D-027).
    pub fn calculate_pivot(&self, pivot: PivotPoint) -> glam::Vec3 {
        if self.selection_domain() == SelectionDomain::Object && pivot != PivotPoint::Cursor3D {
            if let Some(active) = self.project.active()
                && let Some(origin) = active.origin
                && self.session.selection.assets.len() <= 1
            {
                return glam::Vec3::from(origin);
            }
            let points: Vec<_> = self
                .project
                .assets
                .iter()
                .filter(|a| {
                    !a.locked
                        && (self.session.selection.assets.contains(&a.id)
                            || self
                                .project
                                .active()
                                .is_some_and(|active| active.id == a.id))
                })
                .flat_map(|a| a.mesh.verts.iter().map(|v| v.vec()))
                .collect();
            if !points.is_empty() {
                if pivot == PivotPoint::BoundingBoxCenter {
                    let min = points
                        .iter()
                        .copied()
                        .fold(glam::Vec3::splat(f32::INFINITY), glam::Vec3::min);
                    let max = points
                        .iter()
                        .copied()
                        .fold(glam::Vec3::splat(f32::NEG_INFINITY), glam::Vec3::max);
                    return (min + max) * 0.5;
                }
                return points.iter().sum::<glam::Vec3>() / points.len() as f32;
            }
        }
        match pivot {
            PivotPoint::Cursor3D => glam::Vec3::from(self.cursor_3d),
            PivotPoint::BoundingBoxCenter => {
                if self.selection_domain() == SelectionDomain::Object
                    && let Some(active) = self.project.active()
                    && let Some(origin) = active.origin
                {
                    glam::Vec3::from(origin)
                } else if let Some(mesh) = self.project.active_mesh() {
                    let mut min = glam::Vec3::splat(f32::MAX);
                    let mut max = glam::Vec3::splat(f32::MIN);
                    let mut count = 0;
                    for v in &mesh.verts {
                        if v.selected || self.session.selection_domain == SelectionDomain::Object {
                            let p = v.vec();
                            min = min.min(p);
                            max = max.max(p);
                            count += 1;
                        }
                    }
                    if count > 0 && min.is_finite() && max.is_finite() {
                        (min + max) * 0.5
                    } else {
                        glam::Vec3::from(self.cursor_3d)
                    }
                } else {
                    glam::Vec3::from(self.cursor_3d)
                }
            }
            PivotPoint::MedianPoint | PivotPoint::IndividualOrigins => {
                if self.selection_domain() == SelectionDomain::Object
                    && let Some(active) = self.project.active()
                    && let Some(origin) = active.origin
                {
                    glam::Vec3::from(origin)
                } else if let Some(mesh) = self.project.active_mesh() {
                    glam::Vec3::from(mesh.selection_center())
                } else {
                    glam::Vec3::from(self.cursor_3d)
                }
            }
        }
    }

    /// Define a origem do objeto ativo para o centro da sua geometria.
    pub fn set_origin_geometry(&mut self) -> Result<(), &'static str> {
        let center = {
            let active = self.project.active().ok_or("No active asset")?;
            if active.locked {
                self.set_status("Asset is locked");
                return Err("Asset is locked");
            }
            if active.mesh.verts.is_empty() {
                self.set_status("Asset mesh is empty");
                return Err("Asset mesh is empty");
            }
            active.mesh.selection_center()
        };
        let prev = self.project.project.clone();
        if let Some(active) = self.project.active_mut() {
            active.origin = Some(center);
        }
        self.project
            .undo
            .checkpoint_sized("Set Origin to Geometry", &prev, prev.estimated_bytes());
        self.mark_document_dirty();
        self.emit_transform_changed();
        self.set_status("Origin set to geometry center.");
        Ok(())
    }

    /// Define a origem do objeto ativo para a base mais baixa (Y mínimo) da geometria.
    pub fn set_origin_bottom(&mut self) -> Result<(), &'static str> {
        let bottom_origin = {
            let active = self.project.active().ok_or("No active asset")?;
            if active.locked {
                self.set_status("Asset is locked");
                return Err("Asset is locked");
            }
            if active.mesh.verts.is_empty() {
                self.set_status("Asset mesh is empty");
                return Err("Asset mesh is empty");
            }
            let mut min = glam::Vec3::splat(f32::MAX);
            let mut max = glam::Vec3::splat(f32::MIN);
            for v in &active.mesh.verts {
                let p = v.vec();
                min = min.min(p);
                max = max.max(p);
            }
            if !min.is_finite() || !max.is_finite() {
                self.set_status("Mesh vertices are non-finite");
                return Err("Mesh vertices are non-finite");
            }
            [(min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5]
        };
        let prev = self.project.project.clone();
        if let Some(active) = self.project.active_mut() {
            active.origin = Some(bottom_origin);
        }
        self.project
            .undo
            .checkpoint_sized("Set Origin to Bottom", &prev, prev.estimated_bytes());
        self.mark_document_dirty();
        self.emit_transform_changed();
        self.set_status("Origin set to bottom.");
        Ok(())
    }

    /// Define a origem do objeto ativo para a posição atual do 3D Cursor.
    pub fn set_origin_cursor(&mut self) -> Result<(), &'static str> {
        let cursor = self.session.cursor_3d;
        {
            let active = self.project.active().ok_or("No active asset")?;
            if active.locked {
                self.set_status("Asset is locked");
                return Err("Asset is locked");
            }
        }
        let prev = self.project.project.clone();
        if let Some(active) = self.project.active_mut() {
            active.origin = Some(cursor);
        }
        self.project.undo.checkpoint_sized(
            "Set Origin to 3D Cursor",
            &prev,
            prev.estimated_bytes(),
        );
        self.mark_document_dirty();
        self.emit_transform_changed();
        self.set_status("Origin set to 3D Cursor.");
        Ok(())
    }

    /// Define a origem do objeto ativo para a média da seleção atual.
    pub fn set_origin_selection(&mut self) -> Result<(), &'static str> {
        let center = {
            let active = self.project.active().ok_or("No active asset")?;
            if active.locked {
                self.set_status("Asset is locked");
                return Err("Asset is locked");
            }
            if active.mesh.verts.is_empty() {
                self.set_status("Asset mesh is empty");
                return Err("Asset mesh is empty");
            }
            active.mesh.selection_center()
        };
        let prev = self.project.project.clone();
        if let Some(active) = self.project.active_mut() {
            active.origin = Some(center);
        }
        self.project.undo.checkpoint_sized(
            "Set Origin to Selection",
            &prev,
            prev.estimated_bytes(),
        );
        self.mark_document_dirty();
        self.emit_transform_changed();
        self.set_status("Origin set to selection.");
        Ok(())
    }

    /// Translada a geometria para que seu centro coincida com o ponto de origem (0, 0, 0)
    /// ou a origem definida do objeto.
    pub fn set_geometry_to_origin(&mut self) -> Result<(), &'static str> {
        let (delta, target_origin) = {
            let active = self.project.active().ok_or("No active asset")?;
            if active.locked {
                self.set_status("Asset is locked");
                return Err("Asset is locked");
            }
            if active.mesh.verts.is_empty() {
                self.set_status("Asset mesh is empty");
                return Err("Asset mesh is empty");
            }
            let center = active.mesh.selection_center();
            let target_origin = active.origin.unwrap_or([0.0, 0.0, 0.0]);
            let delta = [
                target_origin[0] - center[0],
                target_origin[1] - center[1],
                target_origin[2] - center[2],
            ];
            (delta, target_origin)
        };
        let prev = self.project.project.clone();
        if let Some(active) = self.project.active_mut() {
            for v in &mut active.mesh.verts {
                v.pos[0] += delta[0];
                v.pos[1] += delta[1];
                v.pos[2] += delta[2];
            }
            active.origin = Some(target_origin);
        }
        self.project
            .undo
            .checkpoint_sized("Geometry to Origin", &prev, prev.estimated_bytes());
        self.mark_document_dirty();
        self.emit_project_changed(ProjectChanges::POSITIONS | ProjectChanges::TRANSFORMS);
        self.set_status("Geometry centered at origin.");
        Ok(())
    }

    /// Alterna o modo de edição de pivô (Ajustar Pivô).
    pub fn toggle_edit_pivot(&mut self) -> bool {
        self.session.edit_pivot = !self.session.edit_pivot;
        if self.session.edit_pivot {
            self.set_status("Edit Pivot Mode enabled (Move translates origin only).");
        } else {
            self.set_status("Edit Pivot Mode disabled.");
        }
        self.session.edit_pivot
    }

    /// Constrói o descritor de feedback da ferramenta modal ativa para renderização (P3D-131).
    pub fn current_tool_feedback(&self) -> Option<crate::modal_feedback::ToolFeedback> {
        let modal = self.modal.as_ref()?;
        let delta_text = match modal.kind {
            crate::modal::ModalKind::Move => {
                format!("Δ {:.2} m", modal.components.length())
            }
            crate::modal::ModalKind::Rotate => {
                format!("Rot {:.1}°", modal.value)
            }
            crate::modal::ModalKind::Scale => {
                format!("Scale {:.2}×", modal.value)
            }
            crate::modal::ModalKind::Extrude => {
                format!("Extrude {:.2} m", modal.value)
            }
            crate::modal::ModalKind::ExtrudeIndividual => {
                format!("Extrude Individual {:.2} m", modal.value)
            }
            crate::modal::ModalKind::Inset => {
                format!("Inset {:.2}", modal.value)
            }
            crate::modal::ModalKind::Bevel => {
                format!("Bevel {:.2} m", modal.value)
            }
            crate::modal::ModalKind::PushPull => {
                format!("Push/Pull {:.2} m", modal.value)
            }
        };

        let point = modal.current_point();
        let current = point.unwrap_or(modal.pivot);
        let mut fb =
            crate::modal_feedback::ToolFeedback::new(modal.pivot, current, delta_text, modal.value);
        if point.is_none() {
            // Graus, fatores e frações não são posições: sem linha-guia de mundo.
            fb.guide_line = None;
        }

        match modal.constraint {
            crate::modal::ModalConstraint::Axis(i) => fb.axis_constraint = Some(i),
            crate::modal::ModalConstraint::Plane(i) => fb.plane_constraint = Some(i),
            crate::modal::ModalConstraint::Free => {}
        }

        // Encaixou de fato, não apenas "snap ligado" (P3D-040).
        fb.is_snapped = modal.snapped();
        fb.snap_kind = modal.snap_kind();
        Some(fb)
    }

    /// Descriptor canônico do pincel (iniciativa Paint — P3D-056/057).
    ///
    /// Fonte única da verdade: une os campos legados de sessão (`canvas_brush`,
    /// `paint_brush_kind`, `paint_strength`) com os novos (`brush_hardness`,
    /// `brush_flow`, `brush_spacing`). A UI migra para escrever `BrushSettings`
    /// diretamente; enquanto isso, este método resolve o valor efetivo.
    pub fn brush_settings(&self) -> crate::BrushSettings {
        crate::BrushSettings {
            kind: crate::brush_type_from_kind(self.session.tools.paint_brush_kind),
            size_px: self.session.tools.canvas_brush.max(1) as f32,
            hardness: self.session.tools.brush_hardness,
            strength: self.session.tools.paint_strength,
            flow: self.session.tools.brush_flow,
            spacing: self.session.tools.brush_spacing,
        }
        .sanitized()
    }

    /// Raio de mundo equivalente ao pincel na profundidade do hit 3D.
    ///
    /// Contrato de coerência (iniciativa Paint): o anel de preview na viewport
    /// e o carimbo real usam **o mesmo** valor. Deriva o tamanho em pixels de
    /// tela para mundo pela projeção da câmera na profundidade do ponto.
    pub fn brush_world_radius(&self, hit: Vec3) -> f32 {
        self.world_radius_for_px(hit, self.brush_settings().size_px)
    }

    /// Raio de mundo (metade do diâmetro) de um pincel de `size_px` pixels de
    /// tela na profundidade de `hit`.
    pub fn world_radius_for_px(&self, hit: Vec3, size_px: f32) -> f32 {
        let view_dir = self.camera.forward();
        let depth = (hit - self.camera.eye()).dot(view_dir).max(0.05);
        let half_fov = (self.camera.fov_y * 0.5).to_radians();
        let world_height = 2.0 * depth * half_fov.tan();
        let px_height = self
            .ui
            .viewport_rect
            .map(|r| (r.max[1] - r.min[1]) * self.ui.viewport_pixels_per_point)
            .unwrap_or(1080.0);
        let world_per_px = world_height / px_height.max(1.0);
        size_px * world_per_px * 0.5
    }

    /// Pinta vértices próximos do ponto 3D (vertex paint).
    pub fn paint_at(&mut self, center: Vec3) {
        self.paint_at_with_face(center, None)
    }

    /// Pinta respeitando `fill_scope` e `brush_lock`.
    ///
    /// `face_hint` é o índice da face sob o cursor, quando o chamador tem um:
    /// sem ele os escopos por face caem no comportamento de raio em vez de
    /// inventar uma face.
    pub fn paint_at_with_face(&mut self, center: Vec3, face_hint: Option<usize>) {
        let before = self
            .session
            .tools
            .paint_stroke
            .is_none()
            .then(|| self.project.project.clone());
        let (mut n, col, r, k) = (
            0,
            self.session.tools.paint_color,
            self.session.tools.paint_radius,
            self.session.tools.paint_strength.clamp(0.0, 1.0),
        );
        let r2 = r * r;
        let scope = self.session.tools.fill_scope;
        let lock = self.session.tools.brush_lock;

        // Conjunto de vértices elegíveis pelo escopo. `None` = todos (raio decide).
        let allowed: Option<Vec<bool>> = match scope {
            crate::brush::FillScope::ConnectedPixels => None,
            crate::brush::FillScope::Object => {
                Some(vec![
                    true;
                    self.project.active_mesh().map_or(0, |m| m.verts.len())
                ])
            }
            crate::brush::FillScope::Face => face_hint.and_then(|face| {
                self.project.active_mesh().map(|mesh| {
                    let mut mask = vec![false; mesh.verts.len()];
                    if let Some(face) = mesh.faces.get(face) {
                        for &vi in &face.verts {
                            if let Some(slot) = mask.get_mut(vi as usize) {
                                *slot = true;
                            }
                        }
                    }
                    mask
                })
            }),
            crate::brush::FillScope::SelectedFaces => self.project.active_mesh().map(|mesh| {
                let mut mask = vec![false; mesh.verts.len()];
                for face in &mesh.faces {
                    if face.selected {
                        for &vi in &face.verts {
                            if let Some(slot) = mask.get_mut(vi as usize) {
                                *slot = true;
                            }
                        }
                    }
                }
                mask
            }),
            crate::brush::FillScope::UvIsland => face_hint.and_then(|face| {
                self.project.active_mesh().map(|mesh| {
                    let mut mask = vec![false; mesh.verts.len()];
                    let islands = mesh.uv_islands();
                    if let Some(island) = islands.iter().find(|island| island.faces.contains(&face))
                    {
                        for &fi in &island.faces {
                            if let Some(face) = mesh.faces.get(fi) {
                                for &vi in &face.verts {
                                    if let Some(slot) = mask.get_mut(vi as usize) {
                                        *slot = true;
                                    }
                                }
                            }
                        }
                    }
                    mask
                })
            }),
        };

        // Trava de pincel: restringe a superfície alcançável a partir do primeiro
        // toque do traço, sem bloquear o resto do fluxo.
        let locked_face = match lock {
            crate::brush::BrushLock::None => None,
            crate::brush::BrushLock::FirstObject => Some(None),
            crate::brush::BrushLock::FirstFace => Some(face_hint),
            // Travar nas faces selecionadas é um escopo, não uma trava de
            // primeiro toque: quem restringe é `fill_scope`, não o lock.
            crate::brush::BrushLock::SelectedFaces => None,
        };
        if let Some(face) = locked_face {
            self.session.tools.paint_lock_face.get_or_insert(face);
        }
        let lock_face = self.session.tools.paint_lock_face;

        if let Some(obj) = self.project.active_mut() {
            let mesh = &mut obj.mesh;
            for (index, v) in mesh.verts.iter_mut().enumerate() {
                if allowed.as_ref().is_some_and(|mask| !mask[index]) {
                    continue;
                }
                let d2 = (v.vec() - center).length_squared();
                let in_radius = scope == crate::brush::FillScope::Object
                    || scope == crate::brush::FillScope::SelectedFaces
                    || d2 <= r2;
                if !in_radius {
                    continue;
                }
                if let (Some(locked), Some(current)) = (lock_face, face_hint)
                    && locked.is_some()
                    && locked != Some(current)
                {
                    continue;
                }
                for (ch, cc) in v.color.iter_mut().zip(col.iter()) {
                    *ch = *ch * (1.0 - k) + cc * k;
                }
                n += 1;
            }
        }
        if n > 0 {
            if let Some(before) = before {
                self.project.checkpoint_snapshot("paint", &before);
            }
            self.emit_color_changed();
            self.ui.status = format!("paint: {n} verts");
        }
    }

    pub fn begin_paint_stroke(&mut self) {
        if self.session.tools.modal.is_some() || self.session.tools.mesh_preview.is_some() {
            return;
        }
        if self.session.tools.paint_stroke.is_none() {
            self.session.tools.paint_stroke = Some(self.project.project.clone());
            // A trava de pincel vale por traço: o próximo traço pode começar em
            // outra superfície.
            self.session.tools.paint_lock_face = None;
            self.session.tools.paint_restriction = None;
            self.session.tools.paint_buffer = None;
        }
    }

    pub fn finish_paint_stroke(&mut self, cancel: bool) {
        let Some(original) = self.session.tools.paint_stroke.take() else {
            return;
        };
        self.session.tools.paint_restriction = None;
        self.session.tools.paint_buffer = None;
        self.session.tools.paint_lock_face = None;
        let colors_changed = original.assets.len() != self.project.assets.len()
            || original
                .assets
                .iter()
                .zip(self.project.assets.iter())
                .any(|(before, after)| {
                    before.mesh.verts.len() != after.mesh.verts.len()
                        || before
                            .mesh
                            .verts
                            .iter()
                            .zip(&after.mesh.verts)
                            .any(|(before, after)| before.color != after.color)
                });
        let paint_changed = original.assets.len() != self.project.assets.len()
            || original
                .assets
                .iter()
                .zip(self.project.assets.iter())
                .any(|(before, after)| {
                    before.texture != after.texture || before.paint_stack != after.paint_stack
                });
        let mut changes = ProjectChanges::NONE;
        if colors_changed {
            changes |= ProjectChanges::COLORS;
        }
        if paint_changed {
            changes |= ProjectChanges::TEXTURES;
        }
        if cancel {
            let revision_clock =
                (!changes.is_empty()).then(|| self.project.project.revision_clock());
            self.project.project = original;
            if let Some(revision_clock) = revision_clock {
                self.project
                    .project
                    .rebase_revisions_after_restore(revision_clock);
            }
            self.emit_project_changed(changes);
        } else {
            if !changes.is_empty() {
                self.project.checkpoint_snapshot("Paint stroke", &original);
            }
        }
    }

    /// Vértice mais próximo do raio (pick em perspectiva e ortográfica).
    pub fn pick_vertex(&self, origin: Vec3, dir: Vec3) -> Option<(usize, Vec3)> {
        self.project.pick_vertex(origin, dir)
    }

    /// Aresta mais próxima do raio (modo Edge).
    pub fn pick_edge(&self, origin: Vec3, dir: Vec3) -> Option<((u32, u32), Vec3)> {
        self.project.pick_edge(origin, dir)
    }

    /// Salva o objeto ativo atual como um novo asset permanente na biblioteca do projeto.
    pub fn save_active_as_asset(&mut self) -> bool {
        if let Some(active_asset) = self.project.active() {
            let mut cloned = active_asset.duplicate();
            cloned.name = format!("{} (Asset)", active_asset.name);
            let name = cloned.name.clone();
            self.checkpoint("save asset");
            self.project.assets.push(cloned);
            self.set_status(format!("Asset '{}' salvo na biblioteca do projeto", name));
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Ids dos objetos a capturar: a seleção de objetos ou, sem seleção, o ativo.
    fn selected_or_active_asset_ids(&self) -> Vec<uuid::Uuid> {
        let live = |id: &uuid::Uuid| self.project.assets.iter().any(|a| a.id == *id);
        let selected: Vec<uuid::Uuid> = self
            .session
            .selection
            .assets
            .iter()
            .copied()
            .filter(live)
            .collect();
        if !selected.is_empty() {
            return selected;
        }
        self.project
            .active()
            .map(|a| vec![a.id])
            .unwrap_or_default()
    }

    fn prefab_status(&mut self, key: &str, name: &str) {
        let message = self.t(key).replace("{name}", name);
        self.set_status(message);
    }

    /// Salva a seleção (ou o objeto ativo) como **prefab** da biblioteca.
    /// A cena não muda: o prefab vive à parte de `project.assets`.
    pub fn save_selection_as_prefab(&mut self, name: Option<&str>) -> Option<uuid::Uuid> {
        let ids = self.selected_or_active_asset_ids();
        if ids.is_empty() {
            return None;
        }
        self.checkpoint("save prefab");
        let id = self.project.create_prefab(&ids, name)?;
        let saved = self
            .project
            .prefabs
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone())
            .unwrap_or_default();
        self.prefab_status("prefab.saved", &saved);
        self.mark_dirty();
        Some(id)
    }

    /// Instancia o prefab na cena (posição dada ou 3D Cursor) e seleciona as cópias.
    pub fn instantiate_prefab(
        &mut self,
        prefab_id: uuid::Uuid,
        position: Option<[f32; 3]>,
    ) -> bool {
        let Some(name) = self
            .project
            .prefabs
            .iter()
            .find(|p| p.id == prefab_id)
            .map(|p| p.name.clone())
        else {
            return false;
        };
        self.set_selection_domain(SelectionDomain::Object);
        self.checkpoint("instantiate prefab");
        let target = position.unwrap_or(self.session.cursor_3d);
        let created = self.project.instantiate_prefab(prefab_id, target);
        if created.is_empty() {
            return false;
        }
        self.session.selection.assets = created;
        self.sync_selection();
        self.emit_mesh_changed();
        self.prefab_status("prefab.instantiated", &name);
        self.mark_dirty();
        true
    }

    /// Remove o prefab da biblioteca (instâncias na cena permanecem).
    pub fn delete_prefab(&mut self, prefab_id: uuid::Uuid) -> bool {
        let Some(name) = self
            .project
            .prefabs
            .iter()
            .find(|p| p.id == prefab_id)
            .map(|p| p.name.clone())
        else {
            return false;
        };
        self.checkpoint("delete prefab");
        self.project.remove_prefab(prefab_id);
        self.prefab_status("prefab.deleted", &name);
        self.mark_dirty();
        true
    }

    pub fn rename_prefab(&mut self, prefab_id: uuid::Uuid, name: &str) -> bool {
        let exists = self.project.prefabs.iter().any(|p| p.id == prefab_id);
        if !exists || name.trim().is_empty() {
            return false;
        }
        self.checkpoint("rename prefab");
        if !self.project.rename_prefab(prefab_id, name) {
            return false;
        }
        self.prefab_status("prefab.renamed", name.trim());
        self.mark_dirty();
        true
    }

    pub fn set_prefab_favorite(&mut self, prefab_id: uuid::Uuid, favorite: bool) -> bool {
        let changed = self.project.set_prefab_favorite(prefab_id, favorite);
        if changed {
            self.mark_dirty();
        }
        changed
    }

    /// Re-captura o prefab a partir da seleção atual; instâncias antigas ficam desatualizadas.
    pub fn update_prefab_from_selection(&mut self, prefab_id: uuid::Uuid) -> bool {
        let ids = self.selected_or_active_asset_ids();
        let Some(name) = self
            .project
            .prefabs
            .iter()
            .find(|p| p.id == prefab_id)
            .map(|p| p.name.clone())
        else {
            return false;
        };
        if ids.is_empty() {
            return false;
        }
        self.checkpoint("update prefab");
        if !self.project.update_prefab_from(prefab_id, &ids) {
            return false;
        }
        self.prefab_status("prefab.updated", &name);
        self.mark_dirty();
        true
    }

    /// Limite canônico do nome de ativo exibido no Outliner e nos painéis.
    pub const fn asset_name_max_len() -> usize {
        ASSET_NAME_MAX_LEN
    }

    /// Aplica uma operação booleana entre o ativo (A) e o operando (B).
    ///
    /// O resultado substitui a malha de A e B é removido da cena, que é o
    /// comportamento canônico de Fuse/Cut sem o modificador Keep Parts.
    pub fn apply_boolean(
        &mut self,
        op: petunia_mesh::boolean::BooleanOp,
    ) -> Result<usize, petunia_mesh::boolean::BooleanError> {
        use petunia_mesh::boolean::{BooleanError, boolean_meshes};
        let operand_id = self
            .session
            .tools
            .boolean_operand
            .ok_or(BooleanError::InvalidInput("no operand selected"))?;
        let active_index = self.project.active;
        let operand_index = self
            .project
            .assets
            .iter()
            .position(|asset| asset.id == operand_id)
            .ok_or(BooleanError::InvalidInput("operand no longer exists"))?;
        if operand_index == active_index {
            return Err(BooleanError::InvalidInput("operand is the active asset"));
        }
        // O kernel só aceita triângulos: `boolean_meshes_clean` triangula cópias dos
        // operandos e, depois, funde de volta as regiões coplanares (cap. 04).
        let a = self.project.assets[active_index].mesh.clone();
        let b = self.project.assets[operand_index].mesh.clone();
        let (mut result, report) = if self.session.tools.boolean_cleanup {
            let (mesh, report) = petunia_mesh::boolean::boolean_meshes_clean(&a, &b, op)?;
            (mesh, Some(report))
        } else {
            let (mut ta, mut tb) = (a, b);
            ta.triangulate();
            tb.triangulate();
            (boolean_meshes(&ta, &tb, op)?, None)
        };
        let texture = if report.as_ref().is_some_and(|r| {
            r.uv == petunia_mesh::boolean_cleanup::UvOutcome::Inherited && !r.fell_back
        }) {
            None
        } else {
            crate::boolean_texture::transfer(
                &self.project.project,
                &mut result,
                &self.project.assets[active_index],
                &self.project.assets[operand_index],
            )
        };
        if texture.is_none() && report.as_ref().is_none_or(|r| r.fell_back) {
            result.layout_uv_charts(petunia_mesh::primitives::DEFAULT_UV_PADDING);
        }
        let verts = result.verts.len();
        let mut report = report;
        if texture.is_some()
            && let Some(report) = &mut report
        {
            report.uv = petunia_mesh::boolean_cleanup::UvOutcome::Relaid;
        }
        self.session.tools.boolean_report = report;

        self.checkpoint(match op {
            petunia_mesh::boolean::BooleanOp::Union => "fuse",
            petunia_mesh::boolean::BooleanOp::Difference => "cut",
            petunia_mesh::boolean::BooleanOp::Intersection => "intersect",
        });
        let albedo_name = self.t_id(petunia_config::TextId::new("surface.boolean_albedo"));
        let material_id = texture
            .as_ref()
            .and_then(|transfer| transfer.material.clone())
            .map(|m| self.project.project.add_material(m));
        if let Some(asset) = self.project.assets.get_mut(active_index) {
            asset.mesh = result;
            asset.parametric = None;
            if let Some(material_id) = material_id {
                asset.material_id = Some(material_id);
            }
            if let Some(transfer) = texture {
                let texture = transfer.albedo;
                let name = albedo_name;
                let mut stack = petunia_project::paint_layers::PaintLayerStack::new();
                stack.add_layer(petunia_project::paint_layers::PaintLayer::new_raster(
                    name,
                    texture.clone(),
                ));
                asset.texture = Some(texture);
                asset.paint_stack = Some(stack);
            }
        }
        // Keep Parts mantém B na cena; sem o modificador ele é consumido, e
        // remover antes do ativo desloca o índice.
        if !self.session.tools.boolean_keep_parts {
            self.project.assets.remove(operand_index);
            if operand_index < active_index {
                self.project.active = active_index - 1;
            }
        }
        self.session.tools.boolean_operand = None;
        self.sync_selection();
        self.emit_mesh_changed();
        self.mark_dirty();
        Ok(verts)
    }

    /// **Join**: funde o operando no ativo como um único objeto, sem kernel
    /// booleano — a topologia dos dois é preservada lado a lado.
    pub fn join_active_with_operand(&mut self) -> Result<usize, &'static str> {
        let operand_id = self
            .session
            .tools
            .boolean_operand
            .ok_or("Choose a boolean operand first")?;
        let active_index = self.project.active;
        let operand_index = self
            .project
            .assets
            .iter()
            .position(|asset| asset.id == operand_id)
            .ok_or("The boolean operand no longer exists")?;
        if operand_index == active_index {
            return Err("The boolean operand cannot be the active object");
        }
        let other = self.project.assets[operand_index].mesh.clone();
        let added = other.verts.len();
        self.checkpoint("join");
        if let Some(asset) = self.project.assets.get_mut(active_index) {
            asset.mesh.join(&other);
        }
        self.project.assets.remove(operand_index);
        if operand_index < active_index {
            self.project.active = active_index - 1;
        }
        self.session.tools.boolean_operand = None;
        self.sync_selection();
        self.emit_mesh_changed();
        self.mark_dirty();
        Ok(added)
    }

    /// Renomeia o ativo ativo (P3D-093).
    ///
    /// Retorna `Ok(true)` quando o nome mudou — uma única entrada de undo — e
    /// `Ok(false)` quando o nome pedido já era o atual, para que confirmar sem
    /// editar não empilhe histórico. Espaços nas pontas são removidos antes da
    /// validação, então `"  Cube  "` vira `"Cube"` e `"   "` é recusado.
    pub fn rename_active_asset(&mut self, name: &str) -> Result<bool, AssetRenameError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AssetRenameError::EmptyName);
        }
        if trimmed.chars().count() > ASSET_NAME_MAX_LEN {
            return Err(AssetRenameError::NameTooLong);
        }
        let index = self.project.active;
        let Some(asset) = self.project.assets.get(index) else {
            return Err(AssetRenameError::NoActiveAsset);
        };
        if asset.name == trimmed {
            return Ok(false);
        }
        let previous = asset.name.clone();
        self.checkpoint(&format!("rename: {trimmed}"));
        if let Some(asset) = self.project.assets.get_mut(index) {
            asset.name = trimmed.to_string();
        }
        self.set_status(format!("Renamed '{previous}' to '{trimmed}'"));
        self.mark_dirty();
        Ok(true)
    }

    /// Centraliza e enquadra a câmera 3D na geometria selecionada (ou em todo o modelo ativo).
    pub fn frame_selection(&mut self) {
        if let Some(o) = self.project.assets.get(self.project.active) {
            let mut c = glam::Vec3::ZERO;
            let mut n = 0;
            let mut r: f32 = 0.0;
            for v in &o.mesh.verts {
                if v.selected {
                    c += v.vec();
                    n += 1;
                }
            }
            if n == 0 {
                for v in &o.mesh.verts {
                    c += v.vec();
                }
                n = o.mesh.verts.len().max(1);
            }
            c /= n as f32;
            let has_selection = o.mesh.verts.iter().any(|v| v.selected);
            for v in &o.mesh.verts {
                if !has_selection || v.selected {
                    r = r.max((v.vec() - c).length());
                }
            }
            let mut goal = self.camera.clone();
            goal.frame(c, r.max(0.05));
            self.camera_frame = Some((self.camera.clone(), goal, 0.0));
            self.mark_dirty();
        }
    }

    /// Centraliza e enquadra a câmera 3D em todo o conteúdo visível da cena (P3D-008).
    /// Não altera o estado de dirty do projeto.
    pub fn frame_all(&mut self) {
        let mut min = glam::Vec3::splat(f32::INFINITY);
        let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
        let mut count = 0;

        for asset in &self.project.assets {
            if !asset.visible {
                continue;
            }
            for v in &asset.mesh.verts {
                let p = v.vec();
                min = min.min(p);
                max = max.max(p);
                count += 1;
            }
        }

        for r in &self.project.refs {
            if !r.visible {
                continue;
            }
            let half = r.size * 0.5;
            let offset = r.offset;
            match r.axis {
                RefAxis::Front | RefAxis::Back => {
                    min = min.min(glam::Vec3::new(-half, -half, offset));
                    max = max.max(glam::Vec3::new(half, half, offset));
                }
                RefAxis::Left | RefAxis::Right | RefAxis::Side => {
                    min = min.min(glam::Vec3::new(offset, -half, -half));
                    max = max.max(glam::Vec3::new(offset, half, half));
                }
                RefAxis::Top | RefAxis::Bottom => {
                    min = min.min(glam::Vec3::new(-half, offset, -half));
                    max = max.max(glam::Vec3::new(half, offset, half));
                }
            }
            count += 1;
        }

        let (center, radius) = if count > 0 && min.is_finite() && max.is_finite() {
            let c = (min + max) * 0.5;
            let r = (max - min).length() * 0.5;
            (c, r.max(0.2))
        } else {
            (glam::Vec3::ZERO, 1.0)
        };

        let mut goal = self.camera.clone();
        goal.frame(center, radius);
        self.camera_frame = Some((self.camera.clone(), goal, 0.0));
        self.mark_dirty();
    }

    /// Despacha um comando registrado no CommandDispatcher da aplicação.
    pub fn dispatch_command(&mut self, id: &str) -> Result<(), crate::command::CommandError> {
        let cmd = self
            .commands
            .get(id)
            .ok_or_else(|| crate::command::CommandError::UnknownCommand(id.to_string()))?;
        cmd.can_execute(self)
            .map_err(|reason| crate::command::CommandError::Execution(reason.to_string()))?;
        crate::command::CommandDispatcher::dispatch(self, cmd.as_ref())
    }

    /// Canonical Application API entry: UI / CLI / FFI / MCP / Lua all land here.
    pub fn dispatch_intent(
        &mut self,
        intent: &crate::schema_contracts::CommandIntent,
    ) -> Result<(), crate::command::CommandError> {
        if let Some(id) = intent
            .asset_id
            .as_deref()
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            && let Some(idx) = self.project.find(id)
        {
            self.project.active = idx;
        }
        match intent.command.as_str() {
            "model.extrude" => {
                if let Some(dist) = intent.args.first() {
                    self.tools.extrude_dist = *dist as f32;
                }
                self.dispatch(&crate::command::ExtrudeSelectedCmd {
                    dist: self.tools.extrude_dist,
                })
            }
            "model.subdivide" => {
                if let Some(cuts) = intent.args.first() {
                    self.tools.subdivide_cuts = (*cuts as u32).clamp(1, 6);
                }
                self.dispatch(&crate::command::SubdivideSelectionCmd)
            }
            "model.bevel" => {
                if let Some(amount) = intent.args.first() {
                    self.tools.bevel_amount = *amount as f32;
                }
                if let Some(segs) = intent.args.get(1) {
                    self.tools.bevel_segments = (*segs as u32).clamp(1, 4);
                }
                self.dispatch(&crate::command::BevelCmd {
                    amount: self.tools.bevel_amount,
                    segments: self.tools.bevel_segments,
                    clamp_overlap: self.tools.bevel_clamp_overlap,
                    affect_vertices: self.tools.bevel_affect_vertices,
                })
            }
            "model.scale" | "model.scale_selection" => {
                let factor = intent.args.first().copied().unwrap_or(1.0) as f32;
                self.dispatch(&crate::command::ScaleSelectionCmd { factor })
            }
            "model.loop_cut" => {
                let cuts = intent.args.first().copied().unwrap_or(1.0) as u32;
                let even = intent.args.get(1).copied().unwrap_or(1.0) >= 0.5;
                let balanced = intent.args.get(2).copied().unwrap_or(0.0) >= 0.5;
                self.dispatch(&crate::command::LoopCutCmd {
                    cuts: cuts.clamp(1, 32),
                    even,
                    slide: 0.0,
                    balanced,
                })
            }
            "uv.unwrap_auto" => self.dispatch(&crate::command::UnwrapAutoCmd),
            "uv.pack_islands" => {
                let padding = intent.args.first().copied().unwrap_or(0.01) as f32;
                self.dispatch(&crate::command::UvPackIslandsCmd { padding })
            }
            "uv.project_view" => self.dispatch(&crate::command::UvProjectFromViewCmd),
            "model.add_primitive" | "model.add_cube" => {
                let kind = if intent.command == "model.add_cube" {
                    crate::command::PrimitiveKind::Cube
                } else {
                    match intent.asset_id.as_deref() {
                        Some(name) => crate::command::PrimitiveKind::parse(name)?,
                        None => crate::command::PrimitiveKind::Cube,
                    }
                };
                let name = intent.asset_id.clone();
                self.dispatch(&crate::command::AddPrimitiveCmd {
                    kind,
                    name,
                    at_cursor: true,
                })
            }
            other => self.dispatch_command(other),
        }
    }

    /// Cria uma nova instância de um asset da biblioteca na posição do 3D Cursor.
    pub fn instantiate_asset_at_cursor(&mut self, asset_index: usize) -> bool {
        if let Some(asset) = self.project.assets.get(asset_index) {
            let mut new_asset = asset.duplicate();
            let cursor = self.session.cursor_3d;
            for v in &mut new_asset.mesh.verts {
                v.pos[0] += cursor[0];
                v.pos[1] += cursor[1];
                v.pos[2] += cursor[2];
            }
            let name = new_asset.name.clone();
            self.checkpoint("instantiate asset");
            self.project.assets.push(new_asset);
            self.project.active = self.project.assets.len() - 1;
            self.sync_selection();
            self.emit_mesh_changed();
            self.set_status(format!("Asset '{}' instanciado na cena", name));
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Cria uma nova instância de um asset da biblioteca na posição especificada ou no 3D Cursor.
    pub fn instantiate_asset_by_id(
        &mut self,
        asset_id: uuid::Uuid,
        position: Option<[f32; 3]>,
    ) -> bool {
        if let Some(pos) = self.project.assets.iter().position(|a| a.id == asset_id) {
            self.set_selection_domain(SelectionDomain::Object);
            let mut new_asset = self.project.assets[pos].duplicate();
            let target_pos = position.unwrap_or(self.session.cursor_3d);
            let center = new_asset.mesh.selection_center();
            for v in &mut new_asset.mesh.verts {
                v.pos[0] += target_pos[0] - center[0];
                v.pos[1] += target_pos[1] - center[1];
                v.pos[2] += target_pos[2] - center[2];
            }
            new_asset.origin = Some(target_pos);
            let name = new_asset.name.clone();
            self.checkpoint("instantiate asset");
            self.project.assets.push(new_asset);
            self.project.active = self.project.assets.len() - 1;
            self.sync_selection();
            self.emit_mesh_changed();
            self.set_status(format!("Asset '{}' instanciado na cena", name));
            self.mark_dirty();
            return true;
        }
        false
    }

    /// Alterna o modo de isolamento da seleção atual (Local View / Isolate).
    pub fn toggle_isolate(&mut self) {
        if self.session.isolate_active {
            // Restaura as visibilidades anteriores
            if let Some(prev) = self.session.isolate_prev_visibilities.take() {
                for (i, &vis) in prev.iter().enumerate() {
                    if let Some(asset) = self.project.assets.get_mut(i) {
                        asset.visible = vis;
                    }
                }
            } else {
                for asset in &mut self.project.assets {
                    asset.visible = true;
                }
            }
            self.session.isolate_active = false;
            self.set_status("Modo de isolamento desativado".to_string());
        } else {
            // Salva as visibilidades atuais e isola o ativo
            let prev: Vec<bool> = self.project.assets.iter().map(|a| a.visible).collect();
            self.session.isolate_prev_visibilities = Some(prev);
            let active = self.project.active;
            for (i, asset) in self.project.assets.iter_mut().enumerate() {
                asset.visible = i == active;
            }
            self.session.isolate_active = true;
            let name = self
                .project
                .active()
                .map(|a| a.name.clone())
                .unwrap_or_else(|| "Ativo".to_string());
            self.set_status(format!("Objeto '{name}' isolado na cena"));
        }
        self.mark_dirty();
    }

    /// Mantém o isolamento no objeto ativo depois de trocar de objeto.
    ///
    /// Sem isso, trocar o ativo com o isolamento ligado deixaria o novo objeto
    /// escondido. As visibilidades originais continuam guardadas para o
    /// desfazer do isolamento.
    pub fn refresh_isolation(&mut self) {
        if !self.session.isolate_active {
            return;
        }
        let active = self.project.active;
        for (i, asset) in self.project.assets.iter_mut().enumerate() {
            asset.visible = i == active;
        }
        self.mark_dirty();
    }

    /// Verifica se o asset ativo está bloqueado contra transformações.
    pub fn is_active_locked(&self) -> bool {
        self.project.is_active_locked()
    }

    /// Alterna o bloqueio do asset ativo.
    pub fn toggle_lock_active(&mut self) {
        if let Some((name, locked)) = self.project.toggle_lock_active() {
            self.set_status(if locked {
                format!("Objeto '{name}' bloqueado")
            } else {
                format!("Objeto '{name}' desbloqueado")
            });
            self.mark_dirty();
        }
    }

    /// Carrega um projeto do disco e atualiza a sessão e os projetos recentes.
    pub fn open_project(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), crate::project_service::ProjectServiceError> {
        crate::project_service::ProjectService::load_project(self, path.as_ref())
    }
}

#[cfg(test)]
mod workspace_memory_tests {
    use super::*;
    use crate::selection::Workspace;

    #[test]
    fn switch_saves_and_restores_dock_layout() {
        let mut state = AppState::new("en");
        state.ui.right_dock_split = 0.6;
        state.ui.properties_tab = "material".to_string();
        state.ui.inspector_collapsed = true;

        state.switch_workspace(Workspace::Paint);
        assert_eq!(state.session.workspace, Workspace::Paint);
        // Destino visita pela 1ª vez: padrões.
        assert_eq!(state.ui.right_dock_split, 0.42);
        assert!(!state.ui.inspector_collapsed);

        state.ui.right_dock_split = 0.3;
        state.switch_workspace(Workspace::Model);
        // Retorno restaura a memória do Model.
        assert_eq!(state.ui.right_dock_split, 0.6);
        assert_eq!(state.ui.properties_tab, "material");
        assert!(state.ui.inspector_collapsed);

        // E a do Paint foi salva ao sair.
        state.switch_workspace(Workspace::Paint);
        assert_eq!(state.ui.right_dock_split, 0.3);
    }

    #[test]
    fn switch_to_same_workspace_is_noop() {
        let mut state = AppState::new("en");
        state.ui.right_dock_split = 0.7;
        state.switch_workspace(Workspace::Model);
        assert_eq!(state.ui.right_dock_split, 0.7);
    }

    #[test]
    fn workspace_index_covers_all_workspaces() {
        // A memória de layout é dimensionada por `Workspace::COUNT`: um workspace
        // novo não pode gerar índice fora do array.
        let mut seen = [false; Workspace::COUNT];
        for ws in Workspace::all() {
            let idx = workspace_index(ws);
            assert!(idx < Workspace::COUNT, "índice {idx} fora da memória de UI");
            seen[idx] = true;
        }
        assert!(seen.iter().all(|s| *s));
    }
}

#[cfg(test)]
mod model_quick_action_tests {
    use super::*;

    #[test]
    fn empty_preferences_resolve_to_canonical_defaults() {
        let state = AppState::new("en");
        assert_eq!(
            state.ui.model_quick_action_ids(),
            vec![
                "model.subdivide".to_string(),
                "model.fuse".to_string(),
                "model.cut".to_string(),
                "model.join".to_string(),
            ]
        );
    }

    #[test]
    fn pinning_sanitizes_unknown_ids_duplicates_and_limit() {
        let mut state = AppState::new("en");
        assert!(
            !state
                .ui
                .set_model_quick_action_pinned("model.unknown", true)
        );
        assert!(state.ui.set_model_quick_action_pinned("model.merge", true));
        assert!(!state.ui.set_model_quick_action_pinned("model.merge", true));
        assert!(state.ui.set_model_quick_action_pinned("model.slice", true));
        assert!(
            !state
                .ui
                .set_model_quick_action_pinned("model.loop_cut", false)
        );
        assert_eq!(
            state.ui.model_quick_action_ids(),
            vec![
                "model.subdivide".to_string(),
                "model.fuse".to_string(),
                "model.cut".to_string(),
                "model.join".to_string(),
                "model.merge".to_string(),
                "model.slice".to_string(),
            ]
        );
        assert!(
            !state
                .ui
                .set_model_quick_action_pinned("model.intersect", true)
        );
        assert!(state.ui.set_model_quick_action_pinned("model.merge", false));
        assert!(
            state
                .ui
                .set_model_quick_action_pinned("model.intersect", true)
        );
        assert!(state.ui.reset_model_quick_actions());
        assert!(!state.ui.reset_model_quick_actions());
    }
}
