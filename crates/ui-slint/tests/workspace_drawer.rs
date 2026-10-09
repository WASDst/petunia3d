//! Regressões escritas para U03; execução adiada com a bateria da wave UI.
use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use petunia_ui_slint::PetuniaSlintShell;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize, ModelRc, VecModel};

fn shell() -> PetuniaSlintShell {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 800.0));
    shell.set_active_workspace("PAINT".into());
    shell.set_asset_library_visible(true);
    shell.set_asset_library_height(360.0);
    shell.set_label_project_asset_library("Project assets".into());
    shell.global::<petunia_ui_slint::Motion>().set_reduced(true);
    shell.show().expect("headless window");
    shell
}

fn element(shell: &PetuniaSlintShell, label: &str, role: AccessibleRole) -> ElementHandle {
    ElementHandle::find_by_accessible_label(shell, label)
        .find(|element| {
            element.accessible_role() == Some(role) && element.computed_opacity() > 0.05
        })
        .unwrap_or_else(|| panic!("drawer control {label}"))
}

fn click(shell: &PetuniaSlintShell, element: &ElementHandle) {
    let origin = element.absolute_position();
    let size = element.size();
    assert!(size.width > 0.0 && size.height > 0.0);
    let position = LogicalPosition::new(origin.x + size.width / 2.0, origin.y + size.height / 2.0);
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
fn presets_palette_and_assets_keep_routes_and_keyboard_tab_focus() {
    let shell = shell();
    shell.set_brush_presets(ModelRc::new(VecModel::from(vec![
        "Ink".into(),
        "Soft".into(),
    ])));
    shell.set_brush_preset_index(0);
    let actions = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&actions);
    shell.on_brush_preset_applied(move |index| {
        observed.borrow_mut().push(format!("preset:{index}"))
    });
    let observed = Rc::clone(&actions);
    shell.on_command_file_action(move |id| observed.borrow_mut().push(id.to_string()));
    click(&shell, &element(&shell, "Soft", AccessibleRole::Button));
    assert_eq!(*actions.borrow(), vec!["preset:1"]);
    // Enter after pointer selection remains on the same semantic action.
    key(&shell, Key::Return.into());
    assert_eq!(actions.borrow().len(), 2);
    let presets = petunia_ui_slint::tr::lookup("sl.brush_presets");
    click(&shell, &element(&shell, &presets, AccessibleRole::Tab));
    key(&shell, Key::RightArrow.into());
    click(
        &shell,
        &element(
            &shell,
            &petunia_ui_slint::tr::lookup("sl.import"),
            AccessibleRole::Button,
        ),
    );
    assert_eq!(
        actions.borrow().last().map(String::as_str),
        Some("palette.import")
    );
    shell.set_asset_query("keep filter".into());
    shell.set_asset_only_favorites(true);
    click(
        &shell,
        &element(&shell, "Project assets", AccessibleRole::Tab),
    );
    shell.set_active_workspace("UV".into());
    shell.set_active_workspace("PAINT".into());
    assert_eq!(shell.get_asset_query(), "keep filter");
    assert!(shell.get_asset_only_favorites());
    assert!(
        element(&shell, "Project assets", AccessibleRole::Tab)
            .accessible_checked()
            .unwrap_or(false)
    );
}

#[test]
fn uv_drawer_actions_and_close_keep_existing_application_routes() {
    let shell = shell();
    shell.set_active_workspace("UV".into());
    shell.set_uv_stats("Overlaps: 2".into());
    let actions = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&actions);
    shell.on_command_executed(move |id| observed.borrow_mut().push(id.to_string()));
    let observed = Rc::clone(&actions);
    shell.on_uv_equalize_texel_density(move || observed.borrow_mut().push("density".into()));
    click(
        &shell,
        &element(
            &shell,
            &petunia_ui_slint::tr::lookup("sl.pack_islands"),
            AccessibleRole::Button,
        ),
    );
    click(
        &shell,
        &element(
            &shell,
            &petunia_ui_slint::tr::lookup("sl.equalize_texel_density"),
            AccessibleRole::Button,
        ),
    );
    assert_eq!(*actions.borrow(), vec!["uv.pack_islands", "density"]);
    let observed = Rc::clone(&actions);
    shell.on_asset_library_requested(move || observed.borrow_mut().push("close".into()));
    let tab = petunia_ui_slint::tr::lookup("sl.uv_drawer_diagnostics");
    click(&shell, &element(&shell, &tab, AccessibleRole::Tab));
    key(&shell, Key::Escape.into());
    assert_eq!(actions.borrow().last().map(String::as_str), Some("close"));
}

#[test]
fn animate_drawer_transport_dispatches_existing_procedural_actions() {
    let shell = shell();
    shell.set_animate_texts(petunia_ui_slint::AnimateTexts {
        play: "Play preview".into(),
        pause: "Pause preview".into(),
        keep_live: "Keep live".into(),
        apply_now: "Apply now".into(),
        ..Default::default()
    });
    shell.set_animate_has_creature(true);
    shell.set_animate_has_motion(true);
    shell.set_active_workspace("ANIMATE".into());
    let actions = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&actions);
    shell.on_animate_action(move |action, argument, value| {
        observed
            .borrow_mut()
            .push((action.to_string(), argument.to_string(), value));
    });
    let transport = ElementHandle::find_by_element_id(&shell, "AnimateDrawerContent::transport")
        .next()
        .expect("transport in the drawer");
    // Scope the query to the drawer; an inspector or old in-canvas copy cannot satisfy it.
    for label in ["Play preview", "Keep live", "Apply now"] {
        let button = transport
            .query_descendants()
            .match_accessible_role(AccessibleRole::Button)
            .match_predicate(move |element| {
                element.accessible_label().as_deref() == Some(label)
            })
            .find_first()
            .expect("drawer transport action");
        click(&shell, &button);
    }
    assert_eq!(
        *actions.borrow(),
        vec![
            ("toggle-play".into(), "".into(), 0.0),
            ("apply-now".into(), "keep".into(), 0.0),
            ("apply-now".into(), "convert".into(), 0.0),
        ]
    );
}
