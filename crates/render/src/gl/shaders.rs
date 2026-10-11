//! Montagem das fontes GLSL 3.30 core. Os shaders vivem em `shaders/*.vert|frag`;
//! este módulo só acrescenta `#version` e os prefixos comuns.

const COMMON: &str = include_str!("shaders/common.glsl");
const AO_COMMON: &str = include_str!("shaders/ao_common.glsl");

/// Vertex shader com o remapeamento de clip (`to_gl_clip`).
pub(super) fn vertex(body: &str) -> String {
    format!("#version 330 core\n{COMMON}\n{body}")
}

pub(super) fn fragment(body: &str) -> String {
    format!("#version 330 core\n{body}")
}

/// Fragment dos passes de AO (depth, inversa da projeção e tamanhos).
pub(super) fn ao_fragment(body: &str) -> String {
    format!("#version 330 core\n{AO_COMMON}\n{body}")
}

pub(super) const SURFACE_VS: &str = include_str!("shaders/surface.vert");
pub(super) const SURFACE_FS: &str = include_str!("shaders/surface.frag");
pub(super) const POSITION_VS: &str = include_str!("shaders/position.vert");
pub(super) const MASK_FS: &str = include_str!("shaders/mask.frag");
pub(super) const EMPTY_FS: &str = include_str!("shaders/empty.frag");
pub(super) const WIDE_LINE_VS: &str = include_str!("shaders/wide_line.vert");
pub(super) const COLOR_FS: &str = include_str!("shaders/color.frag");
pub(super) const POINT_VS: &str = include_str!("shaders/point.vert");
pub(super) const POINT_FS: &str = include_str!("shaders/point.frag");
pub(super) const FLAT_VS: &str = include_str!("shaders/flat.vert");
pub(super) const GRID_VS: &str = include_str!("shaders/grid.vert");
pub(super) const SCREEN_VS: &str = include_str!("shaders/screen.vert");
pub(super) const REFERENCE_VS: &str = include_str!("shaders/reference.vert");
pub(super) const REFERENCE_FS: &str = include_str!("shaders/reference.frag");
pub(super) const FULLSCREEN_VS: &str = include_str!("shaders/fullscreen.vert");
pub(super) const OUTLINE_FS: &str = include_str!("shaders/outline.frag");
pub(super) const AO_FS: &str = include_str!("shaders/ao.frag");
pub(super) const AO_BLUR_FS: &str = include_str!("shaders/ao_blur.frag");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vertex_shader_gets_the_clip_remap() {
        for body in [
            SURFACE_VS,
            POSITION_VS,
            WIDE_LINE_VS,
            POINT_VS,
            FLAT_VS,
            GRID_VS,
            REFERENCE_VS,
        ] {
            let source = vertex(body);
            assert!(source.starts_with("#version 330 core\n"));
            assert!(source.contains("vec4 to_gl_clip"));
            assert!(body.contains("to_gl_clip("), "{body}");
        }
    }

    #[test]
    fn ao_passes_share_the_depth_helpers() {
        for body in [AO_FS, AO_BLUR_FS] {
            let source = ao_fragment(body);
            assert!(source.contains("float depth_at"));
            assert!(source.contains("vec3 view_pos"));
        }
    }
}
