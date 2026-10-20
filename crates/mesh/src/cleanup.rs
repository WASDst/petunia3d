//! Cleanup explícito de malha (website/docs/03-geometry/validation-repair-cleanup.md §3–§8).
//!
//! Cada ação é separada, determinística e devolve [`TopologyResult`] com o remap de
//! vértices/faces para que seleção, UV e attachments sejam atualizados pela Application.
//! Cleanup **não** é validação e não faz parte de import: é mutação intencional.

use std::collections::{HashMap, HashSet};

use glam::Vec3;

use crate::topology::{DirtyDomains, ElementRemap, TopologyResult};
use crate::{Face, Mesh, edge_key, triangulate};

const UV_EPSILON: f32 = 1.0e-5;
const MAX_DISSOLVE_PASSES: usize = 64;

/// Diagonal da caixa envolvente (escala de referência para epsilons relativos).
fn scale_of(mesh: &Mesh) -> f32 {
    if mesh.verts.is_empty() {
        return 1.0;
    }
    let mut min = Vec3::splat(f32::MAX);
    let mut max = Vec3::splat(f32::MIN);
    for v in &mesh.verts {
        if v.vec().is_finite() {
            min = min.min(v.vec());
            max = max.max(v.vec());
        }
    }
    if min.x > max.x {
        return 1.0;
    }
    (max - min).length().max(1.0e-6)
}

fn face_area(mesh: &Mesh, face: &Face) -> f32 {
    let n = face.verts.len();
    if n < 3 {
        return 0.0;
    }
    let mut acc = Vec3::ZERO;
    for k in 0..n {
        let a = mesh.verts[face.verts[k] as usize].vec();
        let b = mesh.verts[face.verts[(k + 1) % n] as usize].vec();
        acc += a.cross(b);
    }
    acc.length() * 0.5
}

/// Reindexa vértices/faces e reaplica restrições (seams, sharp, edges selecionadas, pins).
fn apply_compaction(
    mesh: &mut Mesh,
    vertex_map: &[Option<u32>],
    face_map: &[Option<usize>],
) -> usize {
    let remap_pair = |(a, b): (u32, u32)| -> Option<(u32, u32)> {
        let na = vertex_map.get(a as usize).copied().flatten()?;
        let nb = vertex_map.get(b as usize).copied().flatten()?;
        (na != nb).then(|| edge_key(na, nb))
    };
    let mut dropped = 0;
    for set in [
        &mut mesh.uv_seams,
        &mut mesh.sharp_edges,
        &mut mesh.selected_edges,
    ] {
        let old: Vec<_> = set.drain().collect();
        for edge in old {
            match remap_pair(edge) {
                Some(new) => {
                    set.insert(new);
                }
                None => dropped += 1,
            }
        }
    }
    let old_pins: Vec<_> = mesh.uv_pinned.drain().collect();
    for (face, corner) in old_pins {
        match face_map.get(face).copied().flatten() {
            Some(new_face) => {
                mesh.uv_pinned.insert((new_face, corner));
            }
            None => dropped += 1,
        }
    }
    dropped
}

impl Mesh {
    /// Remove faces degeneradas: menos de 3 vértices distintos, índice inválido ou área
    /// abaixo de `relative_epsilon · escala²` (escala = diagonal da malha, não um literal rígido).
    pub fn remove_degenerate_faces(&mut self, relative_epsilon: f32) -> TopologyResult {
        let eps = if relative_epsilon.is_finite() && relative_epsilon >= 0.0 {
            relative_epsilon
        } else {
            0.0
        };
        let scale = scale_of(self);
        let area_floor = eps * scale * scale;
        let vertex_count = self.verts.len();
        let faces_before = self.faces.len();
        let keep: Vec<bool> = self
            .faces
            .iter()
            .map(|face| {
                let distinct: HashSet<u32> = face.verts.iter().copied().collect();
                distinct.len() >= 3
                    && face.verts.iter().all(|&v| (v as usize) < vertex_count)
                    && face_area(self, face) > area_floor
            })
            .collect();
        if keep.iter().all(|&k| k) {
            return TopologyResult::none();
        }
        let mut face_map = vec![None; faces_before];
        let mut next = 0usize;
        for (old, &k) in keep.iter().enumerate() {
            if k {
                face_map[old] = Some(next);
                next += 1;
            }
        }
        let old_faces = std::mem::take(&mut self.faces);
        self.faces = old_faces
            .into_iter()
            .zip(&keep)
            .filter_map(|(face, &k)| k.then_some(face))
            .collect();
        let vertex_map: Vec<Option<u32>> = (0..vertex_count as u32).map(Some).collect();
        apply_compaction(self, &vertex_map, &face_map);
        TopologyResult {
            faces_removed: faces_before - self.faces.len(),
            remap: ElementRemap::new(
                Vec::new(),
                face_map.iter().enumerate().map(|(i, m)| (i, *m)).collect(),
            ),
            invalidated: DirtyDomains::topology_edit(),
            ..TopologyResult::none()
        }
    }

