//! Input físico do inspector PAINT composto: strokes, seleção e callbacks atuais.
//! Gates headless; não substituem avaliação visual, Paint/Undo ou acessibilidade nativa.

use std::cell::RefCell;
use std::rc::Rc;

use i_slint_backend_testing::{AccessibleRole, ElementHandle};
use petunia_ui_slint::{PaintLayerEntry, PetuniaSlintShell};
use slint::platform::{PointerEventButton, WindowEvent};
use slint::{ComponentHandle, LogicalPosition, LogicalSize, ModelRc, VecModel};

fn shell() -> PetuniaSlintShell {
    i_slint_backend_testing::init_no_event_loop();
    let shell = PetuniaSlintShell::new().expect("Slint shell");
    petunia_ui_slint::tr::install(&shell, "en");
    shell.window().set_size(LogicalSize::new(1280.0, 4000.0));
    shell.set_active_workspace("PAINT".into());
    shell.set_model_parts_open(true);
    shell.global::<petunia_ui_slint::Motion>().set_reduced(true);
    shell.show().expect("headless window");
    shell
}

fn send(shell: &PetuniaSlintShell, position: LogicalPosition, down: bool) {
    if down {
        shell
            .window()
            .dispatch_event(WindowEvent::PointerMoved { position });
    }
    shell.window().dispatch_event(if down {
        WindowEvent::PointerPressed {
            position,
            button: PointerEventButton::Left,
        }
    } else {
        WindowEvent::PointerReleased {
            position,
            button: PointerEventButton::Left,
        }
    });
}

fn point(item: &ElementHandle, fraction: f32) -> LogicalPosition {
    let origin = item.absolute_position();
    let size = item.size();
    assert!(size.width > 0.0 && size.height > 0.0);
    LogicalPosition::new(
        origin.x + size.width * fraction,
        origin.y + size.height * 0.5,
    )
}

fn control(shell: &PetuniaSlintShell, label: &str, role: AccessibleRole) -> ElementHandle {
    ElementHandle::find_by_accessible_label(shell, label)
        .find(|item| item.accessible_role() == Some(role) && item.computed_opacity() > 0.05)
        .unwrap_or_else(|| panic!("PAINT {role:?}: {label}"))
}

fn button(shell: &PetuniaSlintShell, label: &str) -> ElementHandle {
    control(shell, label, AccessibleRole::Button)
}

fn click(shell: &PetuniaSlintShell, item: &ElementHandle) {
    let position = point(item, 0.5);
    assert!(
        position.y < shell.window().size().height as f32,
        "test target must be in the viewport"
    );
    send(shell, position, true);
    send(shell, position, false);
}

#[test]
fn canvas_stroke_and_selection_keep_coordinates_and_exclusive_routes() {
    let shell = shell();
    shell.set_active_tool("brush".into());
    let pixels = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(256, 256);
    shell.set_paint_canvas_image(slint::Image::from_rgba8(pixels));
    let strokes = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&strokes);
    shell.on_paint_2d_stroke(move |x, y, phase, shift, raster| {
        observed.borrow_mut().push((x, y, phase, shift, raster))
    });
    let selections = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&selections);
    shell.on_paint_2d_select(move |x0, y0, x1, y1, extend, subtract, island| {
        observed
            .borrow_mut()
            .push((x0, y0, x1, y1, extend, subtract, island))
    });

    for width in [1280.0, 1024.0] {
        shell.set_active_workspace("UV".into());
        shell.set_active_workspace("PAINT".into());
        shell.window().set_size(LogicalSize::new(width, 4000.0));
        shell.set_active_tool("brush".into());
        let canvas = ElementHandle::find_by_element_id(&shell, "PaintInspector::paint-canvas")
            .next()
            .expect("PAINT canvas");
        let start = point(&canvas, 0.25);
        let end = point(&canvas, 0.75);
        strokes.borrow_mut().clear();
        send(&shell, start, true);
        shell
            .window()
            .dispatch_event(WindowEvent::PointerMoved { position: end });
        send(&shell, end, false);
        let events = strokes.borrow();
        assert_eq!(
            events.iter().map(|e| e.2).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert!((events[0].0 - 0.25).abs() < 0.001);
        assert!((events[0].1 - 0.5).abs() < 0.001);
        assert!((events[2].0 - 0.75).abs() < 0.001);
        assert!(!events[0].3 && (events[0].4 - 256.0).abs() < 0.001);
        drop(events);
        shell.set_active_tool("select".into());
        strokes.borrow_mut().clear();
        selections.borrow_mut().clear();
        send(&shell, start, true);
        shell
            .window()
            .dispatch_event(WindowEvent::PointerMoved { position: end });
        send(&shell, end, false);
        assert!(strokes.borrow().is_empty(), "selection must not paint");
        let events = selections.borrow();
        assert_eq!(events.len(), 1);
        let (x0, y0, x1, y1, extend, subtract, island) = events[0];
        assert!((x0 - 0.25).abs() < 0.001 && (x1 - 0.75).abs() < 0.001);
        assert!((y0 - 0.5).abs() < 0.001 && (y1 - 0.5).abs() < 0.001);
        assert!(!extend && !subtract && !island);
    }
}

