//! Bridge das interações de PAINT sobre a viewport 3D.

use std::rc::Rc;
use std::sync::{Arc, Mutex};

use slint::ComponentHandle;

use crate::{
    callbacks::sync_window_properties,
    refresh::{self, RefreshThrottle},
    PetuniaSlintShell, PetuniaViewport, SlintUiBridge,
};

/// Conecta begin/update/end do traço 3D.
///
/// Os modificadores ainda chegam crus ao Rust porque Paint/Decal possui
/// semânticas próprias; a UI não interpreta essas mutações.
pub(crate) fn connect_viewport_paint_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
    throttle: Rc<RefreshThrottle>,
) {
    let paint_begin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_paint_begin(move |x, y, is_shift, is_ctrl| {
        if let Ok(mut bridge) = paint_begin_bridge.lock() {
            bridge.begin_paint_stroke_with_modifiers(x, y, is_shift, is_ctrl);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let paint_update_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let paint_update_throttle = Rc::clone(&throttle);
    window.on_viewport_paint_update(move |x, y, is_shift, is_ctrl| {
        if let Ok(mut bridge) = paint_update_bridge.lock() {
            bridge.paint_stroke_to_with_modifiers(x, y, is_shift, is_ctrl);
            if let Some(window) = window_weak.upgrade() {
                refresh::refresh_interactive(
                    &window,
                    &paint_update_bridge,
                    &mut *bridge,
                    &paint_update_throttle,
                );
                bridge.publish_canvas_image(&window);
            }
        }
    });

    let paint_end_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_paint_end(move |x, y| {
        if let Ok(mut bridge) = paint_end_bridge.lock() {
            bridge.end_paint_stroke_at(x, y);
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
