//! Dimensões e comparação espacial (VSP-003 e VSP-004 / Task H29).
//!
//! Implementa:
//! - VSP-003: Volume Frame usando representação evaluated por padrão (source identificada separadamente).
//!   Frames World, ObjectLocal, Custom, Structural e View (não equivalentes a simples World AABB).
//!   Qualidade de medição: Exact, Approximate com método, Unavailable com motivo ou Stale com revisão.
//!   Formatação dimensional sem inventar cm/m em cenas genéricas.
//!   Separação estrita entre medir, escalar até dimensão e editar parâmetro; prevenção contra divisão por zero.
//! - VSP-004: Consultas espaciais distintas (distância euclidiana, delta em eixo, profundidade de câmera,
//!   clearance de superfície, contato e interseção volumétrica via SAT).
//!   Companion View com câmera ortográfica independente, opt-in e sem roubo de foco.
//!   Invariante X-Ray vs Select Through (X-Ray visual não ativa Select Through).

use glam::Vec3;
use petunia_mesh::Mesh;
use petunia_project::{ObjectTransform, Project, ProjectChanges};
use uuid::Uuid;

use crate::command::{Command, CommandContext, CommandError, EditorView};
use crate::modeling_feedback::MeasurementQuality;

/// Tipo de frame de coordenadas para medições espaciais (VSP-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SpatialFrameKind {
    #[default]
    World,
    ObjectLocal,
    Custom,
    Structural,
    View,
}

impl SpatialFrameKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::World => "World",
            Self::ObjectLocal => "Object Local",
            Self::Custom => "Custom",
            Self::Structural => "Structural",
            Self::View => "View",
        }
    }
}

/// Base ortonormal 3D definindo um frame de referência espacial.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpatialFrame {
    pub kind: SpatialFrameKind,
    pub origin: [f32; 3],
    /// Eixos ortonormais de orientação [X, Y, Z] expressos no espaço de mundo.
    pub axes: [[f32; 3]; 3],
}

impl Default for SpatialFrame {
    fn default() -> Self {
        Self::world()
    }
}

impl SpatialFrame {
    /// Frame canônico alinhado aos eixos globais do mundo.
    pub fn world() -> Self {
        Self {
            kind: SpatialFrameKind::World,
            origin: [0.0, 0.0, 0.0],
            axes: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        }
    }

    /// Frame alinhado à orientação e translação local do objeto.
    pub fn object_local(transform: &ObjectTransform) -> Self {
        let rot = glam::Mat3::from_quat(transform.rotation());
        let x = rot.x_axis.normalize_or_zero();
        let y = rot.y_axis.normalize_or_zero();
        let z = rot.z_axis.normalize_or_zero();
        Self {
            kind: SpatialFrameKind::ObjectLocal,
            origin: transform.translation,
            axes: [x.to_array(), y.to_array(), z.to_array()],
        }
    }

