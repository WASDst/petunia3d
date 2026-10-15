use petunia_core::{AnimatePreview, AppState, RigPresetKind};
use petunia_project::{AnimationAsset, Interpolation, MotionGenerator, Transform3D};

fn rig() -> (AppState, u32) {
    let mut state = AppState::default();
    let id = state.animate_add_rig(RigPresetKind::Humanoid).unwrap();
    let bone = state.project.project.get_skeleton(id).unwrap().bones[1].id;
    state.animate_select_bone(Some(bone));
    (state, bone)
}

fn animated_state() -> (AppState, u32) {
    let (mut state, bone) = rig();
    state.animate_insert_keyframe().unwrap();
    let mut moved = Transform3D::from_euler_degrees(0.0, 0.0, 0.0);
    moved.translation = [10.0, 0.0, 0.0];
    state.session.animate.time = 1.0;
    state.session.animate.set_bone_draft(bone, moved);
    state.animate_insert_keyframe().unwrap();
    (state, bone)
}

#[test]
fn deletion_preserves_neighbors_at_24_30_and_60_fps_and_undo() {
    for fps in [24.0, 30.0, 60.0] {
        let (mut state, bone) = rig();
        for t in [0.0, 1.0 / fps, 2.0 / fps] {
            state.session.animate.time = t;
            state.animate_insert_keyframe().unwrap();
        }
        let id = state.session.animate.active_clip.unwrap();
        let before = state.project.project.get_animation(id).unwrap().clone();
        state.animate_delete_keyframe(Some(1.0 / fps)).unwrap();
        let track = state
            .project
            .project
            .get_animation(id)
            .unwrap()
            .clip
            .get_track(bone)
            .unwrap();
        assert_eq!(track.translations.len(), 2);
        assert!(track.has_keyframe_at(0.0, 0.0));
        assert!(track.has_keyframe_at(2.0 / fps, 0.0));
        assert!(state.undo());
        assert_eq!(state.project.project.get_animation(id).unwrap(), &before);
        assert!(state.redo());
        assert_eq!(
            state
                .project
                .project
                .get_animation(id)
                .unwrap()
                .clip
                .get_track(bone)
                .unwrap()
                .translations
                .len(),
            2
        );
    }
}

#[test]
fn keying_without_draft_preserves_visible_interpolated_pose() {
    let (mut state, bone) = animated_state();
    state.session.animate.time = 0.5;
    let sid = state.session.animate.skeleton.unwrap();
    let index = state
        .project
        .project
        .get_skeleton(sid)
        .unwrap()
        .bone_index(bone)
        .unwrap();
    let mut preview = AnimatePreview::default();
    let before = preview
        .local_pose(&state.project.project, &state.session.animate)
        .unwrap()
        .1[index];
    assert!((before.translation[0] - 5.0).abs() < 1e-5);
    state.animate_insert_keyframe().unwrap();
    let after = preview
        .local_pose(&state.project.project, &state.session.animate)
        .unwrap()
        .1[index];
    assert_eq!(before, after);
}

#[test]
fn keying_procedural_pose_preserves_the_sample() {
    let (mut state, bone) = rig();
    state
        .animate_add_motion(MotionGenerator::IdleBreath)
        .unwrap();
    state.session.animate.time = 0.37;
    let sid = state.session.animate.skeleton.unwrap();
    let index = state
        .project
        .project
        .get_skeleton(sid)
        .unwrap()
        .bone_index(bone)
        .unwrap();
    let mut preview = AnimatePreview::default();
    let before = preview
        .local_pose(&state.project.project, &state.session.animate)
        .unwrap()
        .1[index];
    state.animate_insert_keyframe().unwrap();
    assert_eq!(
        preview
            .local_pose(&state.project.project, &state.session.animate)
            .unwrap()
            .1[index],
        before
    );
}

