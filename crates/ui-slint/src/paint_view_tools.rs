//! Leitura da superfície no PAINT: seleção de faces pelo UV no canvas 2D
//! (P3D-132, S3) e pegada elíptica do pincel 3D em superfície inclinada (B1).

use petunia_core::{SelectionDomain, Workspace};

use crate::projection::{pick_face_hit, project_world_point};
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

impl<V: PetuniaViewport> SlintUiBridge<V> {
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
        let Some(mesh) = self.state.project.active_mesh_mut() else {
            return false;
        };
        let first = picked.first().copied();
        if click {
            let was_selected = first.is_some_and(|face| mesh.faces[face].selected);
            if !extend {
                mesh.deselect_all();
            }
            for &face in &picked {
                mesh.faces[face].selected = !(extend && was_selected);
            }
        } else {
            if !extend && !subtract {
                mesh.deselect_all();
            }
            for &face in &picked {
                mesh.faces[face].selected = !subtract;
            }
        }
        mesh.sync_vert_selection_from_faces();
        let selected = mesh.faces.iter().filter(|face| face.selected).count();
        self.state.sync_selection();
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
        let normal = self
            .state
            .project
            .active_mesh()?
            .face_normal(face)
            .normalize_or_zero();
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
mod tests {
    use super::in_triangle;

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
