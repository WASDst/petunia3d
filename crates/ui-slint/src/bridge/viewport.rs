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
    PetuniaSlintShell, PetuniaViewport, SlintUiBridge, UiIntent, ViewportGesture,
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
