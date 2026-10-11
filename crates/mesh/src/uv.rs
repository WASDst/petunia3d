//! Petunia3D mesh — UVs e normais.

use std::collections::HashSet;

use super::Mesh;
use glam::{Vec2, Vec3};

/// Ângulo no plano XZ → `u` em `0..1` (0 em +Z, crescendo em direção a +X).
fn angle_to_u(polar: Vec2) -> f32 {
    polar.x.atan2(polar.y) / std::f32::consts::TAU + 0.5
}

impl Mesh {
    // ---------------- UV ----------------

    /// Projeção planar por face (eixo dominante da normal), normalizada no bbox.
    pub fn project_planar(&mut self) {
        if self.verts.is_empty() || self.faces.is_empty() {
            return;
        }
        let mut bb_min = Vec3::splat(f32::MAX);
        let mut bb_max = Vec3::splat(f32::MIN);
        for v in &self.verts {
            bb_min = bb_min.min(v.vec());
            bb_max = bb_max.max(v.vec());
        }
        let size = (bb_max - bb_min).max(Vec3::splat(1e-6));
        for fi in 0..self.faces.len() {
            let n = self.face_normal(fi).abs();
            let idx: Vec<u32> = self.faces[fi].verts.clone();
            let mut uv = Vec::new();
            for (c_idx, &vi) in idx.iter().enumerate() {
                if self.uv_pinned.contains(&(fi, c_idx))
                    && let Some(&existing) = self.faces[fi].uv.get(c_idx)
                {
                    uv.push(existing);
                    continue;
                }
                let p = self.verts[vi as usize].vec();
                let q = (p - bb_min) / size;
                // eixo dominante vira o descartado
                let t = if n.y >= n.x && n.y >= n.z {
                    [q.x, q.z]
                } else if n.x >= n.z {
                    [q.z, q.y]
                } else {
                    [q.x, q.y]
                };
                uv.push(t);
            }
            self.faces[fi].uv = uv;
        }
    }

    /// Projeção cúbica canônica (Box Mapping): projeta cada face no plano
    /// alinhado (+X, -X, +Y, -Y, +Z, -Z) com escala normalizada no bbox.
    pub fn project_cube(&mut self) {
        if self.verts.is_empty() || self.faces.is_empty() {
            return;
        }
        let mut bb_min = Vec3::splat(f32::MAX);
        let mut bb_max = Vec3::splat(f32::MIN);
        for v in &self.verts {
            bb_min = bb_min.min(v.vec());
            bb_max = bb_max.max(v.vec());
        }
        let size = (bb_max - bb_min).max(Vec3::splat(1e-6));

        for fi in 0..self.faces.len() {
            let n = self.face_normal(fi);
            let nx = n.x.abs();
            let ny = n.y.abs();
            let nz = n.z.abs();

            let idx: Vec<u32> = self.faces[fi].verts.clone();
            let mut uv = Vec::with_capacity(idx.len());
            for (c_idx, &vi) in idx.iter().enumerate() {
                if self.uv_pinned.contains(&(fi, c_idx))
                    && let Some(&existing) = self.faces[fi].uv.get(c_idx)
                {
                    uv.push(existing);
                    continue;
                }
                let p = self.verts[vi as usize].vec();
                let q = (p - bb_min) / size;
                let t = if nx >= ny && nx >= nz {
                    if n.x > 0.0 {
                        [q.z, q.y]
                    } else {
                        [1.0 - q.z, q.y]
                    }
                } else if ny >= nx && ny >= nz {
                    if n.y > 0.0 {
                        [q.x, q.z]
                    } else {
                        [q.x, 1.0 - q.z]
                    }
                } else {
                    if n.z > 0.0 {
                        [q.x, q.y]
                    } else {
                        [1.0 - q.x, q.y]
                    }
                };
                uv.push(t);
            }
            self.faces[fi].uv = uv;
        }
    }

    /// Projeção cilíndrica em torno do eixo Y que passa pelo centro do bbox das
    /// faces alvo: `u` = ângulo, `v` = altura normalizada. `faces` vazio = malha
    /// inteira. Cantos fixados (pins) mantêm o valor.
    pub fn project_cylindrical(&mut self, faces: &HashSet<usize>) {
        let Some((center, half)) = self.target_bounds(faces) else {
            return;
        };
        let height = (half.y * 2.0).max(1e-6);
        self.project_angular(faces, |p| {
            let d = p - center;
            let polar = Vec2::new(d.x, d.z);
            let u = (polar.length_squared() > 1e-12).then(|| angle_to_u(polar));
            (u, (d.y + half.y) / height)
        });
    }

