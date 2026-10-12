//! Pixels de pincel → raio no mundo (o mesmo valor do anel e do carimbo).

use petunia_core::{AppState, LogicalRect, Projection};

fn state_with_viewport(height: f32) -> AppState {
    let mut state = AppState::default();
    state.session.viewport_rect = Some(LogicalRect {
        min: [0.0, 0.0],
        max: [height * 1.5, height],
    });
    state
}

#[test]
fn a_brush_as_tall_as_the_viewport_spans_the_visible_height() {
    let state = state_with_viewport(900.0);
    let camera = state.session.camera.clone();
    let hit = camera.target;
    let depth = (hit - camera.eye()).dot(camera.forward());
    let visible = 2.0 * depth * (camera.fov_y * 0.5).tan();

    let radius = state.world_radius_for_px(hit, 900.0);
    assert!(
        (radius - visible * 0.5).abs() < visible * 1.0e-3,
        "raio {radius}, metade da altura visível {}",
        visible * 0.5
    );
    // Linear no tamanho em pixels: o dobro de pixels, o dobro do raio.
    let small = state.world_radius_for_px(hit, 12.8);
    assert!((state.world_radius_for_px(hit, 25.6) - 2.0 * small).abs() < 1.0e-6);
}

#[test]
fn orthographic_radius_does_not_depend_on_depth() {
    let mut state = state_with_viewport(900.0);
    state.session.camera.set_projection(Projection::Ortho);
    let camera = state.session.camera.clone();
    let near = camera.target;
    let far = camera.target + camera.forward() * 5.0;

    let radius = state.world_radius_for_px(near, 900.0);
    assert!((radius - camera.ortho_half_h).abs() < camera.ortho_half_h * 1.0e-3);
    assert!((state.world_radius_for_px(far, 900.0) - radius).abs() < 1.0e-6);
}