    /// Remove vértices que nenhuma face usa e reindexa os restantes.
    pub fn remove_orphan_vertices(&mut self) -> TopologyResult {
        let vertex_count = self.verts.len();
        let mut used = vec![false; vertex_count];
        for face in &self.faces {
            for &v in &face.verts {
                if let Some(slot) = used.get_mut(v as usize) {
                    *slot = true;
                }
            }
        }
        if used.iter().all(|&u| u) {
            return TopologyResult::none();
        }
        let mut vertex_map: Vec<Option<u32>> = vec![None; vertex_count];
        let mut kept = Vec::new();
        for (old, vertex) in self.verts.iter().enumerate() {
            if used[old] {
                vertex_map[old] = Some(kept.len() as u32);
                kept.push(vertex.clone());
            }
        }
        for face in &mut self.faces {
            for v in &mut face.verts {
                *v = vertex_map[*v as usize].unwrap_or(*v);
            }
        }
        let removed = vertex_count - kept.len();
        self.verts = kept;
        let face_map: Vec<Option<usize>> = (0..self.faces.len()).map(Some).collect();
        apply_compaction(self, &vertex_map, &face_map);
        TopologyResult {
            verts_removed: removed,
            remap: ElementRemap::new(
                vertex_map
                    .iter()
                    .enumerate()
                    .map(|(i, m)| (i as u32, *m))
                    .collect(),
                Vec::new(),
            ),
            invalidated: DirtyDomains::topology_edit(),
            ..TopologyResult::none()
        }
    }

    /// Limited Dissolve conservador: funde duas faces coplanares através de uma aresta
    /// interna quando **todas** as condições valem; qualquer ambiguidade impede o merge.
    ///
    /// Condições: exatamente duas faces com winding oposto na aresta; normais dentro de
    /// `angle_deg`; mesmo material slot; aresta não é UV seam nem Sharp; UVs coincidem nos
    /// extremos; sem pins; o polígono resultante é simples e triangulável.
    pub fn limited_dissolve(&mut self, angle_deg: f32) -> TopologyResult {
        if !angle_deg.is_finite() || angle_deg < 0.0 {
            return TopologyResult::none();
        }
        let cos_limit = angle_deg.min(90.0).to_radians().cos();
        let faces_before = self.faces.len();
        let mut origin: Vec<Vec<usize>> = (0..faces_before).map(|f| vec![f]).collect();
        let mut merged_edges = 0usize;

        for _ in 0..MAX_DISSOLVE_PASSES {
            let merged_now = self.dissolve_pass(cos_limit, &mut origin);
            merged_edges += merged_now;
            if merged_now == 0 {
                break;
            }
        }
        if merged_edges == 0 {
            return TopologyResult::none();
        }

        // `origin[new]` lista as faces antigas fundidas em cada face nova.
        let mut face_map = vec![None; faces_before];
        for (new, olds) in origin.iter().enumerate() {
            for &old in olds {
                face_map[old] = Some(new);
            }
        }
        let vertex_map: Vec<Option<u32>> = (0..self.verts.len() as u32).map(Some).collect();
        apply_compaction(self, &vertex_map, &face_map);
        TopologyResult {
            faces_removed: faces_before - self.faces.len(),
            remap: ElementRemap::new(
                Vec::new(),
                face_map.iter().enumerate().map(|(i, m)| (i, *m)).collect(),
            ),
            uv_faces_touched: (0..self.faces.len()).collect(),
            invalidated: DirtyDomains::topology_edit(),
            ..TopologyResult::none()
        }
    }

