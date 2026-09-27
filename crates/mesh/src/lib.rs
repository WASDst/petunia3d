//! Petunia3D — domínio de malha low-poly.
//!
//! Malha própria: vértices + faces (tris/quads) + UV por vértice-de-face +
//! arestas selecionáveis. Operações retornam `Result` em input externo e
//! mantêm o invariante `face.uv.len() == face.verts.len()`.

use std::collections::HashSet;

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Vértice expandido p/ render/export: (posição, normal, cor, uv).
pub type Tri = ([f32; 3], [f32; 3], [f32; 3], [f32; 2]);

// ---------------------------------------------------------------------------
// Tipos
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vertex {
    pub pos: [f32; 3],
    pub color: [f32; 3],
    pub selected: bool,
}

impl Vertex {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: [x, y, z],
            color: [0.75, 0.75, 0.78],
            selected: false,
        }
    }
    pub fn vec(&self) -> Vec3 {
        Vec3::from(self.pos)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Face {
    /// 3 (tri) ou 4 (quad) índices.
    pub verts: Vec<u32>,
    /// UV por vértice-de-face; invariante: `uv.len() == verts.len()`.
    pub uv: Vec<[f32; 2]>,
    pub selected: bool,
    #[serde(default)]
    pub material_slot: Option<usize>,
}

impl Face {
    pub fn new(verts: Vec<u32>) -> Self {
        let n = verts.len();
        Self {
            verts,
            uv: vec![[0.0, 0.0]; n],
            selected: false,
            material_slot: None,
        }
    }
    pub fn with_uv(verts: Vec<u32>, uv: Vec<[f32; 2]>) -> Self {
        debug_assert_eq!(verts.len(), uv.len());
        Self {
            verts,
            uv,
            selected: false,
            material_slot: None,
        }
    }
    fn fix_uv(&mut self) {
        if self.uv.len() != self.verts.len() {
            self.uv = vec![[0.0, 0.0]; self.verts.len()];
        }
    }
}

/// Malha. `selected_edges` guarda arestas como `(min, max)`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Mesh {
    pub verts: Vec<Vertex>,
    pub faces: Vec<Face>,
    pub selected_edges: HashSet<(u32, u32)>,
    /// UV0 seams: undirected edges that split charts (ch. 15).
    #[serde(default)]
    pub uv_seams: HashSet<(u32, u32)>,
    /// Vértices UV fixados (pinned): pares (face_idx, corner_idx) estáticos em projeções/unwraps.
    #[serde(default)]
    pub uv_pinned: HashSet<(usize, usize)>,
}

pub fn edge_key(a: u32, b: u32) -> (u32, u32) {
    (a.min(b), a.max(b))
}

impl Mesh {
    pub fn push_face(&mut self, face: Face) {
        let mut f = face;
        f.fix_uv();
        self.faces.push(f);
    }

    /// Valida/repara malha vinda de fora (arquivo): uvs, índices, arestas.
    /// Nunca panica; remove o que não tem conserto.
    pub fn validate(&mut self) {
        // R2: sanitiza NaN/inf (senão o envenenamento persiste no projeto)
        for v in &mut self.verts {
            if !v.pos.iter().all(|x| x.is_finite()) {
                v.pos = [0.0, 0.0, 0.0];
            }
            if !v.color.iter().all(|x| x.is_finite()) {
                v.color = [0.75, 0.75, 0.78];
            }
        }
        let n = self.verts.len();
        for f in &mut self.faces {
            f.fix_uv();
        }
        self.faces.retain(|f| {
            f.verts.len() >= 3 && f.verts.iter().all(|&i| (i as usize) < n) && {
                let mut u = f.verts.clone();
                u.sort_unstable();
                u.dedup();
                u.len() >= 3
            }
        });
        self.selected_edges
            .retain(|&(a, b)| (a as usize) < n && (b as usize) < n && a != b);
    }

