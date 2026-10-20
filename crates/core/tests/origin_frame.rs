use glam::{EulerRot, Mat4, Quat, Vec3};
use petunia_core::modal::{ModalConstraint, ModalError, ModalKind};
use petunia_core::origin_commands::SetObjectOriginCmd;
use petunia_core::{AppState, CommandDispatcher, EditMode, PivotPoint};
use petunia_project::{Asset, ObjectTransform};

fn state() -> AppState {
    let mut s = AppState::new("en");
    s.set_edit_mode(EditMode::Object);
    let a = s.project.active_mut().unwrap();
    for v in &mut a.mesh.verts {
        v.pos = (v.vec() + Vec3::new(2.0, 3.0, -1.0)).to_array();
    }
    a.set_object_transform(ObjectTransform::from_trs(
        Vec3::new(7.0, -2.0, 4.0),
        Quat::from_euler(EulerRot::XYZ, 0.3, -0.7, 0.2),
        Vec3::new(-2.0, 1.5, 0.75),
    ));
    s
}

fn assert_world(actual: &Asset, expected: &[Vec3]) {
    let points = actual.world_positions();
    assert_eq!(points.len(), expected.len());
    for (a, b) in points.iter().zip(expected) {
        assert!(a.distance(*b) < 2.0e-4, "{a} != {b}");
    }
}

fn snapshot(s: &AppState) -> serde_json::Value {
    serde_json::to_value(&s.project.project).unwrap()
}

#[test]
fn cursor_rebases_local_frame_preserving_world_surface_and_all_mesh_attributes() {
    let mut s = state();
    s.session.cursor_3d = [-3.0, 6.0, 2.0];
    let before = s.project.active().unwrap().clone();
    let world = before.world_positions();
    s.set_origin_cursor().unwrap();
    let after = s.project.active().unwrap();
    assert_world(after, &world);
    assert_eq!(after.transform.translation, [-3.0, 6.0, 2.0]);
    assert_eq!(after.transform.rotation, before.transform.rotation);
    assert_eq!(after.transform.scale, before.transform.scale);
    assert_eq!(after.origin, None);
    // Translation must leave topology, normals, corners, pins, colors and flags intact.
    let mut mesh = (*after.mesh).clone();
    for (v, old) in mesh.verts.iter_mut().zip(&before.mesh.verts) {
        v.pos = old.pos;
    }
    assert_eq!(
        serde_json::to_value(mesh).unwrap(),
        serde_json::to_value(&*before.mesh).unwrap()
    );
    let transform = after.transform;
    s.session.cursor_3d = [100.0, 200.0, 300.0];
    assert_eq!(s.project.active().unwrap().transform, transform);
    assert!(s.undo());
    assert_eq!(s.project.active().unwrap().transform, before.transform);
    assert_eq!(
        serde_json::to_value(&*s.project.active().unwrap().mesh).unwrap(),
        serde_json::to_value(&*before.mesh).unwrap()
    );
    assert!(s.redo());
    assert_world(s.project.active().unwrap(), &world);
    assert_eq!(s.project.active().unwrap().transform, transform);
}

#[test]
fn origin_presets_distinguish_all_geometry_selection_and_world_bottom() {
    for preset in 0..3 {
        let mut s = state();
        let a = s.project.active_mut().unwrap();
        a.mesh.deselect_all();
        a.mesh.verts[0].selected = true;
        let world = a.world_positions();
        let expected = match preset {
            0 => world.iter().sum::<Vec3>() / world.len() as f32,
            1 => world[0],
            _ => {
                let min = world
                    .iter()
                    .copied()
                    .fold(Vec3::splat(f32::INFINITY), Vec3::min);
                let max = world
                    .iter()
                    .copied()
                    .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
                Vec3::new((min.x + max.x) * 0.5, min.y, (min.z + max.z) * 0.5)
            }
        };
        match preset {
            0 => s.set_origin_geometry(),
            1 => s.set_origin_selection(),
            _ => s.set_origin_bottom(),
        }
        .unwrap();
        assert_world(s.project.active().unwrap(), &world);
        assert!(
            s.project
                .active()
                .unwrap()
                .origin_world()
                .distance(expected)
                < 1e-4
        );
    }
    let mut s = state();
    s.project.active_mesh_mut().unwrap().deselect_all();
    let before = snapshot(&s);
    assert!(s.set_origin_selection().is_err());
    assert_eq!(snapshot(&s), before);
}

