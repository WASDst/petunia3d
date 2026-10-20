//! Component Gallery standalone verification executable (Petunia3D UI-001 design system).
use petunia_ui_cxxqt::widgets::PetuniaWidgetsApp;

fn main() {
    let _app = PetuniaWidgetsApp::new();
    unsafe extern "C" {
        fn petunia_component_gallery_run_test() -> bool;
    }
    let ok = unsafe { petunia_component_gallery_run_test() };
    if !ok {
        eprintln!("Component gallery self-test failed!");
        std::process::exit(1);
    }
    println!("Component gallery self-test passed successfully!");
}