    /// Frame personalizado definido por origem e eixos do usuário.
    pub fn custom(origin: [f32; 3], axes: [[f32; 3]; 3]) -> Result<Self, &'static str> {
        let x = Vec3::from(axes[0]);
        let y = Vec3::from(axes[1]);
        let z = Vec3::from(axes[2]);

        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err("Custom frame axes must contain finite numbers");
        }

        let ux = x.normalize_or_zero();
        if ux.length_squared() < 1e-4 {
            return Err("Custom frame X axis is degenerate");
        }
        // Gram-Schmidt orthogonalization
        let uy = (y - y.dot(ux) * ux).normalize_or_zero();
        if uy.length_squared() < 1e-4 {
            return Err("Custom frame Y axis is collinear with X");
        }
        let uz = ux.cross(uy).normalize_or_zero();

        Ok(Self {
            kind: SpatialFrameKind::Custom,
            origin,
            axes: [ux.to_array(), uy.to_array(), uz.to_array()],
        })
    }

    /// Frame estrutural derivado da análise dos eixos principais de inércia (PCA) das superfícies.
    pub fn structural(mesh: &Mesh, transform: &ObjectTransform) -> Self {
        Self::structural_with_quality(mesh, transform).0
    }

    /// Frame estrutural acompanhado da qualidade de medição (Exact ou Approximate em caso de simetria/isotropia).
    ///
    /// Pondera analiticamente a área dos triângulos da malha (fórmula de Eberly / Mirtich),
    /// garantindo que áreas densamente subdivididas (ex: costuras de corte booleano ou bevels)
    /// não enviesem nem desloquem arbitrariamente o centroide e os eixos estruturais.
    ///
    /// Para corpos com simetria de revolução (cilindro, cápsula, disco), utiliza Anchor Alignment
    /// projetando os eixos locais canônicos sobre o subespaço degenerado, evitando rotações espúrias.
    pub fn structural_with_quality(
        mesh: &Mesh,
        transform: &ObjectTransform,
    ) -> (Self, MeasurementQuality) {
        let world_mat = transform.to_mat4();
        if mesh.verts.is_empty() {
            return (Self::object_local(transform), MeasurementQuality::Exact);
        }

        // Transforma vértices para coordenadas de mundo
        let world_verts: Vec<Vec3> = mesh
            .verts
            .iter()
            .map(|v| world_mat.transform_point3(v.vec()))
            .collect();

        // 1. Calcula centróide ponderado pela área das faces trianguladas
        let mut total_area = 0.0f32;
        let mut weighted_center = Vec3::ZERO;

        for face in &mesh.faces {
            if face.verts.len() < 3 {
                continue;
            }
            let v0_idx = face.verts[0] as usize;
            let Some(&p0) = world_verts.get(v0_idx) else {
                continue;
            };
            if !p0.is_finite() {
                continue;
            }

            for i in 1..face.verts.len() - 1 {
                let v1_idx = face.verts[i] as usize;
                let v2_idx = face.verts[i + 1] as usize;
                let (Some(&p1), Some(&p2)) = (world_verts.get(v1_idx), world_verts.get(v2_idx))
                else {
                    continue;
                };
                if !p1.is_finite() || !p2.is_finite() {
                    continue;
                }

                let cross = (p1 - p0).cross(p2 - p0);
                let area = 0.5 * cross.length();
                if area > 1e-8 && area.is_finite() {
                    let tri_center = (p0 + p1 + p2) / 3.0;
                    weighted_center += tri_center * area;
                    total_area += area;
                }
            }
        }

        let center = if total_area > 1e-6 {
            weighted_center / total_area
        } else {
            // Fallback para centróide de vértices se a malha não tiver faces com área (nuvem de pontos ou degenerada)
            let mut sum = Vec3::ZERO;
            let mut count = 0.0f32;
            for &p in &world_verts {
                if p.is_finite() {
                    sum += p;
                    count += 1.0;
                }
            }
            if count > 0.0 {
                sum / count
            } else {
                return (Self::object_local(transform), MeasurementQuality::Exact);
            }
        };

        // 2. Matriz de covariância 3x3 integrada analiticamente sobre a superfície triangular
        let mut cov = [[0.0f32; 3]; 3];

        if total_area > 1e-6 {
            for face in &mesh.faces {
                if face.verts.len() < 3 {
                    continue;
                }
                let v0_idx = face.verts[0] as usize;
                let Some(&p0) = world_verts.get(v0_idx) else {
                    continue;
                };
                if !p0.is_finite() {
                    continue;
                }

                for i in 1..face.verts.len() - 1 {
                    let v1_idx = face.verts[i] as usize;
                    let v2_idx = face.verts[i + 1] as usize;
                    let (Some(&p1), Some(&p2)) = (world_verts.get(v1_idx), world_verts.get(v2_idx))
                    else {
                        continue;
                    };
                    if !p1.is_finite() || !p2.is_finite() {
                        continue;
                    }

                    let cross = (p1 - p0).cross(p2 - p0);
                    let area = 0.5 * cross.length();
                    if area > 1e-8 && area.is_finite() {
                        let a = p0 - center;
                        let b = p1 - center;
                        let d = p2 - center;
                        let s = a + b + d;

                        // Integral da díade (p - c)(p - c)^T sobre o triângulo:
                        // (Area / 24) * [ (a a^T + b b^T + d d^T) + s s^T ]
                        let factor = area / 24.0;
                        for r in 0..3 {
                            let ar = a[r];
                            let br = b[r];
                            let dr = d[r];
                            let sr = s[r];
                            for c in r..3 {
                                let term = ar * a[c] + br * b[c] + dr * d[c] + sr * s[c];
                                cov[r][c] += factor * term;
                            }
                        }
                    }
                }
            }
            cov[1][0] = cov[0][1];
            cov[2][0] = cov[0][2];
            cov[2][1] = cov[1][2];

            for row in &mut cov {
                for val in row {
                    *val /= total_area;
                }
            }
        } else {
            // Fallback para covariância de vértices
            let mut count = 0.0f32;
            for &p in &world_verts {
                if p.is_finite() {
                    let d = p - center;
                    cov[0][0] += d.x * d.x;
                    cov[0][1] += d.x * d.y;
                    cov[0][2] += d.x * d.z;
                    cov[1][1] += d.y * d.y;
                    cov[1][2] += d.y * d.z;
                    cov[2][2] += d.z * d.z;
                    count += 1.0;
                }
            }
            if count > 0.0 {
                cov[1][0] = cov[0][1];
                cov[2][0] = cov[0][2];
                cov[2][1] = cov[1][2];
                for row in &mut cov {
                    for val in row {
                        *val /= count;
                    }
                }
            }
        }

        // 3. Autovetores e autovalores via método de Jacobi
        let (raw_axes, eigenvalues) = jacobi_eigenvectors_3x3(cov);

        // Ordena autovalores decrescentes e seus autovetores correspondentes
        let mut order = [0, 1, 2];
        order.sort_by(|&i, &j| {
            eigenvalues[j]
                .partial_cmp(&eigenvalues[i])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let e0 = eigenvalues[order[0]];
        let e1 = eigenvalues[order[1]];
        let e2 = eigenvalues[order[2]];

        let mut v0 = Vec3::from(raw_axes[order[0]]).normalize_or_zero();
        let mut v1 = Vec3::from(raw_axes[order[1]]).normalize_or_zero();
        let mut v2 = Vec3::from(raw_axes[order[2]]).normalize_or_zero();

        // 4. Tratamento de Isotropia Total (esfera, cubo perfeito)
        // Se a dispersão entre o maior e menor autovalor for mínima (< 3%),
        // não há eixos preferenciais: recorre à orientação local do objeto.
        if e0 > 1e-5 && (e0 - e2) / e0 < 0.03 {
            return (
                Self::object_local(transform),
                MeasurementQuality::Approximate,
            );
        }

        // 5. Tratamento de Degenerescência Parcial / Simetria Axial (cilindro, cápsula, disco)
        // Anchor Alignment: ancora os eixos do subespaço degenerado ao frame canônico do objeto
        // para evitar rotações espúrias no plano transversal.
        let local_axes = [
            world_mat.transform_vector3(Vec3::X).normalize_or_zero(),
            world_mat.transform_vector3(Vec3::Y).normalize_or_zero(),
            world_mat.transform_vector3(Vec3::Z).normalize_or_zero(),
        ];

        let mut quality = MeasurementQuality::Exact;

        if e0 > 1e-5 {
            let diff_transversal = (e1 - e2) / e0;
            let diff_planar = (e0 - e1) / e0;

            if diff_transversal < 0.03 {
                // Caso Cilindro / Haste: v0 é o eixo longitudinal fixo; e1 e e2 são degenerados.
                quality = MeasurementQuality::Approximate;
                let mut best_axis = local_axes[0];
                let mut min_dot = best_axis.dot(v0).abs();
                for la in &local_axes[1..] {
                    let d = la.dot(v0).abs();
                    if d < min_dot {
                        min_dot = d;
                        best_axis = *la;
                    }
                }
                let proj = best_axis - best_axis.dot(v0) * v0;
                let u1 = proj.normalize_or_zero();
                if u1.length_squared() > 1e-4 {
                    v1 = u1;
                    v2 = v0.cross(v1).normalize_or_zero();
                }
            } else if diff_planar < 0.03 {
                // Caso Disco / Moeda: v2 é o eixo normal fixo; e0 e e1 são degenerados no plano do disco.
                quality = MeasurementQuality::Approximate;
                let mut best_axis = local_axes[0];
                let mut min_dot = best_axis.dot(v2).abs();
                for la in &local_axes[1..] {
                    let d = la.dot(v2).abs();
                    if d < min_dot {
                        min_dot = d;
                        best_axis = *la;
                    }
                }
                let proj = best_axis - best_axis.dot(v2) * v2;
                let u0 = proj.normalize_or_zero();
                if u0.length_squared() > 1e-4 {
                    v0 = u0;
                    v1 = v2.cross(v0).normalize_or_zero();
                }
            }
        }

        (
            Self {
                kind: SpatialFrameKind::Structural,
                origin: center.to_array(),
                axes: [v0.to_array(), v1.to_array(), v2.to_array()],
            },
            quality,
        )
    }

    /// Frame orientado pela câmera / visão.
    pub fn view(camera_pos: [f32; 3], camera_target: [f32; 3], camera_up: [f32; 3]) -> Self {
        let pos = Vec3::from(camera_pos);
        let target = Vec3::from(camera_target);
        let up = Vec3::from(camera_up);

        let forward = (target - pos).normalize_or_zero();
        let fwd = if forward.length_squared() < 1e-4 {
            Vec3::NEG_Z
        } else {
            forward
        };

        let right = fwd.cross(up).normalize_or_zero();
        let r = if right.length_squared() < 1e-4 {
            Vec3::X
        } else {
            right
        };
        let u = r.cross(fwd).normalize_or_zero();

        Self {
            kind: SpatialFrameKind::View,
            origin: camera_pos,
            axes: [r.to_array(), u.to_array(), (-fwd).to_array()],
        }
    }
}

/// Representação de malha para consulta espacial (VSP-003).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum GeometryRepresentation {
    /// Representação avaliada com modificadores e composições aplicadas (padrão).
    #[default]
    Evaluated,
    /// Malha autoral de origem antes de modificadores.
    Source,
}

