//! Subsistema de Seção Analítica, Sonda de Espessura e Perfis Orgânicos (VSP-005).
//!
//! Este módulo implementa o Section Inspector e consultas espaciais associadas:
//! 1. `SectionPlane` & `SectionInspector`: consulta não-destrutiva de corte geométrico,
//!    separada estritamente de clipping em GPU e de Geometry Split topológico.
//!    Suporta contornos abertos, fechados e furos coplanares, além de caps sintéticas
//!    não-selecionáveis para visualização.
//! 2. `ThicknessProbe`: sonda analítica direcional de espessura de parede com classificação
//!    explícita de grandeza (`WallType`) e qualidade (`MeasurementQuality`).
//! 3. `OrganicCenterline` & `ProfileStation`: estações de perfis orgânicos ao longo de
//!    linhas de centro manuais ou estruturais com Parallel Transport Frame (Bishop Frame / RMF)
//!    para garantir continuidade sem inversão de normais (*flips*), com suporte a bifurcações.

use glam::Vec3;
use petunia_mesh::Mesh;
use petunia_project::{ObjectTransform, ProjectChanges};
use uuid::Uuid;

use crate::command::{Command, CommandContext, CommandError, EditorView};
use crate::modeling_feedback::MeasurementQuality;

// ============================================================================
// Tipos Principais — Section Inspector (VSP-005)
// ============================================================================

/// Ancoragem de referência para o plano de corte (VSP-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SectionAnchor {
    /// Ancorado nos eixos globais do mundo.
    #[default]
    World,
    /// Ancorado na transformação local do objeto inspecionado.
    Object,
}

/// Plano de corte analítico para inspeção geométrica (VSP-005).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SectionPlane {
    pub anchor: SectionAnchor,
    pub origin: [f32; 3],
    pub normal: [f32; 3],
    pub offset: f32,
    pub flipped: bool,
}

impl Default for SectionPlane {
    fn default() -> Self {
        Self {
            anchor: SectionAnchor::World,
            origin: [0.0, 0.0, 0.0],
            normal: [0.0, 1.0, 0.0], // Plano XZ cortando em Y = 0
            offset: 0.0,
            flipped: false,
        }
    }
}

impl SectionPlane {
    /// Cria um novo plano de corte com validação de números finitos e normal não-degenerada.
    pub fn new(
        anchor: SectionAnchor,
        origin: [f32; 3],
        normal: [f32; 3],
    ) -> Result<Self, CommandError> {
        let p = Vec3::from(origin);
        let n = Vec3::from(normal);

        if !p.is_finite() || !n.is_finite() {
            return Err(CommandError::Execution(
                "Section plane coordinates and normal must contain finite numbers".into(),
            ));
        }

        let n_norm = n.normalize_or_zero();
        if n_norm.length_squared() < 1e-4 {
            return Err(CommandError::Execution(
                "Section plane normal cannot be zero or degenerate".into(),
            ));
        }

        Ok(Self {
            anchor,
            origin,
            normal: n_norm.to_array(),
            offset: 0.0,
            flipped: false,
        })
    }

    /// Cria um plano de corte canônico XZ no nível Y especificado.
    pub fn xz(y: f32) -> Self {
        Self {
            anchor: SectionAnchor::World,
            origin: [0.0, y, 0.0],
            normal: [0.0, 1.0, 0.0],
            offset: 0.0,
            flipped: false,
        }
    }

    /// Cria um plano de corte canônico XY no nível Z especificado.
    pub fn xy(z: f32) -> Self {
        Self {
            anchor: SectionAnchor::World,
            origin: [0.0, 0.0, z],
            normal: [0.0, 0.0, 1.0],
            offset: 0.0,
            flipped: false,
        }
    }

    /// Cria um plano de corte canônico YZ no nível X especificado.
    pub fn yz(x: f32) -> Self {
        Self {
            anchor: SectionAnchor::World,
            origin: [x, 0.0, 0.0],
            normal: [1.0, 0.0, 0.0],
            offset: 0.0,
            flipped: false,
        }
    }

    /// Define o deslocamento numérico (offset) do plano ao longo de sua normal.
    pub fn with_offset(mut self, offset: f32) -> Self {
        if offset.is_finite() {
            self.offset = offset;
        }
        self
    }

    /// Inverte a direção da normal do plano de corte.
    pub fn flip(&mut self) {
        self.flipped = !self.flipped;
    }

    /// Resolve o ponto de passagem e a normal efetiva no espaço de mundo.
    pub fn effective_world_plane(
        &self,
        object_transform: Option<&ObjectTransform>,
    ) -> (Vec3, Vec3) {
        let raw_origin = Vec3::from(self.origin);
        let raw_normal = Vec3::from(self.normal).normalize_or_zero();

        let (base_p, mut base_n) = match (self.anchor, object_transform) {
            (SectionAnchor::Object, Some(tr)) => {
                let mat = tr.to_mat4();
                let p_world = mat.transform_point3(raw_origin);
                let n_world = mat.transform_vector3(raw_normal).normalize_or_zero();
                (p_world, n_world)
            }
            _ => (raw_origin, raw_normal),
        };

        if self.flipped {
            base_n = -base_n;
        }

        let effective_p = base_p + base_n * self.offset;
        (effective_p, base_n)
    }
}

