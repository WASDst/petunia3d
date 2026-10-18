//! Renderer OpenGL 3.3 Core (glow) do Petunia3D — o único backend do viewport.
//!
//! Consome somente [`RenderFrameInput`](crate::frame::RenderFrameInput): não
//! conhece `AppState`, `Project`, seleção nem preferências (decisão em
//! `website/docs/04-renderer/architecture-decision.md`). O host possui janela,
//! contexto e present; este módulo possui recursos de GPU, FBOs e passes.
//!
//! Passes, em ordem (cada um declara o próprio estado GL):
//!
//! 1. clear / depth (FBO multisample próprio quando há MSAA)
//! 2. referências atrás da geometria
//! 3. pré-passe de profundidade + GTAO + blur (AO opcional, fora do FBO principal)
//! 4. superfícies opacas
//! 5. superfícies X-Ray (translúcidas)
//! 6. arestas de topologia/feição (largura constante em px)
//! 7. overlays de seleção/preselection de componentes
//! 8. workplane, grid e eixos
//! 9. contorno de seleção de objetos (máscara + dilatação)
//! 10. referências X-Ray/na frente
//! 11. overlay de tela (gizmo) sem depth test
//!
//! Cache de GPU por objeto e por revisão: câmera e seleção nunca reenviam a
//! superfície; texturas sobem por revisão e, com regiões sujas, parcialmente.

#![allow(unsafe_op_in_unsafe_fn)]

mod ao;
mod buffers;
mod components;
mod diagnostics;
mod edges;
mod grid;
mod outline;
mod program;
mod references;
mod screen;
mod shaders;
mod surfaces;
mod target;
mod textures;

pub use diagnostics::detect_caps;
pub use target::GlTarget;

use std::collections::HashMap;
use std::sync::Arc;

use glow::{HasContext, NativeTexture, NativeVertexArray};
use uuid::Uuid;

use crate::GpuCaps;
use crate::frame::RenderFrameInput;
use buffers::StreamBuffer;
use program::Program;
use target::TextureTarget;

/// Buffers de GPU de um objeto, cada um com a revisão que o gerou.
#[derive(Default)]
struct ObjectSlot {
    vertices: StreamBuffer,
    indices: StreamBuffer,
    edges: StreamBuffer,
}

/// Textura GL com a revisão do conteúdo enviado.
struct TextureSlot {
    width: u32,
    height: u32,
    revision: u64,
    texture: NativeTexture,
}

/// Programas de cada passe.
struct Programs {
    surface: Program,
    mask: Program,
    depth: Program,
    wide_line: Program,
    point: Program,
    flat: Program,
    grid: Program,
    screen: Program,
    reference: Program,
    outline: Program,
    ao: Program,
    ao_blur: Program,
}

impl Programs {
    unsafe fn compile(gl: &glow::Context) -> Result<Self, String> {
        use shaders::*;
        let mut built: Vec<glow::NativeProgram> = Vec::new();
        let mut make = |label: &str, vs: String, fs: String, uniforms: &[&'static str]| {
            Program::new(gl, label, &vs, &fs, uniforms).inspect(|program| built.push(program.id))
        };
        let result = (|| {
            Ok(Self {
                surface: make(
                    "surface",
                    vertex(SURFACE_VS),
                    fragment(SURFACE_FS),
                    &[
                        "uViewProj",
                        "uModel",
                        "uNormalMatrix",
                        "uOpacity",
                        "uLightDir",
                        "uAmbient",
                        "uDiffuse",
                        "uMatcap",
                        "uViewRight",
                        "uViewUp",
                        "uTextured",
                        "uTexture",
                        "uAmbientOcclusion",
                        "uAoTexture",
                        "uViewportRect",
                        "uFaceOrientation",
                        "uUvChecker",
                    ],
                )?,
                mask: make(
                    "outline mask",
                    vertex(POSITION_VS),
                    fragment(MASK_FS),
                    &["uViewProj", "uModel", "uColor"],
                )?,
                depth: make(
                    "depth prepass",
                    vertex(POSITION_VS),
                    fragment(EMPTY_FS),
                    &["uViewProj", "uModel"],
                )?,
                wide_line: make(
                    "wide lines",
                    vertex(WIDE_LINE_VS),
                    fragment(COLOR_FS),
                    &[
                        "uViewProj",
                        "uModel",
                        "uViewportSize",
                        "uWidthScale",
                        "uDepthBias",
                    ],
                )?,
                point: make(
                    "points",
                    vertex(POINT_VS),
                    fragment(POINT_FS),
                    &["uViewProj", "uViewportSize", "uPxScale"],
                )?,
                flat: make(
                    "overlay faces",
                    vertex(FLAT_VS),
                    fragment(COLOR_FS),
                    &["uViewProj"],
                )?,
                grid: make("grid", vertex(GRID_VS), fragment(COLOR_FS), &["uViewProj"])?,
                screen: make(
                    "screen overlay",
                    vertex(SCREEN_VS),
                    fragment(COLOR_FS),
                    &["uViewportSize"],
                )?,
                reference: make(
                    "reference images",
                    vertex(REFERENCE_VS),
                    fragment(REFERENCE_FS),
                    &["uViewProj", "uTexture", "uOpacity"],
                )?,
                outline: make(
                    "selection outline",
                    vertex(FULLSCREEN_VS),
                    fragment(OUTLINE_FS),
                    &["uMask", "uRadius", "uOrigin"],
                )?,
                ao: make(
                    "ambient occlusion",
                    vertex(FULLSCREEN_VS),
                    ao_fragment(AO_FS),
                    &["uDepth", "uInvProj", "uParams", "uSize", "uLimits"],
                )?,
                ao_blur: make(
                    "ambient occlusion blur",
                    vertex(FULLSCREEN_VS),
                    ao_fragment(AO_BLUR_FS),
                    &[
                        "uDepth", "uAoRaw", "uInvProj", "uParams", "uSize", "uLimits",
                    ],
                )?,
            })
        })();
        if result.is_err() {
            for program in built {
                gl.delete_program(program);
            }
        }
        result
    }

