//! Inset métrico Region/Individual (G16; website/docs/03-geometry/
//! direct-modeling-semantics.md, "Inset", "Attributes e provenance").
//!
//! - **Region** (padrão): faces selecionadas coplanares e conectadas formam uma
//!   região; só os laços de borda dela ganham anel — faces internas não criam
//!   bordas. Ilhas desconectadas são independentes. Buracos: a mesma mitra
//!   relativa à área preenchida leva a borda do buraco para fora dele.
//! - **Individual:** cada face é a própria região (comportamento face a face).
//! - Distância em unidades de mundo. O clamp (colapso de aresta, inversão de
//!   orientação) é reportado: `requested` × `applied`, nunca silencioso.
//! - Provenance: cantos movidos recebem UV pelo mapa planar da própria face
//!   de origem; o anel herda UV dos cantos de origem e o material slot da
//!   face vizinha; seams/sharp de edges internas seguem os vértices movidos.
//! - Topologia que a região não suporta (vértice "gravata-borboleta", edge
//!   compartilhada por mais de duas faces) é recusada por região, com
//!   contagem no relatório — nunca adivinhada.

use std::collections::{HashMap, HashSet};

use glam::Vec3;

use super::{Face, Mesh, Vertex, edge_key};
use crate::ops::planar_uv_map;

/// Como a seleção vira regiões de inset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InsetMode {
    #[default]
    Region,
    Individual,
}

/// Resultado do inset.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InsetReport {
    /// Distância pedida (unidades de mundo).
    pub requested: f32,
    /// Menor distância efetivamente aplicada entre as regiões (0 se nada).
    pub applied: f32,
    /// `true` quando alguma região foi limitada abaixo do pedido.
    pub limited: bool,
    /// Faces das regiões (agora internas) — a seleção útil depois do inset.
    pub inner_faces: Vec<usize>,
    /// Faces do anel criadas.
    pub ring_faces: Vec<usize>,
    /// Regiões recusadas por topologia não suportada.
    pub skipped_regions: usize,
}

/// Uma edge de borda dirigida pelo winding da face de origem.
#[derive(Clone, Copy)]
struct BoundaryEdge {
    from: u32,
    to: u32,
    face: usize,
    corner: usize,
}

impl Mesh {
    /// Inset das `faces` a `distance` unidades de mundo. Faces inválidas
    /// (menos de 3 cantos, UV incompleta, normal nula) são ignoradas.
    pub fn inset_faces(&mut self, faces: &[usize], distance: f32, mode: InsetMode) -> InsetReport {
        let mut report = InsetReport {
            requested: distance,
            ..InsetReport::default()
        };
        if !distance.is_finite() || distance <= 0.0 {
            return report;
        }
        let mut candidates: Vec<usize> = faces
            .iter()
            .copied()
            .filter(|&fi| {
                self.faces.get(fi).is_some_and(|f| {
                    f.verts.len() >= 3
                        && f.uv.len() == f.verts.len()
                        && f.verts.iter().all(|&v| (v as usize) < self.verts.len())
                }) && self.face_normal(fi).length_squared() > 1.0e-12
            })
            .collect();
        candidates.sort_unstable();
        candidates.dedup();

        let mut applied = f32::INFINITY;
        for region in self.inset_regions(&candidates, mode) {
            match self.inset_region(&region, distance) {
                Some((d, ring)) => {
                    applied = applied.min(d);
                    report.limited |= d < distance * (1.0 - 1.0e-4);
                    report.inner_faces.extend(&region);
                    report.ring_faces.extend(ring);
                }
                None => report.skipped_regions += 1,
            }
        }
        report.applied = if applied.is_finite() { applied } else { 0.0 };
        if report.applied == 0.0 && !candidates.is_empty() {
            report.limited = true;
        }
        report.inner_faces.sort_unstable();

        // Seleção depois da operação: as regiões internas.
        let inner: HashSet<usize> = report.inner_faces.iter().copied().collect();
        for (fi, face) in self.faces.iter_mut().enumerate() {
            face.selected = inner.contains(&fi);
        }
        self.sync_vert_selection_from_faces();
        report
    }

