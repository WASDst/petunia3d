use core::pin::Pin;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qproperty(QString, active_workspace)]
        #[qproperty(bool, can_undo)]
        #[qproperty(bool, can_redo)]
        #[qproperty(i32, selected_count)]
        #[qproperty(bool, is_dirty)]
        #[qproperty(QString, status_message)]
        type PetuniaShellBridge = super::PetuniaShellBridgeRust;

        #[qinvokable]
        fn switch_workspace(self: Pin<&mut PetuniaShellBridge>, workspace_id: &QString);

        #[qinvokable]
        fn dispatch_action(self: Pin<&mut PetuniaShellBridge>, action_name: &QString);
    }
}

#[derive(Default)]
pub struct PetuniaShellBridgeRust {
    pub active_workspace: cxx_qt_lib::QString,
    pub can_undo: bool,
    pub can_redo: bool,
    pub selected_count: i32,
    pub is_dirty: bool,
    pub status_message: cxx_qt_lib::QString,
}

impl qobject::PetuniaShellBridge {
    pub fn switch_workspace(mut self: Pin<&mut Self>, workspace_id: &cxx_qt_lib::QString) {
        self.as_mut().set_active_workspace(workspace_id.clone());
    }

    pub fn dispatch_action(mut self: Pin<&mut Self>, action_name: &cxx_qt_lib::QString) {
        let name_str = action_name.to_string();
        if let Ok(mut guard) = crate::render::opengl_viewport::VIEWPORT_RENDERER.lock() {
            match name_str.as_str() {
                "app.undo" | "edit.undo" => {
                    guard.app.undo();
                }
                "app.redo" | "edit.redo" => {
                    guard.app.redo();
                }
                "project.new" | "file.new" => {
                    guard.app.state.project.reset();
                }
                _ => {
                    let _ = guard.app.execute_registered_command(&name_str);
                }
            }
            let is_dirty = guard.app.state.is_document_dirty();
            let selected_count = if guard.app.state.project.assets.is_empty() {
                0
            } else {
                1
            };
            self.as_mut().set_is_dirty(is_dirty);
            self.as_mut().set_selected_count(selected_count);
        }
    }
}