    /// Uma passada gulosa e determinística. Devolve quantas arestas foram dissolvidas.
    fn dissolve_pass(&mut self, cos_limit: f32, origin: &mut Vec<Vec<usize>>) -> usize {
        // Aresta direcionada -> (face, posição do corner de origem).
        let mut directed: HashMap<(u32, u32), Vec<(usize, usize)>> = HashMap::new();
        for (fi, face) in self.faces.iter().enumerate() {
            let n = face.verts.len();
            if n < 3 {
                continue;
            }
            for k in 0..n {
                directed
                    .entry((face.verts[k], face.verts[(k + 1) % n]))
                    .or_default()
                    .push((fi, k));
            }
        }
        let mut candidates: Vec<(u32, u32)> =
            directed.keys().filter(|&&(u, v)| u < v).copied().collect();
        candidates.sort_unstable();

        let mut touched: HashSet<usize> = HashSet::new();
        let mut replacements: Vec<(usize, usize, Face)> = Vec::new();
        for (u, v) in candidates {
            let (Some(forward), Some(backward)) = (directed.get(&(u, v)), directed.get(&(v, u)))
            else {
                continue;
            };
            // Exatamente duas faces, uma em cada sentido: qualquer outra coisa é ambígua.
            if forward.len() != 1 || backward.len() != 1 {
                continue;
            }
            let (fa, ia) = forward[0];
            let (fb, ib) = backward[0];
            if fa == fb || touched.contains(&fa) || touched.contains(&fb) {
                continue;
            }
            if let Some(merged) = self.try_merge(fa, ia, fb, ib, (u, v), cos_limit) {
                touched.insert(fa);
                touched.insert(fb);
                replacements.push((fa, fb, merged));
            }
        }
        if replacements.is_empty() {
            return 0;
        }
        let merged_count = replacements.len();
        let mut removed: HashSet<usize> = HashSet::new();
        for (fa, fb, face) in replacements {
            self.faces[fa] = face;
            removed.insert(fb);
            let absorbed = std::mem::take(&mut origin[fb]);
            origin[fa].extend(absorbed);
        }
        let old_faces = std::mem::take(&mut self.faces);
        let old_origin = std::mem::take(origin);
        for (fi, (face, olds)) in old_faces.into_iter().zip(old_origin).enumerate() {
            if !removed.contains(&fi) {
                self.faces.push(face);
                origin.push(olds);
            }
        }
        merged_count
    }

    /// Monta o polígono fundido de A e B pela aresta `(u, v)`, ou `None` se algo for ambíguo.
    fn try_merge(
        &self,
        fa: usize,
        ia: usize,
        fb: usize,
        ib: usize,
        edge: (u32, u32),
        cos_limit: f32,
    ) -> Option<Face> {
        let (a, b) = (&self.faces[fa], &self.faces[fb]);
        if a.material_slot != b.material_slot
            || a.uv.len() != a.verts.len()
            || b.uv.len() != b.verts.len()
            || self.uv_seams.contains(&edge_key(edge.0, edge.1))
            || self.sharp_edges.contains(&edge_key(edge.0, edge.1))
        {
            return None;
        }
        if self
            .uv_pinned
            .iter()
            .any(|&(face, _)| face == fa || face == fb)
        {
            return None;
        }
        let na = self.face_normal(fa);
        let nb = self.face_normal(fb);
        if na.dot(nb) < cos_limit {
            return None;
        }
        let (n_a, n_b) = (a.verts.len(), b.verts.len());
        let (u, v) = edge;
        // A: ... u(ia) v(ia+1) ...   B: ... v(ib) u(ib+1) ...
        let uv_close = |p: [f32; 2], q: [f32; 2]| {
            (p[0] - q[0]).abs() <= UV_EPSILON && (p[1] - q[1]).abs() <= UV_EPSILON
        };
        if !uv_close(a.uv[(ia + 1) % n_a], b.uv[ib]) || !uv_close(a.uv[ia], b.uv[(ib + 1) % n_b]) {
            return None;
        }
        let mut verts = Vec::with_capacity(n_a + n_b - 2);
        let mut uvs = Vec::with_capacity(n_a + n_b - 2);
        for step in 1..=n_a {
            let k = (ia + step) % n_a;
            verts.push(a.verts[k]);
            uvs.push(a.uv[k]);
        }
        for step in 2..n_b {
            let k = (ib + step) % n_b;
            verts.push(b.verts[k]);
            uvs.push(b.uv[k]);
        }
        debug_assert_eq!(verts.first(), Some(&v));
        debug_assert!(verts.contains(&u));
        // Polígono simples: sem vértice repetido.
        let distinct: HashSet<u32> = verts.iter().copied().collect();
        if distinct.len() != verts.len() {
            return None;
        }
        if !self.ring_is_simple_and_triangulable(&verts) {
            return None;
        }
        let mut merged = Face::with_uv(verts, uvs);
        merged.selected = a.selected && b.selected;
        merged.material_slot = a.material_slot;
        Some(merged)
    }

    /// O anel é triangulável e a soma das áreas dos triângulos bate com a área do
    /// polígono (rejeita auto-interseção e anéis degenerados).
    fn ring_is_simple_and_triangulable(&self, ring: &[u32]) -> bool {
        let normal = triangulate::face_normal_of(&self.verts, ring).abs();
        let axis = if normal.x >= normal.y && normal.x >= normal.z {
            0
        } else if normal.y >= normal.z {
            1
        } else {
            2
        };
        let points: Vec<[f32; 2]> = ring
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
        let area = triangulate::polygon_area(&points).abs();
        if area <= 1.0e-12 {
            return false;
        }
        let Ok(triangles) = triangulate::ear_clip(&points) else {
            return false;
        };
        let tri_area: f32 = triangles
            .iter()
            .map(|t| triangulate::polygon_area(&[points[t[0]], points[t[1]], points[t[2]]]).abs())
            .sum();
        (tri_area - area).abs() <= area * 1.0e-3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Face, Vertex};

