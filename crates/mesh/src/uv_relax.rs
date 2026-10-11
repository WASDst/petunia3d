//! Relax UV (V02; website/docs/07-uv/architecture-decision.md, "Relax").
//!
//! Substitui o antigo smoothing laplaciano por face (que encolhia ilhas e não
//! minimizava stretch) por uma relaxação **as-rigid-as-possible** leve:
//!
//! 1. por ilha, cantos ligados por edges sem seam e sem divisão de UV formam um
//!    vértice UV — seams e divisões de UV nunca se fundem;
//! 2. passo local: cada triângulo derivado acha a rotação 2D (Procrustes) que
//!    melhor leva a sua forma 3D planificada, na escala atual da ilha, até a UV
//!    atual;
//! 3. passo global (Jacobi): cada vértice UV vai para a média das posições que
//!    os seus triângulos pedem; vértices com pin ficam parados.
//!
//! Minimiza a distorção de ângulo e de área em relação à superfície 3D sem
//! mudar o tamanho da ilha (densidade preservada). Ilhas espelhadas continuam
//! espelhadas. Sem pins, a ilha mantém o centroide.

use std::collections::{HashMap, HashSet};

use glam::Vec2;

use super::{Mesh, edge_key};

/// Triângulo da ilha: vértices UV e forma 3D planificada (centrada).
struct RelaxTriangle {
    vertices: [usize; 3],
    reference: [Vec2; 3],
}

impl Mesh {
    /// Relaxa as ilhas que tocam `faces_filter` (vazio = todas) por
    /// `iterations` passos (limitado a `1..=50`). Retorna o número de faces das
    /// ilhas relaxadas; `0` quando nada pôde ser relaxado.
    pub fn relax_uv(&mut self, faces_filter: &HashSet<usize>, iterations: usize) -> usize {
        let iterations = iterations.clamp(1, 50);
        let mut cuts = self.uv_split_edges();
        cuts.extend(self.uv_seams.iter().copied());
        let mut relaxed = 0;
        for island in self.uv_islands() {
            if !faces_filter.is_empty() && !island.faces.iter().any(|f| faces_filter.contains(f)) {
                continue;
            }
            if self.relax_island(&island.faces, &cuts, iterations) {
                relaxed += island.faces.len();
            }
        }
        relaxed
    }