/// Classificação da topologia do contorno de corte (VSP-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoopKind {
    /// Contorno fechado (curva poligonal com início coincidente com o fim).
    Closed,
    /// Contorno aberto (corte atingiu bordas da malha ou superfície não-fechada).
    Open,
}

/// Contorno poligonal planar resultante do corte geométrico (VSP-005).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SectionContour {
    pub points: Vec<[f32; 3]>,
    pub kind: LoopKind,
    pub is_hole: bool,
    pub perimeter: f32,
    pub area: Option<f32>,
}

/// Cap sintética de preenchimento de seção gerada para visualização na viewport (VSP-005).
///
/// **Invariante Crítico VSP-005:** `is_selectable` é estritamente `false`. A cap sintética
/// é uma primitiva de apresentação visual e **nunca** pode ser selecionada, transformada ou
/// tratada como face autoral da malha por ferramentas de edição ou picking.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct SyntheticCap {
    pub triangles: Vec<[[f32; 3]; 3]>,
    pub is_selectable: bool,
}

/// Resultado detalhado da consulta analítica do Section Inspector (VSP-005).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SectionResult {
    pub asset_id: Uuid,
    pub plane: SectionPlane,
    pub contours: Vec<SectionContour>,
    pub open_contours_count: usize,
    pub closed_contours_count: usize,
    pub holes_count: usize,
    pub total_perimeter: f32,
    pub total_area: f32,
    pub synthetic_cap: Option<SyntheticCap>,
    pub quality: MeasurementQuality,
    pub summary: String,
}

// ============================================================================
// Comandos de Controle do Section Plane (VSP-005)
// ============================================================================

/// Comando para definir ou atualizar o plano de corte de seção ativo (VSP-005).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SetSectionPlaneCmd {
    pub plane: SectionPlane,
}

impl SetSectionPlaneCmd {
    pub fn new(plane: SectionPlane) -> Self {
        Self { plane }
    }
}

impl Command for SetSectionPlaneCmd {
    fn label(&self) -> &'static str {
        "Set Section Plane"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let p = Vec3::from(self.plane.origin);
        let n = Vec3::from(self.plane.normal);
        if !p.is_finite() || !n.is_finite() || !self.plane.offset.is_finite() {
            return Err(CommandError::Execution(
                "Section plane parameters must be finite numbers".into(),
            ));
        }
        state.editor.active_section = Some(self.plane.clone());
        Ok(())
    }
}

/// Comando para desativar e remover o plano de corte ativo (VSP-005).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ClearSectionPlaneCmd;

impl Command for ClearSectionPlaneCmd {
    fn label(&self) -> &'static str {
        "Clear Section Plane"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        state.editor.active_section = None;
        Ok(())
    }
}

/// Comando para inverter a orientação do plano de corte ativo (VSP-005).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FlipSectionPlaneCmd;

impl Command for FlipSectionPlaneCmd {
    fn label(&self) -> &'static str {
        "Flip Section Plane"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.editor.active_section.is_none() {
            return Err("No active section plane to flip");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let Some(plane) = state.editor.active_section.as_mut() else {
            return Err(CommandError::Execution("No active section plane".into()));
        };
        plane.flipped = !plane.flipped;
        Ok(())
    }
}

/// Comando para ajustar o offset do plano de corte ativo numericamente (VSP-005).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OffsetSectionPlaneCmd {
    pub delta: f32,
}

impl OffsetSectionPlaneCmd {
    pub fn new(delta: f32) -> Self {
        Self { delta }
    }
}

impl Command for OffsetSectionPlaneCmd {
    fn label(&self) -> &'static str {
        "Offset Section Plane"
    }

    fn is_destructive(&self) -> bool {
        false
    }

    fn freezes_active_primitive(&self) -> bool {
        false
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::NONE
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if state.editor.active_section.is_none() {
            return Err("No active section plane to offset");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if !self.delta.is_finite() {
            return Err(CommandError::Execution(
                "Offset delta must be a finite number".into(),
            ));
        }
        let Some(plane) = state.editor.active_section.as_mut() else {
            return Err(CommandError::Execution("No active section plane".into()));
        };
        plane.offset += self.delta;
        Ok(())
    }
}

// ============================================================================
// Tipos Principais — Thickness Probe (VSP-005)
// ============================================================================

