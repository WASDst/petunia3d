use glow::HasContext;
use petunia_core::camera::Camera;
use petunia_core::viewport::PhysicalViewport;
use petunia_editor::PetuniaViewport;
use petunia_render::extract::{ComponentStyle, SceneExtractor, SceneParts};
use petunia_render::gl::{GlRenderer, GlTarget};
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{Arc, OnceLock},
};

/// A single document/controller, with independently owned GL resources for each
/// QOpenGLWidget context. All native GL calls must remain on the owning thread.
pub struct QtViewportRenderer {
    pub app: crate::editor::QtEditor,
    surfaces: HashMap<u64, GlSurface>,
    software: HashMap<u64, SoftwareSurface>,
    cameras: HashMap<u64, Camera>,
    active_view: u64,
    dimensions: HashMap<u64, (u32, u32, f32)>,
    width: u32,
    height: u32,
}
struct GlSurface {
    owner: std::thread::ThreadId,
    extractor: SceneExtractor,
    renderer: Option<GlRenderer>,
    gl: Option<Arc<glow::Context>>,
    target: Option<GlTarget>,
    blit_fbo: Option<glow::NativeFramebuffer>,
}
struct SoftwareSurface {
    owner: std::thread::ThreadId,
    viewport: petunia_editor::Software3dViewport,
    bytes: Vec<u8>,
    width: u32,
    height: u32,
}
impl QtViewportRenderer {
    pub fn new() -> Self {
        let mut app = crate::editor::QtEditor::default();
        if !app.state.project.assets.is_empty() {
            app.state.select_object(Some(0), false);
            app.state.sync_selection();
        }
        Self {
            app,
            surfaces: HashMap::new(),
            software: HashMap::new(),
            cameras: HashMap::new(),
            active_view: 1,
            dimensions: HashMap::new(),
            width: 800,
            height: 600,
        }
    }
    pub fn activate(&mut self, id: u64) {
        let bank = |id: u64| if id == 3 { 1 } else { id }; // PiP shares the primary camera; split is independent.
        if id != self.active_view {
            self.cameras.insert(
                bank(self.active_view),
                self.app.state.session.camera.clone(),
            );
            let next = self
                .cameras
                .get(&bank(id))
                .cloned()
                .unwrap_or_else(|| self.app.state.session.camera.clone());
            self.app.state.session.camera = next;
            self.active_view = id;
        }
        if let Some((w, h, dpr)) = self.dimensions.get(&id).copied() {
            self.app.controller.resize_viewport_scaled(w, h, dpr);
        }
    }
    /// # Safety
    /// The loader belongs to the current Qt GL context on this thread.
    pub unsafe fn initialize_with_loader<F>(&mut self, loader: F) -> Result<(), String>
    where
        F: FnMut(&str) -> *const std::ffi::c_void,
    {
        unsafe { self.initialize_surface(1, loader) }
    }
    unsafe fn initialize_surface<F>(&mut self, id: u64, loader: F) -> Result<(), String>
    where
        F: FnMut(&str) -> *const std::ffi::c_void,
    {
        // Context recreation must first release its previous resources while
        // that context is current (aboutToBeDestroyed invokes release_surface).
        if self.surfaces.contains_key(&id) {
            return Err("GL surface already initialized".into());
        }
        let gl = Arc::new(unsafe { glow::Context::from_loader_function(loader) });
        let renderer = GlRenderer::new(gl.clone())?;
        let blit_fbo = Some(unsafe { gl.create_framebuffer() }?);
        self.surfaces.insert(
            id,
            GlSurface {
                owner: std::thread::current().id(),
                extractor: SceneExtractor::new(),
                renderer: Some(renderer),
                gl: Some(gl),
                target: None,
                blit_fbo,
            },
        );
        Ok(())
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.app.controller.resize_viewport(self.width, self.height);
    }
    pub fn orbit_camera(&mut self, dx: f32, dy: f32) {
        self.app.controller.orbit_viewport(dx, dy);
    }
    pub fn pan_camera(&mut self, dx: f32, dy: f32) {
        self.app.state.session.camera.pan(dx, dy);
    }
    pub fn zoom_camera(&mut self, delta: f32) {
        self.app.state.session.camera.zoom(delta);
    }
    pub fn reset_camera(&mut self) {
        self.app.state.session.camera.reset();
    }
    pub fn paint_into_qt_fbo(&mut self, fbo: u32, width: u32, height: u32) {
        self.paint_surface(1, fbo, width, height, 1.)
    }
    pub fn paint_surface(&mut self, id: u64, fbo: u32, width: u32, height: u32, ratio: f32) {
        let Some(mut surface) = self.surfaces.remove(&id) else {
            return;
        };
        if surface.owner != std::thread::current().id() {
            self.surfaces.insert(id, surface);
            return;
        }
        let previous = self.active_view;
        self.dimensions.insert(id, (width, height, ratio));
        self.activate(id);
        self.app
            .controller
            .resize_viewport_scaled(width, height, ratio);
        surface.paint(
            &mut self.app,
            fbo,
            (width as f32 * ratio).round().max(1.) as u32,
            (height as f32 * ratio).round().max(1.) as u32,
            ratio,
        );
        self.surfaces.insert(id, surface);
        self.activate(previous);
    }
    pub fn release_surface(&mut self, id: u64) {
        let Some(mut surface) = self.surfaces.remove(&id) else {
            return;
        };
        if surface.owner != std::thread::current().id() {
            self.surfaces.insert(id, surface);
            return;
        }
        if let Some(gl) = surface.gl.take() {
            unsafe {
                if let Some(target) = surface.target.take() {
                    target.delete(&gl);
                }
                if let Some(fbo) = surface.blit_fbo.take() {
                    gl.delete_framebuffer(fbo);
                }
                // Explicit destruction while the owning Qt context is current.
                if let Some(renderer) = surface.renderer.take() {
                    renderer.destroy();
                }
            }
        }
    }
    fn software_frame(&mut self, id: u64, width: u32, height: u32) -> Option<&SoftwareSurface> {
        // Cap CPU raster work while keeping logical input/overlay coordinates.
        let factor = (1280.0 / width.max(1) as f32)
            .min(720.0 / height.max(1) as f32)
            .min(1.0);
        let w = (width as f32 * factor).round().max(1.) as u32;
        let h = (height as f32 * factor).round().max(1.) as u32;
        let mut surface = self
            .software
            .remove(&id)
            .unwrap_or_else(|| SoftwareSurface {
                owner: std::thread::current().id(),
                viewport: petunia_editor::Software3dViewport::new(w, h),
                bytes: Vec::new(),
                width: w,
                height: h,
            });
        if surface.owner != std::thread::current().id() {
            self.software.insert(id, surface);
            return None;
        }
        let previous = self.active_view;
        self.dimensions.insert(id, (width, height, 1.));
        self.activate(id);
        self.app.controller.render_viewport();
        let capture = &self.app.controller.viewport;
        surface.viewport.resize(w, h);
        surface.viewport.set_workspace(self.app.state.workspace);
        surface
            .viewport
            .set_selection_domain(capture.render_state.selection_domain);
        surface.viewport.set_pose_override(capture.pose.clone());
        surface.viewport.set_rigid_previews(&capture.rigid);
        surface
            .viewport
            .set_outlined_objects(&capture.outlined, capture.active);
        surface
            .viewport
            .queue_texture_updates(capture.texture_updates.clone());
        surface.viewport.render_frame(
            &self.app.state.project.project,
            &self.app.state.project.refs,
            &self.app.state.session.camera,
            capture.render_state,
        );
        surface.width = surface.viewport.width;
        surface.height = surface.viewport.height;
        surface.bytes = surface.viewport.color_buffer.clone();
        self.app.controller.viewport.texture_updates.clear();
        self.software.insert(id, surface);
        self.activate(previous);
        self.software.get(&id)
    }
    pub fn is_initialized(&self) -> bool {
        self.surfaces.contains_key(&1)
    }
}
impl Default for QtViewportRenderer {
    fn default() -> Self {
        Self::new()
    }
}
impl GlSurface {
    /// Renderiza um frame da cena Petunia3D e faz o blit para o FBO padrão do QOpenGLWidget.
    fn paint(
        &mut self,
        app: &mut crate::editor::QtEditor,
        qt_fbo: u32,
        width: u32,
        height: u32,
        ratio: f32,
    ) {
        let width = width.max(1);
        let height = height.max(1);
        app.state.session.camera.aspect = width as f32 / height as f32;

        let gl = match &self.gl {
            Some(gl) => Arc::clone(gl),
            None => return,
        };

        // Garante que o GlTarget (FBO interno do Petunia com MSAA) esteja dimensionado corretamente
        let target_needs_recreate = self
            .target
            .as_ref()
            .is_none_or(|t| t.width() != width || t.height() != height);

        if target_needs_recreate {
            unsafe {
                if let Some(old) = self.target.take() {
                    old.delete(&gl);
                }
                if let Ok(new_target) = GlTarget::with_samples(&gl, width, height, 4) {
                    self.target = Some(new_target);
                }
            }
        }

        let Some(target) = &self.target else {
            return;
        };
        let Some(renderer) = &mut self.renderer else {
            return;
        };

        let viewport = PhysicalViewport {
            x: 0,
            y: 0,
            width,
            height,
        };

        app.controller.render_viewport();
        let capture = &app.controller.viewport;
        let state = capture.render_state;
        let mut parts = SceneParts::new(
            &app.state.project.project,
            &app.state.session.camera,
            viewport,
        );
        parts.refs = &app.state.project.refs;
        parts.settings = petunia_render::frame::ViewportRenderSettings {
            shading: state.shading,
            xray: state.xray,
            xray_opacity: state.xray_opacity,
            show_triangulation: state.show_triangulation,
            textured: state.textured,
            show_wireframe_overlay: state.show_wireframe_overlay,
            show_face_orientation: state.show_face_orientation,
            show_uv_checker: state.show_uv_checker,
            edit_mode_is_edit: state.selection_domain.is_component(),
            show_grid: state.show_grid,
            edge_mode: state.edge_mode,
            pixel_ratio: ratio,
            matcap: state.matcap,
            ambient_occlusion: state.ambient_occlusion,
            ..Default::default()
        };
        parts.pose = capture.pose.as_deref();
        parts.rigid_previews = &capture.rigid;
        parts.texture_updates = &capture.texture_updates;
        parts.workplane = state.workplane;
        parts.studio_light_follows_camera = state.studio_light_follows_camera;
        parts.components = ComponentStyle {
            domain: state.selection_domain,
            hover: state.hover,
            selection_rgb: state.selection_rgb,
            thickness: state.selection_thickness,
        };
        parts.outlined = &capture.outlined;
        parts.outlined_active = capture.active;
        let frame_input = self.extractor.extract_parts(parts);

        // 1. Desenha o frame Petunia3D dentro do GlTarget (com multi-pass, MSAA e resolve)
        renderer.draw_into(target, &frame_input);
        app.controller.viewport.texture_updates.clear();

        // 2. Transfere os pixels (Blit) da textura resolvida para o QOpenGLWidget FBO
        unsafe {
            let Some(blit_fbo) = self.blit_fbo else {
                return;
            };
            let tex_id = target.texture_id();
            let native_tex = glow::NativeTexture(tex_id);

            gl.bind_framebuffer(glow::READ_FRAMEBUFFER, Some(blit_fbo));
            gl.framebuffer_texture_2d(
                glow::READ_FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(native_tex),
                0,
            );

            let dest_fbo = if qt_fbo == 0 {
                None
            } else {
                Some(glow::NativeFramebuffer(
                    std::num::NonZeroU32::new(qt_fbo).unwrap(),
                ))
            };

            gl.bind_framebuffer(glow::DRAW_FRAMEBUFFER, dest_fbo);
            gl.blit_framebuffer(
                0,
                0,
                width as i32,
                height as i32,
                0,
                0,
                width as i32,
                height as i32,
                glow::COLOR_BUFFER_BIT,
                glow::NEAREST,
            );

            // Desconecta a textura do READ_FRAMEBUFFER e restaura o DRAW_FRAMEBUFFER do Qt
            gl.framebuffer_texture_2d(
                glow::READ_FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                None,
                0,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, dest_fbo);
        }
    }
}
thread_local! {
    static THREAD_RENDERER: RefCell<QtViewportRenderer> = RefCell::new(QtViewportRenderer::new());
}
static GUI_THREAD: OnceLock<std::thread::ThreadId> = OnceLock::new();
/// Thread-affine access. Headless callers own a separate editor per thread;
/// after QApplication claims ownership, worker calls cannot touch the GUI editor.
pub struct RendererAccess;
pub static VIEWPORT_RENDERER: RendererAccess = RendererAccess;
impl RendererAccess {
    pub fn with<R>(&self, f: impl FnOnce(&RefCell<QtViewportRenderer>) -> R) -> Option<R> {
        if GUI_THREAD
            .get()
            .is_some_and(|id| *id != std::thread::current().id())
        {
            return None;
        }
        THREAD_RENDERER.try_with(f).ok()
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_claim_ui_thread() -> bool {
    let id = std::thread::current().id();
    GUI_THREAD.set(id).is_ok() || GUI_THREAD.get() == Some(&id)
}
// --- FFI para o C++ (PetuniaGlWidget e PetuniaMainWindow) ---

pub type QtGlGetProcAddressFn =
    unsafe extern "C" fn(*const std::ffi::c_char) -> *const std::ffi::c_void;

/// # Safety
/// The loader must resolve functions for the current Qt GL context on this thread.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_qt_gl_initialize(get_proc: Option<QtGlGetProcAddressFn>) -> bool {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            let Some(get_proc) = get_proc else {
                return false;
            };

