use petunia_core::{AppState, BrushSettings, BrushType, FillScope};
use petunia_module_paint::PaintModule;
use petunia_project::Canvas;

fn state() -> AppState {
    let mut state = AppState::default();
    let asset = state.project.active_mut().unwrap();
    asset.texture = Some(Canvas::new(16, 16, [0, 0, 0, 255]));
    for face in &mut asset.mesh.faces {
        face.selected = false;
    }
    for vert in &mut asset.mesh.verts {
        vert.selected = false;
    }
    PaintModule::ensure_stack(&mut state);
    state.paint_color = [1.0, 0.0, 0.0];
    state.session.tools.paint_color = [1.0, 0.0, 0.0];
    state
}
fn pixels(state: &AppState) -> Vec<u8> {
    state
        .project
        .active()
        .unwrap()
        .texture
        .as_ref()
        .unwrap()
        .pixels
        .to_vec()
}

#[test]
fn empty_or_invalid_scope_never_broadens_and_does_not_add_undo() {
    for (face, seed, scope) in [
        (None, Some((8, 8)), FillScope::SelectedFaces),
        (Some(usize::MAX), None, FillScope::Face),
        (None, None, FillScope::Face),
        (Some(usize::MAX), None, FillScope::UvIsland),
        (None, None, FillScope::UvIsland),
        (None, None, FillScope::ConnectedPixels),
    ] {
        let mut state = state();
        let before = pixels(&state);
        let depth = state.project.undo.depth();
        state.begin_paint_stroke();
        PaintModule::canvas_fill_scoped(&mut state, face, seed, scope);
        state.finish_paint_stroke(false);
        assert_eq!(pixels(&state), before, "{scope:?}");
        assert_eq!(state.project.undo.depth(), depth, "{scope:?}");
    }
}

#[test]
fn legacy_fill_respects_selection_mask_and_layer_lock() {
    for lock_layer in [false, true] {
        let mut state = state();
        state.session.tools.paint_isolate_selection = !lock_layer;
        state
            .project
            .active_mut()
            .unwrap()
            .paint_stack
            .as_mut()
            .unwrap()
            .active_mut()
            .unwrap()
            .locked = lock_layer;
        let before = pixels(&state);
        let depth = state.project.undo.depth();
        state.begin_paint_stroke();
        PaintModule::canvas_brush_with_settings(
            &mut state,
            8,
            8,
            BrushSettings {
                kind: BrushType::Fill,
                ..Default::default()
            },
        );
        state.finish_paint_stroke(false);
        assert_eq!(pixels(&state), before);
        assert_eq!(state.project.undo.depth(), depth);
    }
}

#[test]
fn valid_object_fill_has_one_undo_and_cancel_restores_source() {
    let mut state = state();
    let before = pixels(&state);
    let depth = state.project.undo.depth();
    state.begin_paint_stroke();
    PaintModule::canvas_fill_scoped(&mut state, None, None, FillScope::Object);
    assert_ne!(pixels(&state), before);
    state.finish_paint_stroke(true);
    assert_eq!(pixels(&state), before);
    assert_eq!(state.project.undo.depth(), depth);
    state.begin_paint_stroke();
    PaintModule::canvas_fill_scoped(&mut state, None, None, FillScope::Object);
    state.finish_paint_stroke(false);
    assert_eq!(state.project.undo.depth().0, depth.0 + 1);
    assert!(state.undo());
    assert_eq!(pixels(&state), before);
}

#[test]
fn each_valid_scope_paints_and_has_one_reversible_transaction() {
    for scope in [
        FillScope::Face,
        FillScope::SelectedFaces,
        FillScope::UvIsland,
        FillScope::ConnectedPixels,
        FillScope::Object,
    ] {
        let mut s = state();
        if scope == FillScope::SelectedFaces {
            s.project.active_mesh_mut().unwrap().faces[0].selected = true;
        }
        let before = pixels(&s);
        let depth = s.project.undo.depth();
        s.begin_paint_stroke();
        PaintModule::canvas_fill_scoped(&mut s, Some(0), Some((8, 8)), scope);
        s.finish_paint_stroke(false);
        let after = pixels(&s);
        assert_ne!(after, before, "{scope:?}");
        assert_eq!(s.project.undo.depth().0, depth.0 + 1, "{scope:?}");
        assert!(s.undo());
        assert_eq!(pixels(&s), before, "{scope:?}");
        assert!(s.redo());
        assert_eq!(pixels(&s), after, "{scope:?}");
    }
}
