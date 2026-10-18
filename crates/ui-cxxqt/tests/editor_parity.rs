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
    assert!(e.controller.state.session.active_shape.is_some());
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

#[test]
fn headless_threads_cannot_mutate_another_threads_editor() {
    // FFI pointers reference owned fixture strings and writable buffers.
    unsafe {
        use petunia_ui_cxxqt::render::opengl_viewport::{
            petunia_qt_app_execute_command, petunia_qt_app_get_asset_count,
        };
        let before = petunia_qt_app_get_asset_count();
        let child = std::thread::spawn(|| {
            let before = petunia_qt_app_get_asset_count();
            let command = std::ffi::CString::new("edit.duplicate").unwrap();
            assert!(petunia_qt_app_execute_command(command.as_ptr()));
            assert_eq!(petunia_qt_app_get_asset_count(), before + 1);
        });
        child.join().unwrap();
        assert_eq!(petunia_qt_app_get_asset_count(), before);
    }
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

#[test]
fn draw_volume_preview_is_transient_and_make_editable_is_explicit_undoable() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    request(
        &mut e,
        json!({"action":"profile_rectangle","width":4.,"height":3.}),
    );
    let before = serde_json::to_value(&e.state.project.project).unwrap();
    let count = e.state.project.assets.len();
    request(
        &mut e,
        json!({"action":"enter_profile_volume","mode_str":"extrude"}),
    );
    request(&mut e, json!({"action":"set_profile_depth","depth":-2.}));
    assert_eq!(
        serde_json::to_value(&e.state.project.project).unwrap(),
        before
    );
    request(&mut e, json!({"action":"cancel_profile_volume"}));
    assert_eq!(
        serde_json::to_value(&e.state.project.project).unwrap(),
        before
    );
    request(
        &mut e,
        json!({"action":"enter_profile_volume","mode_str":"extrude"}),
    );
    request(&mut e, json!({"action":"commit_profile_volume"}));
    assert_eq!(e.state.project.assets.len(), count + 1);
    let generated = e.state.project.active().unwrap();
    let id = generated.id;
    assert!(generated.geometry_source.is_some());
    assert!(generated.mesh.verts.is_empty());
    assert!(!generated.source_mesh().verts.is_empty());
    request(&mut e, json!({"action":"draw_generator_gesture","phase":0}));
    request(
        &mut e,
        json!({"action":"draw_generator_distance","depth":5.}),
    );
    assert_eq!(e.state.project.active().unwrap().id, id);
    request(&mut e, json!({"action":"draw_generator_gesture","phase":2}));
    assert!(
        matches!(e.state.project.active().unwrap().geometry_source.as_ref().unwrap().kind,petunia_project::GeometryGeneratorKind::Extrude{distance,..} if distance == -2.)
    );
    request(&mut e, json!({"action":"draw_make_editable"}));
    assert!(e.state.project.active().unwrap().geometry_source.is_none());
    assert!(!e.state.project.active().unwrap().mesh.verts.is_empty());
    assert!(e.undo());
    assert!(e.state.project.active().unwrap().geometry_source.is_some());
    assert_eq!(e.state.project.active().unwrap().id, id);
    assert!(e.undo());
    assert_eq!(e.state.project.assets.len(), count);
    assert!(e.redo());
    assert_eq!(e.state.project.assets.len(), count + 1);
}

#[test]
fn draw_numeric_scrub_cancel_keeps_authoring_and_single_commit_undo() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    request(
        &mut e,
        json!({"action":"profile_rectangle","width":2.,"height":3.}),
    );
    let original = serde_json::to_value(&e.state.project.project).unwrap();
    request(
        &mut e,
        json!({"action":"profile_dimension_gesture","phase":0}),
    );
    request(
        &mut e,
        json!({"action":"profile_dimension","field":"width","value":8.}),
    );
    request(
        &mut e,
        json!({"action":"profile_dimension","field":"height","value":7.}),
    );
    assert_eq!(
        serde_json::to_value(&e.state.project.project).unwrap(),
        original
    );
    request(
        &mut e,
        json!({"action":"profile_dimension_gesture","phase":2}),
    );
    assert_eq!(
        serde_json::to_value(&e.state.project.project).unwrap(),
        original
    );
    request(
        &mut e,
        json!({"action":"profile_dimension_gesture","phase":0}),
    );
    request(
        &mut e,
        json!({"action":"profile_dimension","field":"width","value":8.}),
    );
    request(
        &mut e,
        json!({"action":"profile_dimension_gesture","phase":1}),
    );
    assert_eq!(e.snapshot()["profile_width"], 8.);
    assert!(e.undo());
    assert_eq!(e.snapshot()["profile_width"], 2.);
}

