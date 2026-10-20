//! Grid global e eixos do mundo (linhas de 1 px, reenviadas só quando os
//! parâmetros mudam). O workplane é um overlay diferente (ver `components`).

use glow::HasContext;

use super::GlRenderer;
use super::buffers::float_attributes;
use crate::frame::{RenderFrameInput, ViewportRenderSettings};
use crate::scene;

/// Pontos (posição + cor) das linhas do grid e dos eixos.
pub(super) fn grid_data(settings: &ViewportRenderSettings) -> Vec<f32> {
    let mut data = Vec::new();
    let mut push = |a: [f32; 3], b: [f32; 3], color: [f32; 3]| {
        data.extend_from_slice(&a);
        data.extend_from_slice(&color);
        data.extend_from_slice(&b);
        data.extend_from_slice(&color);
    };
    if settings.show_overlays && settings.show_grid {
        let grid = &settings.grid;
        for (a, b, color) in scene::grid_plane_lines(
            grid.size,
            grid.spacing,
            grid.opacity,
            grid.show_isometric_guide,
            grid.isometric_angle_deg,
        ) {
            push(a, b, color);
        }
    }
    // Eixos depois do plano, com depth test (ver `draw_grid`): não atravessam
    // a frente dos objetos. Cada eixo liga/desliga sozinho; o Y nasce oculto.
    if settings.show_overlays && settings.show_axes {
        for (a, b, color) in scene::grid_axis_lines(settings.grid.size, settings.axes_visible) {
            push(a, b, color);
        }
    }
    data
}

impl GlRenderer {
    /// Passe 8 (grid/eixos): depth test sem escrita.
    pub(super) unsafe fn draw_grid(&mut self, frame: &RenderFrameInput<'_>) {
        puffin::profile_function!();
        let gl = std::sync::Arc::clone(&self.gl);
        if self.grid.is_current(frame.overlay_revision) {
            self.stats.skipped_uploads += 1;
        } else {
            let data = grid_data(&frame.settings);
            self.grid.upload(
                &gl,
                glow::ARRAY_BUFFER,
                &data,
                (data.len() / 6) as i32,
                Some(frame.overlay_revision),
                &mut self.stats.buffer_creations,
            );
        }
        if self.grid.count == 0 || !self.grid.bind(&gl, glow::ARRAY_BUFFER) {
            return;
        }
        let program = &self.programs.grid;
        program.bind(&gl);
        program.mat4(&gl, "uViewProj", &frame.camera.view_proj);
        float_attributes(&gl, &[3, 3]);
        gl.disable(glow::BLEND);
        gl.enable(glow::DEPTH_TEST);
        gl.depth_func(glow::LEQUAL);
        gl.depth_mask(false);
        gl.draw_arrays(glow::LINES, 0, self.grid.count);
        gl.depth_func(glow::LESS);
        gl.depth_mask(true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_and_axes_only_with_overlays_enabled() {
        let mut settings = ViewportRenderSettings {
            show_overlays: false,
            ..ViewportRenderSettings::default()
        };
        assert!(grid_data(&settings).is_empty());
        settings.show_overlays = true;
        settings.show_grid = false;
        let axes = grid_data(&settings).len();
        assert_eq!(axes, 2 * 12, "X e Z; o Y nasce oculto");
        settings.axes_visible = [true, true, true];
        assert_eq!(
            grid_data(&settings).len(),
            3 * 12,
            "três eixos quando pedidos"
        );
        settings.axes_visible = [false, false, false];
        assert!(grid_data(&settings).is_empty(), "nenhum eixo pedido");
        settings.show_grid = true;
        assert!(!grid_data(&settings).is_empty());
    }

    #[test]
    fn hostile_spacing_never_explodes_the_line_count() {
        let settings = ViewportRenderSettings {
            grid: crate::frame::RenderGrid {
                size: 1000.0,
                spacing: 1e-9,
                opacity: f32::NAN,
                show_isometric_guide: false,
                isometric_angle_deg: 30.0,
            },
            ..ViewportRenderSettings::default()
        };
        let floats = grid_data(&settings).len();
        // 2 linhas por passo, (2*2000+1) passos, 12 floats por linha, mais os eixos.
        assert!(floats <= (2 * 4001 + 3) * 12, "linhas demais: {floats}");
    }
}