            let mut guard = match renderer.try_borrow_mut() {
                Ok(g) => g,
                Err(_) => return false,
            };

            let loader = |name: &str| {
                let Ok(c_name) = std::ffi::CString::new(name) else {
                    return std::ptr::null();
                };
                unsafe { get_proc(c_name.as_ptr()) }
            };

            match unsafe { guard.initialize_with_loader(loader) } {
                Ok(()) => true,
                Err(e) => {
                    eprintln!("[QtViewportRenderer] Erro na inicialização do renderer GL: {e}");
                    false
                }
            }
        })
        .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_gl_resize(width: i32, height: i32) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut guard) = renderer.try_borrow_mut() {
                guard.resize(width.max(1) as u32, height.max(1) as u32);
            }
        })
        .unwrap_or(())
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_gl_paint(fbo_id: u32, width: i32, height: i32) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut guard) = renderer.try_borrow_mut() {
                let w = width.max(1) as u32;
                let h = height.max(1) as u32;
                guard.paint_into_qt_fbo(fbo_id, w, h);
            }
        })
        .unwrap_or(())
}

// Camera Navigation FFI
#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_orbit(dx: f32, dy: f32) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut guard) = renderer.try_borrow_mut() {
                guard.orbit_camera(dx, dy);
            }
        })
        .unwrap_or(())
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_pan(dx: f32, dy: f32) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut guard) = renderer.try_borrow_mut() {
                guard.pan_camera(dx, dy);
            }
        })
        .unwrap_or(())
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_zoom(delta: f32) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut guard) = renderer.try_borrow_mut() {
                guard.zoom_camera(delta);
            }
        })
        .unwrap_or(())
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_reset() {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut guard) = renderer.try_borrow_mut() {
                guard.reset_camera();
            }
        })
        .unwrap_or(())
}