    /// Agrupa as faces: Individual = uma por região; Region = componentes
    /// conectados por edge entre faces com a mesma normal.
    fn inset_regions(&self, faces: &[usize], mode: InsetMode) -> Vec<Vec<usize>> {
        if mode == InsetMode::Individual {
            return faces.iter().map(|&f| vec![f]).collect();
        }
        let index: HashMap<usize, usize> = faces.iter().enumerate().map(|(i, &f)| (f, i)).collect();
        let normals: Vec<Vec3> = faces
            .iter()
            .map(|&f| self.face_normal(f).normalize_or_zero())
            .collect();
        let mut parent: Vec<usize> = (0..faces.len()).collect();
        fn find(parent: &mut [usize], mut i: usize) -> usize {
            while parent[i] != i {
                parent[i] = parent[parent[i]];
                i = parent[i];
            }
            i
        }
        let mut by_edge: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for &f in faces {
            let verts = &self.faces[f].verts;
            for k in 0..verts.len() {
                by_edge
                    .entry(edge_key(verts[k], verts[(k + 1) % verts.len()]))
                    .or_default()
                    .push(f);
            }
        }
        let mut edges: Vec<_> = by_edge.into_iter().collect();
        edges.sort_unstable_by_key(|(e, _)| *e);
        for (_, users) in edges {
            if let [a, b] = users[..] {
                let (ia, ib) = (index[&a], index[&b]);
                if normals[ia].dot(normals[ib]) >= 1.0 - 1.0e-4 {
                    let (ra, rb) = (find(&mut parent, ia), find(&mut parent, ib));
                    parent[ra.max(rb)] = ra.min(rb);
                }
            }
        }
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for i in 0..faces.len() {
            let root = find(&mut parent, i);
            groups.entry(root).or_default().push(faces[i]);
        }
        let mut out: Vec<Vec<usize>> = groups.into_values().collect();
        out.sort_unstable_by_key(|g| g[0]);
        out
    }

