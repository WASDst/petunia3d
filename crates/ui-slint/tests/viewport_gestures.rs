//! Gestos reais do shell declarativo: o bridge isolado não cobre hit testing Slint.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use petunia_ui_slint::{PetuniaSlintShell, SceneItem};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn move_pointer(shell: &PetuniaSlintShell, x: f32, y: f32) {
    shell.window().dispatch_event(WindowEvent::PointerMoved {
        position: LogicalPosition::new(x, y),
    });
}

fn scroll_pointer(shell: &PetuniaSlintShell, x: f32, y: f32, delta_y: f32) {
    shell.window().dispatch_event(WindowEvent::PointerScrolled {
        position: LogicalPosition::new(x, y),
        delta_x: 0.0,
        delta_y,
    });
}

#[test]
fn viewport_shortcut_drag_parametric_hover_and_navigation_gesture() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_label_parts("Parts".into());
    shell.set_label_search_parts("Search parts".into());
    shell.set_label_selected_parts_only("Show selected parts only".into());
    shell.set_label_sort_parts("Sort parts by name".into());
    shell.set_label_parts_row_size("Row size".into());
    let test_part = SceneItem {
        id: "part-cube".into(),
        name: "Cube".into(),
        visible: true,
        locked: false,
        selected: true,
        active: true,
        verts: 8,
        tris: 12,
        ..Default::default()
    };
    let parts_model = std::rc::Rc::new(slint::VecModel::from(vec![test_part]));
    shell.set_scene_items(parts_model.clone().into());
    shell.set_parts_items(parts_model.into());
    shell.set_active_tool("move".into());
    shell.set_gizmo_hover_axis(-1);
    shell.set_transform_instant_active(true);

    let updates = Rc::new(Cell::new(0));
    let ends = Rc::new(Cell::new(0));
    let updates_callback = Rc::clone(&updates);
    shell.on_viewport_transform_update(move |_, _, _, _| {
        updates_callback.set(updates_callback.get() + 1);
    });
    let ends_callback = Rc::clone(&ends);
    shell.on_viewport_transform_end(move || {
        ends_callback.set(ends_callback.get() + 1);
    });

    move_pointer(&shell, 600.0, 400.0);
    updates.set(0);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: LogicalPosition::new(600.0, 400.0),
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, 630.0, 420.0);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: LogicalPosition::new(630.0, 420.0),
        button: PointerEventButton::Left,
    });
    assert!(
        updates.get() > 0,
        "o mesmo click-drag precisa atualizar Move"
    );
    assert_eq!(ends.get(), 1, "o release precisa confirmar o Move");

    shell.set_transform_instant_active(false);
    shell.set_active_tool("select".into());
    shell.set_tool_modal_active(true);
    shell.set_keyboard_tool_modal_active(true);
    shell.set_tool_activation("drag".into());
    let parametric_updates = Rc::new(Cell::new(0));
    let parametric_callback = Rc::clone(&parametric_updates);
    shell.on_tool_modal_hovered(move |_, _, _| {
        parametric_callback.set(parametric_callback.get() + 1);
    });
    let viewport_hovers = Rc::new(Cell::new(0));
    let viewport_hovers_callback = Rc::clone(&viewport_hovers);
    shell.on_viewport_hover(move |_, _| {
        viewport_hovers_callback.set(viewport_hovers_callback.get() + 1);
    });
    move_pointer(&shell, 620.0, 420.0);
    move_pointer(&shell, 625.0, 390.0);
    assert!(
        viewport_hovers.get() > 0,
        "pré-seleção precisa reagir ao hover livre"
    );
    assert!(
        parametric_updates.get() > 0,
        "atalho paramétrico precisa manipular sem clique prévio"
    );

    shell.set_tool_modal_active(false);
    shell.set_keyboard_tool_modal_active(false);
    shell.set_view_gizmo_origin_x(896.0);
    shell.set_view_gizmo_origin_y(108.0);
    // O tripé desliza (`animate x`, 180 ms) ao mudar a origem; sem avançar o
    // relógio mockado a área de hit ainda está na posição anterior.
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(300));
    let orbits = Rc::new(Cell::new(0));
    let orbit_callback = Rc::clone(&orbits);
    shell.on_viewport_orbit(move |_, _| {
        orbit_callback.set(orbit_callback.get() + 1);
    });
    // O tripé é posicionado dentro do viewport: localizá-lo pelo nome
    // acessível em vez de coordenadas fixas da janela.
    shell.set_label_view_gizmo("View navigation".into());
    let tripod =
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(&shell, "View navigation")
            .next()
            .expect("tripé de navegação com nome acessível");
    let (position, size) = (tripod.absolute_position(), tripod.size());
    let (x, y) = (
        position.x + size.width * 0.75,
        position.y + size.height * 0.75,
    );
    move_pointer(&shell, x, y);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: LogicalPosition::new(x, y),
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, x + 12.0, y + 12.0);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: LogicalPosition::new(x + 12.0, y + 12.0),
        button: PointerEventButton::Left,
    });
    assert!(orbits.get() > 0, "arrastar o tripé deve orbitar a câmera");

    let shading_button =
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(&shell, "Shading options")
            .next()
            .expect("botão de opções de shading acessível");
    let popover_closes = Rc::new(Cell::new(0));
    let close_callback = Rc::clone(&popover_closes);
    let weak_shell = shell.as_weak();
    shell.on_shading_popover_toggled(move |open| {
        if !open {
            close_callback.set(close_callback.get() + 1);
        }
        weak_shell.unwrap().set_shading_popover_open(open);
    });
    let button_origin = shading_button.absolute_position();
    let button_size = shading_button.size();
    let button_center = LogicalPosition::new(
        button_origin.x + button_size.width * 0.5,
        button_origin.y + button_size.height * 0.5,
    );
    move_pointer(&shell, button_center.x, button_center.y);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: button_center,
        button: PointerEventButton::Left,
    });
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: button_center,
        button: PointerEventButton::Left,
    });
    assert!(
        shell.get_shading_popover_open(),
        "botão deve abrir o popover"
    );

    shell.set_label_xray_opacity("X-Ray opacity".into());
    let slider =
        i_slint_backend_testing::ElementHandle::find_by_accessible_label(&shell, "X-Ray opacity")
            .next()
            .expect("slider de X-Ray acessível");
    let origin = slider.absolute_position();
    let size = slider.size();
    assert!(size.width >= 110.0, "slider precisa de trilho utilizável");
    let start = LogicalPosition::new(origin.x + 10.0, origin.y + size.height * 0.5);
    let end = LogicalPosition::new(origin.x + size.width - 10.0, start.y);
    let opacity_updates = Rc::new(Cell::new(0));
    let opacity_callback = Rc::clone(&opacity_updates);
    shell.on_xray_opacity_set(move |_| {
        opacity_callback.set(opacity_callback.get() + 1);
    });
    move_pointer(&shell, start.x, start.y);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: start,
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, end.x, end.y);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: end,
        button: PointerEventButton::Left,
    });
    assert!(
        opacity_updates.get() > 0,
        "arrasto precisa ajustar opacidade"
    );
    assert_eq!(popover_closes.get(), 0, "slider não deve fechar o popover");

    shell.set_shading_popover_open(false);
    shell.set_model_parts_open(true);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(300));
    let parts_search = ElementHandle::find_by_accessible_label(&shell, "Search parts")
        .find(|element| element.accessible_role() == Some(AccessibleRole::Search))
        .expect("busca acessível de Parts");
    let selected_filter =
        ElementHandle::find_by_accessible_label(&shell, "Show selected parts only")
            .next()
            .expect("filtro acessível de Parts");
    let sort_toggle = ElementHandle::find_by_accessible_label(&shell, "Sort parts by name")
        .next()
        .expect("ordenação acessível de Parts");
    assert!(
        (parts_search.absolute_position().y - selected_filter.absolute_position().y).abs() < 0.5,
        "busca e filtro devem compartilhar a mesma faixa"
    );
    assert!(
        (parts_search.absolute_position().y - sort_toggle.absolute_position().y).abs() < 0.5,
        "busca e ordenação devem compartilhar a mesma faixa"
    );

    let row_size_slider = ElementHandle::find_by_accessible_label(&shell, "Row size")
        .find(|element| {
            element.accessible_role() == Some(AccessibleRole::Slider)
                && element.accessible_id().as_deref() == Some("model-inspector-parts-row-height")
        })
        .expect("slider acessível de tamanho das linhas");
    assert_eq!(row_size_slider.accessible_value_minimum(), Some(28.0));
    assert_eq!(row_size_slider.accessible_value_maximum(), Some(44.0));
    assert_eq!(row_size_slider.accessible_value_step(), Some(1.0));
    assert_eq!(row_size_slider.accessible_value().unwrap().as_str(), "28");
    assert!(
        row_size_slider.size().width >= 100.0,
        "slider precisa de trilho utilizável"
    );

    let row = ElementHandle::find_by_accessible_label(&shell, "Cube")
        .find(|element| element.accessible_role() == Some(AccessibleRole::ListItem))
        .expect("item da lista virtualizada Parts");
    assert_eq!(row.size().height, 28.0);

    let row_size_changes = Rc::new(Cell::new(0));
    let row_size_callback = Rc::clone(&row_size_changes);
    let weak_shell = shell.as_weak();
    shell.on_parts_row_height_changed(move |height| {
        row_size_callback.set(row_size_callback.get() + 1);
        if let Some(shell) = weak_shell.upgrade() {
            shell.set_parts_row_height(height);
        }
    });
    let slider_origin = row_size_slider.absolute_position();
    let slider_size = row_size_slider.size();
    let start = LogicalPosition::new(
        slider_origin.x + slider_size.width * 0.25,
        slider_origin.y + slider_size.height * 0.5,
    );
    let end = LogicalPosition::new(slider_origin.x + slider_size.width * 0.9, start.y);
    move_pointer(&shell, start.x, start.y);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: start,
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, end.x, end.y);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: end,
        button: PointerEventButton::Left,
    });

    assert!(
        row_size_changes.get() > 0,
        "arrastar deve alterar o tamanho das linhas"
    );
    assert!(
        shell.get_parts_row_height() >= 43.0,
        "o novo valor deve chegar ao shell"
    );
    assert_eq!(row.size().height, shell.get_parts_row_height());
    assert_eq!(
        row_size_slider.accessible_value().unwrap().as_str(),
        format!("{}", shell.get_parts_row_height() as i32).as_str()
    );

    shell.window().set_size(LogicalSize::new(800.0, 800.0));
    shell.set_compact_shell(true);
    shell.set_inspector_visible(false);
    shell.set_scene_drawer_visible(true);

    let compact_search = ElementHandle::find_by_accessible_label(&shell, "Search parts")
        .find(|element| {
            element.accessible_label().as_deref() == Some("Search parts")
                && element.accessible_role() == Some(AccessibleRole::Search)
                && element.accessible_id().as_deref() == Some("compact-parts-search")
        })
        .expect("busca do drawer compacto");
    assert_eq!(
        compact_search.accessible_label().as_deref(),
        Some("Search parts")
    );
    assert_eq!(
        compact_search.accessible_role(),
        Some(AccessibleRole::Search)
    );
    let compact_filter =
        ElementHandle::find_by_accessible_label(&shell, "Show selected parts only")
            .next()
            .expect("filtro do drawer compacto");
    let compact_sort = ElementHandle::find_by_accessible_label(&shell, "Sort parts by name")
        .next()
        .expect("ordenação do drawer compacto");
    assert!(
        compact_search.size().width > 0.0,
        "busca compacta precisa estar visível"
    );
    assert_eq!(
        compact_search.absolute_position().y,
        compact_filter.absolute_position().y
    );
    assert_eq!(
        compact_search.absolute_position().y,
        compact_sort.absolute_position().y
    );

    let compact_row_size_slider = ElementHandle::find_by_accessible_label(&shell, "Row size")
        .find(|element| {
            element.accessible_role() == Some(AccessibleRole::Slider)
                && element.accessible_id().as_deref() == Some("compact-parts-row-height")
        })
        .expect("slider do drawer compacto");
    assert!(compact_row_size_slider.size().width >= 100.0);
    assert_eq!(
        compact_row_size_slider.accessible_value().unwrap().as_str(),
        format!("{}", shell.get_parts_row_height() as i32).as_str()
    );
    let compact_row = ElementHandle::find_by_accessible_label(&shell, "Cube")
        .find(|element| element.accessible_role() == Some(AccessibleRole::ListItem))
        .expect("item da lista compacta");
    assert_eq!(compact_row.size().height, shell.get_parts_row_height());

    let compact_slider_origin = compact_row_size_slider.absolute_position();
    let compact_slider_size = compact_row_size_slider.size();
    let compact_drag_start = LogicalPosition::new(
        compact_slider_origin.x + compact_slider_size.width * 0.9,
        compact_slider_origin.y + compact_slider_size.height * 0.5,
    );
    let compact_drag_end = LogicalPosition::new(
        compact_slider_origin.x + compact_slider_size.width * 0.1,
        compact_drag_start.y,
    );
    move_pointer(&shell, compact_drag_start.x, compact_drag_start.y);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: compact_drag_start,
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, compact_drag_end.x, compact_drag_end.y);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: compact_drag_end,
        button: PointerEventButton::Left,
    });
    assert!(shell.get_parts_row_height() <= 29.0);
    assert_eq!(compact_row.size().height, shell.get_parts_row_height());
}

