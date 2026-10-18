//! Workspace UV: editor, transformações, seams e pins.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Enquadra a câmera 3D na face cujo UV contém `(u, v)` (PiP do Paint).
    /// Tenta `v` e `1 - v`, pois o canvas tem origem no topo.
    pub fn frame_camera_on_uv(&mut self, u: f32, v: f32) -> bool {
        fn inside(poly: &[[f32; 2]], p: [f32; 2]) -> bool {
            (1..poly.len().saturating_sub(1)).any(|i| {
                let (a, b, c) = (poly[0], poly[i], poly[i + 1]);
                let d = |p1: [f32; 2], p2: [f32; 2], p3: [f32; 2]| {
                    (p1[0] - p3[0]) * (p2[1] - p3[1]) - (p2[0] - p3[0]) * (p1[1] - p3[1])
                };
                let (d1, d2, d3) = (d(p, a, b), d(p, b, c), d(p, c, a));
                let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
                let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
                !(neg && pos)
            })
        }
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        let hit = [[u, v], [u, 1.0 - v]].into_iter().find_map(|p| {
            mesh.faces
                .iter()
                .find(|f| f.uv.len() >= 3 && inside(&f.uv, p))
        });
        let Some(face) = hit else {
            return false;
        };
        // A câmera vive no mundo: a face é enquadrada na posição do objeto.
        let world = crate::projection::active_world_matrix(&self.state);
        let points: Vec<glam::Vec3> = face
            .verts
            .iter()
            .filter_map(|&i| mesh.verts.get(i as usize))
            .map(|v| world.transform_point3(glam::Vec3::from_array(v.pos)))
            .collect();
        if points.is_empty() {
            return false;
        }
        let center = points.iter().copied().sum::<glam::Vec3>() / points.len() as f32;
        let radius = points
            .iter()
            .map(|p| p.distance(center))
            .fold(0.0f32, f32::max);
        self.state
            .session
            .camera
            .frame(center, radius.max(0.25) * 2.5);
        true
    }

    /// Constrói a geometria 2D do editor UV a partir das UVs reais da malha.
    ///
    /// O quadrado 0..1 vira uma caixa de 256 px; cada face contribui um contorno
    /// fechado. Malhas grandes são truncadas e o editor avisa, em vez de
    /// construir uma string gigante em silêncio.
    pub(crate) fn build_uv_editor(&self) -> UvEditorModel {
        const BOX: f32 = 256.0;
        const MAX_FACES: usize = 2_000;
        let Some(mesh) = self
            .state
            .session
            .tools
            .uv_transform
            .as_ref()
            .map(|session| session.preview_mesh())
            .or_else(|| self.state.project.active_mesh())
        else {
            return UvEditorModel::default();
        };
        let face_count = mesh.faces.len();
        let mut commands = String::new();
        let mut seam_commands = String::new();
        let mut selected_commands = String::new();
        let mut pinned_commands = String::new();

        // Com Sync desligado a seleção 3D não aparece como seleção UV.
        let uv_selection = &self.state.session.uv_selection;
        let uv_faces = uv_selection.faces(mesh);
        for (face_idx, face) in mesh.faces.iter().enumerate().take(MAX_FACES) {
            if face.uv.len() < 3 {
                continue;
            }
            let is_selected = uv_faces.contains(&face_idx) || (uv_selection.sync && face.selected);

            // Base wireframe path / Caminho da malha de arame base
            for (index, uv) in face.uv.iter().enumerate() {
                if !uv[0].is_finite() || !uv[1].is_finite() {
                    continue;
                }
                let x = uv[0] * BOX;
                let y = (1.0 - uv[1]) * BOX;
                if index == 0 {
                    commands.push_str(&format!("M {x:.2} {y:.2} "));
                } else {
                    commands.push_str(&format!("L {x:.2} {y:.2} "));
                }
            }
            commands.push_str("Z ");

            // Selected face highlight path / Destaque de faces UV selecionadas
            if is_selected {
                for (index, uv) in face.uv.iter().enumerate() {
                    if !uv[0].is_finite() || !uv[1].is_finite() {
                        continue;
                    }
                    let x = uv[0] * BOX;
                    let y = (1.0 - uv[1]) * BOX;
                    if index == 0 {
                        selected_commands.push_str(&format!("M {x:.2} {y:.2} "));
                    } else {
                        selected_commands.push_str(&format!("L {x:.2} {y:.2} "));
                    }
                }
                selected_commands.push_str("Z ");
            }

            // Pinned UV corners / Vértices UV fixados
            for (c_idx, uv) in face.uv.iter().enumerate() {
                if mesh.uv_pinned.contains(&(face_idx, c_idx))
                    && uv[0].is_finite()
                    && uv[1].is_finite()
                {
                    let cx = uv[0] * BOX;
                    let cy = (1.0 - uv[1]) * BOX;
                    pinned_commands.push_str(&format!(
                        "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                        cx - 3.0,
                        cy,
                        cx,
                        cy - 3.0,
                        cx + 3.0,
                        cy,
                        cx,
                        cy + 3.0
                    ));
                }
            }

            // Highlighted seam edges / Arestas de costura destacadas
            let n = face.verts.len();
            for i in 0..n {
                let v0 = face.verts[i];
                let v1 = face.verts[(i + 1) % n];
                let edge = (v0.min(v1), v0.max(v1));
                if mesh.uv_seams.contains(&edge) && i < face.uv.len() && (i + 1) % n < face.uv.len()
                {
                    let uv0 = face.uv[i];
                    let uv1 = face.uv[(i + 1) % n];
                    if uv0[0].is_finite()
                        && uv0[1].is_finite()
                        && uv1[0].is_finite()
                        && uv1[1].is_finite()
                    {
                        let x0 = uv0[0] * BOX;
                        let y0 = (1.0 - uv0[1]) * BOX;
                        let x1 = uv1[0] * BOX;
                        let y1 = (1.0 - uv1[1]) * BOX;
                        seam_commands.push_str(&format!("M {x0:.2} {y0:.2} L {x1:.2} {y1:.2} "));
                    }
                }
            }
        }
        let islands = mesh.uv_islands();
        let island_rows = islands
            .iter()
            .take(UV_ISLAND_ROWS_MAX)
            .enumerate()
            .map(|(index, island)| {
                let area = island.area();
                UvIslandRowModel {
                    index,
                    face_count: island.faces.len(),
                    area: if area.is_finite() { area } else { 0.0 },
                    selected: !island.faces.is_empty()
                        && island.faces.iter().all(|fi| uv_faces.contains(fi)),
                }
            })
            .collect();
        UvEditorModel {
            layout_commands: commands,
            seam_commands,
            selected_commands,
            pinned_commands,
            pinned_count: mesh.uv_pinned.len(),
            island_count: islands.len(),
            face_count,
            selected_face: mesh
                .faces
                .iter()
                .position(|face| face.selected)
                .map(|index| index as i32)
                .unwrap_or(-1),
            uv_selected_count: uv_faces.len(),
            truncated: face_count > MAX_FACES,
            islands: island_rows,
        }
    }

    /// Seleciona em UV todas as faces da ilha `index` (ordem de
    /// `Mesh::uv_islands`). Índice inválido ou malha sem UV: `false`.
    pub fn uv_select_island(&mut self, index: usize, extend: bool) -> bool {
        let Some(faces) = self
            .state
            .project
            .active_mesh()
            .and_then(|mesh| mesh.uv_islands().into_iter().nth(index))
            .map(|island| island.faces)
        else {
            return false;
        };
        if !self.state.uv_select_faces(&faces, extend) {
            return false;
        }
        self.state.mark_dirty();
        true
    }

    /// Clique no editor UV 2D: seleciona a face cuja ilha contém o ponto.
    ///
    /// `u`/`v` chegam normalizados em 0..1 com origem embaixo, que é a
    /// convenção do domínio; o editor desenha com origem em cima e converte
    /// antes de chamar.
    pub fn uv_editor_click(&mut self, u: f32, v: f32, extend: bool) -> bool {
        if self.state.session.tools.uv_transform.is_some() {
            return false;
        }
        /// Raio de pick de canto/edge: 6 px no editor de 256 px.
        const PICK_RADIUS: f32 = 6.0 / 256.0;
        if !u.is_finite() || !v.is_finite() {
            return false;
        }
        // Seleção UV é da Application: modo (Corner/Edge/Face/Island) e Sync
        // explícito decidem o resultado, inclusive o reflexo nas faces 3D.
        if !self
            .state
            .uv_click(glam::Vec2::new(u, v), PICK_RADIUS, extend)
        {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_UV_NO_FACE_UNDER_THE_CURSOR),
            );
            return false;
        }
        let (face, total) = self
            .state
            .project
            .active_mesh()
            .map(|mesh| {
                (
                    mesh.uv_face_at(glam::Vec2::new(u, v))
                        .map_or_else(|| "-".to_string(), |f| f.to_string()),
                    self.state.session.uv_selection.faces(mesh).len(),
                )
            })
            .unwrap_or_else(|| ("-".to_string(), 0));
        self.state.mark_dirty();
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_UV_FACE_SELECTED_TOTAL),
            &[("face", face), ("0", total.to_string())],
        ));
        true
    }

    /// Move os cantos UV selecionados (um comando = um Undo).
    pub fn uv_move_selected(&mut self, du: f32, dv: f32) -> bool {
        if du == 0.0 && dv == 0.0 {
            return false;
        }
        if !self.dispatch_uv_transform(petunia_mesh::UvTransform::Translate(glam::Vec2::new(
            du, dv,
        ))) {
            return false;
        }
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_UV_MOVED_BY),
            &[("du", format!("{du:.3}")), ("dv", format!("{dv:.3}"))],
        ));
        true
    }

    /// Escala as UVs selecionadas em torno do centroide.
    pub fn uv_scale_selected(&mut self, factor: f32) -> bool {
        if !factor.is_finite() || factor <= 0.0 {
            return false;
        }
        if !self.dispatch_uv_transform(petunia_mesh::UvTransform::Scale(glam::Vec2::splat(factor)))
        {
            return false;
        }
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_UV_SCALED_X),
            &[("factor", format!("{factor:.3}"))],
        ));
        true
    }

    /// Rotaciona as UVs selecionadas em torno do centroide.
    pub fn uv_rotate_selected(&mut self, degrees: f32) -> bool {
        if degrees == 0.0 {
            return false;
        }
        if !self.dispatch_uv_transform(petunia_mesh::UvTransform::Rotate(degrees.to_radians())) {
            return false;
        }
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_UV_ROTATED),
            &[("degrees", format!("{degrees:.1}"))],
        ));
        true
    }

    /// Despacha um transform UV direto sobre a seleção (falha: seleção vazia,
    /// valor não finito ou nada a mover).
    fn dispatch_uv_transform(&mut self, transform: petunia_mesh::UvTransform) -> bool {
        if self.state.session.uv_selection.is_empty() || !transform.is_finite() {
            return false;
        }
        self.state
            .dispatch(&petunia_core::UvTransformCmd { transform })
            .is_ok()
    }

    /// Marca ou desmarca como costura as arestas selecionadas (ou o contorno
    /// das faces selecionadas) por `UvSeamCmd`: uma entrada de Undo.
    pub fn toggle_selected_uv_seams(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        let on_edges = !mesh.selected_edges.is_empty();
        if !on_edges && !mesh.faces.iter().any(|face| face.selected) {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_UV_SELECT_A_FACE_IN_THE_VIEWPORT),
            );
            return false;
        }
        let command = petunia_core::UvSeamCmd {
            op: petunia_core::SeamOp::Toggle,
            edges: None,
        };
        if self.state.dispatch(&command).is_err() {
            return false;
        }
        let count = self
            .state
            .project
            .active_mesh()
            .map_or(0, |mesh| mesh.uv_seams.len());
        let message = if on_edges {
            petunia_config::text_id::STATUS_UV_SEAMS_ON_SELECTED_EDGES_TOGGLED_TOTAL
        } else {
            petunia_config::text_id::STATUS_UV_SEAMS_ON_THE_SELECTED_FACE_TOGGLED
        };
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(message),
            &[("count", format!("{count}"))],
        ));
        true
    }

    /// Limpa todas as costuras da malha ativa (`UvSeamCmd::ClearAll`).
    pub fn clear_all_uv_seams(&mut self) -> bool {
        if self
            .state
            .project
            .active_mesh()
            .is_none_or(|mesh| mesh.uv_seams.is_empty())
        {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_UV_THERE_ARE_NO_SEAMS_TO_CLEAR),
            );
            return false;
        }
        let command = petunia_core::UvSeamCmd {
            op: petunia_core::SeamOp::ClearAll,
            edges: None,
        };
        if self.state.dispatch(&command).is_err() {
            return false;
        }
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_UV_ALL_SEAMS_CLEARED),
        );
        true
    }

    /// Alterna a fixação dos cantos UV das faces selecionadas (`UvPinCmd`).
    pub fn toggle_selected_uv_pins(&mut self) -> bool {
        let Some(mesh) = self.state.project.active_mesh() else {
            return false;
        };
        let uv_faces = self.state.session.uv_selection.faces(mesh);
        if !mesh.faces.iter().any(|f| f.selected) && uv_faces.is_empty() {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_UV_SELECT_FACE_S_TO_PIN_UNPIN),
            );
            return false;
        }
        let command = petunia_core::UvPinCmd {
            op: petunia_core::PinOp::Toggle,
            corners: None,
        };
        if self.state.dispatch(&command).is_err() {
            return false;
        }
        let count = self
            .state
            .project
            .active_mesh()
            .map_or(0, |mesh| mesh.uv_pinned.len());
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_UV_PINS_TOGGLED_PINNED_CORNERS_TOTAL),
            &[("count", format!("{count}"))],
        ));
        true
    }

    /// Limpa todas as fixações de UV da malha ativa (`UvPinCmd::ClearAll`).
    pub fn clear_all_uv_pins(&mut self) -> bool {
        if self
            .state
            .project
            .active_mesh()
            .is_none_or(|mesh| mesh.uv_pinned.is_empty())
        {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_UV_THERE_ARE_NO_PINS_TO_CLEAR),
            );
            return false;
        }
        let command = petunia_core::UvPinCmd {
            op: petunia_core::PinOp::ClearAll,
            corners: None,
        };
        if self.state.dispatch(&command).is_err() {
            return false;
        }
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_UV_ALL_PINNED_VERTICES_CLEARED),
        );
        true
    }

    pub fn toggle_uv_checker(&mut self) -> bool {
        self.state.session.show_uv_checker = !self.state.session.show_uv_checker;
        let enabled = self.state.session.show_uv_checker;
        self.state.render.mark_dirty();
        self.state.set_status(if enabled {
            "UV Checkerboard ativado"
        } else {
            "UV Checkerboard desativado"
        });
        enabled
    }

    pub fn uv_equalize_texel_density(&mut self) -> bool {
        match petunia_module_uv::UvModule::equalize_texel_density(&mut self.state) {
            Ok(_) => {
                self.state.render.mark_dirty();
                true
            }
            Err(e) => {
                self.state.set_status(crate::tr::fill(
                    &self
                        .state
                        .t_id(petunia_config::text_id::STATUS_TEXEL_DENSITY),
                    &[("e", e.to_string())],
                ));
                false
            }
        }
    }
}
