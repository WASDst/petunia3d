//! Real creation / re-edit / history / persistence paths for local primitive frames.
use glam::{Quat, Vec3};
use petunia_core::modal::ModalKind;
use petunia_core::primitive_session::PrimitiveDescriptorExt;
use petunia_core::{
    AppState, CommandDispatcher, EditMode, PrimitiveDescriptor, PrimitiveKind, ProjectService,
};
use petunia_project::ObjectTransform;

fn box_desc(width: f32) -> PrimitiveDescriptor {
    PrimitiveDescriptor::Box {
        width,
        height: 2.0,
        depth: 3.0,
    }
}
fn snapshot(s: &AppState) -> serde_json::Value {
    serde_json::to_value(&s.project.project).unwrap()
}
fn assert_local(s: &AppState, desc: PrimitiveDescriptor) {
    let a = s.project.active().unwrap();
    assert_eq!(a.origin, None);
    assert!(!a.has_legacy_baked_transform());
    assert_eq!(a.parametric, Some(desc));
    assert_eq!(
        serde_json::to_value(&*a.mesh).unwrap(),
        serde_json::to_value(desc.build()).unwrap()
    );
}

#[test]
fn all_species_capture_cursor_in_frame_and_regenerate_locally_with_one_undo() {
    use PrimitiveKind::*;
    for kind in [
        Cube, Plane, Wedge, Cylinder, Cone, Circle, Torus, Sphere, Icosphere, Capsule,
    ] {
        let mut s = AppState::new("en");
        s.session.cursor_3d = [7.0, -2.0, 3.0];
        let depth = s.project.undo.depth().0;
        assert!(s.begin_primitive(kind, None));
        let desc = PrimitiveDescriptor::default_for(kind);
        let id = s.project.active().unwrap().id;
        assert_local(&s, desc);
        assert_eq!(
            s.project.active().unwrap().origin_world(),
            Vec3::new(7.0, -2.0, 3.0)
        );
        s.session.cursor_3d = [100.0, 200.0, -30.0];
        for _ in 0..30 {
            assert!(s.update_primitive(desc));
        }
        assert_local(&s, desc);
        let a = s.project.active().unwrap();
        assert_eq!(a.id, id);
        for (actual, local) in a.world_positions().iter().zip(desc.build().verts) {
            assert!(actual.distance(local.vec() + Vec3::new(7.0, -2.0, 3.0)) < 1.0e-5);
        }
        assert_eq!(s.project.undo.depth().0, depth + 1);
        assert!(s.confirm_primitive());
        assert!(s.undo());
        assert!(s.project.assets.iter().all(|a| a.id != id));
        assert!(s.redo());
        assert_local(&s, desc);
    }
}

#[test]
fn registered_creation_and_grs_keep_live_descriptor_then_reedit_in_same_frame() {
    let mut s = AppState::new("en");
    s.session.cursor_3d = [3.0, 1.0, -2.0];
    CommandDispatcher::canonical()
        .execute("model.add_cube", &mut s)
        .unwrap();
    assert!(s.confirm_primitive());
    s.set_edit_mode(EditMode::Object);
    for (kind, components) in [
        (ModalKind::Move, Vec3::new(1.0, -1.0, 2.0)),
        (ModalKind::Scale, Vec3::new(-2.0, 0.5, 1.5)),
        (ModalKind::Rotate, Vec3::new(0.0, 35.0, 0.0)),
    ] {
        s.begin_modal(kind).unwrap();
        s.update_modal_components(components).unwrap();
        s.commit_modal();
        assert!(s.active_is_parametric());
    }
    let before = s.project.active().unwrap().clone();
    let depth = s.project.undo.depth().0;
    let desc = box_desc(4.0);
    assert!(s.update_active_primitive(desc, true));
    let a = s.project.active().unwrap();
    assert_eq!(a.id, before.id);
    assert_eq!(a.transform, before.transform);
    assert_local(&s, desc);
    assert_eq!(s.project.undo.depth().0, depth + 1);
    for (actual, local) in a.world_positions().iter().zip(desc.build().verts) {
        assert!(actual.distance(before.to_world_point(local.vec())) < 1.0e-5);
    }
    assert!(s.undo());
    assert_eq!(s.project.active().unwrap().parametric, before.parametric);
    assert_eq!(
        s.project.active().unwrap().world_positions(),
        before.world_positions()
    );
    assert!(s.redo());
    assert_local(&s, desc);
}