#[test]
fn loop_cut_armed_tool_routes_hover_scroll_and_click_without_navigating() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_active_tool("loop_cut".into());
    shell.set_loop_cut_armed(true);
    shell.set_loop_cut_cuts(1);

    let zooms = Rc::new(Cell::new(0));
    let zoom_callback = Rc::clone(&zooms);
    shell.on_viewport_zoom(move |_| zoom_callback.set(zoom_callback.get() + 1));
    let counts = Rc::new(Cell::new(0));
    let count_callback = Rc::clone(&counts);
    shell.on_loop_cut_count_committed(move |_| count_callback.set(count_callback.get() + 1));

    // Constituição 11 (ADR 007, Onda 2): a roda sempre faz zoom, inclusive
    // com o Loop Cut armado; a contagem de cortes usa Ctrl+roda ou +/−.
    scroll_pointer(&shell, 600.0, 400.0, 1.0);
    assert_eq!(zooms.get(), 1, "a roda sempre faz zoom");
    assert_eq!(counts.get(), 0, "a roda sozinha não muda os cortes");
}

#[test]
fn inspector_collapsed_rail_uses_spaced_icon_pills_and_single_tool_card() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_label_tab_parts("Parts".into());
    shell.set_label_tab_transform("Transform".into());
    shell.set_label_tab_material("Material".into());
    shell.set_label_tab_object("Object".into());
    shell.set_label_tab_modifiers("Modifiers".into());
    shell.set_label_tool_options_collapse("Collapse tool options".into());
    shell.set_label_tool_options_expand("Expand tool options".into());

    // Rail colapsado: pílulas de ícone separadas, sem textos rotacionados.
    // Modifiers virou subseção de Object (plano de UI F4, D2): sem pílula própria.
    shell.set_inspector_collapsed(true);
    let mut tops = Vec::new();
    for label in ["Parts", "Transform", "Material", "Object"] {
        let pill = ElementHandle::find_by_accessible_label(&shell, label)
            .find(|element| {
                element.accessible_role() == Some(AccessibleRole::Button)
                    && (element.size().width - 36.0).abs() < 0.5
                    && (element.size().height - 36.0).abs() < 0.5
            })
            .unwrap_or_else(|| panic!("pílula 36x36 {label} ausente no rail colapsado"));
        tops.push(pill.absolute_position().y + pill.size().height);
    }
    assert!(
        ElementHandle::find_by_accessible_label(&shell, "Modifiers").all(|element| (element
            .size()
            .width
            - 36.0)
            .abs()
            >= 0.5),
        "Modifiers não tem mais pílula no rail"
    );
    for (i, pair) in tops.windows(2).enumerate() {
        assert!(
            pair[1] - pair[0] >= 8.0,
            "pílulas {i} e {} precisam de respiro",
            i + 1
        );
    }

    // Card único de ferramenta: oculto em repouso.
    shell.set_inspector_collapsed(false);
    shell.set_tool_modal_active(false);
    shell.set_loop_cut_active(false);
    shell.set_loop_cut_armed(false);
    shell.set_profile_active(false);
    shell.set_operation_hud_active(false);
    assert!(
        ElementHandle::find_by_accessible_label(&shell, "Collapse tool options")
            .next()
            .is_none(),
        "sem modal/HUD não há card de ferramenta"
    );

    // During modal, exactly one tool card with collapse / Durante o modal, exatamente um card de ferramenta com colapso.
    shell.set_tool_modal_active(true);
    shell.set_tool_modal_title("Bevel".into());
    let collapses: Vec<_> = ElementHandle::find_by_accessible_label(&shell, "Collapse")
        .filter(|element| element.accessible_role() == Some(AccessibleRole::Button))
        .collect();
    assert_eq!(collapses.len(), 1, "um único card de ferramenta no modal");
}

