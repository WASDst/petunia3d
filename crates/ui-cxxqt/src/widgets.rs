//! Safe Rust wrapper para o PetuniaMainWindow e QApplication (Qt Widgets com docas multijanela).

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
