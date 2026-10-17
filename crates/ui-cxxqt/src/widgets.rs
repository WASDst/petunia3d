//! Safe Rust wrapper para o QApplication e a janela principal (Shell v2, `PetuniaShellWindow`).

pub struct PetuniaWidgetsApp {
    raw: *mut std::ffi::c_void,
}

impl PetuniaWidgetsApp {
    pub fn new() -> Self {
        if std::env::var("QT_QPA_PLATFORM").is_err() && std::env::var("WAYLAND_DISPLAY").is_ok() {
            unsafe {
                std::env::set_var("QT_QPA_PLATFORM", "xcb;wayland");
            }
        }
        unsafe extern "C" {
            fn petunia_qt_app_create() -> *mut std::ffi::c_void;
        }
        let raw = unsafe { petunia_qt_app_create() };
        assert!(!raw.is_null(), "QApplication must stay on its owner thread");
        Self { raw }
    }

    pub fn exec(&self) -> i32 {
        unsafe extern "C" {
            fn petunia_qt_app_exec(app: *mut std::ffi::c_void) -> i32;
        }
        unsafe { petunia_qt_app_exec(self.raw) }
    }
}

impl Default for PetuniaWidgetsApp {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PetuniaWidgetsApp {
    fn drop(&mut self) {
        unsafe extern "C" {
            fn petunia_qt_app_destroy(app: *mut std::ffi::c_void);
        }
        if !self.raw.is_null() {
            unsafe { petunia_qt_app_destroy(self.raw) };
        }
    }
}

pub struct PetuniaStudioWindow {
    raw: *mut std::ffi::c_void,
}

impl PetuniaStudioWindow {
    pub fn new() -> Self {
        unsafe extern "C" {
            fn petunia_main_window_create() -> *mut std::ffi::c_void;
        }
        let raw = unsafe { petunia_main_window_create() };
        assert!(
            !raw.is_null(),
            "Create the window on the QApplication thread"
        );
        Self { raw }
    }

    pub fn show(&self) {
        unsafe extern "C" {
            fn petunia_main_window_show(win: *mut std::ffi::c_void);
        }
        unsafe { petunia_main_window_show(self.raw) };
    }
}

impl Default for PetuniaStudioWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PetuniaStudioWindow {
    fn drop(&mut self) {
        unsafe extern "C" {
            fn petunia_main_window_destroy(win: *mut std::ffi::c_void);
        }
        if !self.raw.is_null() {
            unsafe { petunia_main_window_destroy(self.raw) };
        }
    }
}

/// Inicia e executa a interface gráfica Qt Widgets do Petunia3D.
pub fn run() -> Result<(), String> {
    let app = PetuniaWidgetsApp::new();
    let window = PetuniaStudioWindow::new();
    window.show();
    let code = app.exec();
    if code == 0 {
        Ok(())
    } else {
        Err(format!("Aplicação Qt finalizada com código {code}"))
    }
}
