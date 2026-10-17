//! Ponte Rust↔Qt do Shell v2 (U19): a GUI fala com o editor por pedidos JSON
//! (`petunia_qt_editor_request` → `QtEditor::request`) e pelo FFI de viewport
//! (câmera, gizmo, seleção por raio, tablet, sombreamento). Os getters/setters
//! `petunia_qt_app_*` do shell legado foram removidos; cada comportamento que eles
//! protegiam é exercitado aqui pelo mesmo pedido que a GUI envia.
use petunia_ui_cxxqt::bridge::shell::PetuniaShellBridgeRust;
use petunia_ui_cxxqt::render::opengl_viewport::{
    QtViewportRenderer, VIEWPORT_RENDERER, petunia_qt_app_frame_selection,
    petunia_qt_app_get_shading, petunia_qt_app_search_commands, petunia_qt_app_set_shading,
    petunia_qt_camera_get_orientation, petunia_qt_camera_get_view_name, petunia_qt_camera_orbit,
    petunia_qt_camera_pan, petunia_qt_camera_reset, petunia_qt_camera_set_preset,
    petunia_qt_camera_toggle_projection, petunia_qt_camera_zoom, petunia_qt_gizmo_drag_axis,
    petunia_qt_gizmo_get_projected, petunia_qt_gizmo_hit_test, petunia_qt_tablet_paint_stroke,
    petunia_qt_viewport_hover, petunia_qt_viewport_select,
};
use serde_json::{Value, json};

fn reset_editor_fixture() {
    VIEWPORT_RENDERER
        .with(|renderer| {
            *renderer.borrow_mut() = QtViewportRenderer::new();
        })
        .unwrap();
}

fn with_host<R>(f: impl FnOnce(&mut QtViewportRenderer) -> R) -> R {
    VIEWPORT_RENDERER
        .with(|renderer| f(&mut renderer.borrow_mut()))
        .expect("renderer da thread de UI")
}

/// Envia um pedido como a GUI e falha o teste se o editor recusar.
fn request(action: Value) -> bool {
    with_host(|host| {
        host.app
            .request(&action)
            .unwrap_or_else(|error| panic!("{action}: {error}"))
    })
}

fn try_request(action: Value) -> Result<bool, String> {
    with_host(|host| host.app.request(&action))
}

fn snapshot() -> Value {
    with_host(|host| host.app.snapshot())
}

fn asset_count() -> usize {
    with_host(|host| host.app.state.project.assets.len())
}

fn asset_id(index: usize) -> String {
    with_host(|host| host.app.state.project.assets[index].id.to_string())
}

fn select(index: usize) {
    request(json!({"action": "select", "id": asset_id(index)}));
}

fn translation(index: usize) -> [f32; 3] {
    with_host(|host| host.app.state.project.assets[index].transform.translation)
}

fn selected_assets() -> usize {
    with_host(|host| host.app.state.session.selection.assets.len())
}

#[test]
fn test_shell_bridge_properties_default() {
    let bridge = PetuniaShellBridgeRust::default();
    assert_eq!(bridge.selected_count, 0);
    assert!(!bridge.can_undo);
    assert!(!bridge.can_redo);
    assert!(!bridge.is_dirty);
}

#[test]
fn test_qt_viewport_renderer_initial_state_and_navigation() {
    let mut renderer = QtViewportRenderer::new();
    assert!(!renderer.is_initialized());
    assert!(!renderer.app.state.project.assets.is_empty());

    let initial_yaw = renderer.app.state.session.camera.yaw;
    let initial_dist = renderer.app.state.session.camera.distance;
    let initial_target = renderer.app.state.session.camera.target;

    renderer.orbit_camera(10.0, 5.0);
    assert_ne!(renderer.app.state.session.camera.yaw, initial_yaw);

    renderer.zoom_camera(2.0);
    assert_ne!(renderer.app.state.session.camera.distance, initial_dist);

    renderer.pan_camera(5.0, 5.0);
    assert_ne!(renderer.app.state.session.camera.target, initial_target);

    renderer.reset_camera();
    assert_eq!(renderer.app.state.session.camera.target, glam::Vec3::ZERO);
}

