//! Stitch e Split UV (V01; website/docs/07-uv/architecture-decision.md, "Stitch",
//! "Split UV").
//!
//! Stitch tem um lado **parado** e um lado **móvel**: a ilha móvel recebe uma
//! transformação rígida com escala uniforme (Procrustes 2D) que alinha a borda
//! compartilhada à ilha parada; depois os cantos da borda são fundidos e a seam
//! dessas edges é limpa. Nunca faz média dos dois lados. Casos ambíguos são
//! recusados por edge/vértice em vez de adivinhados.
//!
//! Split UV marca seam na fronteira de um conjunto de faces: a descontinuidade
//! vive nos FaceCorners, sem duplicar vértice 3D.

use std::collections::{HashMap, HashSet};

use glam::Vec2;

use super::{Mesh, edge_key};

/// Distância máxima para dois cantos serem o mesmo vértice UV.
const SAME_UV: f32 = 1.0e-5;

/// Resultado de um Stitch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UvStitchReport {
    /// Edges costuradas (seam limpa, cantos fundidos).
    pub stitched_edges: usize,
    /// Ilhas que se moveram até a ilha parada.
    pub moved_islands: usize,
    /// Ilhas vizinhas não movidas por terem pin (recusa segura).
    pub pinned_islands: usize,
}

impl Mesh {
    /// Stitch de compatibilidade: dentro de `faces_filter` (vazio = todas), a
    /// maior ilha (empate: menor índice de face) fica parada e as vizinhas se
    /// movem até ela. Retorna o número de edges costuradas.
    pub fn stitch_uv(&mut self, faces_filter: &HashSet<usize>) -> usize {
        let islands = self.uv_islands();
        let stationary = islands
            .iter()
            .filter(|i| faces_filter.is_empty() || i.faces.iter().any(|f| faces_filter.contains(f)))
            .max_by(|a, b| {
                a.faces
                    .len()
                    .cmp(&b.faces.len())
                    .then_with(|| b.faces.iter().min().cmp(&a.faces.iter().min()))
            })
            .and_then(|island| island.faces.iter().min().copied());
        match stationary {
            Some(face) => self.stitch_uv_to(face, faces_filter).stitched_edges,
            None => 0,
        }
    }

