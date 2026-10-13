//! Geometria de linhas que precisam preencher a largura (auditoria de 2026-10-11).
//!
//! Em Slint 1.18, qualquer `alignment` de box layout diferente de `stretch`
//! dimensiona os filhos pelo tamanho preferido e ignora `horizontal-stretch`
//! (`i-slint-core` `solve_box_layout`). Com `alignment: center`, o campo de
//! busca encolhia, sliders ficavam no `min-width` e botões à direita migravam
//! para o meio da linha. Cada teste mede uma relação de borda que só vale
//! quando o filho elástico de fato preenche. Gate headless: não substitui
//! inspeção visual nem leitor de tela nativo.

use std::{cell::RefCell, rc::Rc, time::Duration};

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use petunia_ui_slint::PetuniaSlintShell;
use slint::{ComponentHandle, LogicalSize, ModelRc, VecModel};

fn key(shell: &PetuniaSlintShell, key: impl Into<slint::SharedString>) {
    let text: slint::SharedString = key.into();
    shell
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyPressed { text: text.clone() });
    shell
        .window()
        .dispatch_event(slint::platform::WindowEvent::KeyReleased { text });
    settle();
}

/// Arredondamento de layout e bordas de 1 px.
const TOLERANCE: f32 = 1.5;

fn shell(workspace: &str, height: f32) -> PetuniaSlintShell {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, height));
    shell.set_active_workspace(workspace.into());
    shell.global::<petunia_ui_slint::Motion>().set_reduced(true);
    shell.show().expect("headless window");
    shell
}

fn model_shell() -> PetuniaSlintShell {
    let shell = shell("MODEL", 1200.0);
    shell.set_object_has_selection(true);
    shell.set_selection_domain("OBJECT".into());
    shell
}

/// Deixa bindings de Timer e transições (já reduzidas) assentarem.
fn settle() {
    for _ in 0..3 {
        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(16));
    }
}

fn tr(key: &str) -> String {
    petunia_ui_slint::tr::lookup(key)
}

fn element(shell: &PetuniaSlintShell, label: &str, role: AccessibleRole) -> ElementHandle {
    ElementHandle::find_by_accessible_label(shell, label)
        .find(|item| {
            item.accessible_role() == Some(role)
                && item.computed_opacity() > 0.05
                && item.size().width > 0.0
        })
        .unwrap_or_else(|| panic!("visible {role:?}: {label}"))
}

fn left(item: &ElementHandle) -> f32 {
    item.absolute_position().x
}

fn right(item: &ElementHandle) -> f32 {
    item.absolute_position().x + item.size().width
}

/// Instância visível de `type_name` cuja caixa contém `child`.
fn container_of(
    shell: &PetuniaSlintShell,
    type_name: &str,
    child: &ElementHandle,
) -> ElementHandle {
    let (cx, cy) = (child.absolute_position().x, child.absolute_position().y);
    ElementHandle::find_by_element_type_name(shell, type_name)
        .find(|item| {
            let origin = item.absolute_position();
            let size = item.size();
            size.width > 0.0
                && cx >= origin.x - TOLERANCE
                && cx <= origin.x + size.width
                && cy >= origin.y - TOLERANCE
                && cy <= origin.y + size.height
        })
        .unwrap_or_else(|| panic!("{type_name} containing the measured child"))
}

fn visible_of_type(shell: &PetuniaSlintShell, type_name: &str) -> ElementHandle {
    ElementHandle::find_by_element_type_name(shell, type_name)
        .find(|item| item.size().width > 0.0 && item.computed_opacity() > 0.05)
        .unwrap_or_else(|| panic!("visible {type_name}"))
}

/// Descendente de `container` com o rótulo acessível `label`.
fn inside(container: &ElementHandle, label: &str) -> ElementHandle {
    let label = label.to_string();
    container
        .query_descendants()
        .match_predicate(move |item| item.accessible_label().is_some_and(|l| l == label))
        .find_first()
        .expect("labelled descendant")
}

#[track_caller]
fn near(actual: f32, expected: f32, what: &str) {
    assert!(
        (actual - expected).abs() <= TOLERANCE,
        "{what}: {actual:.1} px, esperado {expected:.1} px"
    );
}