/// Tipo de parede identificado pela sonda de espessura (VSP-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WallType {
    /// Parede com superfícies opostas paralelas / coplanares com normais contrárias.
    ParallelWall,
    /// Parede com superfície oposta inclinada / oblíqua.
    ObliqueWall,
    /// Nenhuma parede oposta encontrada (malha aberta ou raio divergente).
    NoOpposingWall,
}

/// Parâmetros da consulta da sonda de espessura direcional (VSP-005).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ThicknessProbeQuery {
    pub asset_id: Uuid,
    pub start_point: [f32; 3],
    /// Direção do raio da sonda. Se `None`, usa o inverso da normal da superfície no ponto.
    pub direction: Option<[f32; 3]>,
    pub max_distance: f32,
}

/// Resultado da medição da sonda de espessura direcional (VSP-005).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ThicknessProbeResult {
    pub asset_id: Uuid,
    pub start_point: [f32; 3],
    pub exit_point: Option<[f32; 3]>,
    pub direction: [f32; 3],
    pub thickness: Option<f32>,
    pub wall_type: WallType,
    pub entry_normal: Option<[f32; 3]>,
    pub exit_normal: Option<[f32; 3]>,
    pub quality: MeasurementQuality,
    pub summary: String,
}

// ============================================================================
// Tipos Principais — Organic Profiles (VSP-005)
// ============================================================================

/// Estação de referência posicional e orientacional ao longo de uma linha de centro (VSP-005).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProfileStation {
    pub index: usize,
    pub position: [f32; 3],
    pub tangent: [f32; 3],
    pub normal: [f32; 3],
    pub binormal: [f32; 3],
    pub radius: f32,
    pub curvature: f32,
}

/// Ramificação explícita de perfil orgânico (bifurcação tratada com segmentos separados).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StationBranch {
    pub branch_id: usize,
    pub parent_station_index: usize,
    pub stations: Vec<ProfileStation>,
}

/// Linha de centro de perfil orgânico com estações transportadas via Bishop Frame / RMF.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OrganicCenterline {
    pub trunk_stations: Vec<ProfileStation>,
    pub branches: Vec<StationBranch>,
}

// ============================================================================
// Inspetor e Kernels de Análise Geométrica
// ============================================================================

/// Inspetor analítico puro de seções, espessuras e perfis orgânicos (VSP-005).
pub struct SectionInspector;