// Application & Outliner FFI

#[unsafe(no_mangle)]
/// # Safety
/// C string inputs must be valid and null-terminated. Non-null output pointers
/// must reference writable scalar storage or buffers of the advertised length.
pub unsafe extern "C" fn petunia_qt_app_search_commands(
    query_str: *const std::ffi::c_char,
    results_out: *mut std::ffi::c_char,
    max_len: i32,
) -> i32 {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if results_out.is_null() || max_len <= 0 {
                return 0;
            }
            let query = if query_str.is_null() {
                ""
            } else {
                unsafe { std::ffi::CStr::from_ptr(query_str) }
                    .to_str()
                    .unwrap_or("")
            };

            if let Ok(guard) = renderer.try_borrow_mut() {
                let items = guard.app.controller.search_commands(query);
                let mut buffer = String::new();
                let mut count = 0;
                for item in items.into_iter().take(30) {
                    let sc = item.shortcut.as_deref().unwrap_or("");
                    let line = format!(
                        "{}\t{}\t{}\t{}\t{}\t{}\n",
                        item.id,
                        item.label,
                        item.category.as_str(),
                        sc,
                        item.is_available,
                        item.disabled_reason.unwrap_or("")
                    );
                    if buffer.len() + line.len() + 1 >= (max_len as usize) {
                        break;
                    }
                    buffer.push_str(&line);
                    count += 1;
                }
                let bytes = buffer.as_bytes();
                let copy_len = bytes.len().min((max_len - 1) as usize);
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), results_out as *mut u8, copy_len);
                    *results_out.add(copy_len) = 0;
                }
                count
            } else {
                0
            }
        })
        .unwrap_or(0)
}

