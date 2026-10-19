//! Editar pivô como sessão: liga o Mover, Enter confirma, Esc desfaz exatamente o
//! que a sessão fez, trocar de ferramenta confirma e objetos recusados explicam.

use glam::Vec3;
use petunia_core::AppState;
use petunia_editor::{PlaceholderViewport, SlintUiBridge, UiIntent};
use petunia_mesh::Mesh;

fn bridge() -> SlintUiBridge<PlaceholderViewport> {
    let mut b = SlintUiBridge::new(AppState::new("en"), PlaceholderViewport::default());
    b.resize_viewport(800, 600);
    b.state.project.add("Caixa", Mesh::cube(2.0));
    b.state.project.active = b.state.project.assets.len() - 1;
    // Longe da origem do mundo: "pivô na origem" precisa ser uma mudança de verdade.
    let asset = b.state.project.active_mut().expect("ativo");
    let mut transform = asset.transform;
    transform.translation = [2.0, 0.0, 0.0];
    asset.set_object_transform(transform);
    b
}

fn origin(b: &SlintUiBridge<PlaceholderViewport>) -> Vec3 {
    b.state.project.active().expect("ativo").origin_world()
}

fn world(b: &SlintUiBridge<PlaceholderViewport>) -> Vec<Vec3> {
    b.state.project.active().expect("ativo").world_positions()
}

fn depth(b: &SlintUiBridge<PlaceholderViewport>) -> usize {
    b.state.project.undo.depth().0
}

fn tool(b: &SlintUiBridge<PlaceholderViewport>) -> String {
    b.state.session.tools.active_tool.clone()
}

#[test]
fn entering_the_mode_turns_on_the_move_tool_and_remembers_the_previous_one() {
    let mut b = bridge();
    b.apply(UiIntent::SetActiveTool("rotate".into()));
    assert!(b.toggle_edit_pivot());
    assert!(b.state.session.edit_pivot);
    assert_eq!(tool(&b), "move");
    let session = b.state.session.edit_pivot_session.clone().expect("sessão");
    assert_eq!(session.previous_tool, "rotate");
    assert_eq!(session.undo_depth, depth(&b));
}

#[test]
fn enter_keeps_the_pivot_and_restores_the_tool() {
    let mut b = bridge();
    b.apply(UiIntent::SetActiveTool("rotate".into()));
    let before = world(&b);
    assert!(b.toggle_edit_pivot());
    assert!(b.set_origin_world());
    assert!(b.confirm_edit_pivot());
    assert!(!b.state.session.edit_pivot);
    assert!(b.state.session.edit_pivot_session.is_none());
    assert_eq!(tool(&b), "rotate", "a ferramenta anterior volta");
    assert!(origin(&b).length() < 1.0e-4, "o pivô ficou onde foi posto");
    assert!(
        world(&b)
            .iter()
            .zip(&before)
            .all(|(a, c)| (*a - *c).length() < 1.0e-4)
    );
}

#[test]
fn escape_undoes_exactly_what_the_session_did() {
    let mut b = bridge();
    let start_origin = origin(&b);
    let start_world = world(&b);
    let start_depth = depth(&b);
    assert!(b.toggle_edit_pivot());
    assert!(b.state.set_origin_point(Vec3::new(1.0, 1.0, 1.0)).is_ok());
    assert!(b.state.set_origin_point(Vec3::new(-2.0, 0.5, 0.0)).is_ok());
    assert_eq!(depth(&b), start_depth + 2);
    assert!(b.cancel_edit_pivot());
    assert!(!b.state.session.edit_pivot);
    assert_eq!(
        depth(&b),
        start_depth,
        "só os passos da sessão foram desfeitos"
    );
    assert!((origin(&b) - start_origin).length() < 1.0e-4);
    assert!(
        world(&b)
            .iter()
            .zip(&start_world)
            .all(|(a, c)| (*a - *c).length() < 1.0e-4)
    );
    // Os passos ficam no Redo.
    assert!(b.state.redo());
}

#[test]
fn escape_never_undoes_work_from_before_the_session() {
    let mut b = bridge();
    assert!(b.set_origin_world());
    let before_session = depth(&b);
    assert!(b.toggle_edit_pivot());
    assert!(b.cancel_edit_pivot());
    assert_eq!(depth(&b), before_session);
    assert!(origin(&b).length() < 1.0e-4);
}

#[test]
fn choosing_another_tool_confirms_the_pivot() {
    let mut b = bridge();
    assert!(b.toggle_edit_pivot());
    assert!(b.set_origin_world());
    b.apply(UiIntent::SetActiveTool("rotate".into()));
    assert!(!b.state.session.edit_pivot);
    assert!(b.state.session.edit_pivot_session.is_none());
    assert_eq!(tool(&b), "rotate");
    assert!(origin(&b).length() < 1.0e-4, "o pivô foi mantido");
}

#[test]
fn toggling_again_confirms() {
    let mut b = bridge();
    assert!(b.toggle_edit_pivot());
    assert!(b.toggle_edit_pivot());
    assert!(!b.state.session.edit_pivot);
}

#[test]
fn a_locked_object_refuses_to_enter_and_says_why() {
    let mut b = bridge();
    b.state.project.active_mut().expect("ativo").locked = true;
    assert!(!b.toggle_edit_pivot());
    assert!(!b.state.session.edit_pivot);
    assert!(b.state.session.edit_pivot_session.is_none());
    assert!(
        b.state.ui.status.to_lowercase().contains("locked"),
        "status: {}",
        b.state.ui.status
    );
}

#[test]
fn cancelling_without_a_session_does_nothing() {
    let mut b = bridge();
    assert!(!b.cancel_edit_pivot());
    assert!(!b.confirm_edit_pivot());
}

#[test]
fn origin_at_a_screen_point_keeps_the_geometry_and_lands_on_something() {
    let mut b = bridge();
    let before = world(&b);
    let start = origin(&b);
    assert!(b.set_origin_at_screen_point(0.5, 0.5));
    assert!(origin(&b).is_finite());
    assert!(
        (origin(&b) - start).length() > 1.0e-3,
        "o pivô mudou de lugar"
    );
    assert!(
        world(&b)
            .iter()
            .zip(&before)
            .all(|(a, c)| (*a - *c).length() < 1.0e-3)
    );
    assert!(!b.set_origin_at_screen_point(f32::NAN, 0.5));
}

#[test]
fn the_cursor_follows_the_selection_and_the_object_pivot() {
    let mut b = bridge();
    {
        let mesh = b.state.project.active_mesh_mut().expect("malha");
        mesh.deselect_all();
        mesh.verts[0].selected = true;
    }
    let vertex = {
        let asset = b.state.project.active().expect("ativo");
        asset.to_world_point(asset.mesh.verts[0].vec())
    };
    assert!(b.cursor_to_selection());
    assert!((Vec3::from(b.state.session.cursor_3d) - vertex).length() < 1.0e-4);
    assert!(b.cursor_to_active());
    assert!((Vec3::from(b.state.session.cursor_3d) - origin(&b)).length() < 1.0e-4);
}

#[test]
fn a_hidden_object_keeps_its_gizmo_but_a_locked_one_does_not() {
    let mut b = bridge();
    b.apply(UiIntent::SetActiveTool("move".into()));
    b.state.project.active_mut().expect("ativo").visible = false;
    assert!(
        b.view_model().gizmo.visible,
        "oculto continua transformável: o gizmo aparece"
    );
    b.state.project.active_mut().expect("ativo").locked = true;
    assert!(!b.view_model().gizmo.visible, "bloqueado não oferece alças");
}