impl SectionInspector {
    /// Executa corte analítico não-destrutivo de uma malha por um plano arbitrário.
    ///
    /// Não altera a malha original nem o documento; retorna contornos poligonais fechados/abertos
    /// e malha sintética de preenchimento de cap opcional para exibição na viewport.
    pub fn intersect_mesh_plane(
        asset_id: Uuid,
        mesh: &Mesh,
        transform: Option<&ObjectTransform>,
        plane: &SectionPlane,
    ) -> SectionResult {
        let (world_p, world_n) = plane.effective_world_plane(transform);

        if mesh.verts.is_empty() || mesh.faces.is_empty() {
            return SectionResult {
                asset_id,
                plane: plane.clone(),
                contours: Vec::new(),
                open_contours_count: 0,
                closed_contours_count: 0,
                holes_count: 0,
                total_perimeter: 0.0,
                total_area: 0.0,
                synthetic_cap: None,
                quality: MeasurementQuality::Exact,
                summary: "Empty mesh: 0 contours found".into(),
            };
        }

        // Transforma o plano para o espaço local da malha para preservar precisão absoluta
        let world_mat = transform
            .map(|t| t.to_mat4())
            .unwrap_or(glam::Mat4::IDENTITY);
        let inv_mat = world_mat.inverse();

        let local_p = inv_mat.transform_point3(world_p);
        let local_n = inv_mat.transform_vector3(world_n).normalize_or_zero();

        if local_n.length_squared() < 1e-4 {
            return SectionResult {
                asset_id,
                plane: plane.clone(),
                contours: Vec::new(),
                open_contours_count: 0,
                closed_contours_count: 0,
                holes_count: 0,
                total_perimeter: 0.0,
                total_area: 0.0,
                synthetic_cap: None,
                quality: MeasurementQuality::Unavailable,
                summary: "Degenerate plane transformation".into(),
            };
        }

        // 1. Interseção analítica de cada triângulo da malha com o plano local
        let mut raw_segments: Vec<(Vec3, Vec3)> = Vec::new();

        for face in &mesh.faces {
            if face.verts.len() < 3 {
                continue;
            }
            let Some(&idx0) = face.verts.first() else {
                continue;
            };
            let Some(v0_vert) = mesh.verts.get(idx0 as usize) else {
                continue;
            };
            let v0 = v0_vert.vec();

            for i in 1..face.verts.len().saturating_sub(1) {
                let Some(&idx1) = face.verts.get(i) else {
                    continue;
                };
                let Some(&idx2) = face.verts.get(i + 1) else {
                    continue;
                };
                let Some(v1_vert) = mesh.verts.get(idx1 as usize) else {
                    continue;
                };
                let Some(v2_vert) = mesh.verts.get(idx2 as usize) else {
                    continue;
                };
                let v1 = v1_vert.vec();
                let v2 = v2_vert.vec();

                if let Some((p_a, p_b)) = intersect_triangle_plane(v0, v1, v2, local_p, local_n)
                    && (p_b - p_a).length_squared() > 1e-8
                {
                    raw_segments.push((p_a, p_b));
                }
            }
        }

        // 2. Encadeamento dos segmentos em loops de contorno fechados e abertos
        let local_contours = chain_segments_into_contours(raw_segments);

        // 3. Conversão de coordenadas para o mundo e cálculo métrico
        let mut world_contours = Vec::with_capacity(local_contours.len());
        let mut open_count = 0usize;
        let mut closed_count = 0usize;
        let mut hole_count = 0usize;
        let mut total_perimeter = 0.0f32;
        let mut total_area = 0.0f32;

        // Base 2D no plano para cálculo de área e orientação
        let (plane_u, plane_v) = compute_plane_basis(world_n);

        for (local_pts, kind) in local_contours {
            let world_pts: Vec<[f32; 3]> = local_pts
                .iter()
                .map(|p| world_mat.transform_point3(*p).to_array())
                .collect();

            // Perímetro
            let mut perimeter = 0.0f32;
            for i in 0..world_pts.len().saturating_sub(1) {
                let Some(&p0) = world_pts.get(i) else {
                    continue;
                };
                let Some(&p1) = world_pts.get(i + 1) else {
                    continue;
                };
                perimeter += (Vec3::from(p1) - Vec3::from(p0)).length();
            }
            if kind == LoopKind::Closed
                && world_pts.len() >= 3
                && let (Some(&first), Some(&last)) = (world_pts.first(), world_pts.last())
            {
                perimeter += (Vec3::from(first) - Vec3::from(last)).length();
            }

            // Área em 2D se for fechado
            let area = if kind == LoopKind::Closed && world_pts.len() >= 3 {
                let mut pts_2d = Vec::with_capacity(world_pts.len());
                for pt in &world_pts {
                    let rel = Vec3::from(*pt) - world_p;
                    pts_2d.push((rel.dot(plane_u), rel.dot(plane_v)));
                }
                let a = compute_polygon_area_2d(&pts_2d);
                Some(a.abs())
            } else {
                None
            };

            match kind {
                LoopKind::Closed => closed_count += 1,
                LoopKind::Open => open_count += 1,
            }

            total_perimeter += perimeter;
            if let Some(a) = area {
                total_area += a;
            }

            world_contours.push(SectionContour {
                points: world_pts,
                kind,
                is_hole: false,
                perimeter,
                area,
            });
        }

        // 4. Detecção de furos coplanares: contorno fechado contido dentro de outro fechado maior
        if closed_count > 1 {
            let n_cnt = world_contours.len();
            for i in 0..n_cnt {
                let Some(c_i) = world_contours.get(i) else {
                    continue;
                };
                if c_i.kind != LoopKind::Closed {
                    continue;
                }
                let Some(&first_pt_i) = c_i.points.first() else {
                    continue;
                };
                let rel_i = Vec3::from(first_pt_i) - world_p;
                let test_pt = (rel_i.dot(plane_u), rel_i.dot(plane_v));
                let area_i = c_i.area.unwrap_or(0.0);

                for j in 0..n_cnt {
                    if i == j {
                        continue;
                    }
                    let Some(c_j) = world_contours.get(j) else {
                        continue;
                    };
                    if c_j.kind != LoopKind::Closed {
                        continue;
                    }
                    let area_j = c_j.area.unwrap_or(0.0);

                    // Só pode ser furo de um polígono com área maior
                    if area_i < area_j {
                        let poly_j: Vec<(f32, f32)> = c_j
                            .points
                            .iter()
                            .map(|p| {
                                let rel = Vec3::from(*p) - world_p;
                                (rel.dot(plane_u), rel.dot(plane_v))
                            })
                            .collect();

                        if point_in_polygon_2d(test_pt, &poly_j) {
                            if let Some(target) = world_contours.get_mut(i) {
                                target.is_hole = true;
                            }
                            hole_count += 1;
                            break;
                        }
                    }
                }
            }
        }

        // 5. Geração de cap sintética opcional (não selecionável) para visualização
        let mut cap_triangles = Vec::new();
        for contour in &world_contours {
            if contour.kind == LoopKind::Closed && !contour.is_hole && contour.points.len() >= 3 {
                // Triangulação em leque simples a partir do primeiro ponto
                if let Some(&v0) = contour.points.first() {
                    for k in 1..contour.points.len().saturating_sub(1) {
                        if let (Some(&vk), Some(&v_next)) =
                            (contour.points.get(k), contour.points.get(k + 1))
                        {
                            cap_triangles.push([v0, vk, v_next]);
                        }
                    }
                }
            }
        }

        let synthetic_cap = if !cap_triangles.is_empty() {
            Some(SyntheticCap {
                triangles: cap_triangles,
                is_selectable: false, // Invariante VSP-005: sempre falso!
            })
        } else {
            None
        };

        let summary = format!(
            "Section: {} closed loops ({} holes), {} open paths | Perimeter: {:.3}, Area: {:.3}",
            closed_count, hole_count, open_count, total_perimeter, total_area
        );

        SectionResult {
            asset_id,
            plane: plane.clone(),
            contours: world_contours,
            open_contours_count: open_count,
            closed_contours_count: closed_count,
            holes_count: hole_count,
            total_perimeter,
            total_area,
            synthetic_cap,
            quality: MeasurementQuality::Exact,
            summary,
        }
    }

