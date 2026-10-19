//! Leitura da superfície no PAINT: seleção de faces pelo UV no canvas 2D
//! (P3D-132, S3) e pegada elíptica do pincel 3D em superfície inclinada (B1).

use petunia_core::{SelectionDomain, Workspace};

use crate::projection::{active_world_matrix, pick_face_hit, project_world_point};
use crate::{PetuniaViewport, SlintUiBridge};

/// Abaixo deste lado (fração da textura), a caixa é um clique.
const CLICK_EXTENT: f32 = 0.004;
/// Menor achatamento do cursor: superfícies quase de perfil continuam com um
/// anel legível.
const MIN_SQUASH: f32 = 0.2;

/// O ponto `p` está no triângulo `a b c` (inclui as bordas).
fn in_triangle(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let cross = |o: [f32; 2], u: [f32; 2], v: [f32; 2]| {
        (u[0] - o[0]) * (v[1] - o[1]) - (u[1] - o[1]) * (v[0] - o[0])
    };
    let (d1, d2, d3) = (cross(p, a, b), cross(p, b, c), cross(p, c, a));
    let negative = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let positive = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(negative && positive)
}

/// Pesos baricêntricos de `p` no triângulo `abc` (2D). `None` se degenerado.
fn barycentric_2d(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> Option<[f32; 3]> {
    let denom = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if !denom.is_finite() || denom.abs() < 1.0e-12 {
        return None;
    }
    let w0 = ((b[1] - c[1]) * (p[0] - c[0]) + (c[0] - b[0]) * (p[1] - c[1])) / denom;
    let w1 = ((c[1] - a[1]) * (p[0] - c[0]) + (a[0] - c[0]) * (p[1] - c[1])) / denom;
    Some([w0, w1, 1.0 - w0 - w1])
}

/// Pesos baricêntricos de `p` (sobre o plano) no triângulo `abc` (3D).
fn barycentric_3d(p: glam::Vec3, a: glam::Vec3, b: glam::Vec3, c: glam::Vec3) -> Option<[f32; 3]> {
    let (v0, v1, v2) = (b - a, c - a, p - a);
    let (d00, d01, d11) = (v0.dot(v0), v0.dot(v1), v1.dot(v1));
    let (d20, d21) = (v2.dot(v0), v2.dot(v1));
    let denom = d00 * d11 - d01 * d01;
    if !denom.is_finite() || denom.abs() < 1.0e-12 {
        return None;
    }
    let w1 = (d11 * d20 - d01 * d21) / denom;
    let w2 = (d00 * d21 - d01 * d20) / denom;
    Some([1.0 - w1 - w2, w1, w2])
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Ponto da textura (fração, y para baixo, como o canvas 2D) sob `(x, y)` px
    /// da viewport na PAINT — o cursor do pincel espelhado no 2D (ADR-C16, B2).
    pub fn brush_texture_point_at(&self, x: f32, y: f32) -> Option<[f32; 2]> {
        if self.state.workspace != Workspace::Paint {
            return None;
        }
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 || !x.is_finite() || !y.is_finite() {
            return None;
        }
        let camera = &self.state.session.camera;
        let (origin, direction) = camera.ray(x / width * 2.0 - 1.0, 1.0 - y / height * 2.0);
        let (face_index, hit) = pick_face_hit(&self.state, origin, direction)?;
        let world = active_world_matrix(&self.state);
        if !world.determinant().is_finite() || world.determinant().abs() < 1.0e-12 {
            return None;
        }
        let local = world.inverse().transform_point3(hit);
        let mesh = self.state.project.active_mesh()?;
        let face = mesh.faces.get(face_index)?;
        let position = |corner: usize| {
            face.verts
                .get(corner)
                .and_then(|&vertex| mesh.verts.get(vertex as usize))
                .map(|vertex| vertex.vec())
        };
        for [a, b, c] in mesh.face_triangle_corners(face_index) {
            let (Some(pa), Some(pb), Some(pc)) = (position(a), position(b), position(c)) else {
                continue;
            };
            let (Some(ua), Some(ub), Some(uc)) = (face.uv.get(a), face.uv.get(b), face.uv.get(c))
            else {
                continue;
            };
            let Some(w) = barycentric_3d(local, pa, pb, pc) else {
                continue;
            };
            if w.iter().any(|weight| *weight < -1.0e-3) {
                continue;
            }
            let u = ua[0] * w[0] + ub[0] * w[1] + uc[0] * w[2];
            let v = ua[1] * w[0] + ub[1] * w[1] + uc[1] * w[2];
            if u.is_finite() && v.is_finite() {
                return Some([u, 1.0 - v]);
            }
        }
        None
    }

    /// Posição (px da viewport) do ponto da superfície com a fração de textura
    /// `(tx, ty)` — o cursor do canvas 2D espelhado na vista 3D (B2). `None` fora
    /// de qualquer ilha UV ou atrás da câmera.
    pub fn texture_point_to_viewport(&self, tx: f32, ty: f32) -> Option<[f32; 2]> {
        if self.state.workspace != Workspace::Paint || !tx.is_finite() || !ty.is_finite() {
            return None;
        }
        let mesh = self.state.project.active_mesh()?;
        let point = [tx, 1.0 - ty];
        let world = active_world_matrix(&self.state);
        for (face_index, face) in mesh.faces.iter().enumerate() {
            for [a, b, c] in mesh.face_triangle_corners(face_index) {
                let (Some(&ua), Some(&ub), Some(&uc)) =
                    (face.uv.get(a), face.uv.get(b), face.uv.get(c))
                else {
                    continue;
                };
                if !in_triangle(point, ua, ub, uc) {
                    continue;
                }
                let Some(w) = barycentric_2d(point, ua, ub, uc) else {
                    continue;
                };
                let corner = |corner: usize| {
                    face.verts
                        .get(corner)
                        .and_then(|&vertex| mesh.verts.get(vertex as usize))
                        .map(|vertex| vertex.vec())
                };
                let (Some(pa), Some(pb), Some(pc)) = (corner(a), corner(b), corner(c)) else {
                    continue;
                };
                let local = pa * w[0] + pb * w[1] + pc * w[2];
                return project_world_point(
                    &self.state.session.camera,
                    self.viewport_size,
                    world.transform_point3(local),
                );
            }
        }
        None
    }

    /// Seleção de faces no canvas 2D com a ferramenta Select. `(x0, y0)` e
    /// `(x1, y1)` são frações da textura (origem no canto superior esquerdo).
    /// Um clique escolhe a face sob o ponto (Alt: a ilha UV inteira; Shift
    /// alterna); uma caixa escolhe as faces com o centro UV dentro (Shift
    /// soma, Ctrl subtrai). Em Objeto, o domínio passa visivelmente a Face.
    #[allow(clippy::too_many_arguments)]
    pub fn paint_2d_select(
        &mut self,
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        extend: bool,
        subtract: bool,
        island: bool,
    ) -> bool {
        if self.state.workspace != Workspace::Paint
            || ![x0, y0, x1, y1].iter().all(|v| v.is_finite())
        {
            return false;
        }
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        let click = (x1 - x0).abs() < CLICK_EXTENT && (y1 - y0).abs() < CLICK_EXTENT;
        let picked: Vec<usize> = if click {
            // Textura com y para baixo; UV com v para cima.
            let point = [x1, 1.0 - y1];
            let face = (0..mesh.faces.len()).find(|&fi| {
                let uv = &mesh.faces[fi].uv;
                mesh.face_triangle_corners(fi).into_iter().any(|[a, b, c]| {
                    match (uv.get(a), uv.get(b), uv.get(c)) {
                        (Some(&a), Some(&b), Some(&c)) => in_triangle(point, a, b, c),
                        _ => false,
                    }
                })
            });
            match (face, island) {
                (Some(face), true) => mesh
                    .uv_islands()
                    .into_iter()
                    .find(|item| item.faces.contains(&face))
                    .map_or_else(|| vec![face], |item| item.faces),
                (Some(face), false) => vec![face],
                (None, _) => Vec::new(),
            }
        } else {
            let (u0, u1) = (x0.min(x1), x0.max(x1));
            let (v0, v1) = (1.0 - y0.max(y1), 1.0 - y0.min(y1));
            mesh.faces
                .iter()
                .enumerate()
                .filter(|(_, face)| !face.uv.is_empty())
                .filter(|(_, face)| {
                    let n = face.uv.len() as f32;
                    let u = face.uv.iter().map(|uv| uv[0]).sum::<f32>() / n;
                    let v = face.uv.iter().map(|uv| uv[1]).sum::<f32>() / n;
                    (u0..=u1).contains(&u) && (v0..=v1).contains(&v)
                })
                .map(|(index, _)| index)
                .collect()
        };
        if self.state.selection_domain() == SelectionDomain::Object {
            self.state.set_selection_domain(SelectionDomain::Face);
        }
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        let first = picked.first().copied();
        let op = if click {
            let was_selected = first.is_some_and(|face| mesh.faces[face].selected);
            match (extend, was_selected) {
                (false, _) => petunia_core::SelectionOp::Replace,
                (true, true) => petunia_core::SelectionOp::Subtract,
                (true, false) => petunia_core::SelectionOp::Add,
            }
        } else if subtract {
            petunia_core::SelectionOp::Subtract
        } else if extend {
            petunia_core::SelectionOp::Add
        } else {
            petunia_core::SelectionOp::Replace
        };
        // A seleção muda pelo core, nunca escrevendo flags da malha daqui.
        if self
            .state
            .apply_component_selection(&petunia_core::ComponentSet::Faces(picked), op)
            .is_err()
        {
            return false;
        }
        let selected = self.state.project.active_mesh().map_or(0, |mesh| {
            mesh.faces.iter().filter(|face| face.selected).count()
        });
        let status = match (click, first) {
            (true, Some(face)) => crate::tr::fill(
                &self
                    .state
                    .t_id(petunia_config::text_id::STATUS_FACE_SELECTED),
                &[("face", face.to_string())],
            ),
            (true, None) => self
                .state
                .t_id(petunia_config::text_id::STATUS_NO_FACE_UNDER_THE_CURSOR),
            (false, _) => crate::tr::fill(
                &self
                    .state
                    .t_id(petunia_config::text_id::STATUS_UV_FACES_SELECTED),
                &[("count", selected.to_string())],
            ),
        };
        self.state.set_status(status);
        self.sync_viewport_context();
        self.state.mark_dirty();
        true
    }

    /// Pegada do pincel 3D sob `(x, y)` px da viewport, como elipse de tela:
    /// `(achatamento, giro em graus)`. O carimbo pinta os texels dentro de uma
    /// esfera em torno do ponto; numa superfície inclinada isso é um círculo
    /// que a câmera vê achatado pelo cosseno entre a normal e a direção de
    /// vista, com o eixo menor ao longo da normal projetada. `None` fora do
    /// objeto (o cursor volta a ser um círculo).
    pub fn brush_footprint_at(&self, x: f32, y: f32) -> Option<(f32, f32)> {
        if self.state.workspace != Workspace::Paint {
            return None;
        }
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 || !x.is_finite() || !y.is_finite() {
            return None;
        }
        let camera = &self.state.session.camera;
        let (origin, direction) = camera.ray(x / width * 2.0 - 1.0, 1.0 - y / height * 2.0);
        let (face, hit) = pick_face_hit(&self.state, origin, direction)?;
        // `hit` é do mundo; a normal da face é local e vai ao mundo também.
        let normal = crate::projection::active_world_normal(
            &self.state,
            self.state.project.active_mesh()?.face_normal(face),
        );
        let to_eye = camera.eye() - hit;
        let distance = to_eye.length();
        if normal == glam::Vec3::ZERO || !(distance.is_finite() && distance > 1.0e-6) {
            return None;
        }
        let squash = normal.dot(to_eye / distance).abs().clamp(MIN_SQUASH, 1.0);
        let viewport = self.viewport_size;
        let base = project_world_point(camera, viewport, hit)?;
        let tip = project_world_point(camera, viewport, hit + normal * (distance * 0.1));
        let tilt = match tip {
            Some(tip) if (tip[0] - base[0]).hypot(tip[1] - base[1]) > 0.5 => {
                // O eixo Y local do cursor (achatado) aponta para a normal
                // projetada; a rotação do Slint é no sentido horário.
                (tip[1] - base[1]).atan2(tip[0] - base[0]).to_degrees() - 90.0
            }
            _ => 0.0,
        };
        Some((squash, tilt))
    }
}

#[cfg(test)]
mod mirror_tests {
    use crate::{PlaceholderViewport, SlintUiBridge, UiIntent};
    use petunia_core::{AppState, Workspace};

    fn paint_bridge() -> SlintUiBridge<PlaceholderViewport> {
        let mut b = SlintUiBridge::new(AppState::default(), PlaceholderViewport::default());
        b.resize_viewport(800, 600);
        b.apply(UiIntent::SetWorkspace(Workspace::Paint));
        b
    }

    /// B2: o cursor no centro da vista cai no cubo; o ponto da textura volta ao
    /// mesmo pixel da vista (ida e volta 3D → UV → 3D).
    #[test]
    fn brush_cursor_round_trips_between_view_and_texture() {
        let b = paint_bridge();
        let diag = format!(
            "workspace={:?} size={:?} active={:?}",
            b.state.workspace,
            b.viewport_size,
            b.state.project.active().map(|a| a.name.clone())
        );
        let texture = b
            .brush_texture_point_at(400.0, 300.0)
            .unwrap_or_else(|| panic!("sem ponto da textura no centro: {diag}"));
        assert!(
            texture.iter().all(|v| (0.0..=1.0).contains(v)),
            "{texture:?}"
        );
        let view = b
            .texture_point_to_viewport(texture[0], texture[1])
            .expect("ponto da textura volta à vista");
        assert!(
            (view[0] - 400.0).hypot(view[1] - 300.0) < 2.0,
            "ida e volta: {view:?}"
        );
        assert!(b.brush_texture_point_at(f32::NAN, 1.0).is_none());
        assert!(
            b.texture_point_to_viewport(7.0, 7.0).is_none(),
            "fora de toda ilha UV"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{barycentric_2d, barycentric_3d, in_triangle};

    #[test]
    fn barycentric_weights_round_trip_and_reject_degenerate_triangles() {
        let w =
            barycentric_2d([0.25, 0.25], [0.0, 0.0], [1.0, 0.0], [0.0, 1.0]).expect("triângulo");
        assert!(
            (w[0] - 0.5).abs() < 1e-5 && (w[1] - 0.25).abs() < 1e-5 && (w[2] - 0.25).abs() < 1e-5
        );
        assert!(barycentric_2d([0.0, 0.0], [0.0, 0.0], [1.0, 1.0], [2.0, 2.0]).is_none());
        let p = glam::Vec3::new(0.25, 0.25, 0.0);
        let w =
            barycentric_3d(p, glam::Vec3::ZERO, glam::Vec3::X, glam::Vec3::Y).expect("triângulo");
        assert!((w[0] - 0.5).abs() < 1e-5, "{w:?}");
        assert!(barycentric_3d(p, glam::Vec3::ZERO, glam::Vec3::X, glam::Vec3::X * 2.0).is_none());
    }

    #[test]
    fn point_in_triangle_includes_edges_and_rejects_outside() {
        let (a, b, c) = ([0.0, 0.0], [1.0, 0.0], [0.0, 1.0]);
        assert!(in_triangle([0.2, 0.2], a, b, c));
        assert!(in_triangle([0.5, 0.0], a, b, c));
        assert!(!in_triangle([0.8, 0.8], a, b, c));
        // Ordem dos vértices não importa.
        assert!(in_triangle([0.2, 0.2], a, c, b));
    }
}
