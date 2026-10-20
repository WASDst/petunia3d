//! U05: ações de ponteiro do PAINT vêm do keymap `[pointer]`, inclusive Alt.
use petunia_config::keybinds::{Mods2, POINTER_CLONE_SOURCE};
use petunia_core::{AppState, PrimitiveKind, Workspace};
use petunia_editor::{PlaceholderViewport, SlintUiBridge, UiIntent};

fn paint_bridge() -> SlintUiBridge<PlaceholderViewport> {
    let mut b = SlintUiBridge::new(AppState::new("en"), PlaceholderViewport::default());
    b.apply(UiIntent::AddPrimitive(PrimitiveKind::Cube));
    b.state.confirm_primitive();
    b.resize_viewport(800, 600);
    b.apply(UiIntent::SetWorkspace(Workspace::Paint));
    assert!(b.create_albedo_texture(), "textura pintável");
    b.state.session.tools.active_tool = "clone".to_string();
    b
}

/// Mensagem do ramo "definir origem do Clone" (mesma com ou sem superfície sob
/// o cursor quando não há acerto): prova que a tecla vinculada chegou à ação.
fn reached_clone_source(b: &SlintUiBridge<PlaceholderViewport>) -> bool {
    let point_at = b
        .state
        .t_id(petunia_config::text_id::STATUS_CLONE_POINT_AT_THE_SURFACE_TO_SET);
    let set = b
        .state
        .t_id(petunia_config::text_id::STATUS_CLONE_SOURCE_SET_PAINT_TO_COPY_FROM);
    b.state.ui.status == point_at || b.state.ui.status == set
}

#[test]
fn clone_source_rebound_to_alt_is_honoured_and_old_key_no_longer_sets_it() {
    let mut b = paint_bridge();
    b.state.ui.keybinds.set_pointer_modifier(
        POINTER_CLONE_SOURCE,
        Mods2 {
            alt: true,
            ..Mods2::default()
        },
    );
    // Com uma origem já definida, só a ação "origem do Clone" a redefine.
    b.state.session.tools.clone_source = Some([1.0, 1.0]);

    b.state.ui.status.clear();
    b.begin_paint_stroke_with_modifiers(400.0, 300.0, false, true, false);
    b.end_paint_stroke_at(400.0, 300.0);
    assert!(
        !reached_clone_source(&b),
        "Ctrl (padrão antigo) não é mais a ação"
    );

    b.state.ui.status.clear();
    b.begin_paint_stroke_with_modifiers(400.0, 300.0, false, false, true);
    assert!(
        reached_clone_source(&b),
        "Alt vinculado chega à ação: {:?}",
        b.state.ui.status
    );
}