    pub fn vert_count(&self) -> usize {
        self.verts.len()
    }
    #[allow(dead_code)]
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }
    /// tris equivalentes (quads = 2 tris).
    pub fn tri_count(&self) -> usize {
        self.faces
            .iter()
            .map(|f| f.verts.len().saturating_sub(2))
            .sum()
    }

    /// Arestas únicas da malha.
    pub fn edges_unique(&self) -> Vec<(u32, u32)> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for f in &self.faces {
            let m = f.verts.len();
            for k in 0..m {
                let key = edge_key(f.verts[k], f.verts[(k + 1) % m]);
                if seen.insert(key) {
                    out.push(key);
                }
            }
        }
        out
    }

    /// Faces adjacentes a uma aresta (índices).
    pub fn edge_faces(&self, a: u32, b: u32) -> Vec<usize> {
        self.faces
            .iter()
            .enumerate()
            .filter(|(_, f)| {
                let m = f.verts.len();
                (0..m).any(|k| {
                    let x = f.verts[k];
                    let y = f.verts[(k + 1) % m];
                    (x == a && y == b) || (x == b && y == a)
                })
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn face_centroid(&self, fi: usize) -> Vec3 {
        let f = &self.faces[fi];
        let mut c = Vec3::ZERO;
        for &vi in &f.verts {
            c += self.verts[vi as usize].vec();
        }
        c / f.verts.len().max(1) as f32
    }

    /// Raycast against the render triangulation, including concave n-gons.
    /// Retorna `(face, t, ponto)` mais próximo, se houver.
    pub fn ray_hit(&self, origin: Vec3, dir: Vec3) -> Option<(usize, f32, Vec3)> {
        let mut best: Option<(usize, f32)> = None;
        for (fi, f) in self.faces.iter().enumerate() {
            for corners in self.face_triangle_corners(fi) {
                let [a, b, c] = corners.map(|corner| f.verts[corner]);
                if let Some(t) = triangulate::ray_tri(
                    origin,
                    dir,
                    self.verts[a as usize].vec(),
                    self.verts[b as usize].vec(),
                    self.verts[c as usize].vec(),
                ) && t > 1e-6
                    && best.map(|(_, bt)| t < bt).unwrap_or(true)
                {
                    best = Some((fi, t));
                }
            }
        }
        best.map(|(fi, t)| (fi, t, origin + dir * t))
    }

    pub fn face_normal(&self, fi: usize) -> Vec3 {
        triangulate::face_normal_of(&self.verts, &self.faces[fi].verts)
    }

    /// Calcula o centroide médio dos vértices da malha.
    pub fn center(&self) -> Vec3 {
        if self.verts.is_empty() {
            return Vec3::ZERO;
        }
        let sum: Vec3 = self.verts.iter().map(|v| v.vec()).sum();
        sum / (self.verts.len() as f32)
    }
}

mod bevel;
pub mod boolean;
mod connect;
pub mod curve;
pub mod half_edge;
pub mod knife;
pub mod loop_cut;
pub mod obj;
pub mod ops;
pub mod path_frames;
pub mod primitives;
pub mod profile_geo;
pub mod sweep;
pub mod topology;
pub mod triangulate;
pub mod uv;
pub mod uv_tools;
pub mod uv_xatlas;

pub use curve::{BezierNode, BezierNodeKind, BezierPath, create_hollow_profile, offset_polygon};
pub use half_edge::{
    EdgeId, FaceId, HalfEdge, HalfEdgeId, HalfEdgeMesh, TopologyDefect, TopologyReport, VertexId,
};
pub use path_frames::{PathFrame, PathFrameError, compute_parallel_transport_frames};
pub use primitives::{CircleFill, PrimitiveAudit, PrimitiveDescriptor, primitive_audit};
pub use sweep::{SweepFrame, SweepOptions, compute_rmf_frames, generate_sweep};
pub use topology::{DirtyDomains, ElementRemap, TopologyResult};
pub use uv_tools::{UvDiagnostics, UvIsland};

impl Mesh {
    /// Executa varredura 3D (Sweep) de um perfil 2D ao longo de um caminho tridimensional
    /// usando Rotation Minimizing Frames (RMF) e mitering em cantos vivos.
    pub fn from_sweep(
        profile: &[[f32; 2]],
        path: &[glam::Vec3],
        options: sweep::SweepOptions,
    ) -> Result<Self, String> {
        sweep::generate_sweep(profile, path, options)
    }

    // ---------------- saída p/ render/export ----------------

    /// Triangles expressed as face-corner indices, preserving UV seams and
    /// winding. Concave polygons must use the same tessellation for drawing
    /// and picking; a fan can otherwise make empty space selectable.
    pub fn face_triangle_corners(&self, face_index: usize) -> Vec<[usize; 3]> {
        let Some(face) = self.faces.get(face_index) else {
            return Vec::new();
        };
        if face.verts.len() < 3
            || face.verts.iter().any(|&v| {
                v as usize >= self.verts.len() || !self.verts[v as usize].vec().is_finite()
            })
        {
            return Vec::new();
        }
        if face.verts.len() == 3 {
            return vec![[0, 1, 2]];
        }
        let normal = self.face_normal(face_index).abs();
        let axis = if normal.x >= normal.y && normal.x >= normal.z {
            0
        } else if normal.y >= normal.z {
            1
        } else {
            2
        };
        let points: Vec<[f32; 2]> = face
            .verts
            .iter()
            .map(|&v| {
                let p = self.verts[v as usize].pos;
                match axis {
                    0 => [p[1], p[2]],
                    1 => [p[2], p[0]],
                    _ => [p[0], p[1]],
                }
            })
            .collect();
        let area = triangulate::polygon_area(&points);
        let convex = (0..points.len()).all(|i| {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            let c = points[(i + 2) % points.len()];
            ((b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])) * area >= 0.0
        });
        if convex && area.abs() > 1.0e-12 {
            return (1..face.verts.len() - 1).map(|i| [0, i, i + 1]).collect();
        }
        let Ok(mut triangles) = triangulate::ear_clip(&points) else {
            return Vec::new();
        };
        if area < 0.0 {
            for tri in &mut triangles {
                tri.swap(1, 2);
            }
        }
        triangles
    }

    /// Identidade estável da conectividade e da triangulação autoral.
    /// Posições só participam quando mudam a decomposição de uma face côncava.
    pub fn topology_fingerprint(&self) -> u64 {
        let mut fingerprint = 0xcbf2_9ce4_8422_2325_u64;
        let mix = |value: u64, next: u64| {
            let value = value ^ next;
            value.wrapping_mul(0x100_0000_01b3)
        };
        fingerprint = mix(fingerprint, self.verts.len() as u64);
        fingerprint = mix(fingerprint, self.faces.len() as u64);
        for (face_index, face) in self.faces.iter().enumerate() {
            fingerprint = mix(fingerprint, face.verts.len() as u64);
            for &vertex in &face.verts {
                fingerprint = mix(fingerprint, u64::from(vertex));
            }
            let triangles = self.face_triangle_corners(face_index);
            fingerprint = mix(fingerprint, triangles.len() as u64);
            for triangle in triangles {
                for corner in triangle {
                    fingerprint = mix(fingerprint, corner as u64);
                }
            }
        }
        fingerprint
    }

    /// (pos, normal, cor, uv) por triângulo com normais facetadas (flat).
    pub fn to_triangles(&self) -> Vec<Tri> {
        self.to_triangles_smooth(false)
    }

    /// (pos, normal, cor, uv) por triângulo, com opção de normais suaves (interpoladas por vértice)
    /// ou facetadas (flat).
    pub fn to_triangles_smooth(&self, smooth: bool) -> Vec<Tri> {
        let vert_normals: Vec<[f32; 3]> = if smooth {
            let mut v_norms = vec![Vec3::ZERO; self.verts.len()];
            for (fi, f) in self.faces.iter().enumerate() {
                let fnorm = self.face_normal(fi);
                for &vi in &f.verts {
                    if (vi as usize) < v_norms.len() {
                        v_norms[vi as usize] += fnorm;
                    }
                }
            }
            v_norms
                .into_iter()
                .map(|vn| vn.normalize_or_zero().to_array())
                .collect()
        } else {
            Vec::new()
        };

        let mut out = Vec::new();
        for (fi, f) in self.faces.iter().enumerate() {
            for corners in self.face_triangle_corners(fi) {
                let [idx0, idx1, idx2] = corners.map(|i| f.verts[i] as usize);
                let pa = self.verts[idx0].vec();
                let pb = self.verts[idx1].vec();
                let pc = self.verts[idx2].vec();
                let flat_n = (pb - pa).cross(pc - pa).normalize_or_zero().to_array();

                let tri_indices = [idx0, idx1, idx2];
                let tri_uvs = corners.map(|i| f.uv.get(i).copied().unwrap_or([0.0, 0.0]));

                for (k, &vi) in tri_indices.iter().enumerate() {
                    let v = &self.verts[vi];
                    // A cor da geometria é do material, nunca da seleção: pintar
                    // aqui fazia uma aresta selecionada tingir todas as faces que
                    // tocam seus vértices. Seleção é uma camada de render à parte.
                    let col = v.color;
                    let n = if smooth && vi < vert_normals.len() {
                        vert_normals[vi]
                    } else {
                        flat_n
                    };
                    out.push((v.pos, n, col, tri_uvs[k]));
                }
            }
        }
        out
    }

    /// (pos, normal, cor, uv) com normais nulas para renderização unlit (sem iluminação).
    pub fn to_triangles_unlit(&self) -> Vec<Tri> {
        let mut tris = self.to_triangles_smooth(false);
        for t in &mut tris {
            t.1 = [0.0, 0.0, 0.0];
        }
        tris
    }

    /// (a, b, selecionada) p/ overlay wireframe.
    pub fn to_edges(&self) -> Vec<([f32; 3], [f32; 3], bool)> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for f in &self.faces {
            let m = f.verts.len();
            for k in 0..m {
                let a = f.verts[k];
                let b = f.verts[(k + 1) % m];
                let key = edge_key(a, b);
                if seen.insert(key) {
                    let sel = self.selected_edges.contains(&key);
                    out.push((self.verts[a as usize].pos, self.verts[b as usize].pos, sel));
                }
            }
        }
        out
    }

    /// Retorna pares de pontos `(p0, p1)` representando as diagonais internas de triangulação
    /// para todos os polígonos com 4 ou mais lados da malha (para exibição em Show Triangulation).
    pub fn triangulation_wireframe(&self) -> Vec<([f32; 3], [f32; 3])> {
        let mut lines = Vec::new();
        for f in &self.faces {
            let m = f.verts.len();
            if m <= 3 {
                continue;
            }
            let idx0 = f.verts[0] as usize;
            if idx0 >= self.verts.len() {
                continue;
            }
            let p0 = self.verts[idx0].pos;
            for i in 2..(m - 1) {
                let idxi = f.verts[i] as usize;
                if idxi < self.verts.len() {
                    let pi = self.verts[idxi].pos;
                    lines.push((p0, pi));
                }
            }
        }
        lines
    }
}