#[test]
fn inspector_rail_collapsed_by_default_and_expands_on_hover() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());

    // By default, no sections are pinned open, and nothing is hovered -> flyout closed.
    assert!(
        !shell.get_inspector_flyout_open(),
        "flyout deve iniciar fechado/recolhido no rail de 56px"
    );

    // Hovering over an inspector section opens the flyout for peek-on-hover.
    shell.set_hovered_inspector_section("material".into());
    assert!(
        shell.get_inspector_flyout_open(),
        "passar mouse sobre uma seção deve abrir o flyout para visualização rápida"
    );

    // Leaving hover closes the flyout again.
    shell.set_hovered_inspector_section("".into());
    assert!(
        !shell.get_inspector_flyout_open(),
        "remover o hover deve fechar o flyout automaticamente"
    );

    // Hovering over the rail directly also expands the flyout.
    shell.set_rail_hovered(true);
    assert!(
        shell.get_inspector_flyout_open(),
        "hover sobre o rail deve abrir o flyout"
    );
    shell.set_rail_hovered(false);
    assert!(
        !shell.get_inspector_flyout_open(),
        "sair do rail deve fechar o flyout"
    );
}

#[test]
fn inspector_pinned_section_keeps_flyout_open_without_hover() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());

    assert!(!shell.get_inspector_flyout_open());

    // Pinning open a section keeps the flyout open even without hovering.
    shell.set_model_material_open(true);
    assert!(
        shell.get_inspector_flyout_open(),
        "seção fixada (pin) deve manter o painel aberto sem necessidade de hover"
    );

    // Unpinning minimizes it back to rail if not hovered.
    shell.set_model_material_open(false);
    assert!(
        !shell.get_inspector_flyout_open(),
        "desafixar (unpin) sem hover deve recolher o painel de volta para o rail"
    );
}

