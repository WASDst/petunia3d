//! Contorno de objetos em largura constante, com a cor de cada parte: máscara
//! colorida num FBO próprio → dilatação em tela cheia → composição.
//! (Não usa mesh escalada: falha em côncavos e escala não uniforme.)

use glow::HasContext;

use super::GlRenderer;
use super::buffers::no_attributes;
use super::target::{AuxFormat, ensure_target};
use crate::frame::RenderFrameInput;

impl GlRenderer {
    /// Grava a máscara (RGB = cor do contorno, A = coberto). `false` sem entradas.
    pub(super) unsafe fn draw_outline_mask(&mut self, frame: &RenderFrameInput<'_>) -> bool {
        let outline = frame.outline;
        if !outline.entries.iter().any(|entry| {
            frame
                .objects
                .iter()
                .any(|object| object.object_id == entry.object_id)
        }) {
            return false;
        }
        let gl = std::sync::Arc::clone(&self.gl);
        let (width, height) = (frame.viewport.width, frame.viewport.height);
        let Some(mask) =
            ensure_target(&gl, &mut self.outline_mask, width, height, AuxFormat::Rgba8)
        else {
            return false;
        };
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(mask.framebuffer));
        gl.viewport(0, 0, mask.width as i32, mask.height as i32);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.disable(glow::DEPTH_TEST);
        gl.clear_color(0.0, 0.0, 0.0, 0.0);
        gl.clear(glow::COLOR_BUFFER_BIT);
        let program = &self.programs.mask;
        program.bind(&gl);
        program.mat4(&gl, "uViewProj", &frame.camera.view_proj);
        let mut drew = false;
        // Ordem das entradas: a última vence onde as silhuetas se sobrepõem.
        for entry in &outline.entries {
            program.vec4(&gl, "uColor", entry.color);
            drew |=
                self.draw_positions(frame, program, |object| object.object_id == entry.object_id);
        }
        gl.enable(glow::DEPTH_TEST);
        drew
    }

    /// Passe 9: compõe o anel ao redor da máscara, por cima de tudo.
    pub(super) unsafe fn draw_outline(&mut self, frame: &RenderFrameInput<'_>) {
        let Some(mask) = self.outline_mask.as_ref() else {
            return;
        };
        let gl = &self.gl;
        let outline = frame.outline;
        let program = &self.programs.outline;
        program.bind(gl);
        program.i32(gl, "uMask", 0);
        program.f32(
            gl,
            "uRadius",
            outline.radius_px.clamp(1.0, 4.0) * frame.settings.pixel_ratio,
        );
        program.vec2(
            gl,
            "uOrigin",
            [
                frame.viewport.x as f32,
                frame.viewport.gl_y(frame.framebuffer_height) as f32,
            ],
        );
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(mask.texture));
        no_attributes(gl);
        gl.disable(glow::DEPTH_TEST);
        gl.depth_mask(false);
        gl.enable(glow::BLEND);
        super::buffers::alpha_blend(gl);
        gl.draw_arrays(glow::TRIANGLES, 0, 3);
        gl.disable(glow::BLEND);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_mask(true);
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
}