#[test]
fn commands_reject_unsupported_sources_without_baking_or_history() {
    for case in 0..9 {
        let mut s = state();
        let a = s.project.active_mut().unwrap();
        match case {
            0 => a.locked = true,
            1 => a.skeleton_id = Some(uuid::Uuid::new_v4()),
            2 => a.modifiers.push(petunia_project::ModifierInstance {
                id: uuid::Uuid::new_v4(),
                enabled: false,
                kind: petunia_project::ModifierKind::Mirror { axis: 0, weld: 0.0 },
            }),
            3 => {
                a.parametric = Some(petunia_mesh::PrimitiveDescriptor::Box {
                    width: 1.0,
                    height: 1.0,
                    depth: 1.0,
                })
            }
            4 => a.transform.scale[0] = 0.0,
            5 => s.session.cursor_3d = [f32::NAN, 0.0, 0.0],
            6 => a.mesh.verts.clear(),
            7 => {
                a.skin_data = Some(petunia_project::SkinData::new(
                    uuid::Uuid::new_v4(),
                    a.mesh.verts.len(),
                ))
            }
            _ => a.mesh.verts[0].pos[0] = f32::NAN,
        }
        let before = snapshot(&s);
        let history = s.project.undo.undo_label().map(str::to_owned);
        let revision = s.project.project.revision_clock();
        assert!(s.set_origin_cursor().is_err(), "case {case}");
        assert_eq!(snapshot(&s), before);
        assert_eq!(s.project.project.revision_clock(), revision);
        assert_eq!(s.project.undo.undo_label(), history.as_deref());
    }
}

#[test]
fn stale_explicit_target_noop_and_registered_commands_use_transactional_dispatch() {
    let mut s = state();
    let cmd = SetObjectOriginCmd {
        object: s.project.active().unwrap().id,
        world_point: [1.0, 2.0, 3.0],
        revision: s.project.project.revision_clock(),
    };
    s.set_origin_geometry().unwrap();
    let before = snapshot(&s);
    assert!(s.dispatch(&cmd).is_err());
    assert_eq!(snapshot(&s), before);
    let noop = SetObjectOriginCmd {
        object: cmd.object,
        world_point: s.project.active().unwrap().transform.translation,
        revision: s.project.project.revision_clock(),
    };
    let label = s.project.undo.undo_label().map(str::to_owned);
    assert!(s.dispatch(&noop).is_err());
    assert_eq!(snapshot(&s), before);
    assert_eq!(s.project.undo.undo_label(), label.as_deref());
    let dispatcher = CommandDispatcher::canonical();
    for id in [
        "model.origin_geometry",
        "model.origin_bottom",
        "model.origin_cursor",
        "model.origin_selection",
        "model.geometry_to_origin",
    ] {
        assert!(dispatcher.contains(id));
    }
    s.session.cursor_3d = [2.0, 3.0, 4.0];
    dispatcher.execute("model.origin_cursor", &mut s).unwrap();
    assert_eq!(
        s.project.active().unwrap().transform.translation,
        [2.0, 3.0, 4.0]
    );
}

