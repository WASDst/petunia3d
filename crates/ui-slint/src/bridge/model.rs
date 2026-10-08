//! Bridge das interações de MODEL/DRAW que acontecem sobre a viewport.

use std::rc::Rc;
use std::sync::{Arc, Mutex};

use slint::ComponentHandle;

use crate::{
    callbacks::sync_window_properties,
    refresh::{self, RefreshThrottle},
    PetuniaSlintShell, PetuniaViewport, SlintUiBridge,
};

/// Conecta edição de Profile/Shape do workspace DRAW.
///
/// A UI envia coordenadas/gestos; spline, picking e mutações permanecem no bridge/core.
pub(crate) fn connect_profile_callbacks<V: PetuniaViewport + 'static>(
    window: &PetuniaSlintShell,
    bridge: Arc<Mutex<SlintUiBridge<V>>>,
    throttle: Rc<RefreshThrottle>,
) {
    let viewport_profile_drag_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_profile_drag_handle(move |x, y, break_tangent| {
        if let Ok(mut bridge) = viewport_profile_drag_bridge.lock()
            && bridge.profile_update_drag_handle(x, y, break_tangent)
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

    let profile_down_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_profile_pointer_down(move |x, y, alt, ctrl| {
        if let Ok(mut bridge) = profile_down_bridge.lock() {
            let hit = bridge.profile_pointer_down_ex(x, y, alt, ctrl);
            let vm = bridge.view_model();
            let new_frame = bridge.render_viewport();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
                if let Some(frame) = new_frame {
                    window.set_viewport_image(frame);
                }
            }
            hit
        } else {
            false
        }
    });

    let profile_move_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    let profile_move_throttle = Rc::clone(&throttle);
    window.on_viewport_profile_pointer_move(move |x, y, alt| {
        if let Ok(mut bridge) = profile_move_bridge.lock()
            && bridge.profile_pointer_move(x, y, alt)
            && let Some(window) = window_weak.upgrade()
        {
            refresh::refresh_interactive(
                &window,
                &profile_move_bridge,
                &mut *bridge,
                &profile_move_throttle,
            );
        }
    });

    let profile_up_bridge = Arc::clone(&bridge);
    let window_weak = window.as_weak();
    window.on_viewport_profile_pointer_up(move || {
        if let Ok(mut bridge) = profile_up_bridge.lock() {
            bridge.profile_pointer_up();
            let vm = bridge.view_model();
            if let Some(window) = window_weak.upgrade() {
                sync_window_properties(&window, &vm);
            }
        }
    });
}
