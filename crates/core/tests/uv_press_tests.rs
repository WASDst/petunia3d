//! Mover várias ilhas de uma vez: pressionar num item já selecionado mantém a
//! multisseleção (para o arrasto mover todos) e pressionar no vazio não limpa
//! (o soltar decide: clique limpa, arrasto seleciona por caixa).

use glam::Vec2;
use petunia_core::ProjectService;
use petunia_core::state::AppState;

const RADIUS: f32 = 6.0 / 256.0;

/// Projeto novo com o cubo padrão e duas faces escolhidas para a seleção UV.
fn scene() -> (AppState, [usize; 2], [Vec2; 2]) {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let mesh = state.project.active_mesh().expect("malha ativa");
    let centroid = |face: usize| {
        let uv = &mesh.faces[face].uv;
        assert!(!uv.is_empty(), "a face {face} tem UV");
        let n = uv.len() as f32;
        Vec2::new(
            uv.iter().map(|p| p[0]).sum::<f32>() / n,
            uv.iter().map(|p| p[1]).sum::<f32>() / n,
        )
    };
    // Duas faces cujo centro UV não cai dentro da outra (cubo com UV por face).
    let (first, second) = (0, 2);
    let points = [centroid(first), centroid(second)];
    assert!(state.uv_select_faces(&[first, second], false));
    (state, [first, second], points)
}

fn selected_faces(state: &AppState) -> usize {
    let mesh = state.project.active_mesh().expect("malha");
    state.session.uv_selection.faces(mesh).len()
}

#[test]
fn pressing_a_selected_item_keeps_the_whole_selection() {
    let (mut state, _, points) = scene();
    assert_eq!(selected_faces(&state), 2);
    assert!(
        state.uv_press(points[0], RADIUS, false),
        "acertou um item selecionado"
    );
    assert_eq!(selected_faces(&state), 2, "a multisseleção foi mantida");
}

#[test]
fn pressing_an_unselected_item_selects_only_it() {
    let (mut state, [first, _], points) = scene();
    // Troca a seleção para só a segunda face e pressiona a primeira.
    assert!(state.uv_select_faces(&[2], false));
    assert!(state.uv_press(points[0], RADIUS, false));
    let mesh = state.project.active_mesh().expect("malha");
    let faces = state.session.uv_selection.faces(mesh);
    assert!(faces.contains(&first));
    assert!(!faces.contains(&2), "o alvo novo substituiu a seleção");
}

#[test]
fn pressing_empty_space_does_not_clear_the_selection() {
    let (mut state, _, _) = scene();
    assert!(!state.uv_press(Vec2::new(5.0, 5.0), RADIUS, false));
    assert_eq!(selected_faces(&state), 2, "o soltar é que decide");
    // O clique (soltar sem arrastar) é que limpa.
    state.uv_click(Vec2::new(5.0, 5.0), RADIUS, false);
    assert_eq!(selected_faces(&state), 0);
}

#[test]
fn shift_press_toggles_like_a_click() {
    let (mut state, _, points) = scene();
    assert!(state.uv_press(points[0], RADIUS, true));
    assert_eq!(
        selected_faces(&state),
        1,
        "Shift em item selecionado o tira"
    );
}

#[test]
fn box_select_through_the_state_reaches_the_3d_selection_when_synced() {
    let (mut state, _, _) = scene();
    assert!(state.uv_box_select(Vec2::splat(-10.0), Vec2::splat(10.0), false, false));
    let mesh = state.project.active_mesh().expect("malha");
    assert_eq!(
        state.session.uv_selection.faces(mesh).len(),
        mesh.faces.iter().filter(|face| !face.uv.is_empty()).count(),
        "uma caixa enorme pega todas as faces com UV"
    );
}
