//! Fechamento automatizado de U01: F6 realmente entrega foco ao rail e o devolve.
//! Verificação headless; a aceitação visual e AT-SPI/leitor de tela são gates distintos.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use petunia_ui_slint::PetuniaSlintShell;
use slint::platform::{Key, WindowEvent};
use slint::{ComponentHandle, LogicalSize};

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

#[test]
fn f6_enters_the_workspace_rail_and_restores_global_keyboard_routing() {
    let shell = shell();
    let sections = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&sections);
    shell.on_section_pill_clicked(move |id| observed.borrow_mut().push(id.to_string()));
    let shortcuts = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&shortcuts);
    shell.on_shortcut_requested(move |text, ctrl, shift, alt| {
        observed
            .borrow_mut()
            .push((text.to_string(), ctrl, shift, alt))
    });

    let panes = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&panes);
    shell.on_inspector_pane_collapsed_set(move |pane, collapsed| {
        observed.borrow_mut().push((pane, collapsed))
    });

    for workspace in ["MODEL", "PAINT", "UV", "ANIMATE", "MODEL"] {
        shell.set_active_workspace(workspace.into());
        for theme in ["petunia-dark", "petunia-light", "petunia-high-contrast"] {
            petunia_ui_slint::theme::apply_theme(&shell, theme);
            sections.borrow_mut().clear();
            shortcuts.borrow_mut().clear();
            panes.borrow_mut().clear();
            key(&shell, Key::F6.into());
            assert!(
                shell.get_f6_in_rail(),
                "F6 enters {workspace} rail in {theme}"
            );
            key(&shell, Key::Return.into());
            if workspace == "ANIMATE" {
                assert!(
                    sections.borrow().is_empty(),
                    "hidden rail must not receive Enter"
                );
                assert_eq!(
                    *panes.borrow(),
                    vec![(0, true)],
                    "Enter reaches visible Structure header in {workspace}/{theme}"
                );
            } else {
                assert_eq!(
                    *sections.borrow(),
                    vec!["parts"],
                    "Enter activates visible pill in {workspace}/{theme}"
                );
                assert!(
                    panes.borrow().is_empty(),
                    "collapsed flyout must not steal rail focus"
                );
            }
            assert!(shortcuts.borrow().is_empty(), "the pill consumes Enter");
            key(&shell, Key::F6.into());
            assert!(!shell.get_f6_in_rail(), "second F6 restores viewport focus");
            key(&shell, "a".into());
            assert_eq!(
                *shortcuts.borrow(),
                vec![("a".to_string(), false, false, false)],
                "global shortcut routing restored"
            );
            assert_eq!(
                sections.borrow().len(),
                usize::from(workspace != "ANIMATE"),
                "global keyboard must not activate another pill"
            );
        }
    }
}

#[test]
fn f6_preserves_keyboard_routing_when_the_rail_is_hidden_or_compact() {
    let shell = shell();
    let shortcuts = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&shortcuts);
    shell.on_shortcut_requested(move |text, _, _, _| observed.borrow_mut().push(text.to_string()));
    shell.set_inspector_visible(false);
    let token = shell.get_f6_rail_token();
    key(&shell, Key::F6.into());
    key(&shell, "h".into());
    assert_eq!(
        shell.get_f6_rail_token(),
        token,
        "hidden rail must not gain focus"
    );
    assert_eq!(*shortcuts.borrow(), vec!["h"]);

    shell.set_inspector_visible(true);
    shell.window().set_size(LogicalSize::new(800.0, 800.0));
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    assert!(shell.get_compact_shell());
    key(&shell, Key::F6.into());
    key(&shell, "c".into());
    assert_eq!(
        shell.get_f6_rail_token(),
        token,
        "compact rail must not gain focus"
    );
    assert_eq!(*shortcuts.borrow(), vec!["h", "c"]);

    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
    assert!(!shell.get_compact_shell());
    key(&shell, Key::F6.into());
    assert_eq!(
        shell.get_f6_rail_token(),
        token + 1,
        "rail navigation recovers after resize"
    );
}

#[test]
fn f6_targets_structure_in_a_pinned_model_panel_and_can_return_to_viewport() {
    let shell = shell();
    shell.set_model_parts_open(true);
    let panes = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&panes);
    shell.on_inspector_pane_collapsed_set(move |pane, value| {
        observed.borrow_mut().push((pane, value))
    });
    let shortcuts = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&shortcuts);
    shell.on_shortcut_requested(move |text, _, _, _| observed.borrow_mut().push(text.to_string()));
    key(&shell, Key::F6.into());
    key(&shell, Key::Return.into());
    assert_eq!(
        *panes.borrow(),
        vec![(0, true)],
        "visible Structure header receives Enter"
    );
    key(&shell, Key::F6.into());
    key(&shell, "p".into());
    assert_eq!(*shortcuts.borrow(), vec!["p"]);
}
