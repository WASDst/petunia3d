//! C8 — modos das operações com vários elementos (extrusão/inset).
//!
//! O modo é escolhido no HUD da ferramenta: o editor publica a lista de modos
//! e o modo atual, e a escolha muda a ferramenta (que é quem manda na
//! topologia da extrusão).

use petunia_core::{AppState, EditMode, SelectionDomain};
use petunia_editor::{PlaceholderViewport, SlintUiBridge, ToolModalKind};

fn bridge_with_faces() -> SlintUiBridge<PlaceholderViewport> {
    let mut b = SlintUiBridge::new(AppState::new("en"), PlaceholderViewport::default());
    b.state.set_edit_mode(EditMode::Edit);
    b.state.set_selection_domain(SelectionDomain::Face);
    // Duas faces selecionadas: a extrusão precisa de seleção de faces.
    {
        let mesh = b.state.project.active_mesh_mut().expect("malha padrão");
        for face in mesh.faces.iter_mut().take(2) {
            face.selected = true;
        }
    }
    b.state.sync_selection();
    b
}

#[test]
fn the_extrude_publishes_its_three_modes() {
    let mut b = bridge_with_faces();
    assert!(b.begin_tool_modal(ToolModalKind::Extrude), "a sessão abre");
    let vm = b.view_model();
    let modes: Vec<&str> = vm
        .tool_modal_modes
        .iter()
        .map(|mode| mode.id.as_str())
        .collect();
    assert_eq!(modes, vec!["region", "individual", "along_normals"]);
    assert_eq!(vm.tool_modal_mode, "region");
    // Os rótulos vêm traduzidos (nunca vazios).
    assert!(
        vm.tool_modal_modes
            .iter()
            .all(|mode| !mode.label.is_empty())
    );
}

#[test]
fn the_inset_publishes_its_two_modes() {
    let mut b = bridge_with_faces();
    assert!(b.begin_tool_modal(ToolModalKind::Inset), "a sessão abre");
    let vm = b.view_model();
    let modes: Vec<&str> = vm
        .tool_modal_modes
        .iter()
        .map(|mode| mode.id.as_str())
        .collect();
    assert_eq!(modes, vec!["region", "individual"]);
    assert_eq!(vm.tool_modal_mode, "region");
}

#[test]
fn the_mode_follows_the_tool_and_is_remembered() {
    let mut b = bridge_with_faces();
    assert!(b.begin_tool_modal(ToolModalKind::Extrude), "a sessão abre");
    assert!(b.set_operation_mode("individual"), "o modo é aceito");
    assert_eq!(b.view_model().tool_modal_mode, "individual");
    assert_eq!(
        b.state.tools.extrude_mode,
        petunia_mesh::ExtrudeMode::Individual
    );
    // A escolha é recusada quando não existe (sem efeito silencioso).
    assert!(!b.set_operation_mode("inexistente"));
    assert_eq!(b.view_model().tool_modal_mode, "individual");
}

#[test]
fn operations_without_a_mode_publish_nothing() {
    let mut b = bridge_with_faces();
    assert!(b.begin_tool_modal(ToolModalKind::Bevel), "a sessão abre");
    let vm = b.view_model();
    assert!(vm.tool_modal_modes.is_empty(), "bevel não tem modo");
    assert!(vm.tool_modal_mode.is_empty());
}