#[test]
fn model_parts_search_and_row_size_fill_their_rows() {
    let shell = model_shell();
    shell.set_model_parts_open(true);
    shell.set_label_search_parts("Search model parts".into());
    shell.set_label_selected_parts_only("Model selected only".into());
    shell.set_label_parts_row_size("Model row size".into());
    shell.set_parts_items(ModelRc::new(VecModel::from(vec![
        petunia_ui_slint::SceneItem {
            id: "part-a".into(),
            name: "Part A".into(),
            visible: true,
            ..Default::default()
        },
    ])));
    settle();

    // PartsFilterRow: a caixa de busca começa na borda e ocupa a sobra da linha.
    let search = element(&shell, "Search model parts", AccessibleRole::Search);
    let input = element(&shell, "Search model parts", AccessibleRole::TextInput);
    let row = container_of(&shell, "PartsFilterRow", &search);
    near(left(&search), left(&row), "busca encostada à esquerda");
    assert!(
        search.size().width > row.size().width * 0.5,
        "busca ocupa a sobra: {} de {}",
        search.size().width,
        row.size().width
    );
    // Padding 7+7, ícone 13, espaçamento 5: o resto é área de digitação.
    near(
        input.size().width,
        search.size().width - 32.0,
        "TextInput preenche a caixa de busca",
    );
    let only = element(&shell, "Model selected only", AccessibleRole::Button);
    near(
        left(&only),
        right(&search) + 6.0,
        "filtros logo após a busca",
    );

    // PartsRowSizeControl: slider até o valor de 24 px na borda direita.
    let slider = element(&shell, "Model row size", AccessibleRole::Slider);
    let row = container_of(&shell, "PartsRowSizeControl", &slider);
    near(
        right(&slider) + 6.0 + 24.0,
        right(&row),
        "slider preenche a linha",
    );
}

#[test]
fn model_material_actions_stay_on_the_right_edge() {
    let shell = model_shell();
    shell.set_model_material_open(true);
    shell.set_label_material_new("Create model material".into());
    shell.set_label_material_remove("Remove model material".into());
    shell.set_label_material_create_texture("Create albedo".into());
    shell.set_material_slots(ModelRc::new(VecModel::from(vec![
        "Material A".into(),
        "Material B".into(),
    ])));
    settle();

    let remove = element(&shell, "Remove model material", AccessibleRole::Button);
    let body = container_of(&shell, "MaterialBody", &remove);
    near(right(&remove), right(&body), "Remover na borda direita");
    let create = element(&shell, "Create model material", AccessibleRole::Button);
    near(
        right(&create) + 2.0 * (6.0 + 26.0),
        right(&body),
        "Novo/Duplicar/Remover agrupados à direita",
    );

    // Botão da textura albedo: nomeado e com alvo operacional de 28 px.
    let albedo = element(&shell, "Create albedo", AccessibleRole::Button);
    near(right(&albedo) + 8.0, right(&body), "botão albedo na borda");
    assert!(albedo.size().height >= 28.0, "alvo mínimo de 28 px");

    // Lista de slots: o nome começa logo após o marcador, não no centro.
    let slot = ElementHandle::find_by_accessible_label(&shell, "Material B")
        .find(|item| item.accessible_role() == Some(AccessibleRole::Text))
        .expect("slot row text");
    near(
        left(&slot),
        left(&body) + 8.0 + 8.0 + 6.0,
        "nome do slot à esquerda",
    );

    // Foco por clique permanece no controle e aciona o mesmo callback por teclado.
    let edits = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&edits);
    shell.on_material_albedo_create(move || observed.borrow_mut().push("create"));
    let observed = Rc::clone(&edits);
    shell.on_material_albedo_clear(move || observed.borrow_mut().push("clear"));
    albedo.mock_single_click(slint::platform::PointerEventButton::Left);
    key(&shell, slint::platform::Key::Return);
    key(&shell, " ");
    assert_eq!(&*edits.borrow(), &["create", "create", "create"]);
    shell.set_label_material_clear_texture("Clear albedo".into());
    shell.set_material_has_albedo(true);
    settle();
    let clear = element(&shell, "Clear albedo", AccessibleRole::Button);
    clear.invoke_accessible_default_action();
    key(&shell, slint::platform::Key::Return);
    assert_eq!(
        &*edits.borrow(),
        &["create", "create", "create", "clear", "clear"]
    );
}