    /// Executa uma sonda direcional de espessura de parede sobre um ativo.
    pub fn probe_thickness(
        asset_id: Uuid,
        mesh: &Mesh,
        transform: Option<&ObjectTransform>,
        query: &ThicknessProbeQuery,
    ) -> ThicknessProbeResult {
        let world_mat = transform
            .map(|t| t.to_mat4())
            .unwrap_or(glam::Mat4::IDENTITY);
        let inv_mat = world_mat.inverse();

        let start_world = Vec3::from(query.start_point);
        let start_local = inv_mat.transform_point3(start_world);

        // Se direção não foi especificada, calcula a normal média dos triângulos próximos
        let dir_local = if let Some(d) = query.direction {
            inv_mat.transform_vector3(Vec3::from(d)).normalize_or_zero()
        } else {
            // Encontra o triângulo mais próximo para obter a normal e apontar para o interior
            let mut nearest_normal = Vec3::Y;
            let mut min_d = f32::INFINITY;
            for face in &mesh.faces {
                if face.verts.len() < 3 {
                    continue;
                }
                let Some(&idx0) = face.verts.first() else {
                    continue;
                };
                let Some(&idx1) = face.verts.get(1) else {
                    continue;
                };
                let Some(&idx2) = face.verts.get(2) else {
                    continue;
                };
                let Some(v0_vert) = mesh.verts.get(idx0 as usize) else {
                    continue;
                };
                let Some(v1_vert) = mesh.verts.get(idx1 as usize) else {
                    continue;
                };
                let Some(v2_vert) = mesh.verts.get(idx2 as usize) else {
                    continue;
                };
                let v0 = v0_vert.vec();
                let v1 = v1_vert.vec();
                let v2 = v2_vert.vec();

                let center = (v0 + v1 + v2) / 3.0;
                let d = (center - start_local).length_squared();
                if d < min_d {
                    min_d = d;
                    nearest_normal = (v1 - v0).cross(v2 - v0).normalize_or_zero();
                }
            }
            -nearest_normal
        };

        if dir_local.length_squared() < 1e-4 {
            return ThicknessProbeResult {
                asset_id,
                start_point: query.start_point,
                exit_point: None,
                direction: [0.0, 0.0, 0.0],
                thickness: None,
                wall_type: WallType::NoOpposingWall,
                entry_normal: None,
                exit_normal: None,
                quality: MeasurementQuality::Unavailable,
                summary: "Invalid probe direction".into(),
            };
        }

        let dir_world = world_mat.transform_vector3(dir_local).normalize_or_zero();

        // Traçado de raio contra todos os triângulos da malha
        let mut min_t = f32::INFINITY;
        let mut exit_n_local = Vec3::ZERO;
        let eps = 1e-4;

        for face in &mesh.faces {
            if face.verts.len() < 3 {
                continue;
            }
            let Some(&idx0) = face.verts.first() else {
                continue;
            };
            let Some(v0_vert) = mesh.verts.get(idx0 as usize) else {
                continue;
            };
            let v0 = v0_vert.vec();

            for i in 1..face.verts.len().saturating_sub(1) {
                let Some(&idx1) = face.verts.get(i) else {
                    continue;
                };
                let Some(&idx2) = face.verts.get(i + 1) else {
                    continue;
                };
                let Some(v1_vert) = mesh.verts.get(idx1 as usize) else {
                    continue;
                };
                let Some(v2_vert) = mesh.verts.get(idx2 as usize) else {
                    continue;
                };
                let v1 = v1_vert.vec();
                let v2 = v2_vert.vec();

                let tri_n = (v1 - v0).cross(v2 - v0).normalize_or_zero();

                if let Some(t) = ray_triangle_intersection(start_local, dir_local, v0, v1, v2)
                    && t > eps
                    && t < min_t
                    && t <= query.max_distance
                {
                    min_t = t;
                    exit_n_local = tri_n;
                }
            }
        }

        if min_t.is_finite() {
            let exit_local = start_local + dir_local * min_t;
            let exit_world = world_mat.transform_point3(exit_local);
            let exit_n_world = world_mat
                .transform_vector3(exit_n_local)
                .normalize_or_zero();
            let entry_n_world = -dir_world;

            let thickness = (exit_world - start_world).length();

            // Avalia paralelismo da parede oposta
            let dot_norm = entry_n_world.dot(exit_n_world);
            let (wall_type, quality) = if dot_norm < -0.9 {
                (WallType::ParallelWall, MeasurementQuality::Exact)
            } else {
                (WallType::ObliqueWall, MeasurementQuality::Approximate)
            };

            let summary = format!(
                "Thickness: {:.3} ({:?}, quality: {:?})",
                thickness, wall_type, quality
            );

            ThicknessProbeResult {
                asset_id,
                start_point: query.start_point,
                exit_point: Some(exit_world.to_array()),
                direction: dir_world.to_array(),
                thickness: Some(thickness),
                wall_type,
                entry_normal: Some(entry_n_world.to_array()),
                exit_normal: Some(exit_n_world.to_array()),
                quality,
                summary,
            }
        } else {
            ThicknessProbeResult {
                asset_id,
                start_point: query.start_point,
                exit_point: None,
                direction: dir_world.to_array(),
                thickness: None,
                wall_type: WallType::NoOpposingWall,
                entry_normal: Some((-dir_world).to_array()),
                exit_normal: None,
                quality: MeasurementQuality::Unavailable,
                summary:
                    "No opposing wall found within maximum distance (open mesh or divergent ray)"
                        .into(),
            }
        }
    }

