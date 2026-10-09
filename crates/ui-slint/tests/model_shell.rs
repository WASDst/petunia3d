//! Regressões físicas da extração MODEL: callbacks, bindings e condições de seleção.
//! Exercita o componente composto no shell; não substitui Geometry/Undo nem gates visuais.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use petunia_ui_slint::{PetuniaSlintShell, SceneItem};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize, ModelRc, VecModel};

fn shell() -> PetuniaSlintShell {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 1200.0));
    shell.set_active_workspace("MODEL".into());
    shell.set_object_has_selection(true);
    shell.set_selection_domain("OBJECT".into());
    shell.global::<petunia_ui_slint::Motion>().set_reduced(true);
    shell.show().expect("headless window");
    shell
}

fn element(shell: &PetuniaSlintShell, label: &str, role: AccessibleRole) -> ElementHandle {
    ElementHandle::find_by_accessible_label(shell, label)
        .find(|item| item.accessible_role() == Some(role) && item.computed_opacity() > 0.05)
        .unwrap_or_else(|| panic!("reachable {role:?}: {label}"))
}

fn click(shell: &PetuniaSlintShell, item: &ElementHandle) {
    let origin = item.absolute_position();
    let size = item.size();
    assert!(size.width > 0.0 && size.height > 0.0);
    let position = LogicalPosition::new(origin.x + size.width * 0.5, origin.y + size.height * 0.5);
    assert!(position.x > 0.0 && position.y > 0.0);
    assert!(position.x < shell.window().size().width as f32);
    assert!(position.y < shell.window().size().height as f32);
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

fn key(shell: &PetuniaSlintShell, text: &str) {
    shell
        .window()
        .dispatch_event(WindowEvent::KeyPressed { text: text.into() });
    shell
        .window()
        .dispatch_event(WindowEvent::KeyReleased { text: text.into() });
}

fn type_text(shell: &PetuniaSlintShell, text: &str) {
    for character in text.chars() {
        key(shell, &character.to_string());
    }
}

fn enter(shell: &PetuniaSlintShell) {
    let text: slint::SharedString = slint::platform::Key::Return.into();
    key(shell, text.as_str());
}

#[test]
fn parts_query_and_actions_survive_workspace_switch_and_resize() {
    let shell = shell();
    shell.set_model_parts_open(true);
    shell.set_label_search_parts("Search model parts".into());
    shell.set_label_selected_parts_only("Model selected only".into());
    shell.set_label_sort_parts("Sort model parts".into());
    shell.set_label_hide_part("Hide model part".into());
    shell.set_parts_items(ModelRc::new(VecModel::from(vec![SceneItem {
        id: "part-a".into(),
        name: "Part A".into(),
        visible: true,
        ..SceneItem::default()
    }])));
    let queries = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&queries);
    shell.on_parts_query_changed(move |query| observed.borrow_mut().push(query.to_string()));
    let hidden = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&hidden);
    shell.on_scene_toggle_visibility(move |id| observed.borrow_mut().push(id.to_string()));
    let filters = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&filters);
    shell.on_parts_selected_only_changed(move |value| observed.borrow_mut().push(value));

    for width in [1280.0, 1024.0] {
        shell.set_active_workspace("UV".into());
        shell.set_active_workspace("MODEL".into());
        shell.window().set_size(LogicalSize::new(width, 1200.0));
        shell.set_parts_query("".into());
        queries.borrow_mut().clear();
        hidden.borrow_mut().clear();
        filters.borrow_mut().clear();
        click(
            &shell,
            &element(&shell, "Search model parts", AccessibleRole::TextInput),
        );
        type_text(&shell, "cube");
        assert_eq!(
            shell.get_parts_query(),
            "cube",
            "two-way query reaches the shell"
        );
        assert_eq!(queries.borrow().last().map(String::as_str), Some("cube"));
        click(
            &shell,
            &element(&shell, "Model selected only", AccessibleRole::Button),
        );
        click(
            &shell,
            &element(&shell, "Hide model part", AccessibleRole::Button),
        );
        assert_eq!(*filters.borrow(), vec![true]);
        assert_eq!(*hidden.borrow(), vec!["part-a"]);
    }
}

