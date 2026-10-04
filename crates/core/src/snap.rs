//! Petunia3D — Motor de Snapping Geométrico e de Grade (P3D-040).
//!
//! Fornece cálculos determinísticos e headless para atração magnética
//! de coordenadas a grade, incrementos, vértices, arestas e faces da malha.

use glam::Vec3;
use petunia_mesh::Mesh;
use serde::{Deserialize, Serialize};

/// Alvo de snapping configurado pelo usuário.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SnapTarget {
    #[default]
    Grid,
    Increment,
    Vertex,
    Edge,
    Face,
}

impl SnapTarget {
    pub fn all() -> [Self; 5] {
        [
            Self::Grid,
            Self::Increment,
            Self::Vertex,
            Self::Edge,
            Self::Face,
        ]
    }

    pub fn key(&self) -> &'static str {
        match self {
            Self::Grid => "snap.grid",
            Self::Increment => "snap.increment",
            Self::Vertex => "snap.vertex",
            Self::Edge => "snap.edge",
            Self::Face => "snap.face",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Grid => "Grid",
            Self::Increment => "Increment",
            Self::Vertex => "Point",
            Self::Edge => "Edge",
            Self::Face => "Face",
        }
    }
}

/// Elemento de referência para cálculo do snap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum SnapElement {
    #[default]
    Closest,
    Center,
    Median,
}

/// Configurações do sistema de snap mantidas na sessão do editor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapSettings {
    pub enabled: bool,
    pub target: SnapTarget,
    pub element: SnapElement,
    pub grid_spacing: f32,
    /// Legado (unidades de mundo) das funções `snap_point*`; as ferramentas
    /// interativas usam [`Self::radius_pixels`].
    pub snap_distance: f32,
    /// Raio do snap interativo em px lógicos (P3D-040).
    #[serde(default = "default_radius_pixels")]
    pub radius_pixels: f32,
}

fn default_radius_pixels() -> f32 {
    crate::inference::DEFAULT_SNAP_RADIUS_PIXELS
}

impl Default for SnapSettings {
    fn default() -> Self {
        Self {
            // Snapping contextual por padrão (capítulo 01, "Smart Snap").
            enabled: true,
            target: SnapTarget::Grid,
            element: SnapElement::Closest,
            grid_spacing: 1.0,
            snap_distance: 0.35,
            radius_pixels: default_radius_pixels(),
        }
    }
}

/// Consulta de snapping enviada por ferramentas ou gizmos.
#[derive(Debug, Clone, Copy)]
pub struct SnapQuery<'a> {
    pub point: Vec3,
    pub start_point: Option<Vec3>,
    pub settings: &'a SnapSettings,
    pub mesh: Option<&'a Mesh>,
}

/// Resultado retornado pela consulta de snapping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapResult {
    pub point: Vec3,
    pub snapped: bool,
    pub target: SnapTarget,
    pub distance: f32,
}

/// Arredonda um ponto para o múltiplo mais próximo do espaçamento da grade.
pub fn snap_point_to_grid(point: Vec3, spacing: f32) -> Vec3 {
    let s = if spacing > 1e-4 { spacing } else { 1.0 };
    Vec3::new(
        (point.x / s).round() * s,
        (point.y / s).round() * s,
        (point.z / s).round() * s,
    )
}

/// Aplica incremento em relação a um ponto inicial.
pub fn snap_point_to_increment(point: Vec3, start_point: Vec3, increment: f32) -> Vec3 {
    let delta = point - start_point;
    start_point + snap_point_to_grid(delta, increment)
}