// Tablet Stylus / Pressure FFI
#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_tablet_paint_stroke(
    x: f32,
    y: f32,
    pressure: f32,
    tilt_x: f32,
    tilt_y: f32,
    is_eraser: bool,
) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut host) = renderer.try_borrow_mut() {
                let _ = host.app.request(
                    &serde_json::json!({"action":"tablet_input", "pressure":pressure,
            "tilt_x":tilt_x,"tilt_y":tilt_y,"eraser":is_eraser}),
                );
                let nx = x / host.width.max(1) as f32;
                let ny = y / host.height.max(1) as f32;
                host.app.controller.hover_component(nx, ny);
            }
        })
        .unwrap_or(())
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_frame_selection() {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut host) = renderer.try_borrow_mut() {
                let _ = host.app.execute_registered_command("view.frame_selection");
            }
        })
        .unwrap_or(())
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_set_preset(preset: i32) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut guard) = renderer.try_borrow_mut() {
                let p = match preset {
                    1 => petunia_core::camera::ViewPreset::Front,
                    2 => petunia_core::camera::ViewPreset::Back,
                    3 => petunia_core::camera::ViewPreset::Right,
                    4 => petunia_core::camera::ViewPreset::Left,
                    5 => petunia_core::camera::ViewPreset::Top,
                    6 => petunia_core::camera::ViewPreset::Bottom,
                    7 => petunia_core::camera::ViewPreset::IsometricNE,
                    8 => petunia_core::camera::ViewPreset::PseudoIsometric2To1,
                    _ => petunia_core::camera::ViewPreset::Persp,
                };
                guard.app.state.session.camera.set_preset(p);
            }
        })
        .unwrap_or(())
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_toggle_projection() {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut guard) = renderer.try_borrow_mut() {
                guard.app.state.session.camera.toggle_projection();
            }
        })
        .unwrap_or(())
}

#[unsafe(no_mangle)]
/// # Safety
/// C string inputs must be valid and null-terminated. Non-null output pointers
/// must reference writable scalar storage or buffers of the advertised length.
pub unsafe extern "C" fn petunia_qt_camera_get_view_name(
    buf: *mut std::ffi::c_char,
    max_len: i32,
) -> bool {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if buf.is_null() || max_len <= 0 {
                return false;
            }
            if let Ok(guard) = renderer.try_borrow_mut() {
                let (name, _, _) = guard.app.state.session.camera.nominal_view();
                if let Ok(c_str) = std::ffi::CString::new(name) {
                    let bytes = c_str.as_bytes_with_nul();
                    if bytes.len() <= max_len as usize {
                        unsafe {
                            std::ptr::copy_nonoverlapping(
                                bytes.as_ptr() as *const i8,
                                buf,
                                bytes.len(),
                            );
                        }
                        return true;
                    }
                }
            }
            false
        })
        .unwrap_or(false)
}

#[unsafe(no_mangle)]
/// # Safety
/// C string inputs must be valid and null-terminated. Non-null output pointers
/// must reference writable scalar storage or buffers of the advertised length.
pub unsafe extern "C" fn petunia_qt_camera_get_orientation(
    right_x: *mut f32,
    right_y: *mut f32,
    right_z: *mut f32,
    up_x: *mut f32,
    up_y: *mut f32,
    up_z: *mut f32,
    fwd_x: *mut f32,
    fwd_y: *mut f32,
    fwd_z: *mut f32,
) -> bool {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if right_x.is_null()
                || right_y.is_null()
                || right_z.is_null()
                || up_x.is_null()
                || up_y.is_null()
                || up_z.is_null()
                || fwd_x.is_null()
                || fwd_y.is_null()
                || fwd_z.is_null()
            {
                return false;
            }
            if let Ok(guard) = renderer.try_borrow_mut() {
                let r = guard.app.state.session.camera.right();
                let u = guard.app.state.session.camera.up();
                let f = guard.app.state.session.camera.forward();
                unsafe {
                    *right_x = r.x;
                    *right_y = r.y;
                    *right_z = r.z;
                    *up_x = u.x;
                    *up_y = u.y;
                    *up_z = u.z;
                    *fwd_x = f.x;
                    *fwd_y = f.y;
                    *fwd_z = f.z;
                }
                return true;
            }
            false
        })
        .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_shading() -> i32 {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(guard) = renderer.try_borrow_mut() {
                return match guard.app.state.shading {
                    petunia_core::state::Shading::Wireframe => 0,
                    petunia_core::state::Shading::Solid => 1,
                    petunia_core::state::Shading::MaterialPreview => 2,
                    petunia_core::state::Shading::Rendered => 3,
                };
            }
            1
        })
        .unwrap_or(0)
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_set_shading(shading: i32) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut host) = renderer.try_borrow_mut() {
                host.app.controller.set_shading_mode(match shading {
                    0 => "wireframe",
                    2 => "material",
                    3 => "rendered",
                    _ => "solid",
                });
            }
        })
        .unwrap_or(())
}

