use petunia_core::{EditReferenceCmd, ReferenceOp, Workspace};
use petunia_project::{RefAxis, ReferenceImage};
use petunia_ui_cxxqt::editor::QtEditor;
use serde_json::{Value, json};
fn request(editor: &mut QtEditor, value: Value) {
    editor.request(&value).unwrap();
}
fn editor() -> QtEditor {
    let mut e = QtEditor::default();
    let id = e.state.project.assets[0].id.to_string();
    request(&mut e, json!({"action":"select","id":id}));
    e
}
#[test]
fn native_workspace_and_transform_use_document_and_undo() {
    let mut e = editor();
    let before = e.state.project.project.clone();
    for (id, workspace) in [
        ("PAINT", Workspace::Paint),
        ("UV", Workspace::Uv),
        ("ANIMATE", Workspace::Animate),
        ("POLY", Workspace::Model),
        ("DRAW", Workspace::Model),
    ] {
        request(&mut e, json!({"action":"workspace","id":id}));
        assert_eq!(e.state.workspace, workspace);
    }
    request(&mut e, json!({"action":"workspace","id":"POLY"}));
    request(
        &mut e,
        json!({"action":"transform","kind":"position","axis":0,"text":"2+3"}),
    );
    assert_eq!(e.state.project.assets[0].position[0], 5.);
    let moved = e.state.project.assets[0].world_positions();
    let original = before.assets[0].world_positions();
    for (actual, source) in moved.iter().zip(&original) {
        assert!(actual.distance(*source + glam::Vec3::X * 5.) < 1e-4);
    }
    assert!(e.undo());
    assert_eq!(
        e.state.project.assets[0].position,
        before.assets[0].position
    );
    assert_eq!(e.state.project.assets[0].world_positions(), original);
    assert!(e.redo());
    assert_eq!(e.state.project.assets[0].position[0], 5.);
    assert_eq!(e.state.project.assets[0].world_positions(), moved);
}
#[test]
fn reference_edits_are_transactional_locked_and_revision_checked() {
    let mut e = editor();
    let image = ReferenceImage::from_rgba("fixture".into(), 2, 2, vec![255; 16]);
    e.execute_command(&EditReferenceCmd {
        axis: RefAxis::Front,
        expected_revision: None,
        op: ReferenceOp::SetSlot(image),
    })
    .unwrap();
    assert_eq!(e.state.project.project.references.len(), 1);
    let revision = e.state.project.refs[0].revision;
    let initial_opacity = e.state.project.refs[0].opacity;
    request(
        &mut e,
        json!({"action":"set_reference_param","axis_str":"front","param":"opacity","val":0.25}),
    );
    assert_eq!(e.state.project.refs[0].opacity, 0.25);
    assert_eq!(e.snapshot()["reference_front"]["opacity"], 0.25);
    assert_eq!(e.snapshot()["reference_front"]["visible"], true);
    assert!(
        e.execute_command(&EditReferenceCmd {
            axis: RefAxis::Front,
            expected_revision: Some(revision),
            op: ReferenceOp::Remove
        })
        .is_err()
    );
    assert!(e.undo());
    assert_eq!(e.state.project.refs[0].opacity, initial_opacity);
    assert_eq!(
        e.state.project.project.references[0].opacity,
        initial_opacity
    );
    request(
        &mut e,
        json!({"action":"toggle_reference_lock","axis_str":"front"}),
    );
    assert_eq!(e.snapshot()["reference_front"]["locked"], true);
    let frozen = e.state.project.refs.clone();
    assert!(
        e.execute_command(&EditReferenceCmd {
            axis: RefAxis::Front,
            expected_revision: None,
            op: ReferenceOp::Remove
        })
        .is_err()
    );
    assert_eq!(
        serde_json::to_value(&e.state.project.refs).unwrap(),
        serde_json::to_value(&frozen).unwrap()
    );
}
#[test]
fn load_failure_preserves_document_and_success_clears_old_history() {
    let mut e = editor();
    let id = e.state.project.assets[0].id;
    request(&mut e, json!({"action":"duplicate"}));
    assert!(e.snapshot()["can_undo"].as_bool().unwrap());
    let path = std::env::temp_dir().join(format!("qt-parity-{}.p3d", uuid::Uuid::new_v4()));
    request(&mut e, json!({"action":"project.save","path":path}));
    let count = e.state.project.assets.len();
    assert!(e.open(&path.with_extension("missing")).is_err());
    assert_eq!(e.state.project.assets.len(), count);
    assert_eq!(e.state.project.assets[0].id, id);
    request(&mut e, json!({"action":"delete"}));
    e.open(&path).unwrap();
    assert_eq!(e.state.project.assets.len(), count);
    assert!(!e.snapshot()["can_undo"].as_bool().unwrap());
    assert!(e.state.session.selection.assets.is_empty());
    std::fs::remove_file(path).unwrap();
}
#[test]
fn uv_drag_rejects_stale_document_instead_of_editing_new_selection() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"UV"}));
    let snapshot = e.snapshot();
    request(&mut e, json!({"action":"duplicate"}));
    let revision = e.state.project.project.revision_id();
    assert!(e.request(&json!({"action":"uv_drag_commit","revision":snapshot["document_revision"],"asset":snapshot["active_asset_id"],"du":0.1,"dv":0.1})).is_err());
    assert_eq!(e.state.project.project.revision_id(), revision);
}
#[test]
fn paint_canvas_writes_pixels_and_a_single_undo_restores_stroke() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"PAINT"}));
    request(&mut e, json!({"action":"add_paint_layer"}));
    request(&mut e, json!({"action":"tool","id":"brush"}));
    let before = e.state.project.active().unwrap().paint_stack.clone();
    for (phase, x) in [(0, 0.4), (1, 0.5), (2, 0.6)] {
        request(
            &mut e,
            json!({"action":"paint_stroke","x":x,"y":0.5,"phase":phase}),
        );
    }
    assert_ne!(e.state.project.active().unwrap().paint_stack, before);
    assert!(e.undo());
    assert_eq!(e.state.project.active().unwrap().paint_stack, before);
}
#[test]
fn object_color_is_saved_undoable_and_cannot_mutate_locked_assets() {
    let mut e = editor();
    let id = e.state.project.assets[0].id.to_string();
    let initial = e.state.project.assets[0].selection_overlay_color;
    request(
        &mut e,
        json!({"action":"part_selection_color","id":id,"color":[1,2,3]}),
    );
    assert_eq!(
        e.state.project.assets[0].selection_overlay_color,
        Some([1, 2, 3])
    );
    assert!(e.undo());
    assert_eq!(e.state.project.assets[0].selection_overlay_color, initial);
    request(&mut e, json!({"action":"lock","id":id}));
    let frozen = e.state.project.project.revision_id();
    let changed = e
        .request(&json!({"action":"part_selection_color","id":id,"color":[3,4,5]}))
        .unwrap();
    assert!(!changed);
    assert_eq!(e.state.project.project.revision_id(), frozen);
}