#[test]
fn draw_plane_change_does_not_move_camera_and_open_curve_is_sweep_path() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    let camera = e.state.session.camera.clone();
    request(
        &mut e,
        json!({"action":"draw_plane_component","field":"origin_x","value":12.}),
    );
    request(
        &mut e,
        json!({"action":"draw_plane_component","field":"rotate_z","value":35.}),
    );
    assert_eq!(e.state.session.camera, camera);
    assert_eq!(e.state.session.active_workplane.plane.origin[0], 12.);
    request(&mut e, json!({"action":"workplane_look"}));
    assert!(e.state.session.camera.planar_basis.is_some());
    request(&mut e, json!({"action":"draw_angle_step","value":30.}));
    assert_eq!(e.snapshot()["draw"]["angle_step"], 30.);
    let spline = petunia_project::SplineResource::from_polyline(
        "route",
        &[[0., 0., 0.], [0., 4., 0.]],
        false,
    );
    let path = spline.id;
    let shape = petunia_project::PlanarShape::new(
        "route plane",
        path,
        petunia_project::Workplane::default(),
    );
    e.execute_command(&petunia_core::CreateProfileCmd {
        profile: shape,
        spline,
    })
    .unwrap();
    request(
        &mut e,
        json!({"action":"profile_rectangle","width":2.,"height":2.}),
    );
    assert!(
        e.snapshot()["draw"]["paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == path.to_string())
    );
    request(
        &mut e,
        json!({"action":"draw_sweep_path","id":path.to_string()}),
    );
    request(
        &mut e,
        json!({"action":"enter_profile_volume","mode_str":"sweep"}),
    );
    request(&mut e, json!({"action":"commit_profile_volume"}));
    assert!(
        matches!(e.state.project.active().unwrap().geometry_source.as_ref().unwrap().kind,petunia_project::GeometryGeneratorKind::Sweep{path_object,..} if path_object == path)
    );
}

#[test]
fn draw_point_gesture_never_writes_authoring_until_release_and_cancels_cleanly() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    request(&mut e, json!({"action":"tool","id":"draw_profile"}));
    e.controller.viewport_size = [800., 600.];
    request(&mut e, json!({"action":"workplane_look"}));
    let original = serde_json::to_value(&e.state.project.project).unwrap();
    e.controller.tool_pointer(0, 400., 300., false, false);
    assert_eq!(
        serde_json::to_value(&e.state.project.project).unwrap(),
        original
    );
    assert!(e.controller.tool_pointer(2, 400., 300., false, false));
    assert_eq!(e.state.project.project.profiles.len(), 1);
    assert_eq!(e.state.project.project.splines.len(), 1);
    assert!(e.undo());
    assert!(e.state.project.project.profiles.is_empty());
}

#[test]
fn native_split_banks_keep_workplane_navigation_and_snap_independent() {
    let mut host = petunia_ui_cxxqt::render::opengl_viewport::QtViewportRenderer::new();
    host.app
        .state
        .session
        .active_workplane
        .set(
            petunia_core::WorkplaneMode::Custom,
            petunia_project::Workplane::default(),
        )
        .unwrap();
    host.activate(2);
    let plane = petunia_project::Workplane {
        origin: [7., 8., 9.],
        ..Default::default()
    };
    host.app
        .state
        .session
        .active_workplane
        .set(petunia_core::WorkplaneMode::Custom, plane)
        .unwrap();
    host.app.state.session.snap_settings.angle_step_degrees = 30.;
    host.activate(1);
    assert_eq!(
        host.app.state.session.active_workplane.plane.origin,
        [0.; 3]
    );
    assert_eq!(host.app.state.session.snap_settings.angle_step_degrees, 15.);
    host.activate(2);
    assert_eq!(
        host.app.state.session.active_workplane.plane.origin,
        [7., 8., 9.]
    );
    assert_eq!(host.app.state.session.snap_settings.angle_step_degrees, 30.);
}

