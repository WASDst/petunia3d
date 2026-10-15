//! Headless surface/frame evidence through the real editor intent boundary.
use glam::{Quat, Vec3};
use petunia_core::{AppState, EditMode};
use petunia_editor::{SlintUiBridge, Software3dViewport, UiIntent};
use petunia_project::ObjectTransform;
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: origin_frame_feedback <output-directory>")?;
    std::fs::create_dir_all(&output)?;
    let mut bridge = SlintUiBridge::new(AppState::new("en"), Software3dViewport::new(800, 600));
    bridge.resize_viewport(800, 600);
    bridge.state.set_edit_mode(EditMode::Object);
    let asset = bridge
        .state
        .project
        .active_mut()
        .ok_or("No active object")?;
    asset.set_object_transform(ObjectTransform::from_trs(
        Vec3::new(0.2, 0.0, 0.0),
        Quat::from_rotation_y(0.4),
        Vec3::new(-1.4, 0.7, 1.0),
    ));
    bridge.state.session.cursor_3d = [1.2, 0.4, -0.5];
    bridge.render_viewport();
    let before = bridge.viewport.color_buffer.clone();
    image::save_buffer(
        output.join("before.png"),
        &before,
        800,
        600,
        image::ColorType::Rgba8,
    )?;
    bridge.apply(UiIntent::SetOriginCursor);
    if bridge
        .state
        .project
        .active()
        .ok_or("Missing origin target")?
        .transform
        .translation
        != [1.2, 0.4, -0.5]
    {
        return Err("Origin intent did not rebase the object".into());
    }
    bridge.render_viewport();
    let after = bridge.viewport.color_buffer.clone();
    image::save_buffer(
        output.join("rebased.png"),
        &after,
        800,
        600,
        image::ColorType::Rgba8,
    )?;
    let rebased_pixels = before
        .as_chunks::<4>()
        .0
        .iter()
        .zip(after.as_chunks::<4>().0.iter())
        .filter(|(a, b)| a != b)
        .count();
    // Floating-point reprojection may move a few edge samples; a displaced
    // surface changes thousands. This supplements the exact world-point tests.
    if rebased_pixels > 400 {
        return Err("Rebased surface moved in the headless image".into());
    }
    bridge
        .state
        .begin_modal(petunia_core::modal::ModalKind::Move)?;
    bridge
        .state
        .update_modal_components(Vec3::new(1.5, 0.0, 0.0))?;
    bridge.state.commit_modal();
    bridge.render_viewport();
    let moved = &bridge.viewport.color_buffer;
    image::save_buffer(
        output.join("moved.png"),
        moved,
        800,
        600,
        image::ColorType::Rgba8,
    )?;
    let moved_pixels = after
        .as_chunks::<4>()
        .0
        .iter()
        .zip(moved.as_chunks::<4>().0.iter())
        .filter(|(a, b)| a != b)
        .count();
    if moved_pixels < 1000 {
        return Err("Object frame move was not drawn".into());
    }
    let proof = serde_json::json!({"backend": "Software3dViewport", "native_qt": "not run",
        "rebased_changed_pixels": rebased_pixels, "moved_changed_pixels": moved_pixels,
        "size": [800, 600], "boundary": "UiIntent::SetOriginCursor -> CommandDispatcher"});
    std::fs::write(
        output.join("pixels.json"),
        serde_json::to_vec_pretty(&proof)?,
    )?;
    Ok(())
}
