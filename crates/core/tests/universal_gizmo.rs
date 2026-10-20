//! Integration tests for universal gizmo, orientation frames, and snap (SEL-001, SEL-003, SEL-005, SEL-007, Task H23).

use petunia_core::{
    ActiveElement, AppState, ModalConstraint, ModalKind, PivotPoint, SelectionFilters,
    SelectionRef, TransformFrame, TransformOrientation,
};
use petunia_mesh::{Face, Mesh, Vertex};
use petunia_project::Asset;

fn sample_pyramid_mesh() -> Mesh {
    // Base quad on XY plane (Z=0) with normal (0, 0, -1), apex at (0.5, 0.5, 1.0)
    Mesh {
        verts: vec![
            Vertex::new(0.0, 0.0, 0.0), // 0
            Vertex::new(1.0, 0.0, 0.0), // 1
            Vertex::new(1.0, 1.0, 0.0), // 2
            Vertex::new(0.0, 1.0, 0.0), // 3
            Vertex::new(0.5, 0.5, 1.0), // 4 (apex)
        ],
        faces: vec![
            Face::new(vec![3, 2, 1, 0]), // Base face pointing -Z
            Face::new(vec![0, 1, 4]),    // Slanted face +Y normal
            Face::new(vec![1, 2, 4]),    // Slanted face +X normal
            Face::new(vec![2, 3, 4]),    // Slanted face -Y normal
            Face::new(vec![3, 0, 4]),    // Slanted face -X normal
        ],
        ..Default::default()
    }
}

#[test]
fn transform_frame_constructors_and_axes() {
    // 1. Global frame
    let global = TransformFrame::global();
    assert_eq!(global.x_axis, glam::Vec3::X);
    assert_eq!(global.y_axis, glam::Vec3::Y);
    assert_eq!(global.z_axis, glam::Vec3::Z);
    assert_eq!(global.axis(0), glam::Vec3::X);
    assert_eq!(global.axis(1), glam::Vec3::Y);
    assert_eq!(global.axis(2), glam::Vec3::Z);
    assert_eq!(global.plane_normal(0), glam::Vec3::X);

    // 2. Rotated frame (90 degrees around Z axis)
    let rot = glam::Quat::from_rotation_z(90.0_f32.to_radians());
    let rotated = TransformFrame::from_rotation(rot);
    assert!((rotated.x_axis.x - 0.0).abs() < 1e-5);
    assert!((rotated.x_axis.y - 1.0).abs() < 1e-5);
    assert!((rotated.y_axis.x - (-1.0)).abs() < 1e-5);
    assert!((rotated.y_axis.y - 0.0).abs() < 1e-5);
    assert!((rotated.z_axis.z - 1.0).abs() < 1e-5);

    // 3. From normal / Z axis
    let normal = glam::Vec3::new(0.0, 1.0, 0.0);
    let from_norm = TransformFrame::from_z_axis(normal);
    assert_eq!(from_norm.z_axis, glam::Vec3::Y);
    assert!((from_norm.x_axis.dot(from_norm.y_axis)).abs() < 1e-5);
    assert!((from_norm.x_axis.dot(from_norm.z_axis)).abs() < 1e-5);
    assert!((from_norm.y_axis.dot(from_norm.z_axis)).abs() < 1e-5);

    // 4. From edge
    let p0 = glam::Vec3::new(0.0, 0.0, 0.0);
    let p1 = glam::Vec3::new(5.0, 0.0, 0.0);
    let from_edge = TransformFrame::from_edge(p0, p1, glam::Vec3::Y);
    assert_eq!(from_edge.x_axis, glam::Vec3::X);
    assert!((from_edge.x_axis.dot(from_edge.y_axis)).abs() < 1e-5);
    assert!((from_edge.x_axis.dot(from_edge.z_axis)).abs() < 1e-5);
}