    /// Projeção esférica em torno do centro do bbox das faces alvo:
    /// `u` = longitude, `v` = latitude. `faces` vazio = malha inteira. Cantos
    /// fixados (pins) mantêm o valor.
    pub fn project_spherical(&mut self, faces: &HashSet<usize>) {
        let Some((center, _)) = self.target_bounds(faces) else {
            return;
        };
        self.project_angular(faces, |p| {
            let d = p - center;
            let r = d.length();
            if r < 1e-12 {
                return (None, 0.5);
            }
            let polar = Vec2::new(d.x, d.z);
            // No polo a longitude é indefinida: herda a média da face.
            let u = (polar.length() > r * 1e-4).then(|| angle_to_u(polar));
            (
                u,
                0.5 + (d.y / r).clamp(-1.0, 1.0).asin() / std::f32::consts::PI,
            )
        });
    }

    /// Centro e meia-extensão do bbox dos vértices das faces alvo.
    fn target_bounds(&self, faces: &HashSet<usize>) -> Option<(Vec3, Vec3)> {
        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for (fi, face) in self.faces.iter().enumerate() {
            if !faces.is_empty() && !faces.contains(&fi) {
                continue;
            }
            for &vi in &face.verts {
                let p = self.verts.get(vi as usize)?.vec();
                min = min.min(p);
                max = max.max(p);
            }
        }
        (min.x <= max.x && min.is_finite() && max.is_finite())
            .then(|| ((min + max) * 0.5, (max - min) * 0.5))
    }

    /// Núcleo das projeções angulares. `map` devolve `(u, v)` por posição, com
    /// `u = None` onde o ângulo é indefinido (eixo/polo). Por face: cantos sem
    /// `u` herdam a média circular dos demais, e uma face que cruza a costura do
    /// ângulo é desdobrada (`u` pode passar de 1 — diagnóstico, não erro).
    fn project_angular(
        &mut self,
        faces: &HashSet<usize>,
        map: impl Fn(Vec3) -> (Option<f32>, f32),
    ) {
        for fi in 0..self.faces.len() {
            if !faces.is_empty() && !faces.contains(&fi) {
                continue;
            }
            let mapped: Vec<(Option<f32>, f32)> = self.faces[fi]
                .verts
                .iter()
                .map(|&vi| map(self.verts[vi as usize].vec()))
                .collect();
            let known: Vec<f32> = mapped.iter().filter_map(|m| m.0).collect();
            // Desdobra a costura: se o salto entre cantos passa de meia volta,
            // os `u` pequenos ganham +1 para a face ficar contígua.
            let wraps = known
                .iter()
                .any(|&a| known.iter().any(|&b| (a - b).abs() > 0.5));
            let unwrap = |u: f32| if wraps && u < 0.5 { u + 1.0 } else { u };
            let fallback = if known.is_empty() {
                0.5
            } else {
                known.iter().map(|&u| unwrap(u)).sum::<f32>() / known.len() as f32
            };
            let uv: Vec<[f32; 2]> = mapped
                .iter()
                .enumerate()
                .map(|(c, &(u, v))| match self.faces[fi].uv.get(c) {
                    Some(&existing) if self.uv_pinned.contains(&(fi, c)) => existing,
                    _ => [u.map_or(fallback, unwrap), v],
                })
                .collect();
            self.faces[fi].uv = uv;
        }
    }

    /// Executa unwrap automático genérico usando o provider xatlas (P1-06 / P3D-064).
    pub fn unwrap_auto(&mut self) -> Result<usize, crate::uv_xatlas::UnwrapError> {
        crate::uv_xatlas::unwrap_fallback(self)
    }

    /// Normais por vértice (média ponderada por área) p/ export.
    pub fn compute_normals(&self) -> Vec<[f32; 3]> {
        let mut acc = vec![Vec3::ZERO; self.verts.len()];
        for (fi, f) in self.faces.iter().enumerate() {
            let n = self.face_normal(fi);
            // área aproximada
            let mut area = 0.0;
            if f.verts.len() >= 3 {
                let a = self.verts[f.verts[0] as usize].vec();
                for k in 1..f.verts.len() - 1 {
                    let b = self.verts[f.verts[k] as usize].vec();
                    let c = self.verts[f.verts[k + 1] as usize].vec();
                    area += (b - a).cross(c - a).length() * 0.5;
                }
            }
            for &vi in &f.verts {
                acc[vi as usize] += n * area.max(1e-9);
            }
        }
        acc.iter()
            .map(|v| v.normalize_or_zero().to_array())
            .collect()
    }

