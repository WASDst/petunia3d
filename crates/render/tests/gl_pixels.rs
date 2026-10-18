//! Testes de pixel do renderer OpenGL 3.3 em contexto real (EGL sem janela).
//!
//! São os gates de paridade de `website/docs/04-renderer/architecture-decision.md`
//! ("Gates para remover WGPU"), portados dos testes de aparência do antigo
//! backend WGPU (o oráculo de comportamento): comportamento e tolerância, nunca
//! identidade de pixels entre APIs.
//!
//! Sem dispositivo EGL os testes imprimem `SKIPPED` e retornam; com
//! `PETUNIA_REQUIRE_GL=1` a ausência de GPU vira falha (CI com GPU).

use std::num::NonZeroU32;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use glam::Vec3;
use glutin::api::egl::{context::PossiblyCurrentContext, device::Device, display::Display};
use glutin::config::{ConfigSurfaceTypes, ConfigTemplateBuilder};
use glutin::context::{ContextApi, ContextAttributesBuilder, GlProfile, Version};
use glutin::display::GlDisplay;
use petunia_core::viewport::PhysicalViewport;
use petunia_core::{
    Camera, HoverTarget, Projection, SelectionDomain, TextureDirtyRect, TextureUpdate, ViewPreset,
};
use petunia_mesh::{Face, Vertex};
use petunia_project::{Canvas, Project};
use petunia_render::extract::{SceneExtractor, SceneParts};
use petunia_render::frame::{OwnedRenderFrame, ViewportRenderSettings};
use petunia_render::gl::{GlRenderer, GlTarget};
use petunia_render::{EdgeMode, OverlayShape, Shading, WorkplaneOverlay};

/// Contextos nativos são criados um por vez (drivers instáveis em paralelo).
fn gpu_guard() -> MutexGuard<'static, ()> {
    static GPU: Mutex<()> = Mutex::new(());
    GPU.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Contexto OpenGL 3.3 Core sem superfície + renderer + alvo.
struct Harness {
    _context: PossiblyCurrentContext,
    gl: Arc<glow::Context>,
    renderer: GlRenderer,
    target: GlTarget,
    extractor: SceneExtractor,
    width: u32,
    height: u32,
}

fn headless_context() -> Result<(PossiblyCurrentContext, Arc<glow::Context>), String> {
    let devices = Device::query_devices().map_err(|error| format!("EGL devices: {error}"))?;
    let mut last_error = String::from("nenhum dispositivo EGL");
    for device in devices {
        // SAFETY: no raw display handle is passed (EGL device platform).
        let display = match unsafe { Display::with_device(&device, None) } {
            Ok(display) => display,
            Err(error) => {
                last_error = format!("EGL display: {error}");
                continue;
            }
        };
        let template = ConfigTemplateBuilder::new()
            .with_surface_type(ConfigSurfaceTypes::empty())
            .build();
        // SAFETY: the display is valid for the duration of this call.
        let config = match unsafe { display.find_configs(template) }
            .ok()
            .and_then(|mut configs| configs.next())
        {
            Some(config) => config,
            None => {
                last_error = "EGL sem config".into();
                continue;
            }
        };
        let attributes = ContextAttributesBuilder::new()
            .with_profile(GlProfile::Core)
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 3))))
            .build(None);
        // SAFETY: config belongs to display; no window handle is involved.
        let context = match unsafe { display.create_context(&config, &attributes) } {
            Ok(context) => context,
            Err(error) => {
                last_error = format!("EGL context: {error}");
                continue;
            }
        };
        let context = match context.make_current_surfaceless() {
            Ok(context) => context,
            Err(error) => {
                last_error = format!("EGL surfaceless: {error}");
                continue;
            }
        };
        // SAFETY: the context is current on this thread.
        let gl = unsafe {
            glow::Context::from_loader_function_cstr(|name| display.get_proc_address(name))
        };
        return Ok((context, Arc::new(gl)));
    }
    Err(last_error)
}

impl Harness {
    fn new(width: u32, height: u32) -> Option<Self> {
        Self::with_samples(width, height, 4)
    }

    fn with_samples(width: u32, height: u32, samples: u32) -> Option<Self> {
        let (context, gl) = match headless_context() {
            Ok(pair) => pair,
            Err(error) => {
                if std::env::var_os("PETUNIA_REQUIRE_GL").is_some() {
                    panic!("PETUNIA_REQUIRE_GL: {error}");
                }
                eprintln!("SKIPPED: sem contexto OpenGL ({error})");
                return None;
            }
        };
        let renderer = GlRenderer::new(Arc::clone(&gl)).expect("renderer GL 3.3");
        // SAFETY: the context is current on this thread.
        let target = unsafe { GlTarget::with_samples(&gl, width, height, samples) }.expect("alvo");
        Some(Self {
            _context: context,
            gl,
            renderer,
            target,
            extractor: SceneExtractor::new(),
            width,
            height,
        })
    }

    fn viewport(&self) -> PhysicalViewport {
        PhysicalViewport {
            x: 0,
            y: 0,
            width: self.width,
            height: self.height,
        }
    }

    fn camera(&self, preset: ViewPreset, half_height: f32) -> Camera {
        let mut camera = Camera::default();
        camera.set_preset(preset);
        camera.target = Vec3::ZERO;
        camera.set_projection(Projection::Ortho);
        camera.ortho_half_h = half_height;
        camera.aspect = self.width as f32 / self.height as f32;
        camera
    }

