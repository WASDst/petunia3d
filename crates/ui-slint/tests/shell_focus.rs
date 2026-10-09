//! F6/Shift+F6 regionais. Código de regressão preparado; bateria adiada pelo usuário.
//! Estes cenários não substituem Tab trap, reader ou aceite nativo.
use petunia_ui_slint::PetuniaSlintShell;
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, LogicalSize};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

fn shell() -> PetuniaSlintShell {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.set_label_parts("Workspace structure".into());
    shell.global::<petunia_ui_slint::Motion>().set_reduced(true);
    shell.show().expect("headless window");
    shell
}
fn key(shell: &PetuniaSlintShell, text: slint::SharedString) {
    shell
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    shell
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text });
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    // Um tick inicia bindings de Timer; o seguinte conclui a transferência.
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
}
fn backward(shell: &PetuniaSlintShell) {
    shell.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    key(shell, Key::F6.into());
    shell.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
}

#[test]
fn f6_cycles_all_regions_and_dispatches_to_real_visible_entries() {
    let shell = shell();
    shell.set_model_parts_open(true);
    shell.set_asset_library_visible(true);
    let panes = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&panes);
    shell.on_inspector_pane_collapsed_set(move |pane, value| {
        observed.borrow_mut().push((pane, value))
    });
    let menus = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&menus);
    shell.on_menu_toggled(move |id| observed.borrow_mut().push(id.to_string()));
    let tools = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&tools);
    shell.on_active_tool_changed(move |id| observed.borrow_mut().push(id.to_string()));
    let shortcuts = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&shortcuts);
    shell.on_shortcut_requested(move |id, _, _, _| observed.borrow_mut().push(id.to_string()));
    for workspace in ["MODEL", "PAINT", "UV", "ANIMATE"] {
        shell.set_active_workspace(workspace.into());
        for theme in ["petunia-dark", "petunia-light", "petunia-high-contrast"] {
            petunia_ui_slint::theme::apply_theme(&shell, theme);
            panes.borrow_mut().clear();
            menus.borrow_mut().clear();
            tools.borrow_mut().clear();
            shortcuts.borrow_mut().clear();
            assert_eq!(shell.get_focus_region(), 2);
            let forward: &[i32] = if workspace == "ANIMATE" {
                &[3, 4, 5, 0, 1, 2]
            } else {
                &[3, 4, 5, 6, 0, 1, 2]
            };
            for &expected in forward {
                key(&shell, Key::F6.into());
                assert_eq!(shell.get_focus_region(), expected, "{workspace}/{theme}");
                if [3, 4, 0, 1].contains(&expected) {
                    key(&shell, Key::Return.into());
                }
            }
            assert_eq!(
                *panes.borrow(),
                vec![(0, true), (1, true)],
                "visible pane headers receive Enter"
            );
            assert_eq!(
                *menus.borrow(),
                vec!["file"],
                "header entry invokes the real File callback"
            );
            assert_eq!(
                *tools.borrow(),
                vec!["select"],
                "tool region reaches the existing route"
            );
            assert!(
                shortcuts.borrow().is_empty(),
                "entry activation is consumed locally"
            );
            key(&shell, "a".into());
            assert_eq!(
                *shortcuts.borrow(),
                vec!["a"],
                "surface restores global keyboard routing"
            );
            let reverse: &[i32] = if workspace == "ANIMATE" {
                &[1, 0, 5, 4, 3, 2]
            } else {
                &[1, 0, 6, 5, 4, 3, 2]
            };
            for &expected in reverse {
                backward(&shell);
                assert_eq!(
                    shell.get_focus_region(),
                    expected,
                    "inverse cycle {workspace}/{theme}"
                );
            }
        }
    }
}

#[test]
fn hidden_regions_are_skipped_and_disappearing_regions_restore_surface() {
    let shell = shell();
    shell.set_inspector_visible(false);
    for expected in [6, 0, 1, 2] {
        key(&shell, Key::F6.into());
        assert_eq!(shell.get_focus_region(), expected);
    }
    shell.set_inspector_visible(true);
    key(&shell, Key::F6.into());
    assert_eq!(shell.get_focus_region(), 3);
    let sections = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&sections);
    shell.on_section_pill_clicked(move |id| observed.borrow_mut().push(id.to_string()));
    key(&shell, Key::Return.into());
    assert_eq!(
        *sections.borrow(),
        vec!["parts"],
        "closed inspector uses its visible pill"
    );
    shell.window().set_size(LogicalSize::new(800.0, 800.0));
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    assert_eq!(
        shell.get_focus_region(),
        2,
        "compact layout cannot retain hidden inspector focus"
    );
    assert!(
        shell.get_hovered_inspector_section().is_empty(),
        "keyboard peek closes when leaving Structure"
    );
    key(&shell, Key::F6.into());
    assert_eq!(shell.get_focus_region(), 6);
    shell.set_asset_library_visible(true);
    backward(&shell);
    assert_eq!(shell.get_focus_region(), 5);
    shell.set_asset_library_visible(false);
    assert_eq!(
        shell.get_focus_region(),
        2,
        "closing focused drawer restores the surface"
    );
    key(&shell, Key::F6.into());
    assert_eq!(shell.get_focus_region(), 6);
    shell.set_active_workspace("ANIMATE".into());
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    assert_eq!(shell.get_focus_region(), 2, "empty Context Bar is omitted");
}

