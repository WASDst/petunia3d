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
    let checks = report["checks"].as_array().expect("lista de verificações");
    assert!(
        checks.len() >= 40,
        "cobertura mínima do harness: {} verificações",
        checks.len()
    );
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