    fn quad_split_in_two_triangles() -> Mesh {
        let mut mesh = Mesh::default();
        for p in [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ] {
            mesh.verts.push(Vertex::new(p[0], p[1], p[2]));
        }
        let uv = |i: usize| [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]][i];
        mesh.faces
            .push(Face::with_uv(vec![0, 1, 2], vec![uv(0), uv(1), uv(2)]));
        mesh.faces
            .push(Face::with_uv(vec![0, 2, 3], vec![uv(0), uv(2), uv(3)]));
        mesh
    }

    #[test]
    fn coplanar_triangles_dissolve_into_one_quad_keeping_uv() {
        let mut mesh = quad_split_in_two_triangles();
        let result = mesh.limited_dissolve(1.0);
        assert_eq!(mesh.faces.len(), 1);
        assert_eq!(result.faces_removed, 1);
        assert_eq!(mesh.faces[0].verts.len(), 4);
        assert_eq!(mesh.faces[0].uv.len(), 4);
        assert!(result.remap.faces.iter().all(|(_, to)| *to == Some(0)));
    }

    #[test]
    fn dissolve_respects_uv_seams_sharp_and_material_slots() {
        let mut seam = quad_split_in_two_triangles();
        seam.uv_seams.insert(edge_key(0, 2));
        assert_eq!(seam.limited_dissolve(1.0), TopologyResult::none());
        assert_eq!(seam.faces.len(), 2);

        let mut sharp = quad_split_in_two_triangles();
        sharp.mark_sharp([(0, 2)]);
        assert_eq!(sharp.limited_dissolve(1.0), TopologyResult::none());

        let mut slots = quad_split_in_two_triangles();
        slots.faces[0].material_slot = Some(0);
        slots.faces[1].material_slot = Some(1);
        assert_eq!(slots.limited_dissolve(1.0), TopologyResult::none());
    }

    #[test]
    fn dissolve_keeps_creased_geometry() {
        let mut cube = Mesh::cube(1.0);
        let before = cube.faces.len();
        cube.triangulate();
        let triangulated = cube.faces.len();
        assert!(triangulated > before);
        cube.limited_dissolve(1.0);
        // Cada face lateral volta a ser um quad; as arestas vivas do cubo permanecem.
        assert_eq!(cube.faces.len(), before);
    }

    #[test]
    fn dissolve_refuses_when_uvs_disagree_along_the_edge() {
        let mut mesh = quad_split_in_two_triangles();
        mesh.faces[1].uv[0] = [0.5, 0.5];
        assert_eq!(mesh.limited_dissolve(1.0), TopologyResult::none());
    }

    #[test]
    fn dissolve_refuses_non_coplanar_faces() {
        let mut mesh = quad_split_in_two_triangles();
        mesh.verts[2].pos[2] = 1.0;
        assert_eq!(mesh.limited_dissolve(1.0), TopologyResult::none());
    }

    #[test]
    fn degenerate_faces_use_a_scale_relative_floor() {
        let mut mesh = quad_split_in_two_triangles();
        mesh.verts.push(Vertex::new(0.5, 0.5, 0.0));
        mesh.faces.push(Face::new(vec![0, 0, 4])); // menos de 3 vértices distintos
        mesh.faces.push(Face::new(vec![0, 1, 99])); // índice inválido
        let result = mesh.remove_degenerate_faces(1.0e-6);
        assert_eq!(result.faces_removed, 2);
        assert_eq!(mesh.faces.len(), 2);
    }

    #[test]
    fn orphan_vertices_are_removed_and_constraints_remapped() {
        let mut mesh = quad_split_in_two_triangles();
        mesh.verts.insert(0, Vertex::new(9.0, 9.0, 9.0));
        for face in &mut mesh.faces {
            for v in &mut face.verts {
                *v += 1;
            }
        }
        mesh.uv_seams.insert(edge_key(1, 3));
        let result = mesh.remove_orphan_vertices();
        assert_eq!(result.verts_removed, 1);
        assert_eq!(mesh.verts.len(), 4);
        assert!(mesh.uv_seams.contains(&edge_key(0, 2)));
        assert_eq!(result.remap.map_vertex(0), None);
        assert_eq!(result.remap.map_vertex(1), Some(0));
    }

    #[test]
    fn cleanup_is_a_no_op_on_a_clean_mesh() {
        let mut cube = Mesh::cube(1.0);
        assert_eq!(cube.remove_degenerate_faces(1.0e-6), TopologyResult::none());
        assert_eq!(cube.remove_orphan_vertices(), TopologyResult::none());
    }
}
