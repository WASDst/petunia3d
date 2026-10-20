//! Contratos dos componentes base da GUI Qt (NumberField, Vector3Field, tema e
//! avaliador de expressões). Roda offscreen, sem display (AGENTS.md Camada 3/6).
use petunia_ui_cxxqt::widgets::PetuniaWidgetsApp;
use serde_json::Value;

#[test]
fn base_components_honor_their_contracts() {
    let _app = PetuniaWidgetsApp::new();
    unsafe extern "C" {
        fn petunia_selftest_components() -> *const std::ffi::c_char;
    }
    let raw = unsafe { std::ffi::CStr::from_ptr(petunia_selftest_components()) }
        .to_str()
        .expect("relatório UTF-8")
        .to_owned();
    let report: Value = serde_json::from_str(&raw).expect("relatório JSON");
    let checks = report["checks"].as_array().expect("lista de verificações");
    assert!(
        checks.len() >= 60,
        "o auto-teste deve cobrir o contrato completo ({} verificações)",
        checks.len()
    );
    let failed: Vec<String> = checks
        .iter()
        .filter(|c| c["pass"] != true)
        .map(|c| format!("{} {}", c["name"], c["detail"].as_str().unwrap_or("")))
        .collect();
    assert!(
        failed.is_empty(),
        "verificações reprovadas:\n{}",
        failed.join("\n")
    );
}