    /// Extrai e desenha; devolve os pixels (linhas de cima para baixo).
    fn render<'a>(
        &mut self,
        parts: impl FnOnce(&mut SceneParts<'a>),
        project: &'a Project,
        camera: &'a Camera,
    ) -> Image {
        let frame = self.frame(parts, project, camera);
        self.draw(&frame)
    }

    fn frame<'a>(
        &mut self,
        parts: impl FnOnce(&mut SceneParts<'a>),
        project: &'a Project,
        camera: &'a Camera,
    ) -> OwnedRenderFrame {
        let mut scene = SceneParts::new(project, camera, self.viewport());
        scene.settings = ViewportRenderSettings {
            show_grid: false,
            show_axes: false,
            ..ViewportRenderSettings::default()
        };
        scene.studio_light_follows_camera = false;
        parts(&mut scene);
        self.extractor.extract_owned(scene)
    }

    fn draw(&mut self, frame: &OwnedRenderFrame) -> Image {
        self.renderer.draw_into(&self.target, &frame.as_input());
        // SAFETY: the context is current on this thread.
        let raw = unsafe { self.target.read_rgba(&self.gl) };
        Image::from_bottom_up(raw, self.width, self.height)
    }
}

/// Imagem RGBA8 com origem no topo-esquerdo.
struct Image {
    pixels: Vec<u8>,
    width: u32,
    height: u32,
}

impl Image {
    fn from_bottom_up(raw: Vec<u8>, width: u32, height: u32) -> Self {
        let row = width as usize * 4;
        let mut pixels = Vec::with_capacity(raw.len());
        for y in (0..height as usize).rev() {
            pixels.extend_from_slice(&raw[y * row..(y + 1) * row]);
        }
        Self {
            pixels,
            width,
            height,
        }
    }

    fn at(&self, x: u32, y: u32) -> [u8; 4] {
        let i = (y as usize * self.width as usize + x as usize) * 4;
        [
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ]
    }

    fn center(&self) -> [u8; 4] {
        self.at(self.width / 2, self.height / 2)
    }

    fn row(&self, y: u32) -> Vec<[u8; 4]> {
        (0..self.width).map(|x| self.at(x, y)).collect()
    }
}

fn luminance(pixel: [u8; 4]) -> f32 {
    0.2126 * f32::from(pixel[0]) + 0.7152 * f32::from(pixel[1]) + 0.0722 * f32::from(pixel[2])
}

/// Dois quads lado a lado (aresta compartilhada em x = 0), de frente.
fn two_quads() -> Project {
    let mut mesh = petunia_core::Mesh::default();
    for [x, y] in [
        [-2.0f32, -1.0],
        [0.0, -1.0],
        [2.0, -1.0],
        [2.0, 1.0],
        [0.0, 1.0],
        [-2.0, 1.0],
    ] {
        mesh.verts.push(Vertex::new(x, y, 0.0));
    }
    mesh.faces.push(Face::new(vec![0, 1, 4, 5]));
    mesh.faces.push(Face::new(vec![1, 2, 3, 4]));
    let mut project = Project::default();
    project.add("Quads", mesh);
    project
}

fn cube_project() -> Project {
    let mut project = Project::default();
    project.add("Cube", petunia_core::Mesh::cube(2.0));
    project
}

/// Pixels escuros da aresta central na linha do meio.
fn center_edge_width(
    harness: &mut Harness,
    width: f32,
    half_height: f32,
    edge_mode: EdgeMode,
    overlay: bool,
    pixel_ratio: f32,
) -> usize {
    let project = two_quads();
    let camera = harness.camera(ViewPreset::Front, half_height);
    let image = harness.render(
        |parts| {
            parts.settings.line_width_px = width;
            parts.settings.edge_mode = edge_mode;
            parts.settings.show_wireframe_overlay = overlay;
            parts.settings.pixel_ratio = pixel_ratio;
        },
        &project,
        &camera,
    );
    let row: Vec<f32> = image
        .row(image.height / 2)
        .into_iter()
        .map(luminance)
        .collect();
    let center = image.width as usize / 2;
    // Face perto do centro: afastado, os quads ocupam só ±20 px.
    let face = row[center - 12];
    (center - 8..center + 8)
        .filter(|&x| row[x] < face * 0.6)
        .count()
}

#[test]
fn context_is_core_33_and_clears_to_the_palette() {
    let _gpu = gpu_guard();
    let Some(mut harness) = Harness::new(64, 48) else {
        return;
    };
    let caps = harness.renderer.caps().clone();
    assert!(caps.supports_core_33(), "{}", caps.summary());
    println!("{}", caps.summary());
    let camera = harness.camera(ViewPreset::Front, 2.0);
    let image = harness.render(
        |parts| parts.palette.background = [1.0, 0.0, 0.0, 1.0],
        &Project::default(),
        &camera,
    );
    assert_eq!(image.center(), [255, 0, 0, 255]);
}

