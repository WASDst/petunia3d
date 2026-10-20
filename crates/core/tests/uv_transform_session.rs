use glam::Vec2;
use petunia_core::{AppState, UvTransformKind};
use petunia_mesh::UvTransform;

fn state() -> AppState {
    let mut s = AppState::default();
    let mesh = s.project.active_mesh().unwrap().clone();
    s.session.uv_selection.sync = false;
    s.session.uv_selection.set_faces(&mesh, [0]);
    s.project
        .active_mesh_mut()
        .unwrap()
        .uv_pinned
        .insert((0, 0));
    s
}
fn uv(s: &AppState) -> Vec<Vec<[f32; 2]>> {
    s.project
        .active_mesh()
        .unwrap()
        .faces
        .iter()
        .map(|f| f.uv.clone())
        .collect()
}
#[test]
fn preview_is_absolute_and_document_is_unchanged_until_single_commit() {
    let mut s = state();
    let source = uv(&s);
    let verts: Vec<_> = s
        .project
        .active_mesh()
        .unwrap()
        .verts
        .iter()
        .map(|v| v.pos)
        .collect();
    let depth = s.project.undo.depth();
    s.begin_uv_transform(UvTransformKind::Move).unwrap();
    for _ in 0..30 {
        s.update_uv_transform(UvTransform::Translate(Vec2::new(0.2, 0.3)))
            .unwrap();
    }
    assert_eq!(uv(&s), source);
    let preview = s
        .session
        .tools
        .uv_transform
        .as_ref()
        .unwrap()
        .preview_mesh();
    assert!((preview.faces[0].uv[0][0] - source[0][0][0] - 0.2).abs() < 1e-6);
    assert_eq!(s.project.undo.depth(), depth);
    s.commit_uv_transform().unwrap();
    assert_eq!(s.project.undo.depth().0, depth.0 + 1);
    assert_ne!(uv(&s), source);
    assert_eq!(uv(&s)[1..], source[1..]);
    assert_eq!(
        s.project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .map(|v| v.pos)
            .collect::<Vec<_>>(),
        verts
    );
    assert!(s.project.active_mesh().unwrap().uv_pinned.contains(&(0, 0)));
    assert!(!s.session.uv_selection.sync);
    assert!(s.undo());
    assert_eq!(uv(&s), source);
    assert!(s.redo());
    assert_ne!(uv(&s), source);
}
#[test]
fn cancel_and_undo_discard_only_the_preview_and_keep_history_and_document() {
    for undo in [false, true] {
        let mut s = state();
        let source = uv(&s);
        let depth = s.project.undo.depth();
        s.begin_uv_transform(UvTransformKind::Rotate).unwrap();
        s.update_uv_transform(UvTransform::Rotate(0.7)).unwrap();
        if undo {
            assert!(s.undo());
        } else {
            assert!(s.cancel_uv_transform());
        }
        assert_eq!(uv(&s), source);
        assert_eq!(s.project.undo.depth(), depth);
        assert!(s.session.tools.uv_transform.is_none());
    }
}
#[test]
fn stale_object_revision_and_selection_refuse_commit_without_overwriting_external_change() {
    for cause in [0, 1, 2] {
        let mut s = state();
        s.begin_uv_transform(UvTransformKind::Move).unwrap();
        s.update_uv_transform(UvTransform::Translate(Vec2::splat(0.5)))
            .unwrap();
        match cause {
            0 => {
                s.project.active_mut().unwrap().id = uuid::Uuid::new_v4();
            }
            1 => {
                s.project.active_mesh_mut().unwrap().faces[0].uv[0] = [0.8, 0.9];
            }
            _ => {
                s.session.uv_selection.clear();
            }
        }
        let before = uv(&s);
        let depth = s.project.undo.depth();
        assert!(s.commit_uv_transform().is_err());
        assert_eq!(uv(&s), before);
        assert_eq!(s.project.undo.depth(), depth);
        assert!(s.cancel_uv_transform());
        assert_eq!(uv(&s), before);
    }
}
#[test]
fn invalid_input_does_not_destroy_the_last_valid_preview() {
    let mut s = state();
    s.begin_uv_transform(UvTransformKind::Scale).unwrap();
    s.update_uv_transform(UvTransform::Scale(Vec2::splat(1.5)))
        .unwrap();
    let before: Vec<_> = s
        .session
        .tools
        .uv_transform
        .as_ref()
        .unwrap()
        .preview_mesh()
        .faces
        .iter()
        .map(|f| f.uv.clone())
        .collect();
    assert!(
        s.update_uv_transform(UvTransform::Scale(Vec2::splat(f32::INFINITY)))
            .is_err()
    );
    assert!(s.update_uv_transform(UvTransform::Rotate(0.5)).is_err());
    let after: Vec<_> = s
        .session
        .tools
        .uv_transform
        .as_ref()
        .unwrap()
        .preview_mesh()
        .faces
        .iter()
        .map(|f| f.uv.clone())
        .collect();
    assert_eq!(before, after);
}
