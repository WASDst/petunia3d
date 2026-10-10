//! Regressões da extração UV: input físico no componente ligado ao shell.
//! Não substitui os gates visuais nem os testes de Geometry/Undo.

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use petunia_ui_slint::PetuniaSlintShell;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn shell() -> PetuniaSlintShell {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 1200.0));
    shell.set_active_workspace("UV".into());
    shell.set_model_parts_open(true);
    shell.global::<petunia_ui_slint::Motion>().set_reduced(true);
    shell.show().expect("headless window");
    shell
}

fn pointer(shell: &PetuniaSlintShell, position: LogicalPosition) {
    shell
        .window()
        .dispatch_event(WindowEvent::PointerMoved { position });
}

fn click(shell: &PetuniaSlintShell, element: &ElementHandle) {
    let origin = element.absolute_position();
    let size = element.size();
    assert!(size.width > 0.0 && size.height > 0.0);
    let position = LogicalPosition::new(origin.x + size.width * 0.5, origin.y + size.height * 0.5);
    pointer(shell, position);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position,
        button: PointerEventButton::Left,
    });
}

#[test]
fn uv_canvas_forwards_selection_and_drag_with_bottom_origin() {
    let shell = shell();
    let selections = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&selections);
    shell.on_uv_editor_clicked(move |u, v, extend| {
        observed.borrow_mut().push((u, v, extend));
    });
    let moves = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&moves);
    shell.on_uv_moved(move |du, dv| observed.borrow_mut().push((du, dv)));

    let canvas = ElementHandle::find_by_element_id(&shell, "UvInspector::uv-box")
        .next()
        .expect("UV canvas composed in the shell");
    let origin = canvas.absolute_position();
    let size = canvas.size();
    assert!(size.width > 0.0 && size.height > 0.0);
    let start = LogicalPosition::new(origin.x + size.width * 0.25, origin.y + size.height * 0.25);
    let end = LogicalPosition::new(start.x + size.width * 0.125, start.y + size.height * 0.125);
    pointer(&shell, start);
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position: start,
        button: PointerEventButton::Left,
    });
    pointer(&shell, end);
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: end,
        button: PointerEventButton::Left,
    });
    let selections = selections.borrow();
    assert_eq!(selections.len(), 1, "one selection per press");
    let (u, v, extend) = selections[0];
    assert!((u - 0.25).abs() < 0.001 && (v - 0.75).abs() < 0.001);
    assert!(!extend);
    let movements = moves.borrow();
    assert_eq!(movements.len(), 1, "one movement for the drag");
    let (du, dv) = movements[0];
    assert!((du - 0.125).abs() < 0.001 && (dv + 0.125).abs() < 0.001);
    drop(movements);
    pointer(&shell, start);
    assert_eq!(moves.borrow().len(), 1, "release stops UV movement");
}

#[test]
fn uv_quick_actions_remain_reachable_after_workspace_switch_and_resize() {
    let shell = shell();
    let actions = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&actions);
    shell.on_uv_moved(move |du, dv| observed.borrow_mut().push(("move", du, dv)));
    let observed = Rc::clone(&actions);
    shell.on_uv_scaled(move |factor| observed.borrow_mut().push(("scale", factor, 0.0)));
    let observed = Rc::clone(&actions);
    shell.on_uv_rotated(move |angle| observed.borrow_mut().push(("rotate", angle, 0.0)));

    for width in [1280.0, 1024.0] {
        shell.set_active_workspace("MODEL".into());
        shell.set_active_workspace("UV".into());
        shell.window().set_size(LogicalSize::new(width, 1200.0));
        actions.borrow_mut().clear();
        for key in [
            "sl.move_uv_right",
            "sl.scale_uv_up",
            "sl.rotate_uv_clockwise",
        ] {
            let label = petunia_ui_slint::tr::lookup(key);
            let button = ElementHandle::find_by_accessible_label(&shell, &label)
                .find(|element| element.accessible_role() == Some(AccessibleRole::Button))
                .unwrap_or_else(|| panic!("accessible UV action {key}"));
            click(&shell, &button);
        }
        assert_eq!(
            *actions.borrow(),
            vec![
                ("move", 0.05, 0.0),
                ("scale", 1.1, 0.0),
                ("rotate", 15.0, 0.0)
            ],
            "physical clicks must reach the original shell callbacks at width {width}"
        );
    }
}

#[test]
fn paint_surface_operations_are_reachable_outside_model_workspace() {
    let shell = shell();
    shell.set_active_workspace("PAINT".into());
    shell.set_label_unwrap_mesh("Unwrap surface".into());
    shell.set_label_pack_islands("Pack surface".into());
    let commands = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&commands);
    shell.on_command_executed(move |command| observed.borrow_mut().push(command.to_string()));

    for label in ["Unwrap surface", "Pack surface"] {
        let button = ElementHandle::find_by_accessible_label(&shell, label)
            .find(|element| element.accessible_role() == Some(AccessibleRole::Button))
            .unwrap_or_else(|| panic!("PAINT operation {label} must be reachable independently"));
        click(&shell, &button);
    }
    assert_eq!(*commands.borrow(), vec!["uv.unwrap", "uv.pack_islands"]);
}
