//! Bridge das interações de PAINT sobre a viewport 3D.

use std::rc::Rc;
use std::sync::{Arc, Mutex};

use slint::ComponentHandle;

use crate::{
    PetuniaSlintShell, PetuniaViewport, SlintUiBridge,
    callbacks::sync_window_properties,
    refresh::{self, RefreshThrottle},
};

/// Conecta begin/update/end do traço 3D.
///
/// O início do traço recebe ações já resolvidas pelo keymap `[pointer]`
/// (reta, origem do clone, escalar/girar decalque); a atualização não usa
/// modificadores. A UI não interpreta essas mutações.
pub(crate) fn connect_viewport_paint_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
    throttle: Rc<RefreshThrottle>,
) {
    let paint_begin_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_paint_begin(
        move |x, y, straight, clone_source, decal_scale, decal_rotate| {
            if let Ok(mut bridge) = paint_begin_bridge.lock() {
                bridge.begin_paint_stroke_actions(
                    x,
                    y,
                    crate::PaintPointerActions {
                        straight,
                        clone_source,
                        decal_scale,
                        decal_rotate,
                    },
                );
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
        },
    );

    let paint_update_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let paint_update_throttle = Rc::clone(&throttle);
    window.on_viewport_paint_update(move |x, y| {
        if let Ok(mut bridge) = paint_update_bridge.lock() {
            bridge.paint_stroke_to_with_modifiers(x, y, false, false);
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

    let paint_cancel_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_paint_cancel(move || {
        if let Ok(mut bridge) = paint_cancel_bridge.lock()
            && bridge.cancel_paint_stroke()
        {
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