/// Encontra o vértice mais próximo na malha dentro de `max_distance`.
pub fn snap_point_to_vertices(point: Vec3, mesh: &Mesh, max_distance: f32) -> Option<(Vec3, f32)> {
    let mut best: Option<(Vec3, f32)> = None;
    let max_dist_sq = max_distance * max_distance;

    for v in &mesh.verts {
        let p = v.vec();
        let dist_sq = (p - point).length_squared();
        if dist_sq <= max_dist_sq {
            if let Some((_, best_dist_sq)) = best {
                if dist_sq < best_dist_sq {
                    best = Some((p, dist_sq));
                }
            } else {
                best = Some((p, dist_sq));
            }
        }
    }

    best.map(|(p, d_sq)| (p, d_sq.sqrt()))
}

/// Encontra o ponto mais próximo sobre qualquer aresta única da malha dentro de `max_distance`.
pub fn snap_point_to_edges(point: Vec3, mesh: &Mesh, max_distance: f32) -> Option<(Vec3, f32)> {
    let mut best: Option<(Vec3, f32)> = None;
    let max_dist_sq = max_distance * max_distance;

    for (a, b) in mesh.edges_unique() {
        let (va, vb) = (a as usize, b as usize);
        if va >= mesh.verts.len() || vb >= mesh.verts.len() {
            continue;
        }
        let pa = mesh.verts[va].vec();
        let pb = mesh.verts[vb].vec();

        let edge_vec = pb - pa;
        let edge_len_sq = edge_vec.length_squared();
        if edge_len_sq < 1e-8 {
            continue;
        }

        let t = ((point - pa).dot(edge_vec) / edge_len_sq).clamp(0.0, 1.0);
        let proj = pa + edge_vec * t;
        let dist_sq = (proj - point).length_squared();

        if dist_sq <= max_dist_sq {
            if let Some((_, best_dist_sq)) = best {
                if dist_sq < best_dist_sq {
                    best = Some((proj, dist_sq));
                }
            } else {
                best = Some((proj, dist_sq));
            }
        }
    }

    best.map(|(p, d_sq)| (p, d_sq.sqrt()))
}

/// Encontra o ponto mais próximo sobre qualquer face (ou centroide) dentro de `max_distance`.
pub fn snap_point_to_faces(point: Vec3, mesh: &Mesh, max_distance: f32) -> Option<(Vec3, f32)> {
    let mut best: Option<(Vec3, f32)> = None;
    let max_dist_sq = max_distance * max_distance;

    for (fi, _) in mesh.faces.iter().enumerate() {
        let centroid = mesh.face_centroid(fi);
        let dist_sq = (centroid - point).length_squared();
        if dist_sq <= max_dist_sq {
            if let Some((_, best_dist_sq)) = best {
                if dist_sq < best_dist_sq {
                    best = Some((centroid, dist_sq));
                }
            } else {
                best = Some((centroid, dist_sq));
            }
        }
    }

    best.map(|(p, d_sq)| (p, d_sq.sqrt()))
}

