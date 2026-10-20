//! Isolated integration test for the Component Gallery (Petunia3D UI-001 design system).
use petunia_ui_cxxqt::widgets::PetuniaWidgetsApp;

#[test]
fn test_component_gallery_contracts() {
    let _app = PetuniaWidgetsApp::new();
    unsafe extern "C" {
        fn petunia_component_gallery_run_test() -> bool;
    }
    let ok = unsafe { petunia_component_gallery_run_test() };
    assert!(ok, "PetuniaComponentGallery self-test must pass");
}
