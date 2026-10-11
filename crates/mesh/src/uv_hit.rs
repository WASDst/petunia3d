//! Primitivas de hit e transformação no espaço UV (Geometry, V01;
//! website/docs/07-uv/architecture-decision.md, "UV Selection", "UV Edge",
//! "Pins", "UV Transform").
//!
//! Consultas puras sobre FaceCorner UV: a UI converte pixels em UV e a
//! Application decide o que fazer com o resultado.

use std::collections::HashSet;

use glam::Vec2;

use super::{FaceCornerIndex, Mesh};

/// Edge no espaço UV: dois cantos consecutivos de uma face. Distinta de
/// `EdgeKey`: uma edge 3D com seam gera duas UV edges.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UvEdge {
    pub face: usize,
    pub corner_a: usize,
    pub corner_b: usize,
}

/// Transformação direta de UV (Move/Rotate/Scale) em torno de um pivô.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum UvTransform {
    Translate(Vec2),
    /// Radianos, anti-horário, em torno do centroide dos cantos.
    Rotate(f32),
    /// Fator por eixo em torno do centroide dos cantos.
    Scale(Vec2),
}

impl UvTransform {
    pub fn is_finite(&self) -> bool {
        match self {
            Self::Translate(d) => d.is_finite(),
            Self::Rotate(a) => a.is_finite(),
            Self::Scale(s) => s.is_finite(),
        }
    }
}

impl Mesh {
    /// Face cujo polígono UV contém `p` (a UV pode ser côncava mesmo quando a
    /// face 3D não é). Em sobreposição, vence o menor índice.
    pub fn uv_face_at(&self, p: Vec2) -> Option<usize> {
        self.faces.iter().enumerate().find_map(|(fi, face)| {
            if face.uv.len() < 3 {
                return None;
            }
            let triangles = crate::triangulate::ear_clip(&face.uv).ok()?;
            triangles
                .into_iter()
                .any(|[a, b, c]| {
                    point_in_triangle(
                        p,
                        Vec2::from_array(face.uv[a]),
                        Vec2::from_array(face.uv[b]),
                        Vec2::from_array(face.uv[c]),
                    )
                })
                .then_some(fi)
        })
    }

    /// Canto UV mais próximo de `p` a até `radius` (unidades UV). Empate: o menor
    /// `(face, canto)`.
    pub fn uv_corner_near(&self, p: Vec2, radius: f32) -> Option<FaceCornerIndex> {
        let mut best: Option<(f32, FaceCornerIndex)> = None;
        for (fi, face) in self.faces.iter().enumerate() {
            for (c, uv) in face.uv.iter().enumerate() {
                let d = Vec2::from_array(*uv).distance(p);
                if d <= radius && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, FaceCornerIndex::new(fi, c)));
                }
            }
        }
        best.map(|(_, corner)| corner)
    }

    /// UV edge mais próxima de `p` a até `radius`. Empate: a menor edge.
    pub fn uv_edge_near(&self, p: Vec2, radius: f32) -> Option<UvEdge> {
        let mut best: Option<(f32, UvEdge)> = None;
        for (fi, face) in self.faces.iter().enumerate() {
            let n = face.uv.len();
            if n < 2 || n != face.verts.len() {
                continue;
            }
            for a in 0..n {
                let b = (a + 1) % n;
                let d = distance_to_segment(
                    p,
                    Vec2::from_array(face.uv[a]),
                    Vec2::from_array(face.uv[b]),
                );
                let edge = UvEdge {
                    face: fi,
                    corner_a: a,
                    corner_b: b,
                };
                if d <= radius && best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, edge));
                }
            }
        }
        best.map(|(_, edge)| edge)
    }

    /// Cantos que formam o mesmo vértice UV de `corner`: mesmo vértice 3D e
    /// mesma UV (UV vertex não é um conceito separado do FaceCorner).
    pub fn uv_coincident_corners(&self, corner: FaceCornerIndex) -> Vec<FaceCornerIndex> {
        const SAME_UV: f32 = 1.0e-5;
        let Some(face) = self.faces.get(corner.face) else {
            return Vec::new();
        };
        let (Some(&vertex), Some(&uv)) =
            (face.verts.get(corner.corner), face.uv.get(corner.corner))
        else {
            return Vec::new();
        };
        let uv = Vec2::from_array(uv);
        let mut out = Vec::new();
        for (fi, f) in self.faces.iter().enumerate() {
            for (c, (&v, other)) in f.verts.iter().zip(&f.uv).enumerate() {
                if v == vertex && Vec2::from_array(*other).distance(uv) <= SAME_UV {
                    out.push(FaceCornerIndex::new(fi, c));
                }
            }
        }
        out
    }

    /// Aplica uma transformação direta aos `corners` (o pivô de Rotate/Scale é
    /// o centroide deles). Pins **não** bloqueiam: um gesto explícito do usuário
    /// move o canto fixado, que continua fixado na nova posição. Retorna os
    /// cantos alterados.
    pub fn transform_uv_corners(
        &mut self,
        corners: &HashSet<FaceCornerIndex>,
        transform: UvTransform,
    ) -> usize {
        if !transform.is_finite() {
            return 0;
        }
        let valid: Vec<FaceCornerIndex> = {
            let mut v: Vec<_> = corners
                .iter()
                .copied()
                .filter(|c| {
                    self.faces
                        .get(c.face)
                        .is_some_and(|f| c.corner < f.uv.len())
                })
                .collect();
            v.sort_unstable();
            v
        };
        if valid.is_empty() {
            return 0;
        }
        let pivot = valid
            .iter()
            .map(|c| Vec2::from_array(self.faces[c.face].uv[c.corner]))
            .sum::<Vec2>()
            / valid.len() as f32;
        let apply = |p: Vec2| match transform {
            UvTransform::Translate(d) => p + d,
            UvTransform::Rotate(angle) => pivot + Vec2::from_angle(angle).rotate(p - pivot),
            UvTransform::Scale(s) => pivot + (p - pivot) * s,
        };
        for c in &valid {
            let uv = &mut self.faces[c.face].uv[c.corner];
            *uv = apply(Vec2::from_array(*uv)).to_array();
        }
        valid.len()
    }
}

