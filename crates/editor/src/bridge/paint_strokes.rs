//! Traços PAINT: pincel, gradiente, shapes, clone e aerógrafo.

use crate::*;

/// Ações de ponteiro do PAINT no início do traço (U05). A UI as resolve pelo
/// keymap `[pointer]`; o bridge nunca lê Shift/Ctrl para decidir semântica.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaintPointerActions {
    /// Reta desde o último ponto pintado.
    pub straight: bool,
    /// Ferramenta Clone: o clique define a origem.
    pub clone_source: bool,
    /// Decalque: o arrasto escala.
    pub decal_scale: bool,
    /// Decalque: o arrasto gira.
    pub decal_rotate: bool,
}

impl PaintPointerActions {
    /// Ações ativas para as teclas físicas, segundo o keymap.
    pub fn resolve(
        keybinds: &petunia_config::keybinds::Keybinds,
        shift: bool,
        ctrl: bool,
        alt: bool,
    ) -> Self {
        use petunia_config::keybinds as kb;
        let held = |action: &str| {
            let mods = keybinds.pointer_modifier(action);
            let any = mods.shift || mods.ctrl || mods.alt;
            any && (!mods.shift || shift) && (!mods.ctrl || ctrl) && (!mods.alt || alt)
        };
        Self {
            straight: held(kb::POINTER_PAINT_STRAIGHT),
            clone_source: held(kb::POINTER_CLONE_SOURCE),
            decal_scale: held(kb::POINTER_DECAL_SCALE),
            decal_rotate: held(kb::POINTER_DECAL_ROTATE),
        }
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Inicia um traço de pintura ou manipulação de decalque contínuo.
    pub fn begin_paint_stroke_at(&mut self, x: f32, y: f32) -> bool {
        self.begin_paint_stroke_with_modifiers(x, y, false, false)
    }

    /// Inicia um traço de pintura ou manipulação interativa de decalque com modificadores (Shift: escala, Ctrl: rotação).
    /// Adaptador de teclas físicas (testes, entradas legadas): resolve as
    /// ações pelo keymap `[pointer]` — nenhuma tecla tem significado fixo.
    pub fn begin_paint_stroke_with_modifiers(
        &mut self,
        x: f32,
        y: f32,
        is_shift: bool,
        is_ctrl: bool,
    ) -> bool {
        let actions =
            PaintPointerActions::resolve(&self.state.ui.keybinds, is_shift, is_ctrl, false);
        self.begin_paint_stroke_actions(x, y, actions)
    }

    /// Início do traço PAINT com as ações já resolvidas pela UI (U05).
    pub fn begin_paint_stroke_actions(
        &mut self,
        x: f32,
        y: f32,
        actions: PaintPointerActions,
    ) -> bool {
        self.validate_surface_target();
        if self.state.workspace != Workspace::Paint {
            return false;
        }
        let luma = self
            .state
            .session
            .tools
            .paint_stencil
            .as_ref()
            .is_some_and(|s| s.luminance);
        self.sync_surface_stencil(luma);
        if self.state.session.tools.active_tool == "select" {
            // A ferramenta de seleção não pinta: o clique troca objeto/face.
            return false;
        }
        if self.state.project.active_mesh().is_none() {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_NO_ACTIVE_MESH_TO_PAINT),
            );
            return false;
        }
        if self.active_layer_is_decal() {
            let is_paint_tool = matches!(
                self.state.session.tools.active_tool.as_str(),
                "brush"
                    | "eraser"
                    | "airbrush"
                    | "pixel"
                    | "smudge"
                    | "blur"
                    | "dodge"
                    | "burn"
                    | "spray"
                    | "clone"
                    | "fill"
            );
            let handle_hit =
                self.decal_handle_at(x, y).is_some() || actions.decal_scale || actions.decal_rotate;
            if handle_hit || !is_paint_tool {
                return self.decal_drag_begin(x, y, actions.decal_scale, actions.decal_rotate);
            }
            let active = self.state.project.active;
            let raster_id = self
                .state
                .project
                .assets
                .get(active)
                .and_then(|a| a.paint_stack.as_ref())
                .and_then(|stack| {
                    stack
                        .layers
                        .iter()
                        .find(|l| {
                            matches!(l.kind, petunia_project::paint_layers::LayerKind::Raster(_))
                        })
                        .map(|l| l.id)
                });
            if let Some(id) = raster_id {
                let id_str = id.to_string();
                self.set_paint_layer_active(&id_str);
            } else {
                self.add_paint_layer();
            }
        }
        if self.state.session.tools.active_tool == "picker" {
            return self.pick_paint_color_at(x, y);
        }
        if self.state.session.tools.active_tool == "clone"
            && (actions.clone_source || self.state.session.tools.clone_source.is_none())
        {
            return self.set_clone_source_at(x, y);
        }
        if self.is_shape_tool() {
            return self.begin_paint_shape_at(x, y);
        }
        if actions.straight {
            let [w, h] = self.viewport_size;
            let (origin, direction) = self.state.camera.ray(x / w * 2.0 - 1.0, 1.0 - y / h * 2.0);
            if let Some((_, hit)) = self.paint_pick(origin, direction) {
                if let Some(from) = self.surface_paint.straight_from {
                    let settings = self.viewport_brush_settings();
                    petunia_module_paint::PaintModule::paint_surface_line(
                        &mut self.state,
                        from,
                        hit,
                        settings,
                        false,
                    );
                    self.surface_paint.straight_from = Some(hit);
                    return true;
                }
                self.surface_paint.straight_from = Some(hit);
            }
        }
        self.state.begin_paint_stroke();
        let settings = self.viewport_brush_settings();
        self.paint_stabilizer.reset();
        let smoothing = self.state.session.tools.brush_style.smoothing;
        let [x, y] = self.paint_stabilizer.filter([x, y], smoothing);
        let dabs = self.paint_sampler.begin([x, y], settings.dab_step_px());
        self.paint_dabs_at(&dabs, settings);
        self.paint_last = Some([x, y]);
        self.state.mark_dirty();
        true
    }

    pub(crate) fn is_gradient_tool(&self) -> bool {
        matches!(
            self.state.session.tools.active_tool.as_str(),
            "gradient" | "gradient_radial"
        )
    }

    /// Aplica o gradiente da âncora ao ponto de soltar, por cima do que existe.
    /// A cor inicial usa a opacidade do pincel; a final é a mesma cor transparente.
    pub(crate) fn commit_gradient(&mut self, radial: bool, from: (u32, u32), to: (u32, u32)) {
        let opacity = self.state.session.tools.paint_strength.clamp(0.0, 1.0);
        let rgb = self
            .state
            .paint_color
            .map(|channel| (channel * 255.0).clamp(0.0, 255.0) as u8);
        let color_start = [rgb[0], rgb[1], rgb[2], (opacity * 255.0).round() as u8];
        let color_end = [rgb[0], rgb[1], rgb[2], 0];
        let shape = if radial {
            let radius = (to.0 as f32 - from.0 as f32).hypot(to.1 as f32 - from.1 as f32);
            petunia_module_paint::GradientShape::Radial {
                center: from,
                radius,
            }
        } else {
            petunia_module_paint::GradientShape::Linear { from, to }
        };
        let command = petunia_module_paint::GradientPaintCmd {
            shape,
            color_start,
            color_end,
        };
        if let Err(error) = self.state.dispatch(&command) {
            self.state.set_status(error.to_string());
            return;
        }
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_GRADIENT_COMMITTED),
        );
    }

    /// Ferramentas de forma ancoram no press e confirmam no release.
    pub fn is_shape_tool(&self) -> bool {
        matches!(
            petunia_core::brush_type_from_kind(self.state.session.tools.paint_brush_kind),
            petunia_core::BrushType::Line
                | petunia_core::BrushType::Rectangle
                | petunia_core::BrushType::Ellipse
        ) || self.is_gradient_tool()
    }

    /// Inicia uma forma (Line/Rectangle/Gradient) no pixel do canvas sob o cursor.
    pub fn begin_paint_shape_at(&mut self, x: f32, y: f32) -> bool {
        if self.state.workspace != Workspace::Paint {
            return false;
        }
        // A forma precisa de canvas para converter UV em pixel.
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let Some((px, py)) = self.canvas_pixel_at(x, y) else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_SHAPE_POINT_AT_THE_SURFACE_TO_ANCHOR),
            );
            return false;
        };
        self.shape_anchor = Some((px, py));
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_SHAPE_ANCHORED_RELEASE_TO_COMMIT),
        );
        true
    }

    /// Confirma a forma como uma única operação de undo.
    pub fn end_paint_shape_at(&mut self, x: f32, y: f32) -> bool {
        let Some((x0, y0)) = self.shape_anchor.take() else {
            return false;
        };
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let Some((x1, y1)) = self.canvas_pixel_at(x, y) else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_SHAPE_RELEASE_POINT_IS_OFF_THE_SURFACE),
            );
            return false;
        };
        if self.is_gradient_tool() {
            let radial = self.state.session.tools.active_tool == "gradient_radial";
            self.commit_gradient(radial, (x0, y0), (x1, y1));
            return true;
        }
        let brush = petunia_core::brush_type_from_kind(self.state.session.tools.paint_brush_kind);
        let color = [
            (self.state.paint_color[0] * 255.0).clamp(0.0, 255.0) as u8,
            (self.state.paint_color[1] * 255.0).clamp(0.0, 255.0) as u8,
            (self.state.paint_color[2] * 255.0).clamp(0.0, 255.0) as u8,
            255,
        ];
        let strength = self.state.session.tools.paint_strength;
        let command = petunia_module_paint::ShapePaintCmd {
            stroke: petunia_module_paint::ShapeStroke {
                x0,
                y0,
                x1,
                y1,
                brush,
                color,
                strength,
            },
        };
        if let Err(error) = self.state.dispatch(&command) {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state.set_status(crate::tr::fill(
            &self
                .state
                .t_id(petunia_config::text_id::STATUS_SHAPE_COMMITTED),
            &[("brush", format!("{brush:?}"))],
        ));
        true
    }

    /// Cancela a forma ancorada sem tocar no documento.
    pub fn cancel_paint_shape(&mut self) -> bool {
        if self.shape_anchor.take().is_none() {
            return false;
        }
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_SHAPE_CANCELLED),
        );
        true
    }

    /// Estende o traço interpolando em espaço de tela e pintando cada dab.
    pub fn paint_stroke_to(&mut self, x: f32, y: f32) -> bool {
        self.paint_stroke_to_with_modifiers(x, y, false, false)
    }

    /// Estende o traço de pintura ou manipulação de decalque com suporte a modificadores.
    pub fn paint_stroke_to_with_modifiers(
        &mut self,
        x: f32,
        y: f32,
        is_shift: bool,
        is_ctrl: bool,
    ) -> bool {
        if self.active_layer_is_decal() {
            return self.decal_drag_to(x, y, is_shift, is_ctrl);
        }
        if self.paint_last.is_none() {
            return false;
        }
        let settings = self.viewport_brush_settings();
        let smoothing = self.state.session.tools.brush_style.smoothing;
        let [x, y] = self.paint_stabilizer.filter([x, y], smoothing);
        let dabs = self.paint_sampler.extend([x, y]);
        self.paint_dabs_at(&dabs, settings);
        self.paint_last = Some([x, y]);
        self.state.mark_dirty();
        true
    }

    /// Define a origem do Clone no texel sob o cursor (Ctrl+clique).
    pub(crate) fn set_clone_source_at(&mut self, x: f32, y: f32) -> bool {
        let Some((px, py)) = self.canvas_pixel_at(x, y) else {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::text_id::STATUS_CLONE_POINT_AT_THE_SURFACE_TO_SET),
            );
            return false;
        };
        self.state.session.tools.clone_source = Some([px as f32, py as f32]);
        self.state.set_status(
            self.state
                .t_id(petunia_config::text_id::STATUS_CLONE_SOURCE_SET_PAINT_TO_COPY_FROM),
        );
        self.state.mark_dirty();
        true
    }

    /// Confirma o traço de pintura ou manipulação de decalque como uma única entrada de undo.
    pub fn end_paint_stroke_at(&mut self, x: f32, y: f32) -> bool {
        if self.decal_gesture.is_some() {
            return self.decal_drag_end();
        }
        if self.shape_anchor.is_some() {
            return self.end_paint_shape_at(x, y);
        }
        if self.paint_last.is_none() {
            return false;
        }
        // O último ponto passa sem suavização: o traço alcança o cursor.
        self.paint_stabilizer.reset();
        self.paint_stroke_to(x, y);
        let [w, h] = self.viewport_size;
        let (origin, direction) = self.state.camera.ray(x / w * 2.0 - 1.0, 1.0 - y / h * 2.0);
        self.surface_paint.straight_from = self.paint_pick(origin, direction).map(|(_, hit)| hit);
        self.paint_last = None;
        self.paint_sampler.reset();
        self.paint_stabilizer.reset();
        self.state.finish_paint_stroke(false);
        true
    }

    pub fn cancel_paint_stroke(&mut self) -> bool {
        if self.cancel_decal_drag() {
            return true;
        }
        if self.cancel_paint_shape() {
            return true;
        }
        if self.paint_2d_last.take().is_some() {
            self.paint_2d_sampler.reset();
            self.state.finish_paint_stroke(true);
            self.state.mark_dirty();
            return true;
        }
        if self.paint_last.take().is_none() {
            return false;
        }
        self.paint_sampler.reset();
        self.state.finish_paint_stroke(true);
        true
    }

    /// Tique periódico para acúmulo contínuo de tinta da ferramenta Airbrush (P3D-056).
    pub fn airbrush_tick(&mut self) -> bool {
        let is_airbrush = matches!(
            self.state.session.tools.active_tool.as_str(),
            "airbrush" | "spray"
        ) || matches!(
            petunia_core::brush_type_from_kind(self.state.session.tools.paint_brush_kind),
            petunia_core::BrushType::Airbrush | petunia_core::BrushType::Spray
        );
        if !is_airbrush {
            return false;
        }
        let mut changed = false;
        if let Some([x, y]) = self.paint_last {
            let settings = self.viewport_brush_settings();
            self.paint_dabs_at(&[[x, y]], settings);
            changed = true;
        }
        if let Some((px, py)) = self.paint_2d_last {
            let settings = self.canvas_brush_settings();
            petunia_module_paint::PaintModule::canvas_brush_with_symmetry(
                &mut self.state,
                px,
                py,
                settings,
            );
            changed = true;
        }
        if changed {
            self.state.mark_dirty();
        }
        changed
    }

    /// Processa interação interativa de desenho no canvas 2D de textura (P3D-057).
    /// phase: 0 = Down, 1 = Move, 2 = Up, outros = Cancel
    pub fn paint_2d_stroke(&mut self, norm_x: f32, norm_y: f32, phase: i32) -> bool {
        if self.state.workspace == Workspace::Paint
            && self.state.modal.is_none()
            && self.surface_paint_canvas_input(norm_x, norm_y, phase)
        {
            return true;
        }
        if !norm_x.is_finite() || !norm_y.is_finite() {
            return false;
        }
        if self.paint_pip && phase == 0 {
            self.frame_camera_on_uv(norm_x, norm_y);
        }
        if self.state.workspace != Workspace::Paint
            || self.state.modal.is_some()
            || self.state.mesh_preview.is_some()
        {
            return false;
        }
        let luma = self
            .state
            .session
            .tools
            .paint_stencil
            .as_ref()
            .is_some_and(|s| s.luminance);
        if phase == 0 {
            self.sync_surface_stencil(luma);
        }
        petunia_module_paint::PaintModule::ensure_stack(&mut self.state);
        let (width, height) = match self
            .state
            .project
            .assets
            .get(self.state.project.active)
            .and_then(|a| a.texture.as_ref())
        {
            Some(t) => (t.w, t.h),
            None => (256, 256),
        };
        let px = ((norm_x * width as f32).floor() as i32).clamp(0, width as i32 - 1) as u32;
        let py = ((norm_y * height as f32).floor() as i32).clamp(0, height as i32 - 1) as u32;

        let tool = self.state.session.tools.active_tool.clone();
        if tool == "gradient" || tool == "gradient_radial" {
            match phase {
                0 => {
                    self.shape_anchor = Some((px, py));
                    self.state.set_status(self.state.t_id(
                        petunia_config::text_id::STATUS_GRADIENT_DRAG_TO_SET_DIRECTION_AND_LENGTH,
                    ));
                    return true;
                }
                1 => {
                    return true;
                }
                2 => {
                    if let Some((x0, y0)) = self.shape_anchor.take() {
                        self.commit_gradient(tool == "gradient_radial", (x0, y0), (px, py));
                        return true;
                    }
                    return false;
                }
                _ => {
                    self.shape_anchor = None;
                    return false;
                }
            }
        }

        match phase {
            0 => {
                let tool = self.state.session.tools.active_tool.clone();
                if tool == "picker" {
                    if let Some(color) = self
                        .state
                        .project
                        .assets
                        .get(self.state.project.active)
                        .and_then(|a| a.texture.as_ref())
                        .and_then(|t| t.get(px, py))
                    {
                        let c = [
                            color[0] as f32 / 255.0,
                            color[1] as f32 / 255.0,
                            color[2] as f32 / 255.0,
                        ];
                        self.state.paint_color = c;
                        self.state.session.tools.paint_color = c;
                        self.state.mark_dirty();
                        self.state.set_status(
                            self.state
                                .t_id(petunia_config::text_id::STATUS_COLOR_SAMPLED_FROM_CANVAS),
                        );
                    }
                    return true;
                }
                if tool == "fill" {
                    let scope = self.state.session.tools.fill_scope;
                    self.state.begin_paint_stroke();
                    petunia_module_paint::PaintModule::canvas_fill_scoped(
                        &mut self.state,
                        None,
                        Some((px, py)),
                        scope,
                    );
                    self.state.finish_paint_stroke(false);
                    self.state.set_status(crate::tr::fill(
                        &self
                            .state
                            .t_id(petunia_config::text_id::STATUS_FILLED_CANVAS),
                        &[("scope", format!("{scope:?}"))],
                    ));
                    return true;
                }
                if tool == "clone" && self.state.session.tools.clone_source.is_none() {
                    self.state.session.tools.clone_source = Some([px as f32, py as f32]);
                    self.state.set_status(
                        self.state.t_id(
                            petunia_config::text_id::STATUS_CLONE_SOURCE_SET_PAINT_TO_COPY_FROM,
                        ),
                    );
                    return true;
                }
                self.state.begin_paint_stroke();
                let settings = self.canvas_brush_settings();
                self.paint_2d_stabilizer.reset();
                let smoothing = self.state.session.tools.brush_style.smoothing;
                let [fx, fy] = self
                    .paint_2d_stabilizer
                    .filter([px as f32, py as f32], smoothing);
                let dabs = self
                    .paint_2d_sampler
                    .begin([fx, fy], settings.dab_step_px());
                let mut dabs = texture_points(&dabs, width, height);
                if self.state.session.tools.brush_style.pixel_perfect
                    && settings.kind == petunia_core::BrushType::Pixel
                {
                    dabs = self.pixel_perfect_samples(&dabs, phase == 0);
                }
                petunia_module_paint::PaintModule::canvas_brush_batch_with_symmetry(
                    &mut self.state,
                    &dabs,
                    settings,
                );
                self.paint_2d_last = Some((px, py));
                self.state.mark_dirty();
                true
            }
            1 => {
                if self.paint_2d_last.is_none() {
                    return false;
                }
                let settings = self.canvas_brush_settings();
                let smoothing = self.state.session.tools.brush_style.smoothing;
                let [fx, fy] = self
                    .paint_2d_stabilizer
                    .filter([px as f32, py as f32], smoothing);
                let dabs = self.paint_2d_sampler.extend([fx, fy]);
                let mut dabs = texture_points(&dabs, width, height);
                if self.state.session.tools.brush_style.pixel_perfect
                    && settings.kind == petunia_core::BrushType::Pixel
                {
                    dabs = self.pixel_perfect_samples(&dabs, phase == 0);
                }
                petunia_module_paint::PaintModule::canvas_brush_batch_with_symmetry(
                    &mut self.state,
                    &dabs,
                    settings,
                );
                self.paint_2d_last = Some((px, py));
                self.state.mark_dirty();
                true
            }
            2 => {
                if self.paint_2d_last.take().is_none() {
                    return false;
                }
                let settings = self.canvas_brush_settings();
                self.paint_2d_stabilizer.reset();
                let dabs = self.paint_2d_sampler.extend([px as f32, py as f32]);
                let mut dabs = texture_points(&dabs, width, height);
                if self.state.session.tools.brush_style.pixel_perfect
                    && settings.kind == petunia_core::BrushType::Pixel
                {
                    dabs = self.pixel_perfect_samples(&dabs, phase == 0);
                }
                petunia_module_paint::PaintModule::canvas_brush_batch_with_symmetry(
                    &mut self.state,
                    &dabs,
                    settings,
                );
                self.paint_2d_sampler.reset();
                self.surface_paint.straight_texel = Some((px, py));
                self.state.finish_paint_stroke(false);
                self.state.mark_dirty();
                true
            }
            _ => {
                if self.paint_2d_last.take().is_none() {
                    return false;
                }
                self.paint_2d_sampler.reset();
                self.state.finish_paint_stroke(true);
                self.state.mark_dirty();
                true
            }
        }
    }
}
