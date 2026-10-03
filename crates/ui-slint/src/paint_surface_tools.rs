//! Ferramentas de superfície: gramática compartilhada e operações PAINT.
use super::*;
use glam::Vec3;
use petunia_module_paint::projection::{ProjectionParams, StencilPlacement};
use petunia_project::{Canvas, SurfaceAttachment};

#[derive(Default)]
pub(crate) struct SurfacePaintDraft {
    pub target: Option<uuid::Uuid>,
    pub nodes: Vec<SurfaceAttachment>,
    pub path_mode: petunia_module_paint::PathPaintMode,
    pub placement: StencilPlacement,
    pub image: Option<std::sync::Arc<Canvas>>,
    pub anchor: Option<[f32; 2]>,
    pub straight_from: Option<Vec3>,
    pub pixel_points: Vec<(i32, i32)>,
    pub stencil_mode: bool,
    pub pixel_skip: bool,
    pub preview_cache: std::cell::RefCell<Option<(Vec<u32>, slint::Image)>>,
    pub straight_texel: Option<(u32, u32)>,
}

pub(crate) const SURFACE_ACTIONS: &[(&str, &str)] = &[
    ("draw.import_svg", "surface.import_svg"),
    ("paint.path_paint", "surface.path"),
    ("paint.path_ribbon", "surface.ribbon"),
    ("paint.path_stamps", "surface.stamps"),
    ("paint.project_decal", "surface.project"),
    ("paint.stencil", "surface.stencil"),
    ("paint.svg_resolution_512", "surface.svg_resolution_512"),
    ("paint.svg_resolution_1024", "surface.svg_resolution_1024"),
    ("paint.stencil_luma", "surface.stencil_luma"),
    ("paint.stencil_place", "surface.stencil_place"),
    ("paint.apply_surface", "surface.apply"),
    ("paint.cancel_surface", "surface.cancel"),
    ("paint.projection_mirror", "surface.mirror"),
    ("paint.projection_rotate", "surface.rotate"),
    ("paint.projection_grow", "surface.grow"),
    ("paint.projection_shrink", "surface.shrink"),
    ("paint.alpha_lock", "surface.alpha_lock"),
    ("paint.pixel_perfect", "surface.pixel_perfect"),
    ("paint.dithering", "surface.dithering"),
    ("paint.palette_lighter", "surface.palette_lighter"),
    ("paint.palette_darker", "surface.palette_darker"),
    ("paint.palette_normal", "surface.palette_normal"),
    ("paint.uv_health", "surface.uv_health"),
];
impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn surface_action_items(
        &self,
        query: &str,
    ) -> Vec<petunia_core::CommandPaletteItem> {
        let query = query.to_lowercase();
        SURFACE_ACTIONS
            .iter()
            .chain(draw_extensions::DRAW_ACTIONS.iter())
            .filter_map(|(id, key)| {
                let label = self.state.t_id(petunia_config::TextId::new(key));
                if !label.to_lowercase().contains(&query) && !id.contains(&query) {
                    return None;
                }
                let available = self.state.modal.is_none()
                    && self.state.mesh_preview.is_none()
                    && self.state.paint_stroke.is_none()
                    && self.profile_volume_mode.is_none()
                    && if id.starts_with("paint.") {
                        self.state.workspace == Workspace::Paint
                    } else {
                        self.state.workspace == Workspace::Model
                            && self.modeling_mode == ModelingMode::Draw
                    };
                Some(petunia_core::CommandPaletteItem {
                    id: (*id).into(),
                    description: label.clone(),
                    label,
                    category: petunia_core::CommandCategory::Tools,
                    shortcut: None,
                    disabled_reason: None,
                    is_available: available,
                    docs_topic: None,
                })
            })
            .collect()
    }
    pub(crate) fn execute_surface_action(&mut self, id: &str) -> bool {
        if !SURFACE_ACTIONS.iter().any(|(action, _)| *action == id) || id == "draw.import_svg" {
            return false;
        }
        if self.state.workspace != Workspace::Paint {
            return false;
        }
        if self.state.modal.is_some()
            || self.state.mesh_preview.is_some()
            || self.state.paint_stroke.is_some()
        {
            self.state.set_status(
                self.state
                    .t_id(petunia_config::TextId::new("surface.finish_operation")),
            );
            return true;
        }
        match id {
            "paint.alpha_lock" => {
                self.state.session.tools.brush_style.alpha_lock =
                    !self.state.session.tools.brush_style.alpha_lock;
            }
            "paint.svg_resolution_512" | "paint.svg_resolution_1024" => {
                let size = if id.ends_with("1024") { 1024 } else { 512 };
                self.mutate_paint_stack("SVG resolution", |stack| {
                    stack
                        .active_mut()
                        .and_then(|layer| match &mut layer.kind {
                            petunia_project::paint_layers::LayerKind::Decal(decal) => {
                                decal.rerasterize(size).ok()
                            }
                            _ => None,
                        })
                        .unwrap_or(false)
                });
            }
            "paint.pixel_perfect" => {
                self.state.session.tools.brush_style.pixel_perfect =
                    !self.state.session.tools.brush_style.pixel_perfect;
                self.apply(UiIntent::SetActiveTool("pixel".into()));
                self.state.session.tools.canvas_brush = 1;
            }
            "paint.dithering" => {
                self.state.session.tools.brush_style.dithering =
                    !self.state.session.tools.brush_style.dithering;
            }
            "paint.palette_lighter" => self.state.session.tools.brush_style.palette_step = 1,
            "paint.palette_darker" => self.state.session.tools.brush_style.palette_step = -1,
            "paint.palette_normal" => self.state.session.tools.brush_style.palette_step = 0,
            "paint.uv_health" => {
                if let Some(mesh) = self.state.project.active_mesh() {
                    let d = mesh.uv_diagnostics();
                    let message = self
                        .state
                        .t_id(petunia_config::TextId::new("surface.uv_report"))
                        .replace("{islands}", &d.island_count.to_string())
                        .replace("{overlaps}", &d.overlapping_islands.to_string())
                        .replace("{zero}", &d.zero_area_faces.to_string())
                        .replace("{outside}", &d.out_of_range_corners.to_string());
                    self.state.set_status(message);
                }
                self.state.session.show_uv_checker = true;
            }
            "paint.path_paint" | "paint.path_ribbon" | "paint.path_stamps" => {
                let decal = self.active_decal();
                if id == "paint.path_stamps" && decal.is_none() {
                    self.state.set_status(
                        self.state
                            .t_id(petunia_config::TextId::new("surface.choose_decal")),
                    );
                    return true;
                }
                self.cancel_surface_paint();
                self.surface_paint.target = self.state.project.active().map(|a| a.id);
                self.surface_paint.path_mode = match id {
                    "paint.path_ribbon" => petunia_module_paint::PathPaintMode::Ribbon,
                    "paint.path_stamps" => petunia_module_paint::PathPaintMode::RepeatedStamp(
                        decal.expect("checked decal").image,
                    ),
                    _ => petunia_module_paint::PathPaintMode::Stroke,
                };
                self.apply(UiIntent::SetActiveTool("path_paint".into()));
            }
            "paint.stencil_place" => {
                if self.surface_paint.stencil_mode {
                    self.apply(UiIntent::SetActiveTool("projection".into()));
                }
            }
            "paint.project_decal" => {
                self.activate_surface_projection();
            }
            "paint.stencil" => {
                if self.activate_surface_projection() {
                    self.surface_paint.stencil_mode = true;
                    let (w, h) = self.paint_canvas_dimensions().unwrap_or((256, 256));
                    let name = self
                        .state
                        .t_id(petunia_config::TextId::new("surface.stencil_name"));
                    self.mutate_paint_stack("stencil paint layer", |stack| {
                        stack.add_layer(petunia_project::PaintLayer::new(&name, w, h, [0; 4]));
                        true
                    });
                    self.apply(UiIntent::SetActiveTool("brush".into()));
                    self.sync_surface_stencil(false);
                }
            }
            "paint.stencil_luma" => {
                let luma = self
                    .state
                    .session
                    .tools
                    .paint_stencil
                    .as_ref()
                    .is_none_or(|s| !s.luminance);
                self.sync_surface_stencil(luma);
            }
            "paint.apply_surface" => {
                self.apply_surface_paint();
            }
            "paint.cancel_surface" => {
                self.cancel_surface_paint();
            }
            "paint.projection_mirror" => {
                self.surface_paint.placement.mirror_x = !self.surface_paint.placement.mirror_x;
            }
            "paint.projection_rotate" => {
                self.surface_paint.placement = self
                    .surface_paint
                    .placement
                    .rotated(std::f32::consts::FRAC_PI_4);
            }
            "paint.projection_grow" => {
                self.surface_paint.placement =
                    self.surface_paint.placement.scaled_about_center(1.25);
            }
            "paint.projection_shrink" => {
                self.surface_paint.placement =
                    self.surface_paint.placement.scaled_about_center(0.8);
            }
            _ => {}
        }
        if self.surface_paint.stencil_mode {
            let luma = self
                .state
                .session
                .tools
                .paint_stencil
                .as_ref()
                .is_some_and(|s| s.luminance);
            self.sync_surface_stencil(luma);
        }
        self.state.mark_dirty();
        true
    }

    /// Drafts belong to the object where the gesture started.
    pub(crate) fn validate_surface_target(&mut self) {
        let current = self.state.project.active().map(|a| a.id);
        if self.surface_paint.target != current {
            self.cancel_surface_paint();
            self.surface_paint.target = current;
        }
    }
    pub(crate) fn sync_surface_stencil(&mut self, luminance: bool) {
        if !self.surface_paint.stencil_mode {
            return;
        }
        let Some(image) = self.surface_paint.image.clone() else {
            return;
        };
        let Some(target) = self
            .surface_paint
            .target
            .filter(|id| self.state.project.active().is_some_and(|a| a.id == *id))
        else {
            self.state.session.tools.paint_stencil = None;
            return;
        };
        let [w, h] = self.viewport_size;
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        let p = self.surface_paint.placement;
        self.state.session.tools.paint_stencil = Some(petunia_core::brush::SurfaceStencil {
            target,
            image,
            world_to_clip: self.state.camera.view_proj(),
            center: [p.center_px[0] / w, p.center_px[1] / h],
            size: [p.size_px[0] / w, p.size_px[1] / h],
            rotation: p.rotation_rad,
            mirror: p.mirror_x,
            luminance,
        });
    }

    pub(crate) fn surface_paint_click(&mut self, at: [f32; 2]) -> bool {
        if self.state.session.tools.active_tool == "projection" {
            self.surface_paint.placement.center_px = at;
            self.state.mark_dirty();
            return true;
        }
        if self.surface_paint.nodes.len() >= 8192 {
            return false;
        }
        let [w, h] = self.viewport_size;
        if w <= 1.0 || h <= 1.0 {
            return false;
        }
        let (origin, direction) = self
            .state
            .camera
            .ray(at[0] / w * 2.0 - 1.0, 1.0 - at[1] / h * 2.0);
        if self.paint_pick(origin, direction).is_none() {
            return false;
        }
        let Some(target) = self.state.project.active().map(|a| a.id) else {
            return false;
        };
        let Ok(Some(hit)) = petunia_project::project_ray_to_surface_target(
            &self.state.project.project,
            target,
            origin.to_array(),
            direction.to_array(),
            1.0e8,
        ) else {
            return false;
        };
        if self
            .surface_paint
            .nodes
            .first()
            .is_some_and(|n| n.target != target)
        {
            return false;
        }
        if self.surface_paint.nodes.last().is_none_or(|n| {
            Vec3::from_array(n.last_world_position)
                .distance(Vec3::from_array(hit.attachment.last_world_position))
                > 1.0e-5
        }) {
            self.surface_paint.nodes.push(hit.attachment);
        }
        self.state.mark_dirty();
        true
    }
    pub(crate) fn begin_surface_paint(&mut self, at: [f32; 2]) -> bool {
        self.validate_surface_target();
        self.surface_paint.anchor = Some(at);
        if self.state.session.tools.active_tool == "path_paint" {
            self.surface_paint_click(at);
        }
        self.tool_gesture = Some(ToolGesture::SurfacePaint);
        true
    }
    pub(crate) fn update_surface_paint(&mut self, at: [f32; 2]) -> bool {
        if self.state.session.tools.active_tool == "path_paint" {
            return self.surface_paint_click(at);
        }
        let Some(from) = self.surface_paint.anchor.replace(at) else {
            return false;
        };
        if self.tool_press_alternate {
            self.surface_paint.placement = self
                .surface_paint
                .placement
                .rotated((at[0] - from[0]) * 0.01);
        } else if self.tool_press_extend {
            self.surface_paint.placement = self
                .surface_paint
                .placement
                .scaled_about_center(((at[0] - from[0]) * 0.01).exp());
        } else {
            self.surface_paint.placement = self
                .surface_paint
                .placement
                .translated([at[0] - from[0], at[1] - from[1]]);
        }
        self.state.mark_dirty();
        true
    }
    pub(crate) fn cancel_surface_paint(&mut self) -> bool {
        let had = !self.surface_paint.nodes.is_empty() || self.surface_paint.image.is_some();
        self.surface_paint = SurfacePaintDraft::default();
        self.state.session.tools.paint_stencil = None;
        self.tool_gesture = None;
        self.tool_session.reset();
        self.state.mark_dirty();
        had
    }
    pub(crate) fn activate_surface_projection(&mut self) -> bool {
        if self.state.project.active().is_none_or(|a| a.locked)
            || self.viewport_size.iter().any(|v| *v <= 1.0)
        {
            return false;
        }
        let Some(decal) = self.active_decal() else {
            return false;
        };
        self.cancel_surface_paint();
        self.surface_paint.target = self.state.project.active().map(|a| a.id);
        let [w, h] = self.viewport_size;
        self.surface_paint.image = Some(std::sync::Arc::new(decal.image.clone()));
        self.surface_paint.placement = StencilPlacement {
            center_px: [w * 0.5, h * 0.5],
            size_px: [
                w * 0.3,
                w * 0.3 * decal.image.h as f32 / decal.image.w.max(1) as f32,
            ],
            ..Default::default()
        };
        self.apply(UiIntent::SetActiveTool("projection".into()));
        true
    }
    pub(crate) fn apply_surface_paint(&mut self) -> bool {
        if self.state.workspace != Workspace::Paint
            || self.state.modal.is_some()
            || self.state.mesh_preview.is_some()
            || self.state.paint_stroke.is_some()
        {
            return false;
        }
        if self
            .state
            .project
            .active()
            .is_none_or(|a| a.locked || Some(a.id) != self.surface_paint.target)
        {
            return false;
        }
        if self.surface_paint.stencil_mode {
            let luma = self
                .state
                .session
                .tools
                .paint_stencil
                .as_ref()
                .is_some_and(|s| s.luminance);
            self.sync_surface_stencil(luma);
            self.apply(UiIntent::SetActiveTool("brush".into()));
            self.state.mark_dirty();
            return true;
        }
        if self.state.session.tools.active_tool == "path_paint" {
            if self.surface_paint.nodes.len() < 2 {
                return false;
            }
            if self.active_layer_is_decal()
                && !matches!(
                    self.surface_paint.path_mode,
                    petunia_module_paint::PathPaintMode::RepeatedStamp(_)
                )
            {
                return false;
            }
            let command = petunia_module_paint::PaintPathCmd {
                anchors: self.surface_paint.nodes.clone(),
                mode: self.surface_paint.path_mode.clone(),
                settings: self.viewport_brush_settings(),
                name: self
                    .state
                    .t_id(petunia_config::TextId::new("surface.path_name")),
            };
            if let Err(error) = self.state.dispatch(&command) {
                self.state.set_status(error.to_string());
                return false;
            }
            self.surface_paint.nodes.clear();
        } else {
            let Some(image) = self.surface_paint.image.clone() else {
                return false;
            };
            if self.state.project.active_mesh().is_none() {
                return false;
            }
            let [w, h] = self.viewport_size;
            let viewport = [w.max(1.0).round() as u32, h.max(1.0).round() as u32];
            let matrix = self.state.camera.view_proj();
            let triangles: Vec<_> = self
                .state
                .project
                .assets
                .iter()
                .filter(|a| a.visible)
                .flat_map(|a| {
                    petunia_module_paint::projection::mesh_world_triangles(&a.evaluated_mesh())
                        .collect::<Vec<_>>()
                })
                .collect();

            let center = self.surface_paint.placement.center_px;
            let (ray_origin, ray_direction) = self
                .state
                .camera
                .ray(center[0] / w * 2.0 - 1.0, 1.0 - center[1] / h * 2.0);
            let first_face = self.paint_pick(ray_origin, ray_direction).map(|h| h.0);
            let params = ProjectionParams {
                view_proj: matrix,
                viewport,
                placement: self.surface_paint.placement,
                opacity: self.state.session.tools.paint_strength,
                back_face_cull: true,
                depth_bias: 1.0e-4,
            };
            let command = petunia_module_paint::ProjectionPaintCmd {
                image: image.as_ref().clone(),
                params,
                scene_triangles: triangles,
                first_face,
                name: self
                    .state
                    .t_id(petunia_config::TextId::new("surface.projection_name")),
            };
            if let Err(error) = self.state.dispatch(&command) {
                self.state.set_status(error.to_string());
                return false;
            }
            self.surface_paint.image = None;
        }
        self.state.mark_dirty();
        true
    }
    pub(crate) fn surface_path_uv_preview(&self) -> String {
        if self.state.session.tools.active_tool != "path_paint" {
            return String::new();
        }
        let Ok(frames) = petunia_project::evaluate_surface_attachments(
            &self.state.project.project,
            &self.surface_paint.nodes,
        ) else {
            return String::new();
        };
        let mut commands = String::new();
        let mut previous = None;
        for (frame, anchor) in frames.iter().zip(&self.surface_paint.nodes) {
            if let Some(uv) = frame.uv {
                let face = anchor.triangle.face_index;
                commands.push_str(&format!(
                    "{} {:.2} {:.2} ",
                    if previous == Some(face) { "L" } else { "M" },
                    uv[0] * 256.0,
                    (1.0 - uv[1]) * 256.0
                ));
                previous = Some(face);
            } else {
                previous = None;
            }
        }
        commands
    }

    pub(crate) fn surface_paint_preview(&self) -> String {
        let mut out = String::new();
        if self.state.session.tools.active_tool == "path_paint" {
            let points: Vec<_> = petunia_project::evaluate_surface_attachments(
                &self.state.project.project,
                &self.surface_paint.nodes,
            )
            .unwrap_or_default()
            .into_iter()
            .map(|f| Vec3::from_array(f.position))
            .collect();
            projection::write_clipped_path(
                &self.state.camera,
                self.viewport_size,
                points.iter().copied(),
                false,
                &mut out,
            );
        } else if self.surface_paint.image.is_some() {
            for (i, [x, y]) in self
                .surface_paint
                .placement
                .outline_px()
                .into_iter()
                .enumerate()
            {
                out.push_str(&format!(
                    "{} {x:.2} {y:.2} ",
                    if i == 0 { "M" } else { "L" }
                ));
            }
            out.push('Z');
        }
        out
    }
    pub(crate) fn stencil_preview_image(&self) -> Option<slint::Image> {
        let image = self.surface_paint.image.as_ref()?;
        let [width, height] = self.viewport_size;
        if width < 1.0 || height < 1.0 {
            return None;
        }
        let key = vec![
            width.to_bits(),
            height.to_bits(),
            self.surface_paint.placement.center_px[0].to_bits(),
            self.surface_paint.placement.center_px[1].to_bits(),
            self.surface_paint.placement.size_px[0].to_bits(),
            self.surface_paint.placement.size_px[1].to_bits(),
            self.surface_paint.placement.rotation_rad.to_bits(),
            self.surface_paint.placement.mirror_x as u32,
            self.state.session.tools.paint_strength.to_bits(),
        ];
        if let Some((previous, image)) = self.surface_paint.preview_cache.borrow().as_ref() {
            if *previous == key {
                return Some(image.clone());
            }
        }
        let scale = (512.0 / width.max(height)).min(1.0);
        let (w, h) = (
            (width * scale).round().max(1.0) as u32,
            (height * scale).round().max(1.0) as u32,
        );
        let mut buffer = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(w, h);
        let pixels = buffer.make_mut_bytes();
        for y in 0..h {
            for x in 0..w {
                let Some(uv) = self
                    .surface_paint
                    .placement
                    .uv_at([(x as f32 + 0.5) / scale, (y as f32 + 0.5) / scale])
                else {
                    continue;
                };
                let color = petunia_module_paint::projection::sample_bilinear(image, uv);
                let index = ((y * w + x) * 4) as usize;
                for c in 0..3 {
                    pixels[index + c] = (color[c] * 255.0).round().clamp(0.0, 255.0) as u8;
                }
                pixels[index + 3] = (color[3] * self.state.session.tools.paint_strength * 160.0)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
        let image = slint::Image::from_rgba8(buffer);
        *self.surface_paint.preview_cache.borrow_mut() = Some((key, image.clone()));
        Some(image)
    }
    pub(crate) fn pixel_perfect_samples(
        &mut self,
        points: &[(u32, u32)],
        begin: bool,
    ) -> Vec<(u32, u32)> {
        if begin {
            self.surface_paint.pixel_points.clear();
            self.surface_paint.pixel_skip = false;
        }
        let mut output = Vec::new();
        let mut removed = Vec::new();
        for &(x, y) in points {
            let point = (x as i32, y as i32);
            let tail = &mut self.surface_paint.pixel_points;
            if tail.last() == Some(&point) {
                continue;
            }
            let remove_corner = tail.len() == 2
                && !self.surface_paint.pixel_skip
                && petunia_module_paint::pixel_perfect(&[tail[0], tail[1], point]).len() == 2;
            if remove_corner {
                let corner = tail[1];
                removed.push(corner);
                output.retain(|p| *p != (corner.0 as u32, corner.1 as u32));
            }
            self.surface_paint.pixel_skip = remove_corner;
            tail.push(point);
            if tail.len() > 2 {
                tail.remove(0);
            }
            output.push((x, y));
        }
        let mut dirty_tiles = Vec::new();
        let sx = self.state.session.tools.paint_symmetry_x;
        let sy = self.state.session.tools.paint_symmetry_y;
        if let Some(buffer) = self.state.session.tools.paint_buffer.as_mut() {
            if let Some(canvas) = self
                .state
                .project
                .active_mut()
                .and_then(|a| a.paint_stack.as_mut())
                .and_then(|s| s.active_mut())
                .and_then(|l| l.canvas_mut())
            {
                for (x, y) in removed {
                    if x < 0 || y < 0 || x >= canvas.w as i32 || y >= canvas.h as i32 {
                        continue;
                    }
                    let (x, y) = (x as u32, y as u32);
                    let mx = canvas.w - 1 - x;
                    let my = canvas.h - 1 - y;
                    for (px, py, enabled) in
                        [(x, y, true), (mx, y, sx), (x, my, sy), (mx, my, sx && sy)]
                    {
                        if !enabled {
                            continue;
                        }
                        let index = (py * canvas.w + px) as usize;
                        canvas.pixels[index * 4..index * 4 + 4]
                            .copy_from_slice(&buffer.base[index * 4..index * 4 + 4]);
                        buffer.coverage[index] = 0.0;
                        let tile = petunia_project::paint_layers::TILE_SIZE;
                        dirty_tiles.push((py / tile) * canvas.w.div_ceil(tile) + (px / tile));
                    }
                }
            }
        }
        if !dirty_tiles.is_empty() {
            dirty_tiles.sort_unstable();
            dirty_tiles.dedup();
            petunia_module_paint::PaintModule::composite_active_tiles(
                &mut self.state,
                &dirty_tiles,
            );
        }
        output
    }

    /// UV clicks can author the same anchored path without painting freehand.
    pub(crate) fn surface_paint_canvas_input(&mut self, x: f32, y: f32, phase: i32) -> bool {
        let tool = self.state.session.tools.active_tool.as_str();
        if tool == "projection" {
            return true;
        }
        if tool != "path_paint" {
            return false;
        }
        if !matches!(phase, 0 | 1) {
            if phase != 2 {
                self.cancel_surface_paint();
            }
            return true;
        }
        if !x.is_finite()
            || !y.is_finite()
            || !(0.0..=1.0).contains(&x)
            || !(0.0..=1.0).contains(&y)
            || self.surface_paint.nodes.len() >= 8192
        {
            return true;
        }
        self.validate_surface_target();
        let Some(asset) = self.state.project.active().filter(|a| !a.locked) else {
            return true;
        };
        let Some((point, normal)) = asset.mesh.uv_to_world([x, 1.0 - y]) else {
            return true;
        };
        let Ok(Some(hit)) = petunia_project::project_ray_to_surface_target(
            &self.state.project.project,
            asset.id,
            (point + normal * 0.001).to_array(),
            (-normal).to_array(),
            0.002,
        ) else {
            return true;
        };
        if self
            .surface_paint
            .nodes
            .last()
            .is_none_or(|n| Vec3::from_array(n.last_world_position).distance(point) > 1e-5)
        {
            self.surface_paint.nodes.push(hit.attachment);
            self.state.mark_dirty();
        }
        true
    }

    pub(crate) fn paint_2d_line(&mut self, x: f32, y: f32, phase: i32, shift: bool) -> bool {
        if phase == 0 {
            self.validate_surface_target();
        }
        if phase != 0 || !shift || self.is_shape_tool() || self.active_layer_is_decal() {
            return false;
        }
        let tool = self.state.session.tools.active_tool.as_str();
        if matches!(
            tool,
            "select"
                | "picker"
                | "fill"
                | "gradient"
                | "gradient_radial"
                | "path_paint"
                | "projection"
        ) {
            return false;
        }
        let Some(from) = self.surface_paint.straight_texel else {
            return false;
        };
        let Some((w, h)) = self.paint_canvas_dimensions() else {
            return false;
        };
        let to = (
            (x * w as f32).floor().clamp(0.0, (w - 1) as f32) as u32,
            (y * h as f32).floor().clamp(0.0, (h - 1) as f32) as u32,
        );
        let points: Vec<_> = petunia_module_paint::line_pixels(
            (from.0 as i32, from.1 as i32),
            (to.0 as i32, to.1 as i32),
        )
        .into_iter()
        .map(|(x, y)| (x as u32, y as u32))
        .collect();
        self.state.begin_paint_stroke();
        let settings = self.state.brush_settings();
        petunia_module_paint::PaintModule::canvas_brush_batch_with_symmetry(
            &mut self.state,
            &points,
            settings,
        );
        self.state.finish_paint_stroke(false);
        self.surface_paint.straight_texel = Some(to);
        true
    }
    pub fn import_svg_profiles(&mut self, path: &std::path::Path) -> bool {
        if self.state.workspace != Workspace::Model || self.modeling_mode != ModelingMode::Draw {
            return false;
        }
        let svg = match files::load_svg(path) {
            Ok(svg) => svg,
            Err(error) => {
                self.state.set_status(error);
                return false;
            }
        };
        let splines = match petunia_project::svg_to_splines(&svg, 2.0) {
            Ok(s) => s,
            Err(error) => {
                self.state.set_status(error.to_string());
                return false;
            }
        };
        if self.state.modal.is_some() || self.profile_volume_mode.is_some() {
            return false;
        }
        let p = &self.state.profile;
        let workplane = petunia_project::ProfileWorkplane {
            origin: p.origin.map(f64::from),
            right: p.right.map(f64::from),
            up: p.up.map(f64::from),
            normal: p.normal.map(f64::from),
        };
        let workplane = workplane.try_normalized().unwrap_or_default();
        let profiles = splines
            .into_iter()
            .map(|spline| {
                let profile = petunia_project::ProfileResource::new(
                    spline.name.clone(),
                    spline.id,
                    workplane,
                );
                (spline, profile)
            })
            .collect();
        if let Err(error) = self.state.dispatch(&petunia_core::DrawBatchCmd {
            update: None,
            profiles,
        }) {
            self.state.set_status(error.to_string());
            return false;
        }
        *self.region_planes_cache.borrow_mut() = None;
        *self.shape_planes_cache.borrow_mut() = None;
        self.state.mark_dirty();
        true
    }
}