#[test]
fn overlays_block_regional_navigation_and_restore_the_invoking_region() {
    let shell = shell();
    shell.set_model_parts_open(true);
    key(&shell, Key::F6.into());
    key(&shell, Key::F6.into());
    assert_eq!(shell.get_focus_region(), 4);
    for overlay in 0..7 {
        match overlay {
            0 => shell.set_settings_visible(true),
            1 => shell.set_command_search_visible(true),
            2 => shell.set_reference_manager_visible(true),
            3 => shell.set_menu_open("file".into()),
            4 => shell.set_context_menu_open(true),
            5 => shell.set_home_open(true),
            _ => shell.set_recovery_open(true),
        }
        key(&shell, Key::F6.into());
        backward(&shell);
        assert_eq!(
            shell.get_focus_region(),
            4,
            "F6 must not escape overlay {overlay}"
        );
        match overlay {
            0 => shell.set_settings_visible(false),
            1 => shell.set_command_search_visible(false),
            2 => shell.set_reference_manager_visible(false),
            3 => shell.set_menu_open("".into()),
            4 => shell.set_context_menu_open(false),
            5 => shell.set_home_open(false),
            _ => shell.set_recovery_open(false),
        }
        let panes = Rc::new(RefCell::new(Vec::new()));
        let observed = Rc::clone(&panes);
        shell.on_inspector_pane_collapsed_set(move |pane, value| {
            observed.borrow_mut().push((pane, value))
        });
        key(&shell, Key::Return.into());
        assert_eq!(
            *panes.borrow(),
            vec![(1, true)],
            "regional restoration reaches Properties after {overlay}"
        );
    }
}

#[test]
fn f6_is_captured_as_a_key_binding_before_any_regional_routing() {
    let shell = shell();
    shell.set_settings_visible(true);
    shell.set_keymap_capture_action("view.test".into());
    let captured = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&captured);
    shell.on_keymap_capture_key(move |key, _, _, _| observed.borrow_mut().push(key.to_string()));
    key(&shell, Key::F6.into());
    assert_eq!(shell.get_focus_region(), 2);
    assert_eq!(*captured.borrow(), vec!["F6"]);
    let tab = shell.get_settings_tab();
    key(&shell, Key::DownArrow.into());
    assert_eq!(
        shell.get_settings_tab(),
        tab,
        "binding capture precedes Settings arrow navigation"
    );
    assert_eq!(captured.borrow().len(), 2, "the arrow is also captured");
}

#[test]
fn palette_initial_focus_and_native_tab_wrap_reach_real_controls() {
    let shell = shell();
    let shortcuts = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&shortcuts);
    shell.on_shortcut_requested(move |id, _, _, _| observed.borrow_mut().push(id.to_string()));
    let weak = shell.as_weak();
    shell.on_escape_requested(move || weak.upgrade().unwrap().set_command_search_visible(false));
    shell.set_command_search_visible(true);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    key(&shell, "q".into());
    assert_eq!(shell.get_command_query(), "q", "opening focuses the query");
    // Backtab from the first input must wrap to the actual footer close.
    shell.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    key(&shell, Key::Tab.into());
    shell.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
    key(&shell, Key::Return.into());
    assert!(
        !shell.get_command_search_visible(),
        "wrapped close consumes Enter"
    );
    assert!(
        shortcuts.borrow().is_empty(),
        "no key leaked to document tools"
    );

    shell.set_command_query("".into());
    shell.set_command_search_visible(true);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    key(&shell, Key::Tab.into());
    key(&shell, Key::Tab.into());
    key(&shell, "w".into());
    assert_eq!(
        shell.get_command_query(),
        "w",
        "forward traversal wraps to input"
    );
}

