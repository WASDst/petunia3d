//! Alvos fora da tela do viewport e salvamento do estado GL do host.
//!
//! O host Slint/FemtoVG compõe a textura de cor resolvida diretamente (sem
//! `glReadPixels`); o renderer só devolve o contexto no estado em que o recebeu.
//! O MSAA usa FBO multisample próprio + resolve: o framebuffer do host nunca
//! precisa ter o sample count desejado.

use std::num::NonZeroU32;

use glow::{HasContext, NativeFramebuffer, NativeRenderbuffer, NativeTexture};

unsafe fn texture_2d(
    gl: &glow::Context,
    width: u32,
    height: u32,
    internal: u32,
    format: u32,
    kind: u32,
    filter: u32,
) -> Result<NativeTexture, String> {
    // SAFETY: caller keeps the context current on this thread.
    unsafe {
        let texture = gl
            .create_texture()
            .map_err(|error| format!("OpenGL texture: {error}"))?;
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            internal as i32,
            width as i32,
            height as i32,
            0,
            format,
            kind,
            glow::PixelUnpackData::Slice(None),
        );
        for (parameter, value) in [
            (glow::TEXTURE_MIN_FILTER, filter),
            (glow::TEXTURE_MAG_FILTER, filter),
            (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
            (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
        ] {
            gl.tex_parameter_i32(glow::TEXTURE_2D, parameter, value as i32);
        }
        Ok(texture)
    }
}

unsafe fn complete(gl: &glow::Context, label: &str) -> Result<(), String> {
    // SAFETY: caller keeps the context current with the framebuffer bound.
    let status = unsafe { gl.check_framebuffer_status(glow::FRAMEBUFFER) };
    if status == glow::FRAMEBUFFER_COMPLETE {
        Ok(())
    } else {
        Err(format!(
            "OpenGL {label} framebuffer incomplete: {status:#x}"
        ))
    }
}

/// FBO do viewport: cor RGBA8 em textura (importada pelo host) e profundidade
/// 24 bits. Com `samples > 1` desenha num FBO multisample e resolve na textura.
pub struct GlTarget {
    /// FBO com a textura de cor (destino do resolve, ou o próprio alvo sem MSAA).
    resolve: NativeFramebuffer,
    color: NativeTexture,
    /// Profundidade do FBO sem MSAA.
    depth: Option<NativeRenderbuffer>,
    /// FBO multisample (cor + profundidade).
    msaa: Option<(NativeFramebuffer, NativeRenderbuffer, NativeRenderbuffer)>,
    width: u32,
    height: u32,
    samples: u32,
    /// Amostras pedidas (antes de limitar ao suportado).
    requested: u32,
}

impl GlTarget {
    /// Alvo sem MSAA.
    ///
    /// # Safety
    /// O contexto GL precisa estar corrente na thread de renderização.
    pub unsafe fn new(gl: &glow::Context, width: u32, height: u32) -> Result<Self, String> {
        // SAFETY: delegated to the caller.
        unsafe { Self::with_samples(gl, width, height, 1) }
    }

    /// Alvo com `samples` amostras (limitado ao suportado; 1 = sem MSAA).
    ///
    /// # Safety
    /// O contexto GL precisa estar corrente na thread de renderização.
    pub unsafe fn with_samples(
        gl: &glow::Context,
        width: u32,
        height: u32,
        samples: u32,
    ) -> Result<Self, String> {
        let (width, height) = (width.max(1), height.max(1));
        // SAFETY: delegated to the caller (current context on this thread).
        unsafe {
            let max_samples = gl.get_parameter_i32(glow::MAX_SAMPLES).max(1) as u32;
            let requested = samples;
            let samples = samples.clamp(1, max_samples);
            let saved = GlStateGuard::save(gl);
            let result = (|| {
                let color = texture_2d(
                    gl,
                    width,
                    height,
                    glow::RGBA8,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::LINEAR,
                )?;
                let resolve = gl
                    .create_framebuffer()
                    .map_err(|error| format!("OpenGL viewport framebuffer: {error}"))?;
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(resolve));
                gl.framebuffer_texture_2d(
                    glow::FRAMEBUFFER,
                    glow::COLOR_ATTACHMENT0,
                    glow::TEXTURE_2D,
                    Some(color),
                    0,
                );
                let mut target = Self {
                    resolve,
                    color,
                    depth: None,
                    msaa: None,
                    width,
                    height,
                    samples,
                    requested,
                };
                if samples > 1 {
                    complete(gl, "viewport resolve")?;
                    let msaa = gl
                        .create_framebuffer()
                        .map_err(|error| format!("OpenGL MSAA framebuffer: {error}"))?;
                    let msaa_color = gl
                        .create_renderbuffer()
                        .map_err(|error| format!("OpenGL MSAA color: {error}"))?;
                    let msaa_depth = gl
                        .create_renderbuffer()
                        .map_err(|error| format!("OpenGL MSAA depth: {error}"))?;
                    target.msaa = Some((msaa, msaa_color, msaa_depth));
                    gl.bind_framebuffer(glow::FRAMEBUFFER, Some(msaa));
                    for (renderbuffer, format, attachment) in [
                        (msaa_color, glow::RGBA8, glow::COLOR_ATTACHMENT0),
                        (msaa_depth, glow::DEPTH_COMPONENT24, glow::DEPTH_ATTACHMENT),
                    ] {
                        gl.bind_renderbuffer(glow::RENDERBUFFER, Some(renderbuffer));
                        gl.renderbuffer_storage_multisample(
                            glow::RENDERBUFFER,
                            samples as i32,
                            format,
                            width as i32,
                            height as i32,
                        );
                        gl.framebuffer_renderbuffer(
                            glow::FRAMEBUFFER,
                            attachment,
                            glow::RENDERBUFFER,
                            Some(renderbuffer),
                        );
                    }
                    if let Err(error) = complete(gl, "viewport MSAA") {
                        target.delete(gl);
                        return Err(error);
                    }
                } else {
                    let depth = gl
                        .create_renderbuffer()
                        .map_err(|error| format!("OpenGL viewport depth: {error}"))?;
                    target.depth = Some(depth);
                    gl.bind_renderbuffer(glow::RENDERBUFFER, Some(depth));
                    gl.renderbuffer_storage(
                        glow::RENDERBUFFER,
                        glow::DEPTH_COMPONENT24,
                        width as i32,
                        height as i32,
                    );
                    gl.framebuffer_renderbuffer(
                        glow::FRAMEBUFFER,
                        glow::DEPTH_ATTACHMENT,
                        glow::RENDERBUFFER,
                        Some(depth),
                    );
                    if let Err(error) = complete(gl, "viewport") {
                        target.delete(gl);
                        return Err(error);
                    }
                }
                Ok(target)
            })();
            saved.restore(gl);
            result
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Amostras efetivas do alvo (1 = sem MSAA).
    pub fn samples(&self) -> u32 {
        self.samples
    }

    /// Amostras pedidas na criação (o host recria o alvo quando o pedido muda).
    pub fn requested_samples(&self) -> u32 {
        self.requested
    }

    /// Id da textura de cor resolvida, para o host importar sem cópia.
    pub fn texture_id(&self) -> NonZeroU32 {
        self.color.0
    }

    /// FBO onde os passes desenham.
    pub(crate) fn draw_framebuffer(&self) -> NativeFramebuffer {
        self.msaa.map_or(self.resolve, |(fbo, _, _)| fbo)
    }

    /// Lê a cor resolvida (RGBA8, linhas de baixo para cima). **Só para testes de
    /// pixel e capturas sob demanda** — nunca no hot path (sem readback por frame).
    ///
    /// # Safety
    /// O contexto GL que criou o alvo precisa estar corrente.
    pub unsafe fn read_rgba(&self, gl: &glow::Context) -> Vec<u8> {
        let mut pixels = vec![0u8; self.width as usize * self.height as usize * 4];
        // SAFETY: delegated to the caller.
        unsafe {
            let saved = GlStateGuard::save(gl);
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(self.resolve));
            gl.pixel_store_i32(glow::PACK_ALIGNMENT, 1);
            gl.read_pixels(
                0,
                0,
                self.width as i32,
                self.height as i32,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelPackData::Slice(Some(&mut pixels)),
            );
            saved.restore(gl);
        }
        pixels
    }

    /// Resolve o MSAA na textura de cor (no-op sem MSAA).
    pub(crate) unsafe fn resolve(&self, gl: &glow::Context) {
        let Some((msaa, _, _)) = self.msaa else {
            return;
        };
        let (w, h) = (self.width as i32, self.height as i32);
        // SAFETY: caller keeps the context current on this thread.
        unsafe {
            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(msaa));
            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, Some(self.resolve));
            gl.blit_framebuffer(
                0,
                0,
                w,
                h,
                0,
                0,
                w,
                h,
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(msaa));
        }
    }

    /// # Safety
    /// O contexto GL que criou o alvo precisa estar corrente.
    pub unsafe fn delete(self, gl: &glow::Context) {
        // SAFETY: delegated to the caller.
        unsafe {
            gl.delete_framebuffer(self.resolve);
            gl.delete_texture(self.color);
            if let Some(depth) = self.depth {
                gl.delete_renderbuffer(depth);
            }
            if let Some((fbo, color, depth)) = self.msaa {
                gl.delete_framebuffer(fbo);
                gl.delete_renderbuffer(color);
                gl.delete_renderbuffer(depth);
            }
        }
    }
}

