//! Boundary Rust da viewport Slint.
//!
//! Esta camada registra callbacks semânticos da viewport e coordena bridge,
//! render e sincronização visual. Não contém picking ou regra geométrica.

use std::sync::{Arc, Mutex};

use slint::ComponentHandle;
use petunia_config::keybinds::Mods2;

use crate::{
    callbacks::sync_viewport_overlays, PetuniaSlintShell, PetuniaViewport, SlintUiBridge, UiIntent,
    ViewportGesture,
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
