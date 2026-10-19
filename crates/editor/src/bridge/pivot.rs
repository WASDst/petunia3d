//! Pivô, cursor 3D e origem de geometria.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn set_pivot_point(&mut self, id: &str) -> bool {
        let Some(pivot) = pivot_from_id(id) else {
            return false;
        };
        if pivot == PivotPoint::Cursor3D {
            if !self.state.session.copy_cursor_to_transform_center() {
                return false;
            }
        } else {
            if self.state.session.pivot_point == pivot {
                return false;
            }
            self.state.session.pivot_point = pivot;
        }
        self.state.mark_dirty();
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_PIVOT),
            &[("0", pivot.as_str().to_string())],
        ));
        true
    }

    /// Ponto de mundo sob um ponto normalizado da viewport: vértice próximo
    /// (imã), senão a superfície sob o ponteiro, senão o chão (y = 0) e, no
    /// último caso, o alvo da câmera. `None` se a entrada não é finita.
    pub(crate) fn pick_world_point(&self, norm_x: f32, norm_y: f32) -> Option<glam::Vec3> {
        if !norm_x.is_finite() || !norm_y.is_finite() {
            return None;
        }
        let nx = norm_x.clamp(0.0, 1.0) * 2.0 - 1.0;
        let ny = 1.0 - norm_y.clamp(0.0, 1.0) * 2.0;

        let (origin, dir) = self.state.session.camera.ray(nx, ny);

        // 1. Snapping magnético a vértices de todas as malhas visíveis
        let mut best_vertex = None;
        let mut min_vert_dist = 0.35_f32;
        for asset in &self.state.project.assets {
            if !asset.visible {
                continue;
            }
            let world = asset.world_matrix();
            for v in &asset.mesh.verts {
                let p = world.transform_point3(v.vec());
                let v_rel = p - origin;
                let t = v_rel.dot(dir);
                if t > 0.0 {
                    let proj = origin + dir * t;
                    let dist = (p - proj).length();
                    if dist < min_vert_dist {
                        min_vert_dist = dist;
                        best_vertex = Some(p);
                    }
                }
            }
        }

        let target_pos = if let Some(v_pos) = best_vertex {
            v_pos
        } else if let Some((_face, hit_pos)) = pick_face_hit(&self.state, origin, dir) {
            hit_pos
        } else if dir.y.abs() > 1e-4 {
            let t = -origin.y / dir.y;
            if t > 0.0 {
                origin + dir * t
            } else {
                let plane_t = (self.state.session.cursor_3d[1] - origin.y) / dir.y;
                if plane_t > 0.0 {
                    origin + dir * plane_t
                } else {
                    self.state.session.camera.target
                }
            }
        } else {
            self.state.session.camera.target
        };
        target_pos.is_finite().then_some(target_pos)
    }

    /// Posiciona o 3D Cursor na viewport dado um ponto normalizado (ou clique na tela).
    pub fn place_cursor_3d(&mut self, norm_x: f32, norm_y: f32) -> bool {
        let Some(target_pos) = self.pick_world_point(norm_x, norm_y) else {
            return false;
        };
        self.state.session.cursor_3d = target_pos.to_array();
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_3D_CURSOR),
            &[
                ("0", format!("{:.2}", target_pos.x)),
                ("1", format!("{:.2}", target_pos.y)),
                ("2", format!("{:.2}", target_pos.z)),
            ],
        ));
        self.state.mark_dirty();
        true
    }

    /// Ajusta uma coordenada individual (X, Y ou Z) do 3D Cursor.
    pub fn set_cursor_3d_coord(&mut self, index: usize, val: f32) -> bool {
        if index < 3 && val.is_finite() {
            self.state.session.cursor_3d[index] = val;
            self.state.mark_dirty();
            true
        } else {
            false
        }
    }

    /// Redefine a posição do 3D Cursor para a origem (0, 0, 0).
    pub fn reset_cursor_3d(&mut self) -> bool {
        self.state.session.cursor_3d = [0.0, 0.0, 0.0];
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_3D_CURSOR_RESET_TO_THE_ORIGIN_0),
        );
        self.state.mark_dirty();
        true
    }

    /// Centraliza o alvo da câmera na posição do 3D Cursor.
    pub fn frame_cursor(&mut self) -> bool {
        if !self.state.session.cursor_3d.iter().all(|v| v.is_finite()) {
            return false;
        }
        self.state.session.camera.target = glam::Vec3::from(self.state.session.cursor_3d);
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_CAMERA_CENTERED_ON_THE_3D_CURSOR),
        );
        self.state.mark_dirty();
        true
    }

    /// Define a origem do objeto ativo para a geometria central.
    pub fn set_origin_geometry(&mut self) -> bool {
        self.state.set_origin_geometry().is_ok()
    }

    /// Define a origem do objeto ativo para o piso / base inferior em Y.
    pub fn set_origin_bottom(&mut self) -> bool {
        self.state.set_origin_bottom().is_ok()
    }

    /// Define a origem do objeto ativo para a coordenada do 3D Cursor.
    pub fn set_origin_cursor(&mut self) -> bool {
        self.state.set_origin_cursor().is_ok()
    }

    /// Define a origem do objeto ativo para o centro dos elementos selecionados.
    pub fn set_origin_selection(&mut self) -> bool {
        self.state.set_origin_selection().is_ok()
    }

    /// Centraliza a geometria do objeto na origem (0, 0, 0) ou origem atual.
    pub fn set_geometry_to_origin(&mut self) -> bool {
        self.state.set_geometry_to_origin().is_ok()
    }

    /// Define a origem do objeto ativo para a origem do mundo.
    pub fn set_origin_world(&mut self) -> bool {
        self.state.set_origin_world().is_ok()
    }

    /// Define a origem do objeto ativo para o ponto sob o mouse (coordenadas
    /// normalizadas da viewport): vértice próximo, superfície ou chão.
    pub fn set_origin_at_screen_point(&mut self, norm_x: f32, norm_y: f32) -> bool {
        let Some(point) = self.pick_world_point(norm_x, norm_y) else {
            return false;
        };
        self.state.set_origin_point(point).is_ok()
    }

    /// Leva o 3D Cursor ao centro da seleção (elementos selecionados, ou a
    /// origem do objeto ativo quando não há elemento selecionado).
    pub fn cursor_to_selection(&mut self) -> bool {
        let Some(asset) = self.state.project.active() else {
            return false;
        };
        let point = petunia_core::origin_commands::selection_center_world(asset)
            .unwrap_or_else(|_| asset.origin_world());
        self.set_cursor_3d_point(point)
    }

    /// Leva o 3D Cursor à origem (pivô) do objeto ativo.
    pub fn cursor_to_active(&mut self) -> bool {
        let Some(asset) = self.state.project.active() else {
            return false;
        };
        let point = asset.origin_world();
        self.set_cursor_3d_point(point)
    }

    fn set_cursor_3d_point(&mut self, point: glam::Vec3) -> bool {
        if !point.is_finite() {
            return false;
        }
        self.state.session.cursor_3d = point.to_array();
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_3D_CURSOR),
            &[
                ("0", format!("{:.2}", point.x)),
                ("1", format!("{:.2}", point.y)),
                ("2", format!("{:.2}", point.z)),
            ],
        ));
        self.state.mark_dirty();
        true
    }

    /// Editar pivô é uma **sessão**: liga a ferramenta Mover (que, nesse modo,
    /// só desloca a origem do objeto), **Enter confirma** e **Esc cancela**,
    /// desfazendo exatamente o que a sessão fez. Chamar de novo confirma.
    pub fn toggle_edit_pivot(&mut self) -> bool {
        if self.state.session.edit_pivot {
            return self.confirm_edit_pivot();
        }
        self.begin_edit_pivot()
    }

    fn begin_edit_pivot(&mut self) -> bool {
        // Recusa já na entrada, com o motivo, em vez de falhar no primeiro arrasto.
        match self.state.project.active() {
            None => {
                self.state.set_status("Select an object to edit its pivot");
                return false;
            }
            Some(asset) => {
                if let Some(reason) = petunia_core::origin_commands::origin_refusal(asset) {
                    self.state.set_status(reason);
                    return false;
                }
            }
        }
        if self.state.session.tools.modal.is_some() || !self.state.toggle_edit_pivot() {
            return false;
        }
        self.state.session.edit_pivot_session = Some(petunia_core::EditPivotSession {
            undo_depth: self.state.project.undo.depth().0,
            previous_tool: self.state.session.tools.active_tool.clone(),
        });
        self.apply(UiIntent::SetActiveTool("move".into()));
        self.state.set_status(
            "Edit Pivot: drag the gizmo to place the pivot. Enter confirms, Esc cancels.",
        );
        true
    }

    /// **Enter**: mantém o pivô onde está e sai do modo.
    pub fn confirm_edit_pivot(&mut self) -> bool {
        self.end_edit_pivot(false)
    }

    /// **Esc**: desfaz o que a sessão fez (os passos ficam disponíveis no Redo) e sai.
    pub fn cancel_edit_pivot(&mut self) -> bool {
        self.end_edit_pivot(true)
    }

    fn end_edit_pivot(&mut self, revert: bool) -> bool {
        if !self.state.session.edit_pivot {
            return false;
        }
        let session = self.state.session.edit_pivot_session.take();
        let mut reverted = 0usize;
        if revert && let Some(session) = &session {
            while self.state.project.undo.depth().0 > session.undo_depth {
                if !self.state.undo() {
                    break;
                }
                reverted += 1;
            }
        }
        self.state.session.edit_pivot = false;
        if let Some(session) = session
            && self.state.session.tools.active_tool == "move"
            && session.previous_tool != "move"
        {
            self.apply(UiIntent::SetActiveTool(session.previous_tool));
        }
        self.state.mark_dirty();
        self.state.set_status(if revert && reverted > 0 {
            "Edit Pivot cancelled: the pivot is back where it was."
        } else if revert {
            "Edit Pivot cancelled."
        } else {
            "Pivot confirmed."
        });
        true
    }

    /// Distância em mundo que um arrasto de tela representa ao longo de um eixo.
    ///
    /// Projeta a direção do eixo em espaço de tela e mede quanto do movimento
    /// do ponteiro caiu nela, convertendo por `visible_height`.
    pub(crate) fn screen_delta_on_axis(
        &self,
        axis: usize,
        total_x: f32,
        total_y: f32,
        viewport: [f32; 2],
    ) -> f32 {
        let world_axis = match axis {
            0 => glam::Vec3::X,
            1 => glam::Vec3::Y,
            _ => glam::Vec3::Z,
        };
        let view_proj = self.state.session.camera.view_proj();
        let project = |point: glam::Vec3| -> Option<[f32; 2]> {
            let clip = view_proj * glam::Vec4::new(point.x, point.y, point.z, 1.0);
            if clip.w <= 0.05 {
                return None;
            }
            let inv_w = 1.0 / clip.w;
            Some([
                (clip.x * inv_w * 0.5 + 0.5) * viewport[0],
                (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * viewport[1],
            ])
        };
        let origin = self.state.session.tools.modal.as_ref().map_or_else(
            || self.state.calculate_pivot(self.state.session.pivot_point),
            |modal| modal.pivot,
        );
        let Some(a) = project(origin) else {
            return 0.0;
        };
        let Some(b) = project(origin + world_axis) else {
            return 0.0;
        };
        let direction = [b[0] - a[0], b[1] - a[1]];
        let length_squared = direction[0] * direction[0] + direction[1] * direction[1];
        if length_squared <= 1.0e-6 {
            return 0.0;
        }
        // Fração do movimento do ponteiro na direção do eixo, em unidades de
        // mundo (a direção projetada corresponde a 1 unidade do eixo).
        (total_x * direction[0] + total_y * direction[1]) / length_squared
    }
}