    /// Relaxa uma ilha. `false` quando ela é degenerada ou totalmente fixada.
    fn relax_island(
        &mut self,
        faces: &[usize],
        cuts: &HashSet<(u32, u32)>,
        iterations: usize,
    ) -> bool {
        // 1. Vértices UV: union-find de cantos através de edges internas da
        //    ilha que não são seam nem divisão de UV (edges não-manifold cortam).
        let mut corner_index: HashMap<(usize, usize), usize> = HashMap::new();
        let mut corners: Vec<(usize, usize)> = Vec::new();
        for &fi in faces {
            let face = &self.faces[fi];
            if face.uv.len() != face.verts.len() {
                return false;
            }
            for c in 0..face.verts.len() {
                corner_index.insert((fi, c), corners.len());
                corners.push((fi, c));
            }
        }
        // edge_key → (canto no extremo menor, canto no extremo maior) por face.
        let mut sides: HashMap<(u32, u32), Vec<(usize, usize)>> = HashMap::new();
        for &fi in faces {
            let verts = &self.faces[fi].verts;
            let n = verts.len();
            for k in 0..n {
                let (a, b) = (verts[k], verts[(k + 1) % n]);
                let key = edge_key(a, b);
                if cuts.contains(&key) {
                    continue;
                }
                let (lo, hi) = if a <= b {
                    (k, (k + 1) % n)
                } else {
                    ((k + 1) % n, k)
                };
                sides
                    .entry(key)
                    .or_default()
                    .push((corner_index[&(fi, lo)], corner_index[&(fi, hi)]));
            }
        }
        let mut parent: Vec<usize> = (0..corners.len()).collect();
        let mut keys: Vec<&(u32, u32)> = sides.keys().collect();
        keys.sort_unstable();
        for key in keys {
            if let [(a0, b0), (a1, b1)] = sides[key][..] {
                union(&mut parent, a0, a1);
                union(&mut parent, b0, b1);
            }
        }
        let mut root_id: HashMap<usize, usize> = HashMap::new();
        let mut corners_of: Vec<Vec<(usize, usize)>> = Vec::new();
        let mut uv: Vec<Vec2> = Vec::new();
        let mut pinned: Vec<bool> = Vec::new();
        let mut corner_vertex: HashMap<(usize, usize), usize> = HashMap::new();
        for (index, &(fi, c)) in corners.iter().enumerate() {
            let root = find(&mut parent, index);
            let id = *root_id.entry(root).or_insert_with(|| {
                uv.push(Vec2::from_array(self.faces[fi].uv[c]));
                corners_of.push(Vec::new());
                pinned.push(false);
                uv.len() - 1
            });
            corners_of[id].push((fi, c));
            pinned[id] |= self.uv_pinned.contains(&(fi, c));
            corner_vertex.insert((fi, c), id);
        }
        if pinned.iter().all(|p| *p) {
            return false;
        }

        // 2. Triângulos derivados com a forma 3D planificada e a escala da ilha.
        let mut triangles = Vec::new();
        let (mut world_area, mut uv_signed_area) = (0.0f32, 0.0f32);
        for &fi in faces {
            let face = &self.faces[fi];
            for corners in self.face_triangle_corners(fi) {
                let [a, b, c] = corners.map(|k| self.verts[face.verts[k] as usize].vec());
                let (e1, e2) = (b - a, c - a);
                let len = e1.length();
                let normal = e1.cross(e2);
                if len < 1.0e-12 || normal.length_squared() < 1.0e-24 {
                    continue;
                }
                let x = e1 / len;
                let y = normal.cross(e1).normalize();
                let reference = [
                    Vec2::ZERO,
                    Vec2::new(len, 0.0),
                    Vec2::new(e2.dot(x), e2.dot(y)),
                ];
                let vertices = corners.map(|k| corner_vertex[&(fi, k)]);
                let [ua, ub, uc] = vertices.map(|v| uv[v]);
                world_area += normal.length() * 0.5;
                uv_signed_area += (ub - ua).perp_dot(uc - ua) * 0.5;
                triangles.push(RelaxTriangle {
                    vertices,
                    reference,
                });
            }
        }
        if triangles.is_empty() || world_area < 1.0e-12 || uv_signed_area.abs() < 1.0e-12 {
            return false;
        }
        let scale = (uv_signed_area.abs() / world_area).sqrt();
        let mirror = uv_signed_area < 0.0;
        for tri in &mut triangles {
            let centroid = (tri.reference[0] + tri.reference[1] + tri.reference[2]) / 3.0;
            for p in &mut tri.reference {
                *p = (*p - centroid) * scale;
                if mirror {
                    p.y = -p.y;
                }
            }
        }

        // 3. Iterações local/global.
        let anchor = (!pinned.iter().any(|p| *p)).then(|| centroid(&uv));
        for _ in 0..iterations {
            let mut sum = vec![Vec2::ZERO; uv.len()];
            let mut count = vec![0u32; uv.len()];
            for tri in &triangles {
                let current = tri.vertices.map(|v| uv[v]);
                let center = (current[0] + current[1] + current[2]) / 3.0;
                // Rotação de Procrustes 2D: referência → UV atual.
                let (mut dot, mut cross) = (0.0f32, 0.0f32);
                for (p, c) in tri.reference.iter().zip(current) {
                    let q = c - center;
                    dot += p.dot(q);
                    cross += p.perp_dot(q);
                }
                let rotation = Vec2::new(dot, cross).try_normalize().unwrap_or(Vec2::X);
                for k in 0..3 {
                    sum[tri.vertices[k]] += center + rotation.rotate(tri.reference[k]);
                    count[tri.vertices[k]] += 1;
                }
            }
            for v in 0..uv.len() {
                if !pinned[v] && count[v] > 0 {
                    uv[v] = sum[v] / count[v] as f32;
                }
            }
            if let Some(anchor) = anchor {
                let drift = anchor - centroid(&uv);
                uv.iter_mut().for_each(|p| *p += drift);
            }
        }

        if uv.iter().any(|p| !p.is_finite()) {
            return false;
        }
        for (id, corners) in corners_of.iter().enumerate() {
            if pinned[id] {
                continue;
            }
            for &(fi, c) in corners {
                self.faces[fi].uv[c] = uv[id].to_array();
            }
        }
        true
    }
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// Une pelo menor representante (resultado independente da ordem de chamada).
fn union(parent: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (find(parent, a), find(parent, b));
    let (lo, hi) = if ra <= rb { (ra, rb) } else { (rb, ra) };
    parent[hi] = lo;
}

fn centroid(points: &[Vec2]) -> Vec2 {
    points.iter().copied().sum::<Vec2>() / points.len().max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Face, Vertex};

