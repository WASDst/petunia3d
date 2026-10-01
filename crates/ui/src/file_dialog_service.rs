//! Serviço integrado de diálogos de arquivo para o Petunia3D.
//!
//! Encapsula todas as chamadas a diálogos nativos do sistema operacional (`rfd::FileDialog`)
//! e diálogos in-canvas (`egui-file-dialog`), delegando todas as operações de persistência
//! e conversão de formatos ao `ProjectService` puro do `petunia_core`.

use std::cell::RefCell;
use std::path::PathBuf;

use egui::Context;
use egui_file_dialog::FileDialog;
use petunia_core::{AppState, ProjectService, RefAxis};

/// Ação pendente solicitada através do diálogo de arquivos in-canvas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDialogAction {
    OpenProject,
    SaveProject,
    SaveProjectAs,
    ImportObj,
    ImportGltf,
    ExportObj,
    ExportGlb,
    ImportPalette,
    ExportPalette,
    AddReferenceImage(Option<RefAxis>),
}

/// Serviço gerenciador de diálogo de arquivos integrado à UI (in-canvas modal).
pub struct PetuniaFileDialogService {
    dialog: FileDialog,
    pending_action: Option<FileDialogAction>,
}

impl Default for PetuniaFileDialogService {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static SERVICE: RefCell<PetuniaFileDialogService> = RefCell::new(PetuniaFileDialogService::new());
}

/// Abre diálogo in-canvas para carregar imagem de referência para um slot ortográfico opcional.
pub fn open_reference_image_dialog(axis: Option<RefAxis>) {
    SERVICE.with(|s| s.borrow_mut().pick_reference_image(axis));
}

/// Abre diálogo in-canvas para carregar um projeto (.petunia).
pub fn open_project_in_canvas() {
    SERVICE.with(|s| s.borrow_mut().open_project());
}

/// Abre diálogo in-canvas para salvar o projeto (.petunia).
pub fn save_project_in_canvas(current_path: Option<&str>, state: &mut AppState) {
    SERVICE.with(|s| s.borrow_mut().save_project(current_path, state));
}

/// Abre diálogo in-canvas para salvar como novo arquivo (.petunia).
pub fn save_project_as_in_canvas() {
    SERVICE.with(|s| s.borrow_mut().save_project_as());
}

/// Abre diálogo in-canvas para importar malha Wavefront OBJ.
pub fn import_obj_in_canvas() {
    SERVICE.with(|s| s.borrow_mut().import_obj());
}

/// Abre diálogo in-canvas para importar malha glTF/GLB (com texturas).
pub fn import_gltf_in_canvas() {
    SERVICE.with(|s| s.borrow_mut().import_gltf());
}

/// Abre diálogo in-canvas para exportar malha Wavefront OBJ.
pub fn export_obj_in_canvas(default_name: &str) {
    SERVICE.with(|s| s.borrow_mut().export_obj(default_name));
}

/// Abre diálogo in-canvas para exportar malha glTF Binário (.glb).
pub fn export_glb_in_canvas(default_name: &str) {
    SERVICE.with(|s| s.borrow_mut().export_glb(default_name));
}

/// Abre diálogo in-canvas para importar paleta de cores.
pub fn import_palette_in_canvas() {
    SERVICE.with(|s| s.borrow_mut().import_palette());
}

/// Abre diálogo in-canvas para exportar paleta de cores.
pub fn export_palette_in_canvas(default_name: &str) {
    SERVICE.with(|s| s.borrow_mut().export_palette(default_name));
}

/// Desenha e processa o diálogo in-canvas no frame atual do egui.
pub fn draw(ctx: &Context, state: &mut AppState) {
    SERVICE.with(|s| s.borrow_mut().update(ctx, state));
}

impl PetuniaFileDialogService {
    pub fn new() -> Self {
        Self {
            dialog: FileDialog::new()
                .as_modal(true)
                .title("Petunia3D File Explorer"),
            pending_action: None,
        }
    }