#[test]
fn settings_backtab_wrap_closes_through_its_real_footer_callback() {
    let shell = shell();
    let closed = Rc::new(RefCell::new(0));
    let observed = Rc::clone(&closed);
    shell.on_settings_closed(move || *observed.borrow_mut() += 1);
    shell.set_settings_visible(true);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    key(&shell, "x".into());
    assert_eq!(
        shell.get_settings_query(),
        "x",
        "initial focus is Settings search"
    );
    // Search -> header close -> leading guard -> footer close.
    shell.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Shift.into(),
    });
    key(&shell, Key::Tab.into());
    key(&shell, Key::Tab.into());
    shell.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Shift.into(),
    });
    key(&shell, Key::Return.into());
    assert_eq!(*closed.borrow(), 1);
    assert!(!shell.get_settings_visible());
}

#[test]
fn modal_close_restores_each_header_invoker_instead_of_the_group_entry() {
    let shell = shell();
    let opens = Rc::new(RefCell::new([0, 0]));
    let observed = Rc::clone(&opens);
    let weak = shell.as_weak();
    shell.on_settings_requested(move || {
        observed.borrow_mut()[0] += 1;
        weak.upgrade().unwrap().set_settings_visible(true);
    });
    let observed = Rc::clone(&opens);
    let weak = shell.as_weak();
    shell.on_search_requested(move || {
        observed.borrow_mut()[1] += 1;
        weak.upgrade().unwrap().set_command_search_visible(true);
    });
    let weak = shell.as_weak();
    shell.on_escape_requested(move || weak.upgrade().unwrap().set_command_search_visible(false));
    for (index, label) in ["Settings", "Search commands…"].into_iter().enumerate() {
        let invoker =
            i_slint_backend_testing::ElementHandle::find_by_accessible_label(&shell, label)
                .find(|item| {
                    item.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button)
                })
                .expect("real Header invoker");
        invoker.mock_single_click(slint::platform::PointerEventButton::Left);
        i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
        assert_eq!(opens.borrow()[index], 1);
        key(&shell, Key::Escape.into());
        assert!(!shell.get_settings_visible() && !shell.get_command_search_visible());
        key(&shell, Key::Return.into());
        assert_eq!(opens.borrow()[index], 2, "Enter reopens the same invoker");
        key(&shell, Key::Escape.into());
    }
}

#[test]
fn binding_capture_intercepts_text_enter_tab_and_escape_before_modal_controls() {
    let shell = shell();
    shell.set_settings_visible(true);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    shell.set_keymap_capture_action("view.test".into());
    let captured = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&captured);
    shell.on_keymap_capture_key(move |id, _, _, _| observed.borrow_mut().push(id.to_string()));
    let weak = shell.as_weak();
    shell.on_keymap_capture_cancelled(move || {
        weak.upgrade().unwrap().set_keymap_capture_action("".into())
    });
    key(&shell, "r".into());
    key(&shell, Key::Return.into());
    key(&shell, Key::Tab.into());
    assert_eq!(*captured.borrow(), vec!["r", "Enter", "Tab"]);
    assert!(
        shell.get_settings_query().is_empty(),
        "capture does not type into the search"
    );
    key(&shell, Key::Escape.into());
    assert!(
        shell.get_settings_visible(),
        "first Esc only cancels binding capture"
    );
    key(&shell, Key::Escape.into());
    assert!(
        !shell.get_settings_visible(),
        "second Esc dismisses the modal"
    );
}

#[test]
fn viewport_tab_tool_binding_remains_available_while_controls_use_native_tab() {
    let shell = shell();
    let routed = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&routed);
    shell.on_shortcut_requested(move |id, _, _, _| observed.borrow_mut().push(id.to_string()));
    key(&shell, Key::Tab.into());
    assert_eq!(
        *routed.borrow(),
        vec!["Tab"],
        "viewport preserves its existing keymap"
    );
    shell.set_inspector_visible(false);
    // Surface -> Context -> Header, then Tab enters Header controls natively.
    key(&shell, Key::F6.into());
    key(&shell, Key::F6.into());
    routed.borrow_mut().clear();
    key(&shell, Key::Tab.into());
    assert!(
        routed.borrow().is_empty(),
        "Header traversal does not execute a tool shortcut"
    );
}

