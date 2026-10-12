//! Loop Cut: candidatos, hover, sessão e commit.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn pick_loop_cut_candidate(
        &self,
        normalized_x: f32,
        normalized_y: f32,
    ) -> Option<(petunia_core::Mesh, petunia_core::LoopRing, (u32, u32))> {
        let mesh = self.state.project.active_mesh()?;
        let (width, height) = (self.viewport_size[0], self.viewport_size[1]);
        if width <= 1.0 || height <= 1.0 {
            return None;
        }
        let cursor_px = [normalized_x * width, normalized_y * height];
        let camera = &self.state.session.camera;
        // Vértices locais projetados com view_proj × mundo.
        let view_proj = camera.view_proj() * crate::projection::active_world_matrix(&self.state);

        let project_pos = |p: glam::Vec3| -> Option<[f32; 2]> {
            let ndc = view_proj.project_point3(p);
            if !ndc.is_finite() || ndc.z < -1.0 || ndc.z > 1.0 {
                return None;
            }
            Some([(ndc.x * 0.5 + 0.5) * width, (0.5 - ndc.y * 0.5) * height])
        };

        let dist_to_segment = |pt: [f32; 2], a: [f32; 2], b: [f32; 2]| -> f32 {
            let ab = [b[0] - a[0], b[1] - a[1]];
            let ap = [pt[0] - a[0], pt[1] - a[1]];
            let len_sq = ab[0] * ab[0] + ab[1] * ab[1];
            if len_sq <= 1e-4 {
                let dx = pt[0] - a[0];
                let dy = pt[1] - a[1];
                return (dx * dx + dy * dy).sqrt();
            }
            let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / len_sq).clamp(0.0, 1.0);
            let proj = [a[0] + t * ab[0], a[1] + t * ab[1]];
            let dx = pt[0] - proj[0];
            let dy = pt[1] - proj[1];
            (dx * dx + dy * dy).sqrt()
        };

        // Any face under the cursor provides a seed; boolean n-gons do not
        // require the user to be within the edge-only fallback tolerance.
        if let petunia_core::HoverTarget::Face(face_idx) =
            self.pick_target_for_domain(SelectionDomain::Face, normalized_x, normalized_y)
            && let Some(face) = mesh.faces.get(face_idx)
        {
            let mut candidates: Vec<_> = (0..face.verts.len())
                .filter_map(|i| {
                    let seed = (face.verts[i], face.verts[(i + 1) % face.verts.len()]);
                    let pa = project_pos(mesh.verts.get(seed.0 as usize)?.vec())?;
                    let pb = project_pos(mesh.verts.get(seed.1 as usize)?.vec())?;
                    Some((dist_to_segment(cursor_px, pa, pb), seed))
                })
                .collect();
            candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (_, seed) in candidates {
                if let Ok(ring) = petunia_core::LoopRing::discover(mesh, seed) {
                    return Some((mesh.clone(), ring, seed));
                }
            }
        }

        // 2. Fallback de proximidade de aresta (24px de tolerância)
        let mut candidates = Vec::new();
        for (a, b) in mesh.edges_unique() {
            let (Some(va), Some(vb)) = (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
            else {
                continue;
            };
            let (Some(pa), Some(pb)) = (project_pos(va.vec()), project_pos(vb.vec())) else {
                continue;
            };
            let dist = dist_to_segment(cursor_px, pa, pb);
            if dist < 24.0 {
                candidates.push((dist, (a, b)));
            }
        }
        candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, seed) in candidates {
            if let Ok(ring) = petunia_core::LoopRing::discover(mesh, seed) {
                return Some((mesh.clone(), ring, seed));
            }
        }

        None
    }

    pub(crate) fn refresh_loop_cut_hover_preview(&self) -> bool {
        let (Some(ring), Some(source)) = (&self.loop_cut_hover_ring, &self.loop_cut_hover_source)
        else {
            return false;
        };
        let Ok(segments) = ring.preview(source, self.loop_cut_hover_cuts, 0.0) else {
            return false;
        };
        !segments.is_empty()
    }

    pub(crate) fn loop_cut_hover_preview_commands(&self) -> String {
        let (Some(ring), Some(source)) = (&self.loop_cut_hover_ring, &self.loop_cut_hover_source)
        else {
            return String::new();
        };
        let Ok(segments) = (if self.loop_cut_balanced {
            ring.preview_balanced(source, self.loop_cut_hover_cuts, 0.0)
        } else {
            ring.preview(source, self.loop_cut_hover_cuts, 0.0)
        }) else {
            return String::new();
        };
        // Segmentos saem da malha local; a projeção espera mundo.
        let world = crate::projection::active_world_matrix(&self.state);
        let mut commands = String::new();
        for [a, b] in segments {
            project_preview_segment(
                &self.state.session.camera,
                self.viewport_size,
                world.transform_point3(a),
                world.transform_point3(b),
                &mut commands,
            );
        }
        commands
    }

    pub fn adjust_loop_cut_hover_count(&mut self, delta: i32) -> bool {
        if self.state.session.tools.active_tool != "loop_cut"
            || self.loop_cut.is_some()
            || self.loop_cut_hover_ring.is_none()
        {
            return false;
        }
        let next = (self.loop_cut_hover_cuts as i32 + delta).clamp(1, 32) as usize;
        if next == self.loop_cut_hover_cuts {
            return false;
        }
        self.loop_cut_hover_cuts = next;
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_LOOP_CUT_CUT_S_CLICK_TO_PLACE),
            &[("next", format!("{next}"))],
        ));
        true
    }

    pub fn place_loop_cut_hover(&mut self) -> bool {
        if self.loop_cut.is_some() || self.state.session.tools.active_tool != "loop_cut" {
            return false;
        }
        let (Some(ring), Some(source)) = (
            self.loop_cut_hover_ring.take(),
            self.loop_cut_hover_source.take(),
        ) else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_LOOP_CUT_MOVE_THE_POINTER_OVER_A),
            );
            return false;
        };
        self.begin_loop_cut_from_ring(ring, source, self.loop_cut_hover_cuts)
    }

    pub fn update_loop_cut_hover(&mut self, x: f32, y: f32) -> bool {
        if self.state.session.tools.active_tool != "loop_cut" || self.loop_cut.is_some() {
            return false;
        }
        if !x.is_finite()
            || !y.is_finite()
            || self.viewport_size[0] <= 1.0
            || self.viewport_size[1] <= 1.0
        {
            return false;
        }
        self.pointer_position = [x, y];
        self.hover_component(x / self.viewport_size[0], y / self.viewport_size[1])
    }

    /// Abre a sessão de loop cut a partir da aresta selecionada.
    pub fn begin_loop_cut(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh().cloned() else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_LOOP_CUT_NO_ACTIVE_MESH),
            );
            return false;
        };
        let Some(seed) = mesh.selected_edges.iter().copied().next() else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_LOOP_CUT_SELECT_AN_EDGE_ON_A),
            );
            return false;
        };
        let Ok(ring) = petunia_core::LoopRing::discover(&mesh, seed) else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_LOOP_CUT_THE_SELECTED_EDGE_IS_NOT),
            );
            return false;
        };
        self.begin_loop_cut_from_ring(ring, mesh, 1)
    }

    pub(crate) fn begin_loop_cut_from_ring(
        &mut self,
        ring: petunia_core::LoopRing,
        mesh: petunia_core::Mesh,
        cuts: usize,
    ) -> bool {
        self.state.freeze_active_primitive();
        let initial_cuts = if self.loop_cut_balanced && cuts < 2 {
            2
        } else {
            cuts.clamp(1, 32)
        };
        self.loop_cut = Some(LoopCutSessionState {
            ring,
            cuts: initial_cuts,
            slide: 0.0,
            balanced: self.loop_cut_balanced,
            source: mesh,
        });
        self.state.session.tools.active_tool = "loop_cut".to_string();
        if !self.apply_loop_cut_preview() {
            self.loop_cut = None;
            return false;
        }
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_LOOP_CUT_DRAG_TO_SLIDE_ENTER_CONFIRMS),
        );
        true
    }

    /// Alterna o modo de corte simétrico / equilibrado (Dual Balanced Loop Rings).
    pub fn toggle_loop_cut_balanced(&mut self) -> bool {
        self.loop_cut_balanced = !self.loop_cut_balanced;
        let balanced = self.loop_cut_balanced;
        if balanced && self.loop_cut_hover_cuts < 2 {
            self.loop_cut_hover_cuts = 2;
        }
        if let Some(session) = self.loop_cut.as_mut() {
            session.balanced = balanced;
            if balanced && session.cuts < 2 {
                session.cuts = 2;
            }
        }
        if self.loop_cut.is_some() {
            self.apply_loop_cut_preview();
        }
        self.state.set_status(if balanced {
            "Loop Cut: Dual Balanced ativado"
        } else {
            "Loop Cut: Dual Balanced desativado"
        });
        self.state.mark_dirty();
        balanced
    }

    pub fn set_loop_cut_balanced(&mut self, balanced: bool) -> bool {
        if self.loop_cut_balanced == balanced {
            return false;
        }
        self.loop_cut_balanced = balanced;
        if balanced && self.loop_cut_hover_cuts < 2 {
            self.loop_cut_hover_cuts = 2;
        }
        if let Some(session) = self.loop_cut.as_mut() {
            session.balanced = balanced;
            if balanced && session.cuts < 2 {
                session.cuts = 2;
            }
        }
        if self.loop_cut.is_some() {
            self.apply_loop_cut_preview();
        }
        self.state.mark_dirty();
        true
    }

    /// Ajusta o slide do loop cut em 2D considerando a projeção em tela da direção de corte.
    pub fn scrub_loop_cut_2d(&mut self, delta_x: f32, delta_y: f32, fine: bool) -> bool {
        let step = if fine { 0.0025 } else { 0.01 };
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        let effective_delta = if let Some(first_edge) = session.ring.edges().first() {
            if let (Some(va), Some(vb)) = (
                session.source.verts.get(first_edge.0 as usize),
                session.source.verts.get(first_edge.1 as usize),
            ) {
                let vp = self.state.session.camera.view_proj()
                    * crate::projection::active_world_matrix(&self.state);
                let pa = vp.project_point3(va.vec());
                let pb = vp.project_point3(vb.vec());
                let screen_dir = glam::Vec2::new(pb.x - pa.x, pb.y - pa.y);
                if screen_dir.length_squared() > 1e-4 {
                    let norm_dir = screen_dir.normalize();
                    let mouse_delta = glam::Vec2::new(delta_x, -delta_y);
                    mouse_delta.dot(norm_dir)
                } else {
                    delta_x - delta_y
                }
            } else {
                delta_x - delta_y
            }
        } else {
            delta_x - delta_y
        };
        session.slide = (session.slide + effective_delta * step).clamp(-1.0, 1.0);
        self.apply_loop_cut_preview()
    }

    /// Ajusta o slide do loop cut e reconstrói a pré-visualização.
    pub fn scrub_loop_cut(&mut self, delta_x: f32, fine: bool) -> bool {
        let step = if fine { 0.0025 } else { 0.01 };
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        session.slide = (session.slide + delta_x * step).clamp(-1.0, 1.0);
        self.apply_loop_cut_preview()
    }

    /// Campo numérico do Slide: não altera a quantidade de cortes.
    pub fn set_loop_cut_slide(&mut self, slide: f32) -> bool {
        if !slide.is_finite() || !(-1.0..=1.0).contains(&slide) {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_LOOP_CUT_SLIDE_MUST_BE_BETWEEN_1),
            );
            return false;
        }
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        if (session.slide - slide).abs() <= f32::EPSILON {
            return false;
        }
        let previous = session.slide;
        session.slide = slide;
        if self.apply_loop_cut_preview() {
            true
        } else {
            if let Some(session) = self.loop_cut.as_mut() {
                session.slide = previous;
            }
            false
        }
    }

    /// Ajusta a quantidade de cortes paralelos (1..=32).
    pub fn set_loop_cut_count(&mut self, cuts: usize) -> bool {
        let Some(session) = self.loop_cut.as_mut() else {
            return false;
        };
        let clamped = cuts.clamp(1, 32);
        if session.cuts == clamped {
            return false;
        }
        session.cuts = clamped;
        self.apply_loop_cut_preview()
    }

    pub fn scroll_loop_cut_count(&mut self, delta_y: f32) -> bool {
        if !delta_y.is_finite() || delta_y.abs() < f32::EPSILON {
            return false;
        }
        let delta = if delta_y > 0.0 { 1 } else { -1 };
        if self.loop_cut.is_some() {
            let current = self.loop_cut.as_ref().map_or(1, |session| session.cuts);
            return self.set_loop_cut_count((current as i32 + delta).clamp(1, 32) as usize);
        }
        self.adjust_loop_cut_hover_count(delta)
    }

    pub fn adjust_loop_cut_count_from_input(&mut self, cuts: usize) -> bool {
        if self.loop_cut.is_some() {
            return self.set_loop_cut_count(cuts);
        }
        let next = cuts.clamp(1, 32);
        let delta = next as i32 - self.loop_cut_hover_cuts as i32;
        self.adjust_loop_cut_hover_count(delta)
    }

    /// Reconstrói a malha a partir do snapshot da sessão, nunca do preview.
    pub(crate) fn apply_loop_cut_preview(&mut self) -> bool {
        let Some(session) = self.loop_cut.as_ref() else {
            return false;
        };
        let res = if session.balanced {
            session
                .ring
                .apply_balanced(&session.source, session.cuts, session.slide)
        } else {
            session
                .ring
                .apply(&session.source, session.cuts, session.slide)
        };
        match res {
            Ok(mesh) => self.show_tool_preview(mesh),
            Err(error) => {
                self.state.set_status(crate::tr::fill(
                    &self.state.t_id(petunia_config::text_id::STATUS_LOOP_CUT),
                    &[("error", format!("{error}"))],
                ));
                false
            }
        }
    }

    /// Confirma o loop cut como uma única operação de undo.
    pub fn commit_loop_cut(&mut self) -> bool {
        let Some(session) = self.loop_cut.take() else {
            return false;
        };
        self.state.session.tools.active_tool = "select".to_string();
        let Ok(cut) = (if session.balanced {
            session
                .ring
                .apply_balanced(&session.source, session.cuts, session.slide)
        } else {
            session
                .ring
                .apply(&session.source, session.cuts, session.slide)
        }) else {
            self.clear_tool_preview();
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_LOOP_CUT_TOPOLOGY_REFUSED_AT_COMMIT),
            );
            return false;
        };
        if !self.commit_tool_result("loop cut", &session.source, cut) {
            return false;
        }
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_LOOP_CUT_2),
            &[
                ("0", format!("{}", session.cuts)),
                (
                    "1",
                    (if session.balanced { " balanced" } else { "" }).to_string(),
                ),
            ],
        ));
        true
    }

    /// Abandona a sessão restaurando a malha original.
    pub fn cancel_loop_cut(&mut self) -> bool {
        if self.loop_cut.take().is_none() {
            return false;
        }
        // O documento nunca recebeu a prévia: cancelar é só descartá-la.
        self.clear_tool_preview();
        self.state.session.tools.active_tool = "select".to_string();
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_LOOP_CUT_CANCELLED),
        );
        true
    }
}
