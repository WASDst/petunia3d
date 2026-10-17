//! O documento pertence à thread de UI (processo próprio: o "dono" é global).
use petunia_ui_cxxqt::render::opengl_viewport::{
    petunia_qt_camera_get_view_name, petunia_qt_claim_ui_thread,
};

#[test]
fn worker_thread_cannot_touch_the_document() {
    assert!(petunia_qt_claim_ui_thread());
    let mut buf = [0i8; 64];
    assert!(unsafe { petunia_qt_camera_get_view_name(buf.as_mut_ptr(), 64) });
    let rejected = std::thread::spawn(|| {
        let mut buf = [0i8; 64];
        !unsafe { petunia_qt_camera_get_view_name(buf.as_mut_ptr(), 64) }
    })
    .join()
    .unwrap();
    assert!(rejected, "outra thread não deve alcançar o editor");
}