#[test]
fn layer_actions_follow_replaced_model_ids_and_opacity() {
    let shell = shell();
    let visibility = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&visibility);
    shell.on_paint_layer_visibility_toggled(move |id| observed.borrow_mut().push(id.to_string()));
    let opacity = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&opacity);
    shell.on_paint_layer_opacity_set(move |id, value| {
        observed.borrow_mut().push((id.to_string(), value))
    });
    for id in ["layer-before", "layer-after"] {
        shell.set_paint_layers(ModelRc::new(VecModel::from(vec![PaintLayerEntry {
            id: id.into(),
            name: id.into(),
            active: true,
            visible: true,
            opacity: 0.25,
            ..PaintLayerEntry::default()
        }])));
        click(
            &shell,
            &control(
                &shell,
                &petunia_ui_slint::tr::lookup("sl.layer_visible"),
                AccessibleRole::Checkbox,
            ),
        );
        let slider = ElementHandle::find_by_element_id(&shell, "PaintInspector::layer-opacity")
            .next()
            .expect("active layer opacity");
        let position = point(&slider, 0.75);
        send(&shell, position, true);
        send(&shell, position, false);
        let values = opacity.borrow();
        let (received_id, value) = values.last().expect("opacity intent");
        assert_eq!(received_id, id);
        assert!((value - 0.75).abs() < 0.05);
    }
    assert_eq!(*visibility.borrow(), vec!["layer-before", "layer-after"]);
}

#[test]
fn effects_brush_parameters_and_canvas_binding_reach_shell() {
    let shell = shell();
    let effects = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&effects);
    shell.on_paint_effect_layer_added(move |kind| observed.borrow_mut().push(kind.to_string()));
    click(&shell, &button(&shell, "Invert"));
    assert_eq!(*effects.borrow(), vec!["Invert"]);
    let flow = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&flow);
    shell.on_brush_param_changed(move |key, value| {
        observed.borrow_mut().push((key.to_string(), value))
    });
    shell.set_brush_flow(0.2);
    let label = petunia_ui_slint::tr::lookup("sl.brush_flow") + " +";
    click(&shell, &button(&shell, &label));
    shell.set_brush_flow(0.6);
    click(&shell, &button(&shell, &label));
    let values = flow.borrow();
    assert_eq!(values.len(), 2);
    assert!(values.iter().all(|(key, _)| key == "flow"));
    assert!((values[0].1 - 0.3).abs() < 0.001 && (values[1].1 - 0.7).abs() < 0.001);
    drop(values);
    assert!(!shell.get_paint_canvas_panel_visible());
    click(
        &shell,
        &button(
            &shell,
            &petunia_ui_slint::tr::lookup("sl.paint_canvas_open"),
        ),
    );
    assert!(
        shell.get_paint_canvas_panel_visible(),
        "two-way canvas panel binding"
    );
}
