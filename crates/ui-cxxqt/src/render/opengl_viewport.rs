use glow::HasContext;
use petunia_application::Application;
use petunia_core::camera::Camera;
use petunia_core::viewport::PhysicalViewport;
use petunia_render::extract::{ComponentStyle, SceneExtractor, SceneParts};
use petunia_render::gl::{GlRenderer, GlTarget};
use std::sync::{Arc, LazyLock, Mutex};

/// Host gerenciador do renderizador OpenGL e aplicação dentro do contexto do Qt.
pub struct QtViewportRenderer {
    pub app: Application,
    pub camera: Camera,
    extractor: SceneExtractor,
    renderer: Option<GlRenderer>,
    gl: Option<Arc<glow::Context>>,
    target: Option<GlTarget>,
    blit_fbo: Option<glow::NativeFramebuffer>,
    width: u32,
    height: u32,
    pub current_frame: i32,
    pub total_frames: i32,
    pub is_playing: bool,
    pub keyframes: std::collections::HashSet<(usize, i32)>,
}

impl QtViewportRenderer {
    pub fn new() -> Self {
        let mut app = Application::default();
        if !app.state.project.assets.is_empty() {
            app.state.select_object(Some(0), false);
            app.state.sync_selection();
        }
        let mut keyframes = std::collections::HashSet::new();
        keyframes.insert((0, 1));
        keyframes.insert((0, 30));
        keyframes.insert((0, 60));
        Self {
            app,
            camera: Camera::default(),
            extractor: SceneExtractor::new(),
            renderer: None,
            gl: None,
            target: None,
            blit_fbo: None,
            width: 800,
            height: 600,
            current_frame: 1,
            total_frames: 250,
            is_playing: false,
            keyframes,
        }
    }

    /// Inicializa o contexto OpenGL (chamado no `initializeGL()` do Qt).
    ///
    /// # Safety
    /// O chamador deve garantir que o contexto OpenGL do Qt está ativo na thread atual.
    pub unsafe fn initialize_with_loader<F>(&mut self, loader: F) -> Result<(), String>
    where
        F: FnMut(&str) -> *const std::ffi::c_void,
    {
        let gl_context = unsafe { glow::Context::from_loader_function(loader) };
        let gl = Arc::new(gl_context);
        let renderer = GlRenderer::new(Arc::clone(&gl))?;

        let blit_fbo = unsafe { gl.create_framebuffer().ok() };

        self.gl = Some(gl);
        self.renderer = Some(renderer);
        self.blit_fbo = blit_fbo;
        Ok(())
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1);
        self.height = height.max(1);
        self.camera.aspect = self.width as f32 / self.height as f32;
    }

    pub fn orbit_camera(&mut self, dx: f32, dy: f32) {
        self.camera.orbit(dx, dy);
    }

    pub fn pan_camera(&mut self, dx: f32, dy: f32) {
        self.camera.pan(dx, dy);
    }

    pub fn zoom_camera(&mut self, delta: f32) {
        self.camera.zoom(delta);
    }

    pub fn reset_camera(&mut self) {
        self.camera.reset();
    }

    /// Renderiza um frame da cena Petunia3D e faz o blit para o FBO padrão do QOpenGLWidget.
    pub fn paint_into_qt_fbo(&mut self, qt_fbo: u32, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        self.camera.aspect = width as f32 / height as f32;

        let gl = match &self.gl {
            Some(gl) => Arc::clone(gl),
            None => return,
        };

        // Garante que o GlTarget (FBO interno do Petunia com MSAA) esteja dimensionado corretamente
        let target_needs_recreate = self
            .target
            .as_ref()
            .map_or(true, |t| t.width() != width || t.height() != height);

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

        // Extrai o frame completo da cena (malhas, materiais, grid, eixos, seleção)
        let domain = self.app.state.selection_domain();
        let hover = self.app.state.session.tools.hover;
        let mut parts = SceneParts::new(&self.app.state.project.project, &self.camera, viewport);
        parts.settings.shading = self.app.state.shading;
        parts.settings.edit_mode_is_edit = domain.is_component();
        parts.components = ComponentStyle {
            domain,
            hover,
            selection_rgb: [233, 106, 0],
            thickness: 2.0,
        };
        parts.outlined = &self.app.state.session.selection.assets;
        parts.outlined_active = self.app.state.project.active().map(|a| a.id);
        let frame_input = self.extractor.extract_parts(parts);

        // 1. Desenha o frame Petunia3D dentro do GlTarget (com multi-pass, MSAA e resolve)
        renderer.draw_into(target, &frame_input);

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

    /// Retorna se o renderer foi inicializado com sucesso.
    pub fn is_initialized(&self) -> bool {
        self.renderer.is_some()
    }
}

