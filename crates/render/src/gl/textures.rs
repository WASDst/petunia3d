//! Cache de texturas GL por revisão (nunca por hash de pixels). Com regiões
//! sujas (Paint), só os retângulos alterados sobem.

use std::collections::HashSet;

use glow::{HasContext, NativeTexture, PixelUnpackData};

use super::{GlRenderer, TextureSlot};
use crate::frame::{DirtyRect, RenderFrameInput};

unsafe fn create_texture(gl: &glow::Context) -> Result<NativeTexture, String> {
    // SAFETY: the context is current on the rendering thread.
    unsafe {
        let texture = gl
            .create_texture()
            .map_err(|error| format!("OpenGL texture: {error}"))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        for (parameter, value) in [
            (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
            (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, value as i32);
        }
        Ok(texture)
    }
}

unsafe fn upload_full(
    gl: &glow::Context,
    texture: NativeTexture,
    width: u32,
    height: u32,
    pixels: &[u8],
) {
    // SAFETY: the context is current on the rendering thread.
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
        gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
        gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA8 as i32,
            1.max(width as i32),
            1.max(height as i32),
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelUnpackData::Slice(Some(pixels)),
        );
    }
}

/// Envia só o retângulo, lendo direto do canvas inteiro (sem cópia em CPU).
unsafe fn upload_region(
    gl: &glow::Context,
    texture: NativeTexture,
    width: u32,
    pixels: &[u8],
    region: DirtyRect,
) {
    // SAFETY: the context is current on the rendering thread.
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, width as i32);
        gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, region.x as i32);
        gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, region.y as i32);
        gl.tex_sub_image_2d(
            glow::TEXTURE_2D,
            0,
            region.x as i32,
            region.y as i32,
            region.width as i32,
            region.height as i32,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            PixelUnpackData::Slice(Some(pixels)),
        );
        gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);
        gl.pixel_store_i32(glow::UNPACK_SKIP_PIXELS, 0);
        gl.pixel_store_i32(glow::UNPACK_SKIP_ROWS, 0);
    }
}

/// Textura 1×1 de cor fixa (amostrador neutro).
pub(super) unsafe fn solid_texture(
    gl: &glow::Context,
    rgba: [u8; 4],
) -> Result<NativeTexture, String> {
    // SAFETY: the context is current on the rendering thread.
    unsafe {
        let texture = create_texture(gl)?;
        upload_full(gl, texture, 1, 1, &rgba);
        gl.bind_texture(glow::TEXTURE_2D, None);
        Ok(texture)
    }
}

impl GlRenderer {
    /// Garante as texturas dos objetos: reenvia quando a revisão muda, por
    /// regiões quando o frame as declara. Poda as que saíram do frame.
    pub(super) unsafe fn sync_object_textures(&mut self, frame: &RenderFrameInput<'_>) {
        let gl = std::sync::Arc::clone(&self.gl);
        let live: HashSet<_> = frame.textures.iter().map(|texture| texture.id).collect();
        self.object_textures.retain(|id, slot| {
            let keep = live.contains(id);
            if !keep {
                gl.delete_texture(slot.texture);
            }
            keep
        });
        for source in &frame.textures {
            let expected = source.width as usize * source.height as usize * 4;
            if source.pixels.len() < expected {
                continue; // pixels ainda não copiados (revisão já enviada)
            }
            match self.object_textures.get_mut(&source.id) {
                Some(slot) if slot.width == source.width && slot.height == source.height => {
                    if slot.revision == source.revision {
                        self.stats.skipped_uploads += 1;
                        continue;
                    }
                    match &source.dirty {
                        Some(regions) => {
                            for region in regions
                                .iter()
                                .filter_map(|r| r.clipped(source.width, source.height))
                            {
                                upload_region(
                                    &gl,
                                    slot.texture,
                                    source.width,
                                    source.pixels,
                                    region,
                                );
                                self.stats.partial_texture_uploads += 1;
                                self.stats.texture_bytes_uploaded +=
                                    u64::from(region.width) * u64::from(region.height) * 4;
                            }
                        }
                        None => {
                            upload_full(
                                &gl,
                                slot.texture,
                                source.width,
                                source.height,
                                source.pixels,
                            );
                            self.stats.full_texture_uploads += 1;
                            self.stats.texture_bytes_uploaded += expected as u64;
                        }
                    }
                    slot.revision = source.revision;
                }
                _ => {
                    let texture = match self.object_textures.remove(&source.id) {
                        Some(slot) => slot.texture,
                        None => match create_texture(&gl) {
                            Ok(texture) => texture,
                            Err(error) => {
                                eprintln!("petunia3d: {error}");
                                continue;
                            }
                        },
                    };
                    upload_full(&gl, texture, source.width, source.height, source.pixels);
                    self.stats.full_texture_uploads += 1;
                    self.stats.texture_bytes_uploaded += expected as u64;
                    self.object_textures.insert(
                        source.id,
                        TextureSlot {
                            width: source.width,
                            height: source.height,
                            revision: source.revision,
                            texture,
                        },
                    );
                }
            }
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
    }

    /// Uma textura por imagem de referência, reenviada quando a revisão muda.
    pub(super) unsafe fn sync_reference_textures(&mut self, frame: &RenderFrameInput<'_>) {
        let gl = std::sync::Arc::clone(&self.gl);
        let count = frame.references.len();
        while self.reference_textures.len() < count {
            let Ok(texture) = solid_texture(&gl, [128, 128, 128, 255]) else {
                return;
            };
            self.reference_textures.push(TextureSlot {
                width: 0,
                height: 0,
                revision: u64::MAX,
                texture,
            });
        }
        if self.reference_textures.len() > count {
            for slot in self.reference_textures.drain(count..) {
                gl.delete_texture(slot.texture);
            }
        }
        for (slot, image) in self.reference_textures.iter_mut().zip(&frame.references) {
            let expected = image.width as usize * image.height as usize * 4;
            if image.rgba.len() < expected || expected == 0 {
                continue;
            }
            if slot.revision != image.revision
                || slot.width != image.width
                || slot.height != image.height
            {
                upload_full(&gl, slot.texture, image.width, image.height, image.rgba);
                slot.width = image.width;
                slot.height = image.height;
                slot.revision = image.revision;
            }
        }
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
}
