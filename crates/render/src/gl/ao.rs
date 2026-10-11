//! Oclusão ambiente opcional (Solid/Material, sem X-Ray): pré-passe de
//! profundidade em resolução cheia, GTAO em meia resolução e blur que respeita
//! a profundidade. Sem compute shaders; o tier de performance só liga/desliga.

use glow::{HasContext, NativeTexture};

use super::GlRenderer;
use super::buffers::no_attributes;
use super::target::{AuxFormat, ensure_target};
use crate::frame::RenderFrameInput;

/// Intensidade da oclusão (expoente sobre a visibilidade).
const AO_INTENSITY: f32 = 1.5;
/// Raio máximo da busca de horizontes em px lógicos.
const AO_MAX_RADIUS_PX: f32 = 48.0;

impl GlRenderer {
    /// Calcula a AO do frame; `None` quando não se aplica ou a GPU recusa.
    pub(super) unsafe fn ambient_occlusion(
        &mut self,
        frame: &RenderFrameInput<'_>,
    ) -> Option<NativeTexture> {
        puffin::profile_function!();
        if !frame.settings.ambient_occlusion_active()
            || frame
                .objects
                .iter()
                .all(|object| object.mesh.indices.is_empty())
        {
            return None;
        }
        let gl = std::sync::Arc::clone(&self.gl);
        let (width, height) = (frame.viewport.width.max(1), frame.viewport.height.max(1));
        let half = ((width / 2).max(1), (height / 2).max(1));
        ensure_target(&gl, &mut self.ao_depth, width, height, AuxFormat::Depth)?;
        ensure_target(&gl, &mut self.ao_raw, half.0, half.1, AuxFormat::R8)?;
        ensure_target(&gl, &mut self.ao_final, half.0, half.1, AuxFormat::R8)?;
        let (depth, raw, finished) = (
            self.ao_depth.as_ref()?,
            self.ao_raw.as_ref()?,
            self.ao_final.as_ref()?,
        );

        // Pré-passe de profundidade.
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(depth.framebuffer));
        gl.viewport(0, 0, width as i32, height as i32);
        gl.disable(glow::SCISSOR_TEST);
        gl.disable(glow::BLEND);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
        gl.clear(glow::DEPTH_BUFFER_BIT);
        let program = &self.programs.depth;
        program.bind(&gl);
        program.mat4(&gl, "uViewProj", &frame.camera.view_proj);
        self.draw_positions(frame, program, |_| true);

        let camera = &frame.camera;
        let radius = (camera.visible_height * 0.06).clamp(0.02, 2.0);
        let params = [
            camera.proj[5],
            if camera.orthographic { 1.0 } else { 0.0 },
            radius,
            AO_INTENSITY,
        ];
        let size = [width as f32, height as f32, half.0 as f32, half.1 as f32];
        let limits = [AO_MAX_RADIUS_PX * frame.settings.pixel_ratio, 0.0, 0.0, 0.0];
        gl.disable(glow::DEPTH_TEST);
        gl.depth_mask(false);
        no_attributes(&gl);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, Some(depth.texture));
        for (program, output, raw_input) in [
            (&self.programs.ao, raw, None),
            (&self.programs.ao_blur, finished, Some(raw.texture)),
        ] {
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(output.framebuffer));
            gl.viewport(0, 0, output.width as i32, output.height as i32);
            program.bind(&gl);
            program.i32(&gl, "uDepth", 0);
            program.mat4(&gl, "uInvProj", &camera.inv_proj);
            program.vec4(&gl, "uParams", params);
            program.vec4(&gl, "uSize", size);
            program.vec4(&gl, "uLimits", limits);
            if let Some(texture) = raw_input {
                program.i32(&gl, "uAoRaw", 1);
                gl.active_texture(glow::TEXTURE1);
                gl.bind_texture(glow::TEXTURE_2D, Some(texture));
                gl.active_texture(glow::TEXTURE0);
            }
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
        gl.active_texture(glow::TEXTURE1);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.active_texture(glow::TEXTURE0);
        gl.bind_texture(glow::TEXTURE_2D, None);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_mask(true);
        Some(finished.texture)
    }
}