fn active_mesh_in_world<'a>(
    state: &'a petunia_core::AppState,
) -> Option<std::borrow::Cow<'a, petunia_core::Mesh>> {
    let asset = state.project.active()?;
    let mesh = state.project.active_mesh()?;
    if !asset.transform.is_identity() {
        let mut world = mesh.clone();
        world.transform_affine(asset.world_matrix());
        Some(std::borrow::Cow::Owned(world))
    } else {
        Some(std::borrow::Cow::Borrowed(mesh))
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_viewport_select(
    screen_x: f32,
    screen_y: f32,
    viewport_w: f32,
    viewport_h: f32,
    extend: bool,
) -> bool {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if !screen_x.is_finite()
                || !screen_y.is_finite()
                || viewport_w <= 1.0
                || viewport_h <= 1.0
            {
                return false;
            }

            let Ok(mut guard) = renderer.try_borrow_mut() else {
                return false;
            };

            let nx = (screen_x / viewport_w).clamp(0.0, 1.0);
            let ny = (screen_y / viewport_h).clamp(0.0, 1.0);
            let ndc_x = nx * 2.0 - 1.0;
            let ndc_y = 1.0 - ny * 2.0;
            let ndc = [ndc_x, ndc_y];
            let cursor_ndc = glam::Vec2::new(ndc_x, ndc_y);
            let viewport_pixels = glam::Vec2::new(viewport_w, viewport_h);

            let domain = guard.app.state.selection_domain();
            let scene = petunia_core::viewport_query::ViewportSceneQuery::new(
                &guard.app.state.project.project,
            );

            if domain == petunia_core::SelectionDomain::Object {
                let hit_object = scene.nearest_object(&guard.app.state.session.camera, ndc);
                let hit = hit_object.is_some();
                match hit_object {
                    Some(index) => {
                        guard.app.state.select_object(Some(index), extend);
                        guard.app.state.session.tools.hover =
                            petunia_core::HoverTarget::Object(index);
                    }
                    None => {
                        if !extend {
                            guard.app.state.select_object(None, false);
                        }
                        guard.app.state.session.tools.hover = petunia_core::HoverTarget::None;
                    }
                }
                guard.app.state.sync_selection();
                guard.app.state.mark_dirty();
                guard.app.state.project.bump_selection();
                return hit;
            }

            // Component modes: Vertex, Edge, Face
            let Some(asset) = guard
                .app
                .state
                .project
                .active()
                .filter(|asset| asset.visible && !asset.locked)
            else {
                return false;
            };

            let has_modifiers = asset.has_enabled_modifiers();
            let Some(mesh_cow) = active_mesh_in_world(&guard.app.state) else {
                return false;
            };

            let mode = match domain {
                petunia_core::SelectionDomain::Vertex => petunia_core::SelectMode::Vertex,
                petunia_core::SelectionDomain::Edge => petunia_core::SelectMode::Edge,
                petunia_core::SelectionDomain::Face => petunia_core::SelectMode::Face,
                petunia_core::SelectionDomain::Object => return false,
            };

            let xray = guard.app.state.session.show_xray
                || guard.app.state.shading == petunia_core::state::Shading::Wireframe;

            let active_idx = guard.app.state.project.project.active;
            let hit = if has_modifiers {
                petunia_core::picking::pick_mesh_filtered(
                    &mesh_cow,
                    &guard.app.state.session.camera,
                    viewport_pixels,
                    cursor_ndc,
                    mode,
                    xray,
                    |point| scene.point_visible(&guard.app.state.session.camera, point),
                )
            } else {
                petunia_core::picking::pick_mesh_in_scene(
                    &mesh_cow,
                    active_idx,
                    &scene,
                    &guard.app.state.session.camera,
                    viewport_pixels,
                    cursor_ndc,
                    mode,
                    xray,
                )
            };

            let op = if extend {
                petunia_core::SelectionOp::Toggle
            } else {
                petunia_core::SelectionOp::Replace
            };

            let did_hit = hit.is_some();
            match hit {
                Some(pick) => {
                    let set = match pick.component {
                        petunia_core::picking::PickComponent::Vertex(i) => {
                            guard.app.state.session.tools.hover =
                                petunia_core::HoverTarget::Vertex(i as u32);
                            petunia_core::ComponentSet::Vertices(vec![i as u32])
                        }
                        petunia_core::picking::PickComponent::Edge(a, b) => {
                            guard.app.state.session.tools.hover =
                                petunia_core::HoverTarget::Edge(a, b);
                            petunia_core::ComponentSet::Edges(vec![(a, b)])
                        }
                        petunia_core::picking::PickComponent::Face(i) => {
                            guard.app.state.session.tools.hover =
                                petunia_core::HoverTarget::Face(i);
                            petunia_core::ComponentSet::Faces(vec![i])
                        }
                    };
                    let _ = guard.app.state.apply_component_selection(&set, op);
                }
                None => {
                    if !extend {
                        let _ = guard.app.state.apply_component_selection(
                            &petunia_core::ComponentSet::Faces(Vec::new()),
                            petunia_core::SelectionOp::Replace,
                        );
                    }
                    guard.app.state.session.tools.hover = petunia_core::HoverTarget::None;
                }
            }

            guard.app.state.sync_selection();
            guard.app.state.mark_dirty();
            guard.app.state.project.bump_selection();
            did_hit
        })
        .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_viewport_hover(
    screen_x: f32,
    screen_y: f32,
    viewport_w: f32,
    viewport_h: f32,
) -> bool {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if !screen_x.is_finite()
                || !screen_y.is_finite()
                || viewport_w <= 1.0
                || viewport_h <= 1.0
            {
                return false;
            }

            let Ok(mut guard) = renderer.try_borrow_mut() else {
                return false;
            };

            let nx = (screen_x / viewport_w).clamp(0.0, 1.0);
            let ny = (screen_y / viewport_h).clamp(0.0, 1.0);
            let ndc_x = nx * 2.0 - 1.0;
            let ndc_y = 1.0 - ny * 2.0;
            let ndc = [ndc_x, ndc_y];
            let cursor_ndc = glam::Vec2::new(ndc_x, ndc_y);
            let viewport_pixels = glam::Vec2::new(viewport_w, viewport_h);

            let domain = guard.app.state.selection_domain();
            let scene = petunia_core::viewport_query::ViewportSceneQuery::new(
                &guard.app.state.project.project,
            );

            if domain == petunia_core::SelectionDomain::Object {
                let hit_object = scene.nearest_object(&guard.app.state.session.camera, ndc);
                let new_hover = hit_object.map_or(
                    petunia_core::HoverTarget::None,
                    petunia_core::HoverTarget::Object,
                );
                let changed = guard.app.state.session.tools.hover != new_hover;
                guard.app.state.session.tools.hover = new_hover;
                return changed;
            }

            let Some(asset) = guard
                .app
                .state
                .project
                .active()
                .filter(|asset| asset.visible && !asset.locked)
            else {
                let changed =
                    guard.app.state.session.tools.hover != petunia_core::HoverTarget::None;
                guard.app.state.session.tools.hover = petunia_core::HoverTarget::None;
                return changed;
            };

            let has_modifiers = asset.has_enabled_modifiers();
            let Some(mesh_cow) = active_mesh_in_world(&guard.app.state) else {
                let changed =
                    guard.app.state.session.tools.hover != petunia_core::HoverTarget::None;
                guard.app.state.session.tools.hover = petunia_core::HoverTarget::None;
                return changed;
            };

            let mode = match domain {
                petunia_core::SelectionDomain::Vertex => petunia_core::SelectMode::Vertex,
                petunia_core::SelectionDomain::Edge => petunia_core::SelectMode::Edge,
                petunia_core::SelectionDomain::Face => petunia_core::SelectMode::Face,
                petunia_core::SelectionDomain::Object => return false,
            };

            let xray = guard.app.state.session.show_xray
                || guard.app.state.shading == petunia_core::state::Shading::Wireframe;

            let active_idx = guard.app.state.project.project.active;
            let hit = if has_modifiers {
                petunia_core::picking::pick_mesh_filtered(
                    &mesh_cow,
                    &guard.app.state.session.camera,
                    viewport_pixels,
                    cursor_ndc,
                    mode,
                    xray,
                    |point| scene.point_visible(&guard.app.state.session.camera, point),
                )
            } else {
                petunia_core::picking::pick_mesh_in_scene(
                    &mesh_cow,
                    active_idx,
                    &scene,
                    &guard.app.state.session.camera,
                    viewport_pixels,
                    cursor_ndc,
                    mode,
                    xray,
                )
            };

            let new_hover = hit.map_or(petunia_core::HoverTarget::None, |h| match h.component {
                petunia_core::picking::PickComponent::Vertex(i) => {
                    petunia_core::HoverTarget::Vertex(i as u32)
                }
                petunia_core::picking::PickComponent::Edge(a, b) => {
                    petunia_core::HoverTarget::Edge(a, b)
                }
                petunia_core::picking::PickComponent::Face(i) => petunia_core::HoverTarget::Face(i),
            });

            let changed = guard.app.state.session.tools.hover != new_hover;
            guard.app.state.session.tools.hover = new_hover;
            changed
        })
        .unwrap_or(false)
}