    /// Abre o diálogo para selecionar uma imagem de referência in-canvas.
    pub fn pick_reference_image(&mut self, axis: Option<RefAxis>) {
        self.pending_action = Some(FileDialogAction::AddReferenceImage(axis));
        let mut dialog = FileDialog::new()
            .as_modal(true)
            .title("Selecionar Imagem de Referência")
            .add_file_filter_extensions(
                "Imagens (*.png, *.jpg, *.jpeg, *.webp)",
                vec!["png", "jpg", "jpeg", "webp"],
            );
        dialog.pick_file();
        self.dialog = dialog;
    }

    /// Abre o diálogo para carregar um projeto (.petunia).
    pub fn open_project(&mut self) {
        self.pending_action = Some(FileDialogAction::OpenProject);
        let mut dialog = FileDialog::new()
            .as_modal(true)
            .title("Open Petunia Project")
            .add_file_filter_extensions("Petunia Project (*.petunia)", vec!["petunia"]);
        dialog.pick_file();
        self.dialog = dialog;
    }

    /// Abre o diálogo para salvar o projeto (.petunia).
    pub fn save_project(&mut self, current_path: Option<&str>, state: &mut AppState) {
        if let Some(path) = current_path {
            let p = PathBuf::from(path);
            self.execute_action(FileDialogAction::SaveProject, p, state);
        } else {
            self.save_project_as();
        }
    }

    /// Abre o diálogo para salvar como novo arquivo (.petunia).
    pub fn save_project_as(&mut self) {
        self.pending_action = Some(FileDialogAction::SaveProjectAs);
        let mut dialog = FileDialog::new()
            .as_modal(true)
            .title("Save Petunia Project")
            .add_file_filter_extensions("Petunia Project (*.petunia)", vec!["petunia"])
            .default_file_name("project.petunia");
        dialog.save_file();
        self.dialog = dialog;
    }

    /// Abre o diálogo para importar malha Wavefront OBJ.
    pub fn import_obj(&mut self) {
        self.pending_action = Some(FileDialogAction::ImportObj);
        let mut dialog = FileDialog::new()
            .as_modal(true)
            .title("Import OBJ Mesh")
            .add_file_filter_extensions("Wavefront OBJ (*.obj)", vec!["obj"]);
        dialog.pick_file();
        self.dialog = dialog;
    }

    /// Abre o diálogo para importar malha glTF/GLB (geometria + materiais + texturas).
    pub fn import_gltf(&mut self) {
        self.pending_action = Some(FileDialogAction::ImportGltf);
        let mut dialog = FileDialog::new()
            .as_modal(true)
            .title("Import glTF / GLB Mesh")
            .add_file_filter_extensions("glTF (*.gltf, *.glb)", vec!["gltf", "glb"]);
        dialog.pick_file();
        self.dialog = dialog;
    }

    /// Abre o diálogo para exportar malha Wavefront OBJ.
    pub fn export_obj(&mut self, default_name: &str) {
        self.pending_action = Some(FileDialogAction::ExportObj);
        let file_name = format!("{default_name}.obj");
        let mut dialog = FileDialog::new()
            .as_modal(true)
            .title("Export OBJ Mesh")
            .add_file_filter_extensions("Wavefront OBJ (*.obj)", vec!["obj"])
            .default_file_name(&file_name);
        dialog.save_file();
        self.dialog = dialog;
    }

    /// Abre o diálogo para exportar malha glTF Binário (.glb).
    pub fn export_glb(&mut self, default_name: &str) {
        self.pending_action = Some(FileDialogAction::ExportGlb);
        let file_name = format!("{default_name}.glb");
        let mut dialog = FileDialog::new()
            .as_modal(true)
            .title("Export glTF Binary")
            .add_file_filter_extensions("glTF Binary (*.glb)", vec!["glb"])
            .default_file_name(&file_name);
        dialog.save_file();
        self.dialog = dialog;
    }