    /// Inset de uma região. `None` = topologia não suportada (nada muda).
    /// `Some((d, ring))` = distância aplicada e faces do anel criadas.
    fn inset_region(&mut self, region: &[usize], distance: f32) -> Option<(f32, Vec<usize>)> {
        let normal = region
            .iter()
            .map(|&f| self.face_normal(f))
            .sum::<Vec3>()
            .normalize_or_zero();
        if normal.length_squared() < 0.5 {
            return None;
        }

        // Edges dirigidas da região; borda = sem a reversa dentro da região.
        let mut directed: HashMap<(u32, u32), (usize, usize)> = HashMap::new();
        let mut uses: HashMap<(u32, u32), usize> = HashMap::new();
        for &f in region {
            let verts = &self.faces[f].verts;
            let n = verts.len();
            for k in 0..n {
                let (a, b) = (verts[k], verts[(k + 1) % n]);
                directed.insert((a, b), (f, k));
                *uses.entry(edge_key(a, b)).or_default() += 1;
            }
        }
        if uses.values().any(|&n| n > 2) {
            return None;
        }
        let mut boundary: Vec<BoundaryEdge> = directed
            .iter()
            .filter(|((a, b), _)| !directed.contains_key(&(*b, *a)))
            .map(|(&(from, to), &(face, corner))| BoundaryEdge {
                from,
                to,
                face,
                corner,
            })
            .collect();
        boundary.sort_unstable_by_key(|e| (e.from, e.to));
        let interior: Vec<(u32, u32)> = {
            let mut v: Vec<_> = uses
                .iter()
                .filter(|(_, n)| **n == 2)
                .map(|(e, _)| *e)
                .collect();
            v.sort_unstable();
            v
        };

        // Cada vértice de borda tem exatamente uma edge de borda saindo.
        let mut outgoing: HashMap<u32, usize> = HashMap::new();
        let mut incoming: HashMap<u32, usize> = HashMap::new();
        for (i, e) in boundary.iter().enumerate() {
            if outgoing.insert(e.from, i).is_some() || incoming.insert(e.to, i).is_some() {
                return None; // gravata-borboleta
            }
        }
        let pos = |v: u32| self.verts[v as usize].vec();
        // Normal interna de cada edge de borda e mitra por vértice.
        let inward: Vec<Vec3> = boundary
            .iter()
            .map(|e| normal.cross(pos(e.to) - pos(e.from)).normalize_or_zero())
            .collect();
        let mut miter: HashMap<u32, Vec3> = HashMap::new();
        for e in &boundary {
            let (a, b) = (inward[*incoming.get(&e.from)?], inward[outgoing[&e.from]]);
            let denominator = 1.0 + a.dot(b);
            miter.insert(
                e.from,
                if denominator < 1.0e-3 {
                    Vec3::ZERO
                } else {
                    (a + b) / denominator
                },
            );
        }

        // Limite de colapso: comprimento L + d·taxa de cada edge de borda.
        let mut limit = f32::INFINITY;
        for e in &boundary {
            let edge = pos(e.to) - pos(e.from);
            let length = edge.length();
            if length < 1.0e-9 {
                continue;
            }
            let rate = (miter[&e.to] - miter[&e.from]).dot(edge / length);
            if rate < -1.0e-6 {
                limit = limit.min(length / -rate);
            }
        }
        let mut d = distance.min(limit * 0.98);
        // Nenhuma face da região pode inverter (cantos côncavos podem cruzar).
        let moved_normal = |face: &Face, d: f32| {
            let pts: Vec<Vec3> = face
                .verts
                .iter()
                .map(|&v| pos(v) + miter.get(&v).copied().unwrap_or(Vec3::ZERO) * d)
                .collect();
            let mut n = Vec3::ZERO;
            for k in 0..pts.len() {
                n += pts[k].cross(pts[(k + 1) % pts.len()]);
            }
            n
        };
        for _ in 0..8 {
            if region
                .iter()
                .all(|&f| moved_normal(&self.faces[f], d).dot(normal) > 1.0e-8)
            {
                break;
            }
            d *= 0.5;
        }
        if d <= 1.0e-6 {
            return Some((0.0, Vec::new()));
        }

        // Aplicação. UV dos cantos movidos: mapa planar da face de origem.
        let originals: HashMap<usize, Face> =
            region.iter().map(|&f| (f, self.faces[f].clone())).collect();
        let mut moved: HashMap<u32, u32> = HashMap::new();
        let mut sorted_boundary: Vec<u32> = miter.keys().copied().collect();
        sorted_boundary.sort_unstable();
        for v in sorted_boundary {
            let source = &self.verts[v as usize];
            let p = source.vec() + miter[&v] * d;
            let color = source.color;
            self.verts.push(Vertex {
                pos: p.to_array(),
                color,
                selected: true,
            });
            moved.insert(v, (self.verts.len() - 1) as u32);
        }
        let mut moved_uv: HashMap<(usize, usize), [f32; 2]> = HashMap::new();
        for (&f, original) in &originals {
            let points: Vec<Vec3> = original
                .verts
                .iter()
                .map(|&v| pos_of(&self.verts, v))
                .collect();
            let uv_at = planar_uv_map(&points, &original.uv, self.face_normal(f));
            for (k, &v) in original.verts.iter().enumerate() {
                if let Some(&nv) = moved.get(&v) {
                    let p = pos_of(&self.verts, nv);
                    moved_uv.insert((f, k), uv_at(p));
                }
            }
        }
        for (&f, original) in &originals {
            let face = &mut self.faces[f];
            for (k, &v) in original.verts.iter().enumerate() {
                if let Some(&nv) = moved.get(&v) {
                    face.verts[k] = nv;
                    face.uv[k] = moved_uv[&(f, k)];
                }
            }
        }
        let mut ring = Vec::with_capacity(boundary.len());
        for e in &boundary {
            let original = &originals[&e.face];
            let k = e.corner;
            let k2 = (k + 1) % original.verts.len();
            let mut quad = Face::with_uv(
                vec![e.from, e.to, moved[&e.to], moved[&e.from]],
                vec![
                    original.uv[k],
                    original.uv[k2],
                    moved_uv[&(e.face, k2)],
                    moved_uv[&(e.face, k)],
                ],
            );
            quad.material_slot = original.material_slot;
            self.push_face(quad);
            ring.push(self.faces.len() - 1);
        }
        // Seams/Sharp de edges internas acompanham os vértices movidos; as
        // edges de borda continuam existindo (entre o anel e o resto).
        let remap = |v: u32| moved.get(&v).copied().unwrap_or(v);
        for (a, b) in interior {
            let new = edge_key(remap(a), remap(b));
            if new == (a, b) {
                continue;
            }
            if self.uv_seams.remove(&(a, b)) {
                self.uv_seams.insert(new);
            }
            if self.sharp_edges.remove(&(a, b)) {
                self.sharp_edges.insert(new);
            }
        }
        Some((d, ring))
    }
}

