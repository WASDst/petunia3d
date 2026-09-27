//! Integração WGPU com o PetuniaViewport e Slint Image.

use std::sync::Arc;

use petunia_core::{Camera, SelectionDomain, Workspace};
use petunia_project::Project;
use petunia_render_wgpu::Renderer;

use crate::{PetuniaViewport, ViewportRenderState};

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
        let renderer = Renderer::new(&device, wgpu::TextureFormat::Rgba8Unorm);
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

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Slint Viewport Encoder"),
            });

        {
            let depth_view = self.renderer.depth_view();
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Slint Viewport Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.082,
                            g: 0.086,
                            b: 0.098,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
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
        if self.width != width || self.height != height {
            self.width = width.max(1);
            self.height = height.max(1);
            self.recreate_target();
        }
    }

    fn update(&mut self, _dt_seconds: f32) {}

    fn set_workspace(&mut self, workspace: Workspace) {
        self.workspace = workspace;
    }

    fn set_selection_domain(&mut self, domain: SelectionDomain) {
        self.selection_domain = domain;
    }

    fn draws_component_guides(&self) -> bool {
        true
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
    use petunia_render::Shading;

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
}