    /// Abre o diálogo para importar paleta de cores.
    pub fn import_palette(&mut self) {
        self.pending_action = Some(FileDialogAction::ImportPalette);
        let mut dialog = FileDialog::new()
            .as_modal(true)
            .title("Import Color Palette")
            .add_file_filter_extensions("Palette (*.hex, *.gpl)", vec!["hex", "gpl"]);
        dialog.pick_file();
        self.dialog = dialog;
    }

    /// Abre o diálogo para exportar paleta de cores.
    pub fn export_palette(&mut self, default_name: &str) {
        self.pending_action = Some(FileDialogAction::ExportPalette);
        let file_name = format!("{default_name}.gpl");
        let mut dialog = FileDialog::new()
            .as_modal(true)
            .title("Export GIMP Palette")
            .add_file_filter_extensions("GIMP Palette (*.gpl)", vec!["gpl"])
            .default_file_name(&file_name);
        dialog.save_file();
        self.dialog = dialog;
    }

    /// Atualiza o diálogo de arquivos no frame e despacha a ação quando confirmada.
    pub fn update(&mut self, ctx: &Context, state: &mut AppState) {
        self.dialog.update(ctx);

        if let Some(path) = self.dialog.take_picked()
            && let Some(action) = self.pending_action.take()
        {
            self.execute_action(action, path, state);
        }
    }

    fn execute_action(&self, action: FileDialogAction, path: PathBuf, state: &mut AppState) {
        match action {
            FileDialogAction::OpenProject => {
                if let Err(e) = ProjectService::load_project(state, &path) {
                    state.set_status(format!("open err: {e}"));
                }
            }
            FileDialogAction::SaveProject | FileDialogAction::SaveProjectAs => {
                if let Err(e) = ProjectService::save_project(state, &path) {
                    state.set_status(format!("save err: {e}"));
                }
            }
            FileDialogAction::ImportObj => {
                if let Err(e) = ProjectService::import_obj(state, &path) {
                    state.set_status(format!("import err: {e}"));
                }
            }
            FileDialogAction::ImportGltf => {
                match ProjectService::import_file_pipeline(
                    state,
                    &path,
                    &petunia_project::pipeline::ImportOptions::default(),
                ) {
                    Ok(names) => state.set_status(format!(
                        "imported {} asset(s) from {}",
                        names.len(),
                        path.display()
                    )),
                    Err(e) => state.set_status(format!("import err: {e}")),
                }
            }
            FileDialogAction::ExportObj => {
                let active = state.project.active;
                match ProjectService::export_obj(state, active, &path) {
                    Ok(()) => state.set_status(format!("exported obj {}", path.display())),
                    Err(e) => state.set_status(format!("export obj err: {e}")),
                }
            }
            FileDialogAction::ExportGlb => {
                let active = state.project.active;
                match ProjectService::export_glb(state, &[active], &path) {
                    Ok(()) => state.set_status(format!("exported glb {}", path.display())),
                    Err(e) => state.set_status(format!("export glb err: {e}")),
                }
            }
            FileDialogAction::ImportPalette => {
                if let Err(e) = ProjectService::import_palette(state, &path) {
                    state.set_status(format!("import palette err: {e}"));
                }
            }
            FileDialogAction::ExportPalette => {
                match ProjectService::export_palette(
                    &state.project.palette,
                    "Petunia Palette",
                    &path,
                ) {
                    Ok(()) => state.set_status(format!("exported palette to {}", path.display())),
                    Err(e) => state.set_status(format!("export palette err: {e}")),
                }
            }
            FileDialogAction::AddReferenceImage(opt_axis) => match crate::load_image_rgba(&path) {
                Ok((w, h, rgba)) => {
                    let name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("ref")
                        .to_string();
                    if let Some(axis) = opt_axis {
                        ProjectService::set_reference_slot(state, axis, name, w, h, rgba);
                    } else {
                        ProjectService::add_reference_image(state, name.clone(), w, h, rgba);
                        state.set_status(format!("Imagem de referência '{name}' adicionada"));
                    }
                }
                Err(e) => state.set_status(format!("Erro ao carregar imagem: {e}")),
            },
        }
    }
}

