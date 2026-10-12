//! Viewport OpenGL integrado ao Slint/FemtoVG — o único caminho de GPU (R02/R03).
//!
//! O contexto GL é do Slint: ele só fica corrente dentro do *rendering notifier*.
//! Por isso o fluxo tem duas metades:
//!
//! 1. [`GlViewport::render_frame`] (qualquer callback da UI) extrai um
//!    [`OwnedRenderFrame`] — CPU apenas — e pede um novo quadro à janela;
//! 2. o notifier (`BeforeRendering`, contexto corrente) desenha esse frame num FBO
//!    próprio (MSAA + resolve) e a textura de cor é importada pelo Slint **sem
//!    readback**.
//!
//! Se o contexto GL não puder ser criado (ou o renderer não for OpenGL 3.3
//! Core), o [`GlViewport`] devolve a imagem do viewport por software.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use petunia_core::viewport::PhysicalViewport;
use petunia_core::{Camera, GridSettings, SelectionDomain, TextureUpdate, Workspace};
use petunia_project::{PoseOverride, Project};
use petunia_render::OverlayShape;
use petunia_render::extract::{ComponentStyle, SceneExtractor, SceneParts};
use petunia_render::frame::{
    DirtyRect, OwnedRenderFrame, RenderGrid, ViewportPalette, ViewportRenderSettings,
};
use petunia_render::gl::{GlRenderer, GlTarget};
use slint::ComponentHandle;
use uuid::Uuid;

use crate::{PetuniaSlintShell, PetuniaViewport, Software3dViewport, ViewportRenderState};

/// Cor de fundo do viewport (a mesma do caminho anterior, sem salto visual).
const BACKGROUND: [f32; 4] = [0.082, 0.086, 0.098, 1.0];

/// MSAA do viewport: 4x por padrão (limitado ao suportado);
/// `PETUNIA_MSAA=1|2|4|8` ajusta para hardware fraco ou capturas.
fn msaa_samples() -> u32 {
    std::env::var("PETUNIA_MSAA")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|samples| matches!(samples, 1 | 2 | 4 | 8))
        .unwrap_or(4)
}

/// Regiões de textura acumuladas entre o último desenho e o próximo: frames
/// extraídos e substituídos antes do notifier não podem perder retângulos.
#[derive(Debug, Clone, PartialEq)]
enum PendingDirty {
    Full,
    Regions(Vec<DirtyRect>),
}

/// Estado compartilhado entre o viewport (callbacks) e o notifier (render).
#[derive(Default)]
pub struct GlShared {
    /// Frame mais recente a desenhar; mantido para redesenhar após resize/setup.
    frame: Option<OwnedRenderFrame>,
    frame_dirty: bool,
    /// Tamanho desejado em pixels físicos.
    size: (u32, u32),
    gl: Option<GlResources>,
    /// Falha na criação do contexto/renderer: o viewport cai para software.
    failed: Option<String>,
    /// A janela foi configurada para desenhar por OpenGL.
    notifier_installed: bool,
    /// Revisão e regiões sujas por textura desde o último desenho.
    pending_dirty: HashMap<Uuid, (u64, PendingDirty)>,
}

/// Acumula as regiões do frame novo; o desenho as aplica de uma vez.
fn accumulate_dirty(pending: &mut HashMap<Uuid, (u64, PendingDirty)>, frame: &OwnedRenderFrame) {
    for texture in &frame.textures {
        let entry = pending
            .entry(texture.id)
            .or_insert((texture.revision, PendingDirty::Regions(Vec::new())));
        if entry.0 == texture.revision {
            continue;
        }
        entry.0 = texture.revision;
        match (&mut entry.1, &texture.dirty) {
            (PendingDirty::Full, _) => {}
            (slot, None) => *slot = PendingDirty::Full,
            (PendingDirty::Regions(regions), Some(more)) => regions.extend(more),
        }
    }
}

/// Aplica as regiões acumuladas ao frame que vai ser desenhado e zera o acúmulo.
fn apply_pending_dirty(
    pending: &mut HashMap<Uuid, (u64, PendingDirty)>,
    frame: &mut OwnedRenderFrame,
) {
    for texture in &mut frame.textures {
        if let Some((_, slot)) = pending.get(&texture.id) {
            texture.dirty = match slot {
                PendingDirty::Full => None,
                PendingDirty::Regions(regions) if regions.is_empty() => texture.dirty.clone(),
                PendingDirty::Regions(regions) => Some(regions.clone()),
            };
        }
    }
    for (_, slot) in pending.values_mut() {
        *slot = PendingDirty::Regions(Vec::new());
    }
}

