use petunia_ui_cxxqt::widgets::{PetuniaStudioWindow, PetuniaWidgetsApp};
fn main() {
    let app = PetuniaWidgetsApp::new();
    let window = PetuniaStudioWindow::new();
    window.show();
    app.exec();
}