#[test]
fn object_grs_changes_only_frame_and_obeys_origin_after_rebase() {
    for (kind, c) in [
        (ModalKind::Move, Vec3::new(1.0, -2.0, 3.0)),
        (ModalKind::Rotate, Vec3::new(0.0, 35.0, 0.0)),
        (ModalKind::Scale, Vec3::splat(-1.5)),
    ] {
        let mut s = state();
        s.session.cursor_3d = [1.0, 2.0, -4.0];
        s.set_origin_cursor().unwrap();
        s.session.pivot_point = PivotPoint::MedianPoint;
        let a = s.project.active().unwrap().clone();
        let pivot = a.origin_world();
        let around = |m| Mat4::from_translation(pivot) * m * Mat4::from_translation(-pivot);
        let matrix = match kind {
            ModalKind::Move => Mat4::from_translation(c),
            ModalKind::Rotate => around(Mat4::from_rotation_y(c.y.to_radians())),
            _ => around(Mat4::from_scale(c)),
        };
        let expected: Vec<_> = a
            .world_positions()
            .into_iter()
            .map(|p| matrix.transform_point3(p))
            .collect();
        s.begin_modal(kind).unwrap();
        for _ in 0..30 {
            s.update_modal_components(c).unwrap();
        }
        assert_world(s.project.active().unwrap(), &expected);
        assert_eq!(
            serde_json::to_value(&*s.project.active().unwrap().mesh).unwrap(),
            serde_json::to_value(&*a.mesh).unwrap()
        );
        assert!(s.rigid_preview_transforms().is_empty());
        assert!(s.commit_modal());
        assert!(s.undo());
        assert_eq!(s.project.active().unwrap().transform, a.transform);
        assert!(s.redo());
        assert_world(s.project.active().unwrap(), &expected);
    }
}

#[test]
fn origin_drag_is_absolute_keeps_surface_and_cancels_or_commits_once() {
    for commit in [false, true] {
        let mut s = state();
        s.session.pivot_point = PivotPoint::BoundingBoxCenter;
        let original = s.project.active().unwrap().clone();
        let world = original.world_positions();
        s.toggle_edit_pivot();
        assert_eq!(
            s.calculate_pivot(PivotPoint::BoundingBoxCenter),
            original.origin_world()
        );
        assert_eq!(
            s.begin_modal(ModalKind::Rotate),
            Err(ModalError::UnsupportedFrame)
        );
        s.begin_modal(ModalKind::Move).unwrap();
        for _ in 0..30 {
            s.update_modal(Vec3::new(1.0, 2.0, 3.0), 0.0).unwrap();
        }
        assert_world(s.project.active().unwrap(), &world);
        assert!(
            s.project
                .active()
                .unwrap()
                .origin_world()
                .distance(original.origin_world() + Vec3::new(1.0, 2.0, 3.0))
                < 1e-4
        );
        if commit {
            s.commit_modal();
            assert!(s.undo());
        } else {
            s.cancel_modal();
        }
        assert_eq!(s.project.active().unwrap().transform, original.transform);
        assert_world(s.project.active().unwrap(), &world);
    }
}

#[test]
fn centers_and_individual_origins_are_distinct_and_shear_rejection_is_atomic() {
    let mut s = state();
    let id = s.project.active().unwrap().id;
    let mut other = s.project.active().unwrap().clone();
    other.id = uuid::Uuid::new_v4();
    other.set_object_transform(ObjectTransform::from_trs(
        Vec3::new(-8.0, 0.0, 1.0),
        Quat::IDENTITY,
        Vec3::ONE,
    ));
    let other_origin = other.origin_world();
    s.session.selection.assets = vec![id, other.id];
    s.project.assets.push(other);
    let median = s.calculate_pivot(PivotPoint::MedianPoint);
    let bounds = s.calculate_pivot(PivotPoint::BoundingBoxCenter);
    assert!(median.distance(bounds) > 0.1);
    s.session.pivot_point = PivotPoint::IndividualOrigins;
    s.begin_modal(ModalKind::Rotate).unwrap();
    s.set_modal_constraint(ModalConstraint::Axis(1)).unwrap();
    s.update_modal(Vec3::ZERO, 45.0).unwrap();
    assert!(s.project.assets[1].origin_world().distance(other_origin) < 1e-4);
    s.cancel_modal();
    s.begin_modal(ModalKind::Scale).unwrap();
    s.update_modal_components(Vec3::splat(1.25)).unwrap();
    let valid = snapshot(&s);
    assert_eq!(
        s.update_modal_components(Vec3::new(2.0, 1.0, 1.0)),
        Err(ModalError::UnsupportedFrame)
    );
    assert_eq!(snapshot(&s), valid);
    s.cancel_modal();
    // First target succeeds, second requires shear: neither may be published.
    s.project.assets.swap(0, 1);
    s.project.active = 0;
    s.begin_modal(ModalKind::Scale).unwrap();
    let before = snapshot(&s);
    assert_eq!(
        s.update_modal_components(Vec3::new(2.0, 1.0, 1.0)),
        Err(ModalError::UnsupportedFrame)
    );
    assert_eq!(snapshot(&s), before);
    s.cancel_modal();
}