    /// Constrói um perfil orgânico com estações transportadas via Bishop Frame / Parallel Transport.
    ///
    /// Garante matematicamente **zero flips e sem inversão de normais** ao longo de qualquer curva 3D,
    /// tratando bifurcações como segmentos independentes sem inferência anatômica mágica (VSP-005).
    pub fn build_organic_centerline(
        trunk_points: &[[f32; 3]],
        branches: &[(usize, Vec<[f32; 3]>)], // (parent_station_index, branch_points)
    ) -> Result<OrganicCenterline, CommandError> {
        if trunk_points.len() < 2 {
            return Err(CommandError::Execution(
                "Centerline requires at least 2 station points".into(),
            ));
        }

        let trunk_stations = compute_parallel_transport_stations(trunk_points, None)?;

        let mut computed_branches = Vec::with_capacity(branches.len());
        for (branch_id, (parent_idx, branch_pts)) in branches.iter().enumerate() {
            if *parent_idx >= trunk_stations.len() {
                return Err(CommandError::Execution(format!(
                    "Branch parent station index {parent_idx} out of range ({})",
                    trunk_stations.len()
                )));
            }
            if branch_pts.len() < 2 {
                continue;
            }
            let initial_frame = trunk_stations.get(*parent_idx);
            let branch_stations = compute_parallel_transport_stations(branch_pts, initial_frame)?;
            computed_branches.push(StationBranch {
                branch_id,
                parent_station_index: *parent_idx,
                stations: branch_stations,
            });
        }

        Ok(OrganicCenterline {
            trunk_stations,
            branches: computed_branches,
        })
    }
}

// ============================================================================
// Helpers Matemáticos Internos
// ============================================================================

/// Interseção analítica entre um triângulo e um plano dado por um ponto e normal unitária.
fn intersect_triangle_plane(
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    p0: Vec3,
    n: Vec3,
) -> Option<(Vec3, Vec3)> {
    let d0 = (v0 - p0).dot(n);
    let d1 = (v1 - p0).dot(n);
    let d2 = (v2 - p0).dot(n);

    let eps = 1e-6;

    // Se todos estiverem estritamente do mesmo lado, não há corte
    if (d0 > eps && d1 > eps && d2 > eps) || (d0 < -eps && d1 < -eps && d2 < -eps) {
        return None;
    }

    let mut cut_points = Vec::with_capacity(3);

    let check_edge = |va: Vec3, vb: Vec3, da: f32, db: f32, cuts: &mut Vec<Vec3>| {
        if (da < -eps && db > eps) || (da > eps && db < -eps) {
            let t = -da / (db - da);
            cuts.push(va + t * (vb - va));
        } else if da.abs() <= eps && !cuts.iter().any(|p| (*p - va).length_squared() < 1e-8) {
            cuts.push(va);
        }
    };

    check_edge(v0, v1, d0, d1, &mut cut_points);
    check_edge(v1, v2, d1, d2, &mut cut_points);
    check_edge(v2, v0, d2, d0, &mut cut_points);

    if cut_points.len() >= 2 {
        if let (Some(&c0), Some(&c1)) = (cut_points.first(), cut_points.get(1)) {
            Some((c0, c1))
        } else {
            None
        }
    } else {
        None
    }
}