#[test]
fn test_document_history_outliner_and_transforms_via_requests() {
    reset_editor_fixture();
    let initial_count = asset_count();
    assert!(initial_count >= 1);
    assert_eq!(snapshot()["scene_items"][0]["name"], "Cube");

    // Comando registrado + histórico.
    request(json!({"action": "command", "id": "model.add_cube"}));
    assert_eq!(asset_count(), initial_count + 1);
    assert_eq!(snapshot()["saved"], false, "documento sujo após editar");
    request(json!({"action": "undo"}));
    assert_eq!(asset_count(), initial_count);
    request(json!({"action": "redo"}));
    assert_eq!(asset_count(), initial_count + 1);

    // Visibilidade pelo outliner.
    let id = asset_id(0);
    request(json!({"action": "visibility", "id": id}));
    assert!(!with_host(|host| host.app.state.project.assets[0].visible));
    request(json!({"action": "visibility", "id": id}));
    assert!(with_host(|host| host.app.state.project.assets[0].visible));

    // Transform digitado (seção Transform): posição, rotação em graus, escala.
    select(0);
    for (axis, text) in ["1.5", "2.5", "3.5"].iter().enumerate() {
        request(json!({"action": "transform", "kind": "position", "axis": axis, "text": text}));
    }
    assert_eq!(translation(0), [1.5, 2.5, 3.5]);
    for (axis, text) in ["2", "3", "0.5"].iter().enumerate() {
        request(json!({"action": "transform", "kind": "scale", "axis": axis, "text": text}));
    }
    assert_eq!(
        with_host(|host| host.app.state.project.assets[0].transform.scale),
        [2.0, 3.0, 0.5]
    );
    // Cada edição de propriedade é um passo de Undo.
    request(json!({"action": "undo"}));
    assert_eq!(
        with_host(|host| host.app.state.project.assets[0].transform.scale),
        [2.0, 3.0, 1.0]
    );
    request(json!({"action": "redo"}));
    // Rotação depois da escala (escala digitada em objeto já girado: ver o handoff de U19).
    request(json!({"action": "transform", "kind": "rotation", "axis": 2, "text": "90"}));
    let rotation = with_host(|host| host.app.state.project.assets[0].transform.rotation);
    let (_, _, z) = glam::Quat::from_array(rotation).to_euler(glam::EulerRot::XYZ);
    assert!(
        (z.to_degrees() - 90.0).abs() < 1e-3,
        "rotação Z = {}",
        z.to_degrees()
    );

    // Expressão inválida é recusada sem alterar o documento.
    assert!(
        try_request(json!({"action": "transform", "kind": "position", "axis": 0, "text": "1/0"}))
            .is_err()
            || translation(0)[0] == 1.5
    );
    assert!(
        try_request(json!({"action": "transform", "kind": "position", "axis": 7, "text": "1"}))
            .is_err()
    );

    // Cor base do material ativo.
    request(
        json!({"action": "set_active_material_base_color", "red": 0.9, "green": 0.4, "blue": 0.1}),
    );
    let color = with_host(|host| {
        let state = &host.app.state;
        let asset = state.project.active().expect("ativo");
        let material = asset
            .material_id
            .and_then(|id| state.project.materials.iter().find(|m| m.id == id))
            .or_else(|| state.project.materials.first())
            .expect("material do objeto");
        material.base_color
    });
    assert!(
        (color[0] - 0.9).abs() < 1e-4
            && (color[1] - 0.4).abs() < 1e-4
            && (color[2] - 0.1).abs() < 1e-4
    );

    // Domínios de seleção (Viewport Header) e enquadramento.
    for domain in ["vertex", "edge", "face", "object"] {
        request(json!({"action": "domain", "id": domain}));
        assert_eq!(
            with_host(|host| format!("{:?}", host.app.state.selection_domain())).to_lowercase(),
            domain
        );
    }
    petunia_qt_app_frame_selection();

    // Busca da paleta de comandos.
    let mut search_buf = [0i8; 2048];
    let query = std::ffi::CString::new("cube").unwrap();
    let matches =
        unsafe { petunia_qt_app_search_commands(query.as_ptr(), search_buf.as_mut_ptr(), 2048) };
    assert!(matches > 0);
    let results = unsafe { std::ffi::CStr::from_ptr(search_buf.as_ptr()) }
        .to_str()
        .unwrap();
    assert!(results.contains("model.add_cube"));

    // Navegação de câmera, presets e projeção.
    petunia_qt_camera_orbit(12.0, -8.0);
    petunia_qt_camera_pan(4.0, 2.0);
    petunia_qt_camera_zoom(1.5);
    petunia_qt_camera_reset();
    petunia_qt_camera_set_preset(1);
    petunia_qt_camera_set_preset(3);
    petunia_qt_camera_set_preset(5);
    petunia_qt_camera_toggle_projection();
    petunia_qt_camera_toggle_projection();
    let mut view_buf = [0i8; 64];
    assert!(unsafe { petunia_qt_camera_get_view_name(view_buf.as_mut_ptr(), 64) });
    assert!(
        !unsafe { std::ffi::CStr::from_ptr(view_buf.as_ptr()) }
            .to_str()
            .unwrap()
            .is_empty()
    );
    let mut o = [0.0f32; 9];
    let [rx, ry, rz, ux, uy, uz, fx, fy, fz] = &mut o;
    assert!(unsafe { petunia_qt_camera_get_orientation(rx, ry, rz, ux, uy, uz, fx, fy, fz) });
    assert!(o.iter().all(|v| v.is_finite()));

    // Modos de sombreamento da vista.
    assert_eq!(petunia_qt_app_get_shading(), 1);
    petunia_qt_app_set_shading(0);
    assert_eq!(petunia_qt_app_get_shading(), 0);
    petunia_qt_app_set_shading(2);
    assert_eq!(petunia_qt_app_get_shading(), 2);
    petunia_qt_app_set_shading(1);

    // Salvar, abrir e exportar pelos pedidos do AppBar.
    let temp_dir = std::env::temp_dir().join(format!("petunia_cxxqt_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let project_file = temp_dir.join("test_studio.petunia");
    let path = project_file.to_str().unwrap();
    request(json!({"action": "project.save", "path": path}));
    assert!(project_file.exists());
    assert_eq!(snapshot()["saved"], true);
    let count = asset_count();
    request(json!({"action": "command", "id": "model.add_cube"}));
    request(json!({"action": "project.open", "path": path}));
    assert_eq!(asset_count(), count, "abrir restaura o arquivo salvo");
    assert_eq!(selected_assets(), 0, "abrir limpa a seleção");

    let glb = temp_dir.join("test_model.glb");
    request(json!({"action": "export_glb", "path": glb.to_str().unwrap()}));
    assert_eq!(&std::fs::read(&glb).unwrap()[..4], b"glTF");
    select(0);
    let obj = temp_dir.join("test_model.obj");
    request(json!({"action": "export_obj", "path": obj.to_str().unwrap()}));
    let obj = std::fs::read_to_string(&obj).unwrap();
    assert!(obj.lines().any(|line| line.starts_with("v ")));
    assert!(obj.lines().any(|line| line.starts_with("f ")));
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Caneta com pressão e inclinação.
    petunia_qt_tablet_paint_stroke(100.0, 150.0, 0.75, 12.0, -5.0, false);
    petunia_qt_tablet_paint_stroke(105.0, 155.0, 0.90, 10.0, -4.0, true);
}

#[test]
fn test_widgets_studio_window_creation() {
    reset_editor_fixture();
    if std::env::var("DISPLAY").is_ok()
        || std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("QT_QPA_PLATFORM").as_deref() == Ok("offscreen")
    {
        let _app = petunia_ui_cxxqt::widgets::PetuniaWidgetsApp::new();
        let window = petunia_ui_cxxqt::widgets::PetuniaStudioWindow::new();
        window.show();
    }
}

#[test]
fn test_viewport_raycast_selection_and_domains() {
    reset_editor_fixture();
    request(json!({"action": "domain", "id": "object"}));
    select(0);
    assert_eq!(with_host(|host| host.app.state.project.active), 0);

    // Clique no centro (onde está o cubo) seleciona; no canto, deseleciona.
    assert!(petunia_qt_viewport_select(
        400.0, 300.0, 800.0, 600.0, false
    ));
    assert_eq!(selected_assets(), 1);
    assert!(!petunia_qt_viewport_select(10.0, 10.0, 800.0, 600.0, false));
    assert_eq!(selected_assets(), 0);
    assert!(petunia_qt_viewport_select(
        400.0, 300.0, 800.0, 600.0, false
    ));
    assert_eq!(selected_assets(), 1);

    let _ = petunia_qt_viewport_hover(400.0, 300.0, 800.0, 600.0);
    let _ = petunia_qt_viewport_hover(10.0, 10.0, 800.0, 600.0);

    for domain in ["vertex", "edge", "face"] {
        request(json!({"action": "domain", "id": domain}));
        let _ = petunia_qt_viewport_select(400.0, 300.0, 800.0, 600.0, false);
        assert_eq!(
            with_host(|host| format!("{:?}", host.app.state.selection_domain())).to_lowercase(),
            domain
        );
    }
    request(json!({"action": "domain", "id": "object"}));
}

#[test]
fn test_legacy_modifier_stack_requests() {
    // A pilha legada ainda aparece para arquivos antigos (cartões "Modificadores
    // (legado)" no Shell v2); arquivos carregados migram para o Operation Graph.
    reset_editor_fixture();
    select(0);
    let modifiers = || with_host(|host| host.app.state.project.assets[0].modifiers.clone());
    let initial = modifiers().len();
    request(json!({"action": "add_modifier", "kind": "mirror"}));
    assert_eq!(modifiers().len(), initial + 1);
    let mirror = modifiers()[initial].id.to_string();
    assert!(modifiers()[initial].enabled);

    request(json!({"action": "set_modifier_enabled", "id": mirror, "enabled": false}));
    assert!(!modifiers()[initial].enabled);

    request(json!({"action": "add_modifier", "kind": "symmetry"}));
    assert_eq!(modifiers().len(), initial + 2);
    let symmetry = modifiers()[initial + 1].id.to_string();
    request(json!({"action": "move_modifier", "id": symmetry, "direction": -1}));
    assert_eq!(modifiers()[initial].id.to_string(), symmetry);
    request(json!({"action": "move_modifier", "id": symmetry, "direction": 1}));
    assert_eq!(modifiers()[initial].id.to_string(), mirror);

    request(json!({"action": "remove_modifier", "id": symmetry}));
    assert_eq!(modifiers().len(), initial + 1);

    request(json!({"action": "set_modifier_enabled", "id": mirror, "enabled": true}));
    request(json!({"action": "apply_modifier", "id": mirror}));
    assert_eq!(modifiers().len(), initial, "aplicar consome o modifier");
}

#[test]
fn test_material_scalars_and_surface_shading_via_requests() {
    reset_editor_fixture();
    select(0);
    for (field, value) in [
        ("roughness", 0.35),
        ("metallic", 0.85),
        ("emission_strength", 2.5),
    ] {
        request(json!({"action": "set_active_material_scalar", "field": field, "value": value}));
    }
    let (roughness, metallic, emission) = with_host(|host| {
        let state = &host.app.state;
        let asset = state.project.active().expect("ativo");
        let m = asset
            .material_id
            .and_then(|id| state.project.materials.iter().find(|m| m.id == id))
            .or_else(|| state.project.materials.first())
            .expect("material do objeto");
        (m.roughness, m.metallic, m.emission_strength)
    });
    assert!((roughness - 0.35).abs() < 1e-4);
    assert!((metallic - 0.85).abs() < 1e-4);
    assert!((emission - 2.5).abs() < 1e-4);

    // Seção Geometria: Facetado / Suave.
    let shading = || with_host(|host| host.app.state.project.assets[0].surface_shading);
    assert_eq!(shading(), petunia_mesh::shading::SurfaceShading::Flat);
    request(json!({"action": "command", "id": "model.shade_smooth"}));
    assert_eq!(shading(), petunia_mesh::shading::SurfaceShading::Smooth);
    request(json!({"action": "command", "id": "model.shade_flat"}));
    assert_eq!(shading(), petunia_mesh::shading::SurfaceShading::Flat);
}

#[test]
fn test_selection_quick_requests() {
    reset_editor_fixture();
    select(0);
    request(json!({"action": "select_all"}));
    assert!(selected_assets() >= 1);
    request(json!({"action": "clear_selection"}));
    assert_eq!(selected_assets(), 0);
    select(0);
    request(json!({"action": "origin_geometry"}));
}

#[test]
fn test_3d_transform_gizmo_ffi() {
    reset_editor_fixture();
    select(0);
    let mut orig = [0.0f32; 2];
    let mut px = [0.0f32; 2];
    let mut py = [0.0f32; 2];
    let mut pz = [0.0f32; 2];
    let projected = unsafe {
        petunia_qt_gizmo_get_projected(
            800.0,
            600.0,
            orig.as_mut_ptr(),
            px.as_mut_ptr(),
            py.as_mut_ptr(),
            pz.as_mut_ptr(),
        )
    };
    assert!(projected);
    assert!(
        orig.iter()
            .chain(&px)
            .chain(&py)
            .chain(&pz)
            .all(|v| v.is_finite())
    );
    assert_eq!(petunia_qt_gizmo_hit_test(orig[0], orig[1], 800.0, 600.0), 4);
    assert_eq!(petunia_qt_gizmo_hit_test(10.0, 10.0, 800.0, 600.0), 0);

    let before = translation(0);
    assert!(petunia_qt_gizmo_drag_axis(1, 10.0, 0.0, 800.0, 600.0));
    assert_ne!(before[0], translation(0)[0]);
}

#[test]
fn test_uv_diagnostics_of_default_cube() {
    reset_editor_fixture();
    let (faces, uv_corners, diagnostics) = with_host(|host| {
        let mesh = &host.app.state.project.assets[0].mesh;
        (
            mesh.faces.len(),
            mesh.faces.first().map_or(0, |face| face.uv.len()),
            mesh.uv_diagnostics(),
        )
    });
    assert!(faces > 0);
    assert!(uv_corners >= 3);
    assert!(diagnostics.island_count >= 1);
    assert!(diagnostics.overlap_ratio >= 0.0);
    assert!(diagnostics.mean_stretch >= 0.0);
}

#[test]
fn test_timeline_and_keyframes_via_animate_requests() {
    use petunia_editor::animate::AnimateIntent;
    reset_editor_fixture();
    select(0);
    request(json!({"action": "workspace", "id": "ANIMATE"}));
    let preset = snapshot()["animate"]["creature_presets"][0]["id"].clone();
    request(json!({"action": "animate", "id": "add-creature", "arg": preset}));
    let bone = snapshot()["animate"]["bones"][0]["id"].clone();
    request(json!({"action": "animate", "id": "select-bone", "arg": bone}));

    let vm = || with_host(|host| host.app.controller.animate_view_model());
    assert_eq!(vm().current_frame, 0);
    assert!(vm().total_frames >= 42);

    // Ir ao quadro 42 (a timeline usa estes passos do controlador).
    let goto = |frame: i32| {
        with_host(|host| {
            host.app
                .controller
                .apply_animate(AnimateIntent::JumpToStart);
            host.app
                .controller
                .apply_animate(AnimateIntent::StepFrame(frame));
        })
    };
    goto(42);
    assert_eq!(vm().current_frame, 42);

    let has_key = |frame: i32| {
        let vm = vm();
        vm.tracks
            .iter()
            .filter(|t| vm.selected_bone == Some(t.bone_id))
            .any(|t| t.keyframes.iter().any(|k| k.frame == frame))
    };
    assert!(vm().can_keyframe);
    assert!(!has_key(42));
    request(json!({"action": "animate", "id": "insert-keyframe"}));
    assert!(has_key(42));
    // Keyframes autorais tocam mesmo sem Motion procedural.
    request(json!({"action": "animate", "id": "toggle-play"}));
    assert!(vm().playing);
    request(json!({"action": "animate", "id": "toggle-play"}));
    assert!(!vm().playing);
    request(json!({"action": "animate", "id": "delete-keyframe", "value": 42.0 / 24.0}));
    assert!(!has_key(42));

    // Clipe vazio não toca; com um Motion procedural real, toca.
    request(json!({"action": "animate", "id": "toggle-play"}));
    assert!(!vm().playing);
    let motion = snapshot()["animate"]["catalog"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["available"] == true)
        .unwrap()["id"]
        .clone();
    request(json!({"action": "animate", "id": "add-motion", "arg": motion}));
    request(json!({"action": "animate", "id": "seek", "value": 0.0}));
    request(json!({"action": "animate", "id": "toggle-play"}));
    assert!(vm().playing);
    request(json!({"action": "animate", "id": "toggle-play"}));
    assert!(!vm().playing);
}

#[test]
fn test_align_and_distribute_objects_via_requests() {
    reset_editor_fixture();
    request(json!({"action": "command", "id": "model.add_cube"}));
    request(json!({"action": "command", "id": "model.add_cube"}));
    select(1);
    request(json!({"action": "transform", "kind": "position", "axis": 0, "text": "20"}));
    with_host(|host| {
        host.app.state.select_object(Some(0), false);
        host.app.state.select_object(Some(1), true);
    });
    request(json!({"action": "command", "id": "model.align_center_x"}));
    let (x0, x1) = (translation(0)[0], translation(1)[0]);
    assert!((x0 - x1).abs() < 1e-4, "x0={x0}, x1={x1}");
    request(json!({"action": "undo"}));
    assert!(
        (translation(1)[0] - 20.0).abs() < 1e-4,
        "Undo restaura X = 20"
    );
}