#[test]
fn inspector_pill_and_pin_callbacks_dispatch() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());

    let pill_clicked = Rc::new(RefCell::new(Vec::new()));
    let pill_clicked_clone = Rc::clone(&pill_clicked);
    shell.on_section_pill_clicked(move |id| {
        pill_clicked_clone.borrow_mut().push(id.to_string());
    });

    let pin_toggled = Rc::new(RefCell::new(Vec::new()));
    let pin_toggled_clone = Rc::clone(&pin_toggled);
    shell.on_section_pin_open_toggled(move |id| {
        pin_toggled_clone.borrow_mut().push(id.to_string());
    });

    let open_toggled = Rc::new(RefCell::new(Vec::new()));
    let open_toggled_clone = Rc::clone(&open_toggled);
    shell.on_section_open_toggled(move |id| {
        open_toggled_clone.borrow_mut().push(id.to_string());
    });

    shell.invoke_section_pill_clicked("material".into());
    shell.invoke_section_pin_open_toggled("transform".into());
    shell.invoke_section_open_toggled("parts".into());

    assert_eq!(*pill_clicked.borrow(), vec!["material"]);
    assert_eq!(*pin_toggled.borrow(), vec!["transform"]);
    assert_eq!(*open_toggled.borrow(), vec!["parts"]);
}