#[test]
fn edges_have_constant_pixel_width_in_any_zoom() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(240, 120) else {
        return;
    };
    // A aresta do meio é comum (0,67 da base): 3 px → 2 px, 6 px → 4 px.
    let thin = center_edge_width(&mut h, 3.0, 2.0, EdgeMode::Overlay, true, 1.0);
    let thick = center_edge_width(&mut h, 6.0, 2.0, EdgeMode::Overlay, true, 1.0);
    assert!((1..=3).contains(&thin), "3 px → {thin}");
    assert!((3..=5).contains(&thick), "6 px → {thick}");
    assert!(thick > thin);
    let zoomed_out = center_edge_width(&mut h, 6.0, 6.0, EdgeMode::Overlay, true, 1.0);
    assert_eq!(zoomed_out, thick, "largura em pixels, não em mundo");
}

#[test]
fn hidpi_ratio_scales_edge_width_in_physical_pixels() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(240, 120) else {
        return;
    };
    let logical = center_edge_width(&mut h, 3.0, 2.0, EdgeMode::Overlay, true, 1.0);
    let doubled = center_edge_width(&mut h, 3.0, 2.0, EdgeMode::Overlay, true, 2.0);
    assert!(
        doubled >= logical * 2 - 1 && doubled > logical,
        "{logical} → {doubled}"
    );
}

#[test]
fn draw_reads_shape_and_poly_reads_topology() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(240, 120) else {
        return;
    };
    // A aresta do meio é plana: não é de feição.
    assert_eq!(
        center_edge_width(&mut h, 3.0, 2.0, EdgeMode::Features, false, 1.0),
        0
    );
    assert_eq!(
        center_edge_width(&mut h, 3.0, 2.0, EdgeMode::Topology, false, 1.0),
        0,
        "POLY sem overlay esconde a aresta plana (toggle funciona)"
    );
    assert_eq!(
        center_edge_width(&mut h, 3.0, 2.0, EdgeMode::Overlay, false, 1.0),
        0
    );
    assert!(center_edge_width(&mut h, 3.0, 2.0, EdgeMode::Features, true, 1.0) >= 1);
    assert!(center_edge_width(&mut h, 3.0, 2.0, EdgeMode::Topology, true, 1.0) >= 1);
}

#[test]
fn workplane_highlight_tints_the_plane_under_the_camera() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(160, 120) else {
        return;
    };
    let mut camera = h.camera(ViewPreset::Top, 4.0);
    camera.set_projection(Projection::Perspective);
    let project = Project::default();
    let render = |h: &mut Harness, workplane: Option<WorkplaneOverlay>| {
        let image = h.render(|parts| parts.workplane = workplane, &project, &camera);
        // Meio de uma célula, fora das linhas da grade do recorte.
        let p = image.at(h.width / 2 + 5, h.height / 2 + 5);
        [p[0], p[1], p[2]]
    };
    let plain = render(&mut h, None);
    let highlighted = render(
        &mut h,
        Some(WorkplaneOverlay {
            origin: [0.0; 3],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 0.0, -1.0],
        }),
    );
    assert_ne!(plain, highlighted, "o recorte precisa aparecer");
    assert!(
        highlighted[2] > plain[2],
        "tinta azul: {plain:?} → {highlighted:?}"
    );
    assert_eq!(render(&mut h, None), plain, "desligar remove o destaque");
}

fn cube_row(h: &mut Harness, outline: bool, half_height: f32) -> Vec<[u8; 4]> {
    let project = Project::new();
    let cube = project.assets[0].id;
    let camera = h.camera(ViewPreset::Front, half_height);
    let selected = [cube];
    let image = h.render(
        |parts| {
            if outline {
                parts.outlined = &selected;
                parts.outlined_active = Some(cube);
            }
            parts.components.thickness = 2.0;
        },
        &project,
        &camera,
    );
    image.row(image.height / 2)
}

fn outline_ring(h: &mut Harness, half_height: f32) -> (usize, bool) {
    let plain = cube_row(h, false, half_height);
    let outlined = cube_row(h, true, half_height);
    let background = plain[plain.len() - 1];
    let center = plain.len() / 2;
    let edge = (center..plain.len())
        .find(|&x| plain[x] == background)
        .expect("borda do cubo");
    let ring = (center..plain.len())
        .filter(|&x| outlined[x] != plain[x])
        .count();
    let interior_intact = (center..edge - 2).all(|x| outlined[x] == plain[x]);
    (ring, interior_intact)
}

#[test]
fn selected_objects_get_a_constant_width_outline() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(200, 120) else {
        return;
    };
    let (ring, interior_intact) = outline_ring(&mut h, 3.0);
    assert!((2..=4).contains(&ring), "2 px de contorno → {ring}");
    assert!(interior_intact, "o contorno fica fora da silhueta");
    let (far_ring, _) = outline_ring(&mut h, 8.0);
    assert!(
        far_ring.abs_diff(ring) <= 1,
        "largura em pixels: {ring} × {far_ring}"
    );
    assert!(h.renderer.last_frame().outline);
}

/// Vale em V aberto para a câmera Front: a dobra (x = 0) é côncava.
fn valley() -> Project {
    let mut mesh = petunia_core::Mesh::default();
    for [x, y, z] in [
        [-2.0f32, -2.0, 0.0],
        [0.0, -2.0, -1.5],
        [2.0, -2.0, 0.0],
        [2.0, 2.0, 0.0],
        [0.0, 2.0, -1.5],
        [-2.0, 2.0, 0.0],
    ] {
        mesh.verts.push(Vertex::new(x, y, z));
    }
    mesh.faces.push(Face::new(vec![0, 1, 4, 5]));
    mesh.faces.push(Face::new(vec![1, 2, 3, 4]));
    let mut project = Project::default();
    project.add("Valley", mesh);
    project
}