// =========================================================================
// MODIFIERS STACK FFI
// =========================================================================

// =========================================================================
// PBR MATERIAL & SURFACE SHADING FFI
// =========================================================================

// =========================================================================
// QUICK ACTIONS FFI
// =========================================================================

// =========================================================================
// GIZMO 3D INTERACTIVE HANDLES MATH & HIT TESTING
// =========================================================================

fn project_world_point(camera: &Camera, viewport: [f32; 2], point: glam::Vec3) -> Option<[f32; 2]> {
    let clip = camera.view_proj() * point.extend(1.0);
    if !clip.is_finite() || clip.w <= 0.05 || clip.z < 0.0 || clip.z > clip.w {
        return None;
    }
    Some([
        (clip.x / clip.w * 0.5 + 0.5) * viewport[0],
        (0.5 - clip.y / clip.w * 0.5) * viewport[1],
    ])
}

#[unsafe(no_mangle)]
/// # Safety
/// C string inputs must be valid and null-terminated. Non-null output pointers
/// must reference writable scalar storage or buffers of the advertised length.
/// Each projected point output has space for two f32 coordinates.
pub unsafe extern "C" fn petunia_qt_gizmo_get_projected(
    viewport_w: f32,
    viewport_h: f32,
    out_origin: *mut f32,
    out_x: *mut f32,
    out_y: *mut f32,
    out_z: *mut f32,
) -> bool {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            let Ok(guard) = renderer.try_borrow_mut() else {
                return false;
            };
            let Some(asset) = guard.app.state.project.active() else {
                return false;
            };
            let origin_world =
                glam::Vec3::new(asset.position[0], asset.position[1], asset.position[2]);
            let dist = (guard.app.state.session.camera.eye() - origin_world)
                .length()
                .max(0.1);
            let gizmo_scale = dist * 0.16;

            let p_orig = project_world_point(
                &guard.app.state.session.camera,
                [viewport_w, viewport_h],
                origin_world,
            );
            let p_x = project_world_point(
                &guard.app.state.session.camera,
                [viewport_w, viewport_h],
                origin_world + glam::Vec3::X * gizmo_scale,
            );
            let p_y = project_world_point(
                &guard.app.state.session.camera,
                [viewport_w, viewport_h],
                origin_world + glam::Vec3::Y * gizmo_scale,
            );
            let p_z = project_world_point(
                &guard.app.state.session.camera,
                [viewport_w, viewport_h],
                origin_world + glam::Vec3::Z * gizmo_scale,
            );

            let (Some(orig), Some(px), Some(py), Some(pz)) = (p_orig, p_x, p_y, p_z) else {
                return false;
            };

            unsafe {
                if !out_origin.is_null() {
                    *out_origin = orig[0];
                    *out_origin.add(1) = orig[1];
                }
                if !out_x.is_null() {
                    *out_x = px[0];
                    *out_x.add(1) = px[1];
                }
                if !out_y.is_null() {
                    *out_y = py[0];
                    *out_y.add(1) = py[1];
                }
                if !out_z.is_null() {
                    *out_z = pz[0];
                    *out_z.add(1) = pz[1];
                }
            }
            true
        })
        .unwrap_or(false)
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_gizmo_hit_test(
    mouse_x: f32,
    mouse_y: f32,
    viewport_w: f32,
    viewport_h: f32,
) -> i32 {
    let mut orig = [0.0f32; 2];
    let mut px = [0.0f32; 2];
    let mut py = [0.0f32; 2];
    let mut pz = [0.0f32; 2];
    let projected = unsafe {
        petunia_qt_gizmo_get_projected(
            viewport_w,
            viewport_h,
            orig.as_mut_ptr(),
            px.as_mut_ptr(),
            py.as_mut_ptr(),
            pz.as_mut_ptr(),
        )
    };
    if !projected {
        return 0; // None
    }

    let dist_to_center = (mouse_x - orig[0]).hypot(mouse_y - orig[1]);
    if dist_to_center <= 14.0 {
        return 4; // Center
    }

    let dist_to_segment = |ax: f32, ay: f32, bx: f32, by: f32| -> f32 {
        let abx = bx - ax;
        let aby = by - ay;
        let apx = mouse_x - ax;
        let apy = mouse_y - ay;
        let ab_len_sq = abx * abx + aby * aby;
        if ab_len_sq < 1e-4 {
            return (mouse_x - ax).hypot(mouse_y - ay);
        }
        let t = ((apx * abx + apy * aby) / ab_len_sq).clamp(0.0, 1.0);
        let proj_x = ax + t * abx;
        let proj_y = ay + t * aby;
        (mouse_x - proj_x).hypot(mouse_y - proj_y)
    };

    if dist_to_segment(orig[0], orig[1], px[0], px[1]) <= 10.0 {
        return 1; // X Axis
    }
    if dist_to_segment(orig[0], orig[1], py[0], py[1]) <= 10.0 {
        return 2; // Y Axis
    }
    if dist_to_segment(orig[0], orig[1], pz[0], pz[1]) <= 10.0 {
        return 3; // Z Axis
    }

    0
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_gizmo_drag_axis(
    axis: i32, // 1=X, 2=Y, 3=Z
    dx: f32,
    dy: f32,
    viewport_w: f32,
    viewport_h: f32,
) -> bool {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            let Ok(mut guard) = renderer.try_borrow_mut() else {
                return false;
            };
            let asset_idx = guard.app.state.project.project.active;
            if asset_idx == usize::MAX || asset_idx >= guard.app.state.project.assets.len() {
                return false;
            }

            let asset = &guard.app.state.project.assets[asset_idx];
            let origin_world =
                glam::Vec3::new(asset.position[0], asset.position[1], asset.position[2]);
            let dist = (guard.app.state.session.camera.eye() - origin_world)
                .length()
                .max(0.1);
            let gizmo_scale = dist * 0.16;

            let p_orig = project_world_point(
                &guard.app.state.session.camera,
                [viewport_w, viewport_h],
                origin_world,
            );
            let target_axis_pt = match axis {
                1 => origin_world + glam::Vec3::X * gizmo_scale,
                2 => origin_world + glam::Vec3::Y * gizmo_scale,
                3 => origin_world + glam::Vec3::Z * gizmo_scale,
                _ => return false,
            };
            let p_axis = project_world_point(
                &guard.app.state.session.camera,
                [viewport_w, viewport_h],
                target_axis_pt,
            );

            let (Some(orig), Some(tip)) = (p_orig, p_axis) else {
                return false;
            };
            let dir_x = tip[0] - orig[0];
            let dir_y = tip[1] - orig[1];
            let len = dir_x.hypot(dir_y);
            if len < 1.0 {
                return false;
            }
            let norm_dir_x = dir_x / len;
            let norm_dir_y = dir_y / len;

            let mouse_proj = dx * norm_dir_x + dy * norm_dir_y;
            let world_delta = mouse_proj * (gizmo_scale / len);

            let mut next = asset.position;
            next[(axis - 1) as usize] += world_delta;
            set_absolute_transform(
                &mut guard.app,
                asset_idx as i32,
                petunia_editor::TransformKind::Position,
                next,
            )
        })
        .unwrap_or(false)
}