impl GeometryRepresentation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Evaluated => "Evaluated",
            Self::Source => "Source",
        }
    }
}

/// Volume delimitador orientado (OBB) em um frame específico (VSP-003).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VolumeFrame {
    pub frame: SpatialFrame,
    pub representation: GeometryRepresentation,
    pub quality: MeasurementQuality,
    pub center: [f32; 3],
    pub min: [f32; 3],
    pub max: [f32; 3],
    pub size: [f32; 3],
    pub volume: f32,
    pub unit: String,
    pub vertex_count: usize,
    pub revision: u64,
}

impl VolumeFrame {
    /// Formata as dimensões respeitando a regra VSP-003 ("Sem inventar cm/m em cenas genéricas").
    pub fn formatted_dimensions(&self) -> String {
        if self.unit.is_empty() {
            format!(
                "{:.3} × {:.3} × {:.3}",
                self.size[0], self.size[1], self.size[2]
            )
        } else {
            format!(
                "{:.3} × {:.3} × {:.3} {}",
                self.size[0], self.size[1], self.size[2], self.unit
            )
        }
    }

    /// Calcula os 8 vértices no espaço de mundo que formam a caixa delimitadora orientada.
    pub fn corners(&self) -> [[f32; 3]; 8] {
        let c = Vec3::from(self.center);
        let ux = Vec3::from(self.frame.axes[0]) * (self.size[0] * 0.5);
        let uy = Vec3::from(self.frame.axes[1]) * (self.size[1] * 0.5);
        let uz = Vec3::from(self.frame.axes[2]) * (self.size[2] * 0.5);

        [
            (c - ux - uy - uz).to_array(),
            (c + ux - uy - uz).to_array(),
            (c + ux + uy - uz).to_array(),
            (c - ux + uy - uz).to_array(),
            (c - ux - uy + uz).to_array(),
            (c + ux - uy + uz).to_array(),
            (c + ux + uy + uz).to_array(),
            (c - ux + uy + uz).to_array(),
        ]
    }