#[test]
fn independence_of_pivot_and_orientation_sel_003() {
    let mut state = AppState::new("en");
    let mesh = sample_pyramid_mesh();
    let mut asset = Asset::new("Pyramid", mesh);
    asset.transform.translation = [10.0, 20.0, 30.0];
    asset.transform.rotation = glam::Quat::from_rotation_y(90.0_f32.to_radians()).to_array();
    state.project.assets.clear();
    state.project.assets.push(asset);
    state.project.active = 0;

    // Verify initial independence
    state.session.pivot_point = PivotPoint::Cursor3D;
    state.session.cursor_3d = [1.0, 2.0, 3.0];
    state.session.transform_orientation = TransformOrientation::Local;

    let pivot_at_cursor = state.calculate_pivot(state.session.pivot_point);
    assert_eq!(pivot_at_cursor, glam::Vec3::new(1.0, 2.0, 3.0));

    let local_frame = state.calculate_transform_frame(state.session.transform_orientation);
    // 90 deg around Y: X becomes -Z, Z becomes X
    assert!((local_frame.x_axis.z - (-1.0)).abs() < 1e-5);
    assert!((local_frame.z_axis.x - 1.0).abs() < 1e-5);

    // Change pivot to BoundingBoxCenter: frame MUST remain unchanged
    state.session.pivot_point = PivotPoint::BoundingBoxCenter;
    let frame_unchanged = state.calculate_transform_frame(state.session.transform_orientation);
    assert_eq!(local_frame, frame_unchanged);

    // Change orientation to Global: pivot MUST remain unchanged
    state.session.transform_orientation = TransformOrientation::Global;
    let pivot_unchanged = state.calculate_pivot(state.session.pivot_point);
    assert_ne!(pivot_unchanged, pivot_at_cursor); // bbox center differs from cursor
    let global_frame = state.calculate_transform_frame(state.session.transform_orientation);
    assert_eq!(global_frame, TransformFrame::global());
}

#[test]
fn custom_orientation_frame_sel_003() {
    let mut state = AppState::new("en");
    let custom = TransformFrame {
        x_axis: glam::Vec3::new(0.0, 1.0, 0.0),
        y_axis: glam::Vec3::new(-1.0, 0.0, 0.0),
        z_axis: glam::Vec3::new(0.0, 0.0, 1.0),
    };

    // When None, fallback to Global
    assert_eq!(
        state.calculate_transform_frame(TransformOrientation::Custom),
        TransformFrame::global()
    );

    // When set, Custom returns the custom frame
    state.session.custom_transform_frame = Some(custom);
    assert_eq!(
        state.calculate_transform_frame(TransformOrientation::Custom),
        custom
    );

    // ViewportSession synchronization preserves custom_transform_frame
    let vs = state.session.viewport_session();
    assert_eq!(vs.custom_transform_frame, Some(custom));
}

#[test]
fn auto_orientation_resolution_from_active_element_sel_003() {
    let mut state = AppState::new("en");
    let mesh = sample_pyramid_mesh();
    let asset = Asset::new("Pyramid", mesh);
    let asset_id = asset.id;
    state.project.assets.clear();
    state.project.assets.push(asset);
    state.project.active = 0;
    state.session.selection.filters = SelectionFilters::all();

    // 1. Auto with active element = Face (Base face pointing -Z)
    state.session.selection.active_element =
        Some(ActiveElement::new(SelectionRef::face(asset_id, 0)));
    let auto_face_frame = state.calculate_transform_frame(TransformOrientation::Auto);
    assert!((auto_face_frame.z_axis.z - (-1.0)).abs() < 1e-4);

    // 2. Auto with active element = Edge (Edge 0 -> 1 along X axis)
    state.session.selection.active_element =
        Some(ActiveElement::new(SelectionRef::edge(asset_id, 0, 1)));
    let auto_edge_frame = state.calculate_transform_frame(TransformOrientation::Auto);
    assert!((auto_edge_frame.x_axis.x - 1.0).abs() < 1e-4);

    // 3. Auto with active element = Vertex (apex vertex 4)
    state.session.selection.active_element =
        Some(ActiveElement::new(SelectionRef::vertex(asset_id, 4)));
    let auto_vert_frame = state.calculate_transform_frame(TransformOrientation::Auto);
    // Apex normal points up (+Z)
    assert!(auto_vert_frame.z_axis.z > 0.5);

    // 4. Auto with Object domain: single object uses object rotation
    state.session.selection.active_element =
        Some(ActiveElement::new(SelectionRef::object(asset_id)));
    state.project.assets[0].transform.rotation =
        glam::Quat::from_rotation_x(45.0_f32.to_radians()).to_array();
    let auto_obj_frame = state.calculate_transform_frame(TransformOrientation::Auto);
    let expected =
        TransformFrame::from_rotation(glam::Quat::from_rotation_x(45.0_f32.to_radians()));
    assert!((auto_obj_frame.y_axis.dot(expected.y_axis) - 1.0).abs() < 1e-4);
}