#[test]
fn transform_fields_and_inspector_header_fill_the_flyout() {
    let shell = model_shell();
    shell.set_model_transform_open(true);
    shell.set_label_duplicate("Duplicate object".into());
    shell.set_label_delete("Delete object".into());
    settle();

    // Vector3Field: X/Y/Z dividem a sobra depois do rótulo de 52 px.
    let position = tr("sl.position");
    let x = element(&shell, &format!("{position} X"), AccessibleRole::Spinbox);
    let y = element(&shell, &format!("{position} Y"), AccessibleRole::Spinbox);
    let z = element(&shell, &format!("{position} Z"), AccessibleRole::Spinbox);
    let row = container_of(&shell, "Vector3Field", &x);
    near(left(&x), left(&row) + 52.0 + 6.0, "X logo após o rótulo");
    near(right(&z), right(&row), "Z na borda direita");
    near(x.size().width, y.size().width, "X e Y com a mesma largura");
    near(y.size().width, z.size().width, "Y e Z com a mesma largura");

    // NumericField em edição: o TextInput ocupa o campo, não só o texto.
    x.mock_single_click(slint::platform::PointerEventButton::Left);
    settle();
    let editor = x
        .query_descendants()
        .match_accessible_role(AccessibleRole::TextInput)
        .find_first()
        .expect("click opens the numeric editor");
    // Padding 6+6, rótulo 10, espaçamento 4.
    near(
        editor.size().width,
        x.size().width - 26.0,
        "editor numérico preenche o campo",
    );

    // Cabeçalho do Inspector: Duplicar/Excluir fixos na borda direita,
    // sem esticar (IconButton herda stretch 1 de Rectangle).
    let delete = element(&shell, "Delete object", AccessibleRole::Button);
    let duplicate = element(&shell, "Duplicate object", AccessibleRole::Button);
    let flyout = ElementHandle::find_by_element_id(&shell, "RightColumnShell::inspector-flyout")
        .find(|item| item.size().width > 0.0)
        .expect("inspector flyout");
    near(
        right(&delete) + 8.0,
        right(&flyout),
        "Excluir na borda direita",
    );
    near(
        right(&duplicate) + 6.0,
        left(&delete),
        "Duplicar ao lado de Excluir",
    );
    near(
        delete.size().width,
        delete.size().height,
        "Excluir continua quadrado",
    );
}

#[test]
fn paint_brush_rows_share_one_right_edge() {
    let shell = shell("PAINT", 4000.0);
    shell.set_model_parts_open(true);
    settle();

    let flow_plus = element(
        &shell,
        &(tr("sl.brush_flow") + " +"),
        AccessibleRole::Button,
    );
    let stepper = container_of(&shell, "BrushStepper", &flow_plus);
    let edge = right(&stepper);
    near(right(&flow_plus), edge, "BrushStepper: + na borda direita");

    // Tamanho/Opacidade/Dureza agora são BrushStepper com nome acessível.
    for key in ["sl.size", "sl.opacity", "sl.hardness"] {
        let plus = element(&shell, &(tr(key) + " +"), AccessibleRole::Button);
        near(right(&plus), edge, key);
    }
    let symmetry_z = element(
        &shell,
        &(tr("sl.symmetry") + " Z"),
        AccessibleRole::Checkbox,
    );
    near(right(&symmetry_z), edge, "Simetria Z na borda");
    let vertex = element(
        &shell,
        &format!("{} · {}", tr("sl.target"), tr("sl.vertex")),
        AccessibleRole::RadioButton,
    );
    near(right(&vertex), edge, "Alvo Vértice na borda");
    let mask = element(&shell, &tr("sl.mask_selection"), AccessibleRole::Checkbox);
    near(right(&mask), edge, "Máscara na borda");

    // Camada albedo: ícone e nome à esquerda, "Ativa" na borda (padding 6).
    shell.set_label_albedo_base_color("Albedo base layer".into());
    settle();
    let inspector = visible_of_type(&shell, "PaintInspector");
    let base = inside(&inspector, "Albedo base layer");
    near(
        left(&base),
        left(&stepper) + 6.0 + 14.0 + 6.0,
        "nome após o ícone",
    );
}

#[test]
fn tool_rail_keeps_the_asset_library_at_the_bottom() {
    let shell = model_shell();
    shell.set_label_asset_library("Asset library".into());
    settle();

    let assets = element(&shell, "Asset library", AccessibleRole::Button);
    let rail = container_of(&shell, "ToolRail", &assets);
    let bottom = |item: &ElementHandle| item.absolute_position().y + item.size().height;
    // O espaçador vertical empurra a biblioteca para o fim (padding 8).
    near(
        bottom(&assets),
        bottom(&rail) - 8.0,
        "biblioteca no fim do trilho",
    );
}