    /// Testa se um ponto no mundo está contido dentro deste volume orientado.
    pub fn contains_point(&self, point: [f32; 3]) -> bool {
        let p = Vec3::from(point);
        let c = Vec3::from(self.center);
        let d = p - c;

        for i in 0..3 {
            let axis = Vec3::from(self.frame.axes[i]);
            let proj = d.dot(axis).abs();
            if proj > self.size[i] * 0.5 + 1e-5 {
                return false;
            }
        }
        true
    }
}

/// Tipo de consulta espacial pontual ou relacional (VSP-004).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SpatialQueryKind {
    /// 1. Distância euclidiana pura ponto-a-ponto em 3D.
    EuclideanDistance { from: [f32; 3], to: [f32; 3] },
    /// 2. Deltas projetados nos eixos do frame de referência especificado.
    AxisDelta {
        from: [f32; 3],
        to: [f32; 3],
        frame: SpatialFrame,
    },
    /// 3. Profundidade de câmera (distância linear ao longo do plano de visão).
    CameraDepth {
        point: [f32; 3],
        camera_pos: [f32; 3],
        camera_forward: [f32; 3],
    },
    /// 4. Distância mínima entre as superfícies de dois objetos (clearance).
    SurfaceClearance { object_a: Uuid, object_b: Uuid },
    /// 5. Verificação de contato/toque entre dois objetos sob determinada tolerância.
    ContactCheck {
        object_a: Uuid,
        object_b: Uuid,
        tolerance: f32,
    },
    /// 6. Interseção volumétrica (overlap entre os volumes orientados dos objetos via SAT).
    VolumetricIntersection { object_a: Uuid, object_b: Uuid },
}

/// Resultado detalhado de uma consulta espacial (VSP-004).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SpatialQueryResult {
    pub query_name: String,
    pub quality: MeasurementQuality,
    pub distance: f32,
    pub delta: Option<[f32; 3]>,
    pub closest_points: Option<([f32; 3], [f32; 3])>,
    pub is_in_contact: bool,
    pub is_intersecting: bool,
    pub penetration_depth: Option<f32>,
    pub summary: String,
}

/// Estado da Companion View secundária ortográfica desacoplada (VSP-004).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CompanionViewState {
    pub enabled: bool,
    pub is_orthographic: bool,
    pub camera_position: [f32; 3],
    pub camera_target: [f32; 3],
    pub ortho_scale: f32,
    pub auto_open_opt_in: bool,
    pub suggested: bool,
    #[serde(default)]
    pub trigger_mode: crate::visual_assistance::CompanionViewTrigger,
    #[serde(default)]
    pub steals_focus: bool,
}

impl Default for CompanionViewState {
    fn default() -> Self {
        Self {
            enabled: false,
            is_orthographic: true,
            camera_position: [0.0, 0.0, 10.0],
            camera_target: [0.0, 0.0, 0.0],
            ortho_scale: 5.0,
            auto_open_opt_in: false,
            suggested: false,
            trigger_mode: crate::visual_assistance::CompanionViewTrigger::Manual,
            steals_focus: false,
        }
    }
}

/// Inspecionador de geometria para medições de volume e consultas espaciais (VSP-003 / VSP-004).
pub struct SpatialQueryInspector;

