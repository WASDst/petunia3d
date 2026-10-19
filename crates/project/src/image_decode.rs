//! Decodificação de imagens externas (texturas) em [`Canvas`].
//!
//! Um único lugar para os limites de segurança e o redimensionamento: glTF
//! (imagem embutida ou externa) e OBJ (`.mtl` com `map_Kd`) usam exatamente a
//! mesma regra, então um arquivo hostil não passa por um caminho mais frouxo.

use crate::Canvas;
use std::path::Path;

/// Teto de bytes por imagem (64 MiB): acima disso a imagem é recusada.
pub const MAX_IMAGE_BYTES: usize = 64 * 1024 * 1024;

/// Decodifica bytes de imagem (PNG, JPEG, …) em um canvas RGBA.
pub fn decode_image_bytes(bytes: &[u8]) -> Result<Canvas, String> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(format!(
            "imagem excede {} MiB",
            MAX_IMAGE_BYTES / (1024 * 1024)
        ));
    }
    let dyn_img = image::load_from_memory(bytes).map_err(|e| format!("decode: {e}"))?;
    let mut rgba = dyn_img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    if w == 0 || h == 0 {
        return Err("imagem vazia".to_string());
    }
    if w > Canvas::MAX_SIDE || h > Canvas::MAX_SIDE {
        let s = (Canvas::MAX_SIDE as f32 / w.max(h) as f32).min(1.0);
        let nw = ((w as f32 * s) as u32).max(1);
        let nh = ((h as f32 * s) as u32).max(1);
        rgba = image::imageops::resize(&rgba, nw, nh, image::imageops::FilterType::Triangle);
    }
    Ok(Canvas {
        w: rgba.width(),
        h: rgba.height(),
        pixels: rgba.into_raw().into(),
    })
}

/// Lê a imagem do disco e a decodifica.
pub fn decode_image_file(path: &Path) -> Result<Canvas, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if meta.len() > MAX_IMAGE_BYTES as u64 {
        return Err(format!(
            "imagem '{}' excede {} MiB",
            path.display(),
            MAX_IMAGE_BYTES / (1024 * 1024)
        ));
    }
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    decode_image_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PNG 1×1 vermelho.
    const PNG_1X1: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90,
        0x77, 0x53, 0xDE, 0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, 0x08, 0xD7, 0x63, 0xF8,
        0xCF, 0xC0, 0x00, 0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00,
        0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn decodes_a_png_into_rgba_canvas() {
        let canvas = decode_image_bytes(PNG_1X1).expect("png");
        assert_eq!((canvas.w, canvas.h), (1, 1));
        assert_eq!(canvas.pixels.len(), 4);
        assert_eq!(&canvas.pixels[..3], &[255, 0, 0]);
    }

    #[test]
    fn garbage_is_refused_without_panicking() {
        assert!(decode_image_bytes(b"nao sou png").is_err());
    }
}
