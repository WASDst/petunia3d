//! Shell v2 (ADR-C14): invariantes de layout, foco e estado por workspace, tema,
//! densidade e largura estreita. Roda offscreen e salva capturas PNG em
//! `target/shell-captures/` para inspeção visual ("olhar o resultado").
use petunia_ui_cxxqt::widgets::PetuniaWidgetsApp;
use serde_json::Value;
use std::{ffi::CString, path::PathBuf};

fn isolated_dirs() -> PathBuf {
    let root = std::env::temp_dir().join(format!("petunia-shell-v2-{}", std::process::id()));
    for name in ["config", "data", "cache"] {
        std::fs::create_dir_all(root.join(name)).expect("diretório isolado");
    }
    // Antes de criar o QApplication e o editor: nenhuma preferência real é lida ou gravada.
    unsafe {
        std::env::set_var("XDG_CONFIG_HOME", root.join("config"));
        std::env::set_var("XDG_DATA_HOME", root.join("data"));
        std::env::set_var("XDG_CACHE_HOME", root.join("cache"));
    }
    root
}

#[test]
fn shell_v2_regions_layout_focus_and_theme() {
    let root = isolated_dirs();
    let captures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/shell-captures");
    std::fs::create_dir_all(&captures).expect("diretório de capturas");
    let _app = PetuniaWidgetsApp::new();
    unsafe extern "C" {
        fn petunia_shell_harness(directory: *const std::ffi::c_char) -> *const std::ffi::c_char;
    }
    let dir = CString::new(captures.to_string_lossy().as_ref()).expect("caminho sem NUL");
    let raw = unsafe { std::ffi::CStr::from_ptr(petunia_shell_harness(dir.as_ptr())) }
        .to_str()
        .expect("relatório UTF-8")
        .to_owned();
    let report: Value = serde_json::from_str(&raw).expect("relatório JSON");
    // Evidência ao lado das capturas: detalhes também das verificações aprovadas.
    let _ = std::fs::write(captures.join("report.json"), &raw);
    let checks = report["checks"].as_array().expect("lista de verificações");
    assert!(
        checks.len() >= 40,
        "cobertura mínima do harness: {} verificações",
        checks.len()
    );
    // As verificações da GUI completa (F7, V03, U07) precisam ter rodado de fato.
    let names: Vec<&str> = checks.iter().filter_map(|c| c["name"].as_str()).collect();
    let missing: Vec<&str> = [
        "paint:native-property-sections",
        "surface:2d-hides-3d-view",
        "surface:3d-hides-canvas",
        "surface:mode-persists-per-workspace",
        "surface:split-shows-both",
        "uv:native-property-sections",
        "uv:structure-lists-islands",
        "uv:selecting-island-row-selects-uv",
        "animate:native-property-sections",
        "timeline:play-button-toggles-playback",
        "viewport:shift-f10-opens-context-menu",
        "viewport:context-menu-has-no-raw-ids",
        "viewport:context-menu-returns-focus",
        "paint:alt-down-reorders-layer",
        "paint:alt-up-restores-order",
        "paint:h-toggles-layer-visibility",
        "paint:delete-removes-layer-not-object",
        "uv:plus-minus-zoom",
        "uv:ctrl-arrow-pans-without-editing",
        "uv:arrow-nudges-selection",
        "uv:home-frames",
        // Refinamento de uso (ADR-C15).
        "rail:only-transform-cursor-and-create",
        "header:icon-only-with-tooltips",
        "structure:ctrl-g-groups-the-selection",
        "structure:drop-on-group-joins-it",
        "structure:drop-on-empty-area-leaves-the-group",
        "structure:drop-above-reorders",
        "structure:isolate-keeps-only-the-selection",
        "structure:leaving-isolation-restores-visibility",
        "structure:ctrl-shift-g-ungroups",
        "align:popover-has-grid-and-distribute",
        "align:enabled-with-three-objects",
        "align:disabled-with-one-object",
        "pivot:marker-published",
        "pivot:edit-mode-reflected",
        "gizmo:size-and-thickness-preferences",
        "navcube:face-hits-map-to-presets",
        "navcube:corner-hit-gives-its-direction",
        "navcube:edge-hit-gives-the-two-faces",
        "navcube:coincident-corner-prefers-the-nearer-one",
        "navcube:home-and-menu-hit",
        "navcube:paints-pixels",
    ]
    .into_iter()
    .filter(|name| !names.contains(name))
    .collect();
    assert!(missing.is_empty(), "verificações ausentes: {missing:?}");
    let failed: Vec<String> = checks
        .iter()
        .filter(|c| c["pass"] != true)
        .map(|c| format!("{} {}", c["name"], c["detail"].as_str().unwrap_or("")))
        .collect();
    let _ = std::fs::remove_dir_all(root);
    assert!(
        failed.is_empty(),
        "verificações reprovadas:\n{}",
        failed.join("\n")
    );
}