#[test]
fn native_animate_catalog_creates_real_rig_motion_and_evaluates_playhead() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"ANIMATE"}));
    let catalog = e.snapshot()["animate"]["creature_presets"]
        .as_array()
        .unwrap()
        .clone();
    let creature = catalog
        .iter()
        .find(|row| row["available"].as_bool().unwrap_or(true))
        .unwrap()["id"]
        .clone();
    request(
        &mut e,
        json!({"action":"animate","id":"add-creature","arg":creature}),
    );
    request(&mut e, json!({"action":"animate","id":"fit-to-model"}));
    let snapshot = e.snapshot();
    assert!(!snapshot["animate"]["bones"].as_array().unwrap().is_empty());
    let motion = snapshot["animate"]["catalog"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["available"].as_bool().unwrap_or(true))
        .unwrap()["id"]
        .clone();
    request(
        &mut e,
        json!({"action":"animate","id":"add-motion","arg":motion}),
    );
    assert!(e.snapshot()["animate"]["has_motion"].as_bool().unwrap());
    request(&mut e, json!({"action":"animate","id":"seek","value":0.5}));
    assert!((e.snapshot()["animate"]["progress"].as_f64().unwrap() - 0.5).abs() < 0.01);
    e.controller.render_viewport();
    assert!(e.controller.viewport.pose.is_some());
    let bone = e.snapshot()["animate"]["bones"][0]["id"].clone();
    request(
        &mut e,
        json!({"action":"animate","id":"select-bone","arg":bone}),
    );
    request(&mut e, json!({"action":"animate","id":"insert-keyframe"}));
    assert!(
        e.snapshot()["animate"]["has_keyframe_at_playhead"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn native_profile_creation_can_be_undone() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    request(&mut e, json!({"action":"modeling_mode","id":"DRAW"}));
    let original = serde_json::to_value(&e.state.project.project.profiles).unwrap();
    let before = e.state.project.project.revision_id();
    request(
        &mut e,
        json!({"action":"profile_rectangle","width":2.,"height":3.}),
    );
    assert!(e.controller.active_profile_id.is_some());
    assert_ne!(e.state.project.project.revision_id(), before);
    assert!(e.undo());
    assert_eq!(
        serde_json::to_value(&e.state.project.project.profiles).unwrap(),
        original
    );
    assert_ne!(e.state.project.project.revision_id(), before);
    assert!(e.redo());
    assert_eq!(
        e.state.project.project.profiles.len(),
        original.as_array().unwrap().len() + 1
    );
}

#[test]
fn split_camera_is_independent_and_picture_in_picture_uses_primary_bank() {
    let mut host = petunia_ui_cxxqt::render::opengl_viewport::QtViewportRenderer::new();
    let primary = host.app.state.session.camera.yaw;
    host.activate(2);
    host.orbit_camera(40., 0.);
    let split = host.app.state.session.camera.yaw;
    assert_ne!(primary, split);
    host.activate(1);
    assert_eq!(host.app.state.session.camera.yaw, primary);
    host.activate(3);
    host.orbit_camera(10., 0.);
    let pip = host.app.state.session.camera.yaw;
    host.activate(1);
    assert_eq!(host.app.state.session.camera.yaw, pip);
    host.activate(2);
    assert_eq!(host.app.state.session.camera.yaw, split);
}

#[test]
fn uv_commit_moves_the_original_selection_once_and_rejects_selection_changes() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"UV"}));
    request(&mut e, json!({"action":"command","id":"uv.unwrap_auto"}));
    let mesh = e.state.project.active().unwrap().mesh.clone();
    e.state.session.uv_selection.set_faces(&mesh, [0]);
    let snapshot = e.snapshot();
    let before = e.state.project.project.revision_id();
    request(
        &mut e,
        json!({"action":"uv_drag_commit","revision":snapshot["document_revision"],"asset":snapshot["active_asset_id"],"selection":snapshot["uv_selection_stamp"],"du":0.1,"dv":0.2}),
    );
    assert_ne!(e.state.project.project.revision_id(), before);
    let moved = e.state.project.active().unwrap().mesh.faces[0].uv.clone();
    assert_ne!(moved, mesh.faces[0].uv.clone());
    assert!(e.undo());
    for face in 0..mesh.faces.len() {
        assert_eq!(
            e.state.project.active().unwrap().mesh.faces[face].uv,
            mesh.faces[face].uv
        );
    }
    let restored_revision = e.state.project.project.revision_id();
    assert_ne!(restored_revision, before);
    let snapshot = e.snapshot();
    e.state.session.uv_selection.set_faces(&mesh, [1]);
    assert!(e.request(&json!({"action":"uv_drag_commit","revision":snapshot["document_revision"],"asset":snapshot["active_asset_id"],"selection":snapshot["uv_selection_stamp"],"du":0.1,"dv":0.1})).is_err());
    assert_eq!(e.state.project.project.revision_id(), restored_revision);
}