/// FBO com uma única textura (cor ou profundidade) para passes auxiliares.
pub(super) struct TextureTarget {
    pub(super) framebuffer: NativeFramebuffer,
    pub(super) texture: NativeTexture,
    pub(super) width: u32,
    pub(super) height: u32,
}

/// Formato do alvo auxiliar.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum AuxFormat {
    /// Máscara do contorno (RGB = cor do contorno, A = coberto).
    Rgba8,
    /// Oclusão ambiente.
    R8,
    /// Profundidade amostrável do pré-passe de AO.
    Depth,
}

impl TextureTarget {
    pub(super) unsafe fn new(
        gl: &glow::Context,
        width: u32,
        height: u32,
        format: AuxFormat,
    ) -> Result<Self, String> {
        let (width, height) = (width.max(1), height.max(1));
        // SAFETY: caller keeps the context current on this thread.
        unsafe {
            let (internal, layout, kind, filter, attachment) = match format {
                AuxFormat::Rgba8 => (
                    glow::RGBA8,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    glow::NEAREST,
                    glow::COLOR_ATTACHMENT0,
                ),
                AuxFormat::R8 => (
                    glow::R8,
                    glow::RED,
                    glow::UNSIGNED_BYTE,
                    glow::LINEAR,
                    glow::COLOR_ATTACHMENT0,
                ),
                AuxFormat::Depth => (
                    glow::DEPTH_COMPONENT24,
                    glow::DEPTH_COMPONENT,
                    glow::UNSIGNED_INT,
                    glow::NEAREST,
                    glow::DEPTH_ATTACHMENT,
                ),
            };
            let texture = texture_2d(gl, width, height, internal, layout, kind, filter)?;
            let framebuffer = match gl.create_framebuffer() {
                Ok(framebuffer) => framebuffer,
                Err(error) => {
                    gl.delete_texture(texture);
                    return Err(format!("OpenGL auxiliary framebuffer: {error}"));
                }
            };
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                attachment,
                glow::TEXTURE_2D,
                Some(texture),
                0,
            );
            if format == AuxFormat::Depth {
                gl.draw_buffer(glow::NONE);
                gl.read_buffer(glow::NONE);
            }
            let target = Self {
                framebuffer,
                texture,
                width,
                height,
            };
            if let Err(error) = complete(gl, "auxiliary") {
                target.delete(gl);
                return Err(error);
            }
            Ok(target)
        }
    }

    pub(super) unsafe fn delete(&self, gl: &glow::Context) {
        // SAFETY: caller keeps the context that created the target current.
        unsafe {
            gl.delete_framebuffer(self.framebuffer);
            gl.delete_texture(self.texture);
        }
    }
}