    /// Costura à ilha que contém `stationary_face` as ilhas vizinhas (por edges
    /// com UV dividida) que tocam `faces_filter` (vazio = todas).
    pub fn stitch_uv_to(
        &mut self,
        stationary_face: usize,
        faces_filter: &HashSet<usize>,
    ) -> UvStitchReport {
        let mut report = UvStitchReport::default();
        let islands = self.uv_islands();
        let mut island_of = vec![usize::MAX; self.faces.len()];
        for (i, island) in islands.iter().enumerate() {
            for &f in &island.faces {
                island_of[f] = i;
            }
        }
        let Some(&stationary) = island_of.get(stationary_face) else {
            return report;
        };
        if stationary == usize::MAX {
            return report;
        }
        let in_filter = |f: usize| faces_filter.is_empty() || faces_filter.contains(&f);

        // Edges entre a ilha parada e cada ilha móvel: (face parada, face móvel, a, b).
        let mut borders: HashMap<usize, Vec<(usize, usize, u32, u32)>> = HashMap::new();
        let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (fi, face) in self.faces.iter().enumerate() {
            let n = face.verts.len();
            for k in 0..n {
                edge_faces
                    .entry(edge_key(face.verts[k], face.verts[(k + 1) % n]))
                    .or_default()
                    .push(fi);
            }
        }
        let mut edges: Vec<_> = edge_faces.into_iter().collect();
        edges.sort_unstable_by_key(|(edge, _)| *edge);
        for ((a, b), faces) in edges {
            let [f, g] = faces[..] else { continue };
            let (fixed, moving) = match (island_of[f] == stationary, island_of[g] == stationary) {
                (true, false) => (f, g),
                (false, true) => (g, f),
                _ => continue,
            };
            if in_filter(moving) {
                borders
                    .entry(island_of[moving])
                    .or_default()
                    .push((fixed, moving, a, b));
            }
        }

        let mut moving_islands: Vec<usize> = borders.keys().copied().collect();
        moving_islands.sort_unstable();
        for island in moving_islands {
            let faces = &islands[island].faces;
            if faces
                .iter()
                .any(|&f| (0..self.faces[f].uv.len()).any(|c| self.uv_pinned.contains(&(f, c))))
            {
                report.pinned_islands += 1;
                continue;
            }
            let edges = &borders[&island];
            // Alvo de cada vértice da borda na ilha parada; vértice com dois
            // alvos diferentes é ambíguo e não é fundido.
            let mut target: HashMap<u32, Option<Vec2>> = HashMap::new();
            let mut source: HashMap<u32, Vec2> = HashMap::new();
            for &(fixed, moving, a, b) in edges {
                for v in [a, b] {
                    let (Some(t), Some(s)) =
                        (self.corner_uv_at(fixed, v), self.corner_uv_at(moving, v))
                    else {
                        continue;
                    };
                    target
                        .entry(v)
                        .and_modify(|e| {
                            if e.is_some_and(|old| old.distance(t) > SAME_UV) {
                                *e = None;
                            }
                        })
                        .or_insert(Some(t));
                    source.entry(v).or_insert(s);
                }
            }
            let mut border_vertices: Vec<u32> = source.keys().copied().collect();
            border_vertices.sort_unstable();
            let pairs: Vec<(Vec2, Vec2)> = border_vertices
                .iter()
                .filter_map(|v| target.get(v).copied().flatten().map(|t| (source[v], t)))
                .collect();
            if pairs.is_empty() {
                continue;
            }
            let mirror =
                self.uv_orientation(faces) * self.uv_orientation(&islands[stationary].faces) < 0.0;
            let transform = similarity(&pairs, mirror);

            // Move a ilha inteira e funde os cantos da borda.
            for &f in faces {
                let verts = self.faces[f].verts.clone();
                for (c, v) in verts.into_iter().enumerate() {
                    let before = Vec2::from_array(self.faces[f].uv[c]);
                    let snapped = match (source.get(&v), target.get(&v).copied().flatten()) {
                        (Some(s), Some(t)) if s.distance(before) <= SAME_UV => t,
                        _ => transform.apply(before),
                    };
                    self.faces[f].uv[c] = snapped.to_array();
                }
            }
            for &(_, _, a, b) in edges {
                if target.get(&a).copied().flatten().is_some()
                    && target.get(&b).copied().flatten().is_some()
                {
                    self.uv_seams.remove(&edge_key(a, b));
                    report.stitched_edges += 1;
                }
            }
            report.moved_islands += 1;
        }
        report
    }

    /// Split UV: marca seam em cada edge entre uma face de `faces` e uma face
    /// fora do conjunto. Não toca em vértices 3D nem em UVs. Retorna as seams
    /// novas.
    pub fn split_uv(&mut self, faces: &HashSet<usize>) -> usize {
        let mut edge_faces: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
        for (fi, face) in self.faces.iter().enumerate() {
            let n = face.verts.len();
            for k in 0..n {
                edge_faces
                    .entry(edge_key(face.verts[k], face.verts[(k + 1) % n]))
                    .or_default()
                    .push(fi);
            }
        }
        let mut added = 0;
        for (edge, adjacent) in edge_faces {
            let inside = adjacent.iter().filter(|f| faces.contains(f)).count();
            if inside > 0 && inside < adjacent.len() && self.uv_seams.insert(edge) {
                added += 1;
            }
        }
        added
    }

    /// UV do canto de `face` que usa o vértice `v`.
    fn corner_uv_at(&self, face: usize, v: u32) -> Option<Vec2> {
        let f = self.faces.get(face)?;
        let c = f.verts.iter().position(|&x| x == v)?;
        f.uv.get(c).map(|uv| Vec2::from_array(*uv))
    }

    /// Sinal da área UV assinada das faces (positivo = mesma orientação do 3D).
    fn uv_orientation(&self, faces: &[usize]) -> f32 {
        let mut area = 0.0;
        for &fi in faces {
            let face = &self.faces[fi];
            if face.uv.len() != face.verts.len() {
                continue;
            }
            for [a, b, c] in self.face_triangle_corners(fi) {
                let [ua, ub, uc] = [a, b, c].map(|k| Vec2::from_array(face.uv[k]));
                area += (ub - ua).perp_dot(uc - ua);
            }
        }
        area
    }
}

/// `p ↦ rotation_scale ⊗ (mirror? p) + translation`, com ⊗ = produto complexo.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Similarity {
    mirror: bool,
    rotation_scale: Vec2,
    translation: Vec2,
}

impl Similarity {
    fn apply(&self, p: Vec2) -> Vec2 {
        let p = if self.mirror { Vec2::new(-p.x, p.y) } else { p };
        self.rotation_scale.rotate(p) + self.translation
    }
}