#[test]
fn ambient_occlusion_darkens_creases_but_not_open_faces() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(400, 240) else {
        return;
    };
    let project = valley();
    let mut camera = h.camera(ViewPreset::Front, 1.5);
    camera.target = Vec3::new(0.0, 0.0, -0.75);
    let row = |h: &mut Harness, ao: bool| -> Vec<f32> {
        let image = h.render(
            |parts| parts.settings.ambient_occlusion = ao,
            &project,
            &camera,
        );
        image
            .row(image.height / 2)
            .into_iter()
            .map(luminance)
            .collect()
    };
    let off = row(&mut h, false);
    assert!(!h.renderer.last_frame().ambient_occlusion);
    let on = row(&mut h, true);
    assert!(
        h.renderer.last_frame().ambient_occlusion,
        "passes de AO aplicados"
    );
    let center = h.width as usize / 2;
    let crease = (center - 1..=center + 1)
        .map(|x| on[x] / off[x].max(1.0))
        .fold(f32::MAX, f32::min);
    assert!(crease < 0.95, "a dobra escurece: razão {crease:.3}");
    for x in [center - 140, center + 140] {
        let ratio = on[x] / off[x].max(1.0);
        assert!(ratio > 0.96, "face aberta em x={x}: razão {ratio:.3}");
    }
}

#[test]
fn ambient_occlusion_is_skipped_in_xray_and_wireframe() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(120, 90) else {
        return;
    };
    let project = valley();
    let camera = h.camera(ViewPreset::Front, 2.5);
    for (xray, shading) in [(true, Shading::Solid), (false, Shading::Wireframe)] {
        h.render(
            |parts| {
                parts.settings.ambient_occlusion = true;
                parts.settings.xray = xray;
                parts.settings.shading = shading;
            },
            &project,
            &camera,
        );
        assert!(!h.renderer.last_frame().ambient_occlusion);
    }
}

#[test]
fn matcap_changes_solid_shading_and_follows_the_view() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(160, 120) else {
        return;
    };
    let project = cube_project();
    let sample = |h: &mut Harness, matcap: bool, preset| {
        let camera = h.camera(preset, 3.0);
        let image = h.render(
            |parts| {
                parts.settings.matcap = matcap;
                parts.settings.show_wireframe_overlay = false;
            },
            &project,
            &camera,
        );
        luminance(image.center())
    };
    let studio = sample(&mut h, false, ViewPreset::Front);
    let matcap_front = sample(&mut h, true, ViewPreset::Front);
    let matcap_back = sample(&mut h, true, ViewPreset::Back);
    assert!(
        (studio - matcap_front).abs() > 3.0,
        "{studio:.1} × {matcap_front:.1}"
    );
    assert!(
        (matcap_front - matcap_back).abs() < 2.0,
        "matcap em espaço de vista: {matcap_front:.1} × {matcap_back:.1}"
    );
}

#[test]
fn studio_light_following_the_camera_reads_the_same_from_any_side() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(160, 120) else {
        return;
    };
    let project = cube_project();
    let sides = |h: &mut Harness, follows: bool| {
        let mut out = [0.0; 2];
        for (side, preset) in [ViewPreset::Front, ViewPreset::Back]
            .into_iter()
            .enumerate()
        {
            let camera = h.camera(preset, 3.0);
            let image = h.render(
                |parts| {
                    parts.studio_light_follows_camera = follows;
                    parts.settings.show_wireframe_overlay = false;
                },
                &project,
                &camera,
            );
            out[side] = luminance(image.center());
        }
        (out[0], out[1])
    };
    let (front, back) = sides(&mut h, true);
    assert!(
        (front - back).abs() < 6.0,
        "frente {front:.1} × trás {back:.1}"
    );
    assert!(front > 120.0, "a face de frente fica clara: {front:.1}");
    let (front, back) = sides(&mut h, false);
    assert!(
        front - back > 30.0,
        "luz fixa: frente {front:.1} × trás {back:.1}"
    );
}

#[test]
fn screen_overlay_draws_on_top_without_depth_test() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(160, 120) else {
        return;
    };
    let project = cube_project();
    let camera = h.camera(ViewPreset::Front, 3.0);
    let square = [OverlayShape {
        points: vec![[70.0, 50.0], [90.0, 50.0], [90.0, 70.0], [70.0, 70.0]],
        closed: true,
        fill: Some([1.0, 0.0, 0.0, 1.0]),
        stroke: None,
    }];
    let image = h.render(|parts| parts.screen_overlay = &square, &project, &camera);
    assert!(h.renderer.last_frame().screen_overlay_vertices > 0);
    let pixel = image.center();
    assert!(
        pixel[0] > 240 && pixel[1] < 15 && pixel[2] < 15,
        "{pixel:?}"
    );
    let image = h.render(|_| {}, &project, &camera);
    assert_eq!(h.renderer.last_frame().screen_overlay_vertices, 0);
    let pixel = image.center();
    assert!(
        !(pixel[0] > 240 && pixel[1] < 15),
        "overlay removido: {pixel:?}"
    );
}