    /// Mapeia uma coordenada UV [0.0..1.0] para uma posição e normal no espaço 3D da superfície da malha.
    pub fn uv_to_world(&self, uv: [f32; 2]) -> Option<(Vec3, Vec3)> {
        let u = uv[0];
        let v = uv[1];
        for (fi, face) in self.faces.iter().enumerate() {
            if face.uv.len() < face.verts.len() || face.verts.len() < 3 {
                continue;
            }
            let fnorm = self.face_normal(fi);
            for corners in self.face_triangle_corners(fi) {
                let [ca, cb, cc] = corners;
                let uv0 = face.uv[ca];
                let uv1 = face.uv[cb];
                let uv2 = face.uv[cc];

                let denom =
                    (uv1[1] - uv2[1]) * (uv0[0] - uv2[0]) + (uv2[0] - uv1[0]) * (uv0[1] - uv2[1]);
                if denom.abs() < 1e-7 {
                    continue;
                }

                let w0 =
                    ((uv1[1] - uv2[1]) * (u - uv2[0]) + (uv2[0] - uv1[0]) * (v - uv2[1])) / denom;
                let w1 =
                    ((uv2[1] - uv0[1]) * (u - uv2[0]) + (uv0[0] - uv2[0]) * (v - uv2[1])) / denom;
                let w2 = 1.0 - w0 - w1;

                const EPS: f32 = 1e-4;
                if w0 >= -EPS && w1 >= -EPS && w2 >= -EPS {
                    let va = self.verts[face.verts[ca] as usize].vec();
                    let vb = self.verts[face.verts[cb] as usize].vec();
                    let vc = self.verts[face.verts[cc] as usize].vec();
                    let pos = va * w0 + vb * w1 + vc * w2;
                    return Some((pos, fnorm));
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cylindrical_projection_keeps_faces_contiguous_across_the_seam() {
        let mut mesh = Mesh::cylinder(16, 1.0, 2.0);
        let sides_before = mesh.faces.len();
        mesh.project_cylindrical(&HashSet::new());
        assert_eq!(mesh.faces.len(), sides_before);
        for (fi, face) in mesh.faces.iter().enumerate() {
            assert_eq!(face.uv.len(), face.verts.len());
            assert!(face.uv.iter().flatten().all(|x| x.is_finite()));
            if mesh.face_normal(fi).y.abs() > 0.5 {
                continue; // tampa: cobre a volta inteira por definição
            }
            let us: Vec<f32> = face.uv.iter().map(|uv| uv[0]).collect();
            let spread = us.iter().cloned().fold(f32::MIN, f32::max)
                - us.iter().cloned().fold(f32::MAX, f32::min);
            assert!(spread <= 0.5, "face crosses the seam unfolded: {us:?}");
        }
    }

    #[test]
    fn spherical_projection_is_finite_at_the_poles_and_respects_pins() {
        let mut mesh = Mesh::sphere_low(12, 8, 1.0);
        mesh.faces[0].uv[0] = [0.42, 0.42];
        mesh.pin_uv(0, 0);
        mesh.project_spherical(&HashSet::new());
        assert_eq!(mesh.faces[0].uv[0], [0.42, 0.42]);
        for face in &mesh.faces {
            assert!(face.uv.iter().flatten().all(|x| x.is_finite()));
            assert!(face.uv.iter().all(|uv| (0.0..=1.0).contains(&uv[1])));
        }
    }

    #[test]
    fn angular_projection_only_touches_target_faces() {
        let mut mesh = Mesh::cube(2.0);
        mesh.project_cube();
        let untouched = mesh.faces[1].uv.clone();
        mesh.project_cylindrical(&HashSet::from([0]));
        assert_eq!(mesh.faces[1].uv, untouched);
    }

    #[test]
    fn test_uv_to_world_on_plane() {
        let mesh = Mesh::plane(2.0);
        // Plane vertices: (-1, -1, 0), (1, -1, 0), (1, 1, 0), (-1, 1, 0)
        // UVs: (0, 0), (1, 0), (1, 1), (0, 1)
        let hit = mesh.uv_to_world([0.5, 0.5]);
        assert!(hit.is_some());
        let (pos, norm) = hit.unwrap();
        assert!((pos.x).abs() < 1e-3);
        assert!((pos.y).abs() < 1e-3);
        assert!((pos.z).abs() < 1e-3);
        assert!(norm.y.abs() > 0.9);
    }
}