/// Recria o alvo quando o tamanho muda; `None` se a GPU recusar.
pub(super) unsafe fn ensure_target<'a>(
    gl: &glow::Context,
    slot: &'a mut Option<TextureTarget>,
    width: u32,
    height: u32,
    format: AuxFormat,
) -> Option<&'a TextureTarget> {
    let (width, height) = (width.max(1), height.max(1));
    // SAFETY: caller keeps the context current on this thread.
    unsafe {
        if slot
            .as_ref()
            .is_some_and(|target| target.width != width || target.height != height)
            && let Some(old) = slot.take()
        {
            old.delete(gl);
        }
        if slot.is_none() {
            match TextureTarget::new(gl, width, height, format) {
                Ok(target) => *slot = Some(target),
                Err(error) => {
                    eprintln!("petunia3d: {error}");
                    return None;
                }
            }
        }
    }
    slot.as_ref()
}

/// Estado GL que o desenho do viewport altera e que o host (FemtoVG) espera
/// intacto. Só o compartilhado é restaurado — sem reset global.
pub(crate) struct GlStateGuard {
    draw_framebuffer: i32,
    read_framebuffer: i32,
    renderbuffer: i32,
    vertex_array: i32,
    program: i32,
    array_buffer: i32,
    active_texture: i32,
    texture_units: [i32; 2],
    viewport: [i32; 4],
    scissor_box: [i32; 4],
    capabilities: [(u32, bool); 5],
    depth_mask: bool,
    depth_func: i32,
    color_mask: [bool; 4],
    blend: [i32; 6],
    unpack: [(u32, i32); 4],
}

const SAVED_CAPABILITIES: [u32; 5] = [
    glow::SCISSOR_TEST,
    glow::DEPTH_TEST,
    glow::BLEND,
    glow::CULL_FACE,
    glow::STENCIL_TEST,
];

const SAVED_UNPACK: [u32; 4] = [
    glow::UNPACK_ALIGNMENT,
    glow::UNPACK_ROW_LENGTH,
    glow::UNPACK_SKIP_PIXELS,
    glow::UNPACK_SKIP_ROWS,
];