#[test]
fn face_orientation_colors_front_blue_and_back_red() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(120, 90) else {
        return;
    };
    let project = two_quads();
    for (preset, blue_wins) in [(ViewPreset::Front, true), (ViewPreset::Back, false)] {
        let camera = h.camera(preset, 2.0);
        let image = h.render(
            |parts| {
                parts.settings.show_face_orientation = true;
                parts.settings.show_wireframe_overlay = false;
            },
            &project,
            &camera,
        );
        let [r, _, b, _] = image.at(h.width / 2 - 20, h.height / 2);
        assert_eq!(b > r, blue_wins, "{preset:?}: r={r} b={b}");
    }
}

#[test]
fn xray_lets_the_background_show_through() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(120, 90) else {
        return;
    };
    let project = cube_project();
    let camera = h.camera(ViewPreset::Front, 3.0);
    let draw = |h: &mut Harness, xray: bool| {
        let image = h.render(
            |parts| {
                parts.palette.background = [0.0, 0.0, 1.0, 1.0];
                parts.settings.xray = xray;
                parts.settings.xray_opacity = 0.4;
                parts.settings.show_wireframe_overlay = false;
            },
            &project,
            &camera,
        );
        image.center()
    };
    let solid = draw(&mut h, false);
    let xray = draw(&mut h, true);
    // Frente e fundo do cubo, cada um a 40 %: ~36 % do fundo azul atravessa.
    assert!(
        xray[2] > solid[2] + 15 && xray[0] + 30 < solid[0],
        "fundo azul atravessa: {solid:?} → {xray:?}"
    );
    assert_eq!(xray[3], 255, "o alvo composto pelo Slint continua opaco");
}

#[test]
fn flat_smooth_and_sharp_shading_follow_the_surface_decision() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(200, 150) else {
        return;
    };
    let mut camera = h.camera(ViewPreset::Front, 2.0);
    camera.set_projection(Projection::Perspective);
    camera.yaw += 0.6;
    camera.pitch += 0.4;
    let spread = |h: &mut Harness, project: &Project| {
        let image = h.render(
            |parts| parts.settings.show_wireframe_overlay = false,
            project,
            &camera,
        );
        // Variação de brilho dentro da face mais próxima do centro.
        let (cx, cy) = (h.width / 2, h.height / 2);
        let samples = [
            luminance(image.at(cx - 6, cy)),
            luminance(image.at(cx + 6, cy)),
            luminance(image.at(cx, cy - 6)),
            luminance(image.at(cx, cy + 6)),
        ];
        let (min, max) = samples
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        max - min
    };
    let mut project = cube_project();
    let id = project.assets[0].id;
    let flat = spread(&mut h, &project);
    // Caminho canônico: muda a decisão e avança a revisão de normais.
    assert!(project.set_smooth_shaded(id, true));
    let smooth = spread(&mut h, &project);
    assert!(flat < 1.5, "Flat: face chapada ({flat:.2})");
    assert!(
        smooth > flat + 1.0,
        "Smooth: gradiente dentro da face ({smooth:.2})"
    );
    // Todas as arestas Sharp: o Smooth volta a ler como facetado.
    let mesh = project.active_mesh_mut().unwrap();
    for (a, b) in mesh.edges_unique() {
        mesh.sharp_edges.insert((a.min(b), a.max(b)));
    }
    project.bump_normals();
    let sharp = spread(&mut h, &project);
    assert!(
        sharp < 1.5,
        "Sharp limita as smoothing islands ({sharp:.2})"
    );
}

#[test]
fn component_overlays_change_without_reuploading_the_surface() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(160, 120) else {
        return;
    };
    let mut project = cube_project();
    let camera = h.camera(ViewPreset::Front, 3.0);
    let plain = h.render(
        |parts| parts.components.domain = SelectionDomain::Face,
        &project,
        &camera,
    );
    let uploads = h.renderer.stats.surface_uploads;
    // Hover em cada face e depois seleção de todas: só a camada de componentes muda.
    let faces = project.assets[0].mesh.faces.len();
    let hover_changes = (0..faces)
        .filter(|&face| {
            let hovered = h.render(
                |parts| {
                    parts.components.domain = SelectionDomain::Face;
                    parts.components.hover = HoverTarget::Face(face);
                },
                &project,
                &camera,
            );
            hovered.pixels != plain.pixels
        })
        .count();
    for face in &mut project.active_mesh_mut().unwrap().faces {
        face.selected = true;
    }
    // Caminho canônico: seleção nova avança a revisão de seleção.
    project.bump_selection();
    let selected = h.render(
        |parts| parts.components.domain = SelectionDomain::Face,
        &project,
        &camera,
    );
    assert_eq!(
        h.renderer.stats.surface_uploads, uploads,
        "seleção não reenvia a malha"
    );
    assert!(hover_changes >= 1, "a face sob o cursor ganha preselection");
    assert!(
        selected.pixels != plain.pixels,
        "faces selecionadas ganham o tom da seleção"
    );
}