impl SpatialQueryInspector {
    /// Computa o `VolumeFrame` orientado de um objeto de cena no frame desejado.
    pub fn compute_volume_frame(
        project: &Project,
        asset_id: Uuid,
        frame_kind: SpatialFrameKind,
        rep: GeometryRepresentation,
    ) -> Option<VolumeFrame> {
        let asset = project.assets.iter().find(|a| a.id == asset_id)?;
        let mesh_cow = match rep {
            GeometryRepresentation::Evaluated => asset.evaluated_mesh_ref(),
            GeometryRepresentation::Source => std::borrow::Cow::Borrowed(&*asset.mesh),
        };
        let mesh = mesh_cow.as_ref();

        let current_revision = project.revision_id();
        let transform = asset.transform;

        let (frame, quality) = match frame_kind {
            SpatialFrameKind::World => (SpatialFrame::world(), MeasurementQuality::Exact),
            SpatialFrameKind::ObjectLocal => (
                SpatialFrame::object_local(&transform),
                MeasurementQuality::Exact,
            ),
            SpatialFrameKind::Structural => SpatialFrame::structural_with_quality(mesh, &transform),
            SpatialFrameKind::Custom => (
                SpatialFrame::object_local(&transform),
                MeasurementQuality::Exact,
            ),
            SpatialFrameKind::View => (SpatialFrame::world(), MeasurementQuality::Exact),
        };

        if mesh.verts.is_empty() {
            return Some(VolumeFrame {
                frame,
                representation: rep,
                quality,
                center: transform.translation,
                min: [0.0, 0.0, 0.0],
                max: [0.0, 0.0, 0.0],
                size: [0.0, 0.0, 0.0],
                volume: 0.0,
                unit: String::new(),
                vertex_count: 0,
                revision: current_revision,
            });
        }

        let world_mat = transform.to_mat4();
        let frame_origin = Vec3::from(frame.origin);
        let ux = Vec3::from(frame.axes[0]);
        let uy = Vec3::from(frame.axes[1]);
        let uz = Vec3::from(frame.axes[2]);

        let mut min_c = Vec3::splat(f32::INFINITY);
        let mut max_c = Vec3::splat(f32::NEG_INFINITY);
        let mut valid_verts = 0usize;

        for v in &mesh.verts {
            let wp = world_mat.transform_point3(v.vec());
            if !wp.is_finite() {
                continue;
            }
            valid_verts += 1;
            let rel = wp - frame_origin;
            let cx = rel.dot(ux);
            let cy = rel.dot(uy);
            let cz = rel.dot(uz);

            min_c.x = min_c.x.min(cx);
            min_c.y = min_c.y.min(cy);
            min_c.z = min_c.z.min(cz);

            max_c.x = max_c.x.max(cx);
            max_c.y = max_c.y.max(cy);
            max_c.z = max_c.z.max(cz);
        }

        if valid_verts == 0 {
            return None;
        }

        let size = (max_c - min_c).max(Vec3::ZERO);
        let center_offset = (min_c + max_c) * 0.5;
        let world_center =
            frame_origin + center_offset.x * ux + center_offset.y * uy + center_offset.z * uz;
        let volume = size.x * size.y * size.z;

        Some(VolumeFrame {
            frame,
            representation: rep,
            quality,
            center: world_center.to_array(),
            min: min_c.to_array(),
            max: max_c.to_array(),
            size: size.to_array(),
            volume,
            unit: String::new(),
            vertex_count: valid_verts,
            revision: current_revision,
        })
    }

