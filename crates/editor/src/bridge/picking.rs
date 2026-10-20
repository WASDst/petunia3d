//! Picking e seleção na viewport 3D.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Status traduzido do domínio de seleção atual.
    pub(crate) fn report_selection_domain(&mut self) {
        let domain = match self.state.selection_domain() {
            SelectionDomain::Object => petunia_config::text_id::SELECTION_DOMAIN_OBJECT,
            SelectionDomain::Vertex => petunia_config::text_id::SELECTION_DOMAIN_POINT,
            SelectionDomain::Edge => petunia_config::text_id::SELECTION_DOMAIN_EDGE,
            SelectionDomain::Face => petunia_config::text_id::SELECTION_DOMAIN_FACE,
        };
        let message = self
            .state
            .t_id(petunia_config::text_id::SELECTION_MODE)
            .replace("{domain}", &self.state.t_id(domain));
        self.state.set_status(message);
    }

    /// A preselection e o clique usam exatamente o mesmo hit test. Ponto e
    /// aresta usam alvos em pixels lógicos, independentes da distância da
    /// câmera; objetos e faces usam a superfície real, nunca uma esfera de
    /// bounding que seleciona no vazio.
    pub fn pick_viewport_target(&self, x: f32, y: f32) -> petunia_core::HoverTarget {
        let filters = self.state.session.selection.filters;
        if filters == petunia_core::SelectionFilters::all() {
            self.pick_contextual_target(filters, x, y)
        } else {
            self.pick_target_for_domain(self.state.selection_domain(), x, y)
        }
    }

    /// Resolução contextual de candidatos (SEL-002).
    /// Ordem de elegibilidade: vértice próximo -> aresta próxima -> face sob o cursor -> objeto.
    pub fn pick_contextual_target(
        &self,
        filters: petunia_core::SelectionFilters,
        x: f32,
        y: f32,
    ) -> petunia_core::HoverTarget {
        use petunia_core::HoverTarget as Target;
        if filters.vertices {
            let hit = self.pick_target_for_domain(SelectionDomain::Vertex, x, y);
            if matches!(hit, Target::Vertex(_)) {
                return hit;
            }
        }
        if filters.edges {
            let hit = self.pick_target_for_domain(SelectionDomain::Edge, x, y);
            if matches!(hit, Target::Edge(_, _)) {
                return hit;
            }
        }
        if filters.faces {
            let hit = self.pick_target_for_domain(SelectionDomain::Face, x, y);
            if matches!(hit, Target::Face(_)) {
                return hit;
            }
        }
        if filters.objects {
            let hit = self.pick_target_for_domain(SelectionDomain::Object, x, y);
            if matches!(hit, Target::Object(_)) {
                return hit;
            }
        }
        Target::None
    }

    pub(crate) fn pick_target_for_domain(
        &self,
        domain: SelectionDomain,
        x: f32,
        y: f32,
    ) -> petunia_core::HoverTarget {
        use petunia_core::HoverTarget as Target;

        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..=1.0).contains(&x)
            || !(0.0..=1.0).contains(&y)
        {
            return Target::None;
        }
        let (width, height) = (self.viewport_size[0], self.viewport_size[1]);
        if width <= 1.0 || height <= 1.0 {
            return Target::None;
        }
        let camera = &self.state.session.camera;
        let scene = self.scene_query();
        let ndc = [x * 2.0 - 1.0, 1.0 - y * 2.0];
        if domain == SelectionDomain::Object {
            return scene
                .nearest_object(camera, ndc)
                .map_or(Target::None, Target::Object);
        }
        let Some(asset) = self
            .state
            .project
            .active()
            .filter(|asset| asset.visible && !asset.locked)
        else {
            return Target::None;
        };
        let mode = match domain {
            SelectionDomain::Vertex => petunia_core::SelectMode::Vertex,
            SelectionDomain::Edge => petunia_core::SelectMode::Edge,
            SelectionDomain::Face => petunia_core::SelectMode::Face,
            SelectionDomain::Object => return Target::None,
        };
        let through = self.state.session.selects_through();
        let viewport = glam::Vec2::new(width, height);
        let cursor = glam::Vec2::from_array(ndc);
        // O picking compara posições com a câmera (mundo): malha editável no
        // mundo, mesmos índices. Copia só com transformação ≠ identidade;
        // TODO(P01 2b): passar `object_to_world` ao picking do core.
        // Faca: os segmentos vivem só na prévia, então o próximo ponto é
        // escolhido sobre a malha já cortada (índices da prévia).
        let knife_preview = self.knife_preview_mesh();
        let Some(base) = knife_preview.or_else(|| self.state.project.active_mesh()) else {
            return Target::None;
        };
        let mesh = crate::projection::mesh_in_world(&self.state, base);
        // A cena em cache tem a BVH da malha avaliada: com modifiers os
        // índices de face não batem com a malha editável e o picking volta
        // ao caminho que triangula a própria malha.
        let hit = if asset.has_enabled_modifiers() || knife_preview.is_some() {
            petunia_core::picking::pick_mesh_filtered(
                &mesh,
                camera,
                viewport,
                cursor,
                mode,
                through,
                |point| scene.point_visible(camera, point),
            )
        } else {
            petunia_core::picking::pick_mesh_in_scene(
                &mesh,
                self.state.project.project.active,
                &scene,
                camera,
                viewport,
                cursor,
                mode,
                through,
            )
        };
        hit.map_or(Target::None, |hit| match hit.component {
            petunia_core::picking::PickComponent::Vertex(i) => Target::Vertex(i as u32),
            petunia_core::picking::PickComponent::Edge(a, b) => Target::Edge(a, b),
            petunia_core::picking::PickComponent::Face(i) => Target::Face(i),
        })
    }

    /// Oclusão do segmento olho→ponto: a preselection respeita faces, salvo
    /// em X-Ray. `origin`/`direction` documentam o raio que gerou `position`;
    /// o teste usa a mesma cena canônica (`point_visible`) do picking.
    pub fn is_occluded(
        &self,
        _origin: glam::Vec3,
        _direction: glam::Vec3,
        position: glam::Vec3,
    ) -> bool {
        if self.state.session.selects_through() {
            return false;
        }
        let scene = self.scene_query();
        !scene.point_visible(&self.state.session.camera, position)
    }

    /// Face do objeto ativo sob o raio, ignorando pontos cobertos por outros
    /// objetos (pintar nunca atravessa a cena).
    pub(crate) fn paint_pick(
        &self,
        origin: glam::Vec3,
        direction: glam::Vec3,
    ) -> Option<(usize, glam::Vec3)> {
        let (face, hit) = pick_face_hit(&self.state, origin, direction)?;
        let scene = self.scene_query();
        scene
            .point_visible(&self.state.session.camera, hit)
            .then_some((face, hit))
    }

    /// Seleção no workspace PAINT: trocar de objeto ou escolher a parte a pintar.
    ///
    /// - clique em outro objeto: ele vira o ativo (e o isolamento o acompanha);
    /// - clique no objeto ativo: seleciona a face (Shift soma, duplo clique
    ///   seleciona a ilha UV inteira), base da máscara "Mask to selection";
    /// - clique no vazio: limpa as faces selecionadas (Shift preserva).
    pub(crate) fn paint_select_at(&mut self, nx: f32, ny: f32, extend: bool, whole_island: bool) {
        use petunia_core::HoverTarget as Target;
        let ndc_x = nx.clamp(0.0, 1.0) * 2.0 - 1.0;
        let ndc_y = 1.0 - ny.clamp(0.0, 1.0) * 2.0;
        let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);
        let object = self.pick_target_for_domain(SelectionDomain::Object, nx, ny);
        let active = self.state.project.active;
        if self.state.selection_domain() == SelectionDomain::Object {
            self.cancel_paint_stroke();
            if let Target::Object(index) = object {
                if !extend
                    || !self
                        .state
                        .session
                        .selection
                        .assets
                        .contains(&self.state.project.assets[index].id)
                {
                    self.state.select_object(Some(index), extend);
                }
                self.state.refresh_isolation();
            } else if !extend {
                self.state.select_object(None, false);
            }
            self.state.mark_dirty();
            return;
        }
        match object {
            Target::Object(index) if index != active => {
                self.clear_component_selection();
                self.state.select_object(Some(index), false);
                self.state.refresh_isolation();
                let name = self
                    .state
                    .project
                    .active()
                    .map_or_else(String::new, |asset| asset.name.clone());
                self.state.set_status(crate::tr::fill(
                    &self.state.t_id(petunia_config::text_id::STATUS_PAINTING),
                    &[("name", name.to_string())],
                ));
            }
            Target::Object(_) => {
                let face = self.paint_pick(origin, direction).map(|(index, _)| index);
                match face {
                    Some(face) => {
                        let Some(mesh) = self.state.project.active_mesh() else {
                            return;
                        };
                        let faces: Vec<usize> = if whole_island {
                            mesh.uv_islands()
                                .into_iter()
                                .find(|island| island.faces.contains(&face))
                                .map(|island| island.faces)
                                .unwrap_or_else(|| vec![face])
                        } else {
                            vec![face]
                        };
                        let was_selected = mesh.faces.get(face).is_some_and(|item| item.selected);
                        // Shift alterna o grupo pelo estado da face clicada.
                        let op = match (extend, was_selected) {
                            (false, _) => petunia_core::SelectionOp::Replace,
                            (true, true) => petunia_core::SelectionOp::Subtract,
                            (true, false) => petunia_core::SelectionOp::Add,
                        };
                        if let Err(error) = self.state.apply_component_selection(
                            &petunia_core::ComponentSet::Faces(faces),
                            op,
                        ) {
                            self.state.set_status(format!("{error}"));
                            return;
                        }
                        self.state.set_status(crate::tr::fill(
                            &self
                                .state
                                .t_id(petunia_config::text_id::STATUS_FACE_SELECTED),
                            &[("face", format!("{face}"))],
                        ));
                    }
                    None => {
                        if !extend {
                            self.clear_component_selection();
                        }
                        self.state.set_status(
                            self.state
                                .t_id(petunia_config::text_id::STATUS_NO_FACE_UNDER_THE_CURSOR),
                        );
                    }
                }
            }
            _ => {
                if !extend {
                    self.clear_component_selection();
                    if self.state.selection_domain() == SelectionDomain::Object {
                        self.state.select_object(None, false);
                    }
                }
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_NOTHING_UNDER_THE_CURSOR),
                );
            }
        }
        self.state.sync_selection();
        self.state.mark_dirty();
    }

    pub fn select_viewport(&mut self, normalized_x: f32, normalized_y: f32, extend: bool) {
        self.select_viewport_ext(normalized_x, normalized_y, extend, false);
    }

    pub fn select_viewport_ext(
        &mut self,
        normalized_x: f32,
        normalized_y: f32,
        extend: bool,
        loop_select: bool,
    ) {
        if !normalized_x.is_finite() || !normalized_y.is_finite() {
            return;
        }
        let now = std::time::Instant::now();
        let max_interval = (self.preferences.double_tap_interval_ms as u128).clamp(300, 800);
        let is_double_click = if let Some((last_pos, last_time)) = self.last_viewport_click {
            let dt = now.duration_since(last_time).as_millis();
            let dx = normalized_x - last_pos[0];
            let dy = normalized_y - last_pos[1];
            (30..=max_interval).contains(&dt) && (dx * dx + dy * dy) < 0.01
        } else {
            false
        };
        self.last_viewport_click = Some(([normalized_x, normalized_y], now));
        let loop_select = loop_select || is_double_click;
        let draw = self.state.workspace == Workspace::Model;
        if draw && self.profile_transform_gesture.is_none() {
            let pixel = [
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ];
            if self.select_draw_component(pixel) {
                return;
            }
            if let Some(profile_id) = self.profile_hit_at(pixel) {
                let drawing = self
                    .active_profile_resources()
                    .is_some_and(|(_, spline)| !spline.closed);
                if (!drawing || self.state.session.tools.active_tool != "draw_profile")
                    && (self.state.session.active_shape != Some(profile_id)
                        || self.state.session.tools.active_tool != "draw_profile")
                {
                    self.activate_profile(profile_id);
                    self.state.set_status(
                        self.state
                            .t_id(petunia_config::text_id::STATUS_SHAPE_SELECTED),
                    );
                    self.state.session.tools.hover = petunia_core::HoverTarget::None;
                    self.state.sync_selection();
                    self.state.mark_dirty();
                    self.reset_transform_fields();
                    return;
                }
            }
        }

        if self.state.session.tools.active_tool == "draw_profile" {
            let ndc_x = normalized_x.clamp(0.0, 1.0) * 2.0 - 1.0;
            let ndc_y = 1.0 - normalized_y.clamp(0.0, 1.0) * 2.0;
            self.add_profile_point(ndc_x, ndc_y);
            return;
        }
        if self.instant_transform && self.state.session.tools.modal.is_some() {
            self.end_viewport_transform();
            return;
        }
        if self.state.session.tools.active_tool == "loop_cut" && self.loop_cut.is_none() {
            let _ = self.place_loop_cut_hover();
            return;
        }
        // No modo Instant um clique confirma a sessão paramétrica em vez de
        // trocar a seleção — é o equivalente ao Enter com o mouse.
        if self.is_instant_tool_mode() && self.tool_modal.is_some() {
            self.commit_tool_modal();
            return;
        }
        // A faca consome o clique antes da seleção: com uma sessão de corte
        // aberta, clicar é escolher ponto de aresta, não selecionar.
        if (self.state.session.tools.active_tool == "cut"
            || self.state.session.tools.active_tool == "knife")
            && self.state.session.tools.cut_session.is_some()
        {
            self.knife_click(normalized_x, normalized_y);
            return;
        }
        if self.state.session.tools.active_tool == "slice" {
            return;
        }
        if self.state.primitive_session_valid() {
            self.state.finalize_primitive_session();
        }
        let ndc_x = normalized_x.clamp(0.0, 1.0) * 2.0 - 1.0;
        let ndc_y = 1.0 - normalized_y.clamp(0.0, 1.0) * 2.0;
        let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);

        if self.state.workspace.is_animate() {
            let screen_point = [
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ];
            let picked = {
                let mut preview = self.animate.preview.borrow_mut();
                let pose = preview.pose(&self.state.project.project, &self.state.session.animate);
                pose.and_then(|p| {
                    petunia_core::pick_bone(
                        &p,
                        &self.state.session.camera,
                        self.viewport_size,
                        screen_point,
                        16.0,
                    )
                })
            };
            self.state.animate_select_bone(picked);
            self.state.mark_dirty();
            return;
        }

        if self.state.workspace == Workspace::Paint {
            if self.state.session.tools.active_tool == "select" {
                self.paint_select_at(normalized_x, normalized_y, extend, loop_select);
                return;
            }
            if let Some((face, hit)) = self.paint_pick(origin, direction) {
                let brush = if self.state.session.tools.active_tool == "fill" {
                    petunia_core::BrushType::Fill
                } else {
                    self.viewport_brush_settings().kind
                };
                if brush == petunia_core::BrushType::Eyedropper {
                    self.pick_paint_color_at(
                        normalized_x * self.viewport_size[0],
                        normalized_y * self.viewport_size[1],
                    );
                } else if self.paint_target_vertex {
                    let radius = (self.state.session.tools.paint_radius * 0.5).max(0.01);
                    let strength = self.state.session.tools.paint_strength;
                    let color = self.state.paint_color;
                    petunia_module_paint::PaintModule::paint_vertex_color_3d(
                        &mut self.state,
                        hit,
                        radius,
                        strength,
                        color,
                    );
                } else {
                    let mut settings = self.viewport_brush_settings();
                    settings.kind = brush;
                    let isolate = self.state.session.tools.paint_isolate_selection;
                    petunia_module_paint::PaintModule::paint_mesh_3d_with_settings(
                        &mut self.state,
                        face,
                        hit,
                        settings,
                        isolate,
                    );
                }
            }
            self.state.mark_dirty();
            return;
        }

        use petunia_core::HoverTarget as Target;
        // O clique confirma a preselection: o alvo sob o cursor vira o hover
        // corrente para o destaque e o rótulo aparecerem de imediato, sem
        // esperar o próximo mousemove. Erro limpa em vez de congelar.
        let hit = self.pick_viewport_target(normalized_x, normalized_y);
        match hit {
            Target::Object(index) => {
                self.state.session.active_shape = None;
                self.state.select_object(Some(index), extend);
                let name = self.state.project.assets[index].name.clone();
                self.state.set_status(crate::tr::fill(
                    &self.state.t_id(petunia_config::text_id::STATUS_SELECTED),
                    &[("name", name.to_string())],
                ));
            }
            Target::Vertex(index) => {
                if loop_select {
                    let count = self
                        .select_loop_through_core(|mesh| mesh.select_vertex_loop(index, extend));
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_SELECTED_VERTEX_LOOP_POINTS),
                        &[("count", format!("{count}"))],
                    ));
                } else {
                    self.click_component(petunia_core::ComponentSet::Vertices(vec![index]), extend);
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_POINT_SELECTED),
                        &[("index", format!("{index}"))],
                    ));
                }
            }
            Target::Edge(a, b) => {
                if loop_select {
                    let count =
                        self.select_loop_through_core(|mesh| mesh.select_edge_loop((a, b), extend));
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_SELECTED_EDGE_LOOP_EDGES),
                        &[("count", format!("{count}"))],
                    ));
                } else {
                    self.click_component(petunia_core::ComponentSet::Edges(vec![(a, b)]), extend);
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_EDGE_SELECTED),
                        &[("a", format!("{a}")), ("b", format!("{b}"))],
                    ));
                }
            }
            Target::Face(face) => {
                let nearest_edge = if loop_select {
                    match self.pick_target_for_domain(
                        SelectionDomain::Edge,
                        normalized_x,
                        normalized_y,
                    ) {
                        petunia_core::HoverTarget::Edge(ea, eb) => Some((ea, eb)),
                        _ => None,
                    }
                } else {
                    None
                };
                if loop_select {
                    let count = self.select_loop_through_core(|mesh| {
                        mesh.select_face_loop(face, nearest_edge, extend)
                    });
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_SELECTED_FACE_LOOP_FACES),
                        &[("count", format!("{count}"))],
                    ));
                } else {
                    self.click_component(petunia_core::ComponentSet::Faces(vec![face]), extend);
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_FACE_SELECTED),
                        &[("face", format!("{face}"))],
                    ));
                }
            }
            Target::None => {
                let pixel = [
                    normalized_x * self.viewport_size[0],
                    normalized_y * self.viewport_size[1],
                ];
                let shape_hit = self.profile_hit_at(pixel).or_else(|| {
                    self.shape_hit_at(pixel).and_then(|h| {
                        let planes = self.shape_planes_cached()?;
                        planes
                            .iter()
                            .find(|sp| sp.plane == h.plane)
                            .and_then(|sp| sp.profiles.first().copied())
                    })
                });
                if let Some(profile_id) = shape_hit {
                    self.activate_profile(profile_id);
                    self.state.select_object(None, false);
                    let name = self
                        .state
                        .project
                        .project
                        .get_profile(profile_id)
                        .map(|p| p.name.clone())
                        .unwrap_or_else(|| "Profile".to_string());
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_SELECTED_SHAPE),
                        &[("name", name.to_string())],
                    ));
                    self.state.session.tools.hover = Target::None;
                    self.state.sync_selection();
                    self.state.mark_dirty();
                    self.reset_transform_fields();
                    return;
                }

                if self.state.selection_domain() == SelectionDomain::Object {
                    self.state.select_object(None, extend);
                }
                self.state.session.active_shape = None;
                if !extend && self.state.selection_domain().is_component() {
                    self.clear_component_selection();
                }
                self.state.set_status(
                    self.state
                        .t_id(petunia_config::text_id::STATUS_NOTHING_UNDER_THE_CURSOR),
                );
            }
        }
        self.state.session.tools.hover = hit;
        self.state.sync_selection();
        self.state.mark_dirty();
        self.reset_transform_fields();
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Clique em componente: sem Shift substitui a seleção; com Shift alterna.
    /// A seleção muda pelo core (website/docs/09-ui/slint-rescue-plan.md §11).
    pub(crate) fn click_component(&mut self, set: petunia_core::ComponentSet, extend: bool) {
        let op = if extend {
            petunia_core::SelectionOp::Toggle
        } else {
            petunia_core::SelectionOp::Replace
        };
        if let Err(error) = self.state.apply_component_selection(&set, op) {
            self.state.set_status(format!("{error}"));
        }
    }

    /// Limpa a seleção de componentes do objeto ativo.
    pub(crate) fn clear_component_selection(&mut self) {
        let _ = self.state.apply_component_selection(
            &petunia_core::ComponentSet::Faces(Vec::new()),
            petunia_core::SelectionOp::Replace,
        );
    }

    /// Seleções de loop/anel: o algoritmo continua no `petunia_mesh` e roda numa
    /// cópia; o conjunto resultante entra pelo core, que é o dono da seleção.
    /// Devolve a quantidade que o algoritmo reportou.
    pub(crate) fn select_loop_through_core(
        &mut self,
        algorithm: impl FnOnce(&mut petunia_core::Mesh) -> usize,
    ) -> usize {
        let Some(mut probe) = self.state.project.active_mesh().cloned() else {
            return 0;
        };
        let count = algorithm(&mut probe);
        let set = match self.state.selection_domain() {
            SelectionDomain::Edge => {
                petunia_core::ComponentSet::Edges(probe.selected_edges.iter().copied().collect())
            }
            SelectionDomain::Face => petunia_core::ComponentSet::Faces(
                probe
                    .faces
                    .iter()
                    .enumerate()
                    .filter_map(|(index, face)| face.selected.then_some(index))
                    .collect(),
            ),
            _ => petunia_core::ComponentSet::Vertices(
                probe
                    .verts
                    .iter()
                    .enumerate()
                    .filter_map(|(index, vertex)| vertex.selected.then_some(index as u32))
                    .collect(),
            ),
        };
        if let Err(error) = self
            .state
            .apply_component_selection(&set, petunia_core::SelectionOp::Replace)
        {
            self.state.set_status(format!("{error}"));
            return 0;
        }
        count
    }
}
