//! CPU viewport evidence through creation intent and the numeric re-edit adapter.
use glam::{Quat, Vec3};
use petunia_core::{AppState, PrimitiveDescriptor, PrimitiveKind};
use petunia_editor::{SlintUiBridge, Software3dViewport, UiIntent};
use petunia_project::ObjectTransform;
use std::{error::Error, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: primitive_frame_feedback <output-directory>")?;
    std::fs::create_dir_all(&output)?;
    let mut bridge = SlintUiBridge::new(AppState::new("en"), Software3dViewport::new(800, 600));
    bridge.resize_viewport(800, 600);
    // Clear only the example's synthetic starter cube so the subject is visible.
    bridge.state.project.assets.clear();
    bridge.state.session.cursor_3d = [0.6, 0.3, -0.2];
    bridge.apply(UiIntent::AddPrimitive(PrimitiveKind::Cube));
    if !bridge.state.confirm_primitive() {
        return Err("Creation intent failed".into());
    }
    let expected = ObjectTransform::from_trs(
        Vec3::new(0.6, 0.3, -0.2),
        Quat::from_rotation_y(0.45),
        Vec3::new(-1.5, 0.7, 1.0),
    );
    bridge
        .state
        .project
        .active_mut()
        .ok_or("Missing primitive")?
        .set_object_transform(expected);
    bridge.render_viewport();
    let created = bridge.viewport.color_buffer.clone();
    image::save_buffer(
        output.join("created.png"),
        &created,
        800,
        600,
        image::ColorType::Rgba8,
    )?;
    bridge.state.session.cursor_3d = [-20.0; 3];
    if !bridge.update_primitive_param_float("width", 2.2) {
        return Err("Re-edit adapter failed".into());
    }
    let a = bridge
        .state
        .project
        .active()
        .ok_or("Missing re-edited primitive")?;
    if a.transform != expected || a.origin.is_some() || a.parametric.is_none() {
        return Err("Re-edit lost local origin, live descriptor or frame".into());
    }
    bridge.render_viewport();
    let resized = bridge.viewport.color_buffer.clone();
    image::save_buffer(
        output.join("resized.png"),
        &resized,
        800,
        600,
        image::ColorType::Rgba8,
    )?;
    // Explicit local authoring + expected TRS is the independent placement oracle.
    let reference = PrimitiveDescriptor::Box {
        width: 2.2,
        height: 1.0,
        depth: 1.0,
    }
    .build();
    let a = bridge
        .state
        .project
        .active_mut()
        .ok_or("Missing reference target")?;
    a.mesh = reference.into();
    a.set_object_transform(expected);
    bridge.state.emit_mesh_changed();
    bridge.render_viewport();
    let reference = &bridge.viewport.color_buffer;
    image::save_buffer(
        output.join("reference.png"),
        reference,
        800,
        600,
        image::ColorType::Rgba8,
    )?;
    let changed = |a: &[u8], b: &[u8]| {
        a.as_chunks::<4>()
            .0
            .iter()
            .zip(b.as_chunks::<4>().0)
            .filter(|(a, b)| a != b)
            .count()
    };
    let resized_pixels = changed(&created, &resized);
    let reference_pixels = changed(&resized, reference);
    if resized_pixels < 1000 || reference_pixels != 0 {
        return Err("Unexpected primitive frame image".into());
    }
    std::fs::write(
        output.join("pixels.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "backend": "Software3dViewport", "size": [800,600],
            "boundary": "UiIntent::AddPrimitive -> model.add_cube; numeric re-edit adapter -> Core",
            "resized_changed_pixels": resized_pixels, "reference_changed_pixels": reference_pixels,
            "native_qt": "not run"
        }))?,
    )?;
    Ok(())
}
