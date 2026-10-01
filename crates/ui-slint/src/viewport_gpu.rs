//! Integração WGPU com o PetuniaViewport e Slint Image.

use std::sync::Arc;

use petunia_core::{Camera, SelectionDomain, Workspace};
use petunia_project::Project;
use petunia_render_wgpu::Renderer;

use crate::{PetuniaViewport, ViewportRenderState};

/// Amostras por pixel do viewport. `Rgba8Unorm` e `Depth24Plus` têm MSAA 4x
/// garantido pelo wgpu em todos os backends.
pub const VIEWPORT_MSAA_SAMPLES: u32 = 4;

/// Viewport acelerado por WGPU que renderiza a cena 3D para uma textura
/// off-screen e converte em [`slint::Image`].
pub struct WgpuViewport {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
    pub renderer: Renderer,
    pub width: u32,
    pub height: u32,
    pub workspace: Workspace,
    pub selection_domain: SelectionDomain,
    target_texture: Option<wgpu::Texture>,
    target_view: Option<wgpu::TextureView>,
    /// Alvo multisample resolvido em `target_texture` a cada frame.
    msaa_view: Option<wgpu::TextureView>,
}

impl WgpuViewport {
    /// Tenta criar um novo `WgpuViewport` inicializando adaptador e dispositivo padrão.
    pub fn try_create_default(width: u32, height: u32) -> Result<Self, &'static str> {
        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::all();
        let instance = wgpu::Instance::new(instance_desc);

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|_| "Nenhum adaptador WGPU disponível")?;

        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Petunia Slint Viewport Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: Default::default(),
            memory_hints: Default::default(),
            trace: Default::default(),
        }))
        .map_err(|_| "Falha ao criar dispositivo WGPU")?;

        Ok(Self::new(Arc::new(device), Arc::new(queue), width, height))
    }

    pub fn create_wgpu_context()
    -> Result<(wgpu::Instance, wgpu::Adapter, wgpu::Device, wgpu::Queue), &'static str> {
        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::all();
        let instance = wgpu::Instance::new(instance_desc);
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
            apply_limit_buckets: false,
        }))
        .map_err(|_| "Nenhum adaptador WGPU disponível")?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Petunia Slint Shared Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: Default::default(),
            memory_hints: Default::default(),
            trace: Default::default(),
        }))
        .map_err(|_| "Falha ao criar dispositivo WGPU")?;
        Ok((instance, adapter, device, queue))
    }

    /// Cria um viewport usando `Device` e `Queue` existentes.
    pub fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        width: u32,
        height: u32,
    ) -> Self {
        let renderer = Renderer::with_sample_count(
            &device,
            wgpu::TextureFormat::Rgba8Unorm,
            VIEWPORT_MSAA_SAMPLES,
        );
        let mut viewport = Self {
            device,
            queue,
            renderer,
            width: width.max(1),
            height: height.max(1),
            workspace: Workspace::Model,
            selection_domain: SelectionDomain::Object,
            target_texture: None,
            target_view: None,
            msaa_view: None,
        };
        viewport.recreate_target();
        viewport
    }

    pub fn recreate_target(&mut self) {
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Petunia Slint Viewport Target"),
            size: wgpu::Extent3d {
                width: self.width.max(1),
                height: self.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.target_texture = Some(texture);
        self.target_view = Some(view);
        let samples = self.renderer.sample_count();
        self.msaa_view = (samples > 1).then(|| {
            self.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Petunia Slint Viewport MSAA"),
                    size: wgpu::Extent3d {
                        width: self.width.max(1),
                        height: self.height.max(1),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: samples,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        });
        self.renderer.resize(&self.device, self.width, self.height);
    }

    /// Renderiza a cena atual para a textura do viewport e exporta como [`slint::Image`].
    pub fn render_frame(
        &mut self,
        project: &Project,
        refs: &[petunia_core::ReferenceImage],
        camera: &Camera,
        state: ViewportRenderState,
    ) -> Result<slint::Image, &'static str> {
        if self.target_texture.is_none() {
            self.recreate_target();
        }

        let texture = self.target_texture.as_ref().ok_or("Textura indisponível")?;
        let view = self
            .target_view
            .as_ref()
            .ok_or("TextureView indisponível")?;

        // O uniforme da câmera é escrito em `update`; a opacidade precisa
        // estar no renderer antes dessa escrita, inclusive no primeiro frame.
        self.renderer.set_xray_opacity(state.xray_opacity);
        self.renderer
            .set_studio_light_follows_camera(state.studio_light_follows_camera);
        self.renderer.set_edge_mode(state.edge_mode);
        self.renderer.set_workplane(state.workplane);
        self.renderer
            .set_selection_style(state.selection_rgb, state.selection_thickness);
        self.renderer.update(
            &self.device,
            &self.queue,
            project,
            refs,
            camera,
            state.shading,
            state.xray,
            state.show_triangulation,
            state.textured,
            state.show_wireframe_overlay,
            state.show_face_orientation,
            state.show_uv_checker,
            state.selection_domain,
            state.hover,
        );
        self.renderer.set_overlays(true, state.show_grid);
        self.renderer.upload_ref_pixels(&self.queue, refs);

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Slint Viewport Encoder"),
            });
        self.renderer
            .encode_selection_outline_mask(&self.queue, &mut encoder);

        {
            let depth_view = self.renderer.depth_view();
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Slint Viewport Pass"),
                // Com MSAA, desenha no alvo multisample e resolve na textura
                // exibida pelo Slint; as amostras não precisam ser guardadas.
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.msaa_view.as_ref().unwrap_or(view),
                    resolve_target: self.msaa_view.as_ref().map(|_| view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.082,
                            g: 0.086,
                            b: 0.098,
                            a: 1.0,
                        }),
                        store: if self.msaa_view.is_some() {
                            wgpu::StoreOp::Discard
                        } else {
                            wgpu::StoreOp::Store
                        },
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: depth_view.map(|dview| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: dview,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            self.renderer.render(&mut rpass, refs);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        slint::Image::try_from(texture.clone()).map_err(|_| "Viewport texture import failed")
    }
}