impl GlStateGuard {
    pub(crate) unsafe fn save(gl: &glow::Context) -> Self {
        // SAFETY: the context is current on the calling thread (caller contract).
        unsafe {
            let mut viewport = [0i32; 4];
            gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
            let mut scissor_box = [0i32; 4];
            gl.get_parameter_i32_slice(glow::SCISSOR_BOX, &mut scissor_box);
            let mut depth_mask = [0i32; 1];
            gl.get_parameter_i32_slice(glow::DEPTH_WRITEMASK, &mut depth_mask);
            let mut color_mask = [0i32; 4];
            gl.get_parameter_i32_slice(glow::COLOR_WRITEMASK, &mut color_mask);
            let active_texture = gl.get_parameter_i32(glow::ACTIVE_TEXTURE);
            let mut texture_units = [0i32; 2];
            for (unit, slot) in texture_units.iter_mut().enumerate() {
                gl.active_texture(glow::TEXTURE0 + unit as u32);
                *slot = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D);
            }
            gl.active_texture(active_texture as u32);
            Self {
                draw_framebuffer: gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING),
                read_framebuffer: gl.get_parameter_i32(glow::READ_FRAMEBUFFER_BINDING),
                renderbuffer: gl.get_parameter_i32(glow::RENDERBUFFER_BINDING),
                vertex_array: gl.get_parameter_i32(glow::VERTEX_ARRAY_BINDING),
                program: gl.get_parameter_i32(glow::CURRENT_PROGRAM),
                array_buffer: gl.get_parameter_i32(glow::ARRAY_BUFFER_BINDING),
                active_texture,
                texture_units,
                viewport,
                scissor_box,
                capabilities: SAVED_CAPABILITIES.map(|cap| (cap, gl.is_enabled(cap))),
                depth_mask: depth_mask[0] != 0,
                depth_func: gl.get_parameter_i32(glow::DEPTH_FUNC),
                color_mask: color_mask.map(|channel| channel != 0),
                blend: [
                    glow::BLEND_SRC_RGB,
                    glow::BLEND_DST_RGB,
                    glow::BLEND_SRC_ALPHA,
                    glow::BLEND_DST_ALPHA,
                    glow::BLEND_EQUATION_RGB,
                    glow::BLEND_EQUATION_ALPHA,
                ]
                .map(|parameter| gl.get_parameter_i32(parameter)),
                unpack: SAVED_UNPACK.map(|parameter| (parameter, gl.get_parameter_i32(parameter))),
            }
        }
    }

    pub(crate) unsafe fn restore(&self, gl: &glow::Context) {
        let non_zero = |id: i32| NonZeroU32::new(id as u32);
        // SAFETY: the context is current on the calling thread (caller contract).
        unsafe {
            gl.bind_framebuffer(
                glow::DRAW_FRAMEBUFFER,
                non_zero(self.draw_framebuffer).map(NativeFramebuffer),
            );
            gl.bind_framebuffer(
                glow::READ_FRAMEBUFFER,
                non_zero(self.read_framebuffer).map(NativeFramebuffer),
            );
            gl.bind_renderbuffer(
                glow::RENDERBUFFER,
                non_zero(self.renderbuffer).map(NativeRenderbuffer),
            );
            gl.bind_vertex_array(non_zero(self.vertex_array).map(glow::NativeVertexArray));
            gl.use_program(non_zero(self.program).map(glow::NativeProgram));
            gl.bind_buffer(
                glow::ARRAY_BUFFER,
                non_zero(self.array_buffer).map(glow::NativeBuffer),
            );
            for (unit, texture) in self.texture_units.iter().enumerate() {
                gl.active_texture(glow::TEXTURE0 + unit as u32);
                gl.bind_texture(glow::TEXTURE_2D, non_zero(*texture).map(NativeTexture));
            }
            gl.active_texture(self.active_texture as u32);
            gl.viewport(
                self.viewport[0],
                self.viewport[1],
                self.viewport[2],
                self.viewport[3],
            );
            gl.scissor(
                self.scissor_box[0],
                self.scissor_box[1],
                self.scissor_box[2],
                self.scissor_box[3],
            );
            for (capability, enabled) in self.capabilities {
                if enabled {
                    gl.enable(capability);
                } else {
                    gl.disable(capability);
                }
            }
            gl.depth_mask(self.depth_mask);
            gl.depth_func(self.depth_func as u32);
            gl.color_mask(
                self.color_mask[0],
                self.color_mask[1],
                self.color_mask[2],
                self.color_mask[3],
            );
            gl.blend_func_separate(
                self.blend[0] as u32,
                self.blend[1] as u32,
                self.blend[2] as u32,
                self.blend[3] as u32,
            );
            gl.blend_equation_separate(self.blend[4] as u32, self.blend[5] as u32);
            for (parameter, value) in self.unpack {
                gl.pixel_store_i32(parameter, value);
            }
        }
    }
}