fn point_in_triangle(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let (v0, v1, v2) = (c - a, b - a, p - a);
    let (d00, d01, d11, d20, d21) = (v0.dot(v0), v0.dot(v1), v1.dot(v1), v2.dot(v0), v2.dot(v1));
    let den = d00 * d11 - d01 * d01;
    if den.abs() < 1e-12 {
        return false;
    }
    let v = (d11 * d20 - d01 * d21) / den;
    let w = (d00 * d21 - d01 * d20) / den;
    v >= -1e-6 && w >= -1e-6 && v + w <= 1.0 + 1e-6
}

fn distance_to_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-12 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Face, Vertex};

    fn concave_pentagon() -> Mesh {
        Mesh {
            verts: (0..5).map(|x| Vertex::new(x as f32, 0.0, 0.0)).collect(),
            faces: vec![Face::with_uv(
                vec![0, 1, 2, 3, 4],
                vec![
                    [0.0, 0.0],
                    [1.0, 0.0],
                    [1.0, 1.0],
                    [0.5, 1.0 / 3.0],
                    [0.0, 1.0],
                ],
            )],
            ..Mesh::default()
        }
    }

    #[test]
    fn face_hit_respects_a_concave_island() {
        let m = concave_pentagon();
        assert_eq!(m.uv_face_at(Vec2::new(0.25, 0.1)), Some(0));
        assert_eq!(m.uv_face_at(Vec2::new(0.5, 0.5)), None);
    }

    #[test]
    fn corner_and_edge_hits_use_the_radius() {
        let m = concave_pentagon();
        assert_eq!(
            m.uv_corner_near(Vec2::new(0.98, 0.02), 0.05),
            Some(FaceCornerIndex::new(0, 1))
        );
        assert_eq!(m.uv_corner_near(Vec2::new(0.5, 0.0), 0.05), None);
        let edge = m.uv_edge_near(Vec2::new(0.5, 0.01), 0.05).unwrap();
        assert_eq!((edge.corner_a, edge.corner_b), (0, 1));
    }

    #[test]
    fn direct_transform_moves_pinned_corners() {
        let mut m = Mesh::plane(2.0);
        m.pin_uv(0, 0);
        let all: HashSet<_> = (0..4).map(|c| FaceCornerIndex::new(0, c)).collect();
        let before = m.faces[0].uv[0];
        assert_eq!(
            m.transform_uv_corners(&all, UvTransform::Translate(Vec2::new(0.5, 0.0))),
            4
        );
        assert!((m.faces[0].uv[0][0] - (before[0] + 0.5)).abs() < 1e-6);
        assert!(m.is_uv_pinned(0, 0));
    }

    #[test]
    fn rotate_and_scale_use_the_selection_centroid() {
        let mut m = Mesh::plane(2.0);
        let all: HashSet<_> = (0..4).map(|c| FaceCornerIndex::new(0, c)).collect();
        let centroid = |m: &Mesh| {
            m.faces[0]
                .uv
                .iter()
                .map(|uv| Vec2::from_array(*uv))
                .sum::<Vec2>()
                / 4.0
        };
        let before = centroid(&m);
        m.transform_uv_corners(&all, UvTransform::Rotate(std::f32::consts::FRAC_PI_2));
        m.transform_uv_corners(&all, UvTransform::Scale(Vec2::splat(2.0)));
        assert!(centroid(&m).distance(before) < 1e-5);
        assert_eq!(
            m.transform_uv_corners(&all, UvTransform::Scale(Vec2::splat(f32::NAN))),
            0
        );
    }
}