fn pos_of(verts: &[Vertex], v: u32) -> Vec3 {
    verts[v as usize].vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Grade 2×1 de quads coplanares no plano XY (normal +Z).
    fn strip() -> Mesh {
        let verts = [
            [0.0, 0.0],
            [1.0, 0.0],
            [2.0, 0.0],
            [0.0, 1.0],
            [1.0, 1.0],
            [2.0, 1.0],
        ]
        .into_iter()
        .map(|[x, y]| Vertex::new(x, y, 0.0))
        .collect();
        let quad = |ids: [u32; 4]| {
            let uv = ids
                .iter()
                .map(|&i| [(i % 3) as f32 * 0.5, (i / 3) as f32])
                .collect();
            Face::with_uv(ids.to_vec(), uv)
        };
        Mesh {
            verts,
            faces: vec![quad([0, 1, 4, 3]), quad([1, 2, 5, 4])],
            ..Mesh::default()
        }
    }

    #[test]
    fn region_inset_rings_only_the_outer_boundary() {
        let mut m = strip();
        let report = m.inset_faces(&[0, 1], 0.1, InsetMode::Region);
        assert_eq!(report.skipped_regions, 0);
        // Borda da região tem 6 edges; a edge interna 1–4 não ganha anel.
        assert_eq!(report.ring_faces.len(), 6);
        assert_eq!(m.faces.len(), 8);
        assert!((report.applied - 0.1).abs() < 1e-5 && !report.limited);
        assert_eq!(report.inner_faces, vec![0, 1]);
        assert!(m.faces[0].selected && m.faces[1].selected && !m.faces[2].selected);
        // A face interna encolheu 0.1 a partir da borda externa.
        let min_x = m.faces[0]
            .verts
            .iter()
            .map(|&v| m.verts[v as usize].pos[0])
            .fold(f32::MAX, f32::min);
        assert!((min_x - 0.1).abs() < 1e-5);
    }

    #[test]
    fn individual_inset_rings_every_face() {
        let mut m = strip();
        let report = m.inset_faces(&[0, 1], 0.1, InsetMode::Individual);
        assert_eq!(report.ring_faces.len(), 8);
    }

    #[test]
    fn clamp_is_reported_not_silent() {
        let mut m = strip();
        let report = m.inset_faces(&[0, 1], 5.0, InsetMode::Region);
        assert!(report.limited);
        assert!(report.applied > 0.0 && report.applied < 0.5);
        assert!(m.validate_invariants().is_ok());
    }

    #[test]
    fn ring_inherits_material_and_source_uv() {
        let mut m = strip();
        m.faces[0].material_slot = Some(3);
        let source_uv = m.faces[0].uv[0];
        let report = m.inset_faces(&[0], 0.1, InsetMode::Region);
        assert!(
            report
                .ring_faces
                .iter()
                .all(|&f| m.faces[f].material_slot == Some(3))
        );
        // O anel que nasce na edge 0→1 começa com a UV original do canto 0.
        let bottom = report
            .ring_faces
            .iter()
            .copied()
            .find(|&f| m.faces[f].verts[0] == 0)
            .unwrap();
        assert_eq!(m.faces[bottom].uv[0], source_uv);
        // Canto interno com UV interpolada dentro do quad de origem.
        let inner_uv = m.faces[0].uv[0];
        assert!(inner_uv[0] > source_uv[0] && inner_uv[1] > source_uv[1]);
    }

    #[test]
    fn non_positive_or_non_finite_distance_does_nothing() {
        for d in [0.0, -1.0, f32::NAN] {
            let mut m = strip();
            let report = m.inset_faces(&[0, 1], d, InsetMode::Region);
            assert_eq!(m.faces.len(), 2);
            assert_eq!(report.applied, 0.0);
        }
    }

    #[test]
    fn interior_sharp_edges_follow_moved_vertices() {
        let mut m = strip();
        m.sharp_edges.insert(edge_key(1, 4));
        m.inset_faces(&[0, 1], 0.1, InsetMode::Region);
        assert!(!m.sharp_edges.contains(&edge_key(1, 4)));
        assert_eq!(m.sharp_edges.len(), 1);
    }
}