#[test]
fn frozen_transform_frame_during_modal_operation_sel_003() {
    let mut state = AppState::new("en");
    let mesh = sample_pyramid_mesh();
    let asset = Asset::new("Pyramid", mesh);
    let asset_id = asset.id;
    state.project.assets.clear();
    state.project.assets.push(asset);
    state.project.active = 0;
    state.session.selection.filters = SelectionFilters::all();

    // Select vertex 4 (apex) in Edit mode
    state.set_edit_mode(petunia_core::EditMode::Edit);
    let mesh = state.project.active_mesh_mut().unwrap();
    mesh.deselect_all();
    mesh.verts[4].selected = true;
    state.sync_selection();
    state
        .session
        .selection
        .select_item(SelectionRef::vertex(asset_id, 4));
    assert!(state.session.selection.verts.contains(&4));

    // Set custom frame where X is Z, and Z is X
    let custom_frame = TransformFrame {
        x_axis: glam::Vec3::Z,
        y_axis: glam::Vec3::Y,
        z_axis: glam::Vec3::X,
    };
    state.session.custom_transform_frame = Some(custom_frame);
    state.session.transform_orientation = TransformOrientation::Custom;

    // Begin modal Move operation
    state
        .begin_modal(ModalKind::Move)
        .expect("Begin modal move");
    let modal_frame = state
        .session
        .tools
        .modal
        .as_ref()
        .expect("Active modal")
        .frame;
    assert_eq!(modal_frame, custom_frame);

    // Change orientation on session while gesture is active: modal.frame MUST remain frozen!
    state.session.transform_orientation = TransformOrientation::Global;
    let modal_frame_after = state
        .session
        .tools
        .modal
        .as_ref()
        .expect("Active modal")
        .frame;
    assert_eq!(modal_frame_after, custom_frame);

    // Constrained translation along Axis(0): MUST translate along modal.frame.x_axis (which is Z)
    state
        .set_modal_constraint(ModalConstraint::Axis(0))
        .expect("Set axis constraint");
    state
        .update_modal(glam::Vec3::ZERO, 5.0)
        .expect("Update modal with 5.0 units along Axis 0");

    let vert_pos = state.project.active().unwrap().mesh.verts[4].vec();
    // Original vertex 4 pos was (0.5, 0.5, 1.0).
    // Axis 0 is custom_frame.x_axis = Vec3::Z.
    // So pos.z should be 1.0 + 5.0 = 6.0, while x and y remain 0.5!
    assert!((vert_pos.x - 0.5).abs() < 1e-4);
    assert!((vert_pos.y - 0.5).abs() < 1e-4);
    assert!((vert_pos.z - 6.0).abs() < 1e-4);

    // Commit modal operation: records single undo checkpoint
    let committed = state.commit_modal();
    assert!(committed);
    assert!(state.project.undo.can_undo());

    // Undo restores apex to (0.5, 0.5, 1.0)
    let undone = state.undo();
    assert!(undone);
    let restored_pos = state.project.active().unwrap().mesh.verts[4].vec();
    assert_eq!(restored_pos, glam::Vec3::new(0.5, 0.5, 1.0));
}