struct GlResources {
    context: Arc<glow::Context>,
    renderer: GlRenderer,
    target: Option<GlTarget>,
}

// SAFETY: `GlShared` só é tocado pela thread da UI (callbacks do Slint e o notifier de
// render rodam nela); o `Mutex` existe para satisfazer `PetuniaViewport: Send`.
unsafe impl Send for GlShared {}

pub type SharedGl = Arc<Mutex<GlShared>>;

/// Liga o desenho GL ao ciclo de render da janela. `Err` quando o renderer atual não
/// é OpenGL (ex.: software) — o chamador usa o viewport por software.
pub fn install_notifier(
    window: &PetuniaSlintShell,
    shared: &SharedGl,
) -> Result<(), slint::SetRenderingNotifierError> {
    let notifier_shared = Arc::clone(shared);
    let weak = window.as_weak();
    window
        .window()
        .set_rendering_notifier(move |state, api| match state {
            slint::RenderingState::RenderingSetup => setup(&notifier_shared, api),
            slint::RenderingState::BeforeRendering => before_rendering(&notifier_shared, &weak),
            slint::RenderingState::RenderingTeardown => teardown(&notifier_shared),
            _ => {}
        })?;
    if let Ok(mut guard) = shared.lock() {
        guard.notifier_installed = true;
    }
    Ok(())
}

fn setup(shared: &SharedGl, api: &slint::GraphicsAPI<'_>) {
    let Ok(mut guard) = shared.lock() else {
        return;
    };
    let slint::GraphicsAPI::NativeOpenGL { get_proc_address } = api else {
        guard.failed = Some("o renderer do Slint não é OpenGL".into());
        return;
    };
    // SAFETY: Slint guarantees the GL context is current during RenderingSetup.
    let context = unsafe {
        Arc::new(glow::Context::from_loader_function_cstr(|name| {
            get_proc_address(name)
        }))
    };
    match GlRenderer::new(Arc::clone(&context)) {
        Ok(renderer) => {
            println!("Viewport OpenGL: {}", renderer.caps().summary());
            guard.gl = Some(GlResources {
                context,
                renderer,
                target: None,
            });
            guard.frame_dirty = true;
        }
        Err(error) => {
            eprintln!("petunia3d: viewport OpenGL indisponível: {error}");
            guard.failed = Some(error);
        }
    }
}