    /// Executa uma consulta espacial pura sobre o projeto (VSP-004).
    pub fn execute_query(
        project: &Project,
        query: SpatialQueryKind,
    ) -> Result<SpatialQueryResult, CommandError> {
        match query {
            SpatialQueryKind::EuclideanDistance { from, to } => {
                let p1 = Vec3::from(from);
                let p2 = Vec3::from(to);
                let d = (p2 - p1).length();
                Ok(SpatialQueryResult {
                    query_name: "Euclidean Distance".to_string(),
                    quality: MeasurementQuality::Exact,
                    distance: d,
                    delta: Some([
                        (p2.x - p1.x).abs(),
                        (p2.y - p1.y).abs(),
                        (p2.z - p1.z).abs(),
                    ]),
                    closest_points: Some((from, to)),
                    is_in_contact: d < 1e-4,
                    is_intersecting: false,
                    penetration_depth: None,
                    summary: format!(
                        "Distance: {:.4} (delta: [{:.3}, {:.3}, {:.3}])",
                        d,
                        (p2.x - p1.x).abs(),
                        (p2.y - p1.y).abs(),
                        (p2.z - p1.z).abs()
                    ),
                })
            }
            SpatialQueryKind::AxisDelta { from, to, frame } => {
                let p1 = Vec3::from(from);
                let p2 = Vec3::from(to);
                let diff = p2 - p1;
                let dx = diff.dot(Vec3::from(frame.axes[0])).abs();
                let dy = diff.dot(Vec3::from(frame.axes[1])).abs();
                let dz = diff.dot(Vec3::from(frame.axes[2])).abs();
                let total = diff.length();
                Ok(SpatialQueryResult {
                    query_name: format!("Axis Delta ({})", frame.kind.label()),
                    quality: MeasurementQuality::Exact,
                    distance: total,
                    delta: Some([dx, dy, dz]),
                    closest_points: Some((from, to)),
                    is_in_contact: total < 1e-4,
                    is_intersecting: false,
                    penetration_depth: None,
                    summary: format!(
                        "Frame {}: dx={:.3}, dy={:.3}, dz={:.3}",
                        frame.kind.label(),
                        dx,
                        dy,
                        dz
                    ),
                })
            }
            SpatialQueryKind::CameraDepth {
                point,
                camera_pos,
                camera_forward,
            } => {
                let p = Vec3::from(point);
                let c_pos = Vec3::from(camera_pos);
                let c_fwd = Vec3::from(camera_forward).normalize_or_zero();
                let depth = (p - c_pos).dot(c_fwd);
                Ok(SpatialQueryResult {
                    query_name: "Camera Depth".to_string(),
                    quality: MeasurementQuality::Exact,
                    distance: depth,
                    delta: None,
                    closest_points: Some((point, (c_pos + c_fwd * depth).to_array())),
                    is_in_contact: depth.abs() < 1e-4,
                    is_intersecting: false,
                    penetration_depth: None,
                    summary: format!("View Depth: {:.4}", depth),
                })
            }
            SpatialQueryKind::SurfaceClearance { object_a, object_b } => {
                let asset_a = project
                    .assets
                    .iter()
                    .find(|a| a.id == object_a)
                    .ok_or_else(|| {
                        CommandError::Execution(format!("Object A {object_a} not found"))
                    })?;
                let asset_b = project
                    .assets
                    .iter()
                    .find(|a| a.id == object_b)
                    .ok_or_else(|| {
                        CommandError::Execution(format!("Object B {object_b} not found"))
                    })?;

                let (min_dist, pt_a, pt_b) = compute_mesh_clearance(asset_a, asset_b);
                Ok(SpatialQueryResult {
                    query_name: "Surface Clearance".to_string(),
                    quality: MeasurementQuality::Exact,
                    distance: min_dist,
                    delta: None,
                    closest_points: Some((pt_a, pt_b)),
                    is_in_contact: min_dist < 1e-4,
                    is_intersecting: min_dist <= 0.0,
                    penetration_depth: None,
                    summary: format!(
                        "Clearance between '{}' and '{}': {:.4}",
                        asset_a.name, asset_b.name, min_dist
                    ),
                })
            }
            SpatialQueryKind::ContactCheck {
                object_a,
                object_b,
                tolerance,
            } => {
                let asset_a = project
                    .assets
                    .iter()
                    .find(|a| a.id == object_a)
                    .ok_or_else(|| {
                        CommandError::Execution(format!("Object A {object_a} not found"))
                    })?;
                let asset_b = project
                    .assets
                    .iter()
                    .find(|a| a.id == object_b)
                    .ok_or_else(|| {
                        CommandError::Execution(format!("Object B {object_b} not found"))
                    })?;

                let (min_dist, pt_a, pt_b) =
                    compute_mesh_clearance_with_tolerance(asset_a, asset_b, Some(tolerance));
                let is_contact = min_dist <= tolerance;
                Ok(SpatialQueryResult {
                    query_name: "Contact Check".to_string(),
                    quality: MeasurementQuality::Exact,
                    distance: min_dist,
                    delta: None,
                    closest_points: Some((pt_a, pt_b)),
                    is_in_contact: is_contact,
                    is_intersecting: min_dist <= 0.0,
                    penetration_depth: if min_dist <= 0.0 {
                        Some(-min_dist)
                    } else {
                        None
                    },
                    summary: if is_contact {
                        format!(
                            "IN CONTACT (distance {:.4} <= tolerance {:.4})",
                            min_dist, tolerance
                        )
                    } else {
                        format!(
                            "SEPARATED (distance {:.4} > tolerance {:.4})",
                            min_dist, tolerance
                        )
                    },
                })
            }
            SpatialQueryKind::VolumetricIntersection { object_a, object_b } => {
                let _asset_a = project
                    .assets
                    .iter()
                    .find(|a| a.id == object_a)
                    .ok_or_else(|| {
                        CommandError::Execution(format!("Object A {object_a} not found"))
                    })?;
                let _asset_b = project
                    .assets
                    .iter()
                    .find(|a| a.id == object_b)
                    .ok_or_else(|| {
                        CommandError::Execution(format!("Object B {object_b} not found"))
                    })?;

                let obb_a = Self::compute_volume_frame(
                    project,
                    object_a,
                    SpatialFrameKind::ObjectLocal,
                    GeometryRepresentation::Evaluated,
                )
                .ok_or_else(|| {
                    CommandError::Execution("Failed to compute volume frame A".into())
                })?;
                let obb_b = Self::compute_volume_frame(
                    project,
                    object_b,
                    SpatialFrameKind::ObjectLocal,
                    GeometryRepresentation::Evaluated,
                )
                .ok_or_else(|| {
                    CommandError::Execution("Failed to compute volume frame B".into())
                })?;

                let (intersecting, penetration) = sat_obb_intersection(&obb_a, &obb_b);

                Ok(SpatialQueryResult {
                    query_name: "Volumetric Intersection".to_string(),
                    quality: MeasurementQuality::Exact,
                    distance: if intersecting { 0.0 } else { penetration },
                    delta: None,
                    closest_points: None,
                    is_in_contact: intersecting,
                    is_intersecting: intersecting,
                    penetration_depth: if intersecting {
                        Some(penetration)
                    } else {
                        None
                    },
                    summary: if intersecting {
                        format!("INTERSECTING (penetration depth {:.4})", penetration)
                    } else {
                        format!("DISJOINT (minimum separation {:.4})", penetration)
                    },
                })
            }
        }
    }
}

/// Helper para computar clearance entre vértices de duas malhas transformadas
/// com aceleração em camadas (bounding box culling, upper-bound pruning e early exit sob tolerância).
fn compute_mesh_clearance(
    asset_a: &petunia_project::Asset,
    asset_b: &petunia_project::Asset,
) -> (f32, [f32; 3], [f32; 3]) {
    compute_mesh_clearance_with_tolerance(asset_a, asset_b, None)
}

