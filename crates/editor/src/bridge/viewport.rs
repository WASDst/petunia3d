//! Viewport controller methods for Petunia editor bridge.

use petunia_core::Workspace;

use crate::{PetuniaViewport, SlintUiBridge, ViewportGesture, ViewportRenderState};

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Navegação da câmera. Nunca é suspensa por ferramenta; a roda sempre faz zoom.
    pub fn apply_viewport_gesture(&mut self, gesture: ViewportGesture) {
        match gesture {
            // Na navegação planar do DRAW o orbit livre fica desligado; o
            // mesmo arrasto desloca a vista para não deixar o gesto morto.
            ViewportGesture::Orbit { dx, dy } if self.planar_navigation_active() => {
                self.state.session.camera.pan(dx, dy);
            }
            ViewportGesture::Orbit { dx, dy } => {
                self.orbit_viewport(dx, dy);
                return;
            }
            ViewportGesture::Pan { dx, dy } => {
                self.state.session.camera.pan(dx, dy);
            }
            ViewportGesture::Zoom { delta } => {
                self.state.session.camera.zoom(delta);
            }
        }
        self.state.mark_dirty();
    }

    pub fn resize_viewport(&mut self, width: u32, height: u32) {
        self.resize_viewport_scaled(width, height, self.pixel_ratio);
    }

    /// Redimensiona a viewport em px lógicos e adapta o backend quando ele usa
    /// pixels físicos. Picking e câmera continuam em coordenadas lógicas.
    pub fn resize_viewport_scaled(&mut self, width: u32, height: u32, pixel_ratio: f32) {
        let width = width.max(1);
        let height = height.max(1);
        let ratio = if pixel_ratio.is_finite() && pixel_ratio > 0.0 {
            pixel_ratio.clamp(0.5, 4.0)
        } else {
            1.0
        };
        self.pixel_ratio = ratio;
        if self.viewport.uses_physical_pixels() {
            self.viewport.set_pixel_ratio(ratio);
            self.viewport.resize(
                (width as f32 * ratio).round().max(1.0) as u32,
                (height as f32 * ratio).round().max(1.0) as u32,
            );
        } else {
            self.viewport.resize(width, height);
        }
        self.viewport_size = [width as f32, height as f32];
        // O núcleo converte pixels de pincel em mundo por esta altura; sem
        // ela caía no padrão de 1080 px (resto do shell egui).
        self.state.session.viewport_rect = Some(petunia_core::LogicalRect {
            min: [0.0, 0.0],
            max: [width as f32, height as f32],
        });
        self.state.session.camera.aspect = width as f32 / height as f32;
        self.state.mark_dirty();
    }

    /// Orbita a câmera usando Cursor 3D ou seleção como pivô quando aplicável.
    pub fn orbit_viewport(&mut self, dx: f32, dy: f32) -> bool {
        if !dx.is_finite() || !dy.is_finite() {
            return false;
        }
        if self.state.session.tools.active_tool == "cursor"
            || self.state.session.tools.active_tool == "cursor_3d"
            || self.state.session.pivot_point == petunia_core::PivotPoint::Cursor3D
        {
            self.state.session.camera.target = glam::Vec3::from(self.state.session.cursor_3d);
        } else if let Some(center) = self.selection_pivot() {
            self.state.session.camera.target = center;
        }
        self.state.session.camera.orbit(dx, dy);
        self.state.mark_dirty();
        true
    }

    /// Opções de render compartilhadas pela vista principal e pela secundária.
    pub(crate) fn viewport_render_state(&self) -> ViewportRenderState {
        ViewportRenderState {
            shading: self.state.session.shading,
            xray: self.state.session.show_xray,
            show_triangulation: self.state.session.show_triangulation,
            // Na PAINT a textura que se pinta aparece sempre; nos outros
            // workspaces ela segue opcional (toggle ou Material/Rendered).
            // Sem isto o traço era gravado e o viewport seguia liso (Solid).
            textured: self.state.session.textured || self.state.workspace == Workspace::Paint,
            show_wireframe_overlay: self.state.session.show_wireframe_overlay,
            show_face_orientation: self.state.session.show_face_orientation,
            show_uv_checker: self.state.session.show_uv_checker,
            selection_domain: self.state.selection_domain(),
            xray_opacity: self.state.session.xray_opacity,
            selection_rgb: self.state.ui.selection_rgb,
            selection_thickness: self.state.ui.selection_thickness,
            show_grid: self.state.session.show_grid,
            hover: self.state.session.tools.hover,
            boolean_operand: self.state.session.tools.boolean_operand,
            studio_light_follows_camera: self.preferences.studio_light_follows_camera,
            matcap: self.preferences.viewport_matcap,
            ambient_occlusion: self.preferences.viewport_ambient_occlusion,
            edge_mode: self.edge_mode(),
            workplane: self.workplane_overlay(),
        }
    }

    /// Renderiza um frame da viewport a partir do estado projetado do editor.
    pub fn render_viewport(&mut self) {
        puffin::profile_function!();
        let render_state = self.viewport_render_state();
        self.viewport
            .queue_texture_updates(self.state.render.take_texture_updates());
        // A pose do Animate continua sendo avaliada (cache); a prévia de
        // ferramenta, quando existe, a substitui só no desenho.
        let pose = self.animate_pose_override();
        self.viewport
            .set_pose_override(self.draw_override().or(pose));
        let (outlined, active) = self.outlined_objects();
        self.viewport.set_outlined_objects(&outlined, active);
        self.viewport
            .set_rigid_previews(self.state.rigid_preview_transforms());
        if self.viewport.draws_gizmo() {
            let shapes = self.gizmo_overlay_shapes();
            self.viewport.set_screen_overlay(shapes);
        }
        self.viewport.render_frame(
            &self.state.project,
            &self.state.project.refs,
            &self.state.session.camera,
            render_state,
        );
    }

    /// Limpa a preselection quando o ponteiro sai da viewport.
    pub fn clear_hover(&mut self) -> bool {
        let had_preview =
            self.profile_hover_snap.take().is_some() | self.region_hover.take().is_some();
        if !self.state.session.tools.hover.is_some() {
            return had_preview;
        }
        self.state.session.tools.hover = petunia_core::HoverTarget::None;
        true
    }

    /// Atualiza o handle do gizmo sob o cursor.
    pub fn hover_gizmo(&mut self, x: f32, y: f32) -> bool {
        if self.gizmo_drag.is_some() {
            return false;
        }
        let next = self.gizmo_handle_at(x, y);
        if next == self.gizmo_hover {
            return false;
        }
        self.gizmo_hover = next;
        true
    }

    /// Ajuste contextual da roda com o modificador configurado em
    /// `pointer.adjust`. O nome legado do método é preservado durante a migração.
    pub fn viewport_ctrl_scroll(&mut self, delta: f32) -> bool {
        if !delta.is_finite() {
            return false;
        }
        if self.state.session.tools.active_tool == "loop_cut" || self.loop_cut.is_some() {
            self.scroll_loop_cut_count(delta);
        } else if self.state.session.tools.modal.is_some()
            && self.state.session.proportional_editing
        {
            let step = if delta > 0.0 { 0.25 } else { -0.25 };
            self.adjust_proportional_radius(step);
            if let Some(drag) = self.drag {
                self.update_viewport_transform_modified(
                    drag.last_pointer[0],
                    drag.last_pointer[1],
                    false,
                    false,
                );
            }
        } else {
            self.state.session.camera.zoom(delta);
        }
        self.state.mark_dirty();
        true
    }
}