fn before_rendering(shared: &SharedGl, weak: &slint::Weak<PetuniaSlintShell>) {
    let Ok(mut guard) = shared.lock() else {
        return;
    };
    let GlShared {
        frame,
        frame_dirty,
        size: (width, height),
        gl,
        failed,
        pending_dirty,
        ..
    } = &mut *guard;
    let (width, height) = (*width, *height);
    let Some(GlResources {
        context,
        renderer,
        target,
    }) = gl.as_mut()
    else {
        return;
    };
    if width == 0 || height == 0 {
        return;
    }
    let samples = frame
        .as_ref()
        .map_or(1, |frame| frame.settings.msaa_samples);
    // Alvo com o tamanho e as amostras pedidas (recriado no resize). O alvo
    // limita as amostras ao suportado; só recria quando o pedido muda.
    let target_ok = target.as_ref().is_some_and(|t| {
        t.width() == width && t.height() == height && t.requested_samples() == samples
    });
    if !target_ok {
        // SAFETY: the Slint GL context is current inside BeforeRendering.
        unsafe {
            if let Some(old) = target.take() {
                old.delete(context);
            }
            match GlTarget::with_samples(context, width, height, samples) {
                Ok(created) => *target = Some(created),
                Err(error) => {
                    *failed = Some(error);
                    return;
                }
            }
        }
        *frame_dirty = true;
    }
    let Some(target) = target.as_ref() else {
        return;
    };
    if *frame_dirty && let Some(frame) = frame.as_mut() {
        // Frame extraído com outro tamanho de viewport: ajusta ao alvo atual.
        frame.viewport = PhysicalViewport {
            x: 0,
            y: 0,
            width,
            height,
        };
        frame.framebuffer_height = height;
        apply_pending_dirty(pending_dirty, frame);
        // CPU: custo de enfileirar os comandos GL (inclui as consultas de estado).
        crate::perf::measure("gl_draw_into_cpu", || {
            renderer.draw_into(target, &frame.as_input());
        });
        // Diagnóstico opt-in: o que cada desenho recebeu e enviou à GPU.
        if std::env::var_os("PETUNIA_GL_TRACE").is_some() {
            let input = frame.as_input();
            let stats = renderer.stats;
            eprintln!(
                "petunia3d: gl draw texturas={} com_pixels={} objetos_texturizados={}/{} \
                 uploads_full={} uploads_parciais={} pulados={}",
                input.textures.len(),
                input
                    .textures
                    .iter()
                    .filter(|t| t.pixels.len() >= t.width as usize * t.height as usize * 4)
                    .count(),
                input.objects.iter().filter(|o| o.texture.is_some()).count(),
                input.objects.len(),
                stats.full_texture_uploads,
                stats.partial_texture_uploads,
                stats.skipped_uploads,
            );
        }
        if crate::perf::enabled() {
            let stats = renderer.stats;
            crate::perf::record_stage(
                "gl_state_save",
                std::time::Duration::from_nanos(stats.last_state_save_nanos),
            );
            crate::perf::record_stage(
                "gl_state_restore",
                std::time::Duration::from_nanos(stats.last_state_restore_nanos),
            );
            // GPU: o que sobrou na fila. `glFinish` só roda com o timing ligado
            // porque serializa CPU e GPU; sem a variável o pipeline fica intacto.
            crate::perf::measure("gl_gpu_wait", || {
                use glow::HasContext;
                // SAFETY: the Slint GL context is current inside BeforeRendering.
                unsafe { context.finish() };
            });
        }
        *frame_dirty = false;
    }
    if let Some(window) = weak.upgrade() {
        window.set_viewport_image(texture_image(target));
    }
}

fn teardown(shared: &SharedGl) {
    if let Ok(mut guard) = shared.lock()
        && let Some(mut resources) = guard.gl.take()
    {
        // SAFETY: the context is still current during RenderingTeardown.
        unsafe {
            if let Some(target) = resources.target.take() {
                target.delete(&resources.context);
            }
            resources.renderer.destroy();
        }
    }
}

/// Importa a textura de cor do FBO como imagem Slint (origem embaixo-esquerda).
fn texture_image(target: &GlTarget) -> slint::Image {
    // SAFETY: the texture was created by the context that Slint renders with.
    unsafe {
        slint::BorrowedOpenGLTextureBuilder::new_gl_2d_rgba_texture(
            target.texture_id(),
            euclid::default::Size2D::<u32>::new(target.width(), target.height()),
        )
    }
    .origin(slint::BorrowedOpenGLTextureOrigin::BottomLeft)
    .build()
}

pub struct GlViewport {
    shared: SharedGl,
    extractor: SceneExtractor,
    window: slint::Weak<PetuniaSlintShell>,
    pub workspace: Workspace,
    pub selection_domain: SelectionDomain,
    pixel_ratio: f32,
    outlined: Vec<Uuid>,
    outlined_active: Option<Uuid>,
    rigid_previews: Vec<(Uuid, glam::Mat4)>,
    pose: Option<Arc<PoseOverride>>,
    texture_updates: Vec<TextureUpdate>,
    screen_overlay: Vec<OverlayShape>,
    msaa_samples: u32,
    /// Caminho de reserva usado se o OpenGL falhar.
    software: Software3dViewport,
}

impl GlViewport {
    pub fn new(shared: SharedGl, window: &PetuniaSlintShell, width: u32, height: u32) -> Self {
        if let Ok(mut guard) = shared.lock() {
            guard.size = (width.max(1), height.max(1));
        }
        Self {
            shared,
            extractor: SceneExtractor::new(),
            window: window.as_weak(),
            workspace: Workspace::Model,
            selection_domain: SelectionDomain::Object,
            pixel_ratio: 1.0,
            outlined: Vec::new(),
            outlined_active: None,
            rigid_previews: Vec::new(),
            pose: None,
            texture_updates: Vec::new(),
            screen_overlay: Vec::new(),
            msaa_samples: msaa_samples(),
            software: Software3dViewport::new(width, height),
        }
    }