#[test]
fn native_point_selection_uses_shared_picking_and_can_clear_selection() {
    let mut e = editor();
    e.controller.resize_viewport(800, 600);
    request(&mut e, json!({"action":"workspace","id":"POLY"}));
    request(&mut e, json!({"action":"domain","id":"object"}));
    request(&mut e, json!({"action":"clear_selection"}));
    assert!(e.state.session.selection.assets.is_empty());
    request(
        &mut e,
        json!({"action":"viewport_select","x":0.5,"y":0.5,"extend":false}),
    );
    assert_eq!(e.state.session.selection.assets.len(), 1);
    request(
        &mut e,
        json!({"action":"viewport_select","x":0.01,"y":0.01,"extend":false}),
    );
    assert!(e.state.session.selection.assets.is_empty());
}

#[test]
fn reference_pixels_and_parameters_round_trip_through_project_files() {
    let mut e = editor();
    assert!(e.controller.load_reference_slot(
        "front",
        "fixture".into(),
        2,
        2,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255
        ]
    ));
    request(
        &mut e,
        json!({"action":"set_reference_param","axis_str":"front","param":"opacity","val":0.35}),
    );
    let path = std::env::temp_dir().join(format!("qt-reference-{}.p3d", uuid::Uuid::new_v4()));
    request(&mut e, json!({"action":"project.save","path":path}));
    e.new_document();
    assert!(e.state.project.refs.is_empty());
    e.open(&path).unwrap();
    assert_eq!(e.state.project.project.references.len(), 1);
    assert_eq!(e.state.project.refs[0].opacity, 0.35);
    assert_eq!(
        e.state.project.refs[0].rgba,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255
        ]
    );
    request(
        &mut e,
        json!({"action":"remove_reference_slot","axis_str":"front"}),
    );
    assert!(e.state.project.project.references.is_empty());
    assert!(e.state.project.refs.is_empty());
    assert!(e.undo());
    assert_eq!(e.state.project.project.references.len(), 1);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn idle_tick_does_not_request_redraw_and_motion_tick_advances() {
    let mut e = editor();
    assert!(!e.request(&json!({"action":"tick","dt":0.033})).unwrap());
    request(&mut e, json!({"action":"workspace","id":"ANIMATE"}));
    let preset = e.snapshot()["animate"]["creature_presets"][0]["id"].clone();
    request(
        &mut e,
        json!({"action":"animate","id":"add-creature","arg":preset}),
    );
    let motion = e.snapshot()["animate"]["catalog"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["available"] == true)
        .unwrap()["id"]
        .clone();
    request(
        &mut e,
        json!({"action":"animate","id":"add-motion","arg":motion}),
    );
    let before = e.state.session.animate.time;
    assert!(e.request(&json!({"action":"tick","dt":0.033})).unwrap());
    assert!(e.state.session.animate.time > before);
    request(&mut e, json!({"action":"animate","id":"toggle-play"}));
    assert!(!e.request(&json!({"action":"tick","dt":0.033})).unwrap());
}

