//! Canvas PAINT: dimensões, render, grid, brush e simetria.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Dimensões do canvas composto do ativo, quando existe.
    pub fn paint_canvas_dimensions(&self) -> Option<(u32, u32)> {
        let asset = self.state.project.assets.get(self.state.project.active)?;
        if let Some(texture) = asset.texture.as_ref() {
            return Some((texture.w, texture.h));
        }
        let stack = asset.paint_stack.as_ref()?;
        let layer = stack.active()?;
        let canvas = layer.canvas()?;
        Some((canvas.w, canvas.h))
    }

    /// Publica a imagem do canvas 2D na janela, quando houver camada.
    pub fn publish_canvas_image(&mut self, window: &PetuniaSlintShell) {
        if let Some(image) = self.render_paint_canvas() {
            window.set_paint_canvas_image(image);
        }
    }

    /// Converte a camada ativa em imagem Slint para o editor 2D.
    ///
    /// A camada ativa é a superfície que o pincel realmente altera; o composto
    /// (`Asset.texture`) é o que vai para o material.
    pub fn render_paint_canvas(&mut self) -> Option<slint::Image> {
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let asset = self.state.project.assets.get(self.state.project.active)?;
        let canvas = asset
            .paint_stack
            .as_ref()
            .and_then(|stack| stack.active())
            .and_then(|layer| layer.canvas())?;
        let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(canvas.w, canvas.h);
        let pixels = buffer.make_mut_bytes();
        let source = &canvas.pixels;
        let length = pixels.len().min(source.len());
        pixels[..length].copy_from_slice(&source[..length]);
        Some(slint::Image::from_rgba8(buffer))
    }

    /// Grade vetorial contextual do editor 2D, nas coordenadas do quadro Slint.
    pub fn paint_canvas_grid_commands(&self) -> String {
        use std::fmt::Write as _;

        const VIEW_SIZE: f32 = 256.0;
        if !self.paint_pixel_grid {
            return String::new();
        }
        let Some((width, height)) = self.paint_canvas_dimensions() else {
            return String::new();
        };
        let zoom = self.paint_canvas_zoom.clamp(1, 16) as f32;
        let scaled_size = VIEW_SIZE * zoom;
        let offset = (VIEW_SIZE - scaled_size) * 0.5;
        let step_x = scaled_size / width.max(1) as f32;
        let step_y = scaled_size / height.max(1) as f32;
        let mut commands = String::new();

        if step_x >= 4.0 {
            let first = ((-offset) / step_x).ceil().max(0.0) as u32;
            let last = ((VIEW_SIZE - offset) / step_x).floor().min(width as f32) as u32;
            for column in first..=last {
                let x = offset + column as f32 * step_x;
                let _ = write!(&mut commands, "M {x:.2} 0 L {x:.2} {VIEW_SIZE:.2} ");
            }
        }
        if step_y >= 4.0 {
            let first = ((-offset) / step_y).ceil().max(0.0) as u32;
            let last = ((VIEW_SIZE - offset) / step_y).floor().min(height as f32) as u32;
            for row in first..=last {
                let y = offset + row as f32 * step_y;
                let _ = write!(&mut commands, "M 0 {y:.2} L {VIEW_SIZE:.2} {y:.2} ");
            }
        }
        commands
    }

    /// Preenche o HUD da operação e a barra de status contextual.
    ///
    /// O HUD diz *o que está acontecendo e com que valor*; a barra diz *como
    /// controlar*. Os dois nunca repetem a mesma informação.
    pub(crate) fn fill_operation_hud(&self, vm: &mut ShellViewModel) {
        use petunia_config::text_id as t;
        let axis_name = |index: usize| ["X", "Y", "Z"][index.min(2)];
        let tr = |id: petunia_config::TextId| self.state.t_id(id);
        let fill = |id: petunia_config::TextId, values: &[String]| {
            let pairs: Vec<(String, String)> = values
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), v.clone()))
                .collect();
            let refs: Vec<(&str, String)> =
                pairs.iter().map(|(k, v)| (k.as_str(), v.clone())).collect();
            crate::tr::fill(&self.state.t_id(id), &refs)
        };
        let input_line = || {
            let typed = self.tool_session.numeric_text();
            (!typed.is_empty()).then(|| fill(t::HUD_INPUT, &[typed.to_string()]))
        };

        // Ferramenta de decalque: gesto em andamento ou dica de uso.
        if let Some(gesture) = self.decal_gesture.as_ref() {
            let decal = self.active_decal();
            let value = |param| {
                decal
                    .as_ref()
                    .map_or(0.0, |d| decal_tool::decal_value(d, param))
            };
            let mark = |param| if gesture.param == param { "▸ " } else { "" };
            let width = value(DecalParam::Width);
            let width_line = if decal.as_ref().is_some_and(|d| d.anchor.is_some()) {
                fill(t::HUD_DECAL_WIDTH, &[format!("{width:.3}")])
            } else {
                format!("{}: {width:.3}", tr(t::UI_DECAL_SCALE))
            };
            let mut lines = vec![
                format!("{}{width_line}", mark(DecalParam::Width)),
                format!(
                    "{}{}",
                    mark(DecalParam::Rotation),
                    fill(
                        t::HUD_DECAL_ROTATION,
                        &[format!("{:.1}", value(DecalParam::Rotation))]
                    )
                ),
            ];
            lines.extend(input_line());
            vm.operation_hud_active = true;
            vm.operation_hud_title = tr(t::TOOLS_DECAL);
            vm.operation_hud_lines = lines;
            vm.operation_hud_hint = tr(t::HUD_DECAL_HINT);
            vm.context_hint = vm.operation_hud_hint.clone();
            return;
        }
        if self.decal_tool_active() {
            vm.context_hint = tr(t::HUD_DECAL_IDLE_HINT);
            return;
        }

        if self.state.session.tools.active_tool == "loop_cut" && self.loop_cut.is_none() {
            vm.operation_hud_active = true;
            vm.operation_hud_title = self.state.t_id(t::TOOLS_LOOP_CUT);
            vm.operation_hud_lines =
                vec![fill(t::HUD_CUTS, &[self.loop_cut_hover_cuts.to_string()])];
            vm.operation_hud_hint = tr(t::HUD_LOOP_CUT_HOVER_HINT);
            vm.context_hint = vm.operation_hud_hint.clone();
            return;
        }
        if self.state.session.tools.active_tool == "draw_profile" {
            vm.operation_hud_active = true;
            vm.operation_hud_title = self.state.t("tools.draw_profile");
            vm.operation_hud_lines = vec![fill(
                t::HUD_POINTS,
                &[self.active_profile_point_count().to_string()],
            )];
            vm.operation_hud_hint = if self.active_profile_closed() {
                tr(t::HUD_PROFILE_CLOSED_HINT)
            } else {
                tr(t::HUD_PROFILE_OPEN_HINT)
            };
            vm.context_hint = vm.operation_hud_hint.clone();
            return;
        }

        // Ferramenta paramétrica modal.
        if let Some(kind) = self.tool_modal {
            let title = self.state.t(kind.title_key());
            vm.operation_hud_active = true;
            vm.operation_hud_title = title.clone();
            vm.operation_hud_lines = vec![format!(
                "{}   {:.3}",
                tr(kind.label_id()),
                self.tool_modal_value
            )];
            vm.operation_hud_lines.extend(input_line());
            vm.operation_hud_subject = self
                .state
                .project
                .active_mesh()
                .map(|mesh| {
                    let faces = mesh.faces.iter().filter(|face| face.selected).count();
                    let verts = mesh.verts.iter().filter(|vert| vert.selected).count();
                    if faces > 0 {
                        fill(t::HUD_FACES, &[faces.to_string()])
                    } else {
                        fill(t::HUD_POINTS, &[verts.to_string()])
                    }
                })
                .unwrap_or_default();
            vm.operation_hud_hint = if self.is_instant_tool_mode() {
                tr(t::HUD_HINT_CLICK_CONFIRM)
            } else {
                tr(t::HUD_HINT_RELEASE_CONFIRM)
            };
            vm.context_hint = format!("{title} · {}", vm.operation_hud_hint);
            return;
        }

        // Loop Cut.
        if let Some(session) = &self.loop_cut {
            let title = self.state.t_id(t::TOOLS_LOOP_CUT);
            vm.operation_hud_active = true;
            vm.operation_hud_title = title.clone();
            vm.operation_hud_lines = vec![
                fill(t::HUD_CUTS, &[session.cuts.to_string()]),
                fill(t::HUD_SLIDE, &[format!("{:.3}", session.slide)]),
            ];
            vm.operation_hud_subject = fill(t::HUD_CUT_COUNT, &[session.cuts.to_string()]);
            vm.operation_hud_hint = tr(t::HUD_LOOP_CUT_HINT);
            vm.context_hint = format!("{title} · {}", vm.operation_hud_hint);
            return;
        }

        if self.state.session.tools.active_tool == "cut"
            && let Some(session) = &self.state.session.tools.cut_session
        {
            vm.operation_hud_active = true;
            vm.operation_hud_title = tr(t::HUD_CUT_TITLE);
            vm.operation_hud_lines = vec![fill(t::HUD_SEGMENTS, &[session.segments.to_string()])];
            vm.operation_hud_hint = tr(t::HUD_CUT_HINT);
            vm.context_hint = vm.operation_hud_hint.clone();
            if let Some(point) = session.edge_start {
                // Ponto do corte é local: projeta com view_proj × mundo.
                let clip = self.state.session.camera.view_proj()
                    * crate::projection::active_world_matrix(&self.state)
                    * point.position.extend(1.0);
                if clip.w > 0.0 {
                    let x = (clip.x / clip.w * 0.5 + 0.5) * self.viewport_size[0];
                    let y = (0.5 - clip.y / clip.w * 0.5) * self.viewport_size[1];
                    vm.operation_preview_commands = format!(
                        "M {x:.2} {y:.2} L {:.2} {:.2}",
                        self.pointer_position[0], self.pointer_position[1]
                    );
                }
            }
            return;
        }

        // Transformação por arrasto (gizmo ou ferramenta ativa).
        if let Some(modal) = self.state.session.tools.modal.as_ref() {
            let title = self.state.t(match modal.kind {
                petunia_core::ModalKind::Move => "tools.move",
                petunia_core::ModalKind::Rotate => "tools.rotate",
                petunia_core::ModalKind::Scale => "tools.scale",
                petunia_core::ModalKind::Extrude => "tools.extrude",
                petunia_core::ModalKind::ExtrudeIndividual => "tools.extrude_individual",
                petunia_core::ModalKind::Inset => "tools.inset",
                petunia_core::ModalKind::Bevel => "tools.bevel",
                petunia_core::ModalKind::PushPull => "tools.push_pull",
            });
            let mut lines = Vec::new();
            match modal.constraint {
                petunia_core::ModalConstraint::Axis(i) => {
                    lines.push(format!("{}   {:.3}", axis_name(i), modal.value));
                }
                petunia_core::ModalConstraint::Plane(i) => {
                    lines.push(fill(
                        t::HUD_PLANE,
                        &[
                            format!("{}{}", axis_name((i + 1) % 3), axis_name((i + 2) % 3)),
                            format!("{:.3}", modal.value),
                        ],
                    ));
                }
                petunia_core::ModalConstraint::Free => {
                    let components = modal.components;
                    if modal.kind == petunia_core::ModalKind::Move {
                        lines.push(format!("X   {:.3}", components.x));
                        lines.push(format!("Y   {:.3}", components.y));
                        lines.push(format!("Z   {:.3}", components.z));
                    } else {
                        lines.push(fill(t::HUD_VALUE, &[format!("{:.3}", modal.value)]));
                    }
                }
            }
            if let Some(fb) = self.state.current_tool_feedback() {
                // ToolFeedback telemetry for magnetic snapping (P3D-131)
                // Telemetria de ToolFeedback para atração magnética (P3D-131)
                if let Some(kind) = fb.snap_kind {
                    lines.push(fill(t::HUD_SNAP, &[self.state.t_id(kind.text_id())]));
                }
            }
            vm.operation_hud_active = true;
            vm.operation_hud_title = title.clone();
            lines.extend(input_line());
            vm.operation_hud_lines = lines;
            let subject = if self.state.session.edit_pivot {
                tr(t::HUD_SUBJECT_PIVOT)
            } else if self.state.selection_domain() == petunia_core::SelectionDomain::Object {
                tr(t::HUD_SUBJECT_OBJECT)
            } else {
                tr(t::HUD_SUBJECT_SELECTION)
            };
            let constraint = match modal.constraint {
                petunia_core::ModalConstraint::Axis(i) => {
                    fill(t::HUD_AXIS, &[axis_name(i).to_string()])
                }
                petunia_core::ModalConstraint::Plane(i) => fill(
                    t::HUD_PLANE_SUBJECT,
                    &[format!(
                        "{}{}",
                        axis_name((i + 1) % 3),
                        axis_name((i + 2) % 3)
                    )],
                ),
                petunia_core::ModalConstraint::Free => tr(t::HUD_FREE),
            };
            vm.operation_hud_subject = format!("{subject} · {constraint}");
            vm.operation_hud_hint = tr(t::HUD_TRANSFORM_HINT);
            vm.context_hint = format!("{title} · {}", vm.operation_hud_hint);
            return;
        }

        if self.state.session.edit_pivot {
            let title = tr(t::HUD_EDIT_PIVOT_TITLE);
            vm.operation_hud_active = true;
            vm.operation_hud_title = title.clone();
            vm.operation_hud_subject = tr(t::HUD_PIVOT);
            vm.operation_hud_lines = vec![tr(t::HUD_EDIT_PIVOT_LINE)];
            vm.operation_hud_hint = tr(t::HUD_EDIT_PIVOT_HINT);
            vm.context_hint = format!("{title} · {}", vm.operation_hud_hint);
            return;
        }

        if let Some(session) = &self.state.session.primitive_session {
            let name = session.descriptor.kind().default_name();
            vm.operation_hud_active = false;
            vm.context_hint = fill(t::HUD_ADD_PRIMITIVE_HINT, &[name.to_string()]);
            return;
        }

        // Em repouso não há dica de ferramenta: a status bar mostra os gestos
        // como chips traduzidos (plano de UI F5) e o domínio já aparece na
        // view bar. Antes era uma frase fixa em inglês.
        vm.operation_hud_active = false;
        vm.context_hint = String::new();
    }

    pub(crate) fn canvas_pixel_at(&self, x: f32, y: f32) -> Option<(u32, u32)> {
        let width = self.viewport_size[0].max(1.0);
        let height = self.viewport_size[1].max(1.0);
        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..width).contains(&x)
            || !(0.0..height).contains(&y)
        {
            return None;
        }
        let ndc_x = x / width * 2.0 - 1.0;
        let ndc_y = 1.0 - y / height * 2.0;
        let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);
        let (face, hit) = self.paint_pick(origin, direction)?;
        let isolate = self.state.session.tools.paint_isolate_selection;
        let uv = petunia_module_paint::PaintModule::face_hit_uv(&self.state, face, hit, isolate)?;
        petunia_module_paint::PaintModule::uv_to_px(&self.state, uv)
    }

    pub(crate) fn pick_paint_color_at(&mut self, x: f32, y: f32) -> bool {
        let Some((px, py)) = self.canvas_pixel_at(x, y) else {
            return false;
        };
        let Some(color) = self
            .state
            .project
            .active()
            .and_then(|asset| asset.texture.as_ref())
            .and_then(|texture| texture.get(px, py))
        else {
            return false;
        };
        let color = [color[0], color[1], color[2]].map(|channel| channel as f32 / 255.0);
        self.state.paint_color = color;
        self.state.session.tools.paint_color = color;
        self.state.mark_dirty();
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_COLOR_SAMPLED_FROM_TEXTURE),
        );
        true
    }

    pub fn project_from_reference(&mut self) -> bool {
        match self.execute_core_command("uv.project_reference") {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub fn bake_reference(&mut self) -> bool {
        match self.execute_core_command("paint.bake_reference") {
            Ok(()) => true,
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    /// Define o escopo de preenchimento do pincel.
    pub fn set_fill_scope(&mut self, scope: &str) -> bool {
        let parsed = match scope {
            "ConnectedPixels" => petunia_core::FillScope::ConnectedPixels,
            "Face" => petunia_core::FillScope::Face,
            "SelectedFaces" => petunia_core::FillScope::SelectedFaces,
            "UvIsland" => petunia_core::FillScope::UvIsland,
            "Object" => petunia_core::FillScope::Object,
            _ => return false,
        };
        self.state.session.tools.fill_scope = parsed;
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_FILL_SCOPE),
            &[("scope", scope.to_string())],
        ));
        true
    }

    /// Define o modo de projeção do pincel.
    pub fn set_brush_projection(&mut self, projection: &str) -> bool {
        let parsed = match projection {
            "Surface" => petunia_core::BrushProjectionMode::Surface,
            "ScreenSpace" => petunia_core::BrushProjectionMode::ScreenSpace,
            _ => return false,
        };
        self.state.session.tools.brush_projection = parsed;
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_PROJECTION),
            &[("projection", projection.to_string())],
        ));
        true
    }

    /// Define a trava de pincel.
    pub fn set_brush_lock(&mut self, lock: &str) -> bool {
        let parsed = match lock {
            "None" => petunia_core::BrushLock::None,
            "FirstObject" => petunia_core::BrushLock::FirstObject,
            "FirstFace" => petunia_core::BrushLock::FirstFace,
            "SelectedFaces" => petunia_core::BrushLock::SelectedFaces,
            _ => return false,
        };
        self.state.session.tools.brush_lock = parsed;
        self.state.session.tools.paint_lock_face = None;
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_BRUSH_LOCK),
            &[("lock", lock.to_string())],
        ));
        true
    }

    /// Tipo de pincel que cada ferramenta de pintura usa.
    pub(crate) fn brush_type_for_tool(tool: &str) -> petunia_core::BrushType {
        use petunia_core::BrushType as B;
        match tool {
            "eraser" => B::Eraser,
            "picker" => B::Eyedropper,
            "airbrush" => B::Airbrush,
            "pixel" => B::Pixel,
            "smudge" => B::Smudge,
            "blur" => B::Blur,
            "dodge" => B::Dodge,
            "burn" => B::Burn,
            "spray" => B::Spray,
            "clone" => B::Clone,
            _ => B::Soft,
        }
    }

    /// Pincel do canvas 2D: o mesmo descriptor do viewport (tipo pela
    /// ferramenta, tamanho pelo slider em px de tela), convertido para texels
    /// pela escala em que a textura aparece no canvas.
    pub(crate) fn canvas_brush_settings(&self) -> petunia_core::BrushSettings {
        let mut settings = self.viewport_brush_settings();
        let texture_w = self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|a| a.texture.as_ref())
            .map_or(256, |t| t.w);
        if self.paint_2d_raster_px > 1.0 {
            settings.size_px *= texture_w as f32 / self.paint_2d_raster_px;
        }
        settings.sanitized()
    }

    pub(crate) fn viewport_brush_settings(&self) -> petunia_core::BrushSettings {
        let mut settings = self.state.brush_settings();
        settings.kind = match self.state.session.tools.active_tool.as_str() {
            "eraser" | "airbrush" | "pixel" | "smudge" | "blur" | "dodge" | "burn" | "spray"
            | "clone" => Self::brush_type_for_tool(&self.state.session.tools.active_tool),
            _ => settings.kind,
        };
        settings.size_px =
            petunia_core::brush_size_px_from_slider(self.state.session.tools.paint_radius);
        settings.sanitized()
    }

    pub(crate) fn paint_dabs_at(
        &mut self,
        points: &[[f32; 2]],
        settings: petunia_core::BrushSettings,
    ) {
        if points.is_empty() {
            return;
        }
        let width = self.viewport_size[0].max(1.0);
        let height = self.viewport_size[1].max(1.0);
        let mut hits = Vec::with_capacity(points.len());
        for &[x, y] in points {
            if !x.is_finite()
                || !y.is_finite()
                || !(0.0..width).contains(&x)
                || !(0.0..height).contains(&y)
            {
                continue;
            }
            let ndc_x = x / width * 2.0 - 1.0;
            let ndc_y = 1.0 - y / height * 2.0;
            let (origin, direction) = self.state.session.camera.ray(ndc_x, ndc_y);
            if let Some(hit) = self.paint_pick(origin, direction) {
                hits.push(hit);
            }
        }
        let Some(&(face, hit)) = hits.first() else {
            return;
        };
        if self.state.session.tools.active_tool == "picker" {
            return;
        }
        if self.state.session.tools.active_tool == "fill" {
            let scope = self.state.session.tools.fill_scope;
            let isolate = self.state.session.tools.paint_isolate_selection;
            let seed =
                petunia_module_paint::PaintModule::face_hit_uv(&self.state, face, hit, isolate)
                    .and_then(|uv| petunia_module_paint::PaintModule::uv_to_px(&self.state, uv));
            petunia_module_paint::PaintModule::canvas_fill_scoped(
                &mut self.state,
                Some(face),
                seed,
                scope,
            );
            self.state.set_status(crate::tr::fill(
                &self.state.t_id(petunia_config::text_id::STATUS_FILLED),
                &[("scope", format!("{scope:?}"))],
            ));
            return;
        }
        if self.paint_target_vertex {
            let radius = (self.state.session.tools.paint_radius * 0.5).max(0.01);
            let strength = self.state.session.tools.paint_strength;
            let color = self.state.paint_color;
            let hit_positions: Vec<_> = hits.iter().map(|(_, hit)| *hit).collect();
            petunia_module_paint::PaintModule::paint_vertex_color_3d_batch(
                &mut self.state,
                &hit_positions,
                radius,
                strength,
                color,
            );
        } else {
            let isolate = self.state.session.tools.paint_isolate_selection;
            petunia_module_paint::PaintModule::paint_mesh_3d_batch_with_settings(
                &mut self.state,
                &hits,
                settings,
                isolate,
            );
        }
    }

    pub(crate) fn adjust_brush_size(&mut self, steps: f32) {
        let next = (self.state.session.tools.paint_radius + steps).clamp(0.01, 100.0);
        self.apply(UiIntent::SetBrushSize(next));
        self.state.set_status(crate::tr::fill(
            &self.state.t_id(petunia_config::text_id::STATUS_BRUSH_SIZE),
            &[("next", format!("{next:.2}"))],
        ));
    }

    pub fn set_brush_hardness(&mut self, val: f32) -> bool {
        self.apply(UiIntent::SetBrushHardness(val));
        self.state.mark_dirty();
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_BRUSH_HARDNESS),
            &[(
                "0",
                format!("{:.0}", self.state.session.tools.brush_hardness * 100.0),
            )],
        ));
        true
    }

    pub fn adjust_brush_hardness(&mut self, delta: f32) {
        let next = (self.state.session.tools.brush_hardness + delta).clamp(0.0, 1.0);
        self.set_brush_hardness(next);
    }

    pub fn toggle_paint_symmetry_x(&mut self) -> bool {
        self.apply(UiIntent::TogglePaintSymmetryX);
        self.state.mark_dirty();
        self.state.session.tools.paint_symmetry_x
    }

    pub fn toggle_paint_symmetry_y(&mut self) -> bool {
        self.apply(UiIntent::TogglePaintSymmetryY);
        self.state.mark_dirty();
        self.state.session.tools.paint_symmetry_y
    }

    pub fn toggle_paint_symmetry_z(&mut self) -> bool {
        self.apply(UiIntent::TogglePaintSymmetryZ);
        self.state.mark_dirty();
        self.state.session.tools.paint_symmetry_z
    }

    pub fn set_paint_symmetry_x(&mut self, val: bool) {
        self.apply(UiIntent::SetPaintSymmetryX(val));
        self.state.mark_dirty();
    }

    pub fn set_paint_symmetry_y(&mut self, val: bool) {
        self.apply(UiIntent::SetPaintSymmetryY(val));
        self.state.mark_dirty();
    }

    pub fn set_paint_symmetry_z(&mut self, val: bool) {
        self.apply(UiIntent::SetPaintSymmetryZ(val));
        self.state.mark_dirty();
    }
}