#[test]
fn command_palette_rows_put_the_shortcut_on_the_right() {
    let shell = model_shell();
    shell.set_command_results(ModelRc::new(VecModel::from(vec![
        petunia_ui_slint::CommandItem {
            id: "file.open".into(),
            label: "Open project".into(),
            shortcut: "Ctrl+O".into(),
            available: true,
            ..Default::default()
        },
    ])));
    shell.set_command_search_visible(true);
    settle();

    let palette = visible_of_type(&shell, "CommandPalette");
    let label = inside(&palette, "Open project");
    let shortcut = inside(&palette, "Ctrl+O");
    let row = ElementHandle::find_by_element_id(&shell, "CommandPalette::item-touch")
        .find(|item| item.size().width > 0.0)
        .expect("command result row");
    near(left(&label), left(&row) + 8.0, "rótulo à esquerda");
    near(right(&shortcut), right(&row) - 8.0, "atalho à direita");
}

#[test]
fn settings_input_sliders_fill_the_content_column() {
    let shell = model_shell();
    shell.window().set_size(LogicalSize::new(1280.0, 2400.0));
    shell.set_label_double_tap_interval("Double tap interval".into());
    shell.set_label_drag_threshold("Drag threshold".into());
    shell.set_label_snap_radius("Snap radius".into());
    shell.set_settings_tab(2);
    shell.set_settings_panel_h(2200.0);
    shell.set_settings_visible(true);
    settle();

    // Títulos ocupam a coluna; o valor tem 85 px e espaçamento 8.
    let heading = element(&shell, "Drag threshold", AccessibleRole::Text);
    for label in ["Double tap interval", "Drag threshold", "Snap radius"] {
        let slider = element(&shell, label, AccessibleRole::Slider);
        near(left(&slider), left(&heading), label);
        near(right(&slider) + 8.0 + 85.0, right(&heading), label);
    }
}

#[test]
fn reference_tile_keeps_remove_on_the_right_edge() {
    let shell = model_shell();
    shell.set_reference_slots(ModelRc::new(VecModel::from(vec![
        petunia_ui_slint::ReferenceSlotData {
            axis: "front".into(),
            title: "Front".into(),
            has_image: true,
            visible: true,
            ..Default::default()
        },
    ])));
    shell.set_reference_manager_visible(true);
    settle();

    let remove = element(&shell, &tr("sl.ref_remove"), AccessibleRole::Button);
    let lock = element(&shell, &tr("sl.lock"), AccessibleRole::Button);
    let tile = container_of(&shell, "ReferenceTile", &remove);
    // Remover (destrutivo) fica longe dos toggles, na borda (padding 8).
    near(
        right(&remove),
        right(&tile) - 8.0,
        "Remover na borda do card",
    );
    assert!(
        left(&remove) > right(&lock) + 8.0,
        "Remover não encosta em Bloquear"
    );
}

#[test]
fn status_bar_and_operation_hud_reach_their_edges() {
    let shell = model_shell();
    shell.set_operation_hud_active(true);
    shell.set_operation_hud_subject("Extrude 2 faces".into());
    settle();

    // O espaçador elástico empurra as estatísticas para a direita.
    let bar = visible_of_type(&shell, "StatusBar");
    let saved = inside(&bar, &tr("sl.saved"));
    near(right(&saved), right(&bar) - 14.0, "Salvo no fim da barra");

    let hud = visible_of_type(&shell, "ViewportOperationHud");
    let subject = inside(&hud, "Extrude 2 faces");
    near(
        left(&subject),
        left(&hud),
        "sujeito alinhado às demais linhas",
    );
    near(
        right(&subject),
        right(&hud),
        "sujeito ocupa a largura do HUD",
    );
}

#[test]
fn shading_xray_slider_fills_the_popover_row() {
    let shell = model_shell();
    shell.set_label_xray_opacity("X-ray opacity".into());
    shell.set_shading_popover_open(true);
    settle();

    let slider = element(&shell, "X-ray opacity", AccessibleRole::Slider);
    let toggle = element(&shell, &tr("sl.x_ray_enabled"), AccessibleRole::Checkbox);
    near(
        left(&slider),
        left(&toggle) + 56.0 + 8.0,
        "slider após o rótulo de 56 px",
    );
    near(
        right(&slider) + 8.0 + 36.0,
        right(&toggle),
        "slider até o valor de 36 px",
    );
}

