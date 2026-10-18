//! Gestos de ferramenta na viewport: ponteiro, região, preview e commit/cancel.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Atualiza a preselection de componente sob o cursor.
    ///
    /// Sem X-Ray um componente atrás da geometria não é selecionável, então
    /// também não pode ser destacado: o hover valida profundidade contra a
    /// face frontal antes de aceitar o alvo.
    pub fn hover_component(&mut self, normalized_x: f32, normalized_y: f32) -> bool {
        puffin::profile_function!();
        if normalized_x.is_finite() && normalized_y.is_finite() {
            self.pointer_position = [
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ];
        }
        if self.state.workspace == Workspace::Paint {
            // No Paint o pincel consome o ponteiro: limpar em vez de
            // congelar o último hover do modo Model.
            return self.clear_hover();
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            self.pointer_position = [
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ];
            let snap_before = self.profile_hover_snap;
            let region_before = self.region_hover.clone();
            self.update_profile_preselection(normalized_x, normalized_y);
            // Com um perfil aberto em desenho, o clique adiciona pontos: sem
            // destaque de região para não sugerir outra ação.
            let drawing = self
                .active_profile_resources()
                .is_some_and(|(_, spline)| !spline.closed);
            self.region_hover = if drawing {
                None
            } else {
                self.region_hit_at([
                    normalized_x * self.viewport_size[0],
                    normalized_y * self.viewport_size[1],
                ])
            };
            // A linha elástica do perfil aberto segue o cursor; fora disso só
            // redesenha quando a pré-seleção ou a região sob o cursor mudam.
            return drawing
                || snap_before != self.profile_hover_snap
                || region_before != self.region_hover;
        }
        if matches!(self.grammar_tool(), Some(GrammarTool::Parametric(_))) {
            let hover = self.parametric_handle_at(self.pointer_position);
            if hover != self.parametric_handle_hover {
                self.parametric_handle_hover = hover;
                if hover {
                    self.state.session.tools.hover = petunia_core::HoverTarget::None;
                }
                return true;
            }
            if hover {
                return false;
            }
        }
        if self.state.session.tools.active_tool == "poly_pen" {
            let pixel = [
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ];
            let next = self.poly_pen_target(pixel);
            let changed = next != self.state.session.tools.hover;
            self.state.session.tools.hover = next;
            // A linha até o cursor acompanha o mouse durante a coleta.
            return changed || !self.poly_pen_points.is_empty();
        }
        if self.state.session.tools.active_tool == "shape_builder" && self.shape_builder.is_none() {
            return self.hover_shape_builder([
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ]);
        }
        if self.state.session.tools.active_tool == "push_pull" && self.tool_modal.is_none() {
            let previous = self.region_hover.take();
            self.region_hover = self.region_hit_at([
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            ]);
            if self.region_hover.is_some() {
                self.state.session.tools.hover = petunia_core::HoverTarget::None;
                return previous != self.region_hover;
            }
            if previous.is_some() {
                self.state.session.tools.hover =
                    self.pick_viewport_target(normalized_x, normalized_y);
                return true;
            }
        } else {
            self.region_hover = None;
        }
        if self.state.session.tools.active_tool == "loop_cut" && self.loop_cut.is_none() {
            let previous_edge = match self.state.session.tools.hover {
                petunia_core::HoverTarget::Edge(a, b) => Some((a, b)),
                _ => None,
            };
            let next = self.pick_loop_cut_candidate(normalized_x, normalized_y);
            self.loop_cut_hover_source = next.as_ref().map(|(mesh, _, _)| mesh.clone());
            self.loop_cut_hover_ring = next.as_ref().map(|(_, ring, _)| ring.clone());
            let next_edge = next.map(|(_, _, edge)| edge);
            self.state.session.tools.hover = next_edge
                .map(|(a, b)| petunia_core::HoverTarget::Edge(a, b))
                .unwrap_or_default();
            let changed = self.refresh_loop_cut_hover_preview();
            if let Some(ring) = &self.loop_cut_hover_ring {
                if ring.edge_count() > 0 {
                    self.state.set_status(
                        self.state.t_id(
                            petunia_config::text_id::STATUS_LOOP_CUT_CLICK_TO_PLACE_SCROLL_TO,
                        ),
                    );
                }
                return changed || previous_edge != next_edge;
            }
            return changed || previous_edge.is_some();
        }
        let next = self.pick_viewport_target(normalized_x, normalized_y);

        if next == self.state.session.tools.hover {
            return self.state.session.tools.active_tool == "cut";
        }
        self.state.session.tools.hover = next;
        true
    }

    /// Pré-seleção do Draw (constituição 11: "Idle → pré-seleção"). Com o
    /// plano automático e nenhum perfil em curso, destaca a face que o clique
    /// usaria como plano; depois, mostra o ponto encaixado que o clique criaria.
    pub(crate) fn update_profile_preselection(&mut self, normalized_x: f32, normalized_y: f32) {
        if !normalized_x.is_finite() || !normalized_y.is_finite() {
            self.profile_hover_snap = None;
            self.state.session.tools.hover = petunia_core::HoverTarget::None;
            return;
        }
        let ndc = [normalized_x * 2.0 - 1.0, 1.0 - normalized_y * 2.0];
        let choosing_plane =
            !self.state.profile.workplane_locked && self.active_profile_resources().is_none();
        let (hover, snap) = if choosing_plane {
            let hover = match self.pick_target_for_domain(
                SelectionDomain::Face,
                normalized_x,
                normalized_y,
            ) {
                face @ petunia_core::HoverTarget::Face(_) => face,
                _ => petunia_core::HoverTarget::None,
            };
            let mut mask = if self.state.session.snap_settings.enabled {
                petunia_core::SnapMask::for_target(self.state.session.snap_settings.target)
            } else {
                petunia_core::SnapMask {
                    points: false,
                    edges: false,
                    axes: false,
                    faces: false,
                    grid: false,
                }
            };
            mask.faces = matches!(hover, petunia_core::HoverTarget::Face(_));
            let cursor = glam::Vec2::new(
                normalized_x * self.viewport_size[0],
                normalized_y * self.viewport_size[1],
            );
            (hover, self.screen_snap(cursor, mask, None, None))
        } else {
            let snap = self
                .profile_screen_to_plane_snapped(ndc[0], ndc[1])
                .and_then(|(_, snap)| snap);
            (petunia_core::HoverTarget::None, snap)
        };
        self.state.session.tools.hover = hover;
        self.profile_hover_snap = snap;
    }

    /// Eventos do botão principal na viewport: 0 = pressionar, 1 = mover,
    /// 2 = soltar. Coordenadas em px lógicos da viewport.
    pub fn tool_pointer(&mut self, phase: i32, x: f32, y: f32, shift: bool, ctrl: bool) -> bool {
        self.tool_pointer_ex(phase, x, y, shift, ctrl, false)
    }

    /// Como [`Self::tool_pointer`], com Alt (quebrar tangente no perfil).
    pub fn tool_pointer_ex(
        &mut self,
        phase: i32,
        x: f32,
        y: f32,
        shift: bool,
        ctrl: bool,
        alt: bool,
    ) -> bool {
        let Some(at) = LogicalViewportPoint::new(x, y) else {
            return false;
        };
        self.tool_pointer_intent(ViewportToolPointerIntent {
            phase: ViewportPointerPhase::from(phase),
            at,
            modifiers: PhysicalPointerModifiers { shift, ctrl, alt },
        })
    }

    pub(crate) fn apply_tool_effect(
        &mut self,
        effect: petunia_core::ToolEffect,
        fine: bool,
        snap: bool,
    ) -> bool {
        use petunia_core::ToolEffect as Effect;
        match effect {
            Effect::Nothing | Effect::TypedValue(_) | Effect::TypedCleared => false,
            Effect::Click { at } if self.grammar_tool() == Some(GrammarTool::Decal) => {
                self.decal_click(at)
            }
            Effect::Click { at } if self.grammar_tool() == Some(GrammarTool::SurfacePaint) => {
                self.surface_paint_click(at)
            }
            Effect::Click { at } if self.grammar_tool() == Some(GrammarTool::PolyPen) => {
                self.poly_pen_click(at)
            }
            Effect::Click { at } if self.grammar_tool() == Some(GrammarTool::ShapeBuilder) => {
                self.shape_builder_click(at)
            }
            Effect::Click { at }
                if matches!(self.grammar_tool(), Some(GrammarTool::PlanarPrimitive(_))) =>
            {
                let effect = self
                    .tool_session
                    .begin_latched(at, petunia_core::PressTarget::Surface);
                self.apply_tool_effect(effect, fine, snap)
            }
            Effect::Click { at } if self.grammar_tool() == Some(GrammarTool::DrawProfile) => {
                self.draw_profile_click(at)
            }
            Effect::Click { at } if self.grammar_tool() == Some(GrammarTool::Knife) => {
                let [width, height] = self.viewport_size;
                width > 1.0 && height > 1.0 && self.knife_click(at[0] / width, at[1] / height)
            }
            Effect::Click { at } if self.grammar_tool() == Some(GrammarTool::LoopCut) => {
                if self.loop_cut.is_some() {
                    self.commit_loop_cut()
                } else {
                    self.update_loop_cut_hover(at[0], at[1]);
                    self.place_loop_cut_hover()
                }
            }
            Effect::Click { .. } if self.grammar_tool() == Some(GrammarTool::Slice) => {
                self.slice_anchor.is_some() && self.commit_slice()
            }
            // Clique na alça paramétrica não troca a seleção.
            Effect::Click { .. } if self.tool_press_parametric_handle => false,
            Effect::Click { at } => {
                let [width, height] = self.viewport_size;
                if width > 1.0 && height > 1.0 {
                    self.select_viewport_ext(
                        at[0] / width,
                        at[1] / height,
                        self.tool_press_extend,
                        false,
                    );
                }
                true
            }
            Effect::BeginDrag {
                anchor,
                current,
                target,
            } => {
                if !self.begin_tool_gesture(anchor, target) {
                    self.tool_session.reset();
                    return true;
                }
                self.update_tool_gesture(current, fine, snap);
                true
            }
            Effect::UpdateDrag { current, .. } => self.update_tool_gesture(current, fine, snap),
            Effect::Commit => self.commit_tool_gesture(),
            Effect::Cancel => self.cancel_tool_gesture(),
            Effect::ExitTool => self.exit_grammar_tool(),
            Effect::OpenContextMenu { at } => self.open_viewport_context_menu(at[0], at[1]),
        }
    }

    pub(crate) fn begin_tool_gesture(
        &mut self,
        anchor: [f32; 2],
        target: petunia_core::PressTarget,
    ) -> bool {
        let Some(tool) = self.grammar_tool() else {
            return false;
        };
        let on_handle = matches!(target, petunia_core::PressTarget::Handle(_));
        match tool {
            GrammarTool::Transform(kind) => {
                let profile_target = matches!(target, petunia_core::PressTarget::Handle(0));
                let begun = if profile_target {
                    self.begin_profile_transform(kind, anchor)
                } else if on_handle {
                    self.begin_gizmo_drag(anchor[0], anchor[1])
                } else {
                    self.begin_viewport_transform(kind, anchor[0], anchor[1])
                };
                if begun {
                    self.tool_gesture = Some(ToolGesture::Transform { gizmo: on_handle });
                }
                begun
            }
            GrammarTool::SurfacePaint => self.begin_surface_paint(anchor),
            GrammarTool::Decal => {
                let mode = match target {
                    petunia_core::PressTarget::Handle(decal_tool::DECAL_HANDLE_SCALE) => {
                        DecalMode::Scale
                    }
                    petunia_core::PressTarget::Handle(decal_tool::DECAL_HANDLE_ROTATE) => {
                        DecalMode::Rotate
                    }
                    _ => DecalMode::Move,
                };
                self.begin_decal_gesture(anchor, mode)
            }
            GrammarTool::PolyPen => self.begin_poly_pen_drag(anchor),
            GrammarTool::ShapeBuilder => self.begin_shape_builder(anchor),
            GrammarTool::DrawProfile => self.begin_draw_profile_gesture(anchor, target),
            GrammarTool::PlanarPrimitive(kind) => self.begin_planar_primitive(kind, anchor),
            // O Knife é por cliques: arrastar não abre gesto.
            GrammarTool::Knife => false,
            GrammarTool::LoopCut => {
                if self.loop_cut.is_none() {
                    self.update_loop_cut_hover(anchor[0], anchor[1]);
                    if !self.place_loop_cut_hover() {
                        return false;
                    }
                }
                self.tool_gesture = Some(ToolGesture::LoopSlide { last: anchor });
                true
            }
            GrammarTool::Slice => {
                if !self.begin_slice(anchor[0], anchor[1]) {
                    return false;
                }
                self.tool_gesture = Some(ToolGesture::Slice);
                true
            }
            GrammarTool::Parametric(kind) => {
                // Pela alça, a seleção atual é a do gesto (sem região nem
                // reseleção sob o ponteiro).
                let on_parametric_handle = matches!(
                    target,
                    petunia_core::PressTarget::Handle(PARAMETRIC_HANDLE_TARGET)
                );
                if self.tool_modal.is_none()
                    && !on_parametric_handle
                    && kind == ToolModalKind::PushPull
                    && let Some(hit) = self
                        .region_hit_at(anchor)
                        .or_else(|| self.region_selected.clone())
                {
                    return self.begin_region_gesture(anchor, &hit);
                }
                if self.tool_modal.is_none() {
                    if !on_parametric_handle {
                        self.select_under_anchor_for(kind, anchor);
                    }
                    if !self.begin_tool_modal(kind) {
                        let id = if kind == ToolModalKind::Bevel {
                            petunia_config::text_id::TOOL_GRAMMAR_NEEDS_EDGE
                        } else {
                            petunia_config::text_id::TOOL_GRAMMAR_NEEDS_FACE
                        };
                        let message = self
                            .state
                            .t_id(id)
                            .replace("{tool}", &self.state.t(kind.title_key()));
                        self.state.set_status(message);
                        return false;
                    }
                }
                let kind = self.tool_modal.unwrap_or(kind);
                let Some(frame) = self.drag_frame() else {
                    self.cancel_tool_modal();
                    return false;
                };
                self.tool_gesture = Some(ToolGesture::Parametric {
                    kind,
                    frame,
                    anchor,
                    start_value: self.tool_modal_value,
                });
                true
            }
        }
    }

    /// Arestas por workspace: DRAW/POLY seguem o modo; PAINT e UV mantêm as
    /// faces limpas com o overlay opcional.
    pub(crate) fn edge_mode(&self) -> petunia_render::EdgeMode {
        if self.state.workspace == Workspace::Model {
            self.modeling_mode.edge_mode()
        } else {
            petunia_render::EdgeMode::Overlay
        }
    }

    /// Objetos contornados: a seleção de objetos no domínio Object. Nos
    /// domínios de componente a camada de componentes é o destaque.
    pub(crate) fn outlined_objects(&self) -> (Vec<uuid::Uuid>, Option<uuid::Uuid>) {
        if self.state.selection_domain() != SelectionDomain::Object
            || self.state.workspace != Workspace::Model
        {
            return (Vec::new(), None);
        }
        let selection = &self.state.session.selection;
        (selection.assets.clone(), selection.asset)
    }

    /// Plano de trabalho em destaque (capítulo 05) quando já está decidido:
    /// travado pelo usuário ou fixado pelo perfil em edição. No automático,
    /// antes do 1º clique, o destaque da face sob o cursor mostra o candidato.
    pub(crate) fn workplane_overlay(&self) -> Option<petunia_render::WorkplaneOverlay> {
        let drawing = self.state.workspace == Workspace::Model
            && self.modeling_mode == ModelingMode::Draw
            && self.state.session.tools.active_tool == "draw_profile";
        let decided =
            self.state.profile.workplane_locked || self.active_profile_resources().is_some();
        let profile = &self.state.profile;
        (drawing && decided).then_some(petunia_render::WorkplaneOverlay {
            origin: profile.origin,
            right: profile.right,
            up: profile.up,
        })
    }

    /// Região de perfil sob o ponto (px lógicos da viewport), se não estiver
    /// escondida atrás da malha ativa. Os planos ficam em cache por revisão.
    pub(crate) fn region_hit_at(&mut self, pixel: [f32; 2]) -> Option<petunia_core::RegionHit> {
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 || !pixel[0].is_finite() || !pixel[1].is_finite() {
            return None;
        }
        let cache = self.shape_cache()?;
        if cache.planes.is_empty() {
            return None;
        }
        let ndc = glam::Vec2::new(pixel[0] / width * 2.0 - 1.0, 1.0 - pixel[1] / height * 2.0);
        let camera = &self.state.session.camera;
        let (origin, direction) = camera.ray(ndc.x, ndc.y);
        let hit = petunia_core::region_at_ray(
            &cache.planes,
            origin,
            direction,
            camera.proj == petunia_core::Projection::Perspective,
        )?;
        // Qualquer objeto visível claramente à frente esconde a região (a
        // cena em cache tem BVH; não triangula a malha por evento). Fora do
        // Raio-X/Wireframe, como a oclusão do picking.
        let through = self.state.session.show_xray
            || self.state.session.shading == petunia_core::Shading::Wireframe;
        let hidden = !through
            && self
                .scene_query()
                .nearest_depth(camera, ndc.to_array())
                .is_some_and(|depth| depth < hit.depth - 1.0e-3 * hit.depth.abs().max(1.0));
        (!hidden).then_some(hit)
    }

    /// Push/Pull sobre uma região: prelúdio (imprint ou forma nova) + Extrude
    /// no mesmo gesto, 1 Undo, ajustável na "Última operação".
    pub(crate) fn begin_region_gesture(
        &mut self,
        anchor: [f32; 2],
        hit: &petunia_core::RegionHit,
    ) -> bool {
        self.tool_session.clear_numeric();
        if self.state.primitive_session_valid() {
            self.state.finalize_primitive_session();
        }
        if self.state.view().region_host_face(hit).is_none() {
            if !self.begin_free_region_volume(hit) {
                return false;
            }
            self.tool_gesture = Some(ToolGesture::ProfileDepth {
                start: self.state.profile.depth,
                anchor_y: anchor[1],
            });
            return true;
        }
        if let Err(error) = self.state.begin_region_push_pull(hit) {
            self.state.set_status(error.to_string());
            return false;
        }
        self.region_hover = None;
        *self.region_planes_cache.borrow_mut() = None;
        self.tool_modal = Some(ToolModalKind::Extrude);
        self.keyboard_tool_modal_active = false;
        self.tool_modal_value = 0.0;
        self.sync_viewport_context();
        let Some(frame) = self.drag_frame() else {
            self.cancel_tool_modal();
            return false;
        };
        self.tool_gesture = Some(ToolGesture::Parametric {
            kind: ToolModalKind::Extrude,
            frame,
            anchor,
            start_value: 0.0,
        });
        self.state.mark_dirty();
        true
    }

    /// Contorno (e furos) da região em hover, em px da viewport. Durante o
    /// gesto do Shape Builder também desenha todas as faces já tocadas.
    /// Seleção pelos domínios do DRAW (capítulo 02): Curve escolhe um
    /// segmento do perfil ativo, Point um nó e Region uma região fechada.
    /// Shape (Object) continua no caminho de formas inteiras. Clique no vazio
    /// limpa a seleção do domínio.
    pub(crate) fn select_draw_component(&mut self, pixel: [f32; 2]) -> bool {
        if self.state.session.tools.active_tool == "draw_profile"
            || self.state.selection_domain() != SelectionDomain::Object
        {
            return false;
        }
        let found = match self.draw_component_context.as_str() {
            "segments" => self.profile_segment_hit(pixel).map(|hit| {
                self.state.session.planar_selection.segment = Some(hit.segment);
                self.state.session.planar_selection.point = None;
                self.region_selected = None;
                petunia_config::text_id::STATUS_CURVE_SELECTED
            }),
            "points" => match self.hit_test_profile(pixel[0], pixel[1]) {
                Some(ProfileHitTarget::Anchor(id)) => {
                    self.state.session.planar_selection.point = Some(id);
                    self.state.session.planar_selection.segment = None;
                    self.region_selected = None;
                    Some(petunia_config::text_id::STATUS_PROFILE_POINT_SELECTED)
                }
                _ => None,
            },
            "regions" => self.region_hit_at(pixel).map(|hit| {
                self.region_selected = Some(hit);
                self.state.session.planar_selection.point = None;
                self.state.session.planar_selection.segment = None;
                petunia_config::text_id::STATUS_REGION_SELECTED
            }),
            _ => return false,
        };
        let Some(message) = found else {
            let had = self.state.session.planar_selection.segment.take().is_some()
                | self.region_selected.take().is_some();
            if had {
                self.state.mark_dirty();
            }
            return false;
        };
        self.state.set_status(self.state.t_id(message));
        self.state.session.tools.hover = petunia_core::HoverTarget::None;
        self.state.mark_dirty();
        true
    }

    /// Segmento selecionado (domínio Curve) em comandos de tela.
    pub(crate) fn profile_selection_commands(&self) -> String {
        let mut commands = String::new();
        let Some(segment) = self.state.session.planar_selection.segment else {
            return commands;
        };
        let Some((profile, spline)) = self.active_contour_resources() else {
            return commands;
        };
        if segment >= spline.segment_count() {
            return commands;
        }
        let wp = profile.workplane;
        let [origin, right, up] =
            [wp.origin, wp.right, wp.up].map(|v| glam::DVec3::from_array(v).as_vec3());
        let points = (0..=24).filter_map(|k| {
            spline
                .evaluate_segment(segment, k as f64 / 24.0)
                .ok()
                .map(|p| origin + right * p[0] as f32 + up * p[1] as f32)
        });
        crate::projection::write_clipped_path(
            &self.state.session.camera,
            self.viewport_size,
            points,
            false,
            &mut commands,
        );
        commands
    }

    pub(crate) fn region_hover_commands(&self) -> String {
        if self.solid_shape_builder.is_some() {
            return self.solid_region_commands();
        }
        let mut commands = String::new();
        // A região selecionada (domínio Region) fica destacada sem hover.
        if let Some(hit) = &self.region_selected {
            for ring in std::iter::once(&hit.region.outer).chain(hit.region.holes.iter()) {
                crate::projection::write_clipped_path(
                    &self.state.session.camera,
                    self.viewport_size,
                    ring.iter().map(|p| hit.plane.to_world(*p)),
                    true,
                    &mut commands,
                );
            }
        }
        if let Some(hit) = &self.region_hover {
            for ring in std::iter::once(&hit.region.outer).chain(hit.region.holes.iter()) {
                crate::projection::write_clipped_path(
                    &self.state.session.camera,
                    self.viewport_size,
                    ring.iter().map(|p| hit.plane.to_world(*p)),
                    true,
                    &mut commands,
                );
            }
        }
        for ring in self.shape_builder_touched_rings() {
            crate::projection::write_clipped_path(
                &self.state.session.camera,
                self.viewport_size,
                ring,
                true,
                &mut commands,
            );
        }
        commands
    }

    /// Arrastar sobre um elemento não selecionado o seleciona antes de operar
    /// (como no Cinema 4D e no Modo). Clique sem arrasto continua sendo só
    /// seleção.
    pub(crate) fn select_under_anchor_for(&mut self, kind: ToolModalKind, anchor: [f32; 2]) {
        let domain = match kind {
            ToolModalKind::Bevel => SelectionDomain::Edge,
            ToolModalKind::ScaleSelection => return,
            _ => SelectionDomain::Face,
        };
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 {
            return;
        }
        if self.state.selection_domain() == SelectionDomain::Object {
            self.state.set_selection_domain(domain);
            self.sync_viewport_context();
        }
        let (nx, ny) = (anchor[0] / width, anchor[1] / height);
        let target = self.pick_target_for_domain(self.state.selection_domain(), nx, ny);
        let Some(mesh) = self.state.project.active_mesh() else {
            return;
        };
        let already_selected = match target {
            petunia_core::HoverTarget::Face(index) => {
                mesh.faces.get(index).is_some_and(|face| face.selected)
            }
            petunia_core::HoverTarget::Edge(a, b) => {
                mesh.selected_edges.contains(&(a, b)) || mesh.selected_edges.contains(&(b, a))
            }
            petunia_core::HoverTarget::Vertex(index) => mesh
                .verts
                .get(index as usize)
                .is_some_and(|vertex| vertex.selected),
            // Nada sob o cursor: opera sobre a seleção atual.
            petunia_core::HoverTarget::None | petunia_core::HoverTarget::Object(_) => true,
        };
        if !already_selected {
            self.select_viewport_ext(nx, ny, self.tool_press_extend, false);
        }
    }

    /// Geometria de tela da operação ativa, para o valor seguir o cursor.
    pub(crate) fn drag_frame(&self) -> Option<petunia_core::DragFrame> {
        use petunia_core::transform_projection::project_pixel;
        let modal = self.state.session.tools.modal.as_ref()?;
        let camera = &self.state.session.camera;
        let viewport = glam::Vec2::from_array(self.viewport_size);
        let pivot = project_pixel(camera, viewport, modal.pivot)?;
        let world_per_pixel = project_pixel(camera, viewport, modal.pivot + camera.right())
            .map(|right| (right - pivot).length())
            .filter(|pixels| *pixels > 1.0e-3)
            .map_or(camera.visible_height() / viewport.y.max(1.0), |pixels| {
                1.0 / pixels
            });
        let normal_px_per_unit = project_pixel(camera, viewport, modal.pivot + modal.normal)
            .map(|tip| (tip - pivot).to_array());
        Some(petunia_core::DragFrame {
            pivot_px: pivot.to_array(),
            normal_px_per_unit,
            world_per_pixel,
        })
    }

    pub(crate) fn update_tool_gesture(
        &mut self,
        current: [f32; 2],
        fine: bool,
        snap: bool,
    ) -> bool {
        if self.profile_transform_gesture.is_some() {
            return self.update_profile_transform(current, snap);
        }
        match self.tool_gesture {
            Some(ToolGesture::PlanarPrimitive { kind, anchor }) => {
                self.update_planar_primitive(kind, anchor, current)
            }
            Some(ToolGesture::Transform { .. }) => {
                self.update_viewport_transform_modified(current[0], current[1], fine, snap)
            }
            Some(ToolGesture::Parametric {
                kind,
                frame,
                anchor,
                start_value,
            }) => {
                if !self.tool_session.numeric_text().is_empty() {
                    // O valor digitado vence o mouse (constituição 11).
                    return false;
                }
                let mut value = petunia_core::drag_value(
                    kind.modal_kind(),
                    frame,
                    anchor,
                    current,
                    start_value,
                );
                if fine {
                    value = start_value + (value - start_value) * 0.1;
                }
                self.set_tool_modal_value(value)
            }
            Some(ToolGesture::SurfacePaint) => self.update_surface_paint(current),
            Some(ToolGesture::Decal) => self.update_decal_gesture(current, fine, snap),
            Some(ToolGesture::ShapeBuilder) => self.update_shape_builder(current),
            Some(ToolGesture::ProfilePoint) => {
                self.profile_pointer_move(current[0], current[1], self.tool_pointer_alt)
            }
            Some(ToolGesture::ProfileHandlePull) => {
                let [width, height] = self.viewport_size;
                self.profile_update_drag_handle(
                    current[0] / width.max(1.0),
                    current[1] / height.max(1.0),
                    self.tool_pointer_alt,
                )
            }
            Some(ToolGesture::ProfileDepth { start, anchor_y }) => {
                if !self.tool_session.numeric_text().is_empty() {
                    return false;
                }
                let rate = if fine { 0.005 } else { 0.02 };
                self.set_profile_depth(start + (anchor_y - current[1]) * rate)
            }
            Some(ToolGesture::ProfileRevolve { start, anchor_x }) => {
                if !self.tool_session.numeric_text().is_empty() {
                    return false;
                }
                let rate = if fine { 0.2 } else { 1.0 };
                self.set_profile_revolve_angle(
                    (start + (current[0] - anchor_x) * rate).clamp(1.0, 360.0),
                )
            }
            Some(ToolGesture::LoopSlide { last }) => {
                self.tool_gesture = Some(ToolGesture::LoopSlide { last: current });
                self.scrub_loop_cut_2d(current[0] - last[0], current[1] - last[1], fine)
            }
            Some(ToolGesture::Slice) => self.update_slice_modified(current[0], current[1], snap),
            Some(ToolGesture::PolyPenStrip) => self.update_poly_pen_strip(current),
            None => false,
        }
    }

    pub(crate) fn commit_tool_gesture(&mut self) -> bool {
        if self.profile_transform_gesture.is_some() {
            self.tool_gesture = None;
            return self.commit_profile_transform();
        }
        match self.tool_gesture.take() {
            Some(ToolGesture::PlanarPrimitive { .. }) => self.commit_profile_authoring_preview(),
            Some(ToolGesture::Decal) => self.commit_decal_gesture(),
            Some(ToolGesture::SurfacePaint) => {
                self.surface_paint.anchor = None;
                self.state.mark_dirty();
                true
            }
            Some(ToolGesture::ShapeBuilder) => self.commit_shape_builder(),
            Some(ToolGesture::ProfilePoint | ToolGesture::ProfileHandlePull) => {
                self.profile_pointer_up();
                self.state.mark_dirty();
                true
            }
            Some(ToolGesture::ProfileDepth { .. } | ToolGesture::ProfileRevolve { .. }) => {
                self.commit_profile_volume()
            }
            Some(ToolGesture::LoopSlide { .. }) => self.commit_loop_cut(),
            // O plano fica em prévia até o clique de confirmação (ou Esc).
            Some(ToolGesture::Slice) => true,
            Some(ToolGesture::PolyPenStrip) => {
                let Some(strip) = self.poly_pen_strip.take() else {
                    return false;
                };
                let count = strip.quads();
                let painted = self.state.finish_poly_pen_strip(strip);
                if painted {
                    let message = crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_POLY_PEN_PAINTED),
                        &[("count", count.to_string())],
                    );
                    self.state.set_status(message);
                }
                self.sync_viewport_context();
                painted
            }
            Some(ToolGesture::Transform { gizmo: true }) => self.end_gizmo_drag(),
            Some(ToolGesture::Transform { gizmo: false }) => self.end_viewport_transform(),
            Some(ToolGesture::Parametric { .. }) => self.commit_tool_modal(),
            None => false,
        }
    }

    pub(crate) fn cancel_tool_gesture(&mut self) -> bool {
        if self.profile_transform_gesture.is_some() {
            self.tool_gesture = None;
            return self.cancel_profile_transform();
        }
        match self.tool_gesture.take() {
            Some(ToolGesture::PlanarPrimitive { .. }) => self.cancel_profile_authoring_preview(),
            Some(ToolGesture::Transform { gizmo }) => {
                if gizmo {
                    self.gizmo_drag = None;
                }
                self.cancel_viewport_transform()
            }
            Some(ToolGesture::Parametric { .. }) => self.cancel_tool_modal(),
            Some(ToolGesture::SurfacePaint) => self.cancel_surface_paint(),
            Some(ToolGesture::Decal) => self.cancel_decal_gesture(),
            Some(ToolGesture::ShapeBuilder) => self.cancel_shape_builder(),
            Some(ToolGesture::ProfilePoint | ToolGesture::ProfileHandlePull) => {
                self.cancel_profile_point_drag()
            }
            Some(ToolGesture::ProfileDepth { start, .. }) => {
                self.state.profile.depth = start;
                self.cancel_profile_volume()
            }
            Some(ToolGesture::ProfileRevolve { start, .. }) => {
                self.state.profile.revolve_angle = start;
                self.cancel_profile_volume()
            }
            Some(ToolGesture::LoopSlide { .. }) => self.cancel_loop_cut(),
            Some(ToolGesture::Slice) => self.cancel_slice(),
            Some(ToolGesture::PolyPenStrip) => match self.poly_pen_strip.take() {
                Some(strip) => {
                    self.state.cancel_poly_pen_strip(strip);
                    self.sync_viewport_context();
                    true
                }
                None => false,
            },
            None => false,
        }
    }
}
