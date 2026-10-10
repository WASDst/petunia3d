//! Gestos físicos da divisão Structure/Properties. Não executado até a bateria autorizada.

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::ElementHandle;
use petunia_ui_slint::PetuniaSlintShell;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize};

fn shell() -> PetuniaSlintShell {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.set_model_parts_open(true);
    shell.global::<petunia_ui_slint::Motion>().set_reduced(true);
    shell.show().expect("headless window");
    shell
}

fn element(shell: &PetuniaSlintShell, id: &str) -> ElementHandle {
    ElementHandle::find_by_element_id(shell, id)
        .next()
        .unwrap_or_else(|| panic!("split element {id}"))
}

fn center(element: &ElementHandle) -> LogicalPosition {
    let origin = element.absolute_position();
    let size = element.size();
    assert!(size.width > 0.0 && size.height > 0.0);
    LogicalPosition::new(origin.x + size.width / 2.0, origin.y + size.height / 2.0)
}

fn key(shell: &PetuniaSlintShell, text: slint::SharedString) {
    shell
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    shell
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text });
}

#[test]
fn divider_drag_and_keyboard_resize_panes_without_entering_the_viewport() {
    let shell = shell();
    let ratios = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&ratios);
    let weak = shell.as_weak();
    shell.on_inspector_split_ratio_set(move |ratio| {
        observed.borrow_mut().push(ratio);
        weak.unwrap().set_inspector_structure_ratio(ratio);
    });
    let selected = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&selected);
    shell.on_viewport_select(move |x, y, _, _| observed.borrow_mut().push((x, y)));
    let structure = element(&shell, "ModelInspector::structure-pane");
    let old_height = structure.size().height;
    let position = center(&element(&shell, "InspectorSplitLayout::divider"));
    let end = LogicalPosition::new(position.x, position.y + 50.0);
    shell
        .window()
        .dispatch_event(WindowEvent::PointerMoved { position });
    shell.window().dispatch_event(WindowEvent::PointerPressed {
        position,
        button: PointerEventButton::Left,
    });
    shell
        .window()
        .dispatch_event(WindowEvent::PointerMoved { position: end });
    shell.window().dispatch_event(WindowEvent::PointerReleased {
        position: end,
        button: PointerEventButton::Left,
    });
    assert!(
        !ratios.borrow().is_empty(),
        "drag must emit a layout intent"
    );
    assert!(
        element(&shell, "ModelInspector::structure-pane")
            .size()
            .height
            > old_height + 40.0
    );
    assert!(
        selected.borrow().is_empty(),
        "divider must not select geometry"
    );
    key(&shell, Key::End.into());
    assert!((shell.get_inspector_structure_ratio() - 0.85).abs() < 0.001);
    key(&shell, Key::Home.into());
    assert!((shell.get_inspector_structure_ratio() - 0.15).abs() < 0.001);
    let split = element(&shell, "ModelInspector::split");
    let structure = element(&shell, "ModelInspector::structure-pane");
    let properties = element(&shell, "ModelInspector::properties-pane");
    assert!(structure.size().height > 0.0 && properties.size().height > 0.0);
    assert!(structure.size().height + properties.size().height <= split.size().height + 0.5);
}

#[test]
fn collapsed_structure_releases_space_without_changing_split_memory() {
    let shell = shell();
    shell.set_inspector_structure_ratio(0.6);
    let original_properties = element(&shell, "ModelInspector::properties-pane")
        .size()
        .height;
    shell.set_inspector_structure_collapsed(true);
    let collapsed = element(&shell, "ModelInspector::structure-pane");
    assert!(collapsed.size().height <= 32.5);
    assert!(
        element(&shell, "ModelInspector::properties-pane")
            .size()
            .height
            > original_properties + 50.0
    );
    assert!((shell.get_inspector_structure_ratio() - 0.6).abs() < 0.001);
    shell.set_inspector_structure_collapsed(false);
    let restored = element(&shell, "ModelInspector::properties-pane")
        .size()
        .height;
    assert!((restored - original_properties).abs() < 0.5);
}