#[cfg(test)]
mod tests {
    use crate::{Face, Mesh, triangulate::ear_clip, triangulate::ray_tri_hit};
    use glam::Vec3;

    #[test]
    fn cube_has_8_verts_6_faces() {
        let m = Mesh::cube(2.0);
        assert_eq!(m.verts.len(), 8);
        assert_eq!(m.faces.len(), 6);
    }

    #[test]
    fn ray_triangle_hit_reports_stable_barycentric_coordinates() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(2.0, 0.0, 0.0);
        let c = Vec3::new(0.0, 0.0, 2.0);
        let hit = ray_tri_hit(Vec3::new(0.5, 2.0, 0.25), -Vec3::Y, a, b, c).unwrap();
        let reconstructed =
            a * hit.barycentric[0] + b * hit.barycentric[1] + c * hit.barycentric[2];
        assert!((hit.distance - 2.0).abs() < 1.0e-6);
        assert!((hit.barycentric.iter().sum::<f32>() - 1.0).abs() < 1.0e-6);
        assert!(reconstructed.distance(Vec3::new(0.5, 0.0, 0.25)) < 1.0e-6);
        assert!(ray_tri_hit(Vec3::ZERO, Vec3::NAN, a, b, c).is_none());
    }

    #[test]
    fn topology_fingerprint_ignores_attributes_but_tracks_connectivity() {
        let mut mesh = Mesh::plane(2.0);
        let fingerprint = mesh.topology_fingerprint();
        for vertex in &mut mesh.verts {
            vertex.pos[1] += 3.0;
            vertex.color = [0.1, 0.2, 0.3];
        }
        mesh.faces[0].uv.fill([0.25, 0.75]);
        mesh.faces[0].selected = true;
        assert_eq!(mesh.topology_fingerprint(), fingerprint);

        mesh.faces[0].verts.swap(1, 2);
        assert_ne!(mesh.topology_fingerprint(), fingerprint);
    }

    #[test]
    fn subdivide_quad_grows() {
        let mut m = Mesh::cube(2.0);
        m.select_all();
        m.subdivide_selected();
        assert!(m.faces.len() > 6);
    }

    #[test]
    fn obj_roundtrip_with_uv() {
        let m = Mesh::cube(2.0);
        let s = m.to_obj();
        assert!(s.contains("vt "));
        let m2 = Mesh::from_obj(&s);
        assert_eq!(m2.verts.len(), 8);
        assert!(!m2.faces.is_empty());
    }

    #[test]
    fn ear_clip_concave() {
        // seta côncava: 7 pontos
        let pts = [
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 1.0],
            [1.0, 1.0],
            [1.0, 2.0],
            [0.0, 2.0],
            [0.5, 1.0],
        ];
        let tris = ear_clip(&pts).expect("triangula");
        assert_eq!(tris.len(), pts.len() - 2);
    }

    #[test]
    fn ear_clip_rejects_degenerate() {
        assert!(ear_clip(&[[0.0, 0.0], [1.0, 1.0]]).is_err());
    }

    #[test]
    fn revolve_vase_counts() {
        let profile = [[0.0, 0.0], [1.0, 0.0], [1.0, 2.0], [0.0, 2.0]];
        let m = Mesh::revolve(&profile, 8).expect("revolve");
        assert!(!m.faces.is_empty());
        assert!(m.faces.iter().all(|f| f.uv.len() == f.verts.len()));
    }

    #[test]
    fn revolve_partial_angle_counts() {
        let profile = [[0.0, 0.0], [1.0, 0.0], [1.0, 2.0], [0.0, 2.0]];
        let m = Mesh::revolve_angle(&profile, 8, 180.0).expect("revolve 180");
        assert!(!m.faces.is_empty());
        assert!(!m.verts.is_empty());
        assert!(m.faces.iter().all(|f| f.uv.len() == f.verts.len()));
    }

    #[test]
    fn mirror_welds_center() {
        let mut m = Mesh::plane(2.0);
        m.select_all();
        let before = m.vert_count();
        m.mirror(0, 0.001);
        // plano simétrico em X com weld: volta a ~4 verts (+ tolerância)
        assert!(m.vert_count() <= before + 4);
    }

    #[test]
    fn bevel_manifold_cube_edge() {
        let mut m = Mesh::cube(2.0);
        m.selected_edges.insert((0, 1));
        let (ok, skipped) = m.bevel_selected(0.2);
        assert_eq!(ok, 1);
        assert_eq!(skipped, 0);
        assert!(m.faces.len() > 6);
    }

    #[test]
    fn bevel_clamp_overlap_option() {
        let mut m_clamped = Mesh::cube(2.0);
        m_clamped.selected_edges.insert((0, 1));
        let (ok1, skip1) = m_clamped.bevel_selected_full(1.8, 1, true);
        assert_eq!(ok1, 1);
        assert_eq!(skip1, 0);

        let mut m_unclamped = Mesh::cube(2.0);
        m_unclamped.selected_edges.insert((0, 1));
        let (ok2, skip2) = m_unclamped.bevel_selected_full(1.8, 1, false);
        assert_eq!(ok2, 1);
        assert_eq!(skip2, 0);

        let pos_clamped = m_clamped.verts.last().unwrap().pos;
        let pos_unclamped = m_unclamped.verts.last().unwrap().pos;
        assert_ne!(pos_clamped, pos_unclamped);
    }

    #[test]
    fn bevel_vertex_option() {
        let mut m = Mesh::cube(2.0);
        m.selected_edges.clear();
        m.verts[0].selected = true;
        let (ok, skip) = m.bevel_selected_full(0.3, 1, true);
        assert_eq!(ok, 1);
        assert_eq!(skip, 0);
        let report = m.validate_topology();
        assert!(report.is_manifold && report.is_closed);
        assert_eq!((m.verts.len(), m.faces.len()), (10, 7));
    }

    #[test]
    fn multi_edge_and_multi_vertex_bevel_batch() {
        // 1. Multi-edge: seleciona 4 arestas paralelas verticais do cubo
        let mut m_edges = Mesh::cube(2.0);
        m_edges
            .selected_edges
            .extend([(0, 3), (1, 2), (4, 7), (5, 6)]);
        let (ok_edges, skip_edges) = m_edges.bevel_selected_full(0.2, 1, true);
        assert_eq!(ok_edges, 4);
        assert_eq!(skip_edges, 0);
        let rep_edges = m_edges.validate_topology();
        assert!(rep_edges.is_manifold && rep_edges.is_closed);
        assert_eq!((m_edges.verts.len(), m_edges.faces.len()), (16, 10));

        // 2. Multi-vertex: seleciona todos os 8 cantos do cubo
        let mut m_verts = Mesh::cube(2.0);
        m_verts.selected_edges.clear();
        for v in &mut m_verts.verts {
            v.selected = true;
        }
        let (ok_verts, skip_verts) = m_verts.bevel_selected_full(0.2, 1, true);
        assert_eq!(ok_verts, 8);
        assert_eq!(skip_verts, 0);
        let rep_verts = m_verts.validate_topology();
        assert!(rep_verts.is_manifold && rep_verts.is_closed);
        assert_eq!((m_verts.verts.len(), m_verts.faces.len()), (24, 14));
    }

    #[test]
    fn extrude_keeps_uv_invariant() {
        let mut m = Mesh::cube(2.0);
        m.select_all();
        m.extrude_selected(0.5);
        assert!(m.faces.iter().all(|f| f.uv.len() == f.verts.len()));
    }

    #[test]
    fn capsule_is_closedish() {
        let m = Mesh::capsule(8, 0.5, 2.0);
        assert!(m.vert_count() > 10 && !m.faces.is_empty());
    }

    /// Guarda de winding: normais das primitivas convexas apontam p/ fora.
    /// (Viewers com culling escondem faces invertidas — bug crítico p/ export.)
    #[test]
    fn primitives_wind_outward() {
        use glam::Vec3;
        let prims: Vec<Mesh> = vec![
            Mesh::cube(2.0),
            Mesh::plane(2.0),
            Mesh::cylinder(8, 1.0, 2.0),
            Mesh::cone(8, 1.0, 2.0),
            Mesh::sphere_low(10, 7, 1.2),
            Mesh::capsule(8, 0.5, 2.0),
            Mesh::revolve(&[[0.0, -1.0], [1.0, -1.0], [1.0, 1.0], [0.0, 1.0]], 10).unwrap(),
        ];
        for (pi, m) in prims.iter().enumerate() {
            let mut center = Vec3::ZERO;
            for v in &m.verts {
                center += v.vec();
            }
            center /= m.verts.len().max(1) as f32;
            for (fi, f) in m.faces.iter().enumerate() {
                let n = m.face_normal(fi);
                assert!(n.length() > 1e-6, "prim {pi} face {fi} degenerada");
                let mut c = Vec3::ZERO;
                for &vi in &f.verts {
                    c += m.verts[vi as usize].vec();
                }
                c /= f.verts.len() as f32;
                assert!(
                    n.dot(c - center) > -1e-4,
                    "prim {pi} face {fi} com normal para dentro"
                );
            }
        }
        // plano: convenção +Y (para cima)
        let plane = Mesh::plane(2.0);
        assert!(plane.face_normal(0).y > 0.99);
    }

    #[test]
    fn obj_hostile_indices_dropped_no_panic() {
        // M1: índices fora da malha são descartados, sem panic depois
        let m = Mesh::from_obj("v 0 0 0\nf 999999 999998 999997\n");
        assert!(m.faces.is_empty());
        // render/export helpers não panicam com malha vazia
        assert!(m.to_triangles().is_empty());
        assert_eq!(m.to_obj().matches("vt ").count(), 0);
    }

    #[test]
    fn obj_nan_rejected() {
        // M5: NaN/inf não entram na malha
        let m = Mesh::from_obj("v nan nan nan\nv inf 0 0\nv 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n");
        assert!(m.verts.iter().all(|v| v.pos.iter().all(|x| x.is_finite())));
        assert_eq!(m.faces.len(), 1);
    }

    #[test]
    fn validate_repairs_corrupt() {
        // M2: invariante quebrado é reparado sem panic
        let mut m = Mesh::cube(1.0);
        m.faces.push(Face {
            verts: vec![0, 1, 99],
            uv: vec![[0.0, 0.0]],
            selected: false,
            material_slot: None,
        });
        m.selected_edges.insert((0, 500));
        m.validate();
        assert!(m.faces.iter().all(|f| f.uv.len() == f.verts.len()));
        assert!(
            m.faces
                .iter()
                .all(|f| f.verts.iter().all(|&i| (i as usize) < m.verts.len()))
        );
        assert!(
            m.selected_edges
                .iter()
                .all(|&(a, b)| (a as usize) < m.verts.len() && (b as usize) < m.verts.len())
        );
    }

    #[test]
    fn flip_normals_inverts_face_normal() {
        let mut m = Mesh::plane(2.0);
        let n_before = m.face_normal(0);
        assert!(n_before.y > 0.9);
        m.flip_normals();
        let n_after = m.face_normal(0);
        assert!(n_after.y < -0.9);
    }

    #[test]
    fn recalculate_normals_unifies_inverted_cube() {
        let mut m = Mesh::cube(2.0);
        // Inverte propositalmente a face 0
        m.faces[0].verts.reverse();
        m.faces[0].uv.reverse();
        m.recalculate_normals();
        // Todas as normais devem apontar para fora do centro
        for fi in 0..m.faces.len() {
            let n = m.face_normal(fi);
            let c = m.face_centroid(fi);
            assert!(n.dot(c) > 0.0, "Face {} normal should point outward", fi);
        }
    }

    #[test]
    fn slice_plane_bisects_cube() {
        use glam::Vec3;
        let mut m = Mesh::cube(2.0);
        // Corta o cubo no plano Y = 0
        m.slice_plane(Vec3::ZERO, Vec3::Y, true);
        assert_eq!(m.faces.len(), 6);
        assert_eq!(m.verts.len(), 8);
        assert!(m.verts.iter().all(|v| v.pos[1] >= 0.0));
        let report = m.validate_topology();
        assert!(report.is_manifold && report.is_closed, "{report:?}");
    }

    #[test]
    fn sweep_polyline_creates_mesh() {
        use glam::Vec3;
        let mut m = Mesh::default();
        let profile = [[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]];
        let path = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 2.0, 0.0),
            Vec3::new(2.0, 4.0, 0.0),
        ];
        let res = m.sweep(&profile, &path, true);
        assert!(res.is_ok());
        assert!(!m.faces.is_empty());
        assert!(m.faces.iter().all(|f| f.uv.len() == f.verts.len()));
    }

    #[test]
    fn join_and_fuse_approximate() {
        let mut m1 = Mesh::cube(2.0);
        let m2 = Mesh::cube(2.0);
        let count1 = m1.verts.len();
        m1.join(&m2);
        assert_eq!(m1.verts.len(), count1 * 2);
        assert_eq!(m1.faces.len(), 12);
        // Funde vértices idênticos com eps
        m1.weld(0.001);
        assert_eq!(m1.verts.len(), count1);
    }

    #[test]
    fn connect_loops_bridges_faces() {
        let mut m1 = Mesh::plane(2.0);
        let mut m2 = Mesh::plane(2.0);
        m2.translate_selected([0.0, 3.0, 0.0]);
        m1.join(&m2);
        assert_eq!(m1.faces.len(), 2);
        let res = m1.connect_loops(0, 1);
        assert!(res.is_ok());
        // Deve ter criado 4 quads laterais e removido as 2 faces das pontas
        assert_eq!(m1.faces.len(), 4);
    }

    #[test]
    fn connect_loops_with_segments_and_twist() {
        let mut m1 = Mesh::plane(2.0);
        let mut m2 = Mesh::plane(2.0);
        m2.translate_selected([0.0, 3.0, 0.0]);
        m1.join(&m2);
        let res = m1.connect_loops_with_options(0, 1, 3, 1);
        assert!(res.is_ok());
        // Com 3 segmentos em torno de 4 arestas, deve gerar 4 * 3 = 12 faces
        assert_eq!(m1.faces.len(), 12);
    }

    #[test]
    fn dissolve_selected_edge_merges_faces() {
        let mut m = Mesh::plane(2.0);
        // Subdivide o plano para ter aresta interna
        m.select_all();
        m.subdivide_selected();
        let faces_before = m.faces.len();
        assert_eq!(faces_before, 4);
        // Seleciona a primeira aresta interna compartilhada por 2 faces
        let internal_edge = m
            .edges_unique()
            .into_iter()
            .find(|&(a, b)| m.edge_faces(a, b).len() == 2);
        assert!(internal_edge.is_some());
        m.selected_edges.clear();
        m.selected_edges.insert(internal_edge.unwrap());
        m.dissolve_selected();
        assert_eq!(m.faces.len(), faces_before - 1);
    }

    #[test]
    fn dissolve_selected_faces_merges_faces() {
        let mut m = Mesh::plane(2.0);
        m.select_all();
        m.subdivide_selected();
        assert_eq!(m.faces.len(), 4);
        // Seleciona duas faces adjacentes
        m.deselect_all();
        m.faces[0].selected = true;
        m.faces[1].selected = true;
        m.dissolve_selected();
        // As duas faces devem se fundir em 1, restando 3 faces no plano
        assert_eq!(m.faces.len(), 3);
    }

    #[test]
    fn dissolve_selected_vertex_merges_fan() {
        let mut m = Mesh::plane(2.0);
        m.select_all();
        m.subdivide_selected();
        assert_eq!(m.faces.len(), 4);
        // O vértice central tem índice 8 (ou valência 4)
        let center_vert = (0..m.verts.len()).find(|&v| {
            m.faces
                .iter()
                .filter(|f| f.verts.contains(&(v as u32)))
                .count()
                == 4
        });
        assert!(center_vert.is_some());
        m.deselect_all();
        m.verts[center_vert.unwrap()].selected = true;
        m.dissolve_selected();
        // Dissolver o vértice central deve fundir as 4 faces em 1 polígono
        assert_eq!(m.faces.len(), 1);
    }

    #[test]
    fn equalize_texel_density_scales_islands() {
        use std::collections::HashSet;
        let mut m1 = Mesh::plane(2.0);
        let mut m2 = Mesh::plane(2.0);
        m1.faces[0].uv = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        m2.faces[0].uv = vec![[0.0, 0.0], [0.5, 0.0], [0.5, 0.5], [0.0, 0.5]];
        m2.translate_selected([10.0, 0.0, 0.0]);
        m1.join(&m2);
        assert_eq!(m1.faces.len(), 2);
        assert_eq!(m1.uv_islands().len(), 2);

        let count = m1.equalize_texel_density(&HashSet::new());
        assert!(count >= 1);
        let islands = m1.uv_islands();
        let area0 = islands[0].area();
        let area1 = islands[1].area();
        assert!((area0 - area1).abs() < 0.1, "area0={area0}, area1={area1}");
    }
}