    unsafe fn delete(&self, gl: &glow::Context) {
        for program in [
            &self.surface,
            &self.mask,
            &self.depth,
            &self.wide_line,
            &self.point,
            &self.flat,
            &self.grid,
            &self.screen,
            &self.reference,
            &self.outline,
            &self.ao,
            &self.ao_blur,
        ] {
            program.delete(gl);
        }
    }
}

/// Contadores de diagnóstico (testes, benchmarks e overlay de performance).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RendererStats {
    pub buffer_creations: u64,
    /// Reenvios evitados porque a revisão não mudou.
    pub skipped_uploads: u64,
    pub surface_uploads: u64,
    pub edge_uploads: u64,
    pub component_uploads: u64,
    pub full_texture_uploads: u64,
    pub partial_texture_uploads: u64,
    pub texture_bytes_uploaded: u64,
    pub frames: u64,
    /// Tempo de CPU do último `GlStateGuard::save` (dezenas de `glGet*`).
    pub last_state_save_nanos: u64,
    /// Tempo de CPU do último `GlStateGuard::restore`.
    pub last_state_restore_nanos: u64,
}

/// O que o último frame realmente aplicou (testes e diagnóstico).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrameReport {
    pub ambient_occlusion: bool,
    pub outline: bool,
    pub screen_overlay_vertices: u32,
    pub samples: u32,
}

pub struct GlRenderer {
    gl: Arc<glow::Context>,
    /// VAO único ligado por frame (core profile exige VAO).
    vao: NativeVertexArray,
    programs: Programs,
    caps: GpuCaps,
    objects: HashMap<Uuid, ObjectSlot>,
    object_textures: HashMap<Uuid, TextureSlot>,
    reference_textures: Vec<TextureSlot>,
    /// Textura 1×1 branca ligada quando o passe não amostra nada.
    white: NativeTexture,
    grid: StreamBuffer,
    component_faces: StreamBuffer,
    component_lines: StreamBuffer,
    component_points: StreamBuffer,
    workplane_faces: StreamBuffer,
    workplane_lines: StreamBuffer,
    screen: StreamBuffer,
    references: StreamBuffer,
    outline_mask: Option<TextureTarget>,
    ao_depth: Option<TextureTarget>,
    ao_raw: Option<TextureTarget>,
    ao_final: Option<TextureTarget>,
    pub stats: RendererStats,
    report: FrameReport,
}

impl GlRenderer {
    pub fn new(gl: Arc<glow::Context>) -> Result<Self, String> {
        // SAFETY: the caller supplies the current context on the rendering thread.
        unsafe {
            let caps = detect_caps(&gl);
            if !caps.supports_core_33() {
                return Err(format!(
                    "OpenGL 3.3 Core é necessário (encontrado: {} / GLSL {})",
                    caps.gl_version, caps.glsl_version
                ));
            }
            let programs = Programs::compile(&gl)?;
            let vao = match gl.create_vertex_array() {
                Ok(vao) => vao,
                Err(error) => {
                    programs.delete(&gl);
                    return Err(format!("OpenGL vertex array: {error}"));
                }
            };
            let white = match textures::solid_texture(&gl, [255, 255, 255, 255]) {
                Ok(texture) => texture,
                Err(error) => {
                    programs.delete(&gl);
                    gl.delete_vertex_array(vao);
                    return Err(error);
                }
            };
            Ok(Self {
                gl: Arc::clone(&gl),
                vao,
                programs,
                caps,
                objects: HashMap::new(),
                object_textures: HashMap::new(),
                reference_textures: Vec::new(),
                white,
                grid: StreamBuffer::default(),
                component_faces: StreamBuffer::default(),
                component_lines: StreamBuffer::default(),
                component_points: StreamBuffer::default(),
                workplane_faces: StreamBuffer::default(),
                workplane_lines: StreamBuffer::default(),
                screen: StreamBuffer::default(),
                references: StreamBuffer::default(),
                outline_mask: None,
                ao_depth: None,
                ao_raw: None,
                ao_final: None,
                stats: RendererStats::default(),
                report: FrameReport::default(),
            })
        }
    }

