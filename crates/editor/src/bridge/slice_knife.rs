//! Slice e Knife.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Abre o plano de corte (Slice) ancorado no ponto pressionado.
    pub fn begin_slice(&mut self, x: f32, y: f32) -> bool {
        let mut adjusting_existing = false;
        let mut anchor_pt = [x, y];
        if let Some(old_anchor) = self.slice_anchor {
            let d_anchor = (x - old_anchor[0]).hypot(y - old_anchor[1]);
            let d_end = (x - self.pointer_position[0]).hypot(y - self.pointer_position[1]);
            if d_anchor <= 28.0 {
                adjusting_existing = true;
                anchor_pt = self.pointer_position;
                self.pointer_position = [x, y];
            } else if d_end <= 28.0 {
                adjusting_existing = true;
                anchor_pt = old_anchor;
                self.pointer_position = [x, y];
            } else {
                if self.state.session.tools.cut_session.is_some() {
                    self.commit_slice();
                }
                self.pointer_position = [x, y];
            }
        } else {
            self.pointer_position = [x, y];
        }

        if adjusting_existing {
            // A prévia nunca entrou no documento: ajustar só reabre a sessão.
            if let Some(session) = self.state.session.tools.cut_session.as_mut() {
                session.fill_cap = self.slice_trim;
            }
        } else {
            let Some(mesh) = self.state.project.active_mesh().cloned() else {
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_SLICE_NO_ACTIVE_MESH),
                );
                return false;
            };
            // Plano vem da tela (mundo); a sessão converte para o local.
            let mut session = petunia_core::CutSession::new(mesh)
                .with_object_to_world(crate::projection::active_world_matrix(&self.state));
            session.fill_cap = self.slice_trim;
            self.state.session.tools.cut_session = Some(session);
        }

        self.state.session.tools.active_tool = "slice".to_string();
        self.slice_anchor = Some(anchor_pt);
        self.state.set_status(
            "Slice: arraste para ajustar · T: Trim/Split · Enter/Clique para confirmar · Esc para cancelar",
        );
        true
    }

    /// Atualiza a pré-visualização do plano de corte.
    pub fn update_slice(&mut self, x: f32, y: f32) -> bool {
        self.update_slice_modified(x, y, false)
    }

    /// Atualiza a pré-visualização do plano de corte com restrição angular quando `snap` é verdadeiro.
    pub fn update_slice_modified(&mut self, x: f32, y: f32, snap: bool) -> bool {
        let Some(anchor) = self.slice_anchor else {
            return false;
        };
        let (target_x, target_y) = if snap {
            let dx = x - anchor[0];
            let dy = y - anchor[1];
            let dist = dx.hypot(dy);
            if dist > 1e-4 {
                let angle = dy.atan2(dx);
                let step = std::f32::consts::PI / 12.0; // 15 graus
                let snapped = (angle / step).round() * step;
                (
                    anchor[0] + dist * snapped.cos(),
                    anchor[1] + dist * snapped.sin(),
                )
            } else {
                (x, y)
            }
        } else {
            (x, y)
        };
        self.pointer_position = [target_x, target_y];
        let viewport = petunia_core::LogicalRect::from_min_max(
            [0.0, 0.0],
            [self.viewport_size[0], self.viewport_size[1]],
        );
        if let Some(session) = self.state.session.tools.cut_session.as_mut() {
            session.fill_cap = self.slice_trim;
        }
        let Some(session) = self.state.session.tools.cut_session.as_ref() else {
            return false;
        };
        let Some(sliced) = session.compute_slice(
            &self.state.session.camera,
            anchor,
            [target_x, target_y],
            viewport,
        ) else {
            return false;
        };
        self.show_tool_preview(sliced)
    }

    /// Confirma o corte como uma única operação de undo.
    pub fn commit_slice(&mut self) -> bool {
        // A faca compartilha `cut_session`, então o Slice só age quando é ele que
        // está armado — senão um release cancelaria o corte da faca.
        if self.slice_anchor.is_none() && self.state.session.tools.active_tool != "slice" {
            return false;
        }
        if self.slice_anchor.is_none() {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_SLICE_CLICK_AND_DRAG_TO_DRAW_THE),
            );
            return false;
        }
        let Some(session) = self.state.session.tools.cut_session.take() else {
            self.slice_anchor = None;
            return false;
        };
        self.slice_anchor = None;
        self.state.session.tools.active_tool = "select".to_string();
        // A prévia é o resultado; sem prévia (clique sem arrasto) não há corte.
        let Some(result) = self.tool_preview_mesh().cloned() else {
            self.clear_tool_preview();
            return false;
        };
        if !self.commit_tool_result("slice", &session.source, result) {
            return false;
        }
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_SLICE_APPLIED),
        );
        true
    }

    /// Abandona o plano de corte restaurando a malha original.
    pub fn cancel_slice(&mut self) -> bool {
        if self.slice_anchor.is_none() && self.state.session.tools.active_tool != "slice" {
            return false;
        }
        let Some(session) = self.state.session.tools.cut_session.take() else {
            self.slice_anchor = None;
            if self.state.session.tools.active_tool == "slice" {
                self.state.session.tools.active_tool = "select".to_string();
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_SLICE_CANCELLED),
                );
                return true;
            }
            return false;
        };
        drop(session);
        self.slice_anchor = None;
        self.state.session.tools.active_tool = "select".to_string();
        self.clear_tool_preview();
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_SLICE_CANCELLED),
        );
        true
    }

    pub fn set_slice_trim(&mut self, trim: bool) {
        self.slice_trim = trim;
        if let Some(session) = self.state.session.tools.cut_session.as_mut() {
            session.fill_cap = trim;
        }
        if let Some(anchor) = self.slice_anchor {
            let viewport = petunia_core::LogicalRect::from_min_max(
                [0.0, 0.0],
                [self.viewport_size[0], self.viewport_size[1]],
            );
            if let Some(session) = self.state.session.tools.cut_session.as_ref()
                && let Some(sliced) = session.compute_slice(
                    &self.state.session.camera,
                    anchor,
                    self.pointer_position,
                    viewport,
                )
            {
                self.show_tool_preview(sliced);
            }
        }
    }

    /// Um clique de faca na viewport: primeiro ponto ancora, segundo corta.
    ///
    /// O ponto vem de `AppState::pick_edge`, então a faca corta a aresta que o
    /// usuário realmente apontou. O corte é aplicado como uma única transação.
    /// Prévia da faca com os segmentos já cortados, enquanto a faca está armada.
    /// Os segmentos nunca entram no documento antes do commit.
    pub(crate) fn knife_preview_mesh(&self) -> Option<&petunia_core::Mesh> {
        (self.state.session.tools.active_tool == "cut")
            .then(|| self.tool_preview_mesh())
            .flatten()
    }

    pub fn knife_click(&mut self, normalized_x: f32, normalized_y: f32) -> bool {
        if self.state.session.tools.cut_session.is_none() {
            return false;
        }
        if !normalized_x.is_finite() || !normalized_y.is_finite() {
            return false;
        }
        let petunia_core::HoverTarget::Edge(a, b) =
            self.pick_target_for_domain(SelectionDomain::Edge, normalized_x, normalized_y)
        else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_CUT_POINT_AT_A_VISIBLE_EDGE),
            );
            return false;
        };
        // Índices da aresta são da prévia (ou do documento no 1º segmento).
        let Some(mesh) = self
            .knife_preview_mesh()
            .or_else(|| self.state.project.active_mesh())
        else {
            return false;
        };
        // Extremos locais projetados com view_proj × mundo; `position` (abaixo)
        // continua local porque o corte é aplicado na malha local.
        let vp = self.state.session.camera.view_proj()
            * crate::projection::active_world_matrix(&self.state);
        let va = mesh.verts[a as usize].vec();
        let vb = mesh.verts[b as usize].vec();
        let ca = vp * va.extend(1.0);
        let cb = vp * vb.extend(1.0);
        let screen = |p: glam::Vec4| {
            [
                (p.x / p.w * 0.5 + 0.5) * self.viewport_size[0],
                (0.5 - p.y / p.w * 0.5) * self.viewport_size[1],
            ]
        };
        let pa = screen(ca);
        let pb = screen(cb);
        let mouse = [
            normalized_x * self.viewport_size[0],
            normalized_y * self.viewport_size[1],
        ];
        let delta = [pb[0] - pa[0], pb[1] - pa[1]];
        let length = delta[0] * delta[0] + delta[1] * delta[1];
        if length <= 1.0e-6 {
            return false;
        }
        let t = (((mouse[0] - pa[0]) * delta[0] + (mouse[1] - pa[1]) * delta[1]) / length)
            .clamp(0.0, 1.0);
        let t = (t / cb.w) / ((1.0 - t) / ca.w + t / cb.w);
        let edge = (a, b);
        let position = va.lerp(vb, t);
        let point = petunia_core::CutEdgePoint { edge, position };

        let cut_result = {
            // Resolve a âncora e os dados de leitura antes do borrow mutável:
            // a prévia consulta o bridge e a câmera permanece da sessão.
            let Some(start) = self
                .state
                .session
                .tools
                .cut_session
                .as_ref()
                .and_then(|session| session.edge_start)
            else {
                let Some(session) = self.state.session.tools.cut_session.as_mut() else {
                    return false;
                };
                session.edge_start = Some(point);
                session.anchor = Some([normalized_x, normalized_y]);
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_KNIFE_PICK_THE_SECOND_EDGE_POINT),
                );
                self.state.mark_dirty();
                return true;
            };

            let Some(mesh) = self
                .knife_preview_mesh()
                .or_else(|| self.state.project.active_mesh())
                .cloned()
            else {
                return false;
            };
            // O corte atravessa as faces entre os dois cliques seguindo o que o
            // usuário vê como reta (plano que contém a direção de visão).
            let camera = &self.state.session.camera;
            // Pontos do corte são locais; a direção de vista é de mundo
            // (`cut_knife_path` converte com `object_to_world`).
            let middle = crate::projection::active_world_matrix(&self.state)
                .transform_point3((start.position + point.position) * 0.5);
            let view_direction = if camera.proj == petunia_core::Projection::Perspective {
                (middle - camera.eye()).normalize_or_zero()
            } else {
                camera.forward()
            };
            let Some(session) = self.state.session.tools.cut_session.as_mut() else {
                return false;
            };
            session.cut_knife_path(start, point, &mesh, view_direction)
        };

        match cut_result {
            Ok(cut) => {
                if let Some(session) = self.state.session.tools.cut_session.as_mut() {
                    session.segments += 1;
                    session.edge_start = None;
                    session.anchor = None;
                }
                // Segmento só na prévia; o commit grava tudo (e congela uma
                // primitiva paramétrica) por `ApplyMeshEditCmd`.
                self.show_tool_preview(cut);
                self.state.session.tools.hover = petunia_core::HoverTarget::None;
                self.state
                    .set_status(self.state.t_id(petunia_config::text_id::STATUS_CUT_PREVIEW_CHOOSE_ANOTHER_SEGMENT_ENTER_APPLIES));
                true
            }
            Err(error) => {
                self.state.set_status(crate::tr::fill(
                    &self.state.t_id(petunia_config::text_id::STATUS_CUT),
                    &[("error", error.to_string())],
                ));
                false
            }
        }
    }

    /// Commit all knife segments as one undo record.
    pub fn commit_knife(&mut self) -> bool {
        if self.state.session.tools.active_tool != "cut" {
            return false;
        }
        let Some(session) = self.state.session.tools.cut_session.take() else {
            return false;
        };
        self.state.session.tools.active_tool = "select".into();
        // A prévia acumula os segmentos (o picking da faca mira nela); o
        // documento só muda aqui, numa entrada de Undo.
        let preview = self.tool_preview_mesh().cloned();
        match preview {
            Some(result) if session.segments > 0 => {
                if !self.commit_tool_result("cut segments", &session.source, result) {
                    return false;
                }
            }
            _ => {
                self.clear_tool_preview();
            }
        }
        self.state
            .set_status(self.state.t_id(petunia_config::text_id::STATUS_CUT_APPLIED));
        true
    }

    pub fn cancel_knife(&mut self) -> bool {
        if self.state.session.tools.cut_session.take().is_none() {
            return false;
        }
        // O documento nunca recebeu os segmentos: basta descartar a prévia.
        self.clear_tool_preview();
        self.state.session.tools.active_tool = "select".into();
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_CUT_CANCELLED),
        );
        true
    }
}
