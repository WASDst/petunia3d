//! Boundary Rust da viewport Slint.
//!
//! Esta camada registra callbacks semânticos da viewport e coordena bridge,
//! render e sincronização visual. Não contém picking ou regra geométrica.

use std::rc::Rc;
use std::sync::{Arc, Mutex};

use slint::ComponentHandle;
use petunia_config::keybinds::Mods2;
use petunia_core::{SelectionDomain, Workspace};

use crate::{
    callbacks::{sync_viewport_overlays, sync_window_properties},
    projection::parse_lasso_path,
    refresh::{self, RefreshThrottle},
    PetuniaSlintShell, PetuniaViewport, SlintUiBridge, TransformKind, UiIntent, ViewportGesture, ViewportRenderState,
};

/// Conecta navegação de câmera e resize da viewport.
///
/// Orbit/Pan/Zoom convergem para `UiIntent::ViewportGesture`. Resize permanece
/// uma boundary de apresentação/render e não é um command autoral.
pub(crate) fn connect_navigation_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let pointer_mod_bridge = Arc::clone(&bridge);
    window.on_pointer_modifier_active(move |action, shift, ctrl, alt| {
        let Ok(bridge) = pointer_mod_bridge.lock() else {
            return false;
        };
        bridge
            .state
            .ui
            .keybinds
            .pointer_modifier(action.as_str())
            .held_in(Mods2 { ctrl, shift, alt })
    });

    let orbit_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_orbit(move |dx, dy| {
        if let Ok(mut bridge) = orbit_bridge.lock() {
            bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Orbit { dx, dy }));
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });

    let pan_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_pan(move |dx, dy| {
        if let Ok(mut bridge) = pan_bridge.lock() {
            bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Pan { dx, dy }));
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });

    let zoom_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_zoom(move |delta| {
        if let Ok(mut bridge) = zoom_bridge.lock() {
            bridge.apply(UiIntent::ViewportGesture(ViewportGesture::Zoom { delta }));
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });

    let resize_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_resized(move |width, height, physical_width, _physical_height| {
        // A razão vem do Slint (px físicos / px lógicos), acompanhando HiDPI
        // e troca de monitor sem vazar physical pixels para ferramentas.
        let ratio = if width > 0.5 {
            physical_width / width
        } else {
            1.0
        };
        let width = width.round().max(1.0) as u32;
        let height = height.round().max(1.0) as u32;

        if let Ok(mut bridge) = resize_bridge.lock() {
            bridge.resize_viewport_scaled(width, height, ratio);
            let new_frame = bridge.render_viewport();
            if let (Some(window), Some(frame)) = (window_weak.upgrade(), new_frame) {
                sync_viewport_overlays(&window, &bridge);
                window.set_viewport_image(frame);
            }
        }
    });
}


