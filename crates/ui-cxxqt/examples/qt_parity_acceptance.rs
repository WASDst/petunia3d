//! Final native QA harness; use isolated XDG directories and a temporary output folder.
use petunia_ui_cxxqt::widgets::PetuniaWidgetsApp;
fn main() {
    let dir = std::env::args().nth(1).expect("output directory");
    std::fs::create_dir_all(&dir).unwrap();
    let dir = std::ffi::CString::new(dir).unwrap();
    let _app = PetuniaWidgetsApp::new();
    let worker_rejected = std::thread::spawn(|| {
        petunia_ui_cxxqt::render::opengl_viewport::petunia_qt_app_get_asset_count() == 0
    })
    .join()
    .unwrap();
    assert!(worker_rejected, "worker must not access the Qt document");
    unsafe extern "C" {
        fn petunia_qt_native_acceptance(
            directory: *const std::ffi::c_char,
        ) -> *const std::ffi::c_char;
    }
    let response = unsafe { std::ffi::CStr::from_ptr(petunia_qt_native_acceptance(dir.as_ptr())) }
        .to_str()
        .unwrap();
    let mut report: serde_json::Value = serde_json::from_str(response).unwrap();
    report["worker_thread_rejected"] = serde_json::json!(worker_rejected);
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if report["pass"] != true {
        std::process::exit(1)
    }
}