#[test]
fn draw_relative_shape_transform_changes_all_contours_in_one_undo() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    request(
        &mut e,
        json!({"action":"profile_rectangle","width":4.,"height":3.}),
    );
    request(
        &mut e,
        json!({"action":"draw_offset_inner","distance":0.25}),
    );
    let shape = e.state.session.active_shape.unwrap();
    let original = e.state.project.project.clone();
    request(
        &mut e,
        json!({"action":"draw_shape_transform","field":"scale_x","value":2.}),
    );
    let outer = original.get_profile(shape).unwrap().spline_id;
    let inner = original.get_profile(shape).unwrap().hole_splines[0];
    for id in [outer, inner] {
        for (before, after) in original
            .get_spline(id)
            .unwrap()
            .points
            .iter()
            .zip(&e.state.project.project.get_spline(id).unwrap().points)
        {
            assert_eq!(after.position[0], before.position[0] * 2.);
            assert_eq!(after.position[1], before.position[1]);
        }
    }
    assert!(e.undo());
    assert_eq!(
        e.state.project.project.get_spline(inner).unwrap().points,
        original.get_spline(inner).unwrap().points
    );
}

#[test]
fn draw_delete_undo_redo_restores_shape_selection_with_the_document() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    request(
        &mut e,
        json!({"action":"profile_rectangle","width":2.,"height":2.}),
    );
    let id = e.state.session.active_shape.unwrap();
    assert!(e.controller.delete_or_dissolve_selection());
    assert!(e.state.project.project.get_profile(id).is_none());
    assert!(e.undo());
    assert_eq!(e.state.session.active_shape, Some(id));
    assert!(e.state.project.project.get_profile(id).is_some());
    assert!(e.redo());
    assert_eq!(e.state.session.active_shape, None);
    assert!(e.state.project.project.get_profile(id).is_none());
}

#[test]
fn draw_reset_and_explicit_view_changes_preserve_construction_plane() {
    let mut host = petunia_ui_cxxqt::render::opengl_viewport::QtViewportRenderer::new();
    request(&mut host.app, json!({"action":"workspace","id":"DRAW"}));
    request(
        &mut host.app,
        json!({"action":"draw_plane_component","field":"origin_x","value":7.}),
    );
    request(&mut host.app, json!({"action":"workplane_look"}));
    let plane = host.app.state.session.active_workplane;
    host.reset_camera();
    assert!(host.app.state.session.camera.planar_basis.is_some());
    assert_eq!(
        host.app.state.session.camera.proj,
        petunia_core::Projection::Ortho
    );
    host.app
        .controller
        .set_view_preset(petunia_core::ViewPreset::Front);
    assert!(!host.app.state.session.draw_planar_navigation);
    assert_eq!(host.app.state.session.active_workplane, plane);
}

#[test]
fn draw_poly_draw_preserves_shape_selection_sources_and_undo() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    request(
        &mut e,
        json!({"action":"profile_rectangle","width":2.,"height":3.}),
    );
    let id = e.state.session.active_shape;
    let original = serde_json::to_value(&e.state.project.project).unwrap();
    let history = e.state.project.undo.depth();
    request(&mut e, json!({"action":"workspace","id":"POLY"}));
    assert_eq!(e.state.session.active_shape, id);
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    assert_eq!(e.state.session.active_shape, id);
    assert_eq!(
        serde_json::to_value(&e.state.project.project).unwrap(),
        original
    );
    assert_eq!(e.state.project.undo.depth(), history);
}