#[test]
fn vertex_domain_draws_round_markers_of_constant_size() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(200, 150) else {
        return;
    };
    let project = two_quads();
    let marker_pixels = |h: &mut Harness, half_height: f32| {
        let camera = h.camera(ViewPreset::Front, half_height);
        let image = h.render(
            |parts| {
                parts.palette.background = [0.0, 0.0, 0.0, 1.0];
                parts.settings.shading = Shading::Wireframe;
                parts.components.domain = SelectionDomain::Vertex;
                parts.components.hover = HoverTarget::Vertex(1);
            },
            &project,
            &camera,
        );
        // Cor de hover (#7DDCFF) ao redor do vértice 1 = (0, -1).
        image
            .pixels
            .chunks(4)
            .filter(|p| p[2] > 200 && p[1] > 160 && p[0] < 160)
            .count()
    };
    let near = marker_pixels(&mut h, 2.0);
    let far = marker_pixels(&mut h, 6.0);
    assert!(near > 40, "disco de hover visível: {near}");
    let ratio = near as f32 / far.max(1) as f32;
    assert!((0.7..1.4).contains(&ratio), "tamanho em px: {near} × {far}");
}

#[test]
fn regional_texture_update_uploads_only_the_dirty_rect() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(160, 120) else {
        return;
    };
    let mut project = Project::new();
    let asset_id = project.assets[0].id;
    project.assets[0].texture = Some(Canvas::new(64, 64, [0, 0, 0, 255]));
    let camera = h.camera(ViewPreset::Front, 3.0);
    h.render(
        |parts| parts.settings.shading = Shading::MaterialPreview,
        &project,
        &camera,
    );
    let before = h.renderer.stats;
    let region = TextureDirtyRect {
        x: 7,
        y: 11,
        width: 4,
        height: 3,
    };
    for y in region.y..region.y + region.height {
        for x in region.x..region.x + region.width {
            project.assets[0]
                .texture
                .as_mut()
                .unwrap()
                .set(x, y, [255, 0, 0, 255]);
        }
    }
    project.bump_textures();
    let updates = [TextureUpdate {
        asset_id,
        canvas_width: 64,
        canvas_height: 64,
        regions: Some(vec![region]),
    }];
    h.render(
        |parts| {
            parts.settings.shading = Shading::MaterialPreview;
            parts.texture_updates = &updates;
        },
        &project,
        &camera,
    );
    let after = h.renderer.stats;
    assert_eq!(
        after.surface_uploads, before.surface_uploads,
        "malha intacta"
    );
    assert_eq!(after.full_texture_uploads, before.full_texture_uploads);
    assert_eq!(
        after.partial_texture_uploads,
        before.partial_texture_uploads + 1
    );
    assert_eq!(
        after.texture_bytes_uploaded,
        before.texture_bytes_uploaded + region.byte_len()
    );
}

#[test]
fn msaa_is_preserved_and_smooths_edges() {
    let _gpu = gpu_guard();
    let Some(mut msaa) = Harness::with_samples(120, 90, 4) else {
        return;
    };
    let mut camera = msaa.camera(ViewPreset::Front, 3.0);
    camera.yaw += 0.35;
    let project = cube_project();
    let partials = |h: &mut Harness| {
        let image = h.render(
            |parts| {
                parts.palette.background = [0.0, 0.0, 0.0, 1.0];
                parts.settings.show_wireframe_overlay = false;
            },
            &project,
            &camera,
        );
        image
            .row(image.height / 2)
            .windows(2)
            .filter(|w| {
                let (a, b) = (luminance(w[0]), luminance(w[1]));
                (a - b).abs() > 2.0
            })
            .count()
    };
    assert!(msaa.renderer.last_frame().samples <= 4);
    let with_msaa = partials(&mut msaa);
    assert!(
        msaa.target.samples() > 1,
        "MSAA disponível: {}",
        msaa.target.samples()
    );
    drop(msaa);
    let Some(mut plain) = Harness::with_samples(120, 90, 1) else {
        return;
    };
    let without = partials(&mut plain);
    assert!(with_msaa >= without, "{with_msaa} ≥ {without}");
}

#[test]
fn reference_images_draw_behind_and_in_front() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(120, 90) else {
        return;
    };
    let project = cube_project();
    let camera = h.camera(ViewPreset::Front, 3.0);
    let mut image =
        petunia_core::ReferenceImage::from_rgba("green".into(), 2, 2, [0, 255, 0, 255].repeat(4));
    image.size = 8.0;
    image.opacity = 1.0;
    image.visible = true;
    image.revision = 1;
    image.axis = petunia_core::RefAxis::Front;
    for (xray, green_center) in [(false, false), (true, true)] {
        image.xray = xray;
        let refs = [image.clone()];
        let frame = h.render(
            |parts| {
                parts.refs = &refs;
                parts.settings.show_wireframe_overlay = false;
            },
            &project,
            &camera,
        );
        let [r, g, b, _] = frame.center();
        assert_eq!(
            g > r + 100 && g > b + 100,
            green_center,
            "xray={xray}: {r},{g},{b}"
        );
        // Fora do cubo, a referência aparece nos dois passes.
        let [r, g, _, _] = frame.at(4, 4);
        assert!(g > r + 100, "borda: {r},{g}");
    }
}