    fn failed(&self) -> bool {
        self.shared
            .lock()
            .map_or(true, |guard| guard.failed.is_some())
    }

    fn settings(
        state: &ViewportRenderState,
        pixel_ratio: f32,
        msaa: u32,
    ) -> ViewportRenderSettings {
        let grid = GridSettings::default();
        ViewportRenderSettings {
            shading: state.shading,
            xray: state.xray,
            xray_opacity: state.xray_opacity,
            show_triangulation: state.show_triangulation,
            show_wireframe_overlay: state.show_wireframe_overlay,
            show_face_orientation: state.show_face_orientation,
            show_uv_checker: state.show_uv_checker,
            textured: state.textured,
            edit_mode_is_edit: state.selection_domain.is_component(),
            show_overlays: true,
            show_grid: state.show_grid,
            show_axes: true,
            grid: RenderGrid {
                size: grid.size,
                spacing: grid.subdivisions,
                opacity: grid.opacity,
                show_isometric_guide: grid.show_isometric_guide,
                isometric_angle_deg: grid.isometric_angle_deg,
            },
            edge_mode: state.edge_mode,
            pixel_ratio,
            matcap: state.matcap,
            ambient_occlusion: state.ambient_occlusion,
            msaa_samples: msaa,
            ..ViewportRenderSettings::default()
        }
    }
}

impl PetuniaViewport for GlViewport {
    fn resize(&mut self, width: u32, height: u32) {
        let (width, height) = (width.clamp(1, 16384), height.clamp(1, 16384));
        if let Ok(mut guard) = self.shared.lock()
            && guard.size != (width, height)
        {
            guard.size = (width, height);
            guard.frame_dirty = true;
        }
        self.software.resize(width, height);
    }

    fn uses_physical_pixels(&self) -> bool {
        true
    }

    fn set_pixel_ratio(&mut self, ratio: f32) {
        self.pixel_ratio = if ratio.is_finite() && ratio > 0.0 {
            ratio.clamp(0.5, 4.0)
        } else {
            1.0
        };
    }

    fn update(&mut self, _dt_seconds: f32) {}

    fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
    }

    fn set_selection_domain(&mut self, domain: SelectionDomain) {
        self.selection_domain = domain;
        self.software.set_selection_domain(domain);
    }

    fn queue_texture_updates(&mut self, updates: Vec<TextureUpdate>) {
        self.texture_updates.extend(updates);
    }

    fn draws_component_guides(&self) -> bool {
        !self.failed()
    }

    fn draws_gizmo(&self) -> bool {
        !self.failed()
    }

    fn set_screen_overlay(&mut self, shapes: Vec<OverlayShape>) {
        self.screen_overlay = shapes;
    }

    fn set_pose_override(&mut self, pose: Option<Arc<PoseOverride>>) {
        self.software.set_pose_override(pose.clone());
        self.pose = pose;
    }

    fn set_outlined_objects(&mut self, selected: &[Uuid], active: Option<Uuid>) {
        self.outlined.clear();
        self.outlined.extend_from_slice(selected);
        self.outlined_active = active;
    }

    fn set_rigid_previews(&mut self, previews: &[(Uuid, glam::Mat4)]) {
        self.rigid_previews.clear();
        self.rigid_previews.extend_from_slice(previews);
    }

    fn render_frame(
        &mut self,
        project: &Project,
        refs: &[petunia_core::ReferenceImage],
        camera: &Camera,
        state: ViewportRenderState,
    ) -> Option<slint::Image> {
        let (width, height, failed) = {
            let guard = self.shared.lock().ok()?;
            (guard.size.0, guard.size.1, guard.failed.is_some())
        };
        if failed {
            return self.software.render_frame(project, refs, camera, state);
        }
        let viewport = PhysicalViewport {
            x: 0,
            y: 0,
            width,
            height,
        };
        let updates = std::mem::take(&mut self.texture_updates);
        let mut parts = SceneParts::new(project, camera, viewport);
        parts.refs = refs;
        parts.settings = Self::settings(&state, self.pixel_ratio, self.msaa_samples);
        parts.palette = ViewportPalette {
            background: BACKGROUND,
        };
        parts.studio_light_follows_camera = state.studio_light_follows_camera;
        parts.pose = self.pose.as_deref();
        parts.rigid_previews = &self.rigid_previews;
        parts.components = ComponentStyle {
            domain: state.selection_domain,
            hover: state.hover,
            selection_rgb: state.selection_rgb,
            thickness: state.selection_thickness,
        };
        parts.outlined = &self.outlined;
        parts.outlined_active = self.outlined_active;
        parts.texture_updates = &updates;
        parts.workplane = state.workplane;
        parts.screen_overlay = &self.screen_overlay;
        let frame = self.extractor.extract_owned(parts);
        {
            let mut guard = self.shared.lock().ok()?;
            accumulate_dirty(&mut guard.pending_dirty, &frame);
            guard.frame = Some(frame);
            guard.frame_dirty = true;
        }
        // O desenho acontece no próximo BeforeRendering; garante que ele ocorra.
        if let Some(window) = self.window.upgrade() {
            window.window().request_redraw();
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_render::frame::{CameraMatrices, OwnedTexture};

    #[test]
    fn shared_state_starts_without_gl_and_without_failure() {
        let shared = GlShared::default();
        assert!(shared.gl.is_none());
        assert!(shared.failed.is_none());
        assert!(!shared.notifier_installed);
    }

    #[test]
    fn settings_follow_the_render_state() {
        let state = ViewportRenderState {
            xray: true,
            show_grid: false,
            selection_domain: SelectionDomain::Vertex,
            matcap: true,
            ..ViewportRenderState::default()
        };
        let settings = GlViewport::settings(&state, 2.0, 4);
        assert!(settings.xray && !settings.show_grid);
        assert!(settings.edit_mode_is_edit);
        assert!(settings.show_overlays && settings.show_axes);
        assert!(settings.matcap);
        assert_eq!(settings.pixel_ratio, 2.0);
        assert_eq!(settings.msaa_samples, 4);
    }

    fn frame_with_texture(revision: u64, dirty: Option<Vec<DirtyRect>>) -> OwnedRenderFrame {
        let viewport = PhysicalViewport {
            x: 0,
            y: 0,
            width: 4,
            height: 4,
        };
        let mut frame =
            OwnedRenderFrame::empty(viewport, CameraMatrices::from_camera(&Camera::default()));
        frame.textures.push(OwnedTexture {
            id: Uuid::nil(),
            width: 8,
            height: 8,
            revision,
            pixels: Arc::new(vec![0; 256]),
            dirty,
        });
        frame
    }

    fn rect(x: u32) -> DirtyRect {
        DirtyRect {
            x,
            y: 0,
            width: 1,
            height: 1,
        }
    }

    #[test]
    fn dirty_regions_survive_frames_replaced_before_drawing() {
        let mut shared = GlShared::default();
        accumulate_dirty(
            &mut shared.pending_dirty,
            &frame_with_texture(1, Some(vec![rect(0)])),
        );
        accumulate_dirty(
            &mut shared.pending_dirty,
            &frame_with_texture(2, Some(vec![rect(1)])),
        );
        accumulate_dirty(
            &mut shared.pending_dirty,
            &frame_with_texture(3, Some(vec![rect(2)])),
        );
        let mut frame = frame_with_texture(3, Some(vec![rect(2)]));
        apply_pending_dirty(&mut shared.pending_dirty, &mut frame);
        assert_eq!(frame.textures[0].dirty, Some(vec![rect(1), rect(2)]));
    }

    #[test]
    fn a_full_upload_wins_over_regions_until_drawn() {
        let mut shared = GlShared::default();
        accumulate_dirty(&mut shared.pending_dirty, &frame_with_texture(1, None));
        accumulate_dirty(&mut shared.pending_dirty, &frame_with_texture(2, None));
        accumulate_dirty(
            &mut shared.pending_dirty,
            &frame_with_texture(3, Some(vec![rect(0)])),
        );
        let mut frame = frame_with_texture(3, Some(vec![rect(0)]));
        apply_pending_dirty(&mut shared.pending_dirty, &mut frame);
        assert_eq!(frame.textures[0].dirty, None);
        // Depois do desenho, o próximo frame só leva as próprias regiões.
        accumulate_dirty(
            &mut shared.pending_dirty,
            &frame_with_texture(4, Some(vec![rect(3)])),
        );
        let mut next = frame_with_texture(4, Some(vec![rect(3)]));
        apply_pending_dirty(&mut shared.pending_dirty, &mut next);
        assert_eq!(next.textures[0].dirty, Some(vec![rect(3)]));
    }
}