#[test]
fn imported_palette_is_saved_and_restored_by_native_project_open() {
    let mut e = editor();
    let path = std::env::temp_dir().join(format!("qt-palette-{}.p3d", uuid::Uuid::new_v4()));
    let palette_path = path.with_extension("hex");
    std::fs::write(&palette_path, "#FF0000\n#0000FF\n").unwrap();
    request(
        &mut e,
        json!({"action":"import_palette","path":palette_path}),
    );
    let colors = e.state.project.palette.clone();
    assert_eq!(colors, vec![[1., 0., 0.], [0., 0., 1.]]);
    request(&mut e, json!({"action":"project.save","path":path}));
    e.new_document();
    e.open(&path).unwrap();
    assert_eq!(e.state.project.palette, colors);
    assert_eq!(e.state.project.project.palette, colors);
    assert!(!e.state.project.is_dirty());
    std::fs::remove_file(path).unwrap();
    std::fs::remove_file(palette_path).unwrap();
}

#[test]
fn tablet_pressure_tilt_and_eraser_do_not_overwrite_the_brush_preset() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"PAINT"}));
    request(&mut e, json!({"action":"add_paint_layer"}));
    request(&mut e, json!({"action":"tool","id":"brush"}));
    let t = &e.state.session.tools;
    let previous = (
        t.paint_strength,
        t.brush_flow,
        t.paint_brush_kind,
        t.brush_style.angle_deg,
        t.brush_style.roundness,
        t.active_tool.clone(),
    );
    request(
        &mut e,
        json!({"action":"tablet_input","pressure":0.5,"tilt_x":30.,"tilt_y":15.,"eraser":true}),
    );
    for phase in [0, 1, 2] {
        request(
            &mut e,
            json!({"action":"paint_stroke","x":0.5,"y":0.5,"phase":phase}),
        );
    }
    let t = &e.state.session.tools;
    assert_eq!(
        (
            t.paint_strength,
            t.brush_flow,
            t.paint_brush_kind,
            t.brush_style.angle_deg,
            t.brush_style.roundness,
            t.active_tool.clone()
        ),
        previous
    );
    request(&mut e, json!({"action":"tablet_reset"}));
}

/// Pedido pelo mesmo canal C que a GUI usa (`petunia_qt_editor_request`).
fn ffi_request(input: &str) -> Value {
    use petunia_ui_cxxqt::editor::{petunia_qt_editor_free, petunia_qt_editor_request};
    let input = std::ffi::CString::new(input).unwrap();
    // O canal devolve uma string C própria, liberada pelo par `free`.
    unsafe {
        let raw = petunia_qt_editor_request(input.as_ptr());
        assert!(!raw.is_null());
        let value = serde_json::from_str(std::ffi::CStr::from_ptr(raw).to_str().unwrap()).unwrap();
        petunia_qt_editor_free(raw);
        value
    }
}