#[test]
fn native_project_roundtrip_preserves_rebased_frame_and_legacy_offset_is_explicitly_normalized() {
    let mut s = state();
    s.project.active_mut().unwrap().origin = Some([2.0, -1.0, 0.0]);
    let legacy = serde_json::to_value(s.project.active().unwrap()).unwrap();
    let restored: Asset = serde_json::from_value(legacy).unwrap();
    assert_eq!(restored.origin, Some([2.0, -1.0, 0.0]));
    let world = restored.world_positions();
    s.session.cursor_3d = restored.origin_world().to_array();
    s.set_origin_cursor().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("origin.petunia");
    petunia_project::format::save(&s.project.project, &path).unwrap();
    let loaded = petunia_project::format::load(&path).unwrap();
    assert_world(loaded.active().unwrap(), &world);
    assert_eq!(loaded.active().unwrap().origin, None);
    assert_eq!(
        loaded.active().unwrap().transform,
        s.project.active().unwrap().transform
    );
}

#[test]
fn clear_transform_commands_reset_authoritative_trs_without_freezing_or_editing_mesh() {
    for part in 0..4 {
        let mut s = state();
        s.project.active_mut().unwrap().parametric = Some(petunia_mesh::PrimitiveDescriptor::Box {
            width: 1.0,
            height: 1.0,
            depth: 1.0,
        });
        let before = s.project.active().unwrap().clone();
        let mut expected = before.transform;
        match part {
            0 => {
                expected.translation = [0.0; 3];
                s.dispatch(&petunia_core::ClearLocationCmd).unwrap();
            }
            1 => {
                expected.rotation = [0.0, 0.0, 0.0, 1.0];
                s.dispatch(&petunia_core::ClearRotationCmd).unwrap();
            }
            2 => {
                expected.scale = [1.0; 3];
                s.dispatch(&petunia_core::ClearScaleCmd).unwrap();
            }
            _ => {
                expected = ObjectTransform::IDENTITY;
                s.dispatch(&petunia_core::ClearAllTransformsCmd).unwrap();
            }
        }
        let after = s.project.active().unwrap();
        assert_eq!(after.transform, expected);
        assert_eq!(after.parametric, before.parametric);
        assert_eq!(
            serde_json::to_value(&*after.mesh).unwrap(),
            serde_json::to_value(&*before.mesh).unwrap()
        );
        assert_world(
            after,
            &before
                .mesh
                .verts
                .iter()
                .map(|v| expected.to_mat4().transform_point3(v.vec()))
                .collect::<Vec<_>>(),
        );
        assert!(s.undo());
        assert_eq!(s.project.active().unwrap().transform, before.transform);
        assert!(s.redo());
        assert_eq!(s.project.active().unwrap().transform, expected);
    }
}