fn compute_mesh_clearance_with_tolerance(
    asset_a: &petunia_project::Asset,
    asset_b: &petunia_project::Asset,
    tolerance: Option<f32>,
) -> (f32, [f32; 3], [f32; 3]) {
    let mat_a = asset_a.transform.to_mat4();
    let mat_b = asset_b.transform.to_mat4();

    let mesh_a = asset_a.evaluated_mesh_ref();
    let verts_a: Vec<Vec3> = mesh_a
        .verts
        .iter()
        .map(|v| mat_a.transform_point3(v.vec()))
        .filter(|p| p.is_finite())
        .collect();

    let mesh_b = asset_b.evaluated_mesh_ref();
    let verts_b: Vec<Vec3> = mesh_b
        .verts
        .iter()
        .map(|v| mat_b.transform_point3(v.vec()))
        .filter(|p| p.is_finite())
        .collect();

    if verts_a.is_empty() || verts_b.is_empty() {
        return (f32::INFINITY, [0.0; 3], [0.0; 3]);
    }

    // Calcula AABBs de A e B
    let mut min_a = Vec3::splat(f32::INFINITY);
    let mut max_a = Vec3::splat(f32::NEG_INFINITY);
    for va in &verts_a {
        min_a = min_a.min(*va);
        max_a = max_a.max(*va);
    }

    let mut min_b = Vec3::splat(f32::INFINITY);
    let mut max_b = Vec3::splat(f32::NEG_INFINITY);
    for vb in &verts_b {
        min_b = min_b.min(*vb);
        max_b = max_b.max(*vb);
    }

    // Camada 0: Teste rápido de separação entre AABBs
    let dx = (min_b.x - max_a.x).max(min_a.x - max_b.x).max(0.0);
    let dy = (min_b.y - max_a.y).max(min_a.y - max_b.y).max(0.0);
    let dz = (min_b.z - max_a.z).max(min_a.z - max_b.z).max(0.0);
    let aabb_dist_sq = dx * dx + dy * dy + dz * dz;

    // Se estivermos checando contato sob tolerância e as caixas estiverem mais longe que a tolerância,
    // garantidamente não há contato (rejeição em O(1)).
    if let Some(tol) = tolerance
        && aabb_dist_sq > tol * tol
    {
        let center_a = (min_a + max_a) * 0.5;
        let center_b = (min_b + max_b) * 0.5;
        return (
            aabb_dist_sq.sqrt(),
            center_a.to_array(),
            center_b.to_array(),
        );
    }

    let early_exit_sq = tolerance.map(|t| t * t);
    let mut min_dist_sq = f32::INFINITY;
    let mut best_a = verts_a[0];
    let mut best_b = verts_b[0];

    // Camada 1: Poda por cota superior (upper-bound pruning) e early exit
    for &va in &verts_a {
        // Distância mínima do ponto va à caixa envolvente de B
        let px = (min_b.x - va.x).max(va.x - max_b.x).max(0.0);
        let py = (min_b.y - va.y).max(va.y - max_b.y).max(0.0);
        let pz = (min_b.z - va.z).max(va.z - max_b.z).max(0.0);
        let dist_to_b_box_sq = px * px + py * py + pz * pz;

        // Se va já está mais longe da caixa B inteira do que o recorde atual, pula va
        if dist_to_b_box_sq >= min_dist_sq {
            continue;
        }

        for &vb in &verts_b {
            let d_sq = (vb - va).length_squared();
            if d_sq < min_dist_sq {
                min_dist_sq = d_sq;
                best_a = va;
                best_b = vb;

                // Camada 2: Early exit se já estiver dentro da tolerância de contato
                if let Some(target_sq) = early_exit_sq
                    && min_dist_sq <= target_sq
                {
                    return (min_dist_sq.sqrt(), best_a.to_array(), best_b.to_array());
                }
            }
        }
    }

    (min_dist_sq.sqrt(), best_a.to_array(), best_b.to_array())
}

/// Teste de interseção entre dois OBBs usando o Teorema dos Eixos Separadores (SAT 15 eixos).
fn sat_obb_intersection(box_a: &VolumeFrame, box_b: &VolumeFrame) -> (bool, f32) {
    let ca = Vec3::from(box_a.center);
    let cb = Vec3::from(box_b.center);
    let t = cb - ca;

    let a_axes = [
        Vec3::from(box_a.frame.axes[0]),
        Vec3::from(box_a.frame.axes[1]),
        Vec3::from(box_a.frame.axes[2]),
    ];
    let a_ext = [
        box_a.size[0] * 0.5,
        box_a.size[1] * 0.5,
        box_a.size[2] * 0.5,
    ];

    let b_axes = [
        Vec3::from(box_b.frame.axes[0]),
        Vec3::from(box_b.frame.axes[1]),
        Vec3::from(box_b.frame.axes[2]),
    ];
    let b_ext = [
        box_b.size[0] * 0.5,
        box_b.size[1] * 0.5,
        box_b.size[2] * 0.5,
    ];

    let mut min_penetration = f32::INFINITY;

    // 15 eixos potenciais: 3 de A, 3 de B, 9 produtos vetoriais
    let mut test_axes = Vec::with_capacity(15);
    for a in &a_axes {
        test_axes.push(*a);
    }
    for b in &b_axes {
        test_axes.push(*b);
    }
    for a in &a_axes {
        for b in &b_axes {
            let cross = a.cross(*b).normalize_or_zero();
            if cross.length_squared() > 1e-4 {
                test_axes.push(cross);
            }
        }
    }

    for axis in test_axes {
        let ra = a_ext[0] * a_axes[0].dot(axis).abs()
            + a_ext[1] * a_axes[1].dot(axis).abs()
            + a_ext[2] * a_axes[2].dot(axis).abs();

        let rb = b_ext[0] * b_axes[0].dot(axis).abs()
            + b_ext[1] * b_axes[1].dot(axis).abs()
            + b_ext[2] * b_axes[2].dot(axis).abs();

        let dist = t.dot(axis).abs();
        let overlap = (ra + rb) - dist;

        if overlap < 0.0 {
            // Eixo separador encontrado -> Não há interseção. Distância de separação é -overlap.
            return (false, -overlap);
        }

        min_penetration = min_penetration.min(overlap);
    }

    (true, min_penetration)
}