    /// Capacidades detectadas na criação (versão, vendor, limites, MSAA).
    pub fn caps(&self) -> &GpuCaps {
        &self.caps
    }

    /// O que o último frame aplicou.
    pub fn last_frame(&self) -> FrameReport {
        self.report
    }

    /// Desenha o frame no alvo fora da tela e devolve o contexto ao estado do host
    /// (FBO, VAO, programa, buffers, texturas, viewport, blend e capacidades).
    /// Sem readback.
    pub fn draw_into(&mut self, target: &GlTarget, frame: &RenderFrameInput<'_>) {
        puffin::profile_function!();
        let gl = Arc::clone(&self.gl);
        // SAFETY: the host keeps the GL context current on this thread during draw.
        unsafe {
            let started = std::time::Instant::now();
            let saved = target::GlStateGuard::save(&gl);
            self.stats.last_state_save_nanos = started.elapsed().as_nanos() as u64;
            gl.bind_vertex_array(Some(self.vao));
            gl.disable(glow::STENCIL_TEST);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::SCISSOR_TEST);
            gl.color_mask(true, true, true, true);
            gl.pixel_store_i32(glow::UNPACK_ALIGNMENT, 4);
            self.report = FrameReport {
                samples: target.samples(),
                ..FrameReport::default()
            };

            // Recursos: texturas e buffers de objeto por revisão.
            self.sync_object_textures(frame);
            self.sync_reference_textures(frame);
            self.sync_objects(frame);

            // Passes fora do FBO principal.
            let outline = self.draw_outline_mask(frame);
            let ao = self.ambient_occlusion(frame);
            self.report.ambient_occlusion = ao.is_some();

            // 1. clear.
            let viewport = frame.viewport;
            let gl_y = viewport.gl_y(frame.framebuffer_height) as i32;
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(target.draw_framebuffer()));
            gl.viewport(
                viewport.x as i32,
                gl_y,
                viewport.width as i32,
                viewport.height as i32,
            );
            gl.scissor(
                viewport.x as i32,
                gl_y,
                viewport.width as i32,
                viewport.height as i32,
            );
            gl.enable(glow::SCISSOR_TEST);
            let bg = frame.palette.background;
            gl.clear_color(bg[0], bg[1], bg[2], 1.0);
            gl.depth_mask(true);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            // 2. referências atrás.
            self.draw_references(frame, false);
            // 4–5. superfícies (opacas ou X-Ray).
            self.draw_surfaces(frame, ao);
            // 6. arestas.
            self.draw_edges(frame);
            // 7. seleção de componentes.
            self.draw_components(frame);
            // 8. workplane, grid e eixos.
            self.draw_workplane(frame);
            self.draw_grid(frame);
            // 9. contorno de objetos.
            if outline {
                self.draw_outline(frame);
                self.report.outline = true;
            }
            // 10. referências X-Ray.
            self.draw_references(frame, true);
            // 11. gizmo/overlay de tela.
            self.draw_screen_overlay(frame);

            target.resolve(&gl);
            diagnostics::check_errors(&gl, "frame");
            self.stats.frames += 1;
            let started = std::time::Instant::now();
            saved.restore(&gl);
            self.stats.last_state_restore_nanos = started.elapsed().as_nanos() as u64;
        }
    }

    /// Libera todos os recursos de GPU. Chamar com o contexto corrente.
    ///
    /// # Safety
    /// O contexto GL que criou o renderer precisa estar corrente.
    pub unsafe fn destroy(mut self) {
        let gl = Arc::clone(&self.gl);
        // SAFETY: delegated to the caller.
        unsafe {
            for (_, mut slot) in self.objects.drain() {
                slot.vertices.delete(&gl);
                slot.indices.delete(&gl);
                slot.edges.delete(&gl);
            }
            for (_, slot) in self.object_textures.drain() {
                gl.delete_texture(slot.texture);
            }
            for slot in self.reference_textures.drain(..) {
                gl.delete_texture(slot.texture);
            }
            gl.delete_texture(self.white);
            for buffer in [
                &mut self.grid,
                &mut self.component_faces,
                &mut self.component_lines,
                &mut self.component_points,
                &mut self.workplane_faces,
                &mut self.workplane_lines,
                &mut self.screen,
                &mut self.references,
            ] {
                buffer.delete(&gl);
            }
            for target in [
                self.outline_mask.take(),
                self.ao_depth.take(),
                self.ao_raw.take(),
                self.ao_final.take(),
            ]
            .into_iter()
            .flatten()
            {
                target.delete(&gl);
            }
            self.programs.delete(&gl);
            gl.delete_vertex_array(self.vao);
        }
    }

    /// Tamanho do viewport em px físicos (uniform dos passes em tela).
    fn viewport_size(frame: &RenderFrameInput<'_>) -> [f32; 2] {
        [
            frame.viewport.width.max(1) as f32,
            frame.viewport.height.max(1) as f32,
        ]
    }
}