#[test]
fn geometry_to_origin_moves_surface_in_local_frame_and_keeps_object_frame() {
    let mut s = state();
    let frame = s.project.active().unwrap().transform;
    let before = s.project.active().unwrap().world_positions();
    s.set_geometry_to_origin().unwrap();
    let a = s.project.active().unwrap();
    assert_eq!(a.transform, frame);
    let points = a.world_positions();
    let center = points.iter().sum::<Vec3>() / points.len() as f32;
    assert!(center.distance(frame.translation()) < 1e-4);
    assert!(s.undo());
    assert_world(s.project.active().unwrap(), &before);
}

#[test]
fn separating_geometry_and_instantiating_transformed_objects_preserves_world_frames() {
    let mut s = state();
    s.session.cursor_3d = [4.0, 1.0, -3.0];
    s.set_origin_cursor().unwrap();
    s.set_edit_mode(EditMode::Edit);
    let a = s.project.active_mut().unwrap();
    a.mesh.deselect_all();
    a.mesh.faces[0].selected = true;
    a.mesh.sync_vert_selection_from_faces();
    let selected: Vec<_> = a
        .mesh
        .verts
        .iter()
        .filter(|v| v.selected)
        .map(|v| a.to_world_point(v.vec()))
        .collect();
    let frame = a.transform;
    s.dispatch(&petunia_core::SeparateSelectionCmd).unwrap();
    assert_eq!(s.project.active().unwrap().transform, frame);
    assert_world(s.project.active().unwrap(), &selected);
    let original = s.project.active().unwrap().clone();
    s.session.cursor_3d = [9.0, -1.0, 6.0];
    let index = s.project.active;
    assert!(s.instantiate_asset_at_cursor(index));
    let next = s.project.active().unwrap();
    assert_ne!(next.id, original.id);
    assert_eq!(next.transform.rotation, original.transform.rotation);
    assert_eq!(next.transform.scale, original.transform.scale);
    assert!(
        next.origin_world()
            .distance(Vec3::from(s.session.cursor_3d))
            < 1e-4
    );
    assert_eq!(
        serde_json::to_value(&*next.mesh).unwrap(),
        serde_json::to_value(&*original.mesh).unwrap()
    );
    let shift = Vec3::from(s.session.cursor_3d) - original.origin_world();
    assert_world(
        next,
        &original
            .world_positions()
            .into_iter()
            .map(|p| p + shift)
            .collect::<Vec<_>>(),
    );
}

#[test]
fn prefab_capture_and_placement_use_world_bounds_without_baking_local_geometry() {
    let mut s = state();
    let original = s.project.active().unwrap().clone();
    let world = original.world_positions();
    let min = world
        .iter()
        .copied()
        .fold(Vec3::splat(f32::INFINITY), Vec3::min);
    let max = world
        .iter()
        .copied()
        .fold(Vec3::splat(f32::NEG_INFINITY), Vec3::max);
    let center = min * 0.5 + max * 0.5;
    let id = s
        .project
        .project
        .create_prefab(&[original.id], Some("Frame"))
        .unwrap();
    let position = Vec3::new(4.0, 5.0, 6.0);
    let ids = s
        .project
        .project
        .instantiate_prefab(id, position.to_array());
    assert_eq!(ids.len(), 1);
    let a = s.project.assets.iter().find(|a| a.id == ids[0]).unwrap();
    assert_eq!(
        serde_json::to_value(&*a.mesh).unwrap(),
        serde_json::to_value(&*original.mesh).unwrap()
    );
    assert_world(
        a,
        &world
            .into_iter()
            .map(|p| p - center + position)
            .collect::<Vec<_>>(),
    );
}