impl Default for QtViewportRenderer {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: All internal structures of Application, Camera, SceneExtractor and GlRenderer
// are solely owned and mutually excluded through Mutex.
unsafe impl Send for QtViewportRenderer {}

pub static VIEWPORT_RENDERER: LazyLock<Mutex<QtViewportRenderer>> =
    LazyLock::new(|| Mutex::new(QtViewportRenderer::new()));

// --- FFI para o C++ (PetuniaGlWidget e PetuniaMainWindow) ---

pub type QtGlGetProcAddressFn =
    unsafe extern "C" fn(*const std::ffi::c_char) -> *const std::ffi::c_void;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn petunia_qt_gl_initialize(get_proc: Option<QtGlGetProcAddressFn>) -> bool {
    let Some(get_proc) = get_proc else {
        return false;
    };

    let mut guard = match VIEWPORT_RENDERER.lock() {
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
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_gl_resize(width: i32, height: i32) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.resize(width.max(1) as u32, height.max(1) as u32);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_gl_paint(fbo_id: u32, width: i32, height: i32) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        let w = width.max(1) as u32;
        let h = height.max(1) as u32;
        guard.paint_into_qt_fbo(fbo_id, w, h);
    }
}

// Camera Navigation FFI
#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_orbit(dx: f32, dy: f32) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.orbit_camera(dx, dy);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_pan(dx: f32, dy: f32) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.pan_camera(dx, dy);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_zoom(delta: f32) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.zoom_camera(delta);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_reset() {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.reset_camera();
    }
}