#[test]
fn headless_threads_cannot_mutate_another_threads_editor() {
    let count = |snapshot: &Value| snapshot["scene_items"].as_array().map_or(0, Vec::len);
    let before = count(&ffi_request(r#"{"action":"snapshot"}"#));
    let child = std::thread::spawn(move || {
        let before = count(&ffi_request(r#"{"action":"snapshot"}"#));
        let after = ffi_request(r#"{"action":"command","id":"edit.duplicate"}"#);
        assert_eq!(after["ok"], true);
        assert_eq!(count(&after), before + 1);
    });
    child.join().unwrap();
    assert_eq!(count(&ffi_request(r#"{"action":"snapshot"}"#)), before);
}

#[test]
fn native_keyboard_shortcuts_and_numpad_camera_routes() {
    let mut e = editor();
    // 1. Numpad 1: Front camera
    request(
        &mut e,
        json!({"action":"shortcut","text":"Numpad 1","ctrl":false,"shift":false,"alt":false}),
    );
    assert_eq!(e.state.session.camera.yaw, 0.0);
    assert_eq!(e.state.session.camera.pitch, 0.0);

    // 2. Numpad 3: Right camera
    request(
        &mut e,
        json!({"action":"shortcut","text":"Numpad 3","ctrl":false,"shift":false,"alt":false}),
    );
    assert!(e.state.session.camera.yaw > 1.5 && e.state.session.camera.yaw < 1.6);

    // 3. Numpad 7: Top camera
    request(
        &mut e,
        json!({"action":"shortcut","text":"Numpad 7","ctrl":false,"shift":false,"alt":false}),
    );
    assert!(e.state.session.camera.pitch > 1.5 && e.state.session.camera.pitch < 1.6);

    // 4. Numpad 5: Toggle ortho/persp
    let is_ortho = e.snapshot()["is_orthographic"].as_bool().unwrap();
    request(
        &mut e,
        json!({"action":"shortcut","text":"Numpad 5","ctrl":false,"shift":false,"alt":false}),
    );
    assert_eq!(
        e.snapshot()["is_orthographic"].as_bool().unwrap(),
        !is_ortho
    );
}

/// A textura aparece no canvas 2D com uma escala própria: o carimbo só bate
/// com o anel do cursor se a UI informar a largura raster em px.
#[test]
fn paint_2d_stroke_scales_the_brush_to_the_canvas_raster() {
    fn painted_texels(raster_factor: f64) -> usize {
        let mut e = editor();
        request(&mut e, json!({"action":"workspace","id":"PAINT"}));
        request(&mut e, json!({"action":"add_paint_layer"}));
        request(&mut e, json!({"action":"tool","id":"brush"}));
        request(&mut e, json!({"action":"brush_size","value":4.}));
        let side = e
            .state
            .project
            .active()
            .and_then(|asset| asset.texture.as_ref())
            .map_or(512, |texture| texture.w) as f64;
        for phase in [0, 2] {
            request(
                &mut e,
                json!({
                    "action":"paint_stroke","x":0.5,"y":0.5,"phase":phase,
                    "raster_px":side * raster_factor
                }),
            );
        }
        let canvas = e
            .state
            .project
            .active()
            .and_then(|asset| asset.paint_stack.as_ref())
            .and_then(|stack| stack.active())
            .and_then(|layer| layer.canvas())
            .expect("camada de pintura raster ativa");
        canvas
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| pixel[3] != 0)
            .count()
    }
    // No mesmo canvas, uma textura desenhada 4× menor cobre 16× mais texels.
    assert!(painted_texels(0.25) > painted_texels(1.0) * 4);
}

/// A pegada do pincel 3D alimenta o anel do cursor (achatamento e giro).
#[test]
fn brush_footprint_reports_the_surface_squash_and_tilt() {
    let mut e = editor();
    e.controller.resize_viewport(800, 600);
    request(&mut e, json!({"action":"workspace","id":"PAINT"}));
    request(&mut e, json!({"action":"tool","id":"brush"}));
    let mut hit = None;
    'scan: for x in (10..790).step_by(10) {
        for y in (10..590).step_by(10) {
            if let Some(footprint) = e.controller.brush_footprint_at(x as f32, y as f32) {
                hit = Some((x as f32, y as f32, footprint));
                break 'scan;
            }
        }
    }
    let (x, y, (squash, tilt)) = hit.expect("um ponto pintável na malha padrão");
    assert!(
        (0.2..=1.0).contains(&squash),
        "achatamento fora da faixa: {squash}"
    );
    assert!(tilt.is_finite());
    // Fora do objeto não há pegada: o cursor volta a ser um círculo.
    assert!(
        !e.request(&json!({"action":"brush_footprint","x":2.,"y":2.}))
            .unwrap()
    );
    let outside = e.snapshot();
    assert_eq!(outside["brush_squash"], 1.0);
    assert_eq!(outside["brush_tilt"], 0.0);
    // Sobre a malha a resposta publica o mesmo valor medido pelo bridge.
    assert!(
        e.request(&json!({"action":"brush_footprint","x":x,"y":y}))
            .unwrap()
    );
    let inside = e.snapshot();
    assert!((inside["brush_squash"].as_f64().unwrap() - squash as f64).abs() < 1e-4);
    assert!((inside["brush_tilt"].as_f64().unwrap() - tilt as f64).abs() < 1e-3);
}

/// O painel PAINT publica as alças do decalque para o gizmo nativo.
#[test]
fn decal_handles_and_hover_are_published_for_the_native_gizmo() {
    let mut e = editor();
    e.controller.resize_viewport(800, 600);
    request(&mut e, json!({"action":"workspace","id":"PAINT"}));
    request(&mut e, json!({"action":"add_decal_layer"}));
    request(&mut e, json!({"action":"tool","id":"decal"}));
    let snapshot = e.snapshot();
    assert_eq!(snapshot["active_layer_is_decal"], true);
    assert!(
        !snapshot["decal_preview_commands"]
            .as_str()
            .unwrap()
            .is_empty()
    );
    let handles = snapshot["decal_handles"].clone();
    assert!(handles.is_object(), "alças do decalque ausentes: {handles}");
    assert_eq!(handles["corners"].as_array().unwrap().len(), 4);
    let center = handles["center"].as_array().unwrap();
    let (x, y) = (center[0].as_f64().unwrap(), center[1].as_f64().unwrap());
    // Sobre o centro a alça é a de mover; longe de todas, nenhuma.
    assert!(
        e.request(&json!({"action":"decal_hover","x":x,"y":y}))
            .unwrap()
    );
    assert_eq!(e.snapshot()["decal_hover_handle"], "move");
    assert!(
        !e.request(&json!({"action":"decal_hover","x":2.,"y":2.}))
            .unwrap()
    );
    assert_eq!(e.snapshot()["decal_hover_handle"], serde_json::Value::Null);
}

/// O modo escolhido no painel envia o arrasto da viewport para a
/// transformação pedida, sem pintar e como uma única entrada de Undo.
#[test]
fn decal_gizmo_mode_drives_the_viewport_drag() {
    let mut e = editor();
    e.controller.resize_viewport(800, 600);
    request(&mut e, json!({"action":"workspace","id":"PAINT"}));
    request(&mut e, json!({"action":"add_decal_layer"}));
    request(&mut e, json!({"action":"tool","id":"decal"}));
    let before = e.controller.active_decal().unwrap();
    request(&mut e, json!({"action":"decal_gizmo_mode","id":"rotate"}));
    assert_eq!(e.snapshot()["decal_gizmo_mode"], "rotate");
    for (phase, x, y) in [(0, 400., 300.), (1, 470., 330.), (2, 470., 330.)] {
        request(
            &mut e,
            json!({"action":"pointer","phase":phase,"x":x,"y":y}),
        );
    }
    let rotated = e.controller.active_decal().unwrap();
    assert_ne!(rotated.rotation_rad, before.rotation_rad);
    assert_eq!(rotated.center_uv, before.center_uv);
    assert!(e.undo());
    assert_eq!(
        e.controller.active_decal().unwrap().rotation_rad,
        before.rotation_rad
    );
    // Escalonar muda a escala e manter o modo Automático devolve as alças.
    request(&mut e, json!({"action":"decal_gizmo_mode","id":"scale"}));
    for (phase, x, y) in [(0, 400., 300.), (1, 400., 380.), (2, 400., 380.)] {
        request(
            &mut e,
            json!({"action":"pointer","phase":phase,"x":x,"y":y}),
        );
    }
    let scaled = e.controller.active_decal().unwrap();
    assert!(scaled.scale_uv != before.scale_uv || scaled.anchor != before.anchor);
    request(&mut e, json!({"action":"decal_gizmo_mode","id":"auto"}));
    assert_eq!(e.snapshot()["decal_gizmo_mode"], "auto");
    assert!(
        e.request(&json!({"action":"decal_gizmo_mode","id":"spin"}))
            .is_err()
    );
}