#[test]
fn nonactive_origin_target_does_not_freeze_active_primitive_and_missing_id_is_rejected() {
    let mut s = state();
    let mut other = s.project.active().unwrap().clone();
    other.id = uuid::Uuid::new_v4();
    let id = other.id;
    s.project.assets.push(other);
    s.project.active_mut().unwrap().parametric = Some(petunia_mesh::PrimitiveDescriptor::Plane {
        width: 1.0,
        height: 1.0,
    });
    let cmd = SetObjectOriginCmd {
        object: id,
        world_point: [2.0, 3.0, 4.0],
        revision: s.project.project.revision_clock(),
    };
    s.dispatch(&cmd).unwrap();
    assert!(s.project.active().unwrap().parametric.is_some());
    let before = snapshot(&s);
    assert!(
        s.dispatch(&SetObjectOriginCmd {
            object: uuid::Uuid::new_v4(),
            revision: s.project.project.revision_clock(),
            ..cmd
        })
        .is_err()
    );
    assert_eq!(snapshot(&s), before);
}

#[test]
fn snap_geometry_and_frame_are_frozen_while_origin_drag_keeps_surface_available() {
    for origin in [false, true] {
        let mut s = state();
        let a = s.project.active().unwrap().clone();
        let mut other = a.clone();
        other.id = uuid::Uuid::new_v4();
        let other_id = other.id;
        s.project.assets.push(other);
        s.session.selection.assets = vec![a.id, other_id];
        if origin {
            s.toggle_edit_pivot();
        }
        s.begin_modal(ModalKind::Move).unwrap();
        s.update_modal(Vec3::new(5.0, 2.0, 1.0), 0.0).unwrap();
        let modal = s.modal.as_ref().unwrap();
        assert_eq!(
            modal.snap_source_asset(a.id).unwrap().transform,
            a.transform
        );
        assert_world(modal.snap_source_asset(a.id).unwrap(), &a.world_positions());
        assert_eq!(modal.excludes_snap_object(other_id), !origin);
        assert!(
            modal
                .moving_vertices()
                .iter()
                .all(|moving| *moving != origin)
        );
        assert!(modal.snap_source_asset(uuid::Uuid::new_v4()).is_none());
    }
}

#[test]
fn invalid_instance_and_prefab_frames_leave_document_and_history_untouched() {
    let mut s = state();
    let object = s.project.active().unwrap().id;
    let before = snapshot(&s);
    let history = s.project.undo.depth();
    assert!(!s.instantiate_asset_by_id(object, Some([f32::NAN, 0.0, 0.0])));
    assert_eq!(snapshot(&s), before);
    assert_eq!(s.project.undo.depth(), history);
    let id = s
        .project
        .project
        .create_prefab(&[object], Some("Valid"))
        .unwrap();
    let before = snapshot(&s);
    assert!(
        s.project
            .project
            .instantiate_prefab(id, [f32::INFINITY, 0.0, 0.0])
            .is_empty()
    );
    assert_eq!(snapshot(&s), before);
    let mut invalid = s.project.prefabs[0].parts[0].clone();
    invalid.transform.translation = [f32::MAX, 0.0, 0.0];
    s.project.prefabs[0].parts.push(invalid);
    let before = snapshot(&s);
    assert!(
        s.project
            .project
            .instantiate_prefab(id, [f32::MAX, 0.0, 0.0])
            .is_empty()
    );
    assert_eq!(snapshot(&s), before);
}

#[test]
fn distant_finite_origin_is_rejected_when_f32_rebase_would_collapse_surface() {
    let mut s = state();
    let before = snapshot(&s);
    let history = s.project.undo.depth();
    s.session.cursor_3d = [1.0e30, 0.0, 0.0];
    assert!(s.set_origin_cursor().is_err());
    assert_eq!(snapshot(&s), before);
    assert_eq!(s.project.undo.depth(), history);
}

#[test]
fn geometry_to_legacy_origin_uses_its_local_offset_and_preserves_frame() {
    let mut s = state();
    s.project.active_mut().unwrap().origin = Some([1.0, -2.0, 3.0]);
    let frame = s.project.active().unwrap().transform;
    let target = s.project.active().unwrap().origin_world();
    s.set_geometry_to_origin().unwrap();
    let a = s.project.active().unwrap();
    let world = a.world_positions();
    assert_eq!(a.transform, frame);
    assert!((world.iter().sum::<Vec3>() / world.len() as f32).distance(target) < 1e-4);
}