#[test]
fn active_creation_updates_respect_existing_object_frame_not_session_cursor() {
    let mut s = AppState::new("en");
    assert!(s.begin_primitive(PrimitiveKind::Cube, None));
    let frame = ObjectTransform::from_trs(
        Vec3::new(4.0, 5.0, 6.0),
        Quat::from_rotation_z(0.4),
        Vec3::new(2.0, -1.0, 0.5),
    );
    s.project.active_mut().unwrap().set_object_transform(frame);
    s.session.cursor_3d = [-9.0; 3];
    assert!(s.update_primitive(box_desc(6.0)));
    assert_eq!(s.project.active().unwrap().transform, frame);
    assert_local(&s, box_desc(6.0));
    assert!(s.cancel_primitive());
    assert_eq!(s.project.assets.len(), 1);
}

#[test]
fn save_load_keeps_generator_and_frame_for_subsequent_reedit() {
    let mut s = AppState::new("en");
    s.session.cursor_3d = [8.0, 2.0, -4.0];
    assert!(s.begin_primitive(PrimitiveKind::Cube, None));
    assert!(s.confirm_primitive());
    let frame = ObjectTransform::from_trs(
        Vec3::new(8.0, 2.0, -4.0),
        Quat::from_rotation_y(0.7),
        Vec3::new(-2.0, 3.0, 0.4),
    );
    s.project.active_mut().unwrap().set_object_transform(frame);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("primitive.petunia");
    ProjectService::save_project(&mut s, &path).unwrap();
    let mut loaded = AppState::new("en");
    ProjectService::load_project(&mut loaded, &path).unwrap();
    assert!(loaded.update_active_primitive(box_desc(3.0), true));
    assert_eq!(loaded.project.active().unwrap().transform, frame);
    assert_local(&loaded, box_desc(3.0));
}

#[test]
fn invalid_creation_preserves_previous_session_document_and_history() {
    let mut s = AppState::new("en");
    assert!(s.begin_primitive(PrimitiveKind::Cube, None));
    let before = snapshot(&s);
    let id = s.session.primitive_session.as_ref().unwrap().asset_id;
    let depth = s.project.undo.depth();
    assert!(!s.in_context(|ctx| ctx.begin_primitive_with(box_desc(f32::NAN), None)));
    assert_eq!(snapshot(&s), before);
    assert_eq!(s.session.primitive_session.as_ref().unwrap().asset_id, id);
    assert_eq!(s.project.undo.depth(), depth);
    s.session.cursor_3d = [f32::INFINITY, 0.0, 0.0];
    assert!(!s.begin_primitive(PrimitiveKind::Sphere, None));
    assert_eq!(snapshot(&s), before);
    assert_eq!(s.project.undo.depth(), depth);
    assert_eq!(s.session.primitive_session.as_ref().unwrap().asset_id, id);
}

#[test]
fn invalid_regeneration_is_atomic_for_session_and_confirmed_sources() {
    for live in [true, false] {
        for case in 0..6 {
            let mut s = AppState::new("en");
            assert!(s.begin_primitive(PrimitiveKind::Cube, None));
            if !live {
                assert!(s.confirm_primitive());
            }
            let a = s.project.active_mut().unwrap();
            match case {
                1 => a.locked = true,
                2 => a.transform.scale = [0.0; 3],
                3 => a.transform.translation = [f32::INFINITY; 3],
                4 => a.skeleton_id = Some(uuid::Uuid::new_v4()),
                _ => {}
            }
            let desc = match case {
                0 => box_desc(f32::NAN),
                5 => box_desc(f32::INFINITY),
                _ => box_desc(5.0),
            };
            // JSON cannot distinguish non-finite floats, so also assert the raw frame.
            let frame = s.project.active().unwrap().transform;
            let before = snapshot(&s);
            let depth = s.project.undo.depth();
            assert!(
                !s.update_active_primitive(desc, true),
                "live={live}, case={case}"
            );
            assert_eq!(s.project.active().unwrap().transform, frame);
            assert_eq!(snapshot(&s), before);
            assert_eq!(s.project.undo.depth(), depth);
        }
    }
}

