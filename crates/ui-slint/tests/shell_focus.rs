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
    for overlay in 0..5 {
        match overlay {
            0 => shell.set_settings_visible(true),
            1 => shell.set_command_search_visible(true),
            2 => shell.set_reference_manager_visible(true),
            3 => shell.set_menu_open("file".into()),
            _ => shell.set_context_menu_open(true),
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
            _ => shell.set_context_menu_open(false),
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