#[test]
fn every_authoring_operation_is_confined_to_the_explicit_clip() {
    let (mut state, bone) = animated_state();
    let original_id = state.session.animate.active_clip.unwrap();
    let original = state
        .project
        .project
        .get_animation(original_id)
        .unwrap()
        .clone();
    let mut second = AnimationAsset::new("Second", original.clip.clone());
    second.tags = original.tags.clone();
    let id = second.id;
    state.project.project.add_animation(second);
    state.session.animate.active_clip = Some(id);
    state.session.animate.time = 0.5;
    state.animate_insert_keyframe().unwrap();
    state
        .animate_set_keyframe_interp(Interpolation::Step)
        .unwrap();
    state.animate_delete_keyframe(Some(0.0)).unwrap();
    state.animate_clear_bone_track().unwrap();
    assert_eq!(
        state.project.project.get_animation(original_id).unwrap(),
        &original
    );
    assert!(
        state
            .project
            .project
            .get_animation(id)
            .unwrap()
            .clip
            .get_track(bone)
            .unwrap()
            .translations
            .is_empty()
    );
    // New explicit clip IDs and authored channels survive project serialization.
    let text = serde_json::to_string(&state.project.project).unwrap();
    let reloaded: petunia_project::Project = serde_json::from_str(&text).unwrap();
    assert_eq!(reloaded.get_animation(original_id).unwrap(), &original);
    assert_eq!(
        reloaded.get_animation(id),
        state.project.project.get_animation(id)
    );
}

#[test]
fn stale_ambiguous_and_foreign_clips_do_not_mutate_or_consume_drafts() {
    let (mut state, bone) = animated_state();
    state
        .session
        .animate
        .set_bone_draft(bone, Transform3D::default());
    let draft = state.session.animate.bone_draft().copied();
    let before = state.project.project.clone();
    let depth = state.project.undo.depth();
    state.session.animate.active_clip = Some(uuid::Uuid::new_v4());
    assert!(state.animate_insert_keyframe().is_err());
    assert!(state.animate_delete_keyframe(Some(0.0)).is_err());
    assert!(
        state
            .animate_set_keyframe_interp(Interpolation::Step)
            .is_err()
    );
    assert!(state.animate_clear_bone_track().is_err());
    assert_eq!(state.session.animate.bone_draft(), draft.as_ref());
    assert_eq!(state.project.project.animations, before.animations);
    assert_eq!(state.project.undo.depth(), depth);
    let mut other = AnimationAsset::new("Foreign", before.animations[0].clip.clone());
    other
        .tags
        .push(format!("skeleton:{}", uuid::Uuid::new_v4()));
    state.session.animate.active_clip = Some(other.id);
    state.project.project.add_animation(other);
    assert!(state.animate_insert_keyframe().is_err());
    let mut second = before.animations[0].clone();
    second.id = uuid::Uuid::new_v4();
    state.project.project.add_animation(second);
    state.session.animate.active_clip = None;
    assert!(state.animate_insert_keyframe().is_err());
}

#[test]
fn blended_layers_require_an_explicit_authoring_policy() {
    let (mut state, bone) = animated_state();
    state.animate_add_motion_layer("Blend", false).unwrap();
    state
        .session
        .animate
        .set_bone_draft(bone, Transform3D::default());
    let before = state.project.project.animations.clone();
    let depth = state.project.undo.depth();
    assert!(state.animate_insert_keyframe().is_err());
    assert_eq!(state.project.project.animations, before);
    assert_eq!(state.project.undo.depth(), depth);
    assert!(state.session.animate.bone_draft().is_some());
}

#[test]
fn undoing_first_clip_creation_does_not_leave_an_unusable_authoring_target() {
    for redo in [false, true] {
        let (mut s, _) = rig();
        s.animate_insert_keyframe().unwrap();
        let original = s.session.animate.active_clip.unwrap();
        assert!(s.undo());
        assert!(s.project.project.animations.is_empty());
        assert!(s.session.animate.active_clip.is_none());
        if redo {
            assert!(s.redo());
            assert_eq!(
                s.session
                    .animate
                    .resolved_clip(&s.project.project)
                    .unwrap()
                    .unwrap()
                    .id,
                original
            );
        }
        s.animate_insert_keyframe().unwrap();
        assert_eq!(s.project.project.animations.len(), 1);
        if redo {
            assert_eq!(s.session.animate.active_clip, Some(original));
        }
    }
}