/// Melhor similaridade (mínimos quadrados) levando cada `source` ao seu `target`.
/// Com um único par, só translada.
fn similarity(pairs: &[(Vec2, Vec2)], mirror: bool) -> Similarity {
    let flip = |p: Vec2| if mirror { Vec2::new(-p.x, p.y) } else { p };
    let n = pairs.len() as f32;
    let source_center = pairs.iter().map(|(s, _)| flip(*s)).sum::<Vec2>() / n;
    let target_center = pairs.iter().map(|(_, t)| *t).sum::<Vec2>() / n;
    let (mut dot, mut cross, mut norm) = (0.0f32, 0.0f32, 0.0f32);
    for (s, t) in pairs {
        let p = flip(*s) - source_center;
        let q = *t - target_center;
        dot += p.dot(q);
        cross += p.perp_dot(q);
        norm += p.length_squared();
    }
    let rotation_scale = if norm > 1.0e-12 {
        Vec2::new(dot, cross) / norm
    } else {
        Vec2::X
    };
    Similarity {
        mirror,
        rotation_scale,
        translation: target_center - rotation_scale.rotate(source_center),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Face, Vertex};

    /// Dois quads vizinhos (edge 1–4) com UVs em ilhas separadas: a direita
    /// foi deslocada, girada 90° e reduzida à metade.
    fn split_pair() -> Mesh {
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
        let left = Face::with_uv(
            vec![0, 1, 4, 3],
            vec![[0.0, 0.0], [0.2, 0.0], [0.2, 0.2], [0.0, 0.2]],
        );
        // Imagem de [0.2,0.4]x[0,0.2] girada 90°, escalada por 0.5 e transladada.
        let right = Face::with_uv(
            vec![1, 2, 5, 4],
            vec![[0.8, 0.5], [0.8, 0.6], [0.7, 0.6], [0.7, 0.5]],
        );
        let mut m = Mesh {
            verts,
            faces: vec![left, right],
            ..Mesh::default()
        };
        m.uv_seams.insert(edge_key(1, 4));
        m
    }

    fn close(a: [f32; 2], b: [f32; 2]) -> bool {
        (a[0] - b[0]).abs() < 1e-4 && (a[1] - b[1]).abs() < 1e-4
    }

    #[test]
    fn stitch_moves_only_the_moving_island_and_clears_the_seam() {
        let mut m = split_pair();
        let fixed = m.faces[0].uv.clone();
        let report = m.stitch_uv_to(0, &HashSet::new());
        assert_eq!((report.stitched_edges, report.moved_islands), (1, 1));
        assert_eq!(m.faces[0].uv, fixed);
        assert!(!m.uv_seams.contains(&edge_key(1, 4)));
        // A ilha móvel volta a ser a continuação exata da parada.
        let expected = [[0.2, 0.0], [0.4, 0.0], [0.4, 0.2], [0.2, 0.2]];
        for (got, want) in m.faces[1].uv.iter().zip(expected) {
            assert!(close(*got, want), "{got:?} != {want:?}");
        }
        assert_eq!(m.uv_islands().len(), 1);
    }

    #[test]
    fn compatibility_stitch_keeps_the_larger_island_still() {
        let mut m = split_pair();
        let fixed = m.faces[0].uv.clone();
        assert_eq!(m.stitch_uv(&HashSet::new()), 1);
        assert_eq!(m.faces[0].uv, fixed);
    }

    #[test]
    fn pinned_moving_islands_are_refused() {
        let mut m = split_pair();
        m.pin_uv(1, 0);
        let before = m.faces[1].uv.clone();
        let report = m.stitch_uv_to(0, &HashSet::new());
        assert_eq!((report.pinned_islands, report.stitched_edges), (1, 0));
        assert_eq!(m.faces[1].uv, before);
        assert!(m.uv_seams.contains(&edge_key(1, 4)));
    }

    #[test]
    fn split_marks_only_the_selection_boundary() {
        let mut m = split_pair();
        m.uv_seams.clear();
        assert_eq!(m.split_uv(&HashSet::from([1])), 1);
        assert!(m.uv_seams.contains(&edge_key(1, 4)));
        assert_eq!(m.verts.len(), 6);
        assert_eq!(m.uv_islands().len(), 2);
        // Repetir não cria nada novo.
        assert_eq!(m.split_uv(&HashSet::from([1])), 0);
    }
}