/// Executa a consulta completa de snapping conforme as configurações fornecidas.
pub fn snap_point(query: SnapQuery) -> SnapResult {
    if !query.settings.enabled {
        return SnapResult {
            point: query.point,
            snapped: false,
            target: query.settings.target,
            distance: 0.0,
        };
    }

    match query.settings.target {
        SnapTarget::Grid => {
            let snapped_pt = snap_point_to_grid(query.point, query.settings.grid_spacing);
            SnapResult {
                point: snapped_pt,
                snapped: true,
                target: SnapTarget::Grid,
                distance: (snapped_pt - query.point).length(),
            }
        }
        SnapTarget::Increment => {
            let start = query.start_point.unwrap_or(Vec3::ZERO);
            let snapped_pt =
                snap_point_to_increment(query.point, start, query.settings.grid_spacing);
            SnapResult {
                point: snapped_pt,
                snapped: true,
                target: SnapTarget::Increment,
                distance: (snapped_pt - query.point).length(),
            }
        }
        SnapTarget::Vertex => {
            if let Some(mesh) = query.mesh
                && let Some((pt, dist)) =
                    snap_point_to_vertices(query.point, mesh, query.settings.snap_distance)
            {
                return SnapResult {
                    point: pt,
                    snapped: true,
                    target: SnapTarget::Vertex,
                    distance: dist,
                };
            }
            SnapResult {
                point: query.point,
                snapped: false,
                target: SnapTarget::Vertex,
                distance: 0.0,
            }
        }
        SnapTarget::Edge => {
            if let Some(mesh) = query.mesh
                && let Some((pt, dist)) =
                    snap_point_to_edges(query.point, mesh, query.settings.snap_distance)
            {
                return SnapResult {
                    point: pt,
                    snapped: true,
                    target: SnapTarget::Edge,
                    distance: dist,
                };
            }
            SnapResult {
                point: query.point,
                snapped: false,
                target: SnapTarget::Edge,
                distance: 0.0,
            }
        }
        SnapTarget::Face => {
            if let Some(mesh) = query.mesh
                && let Some((pt, dist)) =
                    snap_point_to_faces(query.point, mesh, query.settings.snap_distance)
            {
                return SnapResult {
                    point: pt,
                    snapped: true,
                    target: SnapTarget::Face,
                    distance: dist,
                };
            }
            SnapResult {
                point: query.point,
                snapped: false,
                target: SnapTarget::Face,
                distance: 0.0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_snap() {
        let pt = Vec3::new(1.18, 2.89, -0.45);
        let snapped = snap_point_to_grid(pt, 1.0);
        assert_eq!(snapped, Vec3::new(1.0, 3.0, -0.0));

        let snapped_half = snap_point_to_grid(pt, 0.5);
        assert_eq!(snapped_half, Vec3::new(1.0, 3.0, -0.5));
    }

    #[test]
    fn test_increment_snap() {
        let start = Vec3::new(0.3, 0.3, 0.3);
        let current = Vec3::new(1.35, 2.1, 0.4);
        let snapped = snap_point_to_increment(current, start, 1.0);
        assert!((snapped.x - 1.3).abs() < 1e-5);
        assert!((snapped.y - 2.3).abs() < 1e-5);
        assert!((snapped.z - 0.3).abs() < 1e-5);
    }

    #[test]
    fn test_vertex_snap() {
        let mesh = Mesh::cube(2.0); // vertices at +-1.0
        let query_pt = Vec3::new(0.95, 1.02, -0.98);
        let (snapped, dist) = snap_point_to_vertices(query_pt, &mesh, 0.5).expect("snaps");
        assert_eq!(snapped, Vec3::new(1.0, 1.0, -1.0));
        assert!(dist < 0.1);

        assert!(snap_point_to_vertices(Vec3::new(5.0, 5.0, 5.0), &mesh, 0.5).is_none());
    }

    #[test]
    fn test_edge_snap() {
        let mesh = Mesh::cube(2.0);
        let query_pt = Vec3::new(1.05, 0.1, -1.02);
        let (snapped, dist) = snap_point_to_edges(query_pt, &mesh, 0.5).expect("snaps to edge");
        assert!((snapped.x - 1.0).abs() < 1e-5);
        assert!((snapped.y - 0.1).abs() < 1e-5);
        assert!((snapped.z - -1.0).abs() < 1e-5);
        assert!(dist < 0.1);
    }

    #[test]
    fn test_snap_query_dispatcher() {
        let mut settings = SnapSettings::default();
        let query1 = SnapQuery {
            point: Vec3::new(1.23, 0.0, 0.0),
            start_point: None,
            settings: &settings,
            mesh: None,
        };
        let res = snap_point(query1);
        assert!(!res.snapped);
        assert_eq!(res.point, Vec3::new(1.23, 0.0, 0.0));

        settings.enabled = true;
        settings.target = SnapTarget::Grid;
        let query2 = SnapQuery {
            point: Vec3::new(1.23, 0.0, 0.0),
            start_point: None,
            settings: &settings,
            mesh: None,
        };
        let res = snap_point(query2);
        assert!(res.snapped);
        assert_eq!(res.point, Vec3::new(1.0, 0.0, 0.0));
    }
}