// =========================================================================
// UV DIAGNOSTICS & GEOMETRY EXTRACTION FFI
// =========================================================================

// =========================================================================
// TIMELINE & DOPESHEET KEYFRAMES FFI
// =========================================================================

/// # Safety
/// The loader must resolve functions for the current owning Qt GL context.
/// The caller must release this surface while that context is still current.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_qt_gl_surface_initialize(
    id: u64,
    get_proc: Option<QtGlGetProcAddressFn>,
) -> bool {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            let Some(loader) = get_proc else { return false };
            let Ok(mut host) = renderer.try_borrow_mut() else {
                return false;
            };
            unsafe { host.initialize_surface(id, |name| CStringName::load(name, loader)) }.is_ok()
        })
        .unwrap_or(false)
}
struct CStringName;
impl CStringName {
    fn load(name: &str, loader: QtGlGetProcAddressFn) -> *const std::ffi::c_void {
        std::ffi::CString::new(name).map_or(std::ptr::null(), |s| unsafe { loader(s.as_ptr()) })
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_gl_surface_release(id: u64) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut host) = renderer.try_borrow_mut() {
                host.release_surface(id)
            }
        })
        .unwrap_or(())
}
#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_gl_surface_paint(
    id: u64,
    fbo: u32,
    width: i32,
    height: i32,
    ratio: f32,
) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if ratio.is_finite()
                && ratio > 0.
                && let Ok(mut host) = renderer.try_borrow_mut()
            {
                host.paint_surface(
                    id,
                    fbo,
                    width.max(1) as u32,
                    height.max(1) as u32,
                    ratio.clamp(0.5, 4.),
                )
            }
        })
        .unwrap_or(())
}
#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_view_activate(id: u64) {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if let Ok(mut host) = renderer.try_borrow_mut() {
                host.activate(id)
            }
        })
        .unwrap_or(())
}