#[test]
fn translation_only_legacy_regenerates_but_baked_rotation_is_refused_without_data_loss() {
    for rotated in [false, true] {
        let mut s = AppState::new("en");
        let old = box_desc(1.0);
        let offset = Vec3::new(3.0, -1.0, 2.0);
        let a = s.project.active_mut().unwrap();
        a.mesh = old.build().into();
        a.parametric = Some(old);
        a.origin = Some(offset.to_array());
        for v in &mut a.mesh.verts {
            v.pos = (if rotated {
                Quat::from_rotation_y(0.7) * v.vec()
            } else {
                v.vec()
            } + offset)
                .to_array();
        }
        if rotated {
            a.rotation = [0.0, 0.7_f32.to_degrees(), 0.0];
        }
        let frame = a.transform;
        let before = snapshot(&s);
        let depth = s.project.undo.depth();
        assert_eq!(s.update_active_primitive(box_desc(4.0), true), !rotated);
        if rotated {
            assert_eq!(snapshot(&s), before);
            assert_eq!(s.project.undo.depth(), depth);
            assert!(s.active_is_parametric());
        } else {
            let a = s.project.active().unwrap();
            assert_eq!(a.transform, frame);
            assert_eq!(a.origin, Some(offset.to_array()));
            for (actual, v) in a.mesh.verts.iter().zip(box_desc(4.0).build().verts) {
                assert!(actual.vec().distance(v.vec() + offset) < 1.0e-5);
            }
            assert!(s.undo());
            assert_eq!(snapshot(&s)["assets"], before["assets"]);
        }
    }
}

#[test]
fn stale_session_never_redirects_parameter_edits_to_previous_primitive() {
    let mut s = AppState::new("en");
    assert!(s.begin_primitive(PrimitiveKind::Cube, None));
    let original = s.project.active().unwrap().clone();
    s.project.active = 0;
    assert!(!s.primitive_session_valid());
    assert!(!s.update_active_primitive(box_desc(5.0), true));
    assert!(!s.cancel_primitive());
    let previous = s
        .project
        .assets
        .iter()
        .find(|a| a.id == original.id)
        .unwrap();
    assert_eq!(previous.parametric, original.parametric);
    assert_eq!(previous.world_positions(), original.world_positions());
}

#[test]
fn parameter_edit_during_transform_is_refused_and_cancel_restores_source() {
    let mut s = AppState::new("en");
    assert!(s.begin_primitive(PrimitiveKind::Cube, None));
    assert!(s.confirm_primitive());
    s.set_edit_mode(EditMode::Object);
    let before = s.project.active().unwrap().clone();
    s.begin_modal(ModalKind::Move).unwrap();
    s.update_modal_components(Vec3::X).unwrap();
    let preview = snapshot(&s);
    let depth = s.project.undo.depth();
    assert!(!s.update_active_primitive(box_desc(5.0), true));
    assert_eq!(snapshot(&s), preview);
    assert_eq!(s.project.undo.depth(), depth);
    assert!(!s.begin_primitive(PrimitiveKind::Sphere, None));
    assert_eq!(snapshot(&s), preview);
    s.cancel_modal();
    assert_eq!(
        s.project.active().unwrap().world_positions(),
        before.world_positions()
    );
}

#[test]
fn free_geometry_edits_are_not_overwritten_by_a_stale_descriptor() {
    let mut s = AppState::new("en");
    assert!(s.begin_primitive(PrimitiveKind::Cube, None));
    assert!(s.confirm_primitive());
    s.project.active_mut().unwrap().mesh.verts[0].pos[0] += 0.2;
    let before = snapshot(&s);
    let depth = s.project.undo.depth();
    assert!(!s.update_active_primitive(box_desc(10.0), true));
    assert_eq!(snapshot(&s), before);
    assert_eq!(s.project.undo.depth(), depth);
}
