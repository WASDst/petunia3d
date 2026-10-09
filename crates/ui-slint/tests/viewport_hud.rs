//! HUDs passivos compostos: geometria, gate de workspace e input atravessando camadas.

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use petunia_ui_slint::PetuniaSlintShell;
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn shell() -> PetuniaSlintShell {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.set_active_workspace("MODEL".into());
    shell.set_active_tool("select".into());
    shell.global::<petunia_ui_slint::Motion>().set_reduced(true);
    shell.show().expect("headless window");
    shell
}

fn element(shell: &PetuniaSlintShell, id: &str) -> ElementHandle {
    ElementHandle::find_by_element_id(shell, id)
        .next()
        .unwrap_or_else(|| panic!("HUD element {id}"))
}

#[test]
fn cursor_pill_clamps_after_resize_and_does_not_capture_selection() {
    let shell = shell();
    shell.set_hud_pill_visible(true);
    shell.set_hud_pill_title("Distance".into());
    shell.set_hud_pill_value("12.50 m".into());
    shell.set_cursor_visible(true);
    shell.set_snap_marker_visible(true);
    shell.set_snap_marker_label("Vertex".into());
    let selections = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&selections);
    shell.on_viewport_select(move |x, y, extend, loop_selection| {
        observed.borrow_mut().push((x, y, extend, loop_selection))
    });

    for width in [1280.0, 1024.0] {
        shell.window().set_size(LogicalSize::new(width, 800.0));
        shell.set_hud_pill_x(10000.0);
        shell.set_hud_pill_y(10000.0);
        let viewport = element(&shell, "PetuniaSlintShell::viewport-region");
        let origin = viewport.absolute_position();
        let extent = viewport.size();
        let pill = element(&shell, "ViewportCursorHud::cursor-pill");
        let position = pill.absolute_position();
        let size = pill.size();
        assert!(position.x >= origin.x + 7.9 && position.y >= origin.y + 7.9);
        assert!(position.x + size.width <= origin.x + extent.width - 7.9);
        assert!(position.y + size.height <= origin.y + extent.height - 7.9);
        // Place every passive layer at the same pointer target, away from interactive chrome.
        shell.set_hud_pill_x(extent.width * 0.4);
        shell.set_hud_pill_y(extent.height * 0.4);
        let pill = element(&shell, "ViewportCursorHud::cursor-pill");
        let corner = pill.absolute_position();
        let size = pill.size();
        let position =
            LogicalPosition::new(corner.x + size.width * 0.5, corner.y + size.height * 0.5);
        shell.set_cursor_screen_x(position.x - origin.x);
        shell.set_cursor_screen_y(position.y - origin.y);
        shell.set_snap_marker_x(position.x - origin.x);
        shell.set_snap_marker_y(position.y - origin.y);
        selections.borrow_mut().clear();
        shell
            .window()
            .dispatch_event(WindowEvent::PointerMoved { position });
        shell.window().dispatch_event(WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        });
        shell.window().dispatch_event(WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        });
        let values = selections.borrow();
        assert_eq!(values.len(), 1, "passive HUDs must not consume clicks");
        assert!((values[0].0 - (position.x - origin.x) / extent.width).abs() < 0.001);
        assert!((values[0].1 - (position.y - origin.y) / extent.height).abs() < 0.001);
        assert!(!values[0].2 && !values[0].3);
    }
}

#[test]
fn projected_measurements_and_cursor_hud_are_model_only() {
    let shell = shell();
    shell.set_dimension_visible(true);
    shell.set_dimension_text("10 m".into());
    shell.set_dimension_x(300.0);
    shell.set_dimension_y(250.0);
    shell.set_measure_visible(true);
    shell.set_measure_hud_text("Total 10 m".into());
    shell.set_hud_pill_visible(true);
    shell.set_hud_pill_title("Dimension".into());
    shell.set_hud_pill_value("10 m".into());
    for workspace in ["MODEL", "PAINT", "UV", "ANIMATE", "MODEL"] {
        shell.set_active_workspace(workspace.into());
        for id in [
            "ViewportCursorHud::cursor-pill",
            "ViewportCursorHud::dimension-label",
            "ViewportStatusHud::measure-summary",
        ] {
            assert_eq!(
                ElementHandle::find_by_element_id(&shell, id)
                    .next()
                    .is_some(),
                workspace == "MODEL",
                "{id} in {workspace}"
            );
        }
    }
    shell.set_measure_visible(false);
    assert!(
        ElementHandle::find_by_element_id(&shell, "ViewportStatusHud::measure-summary")
            .next()
            .is_none()
    );
}