impl PetuniaViewport for WgpuViewport {
    fn resize(&mut self, width: u32, height: u32) {
        let limit = self.device.limits().max_texture_dimension_2d.max(1);
        let width = width.clamp(1, limit);
        let height = height.clamp(1, limit);
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.recreate_target();
        }
    }

    fn uses_physical_pixels(&self) -> bool {
        true
    }

    fn set_pixel_ratio(&mut self, ratio: f32) {
        self.renderer.set_pixel_ratio(ratio);
    }

    fn set_outlined_objects(&mut self, selected: &[uuid::Uuid], active: Option<uuid::Uuid>) {
        self.renderer.set_outlined_objects(selected, active);
    }

    fn update(&mut self, _dt_seconds: f32) {}

    fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
    }

    fn set_selection_domain(&mut self, domain: SelectionDomain) {
        self.selection_domain = domain;
    }

    fn queue_texture_updates(&mut self, updates: Vec<petunia_core::TextureUpdate>) {
        self.renderer.queue_texture_updates(updates);
    }

    fn draws_component_guides(&self) -> bool {
        true
    }

    fn set_pose_override(&mut self, pose: Option<Arc<petunia_project::PoseOverride>>) {
        self.renderer.set_pose_override(pose);
    }

    fn render_frame(
        &mut self,
        project: &Project,
        refs: &[petunia_core::ReferenceImage],
        camera: &Camera,
        state: ViewportRenderState,
    ) -> Option<slint::Image> {
        WgpuViewport::render_frame(self, project, refs, camera, state).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_core::{TextureDirtyRect, TextureUpdate};
    use petunia_project::Canvas;
    use petunia_render::Shading;

    /// Lê o pixel RGBA do centro da textura exibida (teste de aparência).
    fn center_pixel(viewport: &WgpuViewport) -> [u8; 4] {
        let pixels = read_pixels(viewport);
        let offset = (((viewport.height / 2) * viewport.width + viewport.width / 2) * 4) as usize;
        [
            pixels[offset],
            pixels[offset + 1],
            pixels[offset + 2],
            pixels[offset + 3],
        ]
    }

    /// Todos os pixels RGBA da textura exibida, linha a linha, sem padding.
    fn read_pixels(viewport: &WgpuViewport) -> Vec<u8> {
        let texture = viewport.target_texture.as_ref().expect("alvo");
        let bytes_per_row = (viewport.width * 4).div_ceil(256) * 256;
        let buffer = viewport.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("readback"),
            size: u64::from(bytes_per_row * viewport.height),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = viewport
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(viewport.height),
                },
            },
            wgpu::Extent3d {
                width: viewport.width,
                height: viewport.height,
                depth_or_array_layers: 1,
            },
        );
        viewport.queue.submit(std::iter::once(encoder.finish()));
        let slice = buffer.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        viewport
            .device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("poll");
        let data = slice.get_mapped_range().expect("mapeado");
        let row = (viewport.width * 4) as usize;
        (0..viewport.height as usize)
            .flat_map(|y| {
                let start = y * bytes_per_row as usize;
                data[start..start + row].to_vec()
            })
            .collect()
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
            mesh.verts.push(petunia_mesh::Vertex::new(x, y, 0.0));
        }
        mesh.faces.push(petunia_mesh::Face::new(vec![0, 1, 4, 5]));
        mesh.faces.push(petunia_mesh::Face::new(vec![1, 2, 3, 4]));
        // `Project::new()` traz o cubo padrão, que cobriria os quads.
        let mut project = Project::default();
        project.add("Quads", mesh);
        project
    }

    /// Pixels escuros da aresta central na linha do meio da imagem.
    fn center_edge_width(viewport: &mut WgpuViewport, width: f32, half_height: f32) -> usize {
        center_edge_width_in(
            viewport,
            width,
            half_height,
            petunia_render_wgpu::EdgeMode::Overlay,
            true,
        )
    }

    fn center_edge_width_in(
        viewport: &mut WgpuViewport,
        width: f32,
        half_height: f32,
        edge_mode: petunia_render_wgpu::EdgeMode,
        overlay: bool,
    ) -> usize {
        viewport.renderer.set_line_width_px(width);
        let mut camera = Camera::default();
        camera.set_preset(petunia_core::ViewPreset::Front);
        camera.target = glam::Vec3::ZERO;
        camera.ortho_half_h = half_height;
        let state = ViewportRenderState {
            show_grid: false,
            show_wireframe_overlay: overlay,
            edge_mode,
            ..ViewportRenderState::default()
        };
        viewport
            .render_frame(&two_quads(), &[], &camera, state)
            .unwrap();
        let pixels = read_pixels(viewport);
        let (width_px, y) = (viewport.width as usize, viewport.height as usize / 2);
        let row: Vec<f32> = (0..width_px)
            .map(|x| {
                let i = (y * width_px + x) * 4;
                luminance([pixels[i], pixels[i + 1], pixels[i + 2], 255])
            })
            .collect();
        let center = width_px / 2;
        // Amostra da face perto do centro: afastado, os quads ocupam só ±20 px.
        let face = row[center - 9];
        (center - 6..center + 6)
            .filter(|&x| row[x] < face * 0.6)
            .count()
    }

    fn luminance(pixel: [u8; 4]) -> f32 {
        0.2126 * f32::from(pixel[0]) + 0.7152 * f32::from(pixel[1]) + 0.0722 * f32::from(pixel[2])
    }

    /// Brilho do centro do cubo visto de frente e de trás.
    fn front_and_back_luminance(viewport: &mut WgpuViewport, follows: bool) -> (f32, f32) {
        let mut project = Project::new();
        project.add("Cube", petunia_core::Mesh::cube(2.0));
        let state = ViewportRenderState {
            show_grid: false,
            show_wireframe_overlay: false,
            studio_light_follows_camera: follows,
            ..ViewportRenderState::default()
        };
        let mut sides = [0.0; 2];
        for (side, preset) in [
            petunia_core::ViewPreset::Front,
            petunia_core::ViewPreset::Back,
        ]
        .into_iter()
        .enumerate()
        {
            let mut camera = Camera::default();
            camera.set_preset(preset);
            camera.target = glam::Vec3::ZERO;
            viewport
                .render_frame(&project, &[], &camera, state)
                .unwrap();
            sides[side] = luminance(center_pixel(viewport));
        }
        (sides[0], sides[1])
    }

    #[test]
    fn wireframe_edges_have_constant_pixel_width() {
        let Ok(mut viewport) = WgpuViewport::try_create_default(200, 120) else {
            return;
        };
        // A aresta do meio é comum (0,67 da base): 3 px → 2 px, 6 px → 4 px.
        let thin = center_edge_width(&mut viewport, 3.0, 2.0);
        let thick = center_edge_width(&mut viewport, 6.0, 2.0);
        assert!((1..=3).contains(&thin), "3 px → {thin}");
        assert!((3..=5).contains(&thick), "6 px → {thick}");
        assert!(thick > thin);
        // Aproximar ou afastar não muda a espessura na tela.
        let zoomed_out = center_edge_width(&mut viewport, 6.0, 6.0);
        assert_eq!(zoomed_out, thick, "largura em pixels, não em mundo");
    }

    #[test]
    fn draw_reads_shape_and_poly_reads_topology() {
        use petunia_render_wgpu::EdgeMode;
        let Ok(mut viewport) = WgpuViewport::try_create_default(200, 120) else {
            return;
        };
        // A aresta do meio é plana: não é de feição.
        let draw = center_edge_width_in(&mut viewport, 3.0, 2.0, EdgeMode::Features, false);
        let poly = center_edge_width_in(&mut viewport, 3.0, 2.0, EdgeMode::Topology, false);
        let paint = center_edge_width_in(&mut viewport, 3.0, 2.0, EdgeMode::Overlay, false);
        let overlay = center_edge_width_in(&mut viewport, 3.0, 2.0, EdgeMode::Features, true);
        assert_eq!(draw, 0, "DRAW esconde a aresta plana");
        assert_eq!(
            poly, 0,
            "POLY sem overlay esconde a aresta plana (toggle funciona)"
        );
        assert_eq!(paint, 0, "PAINT/UV: faces limpas sem overlay");
        assert!(overlay >= 1, "o overlay acrescenta as arestas finas");
        // Com overlay, POLY mostra a topologia completa.
        let poly_on = center_edge_width_in(&mut viewport, 3.0, 2.0, EdgeMode::Topology, true);
        assert!(poly_on >= 1, "POLY com overlay mostra a topologia");
        // Aresta comum é mais fina que a de feição na mesma largura base.
        let feature_like = center_edge_width_in(&mut viewport, 3.0, 2.0, EdgeMode::Overlay, true);
        assert!(poly_on <= feature_like, "{poly_on} ≤ {feature_like}");
    }

    #[test]
    fn workplane_highlight_tints_the_plane_under_the_camera() {
        let Ok(mut viewport) = WgpuViewport::try_create_default(160, 120) else {
            return;
        };
        let mut camera = Camera::default();
        camera.set_preset(petunia_core::ViewPreset::Top);
        camera.target = glam::Vec3::ZERO;
        let mut render = |workplane| {
            let state = ViewportRenderState {
                show_grid: false,
                workplane,
                ..ViewportRenderState::default()
            };
            viewport
                .render_frame(&Project::default(), &[], &camera, state)
                .unwrap();
            // Fora das linhas da grade do recorte: meio de uma célula.
            let pixels = read_pixels(&viewport);
            let (x, y) = (
                viewport.width as usize / 2 + 5,
                viewport.height as usize / 2 + 5,
            );
            let i = (y * viewport.width as usize + x) * 4;
            [pixels[i], pixels[i + 1], pixels[i + 2]]
        };
        let plain = render(None);
        let highlighted = render(Some(petunia_render_wgpu::WorkplaneOverlay {
            origin: [0.0, 0.0, 0.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 0.0, -1.0],
        }));
        assert_ne!(plain, highlighted, "o recorte precisa aparecer");
        assert!(
            highlighted[2] > plain[2],
            "tinta azul do plano: {plain:?} → {highlighted:?}"
        );
        // Desligar remove o destaque.
        assert_eq!(render(None), plain);
    }

    /// Linha do meio da imagem com o cubo padrão de frente; `outline` liga o
    /// contorno de seleção do cubo.
    fn cube_row(viewport: &mut WgpuViewport, outline: bool, half_height: f32) -> Vec<[u8; 3]> {
        let project = Project::new();
        let cube = project.assets[0].id;
        if outline {
            viewport.renderer.set_outlined_objects(&[cube], Some(cube));
        } else {
            viewport.renderer.set_outlined_objects(&[], None);
        }
        let mut camera = Camera::default();
        camera.set_preset(petunia_core::ViewPreset::Front);
        camera.target = glam::Vec3::ZERO;
        camera.ortho_half_h = half_height;
        let state = ViewportRenderState {
            show_grid: false,
            selection_thickness: 2.0,
            ..ViewportRenderState::default()
        };
        viewport
            .render_frame(&project, &[], &camera, state)
            .unwrap();
        let pixels = read_pixels(viewport);
        let (width, y) = (viewport.width as usize, viewport.height as usize / 2);
        (0..width)
            .map(|x| {
                let i = (y * width + x) * 4;
                [pixels[i], pixels[i + 1], pixels[i + 2]]
            })
            .collect()
    }

    /// Pixels do anel de contorno à direita do cubo (diferem da imagem sem
    /// contorno fora da silhueta) e se o interior ficou intacto.
    fn outline_ring(viewport: &mut WgpuViewport, half_height: f32) -> (usize, bool) {
        let plain = cube_row(viewport, false, half_height);
        let outlined = cube_row(viewport, true, half_height);
        let background = plain[plain.len() - 1];
        let center = plain.len() / 2;
        let edge = (center..plain.len())
            .find(|&x| plain[x] == background)
            .expect("borda do cubo");
        // O anel inclui o pixel suavizado da borda, que fica fora da máscara.
        let ring = (center..plain.len())
            .filter(|&x| outlined[x] != plain[x])
            .count();
        let interior_intact = (center..edge - 2).all(|x| outlined[x] == plain[x]);
        (ring, interior_intact)
    }

    #[test]
    fn selected_objects_get_a_constant_width_outline() {
        let Ok(mut viewport) = WgpuViewport::try_create_default(200, 120) else {
            return;
        };
        let (ring, interior_intact) = outline_ring(&mut viewport, 3.0);
        assert!((2..=3).contains(&ring), "2 px de contorno → {ring}");
        assert!(interior_intact, "o contorno fica fora da silhueta");
        // Afastar a câmera não afina nem engrossa o contorno.
        let (far_ring, _) = outline_ring(&mut viewport, 8.0);
        assert_eq!(far_ring, ring, "largura em pixels, não em mundo");
        // Sem seleção, nada muda.
        assert_eq!(
            cube_row(&mut viewport, false, 3.0),
            cube_row(&mut viewport, false, 3.0)
        );
    }

    #[test]
    fn studio_light_following_the_camera_reads_the_same_from_any_side() {
        let Ok(mut viewport) = WgpuViewport::try_create_default(160, 120) else {
            // Sem adaptador (nem lavapipe): nada a medir.
            return;
        };
        let (front, back) = front_and_back_luminance(&mut viewport, true);
        assert!(
            (front - back).abs() < 6.0,
            "frente {front:.1} × trás {back:.1}"
        );
        assert!(front > 120.0, "a face de frente fica clara: {front:.1}");

        // Luz fixa no mundo: o lado de trás fica só com a luz ambiente.
        let (front, back) = front_and_back_luminance(&mut viewport, false);
        assert!(front - back > 30.0, "frente {front:.1} × trás {back:.1}");
    }

    #[test]
    fn wgpu_viewport_initializes_or_skips_when_no_gpu() {
        match WgpuViewport::try_create_default(640, 480) {
            Ok(mut viewport) => {
                assert_eq!(viewport.width, 640);
                assert_eq!(viewport.height, 480);
                viewport.resize(800, 600);
                assert_eq!(viewport.width, 800);
                assert_eq!(viewport.height, 600);
                viewport.set_workspace(Workspace::Paint);
                assert_eq!(viewport.workspace, Workspace::Paint);

                let project = Project::new();
                let camera = Camera::default();
                let img = viewport.render_frame(
                    &project,
                    &[],
                    &camera,
                    ViewportRenderState {
                        shading: Shading::Solid,
                        xray: false,
                        show_triangulation: false,
                        textured: false,
                        show_wireframe_overlay: true,
                        show_face_orientation: false,
                        show_uv_checker: false,
                        selection_domain: petunia_core::SelectionDomain::Object,
                        xray_opacity: 0.42,
                        selection_rgb: [233, 106, 0],
                        selection_thickness: 2.0,
                        show_grid: true,
                        hover: petunia_core::HoverTarget::None,
                        boolean_operand: None,
                        studio_light_follows_camera: true,
                        edge_mode: petunia_render_wgpu::EdgeMode::Overlay,
                        workplane: None,
                    },
                );
                assert!(img.is_ok());
            }
            Err(err) => {
                println!("WgpuViewport ignorado por falta de GPU física: {err}");
            }
        }
    }

    #[test]
    fn hover_updates_selection_without_rebuilding_scene_geometry() {
        let Ok(mut viewport) = WgpuViewport::try_create_default(320, 240) else {
            // Os testes de domínio ainda executam em hosts sem Vulkan/Metal/DX.
            return;
        };
        let project = Project::new();
        let camera = Camera::default();
        let state = ViewportRenderState::default();
        viewport
            .render_frame(&project, &[], &camera, state)
            .unwrap();
        let rebuilt = viewport.renderer.mesh_rebuilds();
        viewport
            .render_frame(
                &project,
                &[],
                &camera,
                ViewportRenderState {
                    hover: petunia_core::HoverTarget::Face(0),
                    ..state
                },
            )
            .unwrap();
        assert_eq!(viewport.renderer.mesh_rebuilds(), rebuilt);
    }

    #[test]
    fn pose_override_rebuilds_geometry_only_when_its_revision_changes() {
        let Ok(mut viewport) = WgpuViewport::try_create_default(320, 240) else {
            // Sem adaptador físico: a lógica de revisão tem teste puro no renderer.
            return;
        };
        let project = Project::new();
        let asset_id = project.assets[0].id;
        let camera = Camera::default();
        let state = ViewportRenderState::default();
        viewport
            .render_frame(&project, &[], &camera, state)
            .unwrap();
        let base = viewport.renderer.mesh_rebuilds();

        let mut moved = project.assets[0].mesh.clone();
        for v in &mut moved.verts {
            v.pos[0] += 1.0;
        }
        let mut pose = petunia_project::PoseOverride::new(1);
        pose.insert(asset_id, moved);
        let pose = Arc::new(pose);

        viewport.set_pose_override(Some(Arc::clone(&pose)));
        viewport
            .render_frame(&project, &[], &camera, state)
            .unwrap();
        assert_eq!(
            viewport.renderer.mesh_rebuilds(),
            base + 1,
            "a pose aparece"
        );

        viewport.set_pose_override(Some(pose));
        viewport
            .render_frame(&project, &[], &camera, state)
            .unwrap();
        assert_eq!(viewport.renderer.mesh_rebuilds(), base + 1, "mesma revisão");

        viewport.set_pose_override(None);
        viewport
            .render_frame(&project, &[], &camera, state)
            .unwrap();
        assert_eq!(
            viewport.renderer.mesh_rebuilds(),
            base + 2,
            "volta ao repouso"
        );
    }

    #[test]
    fn regional_texture_update_avoids_mesh_rebuild_and_full_upload() {
        let Ok(mut viewport) = WgpuViewport::try_create_default(320, 240) else {
            return;
        };
        let mut project = Project::new();
        let asset_id = project.assets[0].id;
        project.assets[0].texture = Some(Canvas::new(64, 64, [0, 0, 0, 255]));
        let camera = Camera::default();
        let state = ViewportRenderState {
            shading: Shading::MaterialPreview,
            ..ViewportRenderState::default()
        };
        viewport
            .render_frame(&project, &[], &camera, state)
            .unwrap();
        let mesh_rebuilds = viewport.renderer.mesh_rebuilds();
        let full_uploads = viewport.renderer.full_texture_uploads();
        let upload_calls = viewport.renderer.texture_upload_calls();
        let uploaded_bytes = viewport.renderer.texture_bytes_uploaded();
        let partial_uploads = viewport.renderer.partial_texture_uploads();

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
        viewport.queue_texture_updates(vec![TextureUpdate {
            asset_id,
            canvas_width: 64,
            canvas_height: 64,
            regions: Some(vec![region]),
        }]);
        viewport
            .render_frame(&project, &[], &camera, state)
            .unwrap();

        assert_eq!(viewport.renderer.mesh_rebuilds(), mesh_rebuilds);
        assert_eq!(viewport.renderer.full_texture_uploads(), full_uploads);
        assert_eq!(viewport.renderer.texture_upload_calls(), upload_calls + 1);
        assert_eq!(
            viewport.renderer.texture_bytes_uploaded(),
            uploaded_bytes + region.byte_len()
        );
        assert_eq!(
            viewport.renderer.partial_texture_uploads(),
            partial_uploads + 1
        );
    }
}