#[test]
fn transform_numeric_commit_preserves_validation_return_and_selection_scope() {
    let shell = shell();
    shell.set_model_transform_open(true);
    shell.set_pos_x(3.5);
    let label = format!("{} X", petunia_ui_slint::tr::lookup("sl.position"));
    let field = element(&shell, &label, AccessibleRole::Spinbox);
    assert_eq!(field.accessible_value().as_deref(), Some("3.5"));
    let accepted = Rc::new(Cell::new(false));
    let decision = Rc::clone(&accepted);
    let values = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&values);
    shell.on_transform_text_committed(move |kind, axis, text| {
        observed
            .borrow_mut()
            .push((kind.to_string(), axis, text.to_string()));
        decision.get()
    });
    click(&shell, &field);
    assert!(
        field
            .query_descendants()
            .match_accessible_role(AccessibleRole::TextInput)
            .find_first()
            .is_some(),
        "click opens the numeric editor before typing"
    );
    type_text(&shell, "7.25");
    enter(&shell);
    assert_eq!(*values.borrow(), vec![("pos".into(), 0, "7.25".into())]);
    assert!(
        !field
            .query_descendants()
            .match_accessible_role(AccessibleRole::TextInput)
            .find_all()
            .is_empty(),
        "rejected input keeps editing active"
    );
    accepted.set(true);
    enter(&shell);
    assert_eq!(values.borrow().len(), 2, "one commit for each Enter");
    assert!(
        field
            .query_descendants()
            .match_accessible_role(AccessibleRole::TextInput)
            .find_all()
            .is_empty(),
        "accepted input leaves editing"
    );
    shell.set_selection_domain("FACE".into());
    assert!(
        ElementHandle::find_by_accessible_label(&shell, &label)
            .next()
            .is_none()
    );
    shell.set_selection_domain("OBJECT".into());
    assert!(
        ElementHandle::find_by_accessible_label(&shell, &label)
            .next()
            .is_some()
    );
    shell.set_object_has_selection(false);
    assert!(
        ElementHandle::find_by_accessible_label(&shell, &label)
            .next()
            .is_none()
    );
}

#[test]
fn material_actions_forward_the_current_slot_after_host_updates() {
    let shell = shell();
    shell.set_model_material_open(true);
    shell.set_label_material_new("Create model material".into());
    shell.set_label_material_duplicate("Duplicate model material".into());
    shell.set_material_slots(ModelRc::new(VecModel::from(vec![
        "Material A".into(),
        "Material B".into(),
        "Material C".into(),
    ])));
    let created = Rc::new(Cell::new(0));
    let observed = Rc::clone(&created);
    shell.on_create_material(move || observed.set(observed.get() + 1));
    let duplicates = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&duplicates);
    shell.on_duplicate_material(move |slot| observed.borrow_mut().push(slot));
    click(
        &shell,
        &element(&shell, "Create model material", AccessibleRole::Button),
    );
    for slot in [0, 2] {
        shell.set_active_material_slot(slot);
        click(
            &shell,
            &element(&shell, "Duplicate model material", AccessibleRole::Button),
        );
    }
    assert_eq!(created.get(), 1);
    assert_eq!(*duplicates.borrow(), vec![0, 2]);
}

#[test]
fn combine_preserves_checked_state_and_command_dispatch() {
    let shell = shell();
    shell.set_model_object_open(true);
    shell.set_boolean_operand_name("Other part".into());
    shell.set_boolean_keep_parts(true);
    let toggles = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&toggles);
    shell.on_boolean_keep_parts_set(move |value| observed.borrow_mut().push(value));
    let commands = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&commands);
    shell.on_command_executed(move |id| observed.borrow_mut().push(id.to_string()));
    let checkbox = element(
        &shell,
        &petunia_ui_slint::tr::lookup("sl.keep_parts"),
        AccessibleRole::Checkbox,
    );
    assert_eq!(checkbox.accessible_checked(), Some(true));
    click(&shell, &checkbox);
    assert_eq!(*toggles.borrow(), vec![false]);
    shell.set_boolean_keep_parts(false);
    assert_eq!(checkbox.accessible_checked(), Some(false));
    click(
        &shell,
        &element(
            &shell,
            &petunia_ui_slint::tr::lookup("sl.fuse"),
            AccessibleRole::Button,
        ),
    );
    assert_eq!(*commands.borrow(), vec!["model.fuse"]);
}