#[test]
fn test_tooltip_hover() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_label_workspace_draw_title("Draw Workspace".into());

    // Desde a Onda 4 (ADR 007) o seletor mostra DRAW · POLY · PAINT · UV.
    let tab = ElementHandle::find_by_accessible_label(&shell, "Draw Workspace")
        .next()
        .expect("aba DRAW");
    let pos = tab.absolute_position();
    let size = tab.size();
    let center_x = pos.x + size.width / 2.0;
    let center_y = pos.y + size.height / 2.0;
    println!("Tab pos: {:?}, size: {:?}", pos, size);

    move_pointer(&shell, center_x, center_y);
    i_slint_backend_testing::mock_elapsed_time(std::time::Duration::from_millis(1000));

    // Check active popups or tooltip elements
    let all_elements: Vec<_> = ElementHandle::find_by_element_type_name(&shell, "Text").collect();
    for elem in &all_elements {
        if let Some(text) = elem.accessible_label() {
            println!("Found text: {}", text);
        }
    }
}

#[test]
fn collapse_inspector_button_is_clickable_and_returns_to_the_rail() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_reduced_motion(true);
    shell.global::<petunia_ui_slint::Motion>().set_reduced(true);
    shell.set_label_tab_parts("Parts".into());
    shell.set_label_tab_transform("Transform".into());
    shell.set_label_tab_material("Material".into());
    shell.set_label_tab_object("Object".into());
    shell.set_model_material_open(true);
    assert!(shell.get_inspector_flyout_open());

    let requests = Rc::new(Cell::new(0));
    let requests_callback = Rc::clone(&requests);
    let weak = shell.as_weak();
    shell.on_inspector_collapse_requested(move || {
        requests_callback.set(requests_callback.get() + 1);
        // O bridge real fecha todas as seções; aqui basta o efeito visível.
        if let Some(shell) = weak.upgrade() {
            shell.set_model_material_open(false);
        }
    });

    let button = ElementHandle::find_by_accessible_label(&shell, "Collapse Inspector")
        .find(|element| element.accessible_role() == Some(AccessibleRole::Button))
        .expect("botão Recolher no cabeçalho de contexto");
    let pos = button.absolute_position();
    let size = button.size();
    let (x, y) = (pos.x + size.width / 2.0, pos.y + size.height / 2.0);
    move_pointer(&shell, x, y);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: LogicalPosition::new(x, y),
        button: PointerEventButton::Left,
    });
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: LogicalPosition::new(x, y),
        button: PointerEventButton::Left,
    });

    assert_eq!(requests.get(), 1, "o clique chega ao callback de recolher");
    assert!(
        !shell.get_inspector_flyout_open(),
        "recolher fecha o painel mesmo com o ponteiro ainda sobre ele"
    );

    let openings = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&openings);
    let weak = shell.as_weak();
    shell.on_section_pill_clicked(move |id| {
        observed.borrow_mut().push(id.to_string());
        if let Some(shell) = weak.upgrade() {
            match id.as_str() {
                "parts" => shell.set_model_parts_open(true),
                "transform" => shell.set_model_transform_open(true),
                "material" => shell.set_model_material_open(true),
                "object" => shell.set_model_object_open(true),
                _ => panic!("unexpected Inspector section"),
            }
        }
    });
    for reduced in [false, true] {
        shell.set_reduced_motion(reduced);
        shell
            .global::<petunia_ui_slint::Motion>()
            .set_reduced(reduced);
        for width in [1280.0, 1024.0] {
            shell.window().set_size(LogicalSize::new(width, 800.0));
            for (label, id) in [
                ("Parts", "parts"),
                ("Transform", "transform"),
                ("Material", "material"),
                ("Object", "object"),
            ] {
                shell.set_model_parts_open(false);
                shell.set_model_transform_open(false);
                shell.set_model_material_open(false);
                shell.set_model_object_open(false);
                shell.set_hovered_inspector_section("".into());
                shell.set_flyout_hovered(false);
                shell.set_rail_hovered(false);
                move_pointer(&shell, 600.0, 400.0);
                i_slint_backend_testing::testing_backend::mock_elapsed_time(300);
                let pill = ElementHandle::find_by_accessible_label(&shell, label)
                    .find(|element| {
                        element.accessible_role() == Some(AccessibleRole::Button)
                            && (element.size().width - 36.0).abs() < 0.5
                    })
                    .expect("section pill must remain reachable after collapse");
                let pos = pill.absolute_position();
                let size = pill.size();
                let point =
                    LogicalPosition::new(pos.x + size.width / 2.0, pos.y + size.height / 2.0);
                move_pointer(&shell, point.x, point.y);
                assert!(
                    pill.computed_opacity() > 0.95,
                    "{label} remains visible during peek"
                );
                let flyout = ElementHandle::find_by_element_id(
                    &shell,
                    "PetuniaSlintShell::inspector-flyout",
                )
                .next()
                .expect("Inspector flyout");
                let _ = flyout.absolute_position();
                i_slint_backend_testing::testing_backend::mock_elapsed_time(40);
                assert!(
                    flyout.absolute_position().x + flyout.size().width
                        <= pill.absolute_position().x,
                    "hover preview must not overlap {label}"
                );
                shell.window().dispatch_event(WindowEvent::PointerPressed {
                    position: point,
                    button: PointerEventButton::Left,
                });
                shell.window().dispatch_event(WindowEvent::PointerReleased {
                    position: point,
                    button: PointerEventButton::Left,
                });
                assert_eq!(
                    openings.borrow().last().map(String::as_str),
                    Some(id),
                    "hover must not hide or cover {label} before its click"
                );
                assert!(shell.get_inspector_flyout_open());
            }
        }
    }
}