/// Diálogos de arquivo nativos do sistema operacional (Desktop / Fallback síncrono).
pub mod native {
    use std::path::PathBuf;

    /// Abre diálogo nativo do sistema para selecionar um arquivo de projeto (.petunia).
    pub fn pick_project_file() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Petunia Project (*.petunia)", &["petunia"])
            .pick_file()
    }

    /// Abre diálogo nativo para salvar um arquivo de projeto (.petunia).
    pub fn pick_save_project_file(default_name: &str) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Petunia Project (*.petunia)", &["petunia"])
            .set_file_name(default_name)
            .save_file()
    }

    /// Abre diálogo nativo para selecionar malha Wavefront OBJ.
    pub fn pick_obj_file() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Wavefront OBJ (*.obj)", &["obj"])
            .pick_file()
    }

    /// Abre diálogo nativo para selecionar malha glTF/GLB.
    pub fn pick_gltf_file() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("glTF (*.gltf, *.glb)", &["gltf", "glb"])
            .pick_file()
    }

    /// Abre diálogo nativo para salvar malha Wavefront OBJ.
    pub fn pick_export_obj_file(default_name: &str) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Wavefront OBJ (*.obj)", &["obj"])
            .set_file_name(default_name)
            .save_file()
    }

    /// Abre diálogo nativo para salvar arquivo glTF Binário (.glb).
    pub fn pick_export_glb_file(default_name: &str) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("glTF Binary (*.glb)", &["glb"])
            .set_file_name(default_name)
            .save_file()
    }

    /// Abre diálogo nativo para selecionar um diretório destino.
    pub fn pick_folder() -> Option<PathBuf> {
        rfd::FileDialog::new().pick_folder()
    }

    /// Abre diálogo nativo para carregar imagens de textura/referência.
    pub fn pick_image_file() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter(
                "Imagens (*.png, *.jpg, *.jpeg, *.webp)",
                &["png", "jpg", "jpeg", "webp"],
            )
            .pick_file()
    }

    /// Abre diálogo nativo para importar paleta (.hex ou .gpl).
    pub fn pick_palette_import_file() -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("Paleta (*.hex, *.gpl)", &["hex", "gpl"])
            .pick_file()
    }

    /// Abre diálogo nativo para exportar paleta (.gpl).
    pub fn pick_palette_export_file(default_name: &str) -> Option<PathBuf> {
        rfd::FileDialog::new()
            .add_filter("GIMP Palette (*.gpl)", &["gpl"])
            .set_file_name(default_name)
            .save_file()
    }
}

pub use native::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_dialog_service_initialization() {
        let service = PetuniaFileDialogService::new();
        assert!(service.pending_action.is_none());
    }

    #[test]
    fn test_file_dialog_action_triggers() {
        let mut service = PetuniaFileDialogService::new();
        service.open_project();
        assert_eq!(service.pending_action, Some(FileDialogAction::OpenProject));

        service.import_obj();
        assert_eq!(service.pending_action, Some(FileDialogAction::ImportObj));

        service.import_gltf();
        assert_eq!(service.pending_action, Some(FileDialogAction::ImportGltf));

        service.save_project_as();
        assert_eq!(
            service.pending_action,
            Some(FileDialogAction::SaveProjectAs)
        );

        service.export_obj("test_model");
        assert_eq!(service.pending_action, Some(FileDialogAction::ExportObj));

        service.export_glb("test_model");
        assert_eq!(service.pending_action, Some(FileDialogAction::ExportGlb));

        service.import_palette();
        assert_eq!(
            service.pending_action,
            Some(FileDialogAction::ImportPalette)
        );

        service.export_palette("test_palette");
        assert_eq!(
            service.pending_action,
            Some(FileDialogAction::ExportPalette)
        );
    }
}
