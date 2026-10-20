//! Ferramenta Medir: dois cliques com prévia, gravação com Undo, Esc cancela,
//! medidas gravadas desenhadas e `measure.clear`.

use glam::Vec3;
use petunia_core::AppState;
use petunia_editor::{PlaceholderViewport, SlintUiBridge, UiIntent};

fn bridge() -> SlintUiBridge<PlaceholderViewport> {
    let mut b = SlintUiBridge::new(AppState::new("en"), PlaceholderViewport::default());
    b.resize_viewport(800, 600);
    b.apply(UiIntent::SetActiveTool("measure".into()));
    b
}

fn stored(b: &SlintUiBridge<PlaceholderViewport>) -> usize {
    b.state.project.measurements.len()
}

#[test]
fn the_first_click_only_marks_a_point() {
    let mut b = bridge();
    assert!(!b.view_model().measure_pending);
    assert!(b.measure_click(0.3, 0.6));
    assert!(b.view_model().measure_pending, "espera o segundo ponto");
    assert_eq!(stored(&b), 0, "nada gravado ainda");
}

#[test]
fn the_preview_follows_the_pointer_and_shows_the_distance() {
    let mut b = bridge();
    assert!(b.measure_click(0.3, 0.6));
    assert!(
        !b.view_model().measure_visible,
        "sem segundo ponto não há linha"
    );
    assert!(b.measure_hover(0.7, 0.6));
    let vm = b.view_model();
    assert!(vm.measure_visible);
    assert!(vm.measure_distance > 0.0);
    assert!(!vm.measure_commands.is_empty());
    assert_eq!(stored(&b), 0, "prévia não grava");
    // Mover o mouse mais longe aumenta a distância.
    assert!(b.measure_hover(0.9, 0.6));
    assert!(b.view_model().measure_distance > vm.measure_distance);
}

#[test]
fn the_second_click_stores_the_measurement_with_one_undo_step() {
    let mut b = bridge();
    let depth = b.state.project.undo.depth().0;
    assert!(b.measure_click(0.3, 0.6));
    assert!(b.measure_hover(0.7, 0.6));
    let preview = b.view_model().measure_distance;
    assert!(b.measure_click(0.7, 0.6));
    assert_eq!(stored(&b), 1);
    assert!(
        !b.view_model().measure_pending,
        "pronto para a próxima medida"
    );
    let item = &b.state.project.measurements[0];
    let expected = Vec3::from(item.start).distance(Vec3::from(item.end));
    assert!((item.distance - expected).abs() < 1.0e-4);
    assert!(
        (item.distance - preview).abs() < 1.0e-3,
        "gravou o que a prévia mostrava: {} vs {preview}",
        item.distance
    );
    assert_eq!(
        b.state.project.undo.depth().0,
        depth + 1,
        "um passo de Undo"
    );
    assert!(b.state.undo());
    assert_eq!(stored(&b), 0);
}

#[test]
fn stored_measurements_stay_drawn_with_their_tag() {
    let mut b = bridge();
    assert!(b.measure_click(0.3, 0.6));
    assert!(b.measure_click(0.7, 0.6));
    let vm = b.view_model();
    assert!(vm.measure_visible);
    assert!(!vm.measure_commands.is_empty(), "o traço gravado aparece");
    assert_eq!(vm.measure_tags.len(), 1);
    assert!(vm.measure_tags[0].text.ends_with('m'));
    // Oculta pelo projeto ou pelo item: some da vista, continua gravada.
    b.state.project.project.measurements_visible = false;
    assert!(b.view_model().measure_tags.is_empty());
    b.state.project.project.measurements_visible = true;
    b.state.project.project.measurements[0].visible = false;
    assert!(b.view_model().measure_tags.is_empty());
    assert_eq!(stored(&b), 1);
}

#[test]
fn clicking_the_same_point_twice_keeps_waiting() {
    let mut b = bridge();
    assert!(b.measure_click(0.5, 0.5));
    assert!(!b.measure_click(0.5, 0.5), "mesmo ponto: recusa");
    assert_eq!(stored(&b), 0);
    assert!(
        b.view_model().measure_pending,
        "o primeiro ponto não se perde"
    );
    assert!(b.measure_click(0.8, 0.5));
    assert_eq!(stored(&b), 1);
}

#[test]
fn escape_discards_only_the_measurement_in_progress() {
    let mut b = bridge();
    assert!(b.measure_click(0.3, 0.6));
    assert!(b.measure_click(0.7, 0.6));
    assert!(b.measure_click(0.4, 0.4));
    assert!(b.view_model().measure_pending);
    assert!(b.cancel_measure());
    assert!(!b.view_model().measure_pending);
    assert_eq!(stored(&b), 1, "a gravada fica");
    assert!(!b.cancel_measure(), "nada a cancelar");
}

#[test]
fn non_finite_pointer_positions_are_ignored() {
    let mut b = bridge();
    assert!(!b.measure_click(f32::NAN, 0.5));
    assert!(!b.view_model().measure_pending);
    assert!(b.measure_click(0.4, 0.5));
    assert!(!b.measure_hover(f32::NAN, 0.5));
}

#[test]
fn measure_clear_removes_everything_with_one_undo_step_and_refuses_when_empty() {
    let mut b = bridge();
    assert!(
        b.state.dispatch_command("measure.clear").is_err(),
        "sem medidas, recusa"
    );
    for x in [0.3, 0.4] {
        assert!(b.measure_click(x, 0.6));
        assert!(b.measure_click(x + 0.3, 0.6));
    }
    assert_eq!(stored(&b), 2);
    let depth = b.state.project.undo.depth().0;
    b.state.dispatch_command("measure.clear").expect("limpa");
    assert_eq!(stored(&b), 0);
    assert_eq!(b.state.project.undo.depth().0, depth + 1);
    assert!(b.state.undo());
    assert_eq!(stored(&b), 2, "desfazer traz as duas de volta");
}