#[test]
fn inspector_never_shows_transform_values_without_a_selection() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_selection_domain("OBJECT".into());
    shell.set_label_tab_transform("Transform".into());
    shell.set_model_transform_open(true);

    // O cabeçalho da seção é largo; a pílula homônima do trilho tem 36 px e
    // continua na árvore (invisível sob o painel aberto).
    let transform_section = |shell: &PetuniaSlintShell| {
        ElementHandle::find_by_accessible_label(shell, "Transform").any(|element| {
            element.accessible_role() == Some(AccessibleRole::Button)
                && element.size().width > 100.0
        })
    };

    shell.set_object_has_selection(false);
    assert!(!transform_section(&shell), "sem seleção, Transform some");
    assert!(
        ElementHandle::find_by_accessible_label(&shell, "Nothing selected")
            .next()
            .is_some(),
        "sem seleção, o painel explica o que fazer"
    );

    shell.set_object_has_selection(true);
    assert!(transform_section(&shell), "com seleção, Transform volta");
}

#[test]
fn poly_context_bar_follows_the_selection_domain() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1440.0, 900.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_modeling_mode("POLY".into());

    let shows = |shell: &PetuniaSlintShell, label: &str| {
        ElementHandle::find_by_accessible_label(shell, label)
            .any(|element| element.accessible_role() == Some(AccessibleRole::Button))
    };

    shell.set_selection_domain("FACE".into());
    assert!(shows(&shell, "Inset"), "face: Inset");
    assert!(!shows(&shell, "Round Edge"), "face: sem Round Edge");
    assert!(!shows(&shell, "Merge Center"), "face: sem Merge");

    shell.set_selection_domain("EDGE".into());
    assert!(shows(&shell, "Round Edge"), "aresta: Round Edge");
    assert!(!shows(&shell, "Inset"), "aresta: sem Inset");

    shell.set_selection_domain("POINT".into());
    assert!(shows(&shell, "Merge Center"), "ponto: Merge");
    assert!(!shows(&shell, "Round Edge"), "ponto: sem Round Edge");

    // Desenho e corte agem pelo gesto: aparecem em qualquer domínio.
    for domain in ["OBJECT", "FACE", "EDGE", "POINT"] {
        shell.set_selection_domain(domain.into());
        assert!(shows(&shell, "Knife"), "{domain}: Knife sempre visível");
    }
}