#[test]
fn draw_restores_the_host_gl_state() {
    use glow::HasContext;
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(64, 48) else {
        return;
    };
    let gl = Arc::clone(&h.gl);
    // SAFETY: the context is current on this thread.
    unsafe {
        let host_fbo = gl.create_framebuffer().unwrap();
        gl.bind_framebuffer(glow::FRAMEBUFFER, Some(host_fbo));
        gl.viewport(3, 4, 21, 22);
        gl.enable(glow::BLEND);
        gl.blend_func(glow::ONE, glow::ZERO);
        gl.enable(glow::STENCIL_TEST);
        let camera = h.camera(ViewPreset::Front, 3.0);
        let project = Project::new();
        let selected = [project.assets[0].id];
        let frame = h.frame(
            |parts| {
                parts.outlined = &selected;
                parts.settings.ambient_occlusion = true;
            },
            &project,
            &camera,
        );
        h.renderer.draw_into(&h.target, &frame.as_input());
        assert_eq!(
            NonZeroU32::new(gl.get_parameter_i32(glow::DRAW_FRAMEBUFFER_BINDING) as u32),
            Some(host_fbo.0)
        );
        let mut viewport = [0; 4];
        gl.get_parameter_i32_slice(glow::VIEWPORT, &mut viewport);
        assert_eq!(viewport, [3, 4, 21, 22]);
        assert!(gl.is_enabled(glow::BLEND));
        assert!(gl.is_enabled(glow::STENCIL_TEST));
        assert_eq!(gl.get_parameter_i32(glow::BLEND_SRC_RGB) as u32, glow::ONE);
        assert_eq!(gl.get_parameter_i32(glow::BLEND_DST_RGB) as u32, glow::ZERO);
        assert_eq!(gl.get_error(), glow::NO_ERROR);
        gl.bind_framebuffer(glow::FRAMEBUFFER, None);
        gl.delete_framebuffer(host_fbo);
    }
}

#[test]
fn rigid_preview_moves_the_object_without_reuploading_it() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(160, 120) else {
        return;
    };
    let mut project = two_quads();
    let id = project.assets[0].id;
    let camera = h.camera(ViewPreset::Front, 4.0);
    let first = glam::Mat4::IDENTITY;
    let previews = [(id, first)];
    let start = h.render(
        |parts| {
            parts.rigid_previews = &previews;
            parts.palette.background = [0.0; 4];
        },
        &project,
        &camera,
    );
    let uploads = h.renderer.stats.surface_uploads;
    for vertex in &mut project.active_mesh_mut().unwrap().verts {
        vertex.pos[1] += 1.5;
    }
    let moved = [(id, glam::Mat4::from_translation(Vec3::Y * 1.5))];
    let after = h.render(
        |parts| {
            parts.rigid_previews = &moved;
            parts.palette.background = [0.0; 4];
        },
        &project,
        &camera,
    );
    assert_eq!(
        h.renderer.stats.surface_uploads, uploads,
        "só a matriz muda"
    );
    // O quad subiu: a linha de baixo (y = -0.75 no mundo) fica vazia.
    let probe_y = h.height / 2 + (0.75 / 4.0 * h.height as f32 / 2.0) as u32;
    assert!(luminance(start.at(h.width / 2 - 30, probe_y)) > 20.0);
    assert!(luminance(after.at(h.width / 2 - 30, probe_y)) < 5.0);
}

#[test]
fn painted_texture_shows_in_solid_shading_even_when_it_appears_later() {
    // QA de 2026-10-11: no PAINT o canvas tinha o traço, mas o cubo seguia
    // cinza. Sombreamento Solid + `textured` (o padrão do PAINT) precisa
    // amostrar a textura, inclusive quando ela nasce depois do 1º quadro.
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(160, 120) else {
        return;
    };
    let mut project = cube_project();
    let camera = h.camera(ViewPreset::Front, 3.0);
    let textured = |parts: &mut SceneParts<'_>| {
        parts.settings.textured = true;
        parts.settings.show_wireframe_overlay = false;
    };
    let plain = h.render(textured, &project, &camera).center();

    let index = project.assets.len() - 1;
    project.assets[index].texture = Some(Canvas::new(32, 32, [230, 20, 20, 255]));
    project.bump_textures();
    let red = h.render(textured, &project, &camera).center();
    assert!(
        i32::from(red[0]) > i32::from(red[1]) + 60,
        "a textura vermelha aparece: antes {plain:?}, depois {red:?}"
    );
}

#[test]
fn painted_face_shows_in_render() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(160, 120) else {
        return;
    };
    let mut project = cube_project();
    let camera = h.camera(ViewPreset::Front, 3.0);
    let textured = |parts: &mut SceneParts<'_>| {
        parts.settings.textured = true;
        parts.settings.show_wireframe_overlay = false;
    };
    let plain = h.render(textured, &project, &camera).center();

    let index = project.assets.len() - 1;
    let mut canvas = petunia_project::Canvas::new(128, 128, [191, 191, 198, 255]);
    let hit = glam::Vec3::new(0.0, 0.0, 1.0);
    let mut painted_pixels = 0;
    project.assets[index]
        .mesh
        .rasterize_face_near(1, hit, 0.5, 128, 128, 0.0, |x, y, _, _, _| {
            let idx = ((y * 128 + x) * 4) as usize;
            canvas.pixels[idx..idx + 4].copy_from_slice(&[255, 0, 0, 255]);
            painted_pixels += 1;
        });
    assert!(painted_pixels > 0, "rasterize_face_near deve pintar pixels");
    project.assets[index].texture = Some(canvas);
    project.bump_textures();
    let painted = h.render(textured, &project, &camera).center();
    assert!(
        i32::from(painted[0]) > i32::from(painted[1]) + 60,
        "a face pintada de vermelho precisa renderizar vermelho! antes {plain:?}, depois {painted:?}"
    );
}