fn legacy_state() -> AppState {
    let mut s = AppState::new("en");
    s.set_edit_mode(EditMode::Object);
    let a = s.project.active_mut().unwrap();
    let p = Vec3::new(2.0, 3.0, -1.0);
    let matrix =
        Mat4::from_scale_rotation_translation(Vec3::splat(2.0), Quat::from_rotation_y(0.5), p);
    for v in &mut a.mesh.verts {
        v.pos = matrix.transform_point3(v.vec()).to_array();
    }
    a.position = p.to_array();
    a.rotation = [0.0, 0.5_f32.to_degrees(), 0.0];
    a.scale = [2.0; 3];
    a.origin = Some(p.to_array());
    s
}

#[test]
fn legacy_baked_objects_keep_metadata_and_visible_motion_until_explicit_rebase() {
    let mut s = legacy_state();
    let a = s.project.active().unwrap().clone();
    let world = a.world_positions();
    s.begin_modal(ModalKind::Move).unwrap();
    s.update_modal(Vec3::X, 0.0).unwrap();
    assert_world(
        s.project.active().unwrap(),
        &world.iter().map(|p| *p + Vec3::X).collect::<Vec<_>>(),
    );
    let moved = s.project.active().unwrap();
    assert_eq!(moved.transform, a.transform);
    assert_eq!(s.rigid_preview_transforms().len(), 1);
    assert_eq!(moved.rotation, a.rotation);
    assert_eq!(moved.scale, a.scale);
    assert_eq!(
        moved.position,
        (Vec3::from(a.position) + Vec3::X).to_array()
    );
    s.commit_modal();
    assert!(s.undo());
    assert_world(s.project.active().unwrap(), &world);
    s.session.cursor_3d = [1.0, 0.0, 0.0];
    s.set_origin_cursor().unwrap();
    assert!(!s.project.active().unwrap().has_legacy_baked_transform());
    let mesh = serde_json::to_value(&*s.project.active().unwrap().mesh).unwrap();
    s.begin_modal(ModalKind::Move).unwrap();
    s.update_modal(Vec3::Y, 0.0).unwrap();
    assert_eq!(
        serde_json::to_value(&*s.project.active().unwrap().mesh).unwrap(),
        mesh
    );
}

#[test]
fn legacy_reset_still_inverts_baked_trs_and_is_undoable() {
    let mut s = legacy_state();
    let before = s.project.active().unwrap().world_positions();
    let original_cube = petunia_mesh::Mesh::cube(2.0);
    // The old inverse order is S, R, T, around the legacy origin.
    s.dispatch(&petunia_core::ClearAllTransformsCmd).unwrap();
    assert_world(
        s.project.active().unwrap(),
        &original_cube
            .verts
            .iter()
            .map(|v| v.vec())
            .collect::<Vec<_>>(),
    );
    assert_eq!(
        s.project.active().unwrap().transform,
        ObjectTransform::IDENTITY
    );
    assert!(s.undo());
    assert_world(s.project.active().unwrap(), &before);
}

#[test]
fn legacy_without_offset_has_a_compatible_center_and_copy_keeps_it_at_target() {
    let mut s = legacy_state();
    s.project.active_mut().unwrap().origin = None;
    let a = s.project.active().unwrap().clone();
    let world = a.world_positions();
    assert!(a.origin_world().distance(Vec3::from(a.position)) < 1e-4);
    let point = Vec3::new(5.0, -2.0, 3.0);
    let copy = a.duplicate_at_world(point).unwrap();
    assert_eq!(copy.rotation, a.rotation);
    assert_eq!(copy.scale, a.scale);
    assert!(copy.origin_world().distance(point) < 1e-4);
    assert_world(
        &copy,
        &world
            .into_iter()
            .map(|p| p + point - a.origin_world())
            .collect::<Vec<_>>(),
    );
}