#[test]
fn settings_thickness_slider_fills_the_content_column() {
    let shell = model_shell();
    shell.set_label_highlight_thickness("Highlight thickness".into());
    shell.set_settings_tab(0);
    shell.set_settings_panel_h(900.0);
    shell.set_settings_visible(true);
    settle();

    let slider = element(&shell, "Highlight thickness", AccessibleRole::Slider);
    // O título da seção ocupa a coluna inteira (eixo cruzado do VerticalLayout).
    let heading = element(&shell, "Highlight thickness", AccessibleRole::Text);
    near(left(&slider), left(&heading), "slider começa na coluna");
    near(
        right(&slider) + 8.0 + 44.0,
        right(&heading),
        "slider até o valor de 44 px",
    );
}

#[test]
fn command_palette_search_input_fills_the_field() {
    let shell = model_shell();
    shell.set_command_search_visible(true);
    settle();

    let input = element(&shell, &tr("sl.search_commands"), AccessibleRole::TextInput);
    // Caixa de 480 px, padding 12+12, ícone 16, espaçamento 8.
    near(
        input.size().width,
        480.0 - 24.0 - 16.0 - 8.0,
        "busca preenche o campo",
    );
}

#[test]
fn workspace_capsules_keep_pointer_and_keyboard_selection_in_sync() {
    let shell = model_shell();
    shell.set_label_workspace_draw_title("Draw mode".into());
    shell.set_label_workspace_poly_title("Poly mode".into());
    shell.set_modeling_mode("DRAW".into());
    let changes = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&changes);
    let weak = shell.as_weak();
    shell.on_workspace_changed(move |workspace| {
        observed.borrow_mut().push(workspace.to_string());
        let shell = weak.upgrade().expect("live shell");
        if workspace == "DRAW" || workspace == "POLY" {
            shell.set_active_workspace("MODEL".into());
            shell.set_modeling_mode(workspace);
        } else {
            shell.set_active_workspace(workspace);
        }
    });
    settle();

    let draw = element(&shell, "Draw mode", AccessibleRole::Tab);
    let group = container_of(&shell, "WorkspaceSegment", &draw);
    assert!(draw.size().height >= 28.0, "workspace hit target >=28 px");
    near(
        group.size().height,
        30.0,
        "workspace group retains its height",
    );
    near(
        draw.absolute_position().y + draw.size().height / 2.0,
        group.absolute_position().y + group.size().height / 2.0,
        "workspace hit target remains vertically centered",
    );
    for type_name in ["MenuButton", "ViewportBarButton"] {
        let controls: Vec<_> = ElementHandle::find_by_element_type_name(&shell, type_name)
            .filter(|item| item.size().width > 0.0 && item.computed_opacity() > 0.05)
            .collect();
        assert!(!controls.is_empty(), "visible {type_name}");
        assert!(
            controls.iter().all(|item| item.size().height >= 28.0),
            "{type_name} hit targets >=28 px"
        );
    }
    draw.mock_single_click(slint::platform::PointerEventButton::Left);
    changes.borrow_mut().clear();
    key(&shell, slint::platform::Key::RightArrow);
    assert_eq!(&*changes.borrow(), &["POLY"]);
    key(&shell, slint::platform::Key::RightArrow);
    assert_eq!(&*changes.borrow(), &["POLY", "PAINT"]);
    key(&shell, slint::platform::Key::RightArrow);
    assert_eq!(&*changes.borrow(), &["POLY", "PAINT", "UV"]);
    key(&shell, slint::platform::Key::RightArrow);
    assert_eq!(changes.borrow().len(), 3, "no unavailable workspace");
    key(&shell, slint::platform::Key::Home);
    assert_eq!(shell.get_modeling_mode(), "DRAW");
    key(&shell, slint::platform::Key::End);
    assert_eq!(shell.get_active_workspace(), "UV");
    let poly = element(&shell, "Poly mode", AccessibleRole::Tab);
    poly.invoke_accessible_default_action();
    assert_eq!(shell.get_modeling_mode(), "POLY");
}

#[test]
fn shared_brush_stepper_accepts_pointer_keyboard_and_accessible_action() {
    let shell = shell("PAINT", 4000.0);
    shell.set_brush_size(1.0);
    let edits = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&edits);
    shell.on_brush_size_changed(move |value| observed.borrow_mut().push(value));
    settle();
    let plus = element(&shell, &(tr("sl.size") + " +"), AccessibleRole::Button);
    assert!(plus.size().height >= 28.0 && plus.size().width >= 28.0);
    plus.mock_single_click(slint::platform::PointerEventButton::Left);
    key(&shell, slint::platform::Key::Return);
    plus.invoke_accessible_default_action();
    assert_eq!(edits.borrow().len(), 3);
    assert!(
        edits
            .borrow()
            .iter()
            .all(|value| (*value - 1.2).abs() < 0.001)
    );
}
