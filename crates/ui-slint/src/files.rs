//! Boundary de arquivos da UI experimental.
//!
//! Callbacks Slint devem chamar este serviço, nunca `rfd` diretamente. O
//! domínio recebe caminhos; não conhece detalhes do diálogo nativo.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileDialogKind {
    OpenProject,
    SaveProject,
    ImportMesh,
    ExportMesh,
    SelectAssetFolder,
}

impl FileDialogKind {
    pub const fn title_key(self) -> &'static str {
        match self {
            Self::OpenProject => "files.open_project",
            Self::SaveProject => "files.save_project",
            Self::ImportMesh => "files.import_mesh",
            Self::ExportMesh => "files.export_mesh",
            Self::SelectAssetFolder => "files.select_asset_folder",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct FileDialogService;

impl FileDialogService {
    pub const fn new() -> Self {
        Self
    }

    pub async fn open_project(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("Petunia project", &["petunia", "pkg"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    pub async fn save_project(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("Petunia project", &["petunia"])
            .set_file_name("untitled.petunia")
            .save_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    pub async fn import_mesh(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("3D mesh", &["obj", "gltf", "glb"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    pub async fn export_mesh(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("Wavefront OBJ", &["obj"])
            .add_filter("glTF", &["gltf", "glb"])
            .save_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    pub async fn select_asset_folder(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .pick_folder()
            .await
            .map(|file| file.path().to_path_buf())
    }

    /// Importa um modelo 3D do disco (OBJ, glTF ou GLB).
    pub async fn import_model(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("3D model", &["obj", "gltf", "glb"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    /// Escolhe o destino de exportação OBJ.
    pub async fn export_obj(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("Wavefront OBJ", &["obj"])
            .set_file_name("model.obj")
            .save_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    /// Escolhe o destino de exportação GLB.
    pub async fn export_glb(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("glTF Binary", &["glb"])
            .set_file_name("scene.glb")
            .save_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    /// Importa uma paleta de cores do disco (.gpl ou .hex).
    pub async fn import_palette(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("Color Palette", &["gpl", "hex"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    /// Exporta a paleta de cores ativa (.gpl).
    pub async fn export_palette(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("GIMP Palette", &["gpl"])
            .set_file_name("palette.gpl")
            .save_file()
            .await
            .map(|file| file.path().to_path_buf())
    }

    /// Seleciona uma imagem de referência do disco.
    pub async fn open_reference_image(&self) -> Option<PathBuf> {
        rfd::AsyncFileDialog::new()
            .add_filter("Image", &["png", "jpg", "jpeg", "webp", "bmp"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }
}

/// Carrega e decodifica uma imagem em RGBA8 até no máximo 2048x2048.
pub fn load_image_rgba(path: &std::path::Path) -> Result<(u32, u32, Vec<u8>), String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let mut rgba = img.to_rgba8();
    const MAX: u32 = 2048;
    if rgba.width() > MAX || rgba.height() > MAX {
        let (w, h) = (rgba.width(), rgba.height());
        let s = (MAX as f32 / w.max(h) as f32).min(1.0);
        rgba = image::imageops::resize(
            &rgba,
            ((w as f32 * s) as u32).max(1),
            ((h as f32 * s) as u32).max(1),
            image::imageops::FilterType::Lanczos3,
        );
    }
    Ok((rgba.width(), rgba.height(), rgba.into_raw()))
}

/// Gera uma [`slint::Image`] de miniatura a partir de um buffer de pixels RGBA.
pub fn create_thumbnail_image(width: u32, height: u32, rgba: &[u8]) -> Option<slint::Image> {
    if width == 0 || height == 0 || rgba.len() < (width * height * 4) as usize {
        return None;
    }
    const THUMB_MAX: u32 = 180;
    let s = (THUMB_MAX as f32 / width.max(height) as f32).min(1.0);
    let target_w = ((width as f32 * s) as u32).max(1);
    let target_h = ((height as f32 * s) as u32).max(1);

    let mut buf = slint::SharedPixelBuffer::<slint::Rgba8Pixel>::new(target_w, target_h);
    let out_bytes = buf.make_mut_bytes();

    if target_w == width && target_h == height {
        let len = out_bytes.len().min(rgba.len());
        out_bytes[..len].copy_from_slice(&rgba[..len]);
    } else {
        for y in 0..target_h {
            let src_y = (((y as f32) / (target_h as f32)) * (height as f32)) as u32;
            for x in 0..target_w {
                let src_x = (((x as f32) / (target_w as f32)) * (width as f32)) as u32;
                let src_idx = ((src_y * width + src_x) * 4) as usize;
                let dst_idx = ((y * target_w + x) * 4) as usize;
                if src_idx + 3 < rgba.len() && dst_idx + 3 < out_bytes.len() {
                    out_bytes[dst_idx] = rgba[src_idx];
                    out_bytes[dst_idx + 1] = rgba[src_idx + 1];
                    out_bytes[dst_idx + 2] = rgba[src_idx + 2];
                    out_bytes[dst_idx + 3] = rgba[src_idx + 3];
                }
            }
        }
    }
    Some(slint::Image::from_rgba8(buf))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_kinds_have_stable_translation_keys() {
        assert_eq!(
            FileDialogKind::OpenProject.title_key(),
            "files.open_project"
        );
        assert_eq!(FileDialogKind::ExportMesh.title_key(), "files.export_mesh");
    }
}