/// Conecta seleção, Cursor 3D e ajuste contextual da roda.
///
/// A camada apenas encaminha intents/gestos semânticos e sincroniza a janela.
/// Picking e mutações continuam em `SlintUiBridge`/core.
pub(crate) fn connect_selection_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
) {
    let box_bridge = Arc::clone(&bridge);
    let box_window = window.as_weak();
    window.on_viewport_box_select(move |x0, y0, x1, y1, add, subtract| {
        if let Ok(mut bridge) = box_bridge.lock() {
            bridge.state.select_viewport_box(
                [x0 * 2.0 - 1.0, 1.0 - y0 * 2.0],
                [x1 * 2.0 - 1.0, 1.0 - y1 * 2.0],
                add,
                subtract,
            );

            let summary = match bridge.state.selection_domain() {
                SelectionDomain::Object => {
                    let total = bridge.state.session.selection.assets.len();
                    if total == 0 {
                        "Box select: nothing in the region".to_string()
                    } else {
                        format!("Box select: {total} object(s)")
                    }
                }
                SelectionDomain::Vertex | SelectionDomain::Edge | SelectionDomain::Face => {
                    match bridge.state.project.active_mesh() {
                        Some(mesh) => {
                            let points = mesh.verts.iter().filter(|v| v.selected).count();
                            let faces = mesh.faces.iter().filter(|f| f.selected).count();
                            let edges = mesh.selected_edges.len();
                            match bridge.state.selection_domain() {
                                SelectionDomain::Vertex => {
                                    if points == 0 {
                                        "Box select: nothing in the region".to_string()
                                    } else {
                                        format!("Box select: {points} point(s)")
                                    }
                                }
                                SelectionDomain::Edge => {
                                    if edges == 0 {
                                        "Box select: nothing in the region".to_string()
                                    } else {
                                        format!("Box select: {edges} edge(s)")
                                    }
                                }
                                _ => {
                                    if faces == 0 {
                                        "Box select: nothing in the region".to_string()
                                    } else {
                                        format!("Box select: {faces} face(s)")
                                    }
                                }
                            }
                        }
                        None => "Box select: no active object".to_string(),
                    }
                }
            };
            bridge.state.set_status(summary);

            if let Some(window) = box_window.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let lasso_bridge = Arc::clone(&bridge);
    let lasso_window = window.as_weak();
    window.on_viewport_lasso_select(move |path, add, subtract| {
        let Some(polygon) = parse_lasso_path(path.as_str()) else {
            return;
        };
        if let Ok(mut bridge) = lasso_bridge.lock() {
            bridge.state.select_viewport_lasso(&polygon, add, subtract);
            let message = bridge
                .state
                .t_id(petunia_config::text_id::STATUS_LASSO_SELECTION_UPDATED);
            bridge.state.set_status(message);
            if let Some(window) = lasso_window.upgrade() {
                sync_window_properties(&window, &bridge.view_model());
                if let Some(frame) = bridge.render_viewport() {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let place_cursor_bridge = Arc::clone(&bridge);
    let place_cursor_window = window.as_weak();
    window.on_viewport_place_cursor(move |norm_x, norm_y| {
        if let Ok(mut bridge) = place_cursor_bridge.lock()
            && bridge.place_cursor_3d(norm_x, norm_y)
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = place_cursor_window.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let viewport_select_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_select(move |x, y, extend, loop_select| {
        if let Ok(mut bridge) = viewport_select_bridge.lock() {
            bridge.select_viewport_ext(x, y, extend, loop_select);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let ctrl_scroll_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_ctrl_scroll(move |delta| {
        if let Ok(mut bridge) = ctrl_scroll_bridge.lock()
            && bridge.viewport_ctrl_scroll(delta)
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });


    let viewport_context_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_context_requested(move |x, y| {
        if let Ok(mut bridge) = viewport_context_bridge.lock() {
            bridge.viewport_context_triage(x, y);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });
}



/// Conecta hover de componente e gizmo.
///
/// Eventos de alta frequência preservam o throttling já usado pelo shell:
/// viewport/overlays atualizam imediatamente e a sincronização completa é limitada.
pub(crate) fn connect_hover_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
    throttle: Rc<RefreshThrottle>,
) {
    let component_hover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let hover_throttle = Rc::clone(&throttle);
    window.on_viewport_hover(move |x, y| {
        if let Ok(mut bridge) = component_hover_bridge.lock() {
            if bridge.state.workspace == Workspace::Paint
                && let Some(window) = window_weak.upgrade()
            {
                let [width, height] = bridge.viewport_size;
                let (squash, tilt) = bridge
                    .brush_footprint_at(x * width, y * height)
                    .unwrap_or((1.0, 0.0));
                window.set_brush_cursor_squash(squash);
                window.set_brush_cursor_tilt(tilt);
            }

            let changed = if bridge.state.session.tools.active_tool == "loop_cut" {
                let viewport_size = bridge.viewport_size;
                bridge.update_loop_cut_hover(x * viewport_size[0], y * viewport_size[1])
            } else {
                crate::perf::measure("hover_component", || bridge.hover_component(x, y))
            };

            if !changed {
                return;
            }

            if let Some(window) = window_weak.upgrade() {
                refresh::refresh_interactive(
                    &window,
                    &component_hover_bridge,
                    &mut *bridge,
                    &hover_throttle,
                );
            }
        }
    });

    let hover_clear_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_hover_clear(move || {
        if let Some(window) = window_weak.upgrade() {
            window.set_brush_cursor_squash(1.0);
            window.set_brush_cursor_tilt(0.0);
        }
        if let Ok(mut bridge) = hover_clear_bridge.lock()
            && bridge.clear_hover()
        {
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let gizmo_hover_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let gizmo_hover_throttle = Rc::clone(&throttle);
    window.on_gizmo_hover(move |x, y| {
        if let Ok(mut bridge) = gizmo_hover_bridge.lock()
            && bridge.hover_gizmo(x, y)
            && let Some(window) = window_weak.upgrade()
        {
            refresh::refresh_interactive(
                &window,
                &gizmo_hover_bridge,
                &mut *bridge,
                &gizmo_hover_throttle,
            );
        }
    });
}


/// Conecta transformações interativas da viewport e drag de gizmos.
///
/// O reconhecimento de gesto acontece no Slint; a sessão e a matemática ficam no bridge/core.
pub(crate) fn connect_transform_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
    throttle: Rc<RefreshThrottle>,
) {
    let transform_begin_bridge = Arc::clone(&bridge);
    window.on_viewport_transform_begin(move |kind_str, x, y| {
        let kind = match kind_str.as_str() {
            "pos" => TransformKind::Position,
            "rot" => TransformKind::Rotation,
            "scale" => TransformKind::Scale,
            "slice" => {
                if let Ok(mut bridge) = transform_begin_bridge.lock() {
                    bridge.begin_slice(x, y);
                }
                return;
            }
            _ => return,
        };
        if let Ok(mut bridge) = transform_begin_bridge.lock() {
            bridge.begin_viewport_transform(kind, x, y);
        }
    });

    let transform_drag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let transform_drag_throttle = Rc::clone(&throttle);
    window.on_viewport_transform_update(move |x, y, fine, snap| {
        if let Ok(mut bridge) = transform_drag_bridge.lock() {
            bridge.pointer_position = [x, y];
            if !bridge.update_viewport_slice_modified(x, y, snap) {
                bridge.update_viewport_transform_modified(x, y, fine, snap);
            }
            if let Some(window) = window_weak.upgrade() {
                refresh::refresh_interactive(
                    &window,
                    &transform_drag_bridge,
                    &mut *bridge,
                    &transform_drag_throttle,
                );
            }
        }
    });

    let transform_end_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_transform_end(move || {
        if let Ok(mut bridge) = transform_end_bridge.lock() {
            bridge.end_viewport_transform();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });

    let gizmo_begin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_gizmo_drag_begin(move |x, y| {
        if let Ok(mut bridge) = gizmo_begin_bridge.lock() {
            bridge.begin_gizmo_drag(x, y);
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });

    let gizmo_end_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_gizmo_drag_end(move || {
        if let Ok(mut bridge) = gizmo_end_bridge.lock() {
            bridge.end_gizmo_drag();
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
        }
    });
}


impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Navegação da câmera. Nunca é suspensa por ferramenta; a roda sempre faz zoom.
    pub fn apply_viewport_gesture(&mut self, gesture: ViewportGesture) {
        match gesture {
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
    fn viewport_render_state(&self) -> ViewportRenderState {
        ViewportRenderState {
            shading: self.state.session.shading,
            xray: self.state.session.show_xray,
            show_triangulation: self.state.session.show_triangulation,
            textured: self.state.session.textured,
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
    pub fn render_viewport(&mut self) -> Option<slint::Image> {
        puffin::profile_function!();
        let render_state = self.viewport_render_state();
        self.viewport
            .queue_texture_updates(self.state.render.take_texture_updates());
        let pose = self.animate_pose_override();
        self.viewport.set_pose_override(pose);
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
        )
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