    /// Grade 2×2 de quads no plano XY com UV = posição / 2 (sem distorção).
    fn grid() -> Mesh {
        let verts = (0..9)
            .map(|i| Vertex::new((i % 3) as f32, (i / 3) as f32, 0.0))
            .collect();
        let quad = |a: u32| {
            let ids = vec![a, a + 1, a + 4, a + 3];
            let uv = ids
                .iter()
                .map(|&i| [(i % 3) as f32 * 0.5, (i / 3) as f32 * 0.5])
                .collect();
            Face::with_uv(ids, uv)
        };
        Mesh {
            verts,
            faces: vec![quad(0), quad(1), quad(3), quad(4)],
            ..Mesh::default()
        }
    }

    /// Move a UV do vértice central (4) em todas as faces.
    fn displace_center(m: &mut Mesh, to: [f32; 2]) {
        for face in &mut m.faces {
            if let Some(c) = face.verts.iter().position(|&v| v == 4) {
                face.uv[c] = to;
            }
        }
    }

    fn center_uv(m: &Mesh) -> [f32; 2] {
        let c = m.faces[0].verts.iter().position(|&v| v == 4).unwrap();
        m.faces[0].uv[c]
    }

    #[test]
    fn undistorted_island_is_a_fixed_point() {
        let mut m = grid();
        let before: Vec<_> = m.faces.iter().map(|f| f.uv.clone()).collect();
        assert_eq!(m.relax_uv(&HashSet::new(), 10), 4);
        for (face, uv) in m.faces.iter().zip(before) {
            for (a, b) in face.uv.iter().zip(uv) {
                assert!((a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4);
            }
        }
    }

    #[test]
    fn relax_reduces_stretch_and_pulls_the_interior_back() {
        let mut m = grid();
        // Fixa os quatro cantos externos para a resposta ser única.
        for (fi, corner) in [(0, 0), (1, 1), (2, 3), (3, 2)] {
            m.pin_uv(fi, corner);
        }
        displace_center(&mut m, [0.8, 0.7]);
        let before = m.uv_diagnostics().mean_stretch;
        m.relax_uv(&HashSet::new(), 30);
        let after = m.uv_diagnostics().mean_stretch;
        assert!(after < before * 0.5, "before={before} after={after}");
        let c = center_uv(&m);
        assert!(
            (c[0] - 0.5).abs() < 0.05 && (c[1] - 0.5).abs() < 0.05,
            "{c:?}"
        );
    }

    #[test]
    fn pinned_corners_do_not_move_and_fully_pinned_islands_are_skipped() {
        let mut m = grid();
        displace_center(&mut m, [0.8, 0.7]);
        for (fi, face) in m.faces.iter().enumerate() {
            if let Some(c) = face.verts.iter().position(|&v| v == 4) {
                m.uv_pinned.insert((fi, c));
            }
        }
        m.relax_uv(&HashSet::new(), 10);
        assert_eq!(center_uv(&m), [0.8, 0.7]);

        let mut all = grid();
        all.pin_selected_faces_uv(&HashSet::new());
        assert_eq!(all.relax_uv(&HashSet::new(), 10), 0);
    }

    #[test]
    fn uv_split_corners_are_never_merged() {
        let mut m = grid();
        // Separa a face 3 (vértices 4,5,8,7) numa região própria da textura.
        for uv in &mut m.faces[3].uv {
            uv[0] += 2.0;
        }
        m.relax_uv(&HashSet::new(), 10);
        let min_u = m.faces[3]
            .uv
            .iter()
            .map(|uv| uv[0])
            .fold(f32::MAX, f32::min);
        assert!(min_u > 1.5, "split island was pulled back: {min_u}");
    }
}
