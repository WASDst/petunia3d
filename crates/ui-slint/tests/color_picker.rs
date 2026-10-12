//! Color-picker validation-state regressions, prepared but not executed.
//! These exercise the composed Slint controls; color authoring/Undo is separate.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use petunia_ui_slint::PetuniaSlintShell;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalSize};

const CANONICAL_HEX: &str = "#E96A00";

fn settle() {
    i_slint_backend_testing::mock_elapsed_time(Duration::from_millis(1));
}

fn shell() -> PetuniaSlintShell {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.global::<petunia_ui_slint::Motion>().set_reduced(true);
    shell.set_part_color_hex(CANONICAL_HEX.into());
    shell.set_part_color_preview(slint::Color::from_rgb_u8(233, 106, 0));
    shell.set_part_color_value(1.0);
    shell.set_part_color_popover_open(true);
    // Validation deliberately leaves the canonical property unchanged for a
    // same-color lowercase draft. This tests presentation, not HEX parsing.
    shell.on_part_color_hex_entered(|text| text.as_str() == "e96a00");
    shell.show().expect("headless window");
    settle();
    shell
}

fn element(shell: &PetuniaSlintShell, label: &str, role: AccessibleRole) -> ElementHandle {
    ElementHandle::find_by_accessible_label(shell, label)
        .find(|item| item.accessible_role() == Some(role) && item.computed_opacity() > 0.05)
        .unwrap_or_else(|| panic!("reachable picker control {label}"))
}

fn hex_input(shell: &PetuniaSlintShell) -> ElementHandle {
    element(shell, "Hexadecimal color", AccessibleRole::TextInput)
}

fn key(shell: &PetuniaSlintShell, text: slint::SharedString) {
    shell
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    shell
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text });
    settle();
}

fn enter_hex(shell: &PetuniaSlintShell, draft: &str) {
    settle();
    hex_input(shell).mock_single_click(PointerEventButton::Left);
    shell.window().dispatch_event(WindowEvent::KeyPressed {
        text: Key::Control.into(),
    });
    key(shell, "a".into());
    shell.window().dispatch_event(WindowEvent::KeyReleased {
        text: Key::Control.into(),
    });
    for character in draft.chars() {
        key(shell, character.to_string().into());
    }
    key(shell, Key::Return.into());
}

fn enter_invalid_hex(shell: &PetuniaSlintShell) {
    enter_hex(shell, "invalid");
    let input = hex_input(shell);
    assert_eq!(input.accessible_value().unwrap().as_str(), "invalid");
    assert!(
        input
            .accessible_description()
            .unwrap()
            .starts_with("Invalid color"),
        "rejected HEX must expose its error before a recovery action"
    );
}

fn assert_clean_hex(shell: &PetuniaSlintShell, expected: &str) {
    settle();
    let input = hex_input(shell);
    assert_eq!(input.accessible_value().unwrap().as_str(), expected);
    assert_eq!(
        input.accessible_description().unwrap().as_str(),
        "Enter #RGB or #RRGGBB and press Enter",
        "valid color must replace stale validation feedback"
    );
}

#[test]
fn same_color_preset_reset_and_hsv_action_restore_hex_draft_and_validation() {
    let shell = shell();
    let presets = Rc::new(Cell::new(0));
    let observed = Rc::clone(&presets);
    shell.on_part_color_preset_clicked(move |color| {
        assert_eq!(color, slint::Color::from_rgb_u8(233, 106, 0));
        observed.set(observed.get() + 1);
        // Selecting the already selected color does not change the HEX prop.
    });
    let resets = Rc::new(Cell::new(0));
    let observed = Rc::clone(&resets);
    shell.on_part_color_reset_requested(move || observed.set(observed.get() + 1));
    let adjustments = Rc::new(Cell::new(0));
    let observed = Rc::clone(&adjustments);
    shell.on_part_color_hsv_adjusted(move |channel, delta| {
        assert_eq!((channel, delta), (2, 1.0));
        observed.set(observed.get() + 1);
        // Incrementing value already at 100% also preserves the canonical color.
    });

    enter_invalid_hex(&shell);
    element(
        &shell,
        "Color Swatch · RGB: 233 / 106 / 0",
        AccessibleRole::Button,
    )
    .invoke_accessible_default_action();
    assert_eq!(presets.get(), 1);
    assert_clean_hex(&shell, CANONICAL_HEX);

    enter_invalid_hex(&shell);
    element(&shell, "Reset to default color", AccessibleRole::Button)
        .invoke_accessible_default_action();
    assert_eq!(resets.get(), 1);
    assert_clean_hex(&shell, CANONICAL_HEX);

    enter_invalid_hex(&shell);
    element(&shell, "Brightness / value (%)", AccessibleRole::Spinbox)
        .invoke_accessible_increment_action();
    assert_eq!(adjustments.get(), 1);
    assert_clean_hex(&shell, CANONICAL_HEX);
}

#[test]
fn canonical_projection_reopening_and_same_color_hex_acceptance_reset_the_editor() {
    let shell = shell();
    enter_invalid_hex(&shell);
    shell.set_part_color_hex("#00D2FF".into());
    settle();
    assert_clean_hex(&shell, "#00D2FF");

    enter_invalid_hex(&shell);
    shell.set_part_color_popover_open(false);
    settle();
    assert!(
        ElementHandle::find_by_accessible_label(&shell, "Hexadecimal color")
            .all(|item| item.accessible_role() != Some(AccessibleRole::TextInput)),
        "closing must remove the old picker before reopening"
    );
    shell.set_part_color_popover_open(true);
    settle();
    assert_clean_hex(&shell, "#00D2FF");

    shell.set_part_color_hex(CANONICAL_HEX.into());
    settle();
    enter_hex(&shell, "e96a00");
    assert_clean_hex(&shell, CANONICAL_HEX);
    assert_eq!(shell.get_part_color_hex().as_str(), CANONICAL_HEX);
}