#[test]
fn projected_stack_top_owns_focus_when_both_modals_remain_visible() {
    let shell = shell();
    shell.set_overlay_top_id("settings".into());
    shell.set_settings_visible(true);
    shell.set_command_search_visible(true);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    key(&shell, "s".into());
    assert_eq!(shell.get_settings_query(), "s");
    assert!(
        shell.get_command_query().is_empty(),
        "underlying palette must not take focus"
    );
    shell.set_overlay_top_id("command_palette".into());
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    key(&shell, "p".into());
    assert_eq!(shell.get_command_query(), "p");
    assert_eq!(
        shell.get_settings_query(),
        "s",
        "underlying Settings draft survives"
    );
    let weak = shell.as_weak();
    shell.on_escape_requested(move || {
        let shell = weak.upgrade().unwrap();
        shell.set_overlay_top_id("settings".into());
        shell.set_command_search_visible(false);
    });
    key(&shell, Key::Escape.into());
    assert!(shell.get_settings_visible());
    assert!(!shell.get_command_search_visible());
    key(&shell, "t".into());
    assert_eq!(
        shell.get_settings_query(),
        "st",
        "remaining modal regains query focus"
    );
}

#[test]
fn recovery_initial_focus_is_keep_and_escape_never_decides() {
    let shell = shell();
    let decisions = Rc::new(RefCell::new(Vec::<&'static str>::new()));
    let keep = Rc::clone(&decisions);
    shell.on_recovery_keep_requested(move || keep.borrow_mut().push("keep"));
    let discard = Rc::clone(&decisions);
    shell.on_recovery_discard_requested(move || discard.borrow_mut().push("discard"));
    let recover = Rc::clone(&decisions);
    shell.on_recovery_recover_requested(move || recover.borrow_mut().push("recover"));
    shell.set_recovery_open(true);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));

    // Escape é não decisório: nem descarta, nem recupera, nem mantém.
    key(&shell, Key::Escape.into());
    assert!(shell.get_recovery_open(), "Escape leaves the decision open");
    assert!(decisions.borrow().is_empty(), "Escape emits no decision");

    // Foco inicial seguro: Enter ativa "Manter", a ação não destrutiva.
    key(&shell, Key::Return.into());
    assert_eq!(*decisions.borrow(), vec!["keep"], "initial focus is Keep");
}

#[test]
fn creation_popover_returns_focus_to_its_rail_invoker() {
    let shell = shell();
    shell.set_modeling_mode("DRAW".into());
    // O bridge real fecha o popover; aqui a emulação só liga a flag sincronizada.
    let toggles = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&toggles);
    let weak = shell.as_weak();
    shell.on_add_menu_changed(move |open| {
        observed.borrow_mut().push(open);
        if open {
            weak.upgrade().unwrap().set_add_menu_open(true);
        }
    });
    let add = i_slint_backend_testing::ElementHandle::find_by_accessible_label(
        &shell,
        &petunia_ui_slint::tr::lookup("sl.add_primitive"),
    )
    .find(|item| item.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button))
    .expect("real rail invoker");
    add.mock_single_click(slint::platform::PointerEventButton::Left);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    assert_eq!(
        *toggles.borrow(),
        vec![true],
        "the rail control opens the creation popover"
    );
    assert!(shell.get_add_menu_open());
    // Fechamento vindo do bridge (Esc/click-away/atalho): devolve o foco ao invocador.
    shell.set_add_menu_open(false);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    assert!(!shell.get_add_menu_open());
    key(&shell, Key::Return.into());
    assert_eq!(
        *toggles.borrow(),
        vec![true, true],
        "Enter reopens the same rail invoker"
    );
}

#[test]
fn context_menu_items_are_actionable_by_keyboard_and_semantic_action() {
    let shell = shell();
    shell.set_context_menu_open(true);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    let actions = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&actions);
    shell.on_context_menu_action(move |id| observed.borrow_mut().push(id.to_string()));
    let rename = i_slint_backend_testing::ElementHandle::find_by_accessible_label(
        &shell,
        &petunia_ui_slint::tr::lookup("sl.rename"),
    )
    .find(|item| item.accessible_role() == Some(i_slint_backend_testing::AccessibleRole::Button))
    .expect("menu item exposed as a button");
    // A ação semântica percorre o mesmo caminho do Enter/Espaço no item focado.
    rename.invoke_accessible_default_action();
    assert_eq!(*actions.borrow(), vec!["rename"]);
    assert!(!shell.get_context_menu_open(), "activating closes the menu");
}

#[test]
fn home_escape_closes_once_through_its_modal_boundary() {
    let shell = shell();
    let closed = Rc::new(RefCell::new(0));
    let observed = Rc::clone(&closed);
    shell.on_home_closed(move || *observed.borrow_mut() += 1);
    shell.set_home_open(true);
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    key(&shell, Key::Escape.into());
    assert_eq!(*closed.borrow(), 1, "Escape closes Home exactly once");
}