/// Encadeia uma coleção de segmentos desordenados em contornos abertos ou fechados.
fn chain_segments_into_contours(mut segments: Vec<(Vec3, Vec3)>) -> Vec<(Vec<Vec3>, LoopKind)> {
    let mut contours = Vec::new();
    let eps = 1e-4;

    while let Some((start_a, start_b)) = segments.pop() {
        let mut path = vec![start_a, start_b];
        let mut closed = false;

        // Tenta estender para frente a partir de path.last()
        while let Some(cur) = path.last().copied() {
            if path.len() >= 3
                && let Some(first) = path.first().copied()
                && (cur - first).length() < eps
            {
                closed = true;
                break;
            }

            let mut found_idx = None;
            let mut reverse = false;
            for (i, &(sa, sb)) in segments.iter().enumerate() {
                if (sa - cur).length() < eps {
                    found_idx = Some(i);
                    reverse = false;
                    break;
                } else if (sb - cur).length() < eps {
                    found_idx = Some(i);
                    reverse = true;
                    break;
                }
            }

            if let Some(idx) = found_idx {
                let (sa, sb) = segments.swap_remove(idx);
                let next_pt = if reverse { sa } else { sb };
                path.push(next_pt);
            } else {
                break;
            }
        }

        // Se não fechou para frente, estende para trás a partir de path[0]
        if !closed {
            while let Some(cur) = path.first().copied() {
                let mut found_idx = None;
                let mut reverse = false;
                for (i, &(sa, sb)) in segments.iter().enumerate() {
                    if (sb - cur).length() < eps {
                        found_idx = Some(i);
                        reverse = false;
                        break;
                    } else if (sa - cur).length() < eps {
                        found_idx = Some(i);
                        reverse = true;
                        break;
                    }
                }

                if let Some(idx) = found_idx {
                    let (sa, sb) = segments.swap_remove(idx);
                    let prev_pt = if reverse { sb } else { sa };
                    path.insert(0, prev_pt);
                } else {
                    break;
                }
            }

            if path.len() >= 3
                && let (Some(first), Some(last)) = (path.first(), path.last())
                && (*first - *last).length() < eps
            {
                closed = true;
                path.pop();
            }
        } else {
            path.pop();
        }

        if path.len() >= 2 {
            let kind = if closed {
                LoopKind::Closed
            } else {
                LoopKind::Open
            };
            contours.push((path, kind));
        }
    }

    contours
}

/// Constrói uma base ortonormal 2D (u, v) perpendicular a um vetor normal dado.
fn compute_plane_basis(normal: Vec3) -> (Vec3, Vec3) {
    let n = normal.normalize_or_zero();
    let ref_v = if n.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
    let u = n.cross(ref_v).normalize_or_zero();
    let v = n.cross(u).normalize_or_zero();
    (u, v)
}

/// Área com sinal de polígono 2D via fórmula de Gauss (Shoelace).
fn compute_polygon_area_2d(pts: &[(f32, f32)]) -> f32 {
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut area = 0.0;
    for i in 0..n {
        let Some(&(x1, y1)) = pts.get(i) else {
            continue;
        };
        let Some(&(x2, y2)) = pts.get((i + 1) % n) else {
            continue;
        };
        area += x1 * y2 - x2 * y1;
    }
    area * 0.5
}

/// Teste de inclusão ponto-em-polígono 2D (Ray-casting / Jordan curve theorem).
fn point_in_polygon_2d(pt: (f32, f32), poly: &[(f32, f32)]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    for i in 0..n {
        let Some(&(x1, y1)) = poly.get(i) else {
            continue;
        };
        let Some(&(x2, y2)) = poly.get((i + 1) % n) else {
            continue;
        };

        let intersects = ((y1 > pt.1) != (y2 > pt.1))
            && (pt.0 < (x2 - x1) * (pt.1 - y1) / (y2 - y1 + 1e-9) + x1);

        if intersects {
            inside = !inside;
        }
    }
    inside
}

/// Interseção raio-triângulo de Möller–Trumbore.
fn ray_triangle_intersection(orig: Vec3, dir: Vec3, v0: Vec3, v1: Vec3, v2: Vec3) -> Option<f32> {
    let eps = 1e-7;
    let edge1 = v1 - v0;
    let edge2 = v2 - v0;
    let pvec = dir.cross(edge2);
    let det = edge1.dot(pvec);

    if det.abs() < eps {
        return None;
    }

    let inv_det = 1.0 / det;
    let tvec = orig - v0;
    let u = tvec.dot(pvec) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }

    let qvec = tvec.cross(edge1);
    let v = dir.dot(qvec) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }

    let t = edge2.dot(qvec) * inv_det;
    if t > eps { Some(t) } else { None }
}