/// Caixa envolvente (x0, y0, x1, y1) dos pixels que diferem de `base`.
fn changed_bounds(a: &Image, b: &Image) -> Option<(u32, u32, u32, u32)> {
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for y in 0..a.height {
        for x in 0..a.width {
            if a.at(x, y) != b.at(x, y) {
                bounds = Some(match bounds {
                    None => (x, y, x, y),
                    Some((x0, y0, x1, y1)) => (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
                });
            }
        }
    }
    bounds
}

/// Regressão: o contorno de seleção acompanha o objeto depois de mover/escalar
/// (sem "fantasma" da transformação anterior).
#[test]
fn selection_outline_follows_the_object_after_a_transform_edit() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(240, 240) else {
        return;
    };
    let mut project = cube_project();
    let id = project.assets[0].id;
    let camera = h.camera(ViewPreset::Front, 4.0);
    let selected = [id];
    // Estado inicial e depois da edição de transformação (como o painel faz).
    for (position, scale) in [
        ([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]),
        ([-0.17, 0.42, -0.16], [0.44, 2.75, 0.49]),
        ([0.5, -0.3, 0.0], [1.5, 0.5, 1.0]),
    ] {
        project.assets[0].position = position;
        project.assets[0].scale = scale;
        let without = h.render(
            |parts| parts.palette.background = [0.0; 4],
            &project,
            &camera,
        );
        let with = h.render(
            |parts| {
                parts.outlined = &selected;
                parts.outlined_active = Some(id);
                parts.palette.background = [0.0; 4];
            },
            &project,
            &camera,
        );
        let background = Image {
            pixels: without
                .at(0, 0)
                .repeat((without.width * without.height) as usize),
            width: without.width,
            height: without.height,
        };
        let (sx0, sy0, sx1, sy1) = changed_bounds(&without, &background).expect("o objeto aparece");
        let (ox0, oy0, ox1, oy1) = changed_bounds(&without, &with).expect("o contorno aparece");
        let slack = 6;
        assert!(
            ox0 + slack >= sx0 && oy0 + slack >= sy0 && ox1 <= sx1 + slack && oy1 <= sy1 + slack,
            "contorno {:?} fora da silhueta {:?} com escala {scale:?}",
            (ox0, oy0, ox1, oy1),
            (sx0, sy0, sx1, sy1)
        );
        assert!(
            ox0 <= sx0 + slack && oy0 <= sy0 + slack && ox1 + slack >= sx1 && oy1 + slack >= sy1,
            "contorno {:?} não cobre a silhueta {:?} com escala {scale:?}",
            (ox0, oy0, ox1, oy1),
            (sx0, sy0, sx1, sy1)
        );
    }
}

/// Pixels que mudaram entre duas imagens: (total, dominados pelo vermelho).
fn changed_pixels(base: &Image, other: &Image) -> (usize, usize) {
    let mut total = 0;
    let mut red = 0;
    for y in 0..base.height {
        for x in 0..base.width {
            if base.at(x, y) != other.at(x, y) {
                total += 1;
                let [r, g, b, _] = other.at(x, y);
                if r > g.saturating_add(40) && r > b.saturating_add(40) {
                    red += 1;
                }
            }
        }
    }
    (total, red)
}

/// O contorno usa a cor de seleção da própria parte (não a global), no ativo e
/// nos demais selecionados; o hover desenha um contorno discreto sem selecionar.
#[test]
fn outline_uses_each_parts_selection_color_and_draws_hover() {
    let _gpu = gpu_guard();
    let Some(mut h) = Harness::new(200, 120) else {
        return;
    };
    let mut project = cube_project();
    project.assets[0].selection_overlay_color = Some([255, 0, 0]);
    let id = project.assets[0].id;
    let camera = h.camera(ViewPreset::Front, 3.0);
    let selected = [id];
    let plain = h.render(
        |parts| parts.palette.background = [0.0; 4],
        &project,
        &camera,
    );
    let active = h.render(
        |parts| {
            parts.outlined = &selected;
            parts.outlined_active = Some(id);
            parts.palette.background = [0.0; 4];
        },
        &project,
        &camera,
    );
    let (total, red) = changed_pixels(&plain, &active);
    assert!(total > 0, "o contorno do ativo aparece");
    assert!(
        red * 4 >= total * 3,
        "o contorno segue a cor da parte (vermelha): {red} de {total} pixels"
    );

    // Selecionado, mas não ativo: variante mais escura da mesma cor da parte.
    let secondary = h.render(
        |parts| {
            parts.outlined = &selected;
            parts.outlined_active = None;
            parts.palette.background = [0.0; 4];
        },
        &project,
        &camera,
    );
    let (total, red) = changed_pixels(&plain, &secondary);
    assert!(total > 0 && red * 4 >= total * 3, "{red} de {total}");

    // Hover sem seleção: aparece um contorno.
    let hovered = h.render(
        |parts| {
            parts.components.hover = HoverTarget::Object(0);
            parts.palette.background = [0.0; 4];
        },
        &project,
        &camera,
    );
    let (hover_total, _) = changed_pixels(&plain, &hovered);
    assert!(hover_total > 0, "o hover do objeto desenha um contorno");
}