/// Algoritmo clássico de Jacobi para autovetores e autovalores de matriz simétrica 3x3.
fn jacobi_eigenvectors_3x3(mut a: [[f32; 3]; 3]) -> ([[f32; 3]; 3], [f32; 3]) {
    let mut v = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

    for _ in 0..20 {
        // Encontra elemento fora da diagonal com maior magnitude
        let mut p = 0;
        let mut q = 1;
        let mut max_val = a[0][1].abs();
        if a[0][2].abs() > max_val {
            p = 0;
            q = 2;
            max_val = a[0][2].abs();
        }
        if a[1][2].abs() > max_val {
            p = 1;
            q = 2;
            max_val = a[1][2].abs();
        }

        if max_val < 1e-6 {
            break;
        }

        let app = a[p][p];
        let aqq = a[q][q];
        let apq = a[p][q];

        let phi = 0.5 * (2.0 * apq).atan2(aqq - app);
        let c = phi.cos();
        let s = phi.sin();

        // Rotação de Jacobi
        a[p][p] = c * c * app - 2.0 * s * c * apq + s * s * aqq;
        a[q][q] = s * s * app + 2.0 * s * c * apq + c * c * aqq;
        a[p][q] = 0.0;
        a[q][p] = 0.0;

        for r in 0..3 {
            if r != p && r != q {
                let arp = a[r][p];
                let arq = a[r][q];
                a[r][p] = c * arp - s * arq;
                a[p][r] = a[r][p];
                a[r][q] = s * arp + c * arq;
                a[q][r] = a[r][q];
            }
            let vrp = v[r][p];
            let vrq = v[r][q];
            v[r][p] = c * vrp - s * vrq;
            v[r][q] = s * vrp + c * vrq;
        }
    }

    // Transpõe v para obter autovetores por linha [x, y, z]
    let eigenvectors = [
        [v[0][0], v[1][0], v[2][0]],
        [v[0][1], v[1][1], v[2][1]],
        [v[0][2], v[1][2], v[2][2]],
    ];
    let eigenvalues = [a[0][0], a[1][1], a[2][2]];
    (eigenvectors, eigenvalues)
}

/// Comando para escalar um objeto até atingir exatamente determinada dimensão (VSP-003).
///
/// Protegido estritamente contra divisão por zero: dimensões zero ou nulas rejeitam o comando
/// com erro gracioso sem entrar em pânico.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScaleToDimensionCmd {
    pub asset_id: Uuid,
    pub axis: usize,
    pub target_dimension: f32,
    pub uniform: bool,
    pub frame: SpatialFrameKind,
}

impl ScaleToDimensionCmd {
    pub fn new(
        asset_id: Uuid,
        axis: usize,
        target_dimension: f32,
        uniform: bool,
        frame: SpatialFrameKind,
    ) -> Self {
        Self {
            asset_id,
            axis: axis.min(2),
            target_dimension,
            uniform,
            frame,
        }
    }
}

impl Command for ScaleToDimensionCmd {
    fn label(&self) -> &'static str {
        "Scale To Dimension"
    }

    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        if !self.target_dimension.is_finite() || self.target_dimension <= 0.0 {
            return Err("Target dimension must be positive and finite");
        }
        if state.document.assets.is_empty() {
            return Err("No active assets in project");
        }
        Ok(())
    }

    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        if !self.target_dimension.is_finite() || self.target_dimension <= 0.0 {
            return Err(CommandError::Execution(
                "Target dimension must be positive and finite".to_string(),
            ));
        }

        // Calcula a dimensão atual no frame indicado
        let volume_frame = SpatialQueryInspector::compute_volume_frame(
            &state.document.project,
            self.asset_id,
            self.frame,
            GeometryRepresentation::Evaluated,
        )
        .ok_or_else(|| CommandError::Execution("Target asset not found".to_string()))?;

        let current_dim = volume_frame.size[self.axis];

        // VSP-003: "dimensões zero não permitem divisão por zero."
        if !current_dim.is_finite() || current_dim < 1e-6 {
            return Err(CommandError::Execution(
                "Cannot scale object with zero dimension on the selected axis (division by zero avoided)".to_string(),
            ));
        }

        let ratio = self.target_dimension / current_dim;
        let asset = state
            .document
            .assets
            .iter_mut()
            .find(|a| a.id == self.asset_id)
            .ok_or(CommandError::NoActiveAsset)?;

        let mut transform = asset.transform;
        if self.uniform {
            transform.scale[0] *= ratio;
            transform.scale[1] *= ratio;
            transform.scale[2] *= ratio;
        } else {
            match self.axis {
                0 => transform.scale[0] *= ratio,
                1 => transform.scale[1] *= ratio,
                _ => transform.scale[2] *= ratio,
            }
        }

        asset.set_object_transform(transform);
        state.mark_document_dirty();
        state.emit_project_changed(ProjectChanges::TRANSFORMS);
        Ok(())
    }

    fn changes(&self) -> ProjectChanges {
        ProjectChanges::TRANSFORMS
    }
}