#[test]
fn escape_closes_the_home_screen() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_home_open(true);
    let closed = Rc::new(Cell::new(false));
    let closed_flag = Rc::clone(&closed);
    shell.on_home_closed(move || closed_flag.set(true));
    shell.window().dispatch_event(WindowEvent::KeyPressed {
        text: slint::platform::Key::Escape.into(),
    });
    shell.window().dispatch_event(WindowEvent::KeyReleased {
        text: slint::platform::Key::Escape.into(),
    });
    assert!(closed.get(), "Esc fecha a Home");
}


#[test]
fn viewport_drag_threshold_controls_when_transform_drag_starts() {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.show().expect("headless window");
    shell.set_active_workspace("MODEL".into());
    shell.set_active_tool("move".into());
    shell.set_transform_instant_active(false);
    shell.set_gizmo_has_hover(false);

    let begins = Rc::new(Cell::new(0));
    let begin_callback = Rc::clone(&begins);
    shell.on_viewport_transform_begin(move |_, _, _| {
        begin_callback.set(begin_callback.get() + 1);
    });

    // Acessibilidade: um threshold alto mantém um deslocamento moderado como
    // gesto ainda pendente e não promove Move para drag.
    shell.set_drag_threshold_px(100.0);
    move_pointer(&shell, 600.0, 400.0);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: LogicalPosition::new(600.0, 400.0),
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, 630.0, 420.0);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: LogicalPosition::new(630.0, 420.0),
        button: PointerEventButton::Left,
    });
    assert_eq!(
        begins.get(),
        0,
        "drag menor que a preferência não deve iniciar transformação"
    );

    // Com o threshold padrão, o mesmo deslocamento inicia exatamente um drag.
    shell.set_drag_threshold_px(4.0);
    move_pointer(&shell, 600.0, 400.0);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: LogicalPosition::new(600.0, 400.0),
        button: PointerEventButton::Left,
    });
    move_pointer(&shell, 630.0, 420.0);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: LogicalPosition::new(630.0, 420.0),
        button: PointerEventButton::Left,
    });
    assert_eq!(
        begins.get(),
        1,
        "drag acima da preferência deve iniciar transformação"
    );
}