fn select_index(editor: &mut crate::editor::QtEditor, index: i32) -> bool {
    if index < 0 {
        return false;
    }
    let Some(asset) = editor.state.project.assets.get(index as usize) else {
        return false;
    };
    let id = asset.id.to_string();
    if editor.state.project.active != index as usize {
        editor
            .controller
            .apply(petunia_editor::UiIntent::SelectSceneAsset(id));
    }
    true
}
fn set_absolute_transform(
    editor: &mut crate::editor::QtEditor,
    index: i32,
    kind: petunia_editor::TransformKind,
    value: [f32; 3],
) -> bool {
    if !value.iter().all(|v| v.is_finite()) || !select_index(editor, index) {
        return false;
    }
    let intent = match kind {
        petunia_editor::TransformKind::Position => petunia_editor::UiIntent::SetAssetTransform {
            asset_index: Some(index as usize),
            translation: Some(value),
            rotation_euler_deg: None,
            scale: None,
        },
        petunia_editor::TransformKind::Rotation => petunia_editor::UiIntent::SetAssetTransform {
            asset_index: Some(index as usize),
            translation: None,
            rotation_euler_deg: Some(value),
            scale: None,
        },
        petunia_editor::TransformKind::Scale => petunia_editor::UiIntent::SetAssetTransform {
            asset_index: Some(index as usize),
            translation: None,
            rotation_euler_deg: None,
            scale: Some(value),
        },
    };
    editor.controller.apply(intent);
    true
}
/// The active paint layer is the only editable canvas, copied after size checks.
#[unsafe(no_mangle)]
/// # Safety
/// C string inputs must be valid and null-terminated. Non-null output pointers
/// must reference writable scalar storage or buffers of the advertised length.
pub unsafe extern "C" fn petunia_qt_canvas_rgba(
    output: *mut u8,
    capacity: usize,
    width: *mut u32,
    height: *mut u32,
) -> usize {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            let Ok(host) = renderer.try_borrow_mut() else {
                return 0;
            };
            let canvas = host
                .app
                .state
                .project
                .active()
                .and_then(|a| a.paint_stack.as_ref())
                .and_then(|s| s.active())
                .and_then(|l| l.canvas());
            let Some(canvas) = canvas else { return 0 };
            unsafe {
                if !width.is_null() {
                    *width = canvas.w
                }
                if !height.is_null() {
                    *height = canvas.h
                }
            }
            let n = canvas.pixels.len();
            if !output.is_null() && capacity >= n {
                unsafe { std::ptr::copy_nonoverlapping(canvas.pixels.as_ptr(), output, n) }
            }
            n
        })
        .unwrap_or(0)
}

/// The null-output call produces one CPU frame; the copy call consumes that cache.
/// # Safety
/// Non-null output pointers must reference writable storage of the given capacity.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_qt_software_frame_rgba(
    id: u64,
    width: u32,
    height: u32,
    output: *mut u8,
    capacity: usize,
    out_width: *mut u32,
    out_height: *mut u32,
) -> usize {
    crate::render::opengl_viewport::VIEWPORT_RENDERER
        .with(|renderer| {
            if width == 0 || height == 0 || width > 32768 || height > 32768 {
                return 0;
            }
            let Ok(mut host) = renderer.try_borrow_mut() else {
                return 0;
            };
            let surface = if output.is_null() {
                host.software_frame(id, width, height)
            } else {
                host.software.get(&id)
            };
            let Some(surface) = surface else { return 0 };
            if surface.owner != std::thread::current().id() {
                return 0;
            }
            unsafe {
                if !out_width.is_null() {
                    *out_width = surface.width
                }
                if !out_height.is_null() {
                    *out_height = surface.height
                }
            }
            let len = surface.bytes.len();
            if !output.is_null() {
                if capacity < len {
                    return 0;
                }
                unsafe { std::ptr::copy_nonoverlapping(surface.bytes.as_ptr(), output, len) }
            }
            len
        })
        .unwrap_or(0)
}