// Application & Outliner FFI
#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_execute_command(cmd: *const std::ffi::c_char) -> bool {
    if cmd.is_null() {
        return false;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(cmd) };
    let Ok(cmd_str) = c_str.to_str() else {
        return false;
    };
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        match cmd_str {
            "app.undo" | "edit.undo" => guard.app.undo(),
            "app.redo" | "edit.redo" => guard.app.redo(),
            "project.new" | "file.new" => {
                guard.app.state.project.reset();
                true
            }
            _ => guard.app.execute_registered_command(cmd_str).is_ok(),
        }
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_undo() -> bool {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.app.undo()
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_redo() -> bool {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.app.redo()
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_asset_count() -> i32 {
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        guard.app.state.project.assets.len() as i32
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_asset_name(
    index: i32,
    buf: *mut std::ffi::c_char,
    max_len: i32,
) -> bool {
    if buf.is_null() || max_len <= 0 {
        return false;
    }
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        if let Some(asset) = guard.app.state.project.assets.get(index as usize) {
            let bytes = asset.name.as_bytes();
            let copy_len = bytes.len().min((max_len - 1) as usize);
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), buf as *mut u8, copy_len);
                *buf.add(copy_len) = 0;
            }
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_asset_visible(index: i32) -> bool {
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        guard
            .app
            .state
            .project
            .assets
            .get(index as usize)
            .map(|a| a.visible)
            .unwrap_or(true)
    } else {
        true
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_toggle_visibility(index: i32, visible: bool) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        if let Some(asset) = guard.app.state.project.assets.get_mut(index as usize) {
            asset.visible = visible;
            guard.app.state.project.mark_dirty();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_select_asset(index: i32) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        let count = guard.app.state.project.assets.len();
        if count > 0 && index >= 0 && (index as usize) < count {
            guard.app.state.select_object(Some(index as usize), false);
            guard.app.state.sync_selection();
            guard.app.state.mark_dirty();
            guard.app.state.project.bump_selection();
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_active_asset() -> i32 {
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        if guard.app.state.project.active < guard.app.state.project.assets.len() {
            guard.app.state.project.active as i32
        } else {
            -1
        }
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_asset_position(
    index: i32,
    x: *mut f32,
    y: *mut f32,
    z: *mut f32,
) -> bool {
    if x.is_null() || y.is_null() || z.is_null() {
        return false;
    }
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        if let Some(asset) = guard.app.state.project.assets.get(index as usize) {
            unsafe {
                *x = asset.position[0];
                *y = asset.position[1];
                *z = asset.position[2];
            }
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_set_asset_position(index: i32, x: f32, y: f32, z: f32) -> bool {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        if let Some(asset) = guard.app.state.project.assets.get_mut(index as usize) {
            asset.position = [x, y, z];
            guard.app.state.project.mark_dirty();
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_is_dirty() -> bool {
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        guard.app.state.is_document_dirty()
    } else {
        false
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_selected_count() -> i32 {
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        match guard.app.state.selection_domain() {
            petunia_core::SelectionDomain::Object => {
                guard.app.state.session.selection.assets.len() as i32
            }
            petunia_core::SelectionDomain::Vertex => {
                guard.app.state.session.selection.verts.len() as i32
            }
            petunia_core::SelectionDomain::Edge => {
                guard.app.state.session.selection.edges.len() as i32
            }
            petunia_core::SelectionDomain::Face => {
                guard.app.state.session.selection.faces.len() as i32
            }
        }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_selection_domain() -> i32 {
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        match guard.app.state.selection_domain() {
            petunia_core::SelectionDomain::Vertex => 1,
            petunia_core::SelectionDomain::Edge => 2,
            petunia_core::SelectionDomain::Face => 3,
            petunia_core::SelectionDomain::Object => 0,
        }
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_search_commands(
    query_str: *const std::ffi::c_char,
    results_out: *mut std::ffi::c_char,
    max_len: i32,
) -> i32 {
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

    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        let items = guard.app.state.commands.query(query, &guard.app.state);
        let mut buffer = String::new();
        let mut count = 0;
        for item in items.into_iter().take(30) {
            let sc = item.shortcut.as_deref().unwrap_or("");
            let line = format!(
                "{}\t{}\t{}\t{}\n",
                item.id,
                item.label,
                item.category.as_str(),
                sc
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
}

// Tablet Stylus / Pressure FFI
#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_tablet_paint_stroke(
    _x: f32,
    _y: f32,
    pressure: f32,
    _tilt_x: f32,
    _tilt_y: f32,
    is_eraser: bool,
) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        let p = pressure.clamp(0.01, 1.0);
        guard.app.state.session.tools.paint_strength = p;
        guard.app.state.session.tools.brush_flow = p;
        if is_eraser {
            guard.app.state.session.tools.paint_brush_kind = 2; // Eraser
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_asset_rotation(
    index: i32,
    rx: *mut f32,
    ry: *mut f32,
    rz: *mut f32,
) -> bool {
    if rx.is_null() || ry.is_null() || rz.is_null() {
        return false;
    }
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        if let Some(asset) = guard.app.state.project.assets.get(index as usize) {
            unsafe {
                *rx = asset.rotation[0];
                *ry = asset.rotation[1];
                *rz = asset.rotation[2];
            }
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_set_asset_rotation(index: i32, rx: f32, ry: f32, rz: f32) -> bool {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        if let Some(asset) = guard.app.state.project.assets.get_mut(index as usize) {
            asset.rotation = [rx, ry, rz];
            guard.app.state.project.mark_dirty();
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_asset_scale(
    index: i32,
    sx: *mut f32,
    sy: *mut f32,
    sz: *mut f32,
) -> bool {
    if sx.is_null() || sy.is_null() || sz.is_null() {
        return false;
    }
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        if let Some(asset) = guard.app.state.project.assets.get(index as usize) {
            unsafe {
                *sx = asset.scale[0];
                *sy = asset.scale[1];
                *sz = asset.scale[2];
            }
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_set_asset_scale(index: i32, sx: f32, sy: f32, sz: f32) -> bool {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        if let Some(asset) = guard.app.state.project.assets.get_mut(index as usize) {
            asset.scale = [sx, sy, sz];
            guard.app.state.project.mark_dirty();
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_asset_color(
    index: i32,
    r: *mut f32,
    g: *mut f32,
    b: *mut f32,
) -> bool {
    if r.is_null() || g.is_null() || b.is_null() {
        return false;
    }
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        if let Some(asset) = guard.app.state.project.assets.get(index as usize) {
            unsafe {
                *r = asset.base_color[0];
                *g = asset.base_color[1];
                *b = asset.base_color[2];
            }
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_set_asset_color(index: i32, r: f32, g: f32, b: f32) -> bool {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        if let Some(asset) = guard.app.state.project.assets.get_mut(index as usize) {
            asset.base_color = [r.clamp(0.0, 1.0), g.clamp(0.0, 1.0), b.clamp(0.0, 1.0)];
            guard.app.state.project.mark_dirty();
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_scene_stats(verts_out: *mut i32, tris_out: *mut i32) -> bool {
    if verts_out.is_null() || tris_out.is_null() {
        return false;
    }
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        unsafe {
            *verts_out = guard.app.state.project.scene_verts() as i32;
            *tris_out = guard.app.state.project.scene_tris() as i32;
        }
        return true;
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_set_selection_domain(domain: i32) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        let target_domain = match domain {
            1 => petunia_core::SelectionDomain::Vertex,
            2 => petunia_core::SelectionDomain::Edge,
            3 => petunia_core::SelectionDomain::Face,
            _ => petunia_core::SelectionDomain::Object,
        };
        guard.app.state.set_selection_domain(target_domain);
        guard.app.state.sync_selection();
        guard.app.state.mark_dirty();
        guard.app.state.project.bump_selection();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_frame_selection() {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.camera.frame(glam::Vec3::ZERO, 2.5);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_set_preset(preset: i32) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        let p = match preset {
            1 => petunia_core::camera::ViewPreset::Front,
            2 => petunia_core::camera::ViewPreset::Back,
            3 => petunia_core::camera::ViewPreset::Right,
            4 => petunia_core::camera::ViewPreset::Left,
            5 => petunia_core::camera::ViewPreset::Top,
            6 => petunia_core::camera::ViewPreset::Bottom,
            7 => petunia_core::camera::ViewPreset::IsometricNE,
            _ => petunia_core::camera::ViewPreset::Persp,
        };
        guard.camera.set_preset(p);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_toggle_projection() {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.camera.toggle_projection();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_get_view_name(
    buf: *mut std::ffi::c_char,
    max_len: i32,
) -> bool {
    if buf.is_null() || max_len <= 0 {
        return false;
    }
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        let (name, _, _) = guard.camera.nominal_view();
        if let Ok(c_str) = std::ffi::CString::new(name) {
            let bytes = c_str.as_bytes_with_nul();
            if bytes.len() <= max_len as usize {
                unsafe {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const i8, buf, bytes.len());
                }
                return true;
            }
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_camera_get_orientation(
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
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        let r = guard.camera.right();
        let u = guard.camera.up();
        let f = guard.camera.forward();
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
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_shading() -> i32 {
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        return match guard.app.state.shading {
            petunia_core::state::Shading::Wireframe => 0,
            petunia_core::state::Shading::Solid => 1,
            petunia_core::state::Shading::MaterialPreview => 2,
            petunia_core::state::Shading::Rendered => 3,
        };
    }
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_set_shading(shading: i32) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.app.state.shading = match shading {
            0 => petunia_core::state::Shading::Wireframe,
            2 => petunia_core::state::Shading::MaterialPreview,
            3 => petunia_core::state::Shading::Rendered,
            _ => petunia_core::state::Shading::Solid,
        };
        guard.app.state.mark_dirty();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_save_project(path_cstr: *const std::ffi::c_char) -> bool {
    if path_cstr.is_null() {
        return false;
    }
    let Ok(c_str) = unsafe { std::ffi::CStr::from_ptr(path_cstr) }.to_str() else {
        return false;
    };
    let path = std::path::Path::new(c_str);
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        if petunia_project::format::save(&guard.app.state.project.project, path).is_ok() {
            guard
                .app
                .state
                .project
                .set_file_path(Some(c_str.to_string()));
            guard.app.state.project.mark_clean();
            return true;
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_load_project(path_cstr: *const std::ffi::c_char) -> bool {
    if path_cstr.is_null() {
        return false;
    }
    let Ok(c_str) = unsafe { std::ffi::CStr::from_ptr(path_cstr) }.to_str() else {
        return false;
    };
    let path = std::path::Path::new(c_str);
    match petunia_project::format::load(path) {
        Ok(loaded) => {
            if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
                guard.app.state.project.project = loaded;
                guard
                    .app
                    .state
                    .project
                    .set_file_path(Some(c_str.to_string()));
                guard.app.state.project.mark_clean();
                guard.camera.reset();
                return true;
            }
            false
        }
        Err(_) => false,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_export_gltf(path_cstr: *const std::ffi::c_char) -> bool {
    if path_cstr.is_null() {
        return false;
    }
    let Ok(c_str) = unsafe { std::ffi::CStr::from_ptr(path_cstr) }.to_str() else {
        return false;
    };
    let path = std::path::Path::new(c_str);
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        let indices: Vec<usize> = (0..guard.app.state.project.assets.len()).collect();
        if let Ok(bytes) =
            petunia_project::export::export_gltf(&guard.app.state.project.project, &indices)
        {
            return std::fs::write(path, bytes).is_ok();
        }
    }
    false
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_export_obj(path_cstr: *const std::ffi::c_char) -> bool {
    if path_cstr.is_null() {
        return false;
    }
    let Ok(c_str) = unsafe { std::ffi::CStr::from_ptr(path_cstr) }.to_str() else {
        return false;
    };
    let path = std::path::Path::new(c_str);
    if let Ok(guard) = VIEWPORT_RENDERER.lock() {
        let mut full_obj = String::new();
        for asset in &guard.app.state.project.assets {
            full_obj.push_str(&petunia_project::export::export_obj(asset));
            full_obj.push('\n');
        }
        return std::fs::write(path, full_obj).is_ok();
    }
    false
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
    if !screen_x.is_finite() || !screen_y.is_finite() || viewport_w <= 1.0 || viewport_h <= 1.0 {
        return false;
    }

    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
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
    let scene =
        petunia_core::viewport_query::ViewportSceneQuery::new(&guard.app.state.project.project);

    if domain == petunia_core::SelectionDomain::Object {
        let hit_object = scene.nearest_object(&guard.camera, ndc);
        let hit = hit_object.is_some();
        match hit_object {
            Some(index) => {
                guard.app.state.select_object(Some(index), extend);
                guard.app.state.session.tools.hover = petunia_core::HoverTarget::Object(index);
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
            &guard.camera,
            viewport_pixels,
            cursor_ndc,
            mode,
            xray,
            |point| scene.point_visible(&guard.camera, point),
        )
    } else {
        petunia_core::picking::pick_mesh_in_scene(
            &mesh_cow,
            active_idx,
            &scene,
            &guard.camera,
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
                    guard.app.state.session.tools.hover = petunia_core::HoverTarget::Edge(a, b);
                    petunia_core::ComponentSet::Edges(vec![(a, b)])
                }
                petunia_core::picking::PickComponent::Face(i) => {
                    guard.app.state.session.tools.hover = petunia_core::HoverTarget::Face(i);
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
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_viewport_hover(
    screen_x: f32,
    screen_y: f32,
    viewport_w: f32,
    viewport_h: f32,
) -> bool {
    if !screen_x.is_finite() || !screen_y.is_finite() || viewport_w <= 1.0 || viewport_h <= 1.0 {
        return false;
    }

    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
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
    let scene =
        petunia_core::viewport_query::ViewportSceneQuery::new(&guard.app.state.project.project);

    if domain == petunia_core::SelectionDomain::Object {
        let hit_object = scene.nearest_object(&guard.camera, ndc);
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
        let changed = guard.app.state.session.tools.hover != petunia_core::HoverTarget::None;
        guard.app.state.session.tools.hover = petunia_core::HoverTarget::None;
        return changed;
    };

    let has_modifiers = asset.has_enabled_modifiers();
    let Some(mesh_cow) = active_mesh_in_world(&guard.app.state) else {
        let changed = guard.app.state.session.tools.hover != petunia_core::HoverTarget::None;
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
            &guard.camera,
            viewport_pixels,
            cursor_ndc,
            mode,
            xray,
            |point| scene.point_visible(&guard.camera, point),
        )
    } else {
        petunia_core::picking::pick_mesh_in_scene(
            &mesh_cow,
            active_idx,
            &scene,
            &guard.camera,
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
        petunia_core::picking::PickComponent::Edge(a, b) => petunia_core::HoverTarget::Edge(a, b),
        petunia_core::picking::PickComponent::Face(i) => petunia_core::HoverTarget::Face(i),
    });

    let changed = guard.app.state.session.tools.hover != new_hover;
    guard.app.state.session.tools.hover = new_hover;
    changed
}

// =========================================================================
// MODIFIERS STACK FFI
// =========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_modifier_count(asset_index: i32) -> i32 {
    let Ok(guard) = VIEWPORT_RENDERER.lock() else {
        return 0;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return 0;
    }
    guard.app.state.project.assets[asset_index as usize]
        .modifiers
        .len() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_modifier_info(
    asset_index: i32,
    mod_index: i32,
    name_buf: *mut std::ffi::c_char,
    max_len: i32,
    enabled: *mut bool,
    axis: *mut i32,
    weld: *mut f32,
) -> bool {
    let Ok(guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    let asset = &guard.app.state.project.assets[asset_index as usize];
    if mod_index < 0 || mod_index as usize >= asset.modifiers.len() {
        return false;
    }
    let modifier = &asset.modifiers[mod_index as usize];
    let (name, ax, w) = match &modifier.kind {
        petunia_project::ModifierKind::Mirror { axis, weld } => ("Mirror", *axis as i32, *weld),
        petunia_project::ModifierKind::Symmetry { axis, weld, .. } => {
            ("Symmetry", *axis as i32, *weld)
        }
    };

    if !name_buf.is_null() && max_len > 0 {
        let bytes = name.as_bytes();
        let len = bytes.len().min((max_len - 1) as usize);
        unsafe {
            std::ptr::copy_nonoverlapping(bytes.as_ptr() as *const std::ffi::c_char, name_buf, len);
            *name_buf.add(len) = 0;
        }
    }
    if !enabled.is_null() {
        unsafe {
            *enabled = modifier.enabled;
        }
    }
    if !axis.is_null() {
        unsafe {
            *axis = ax;
        }
    }
    if !weld.is_null() {
        unsafe {
            *weld = w;
        }
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_add_modifier(
    asset_index: i32,
    kind: i32, // 0 = Mirror, 1 = Symmetry
    axis: i32,
    weld: f32,
) -> bool {
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    let asset_id = guard.app.state.project.assets[asset_index as usize].id;
    let instance = if kind == 0 {
        petunia_project::ModifierInstance::mirror(axis.clamp(0, 2) as usize, weld.max(0.0001))
    } else {
        petunia_project::ModifierInstance::symmetry(
            axis.clamp(0, 2) as usize,
            true,
            weld.max(0.0001),
        )
    };

    let res = guard
        .app
        .state
        .dispatch(&petunia_core::ModifierCmd {
            op: petunia_core::ModifierOp::Add {
                asset: asset_id,
                instance,
            },
        })
        .is_ok();
    if res {
        guard.app.state.mark_dirty();
    }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_toggle_modifier(
    asset_index: i32,
    mod_index: i32,
    enabled: bool,
) -> bool {
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    let asset = &guard.app.state.project.assets[asset_index as usize];
    if mod_index < 0 || mod_index as usize >= asset.modifiers.len() {
        return false;
    }
    let mod_id = asset.modifiers[mod_index as usize].id;
    let res = guard
        .app
        .state
        .dispatch(&petunia_core::ModifierCmd {
            op: petunia_core::ModifierOp::SetEnabled {
                modifier: mod_id,
                enabled,
            },
        })
        .is_ok();
    if res {
        guard.app.state.mark_dirty();
    }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_remove_modifier(asset_index: i32, mod_index: i32) -> bool {
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    let asset = &guard.app.state.project.assets[asset_index as usize];
    if mod_index < 0 || mod_index as usize >= asset.modifiers.len() {
        return false;
    }
    let mod_id = asset.modifiers[mod_index as usize].id;
    let res = guard
        .app
        .state
        .dispatch(&petunia_core::ModifierCmd {
            op: petunia_core::ModifierOp::Remove { modifier: mod_id },
        })
        .is_ok();
    if res {
        guard.app.state.mark_dirty();
    }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_apply_modifier(asset_index: i32, mod_index: i32) -> bool {
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    let asset = &guard.app.state.project.assets[asset_index as usize];
    if mod_index < 0 || mod_index as usize >= asset.modifiers.len() {
        return false;
    }
    let mod_id = asset.modifiers[mod_index as usize].id;
    let res = guard
        .app
        .state
        .dispatch(&petunia_core::ModifierCmd {
            op: petunia_core::ModifierOp::Apply { modifier: mod_id },
        })
        .is_ok();
    if res {
        guard.app.state.mark_dirty();
    }
    res
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_move_modifier(asset_index: i32, mod_index: i32, up: bool) -> bool {
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    let asset = &guard.app.state.project.assets[asset_index as usize];
    if mod_index < 0 || mod_index as usize >= asset.modifiers.len() {
        return false;
    }
    let mod_id = asset.modifiers[mod_index as usize].id;
    let res = guard
        .app
        .state
        .dispatch(&petunia_core::ModifierCmd {
            op: petunia_core::ModifierOp::Move {
                modifier: mod_id,
                up,
            },
        })
        .is_ok();
    if res {
        guard.app.state.mark_dirty();
    }
    res
}

// =========================================================================
// PBR MATERIAL & SURFACE SHADING FFI
// =========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_material_pbr(
    asset_index: i32,
    roughness: *mut f32,
    metallic: *mut f32,
    emission_strength: *mut f32,
    profile: *mut i32,
) -> bool {
    let Ok(guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    let asset = &guard.app.state.project.assets[asset_index as usize];
    let mat = asset
        .material_id
        .and_then(|id| {
            guard
                .app
                .state
                .project
                .project
                .materials
                .iter()
                .find(|m| m.id == id)
        })
        .or_else(|| guard.app.state.project.project.materials.first());

    let Some(m) = mat else {
        return false;
    };
    unsafe {
        if !roughness.is_null() {
            *roughness = m.roughness;
        }
        if !metallic.is_null() {
            *metallic = m.metallic;
        }
        if !emission_strength.is_null() {
            *emission_strength = m.emission_strength;
        }
        if !profile.is_null() {
            *profile = match m.profile {
                petunia_project::material::ShaderProfile::Pbr => 0,
                petunia_project::material::ShaderProfile::Unlit => 1,
                petunia_project::material::ShaderProfile::Toon => 2,
                petunia_project::material::ShaderProfile::Glass => 3,
                petunia_project::material::ShaderProfile::Emissive => 4,
            };
        }
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_set_material_pbr(
    asset_index: i32,
    roughness: f32,
    metallic: f32,
    emission_strength: f32,
    profile: i32,
) -> bool {
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    let mat_id = guard.app.state.project.assets[asset_index as usize]
        .material_id
        .or_else(|| {
            guard
                .app
                .state
                .project
                .project
                .materials
                .first()
                .map(|m| m.id)
        });

    let Some(mat_id) = mat_id else {
        return false;
    };

    let prof = match profile {
        1 => petunia_project::material::ShaderProfile::Unlit,
        2 => petunia_project::material::ShaderProfile::Toon,
        3 => petunia_project::material::ShaderProfile::Glass,
        4 => petunia_project::material::ShaderProfile::Emissive,
        _ => petunia_project::material::ShaderProfile::Pbr,
    };

    let _ = guard
        .app
        .state
        .dispatch(&petunia_core::material_commands::EditMaterialCmd {
            material: mat_id,
            edit: petunia_core::material_commands::MaterialEdit::Roughness(
                roughness.clamp(0.0, 1.0),
            ),
        });
    let _ = guard
        .app
        .state
        .dispatch(&petunia_core::material_commands::EditMaterialCmd {
            material: mat_id,
            edit: petunia_core::material_commands::MaterialEdit::Metallic(metallic.clamp(0.0, 1.0)),
        });
    let _ = guard
        .app
        .state
        .dispatch(&petunia_core::material_commands::EditMaterialCmd {
            material: mat_id,
            edit: petunia_core::material_commands::MaterialEdit::EmissionStrength(
                emission_strength.max(0.0),
            ),
        });
    let _ = guard
        .app
        .state
        .dispatch(&petunia_core::material_commands::EditMaterialCmd {
            material: mat_id,
            edit: petunia_core::material_commands::MaterialEdit::Profile(prof),
        });

    guard.app.state.mark_dirty();
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_surface_shading(asset_index: i32) -> i32 {
    let Ok(guard) = VIEWPORT_RENDERER.lock() else {
        return 0;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return 0;
    }
    match guard.app.state.project.assets[asset_index as usize].surface_shading {
        petunia_mesh::shading::SurfaceShading::Flat => 0,
        petunia_mesh::shading::SurfaceShading::Smooth => 1,
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_set_surface_shading(asset_index: i32, shading: i32) -> bool {
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    guard.app.state.project.assets[asset_index as usize].surface_shading = if shading == 1 {
        petunia_mesh::shading::SurfaceShading::Smooth
    } else {
        petunia_mesh::shading::SurfaceShading::Flat
    };
    guard.app.state.mark_dirty();
    true
}

// =========================================================================
// QUICK ACTIONS FFI
// =========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_quick_action(action: *const std::ffi::c_char) -> bool {
    if action.is_null() {
        return false;
    }
    let c_str = unsafe { std::ffi::CStr::from_ptr(action) };
    let Ok(act) = c_str.to_str() else {
        return false;
    };

    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    match act {
        "subdivide" => {
            let res = guard
                .app
                .state
                .dispatch(&petunia_core::command::SubdivideSelectionCmd)
                .is_ok();
            guard.app.state.mark_dirty();
            res
        }
        "invert_selection" => {
            let res = guard
                .app
                .state
                .dispatch(&petunia_core::InvertSelectionCmd)
                .is_ok();
            guard.app.state.mark_dirty();
            res
        }
        "select_all" => {
            let res = guard
                .app
                .state
                .dispatch(&petunia_core::SelectAllCmd)
                .is_ok();
            guard.app.state.mark_dirty();
            res
        }
        "deselect_all" => {
            let res = guard
                .app
                .state
                .dispatch(&petunia_core::ClearSelectionCmd)
                .is_ok();
            guard.app.state.mark_dirty();
            res
        }
        "origin_to_geometry" => {
            let _ = guard.app.state.set_origin_geometry();
            guard.app.state.mark_dirty();
            true
        }
        "origin_to_bottom" => {
            let _ = guard.app.state.set_origin_bottom();
            guard.app.state.mark_dirty();
            true
        }
        "symmetrize" => {
            let res = guard
                .app
                .state
                .dispatch(&petunia_core::SymmetrizeCmd {
                    axis: 0,
                    positive_to_negative: true,
                    eps: 1e-4,
                })
                .is_ok();
            guard.app.state.mark_dirty();
            res
        }
        _ => guard.app.execute_registered_command(act).is_ok(),
    }
}

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
pub extern "C" fn petunia_qt_gizmo_get_projected(
    viewport_w: f32,
    viewport_h: f32,
    out_origin: *mut f32,
    out_x: *mut f32,
    out_y: *mut f32,
    out_z: *mut f32,
) -> bool {
    let Ok(guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    let Some(asset) = guard.app.state.project.active() else {
        return false;
    };
    let origin_world = glam::Vec3::new(asset.position[0], asset.position[1], asset.position[2]);
    let dist = (guard.camera.eye() - origin_world).length().max(0.1);
    let gizmo_scale = dist * 0.16;

    let p_orig = project_world_point(&guard.camera, [viewport_w, viewport_h], origin_world);
    let p_x = project_world_point(
        &guard.camera,
        [viewport_w, viewport_h],
        origin_world + glam::Vec3::X * gizmo_scale,
    );
    let p_y = project_world_point(
        &guard.camera,
        [viewport_w, viewport_h],
        origin_world + glam::Vec3::Y * gizmo_scale,
    );
    let p_z = project_world_point(
        &guard.camera,
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
    if !petunia_qt_gizmo_get_projected(
        viewport_w,
        viewport_h,
        orig.as_mut_ptr(),
        px.as_mut_ptr(),
        py.as_mut_ptr(),
        pz.as_mut_ptr(),
    ) {
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
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    let asset_idx = guard.app.state.project.project.active;
    if asset_idx == usize::MAX || asset_idx >= guard.app.state.project.assets.len() {
        return false;
    }

    let asset = &guard.app.state.project.assets[asset_idx];
    let origin_world = glam::Vec3::new(asset.position[0], asset.position[1], asset.position[2]);
    let dist = (guard.camera.eye() - origin_world).length().max(0.1);
    let gizmo_scale = dist * 0.16;

    let p_orig = project_world_point(&guard.camera, [viewport_w, viewport_h], origin_world);
    let target_axis_pt = match axis {
        1 => origin_world + glam::Vec3::X * gizmo_scale,
        2 => origin_world + glam::Vec3::Y * gizmo_scale,
        3 => origin_world + glam::Vec3::Z * gizmo_scale,
        _ => return false,
    };
    let p_axis = project_world_point(&guard.camera, [viewport_w, viewport_h], target_axis_pt);

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

    let asset_mut = &mut guard.app.state.project.assets[asset_idx];
    match axis {
        1 => asset_mut.position[0] += world_delta,
        2 => asset_mut.position[1] += world_delta,
        3 => asset_mut.position[2] += world_delta,
        _ => {}
    }
    let pos = asset_mut.position;
    let rot = asset_mut.rotation;
    let scale = asset_mut.scale;
    asset_mut.transform = petunia_project::object_transform::ObjectTransform::from_trs(
        glam::Vec3::from(pos),
        glam::Quat::from_euler(
            glam::EulerRot::XYZ,
            rot[0].to_radians(),
            rot[1].to_radians(),
            rot[2].to_radians(),
        ),
        glam::Vec3::from(scale),
    );
    guard.app.state.mark_dirty();
    true
}

// =========================================================================
// UV DIAGNOSTICS & GEOMETRY EXTRACTION FFI
// =========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_uv_diagnostics(
    asset_index: i32,
    island_count: *mut i32,
    overlap_ratio: *mut f32,
    mean_stretch: *mut f32,
) -> bool {
    let Ok(guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    let asset = &guard.app.state.project.assets[asset_index as usize];
    let diag = asset.mesh.uv_diagnostics();
    if !island_count.is_null() {
        unsafe {
            *island_count = diag.island_count as i32;
        }
    }
    if !overlap_ratio.is_null() {
        unsafe {
            *overlap_ratio = diag.overlap_ratio;
        }
    }
    if !mean_stretch.is_null() {
        unsafe {
            *mean_stretch = diag.mean_stretch;
        }
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_uv_face_count(asset_index: i32) -> i32 {
    let Ok(guard) = VIEWPORT_RENDERER.lock() else {
        return 0;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return 0;
    }
    guard.app.state.project.assets[asset_index as usize]
        .mesh
        .faces
        .len() as i32
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_uv_face_verts(
    asset_index: i32,
    face_idx: i32,
    out_uvs: *mut f32,
    max_verts: i32,
) -> i32 {
    let Ok(guard) = VIEWPORT_RENDERER.lock() else {
        return 0;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return 0;
    }
    let mesh = &guard.app.state.project.assets[asset_index as usize].mesh;
    if face_idx < 0 || face_idx as usize >= mesh.faces.len() {
        return 0;
    }
    let face = &mesh.faces[face_idx as usize];
    let n = face.uv.len().min(max_verts as usize);
    if !out_uvs.is_null() {
        for i in 0..n {
            unsafe {
                *out_uvs.add(i * 2) = face.uv[i][0];
                *out_uvs.add(i * 2 + 1) = face.uv[i][1];
            }
        }
    }
    n as i32
}

// =========================================================================
// TIMELINE & DOPESHEET KEYFRAMES FFI
// =========================================================================

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_get_timeline_info(
    current_frame: *mut i32,
    total_frames: *mut i32,
    is_playing: *mut bool,
) -> bool {
    let Ok(guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    unsafe {
        if !current_frame.is_null() {
            *current_frame = guard.current_frame;
        }
        if !total_frames.is_null() {
            *total_frames = guard.total_frames;
        }
        if !is_playing.is_null() {
            *is_playing = guard.is_playing;
        }
    }
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_set_timeline_frame(frame: i32) {
    if let Ok(mut guard) = VIEWPORT_RENDERER.lock() {
        guard.current_frame = frame.clamp(1, guard.total_frames);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_toggle_playback() -> bool {
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    guard.is_playing = !guard.is_playing;
    guard.is_playing
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_has_keyframe(asset_index: i32, frame: i32) -> bool {
    let Ok(guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 {
        return false;
    }
    guard.keyframes.contains(&(asset_index as usize, frame))
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_insert_keyframe(asset_index: i32, frame: i32) -> bool {
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 || asset_index as usize >= guard.app.state.project.assets.len() {
        return false;
    }
    guard.keyframes.insert((asset_index as usize, frame));
    guard.app.state.mark_dirty();
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn petunia_qt_app_delete_keyframe(asset_index: i32, frame: i32) -> bool {
    let Ok(mut guard) = VIEWPORT_RENDERER.lock() else {
        return false;
    };
    if asset_index < 0 {
        return false;
    }
    let removed = guard.keyframes.remove(&(asset_index as usize, frame));
    if removed {
        guard.app.state.mark_dirty();
    }
    removed
}