/// Calcula estações transportadas paralelamente (RMF / Bishop Frame) ao longo de uma sequência de pontos 3D.
fn compute_parallel_transport_stations(
    pts: &[[f32; 3]],
    initial_frame: Option<&ProfileStation>,
) -> Result<Vec<ProfileStation>, CommandError> {
    let mut clean_pts: Vec<Vec3> = Vec::with_capacity(pts.len());
    for &p in pts {
        let v = Vec3::from(p);
        if !v.is_finite() {
            continue;
        }
        if let Some(&last) = clean_pts.last()
            && (v - last).length_squared() < 1e-6
        {
            continue; // Pula pontos duplicados consecutivos
        }
        clean_pts.push(v);
    }

    if clean_pts.len() < 2 {
        return Err(CommandError::Execution(
            "Centerline requires at least 2 distinct finite station points".into(),
        ));
    }

    let n = clean_pts.len();
    let mut stations = Vec::with_capacity(n);

    // 1. Tangentes em cada estação
    let mut tangents = Vec::with_capacity(n);
    for i in 0..n {
        let t = if i == 0 {
            let p0 = clean_pts.first().copied().unwrap_or(Vec3::ZERO);
            let p1 = clean_pts.get(1).copied().unwrap_or(Vec3::ZERO);
            (p1 - p0).normalize_or_zero()
        } else if i == n - 1 {
            let pn1 = clean_pts.get(n - 1).copied().unwrap_or(Vec3::ZERO);
            let pn2 = clean_pts
                .get(n.saturating_sub(2))
                .copied()
                .unwrap_or(Vec3::ZERO);
            (pn1 - pn2).normalize_or_zero()
        } else {
            let p_prev = clean_pts
                .get(i.saturating_sub(1))
                .copied()
                .unwrap_or(Vec3::ZERO);
            let p_cur = clean_pts.get(i).copied().unwrap_or(Vec3::ZERO);
            let p_next = clean_pts.get(i + 1).copied().unwrap_or(Vec3::ZERO);
            ((p_next - p_cur).normalize_or_zero() + (p_cur - p_prev).normalize_or_zero())
                .normalize_or_zero()
        };
        tangents.push(t);
    }

    // 2. Frame inicial
    let (mut cur_norm, mut cur_binorm) = if let Some(init) = initial_frame {
        (Vec3::from(init.normal), Vec3::from(init.binormal))
    } else {
        let t0 = tangents.first().copied().unwrap_or(Vec3::Z);
        let ref_v = if t0.x.abs() < 0.9 { Vec3::X } else { Vec3::Y };
        let norm = t0.cross(ref_v).normalize_or_zero();
        let binorm = t0.cross(norm).normalize_or_zero();
        (norm, binorm)
    };

    // 3. Transporte paralelo (RMF) de estação a estação
    for i in 0..n {
        if i > 0 {
            let Some(&t_prev) = tangents.get(i - 1) else {
                continue;
            };
            let Some(&t_cur) = tangents.get(i) else {
                continue;
            };

            let rot_axis = t_prev.cross(t_cur);
            let rot_len = rot_axis.length();

            if rot_len > 1e-5 {
                let dot = t_prev.dot(t_cur).clamp(-1.0, 1.0);
                let angle = dot.acos();
                let axis = rot_axis / rot_len;

                // Rotação de Rodrigues para transportar a normal
                cur_norm = cur_norm * angle.cos()
                    + axis.cross(cur_norm) * angle.sin()
                    + axis * (axis.dot(cur_norm)) * (1.0 - angle.cos());
                cur_norm = cur_norm.normalize_or_zero();
                cur_binorm = t_cur.cross(cur_norm).normalize_or_zero();
            } else {
                cur_binorm = t_cur.cross(cur_norm).normalize_or_zero();
            }
        }

        // Estima curvatura
        let curvature = if i > 0 && i < n - 1 {
            let p_prev = clean_pts.get(i - 1).copied().unwrap_or(Vec3::ZERO);
            let p_next = clean_pts.get(i + 1).copied().unwrap_or(Vec3::ZERO);
            let seg_len = (p_next - p_prev).length() * 0.5;
            if seg_len > 1e-4 {
                let t_prev = tangents.get(i - 1).copied().unwrap_or(Vec3::ZERO);
                let t_next = tangents.get(i + 1).copied().unwrap_or(Vec3::ZERO);
                (t_next - t_prev).length() / seg_len
            } else {
                0.0
            }
        } else {
            0.0
        };

        let pt = clean_pts.get(i).copied().unwrap_or(Vec3::ZERO);
        let tan = tangents.get(i).copied().unwrap_or(Vec3::Z);

        stations.push(ProfileStation {
            index: i,
            position: pt.to_array(),
            tangent: tan.to_array(),
            normal: cur_norm.to_array(),
            binormal: cur_binorm.to_array(),
            radius: 1.0,
            curvature,
        });
    }

    Ok(stations)
}