#[test]
fn draw_point_inspector_edits_handles_and_delete_undo_restores_context() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    request(
        &mut e,
        json!({"action":"profile_rectangle","width":2.,"height":3.}),
    );
    let shape = e.state.session.active_shape.unwrap();
    let spline = e
        .state
        .project
        .project
        .get_profile(shape)
        .unwrap()
        .spline_id;
    let point = e.state.project.project.get_spline(spline).unwrap().points[0].id;
    request(
        &mut e,
        json!({"action":"draw_point","id":point.to_string()}),
    );
    request(
        &mut e,
        json!({"action":"draw_point_parameter","field":"out_x","value":0.5}),
    );
    request(&mut e, json!({"action":"draw_point_mode","id":"mirrored"}));
    let node = e
        .state
        .project
        .project
        .get_spline(spline)
        .unwrap()
        .point(point)
        .unwrap();
    assert_eq!(node.handle_in, [-0.5, 0., 0.]);
    assert_eq!(node.handle_out, [0.5, 0., 0.]);
    assert!(e.controller.delete_or_dissolve_selection());
    assert!(e.state.project.project.get_profile(shape).is_some());
    assert_eq!(
        e.state
            .project
            .project
            .get_spline(spline)
            .unwrap()
            .points
            .len(),
        3
    );
    assert!(e.undo());
    assert_eq!(e.state.session.active_shape, Some(shape));
    assert_eq!(e.state.session.planar_selection.point, Some(point));
    assert_eq!(
        e.state
            .project
            .project
            .get_spline(spline)
            .unwrap()
            .points
            .len(),
        4
    );
}

#[test]
fn draw_volume_and_parameter_previews_reject_changed_geometry_revisions() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    request(
        &mut e,
        json!({"action":"profile_rectangle","width":4.0,"height":3.0}),
    );
    request(
        &mut e,
        json!({"action":"enter_profile_volume","mode_str":"extrude"}),
    );
    let count = e.state.project.assets.len();
    e.state.project.project.bump_positions();
    assert!(!e.controller.commit_profile_volume());
    assert_eq!(e.state.project.assets.len(), count);
    assert!(e.controller.cancel_profile_volume());
    assert!(e.controller.enter_profile_volume("extrude"));
    assert!(e.controller.commit_profile_volume());
    let source = e.state.project.active().unwrap().geometry_source.clone();
    assert!(e.controller.generator_parameter_gesture(0));
    assert!(e.controller.edit_selected_generator_distance(7.));
    e.state.project.project.bump_transforms();
    assert!(!e.controller.generator_parameter_gesture(1));
    assert_eq!(e.state.project.active().unwrap().geometry_source, source);
}

#[test]
fn draw_solid_pointer_selection_cancel_and_remove_share_document_undo() {
    let mut e = editor();
    request(&mut e, json!({"action":"workspace","id":"DRAW"}));
    let first = petunia_project::Asset::new("A", petunia_mesh::Mesh::cube(2.));
    let mut second = petunia_project::Asset::new("B", petunia_mesh::Mesh::cube(2.));
    second.transform.translation = [5., 0., 0.];
    let ids = vec![first.id, second.id];
    e.state.project.project.assets = vec![first, second];
    e.state.project.project.active = 0;
    e.state.session.selection.assets = ids.clone();
    e.state.session.selection.asset = Some(ids[0]);
    e.controller.viewport_size = [800., 600.];
    e.controller
        .set_view_preset(petunia_core::ViewPreset::Front);
    let before = serde_json::to_value(&e.state.project.project).unwrap();
    assert!(e.request(&json!({"action":"draw_solid_begin"})).unwrap());
    e.controller.tool_pointer(0, 400., 300., false, false);
    e.controller.tool_pointer(2, 400., 300., false, false);
    assert_eq!(e.snapshot()["draw"]["solid_can_commit"], true);
    assert_eq!(
        serde_json::to_value(&e.state.project.project).unwrap(),
        before
    );
    assert!(e.request(&json!({"action":"draw_solid_cancel"})).unwrap());
    assert_eq!(
        serde_json::to_value(&e.state.project.project).unwrap(),
        before
    );
    assert!(e.request(&json!({"action":"draw_solid_begin"})).unwrap());
    request(&mut e, json!({"action":"draw_solid_intent","id":"remove"}));
    let region = e.snapshot()["draw"]["solid_regions"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        e.request(&json!({"action":"draw_solid_toggle","id":region}))
            .unwrap()
    );
    assert!(e.request(&json!({"action":"draw_solid_commit"})).unwrap());
    assert_eq!(e.state.project.assets.len(), 1);
    assert_eq!(e.state.project.assets[0].id, ids[1]);
    assert!(e.undo());
    assert_eq!(e.state.project.assets.len(), 2);
    assert!(e.redo());
    assert_eq!(e.state.project.assets.len(), 1);
}
