//! Viewport dividida (cap. 36, revisão 2026-09-30): uma segunda vista 3D **opcional**.
//!
//! A vista secundária é um companheiro de navegação: câmera própria (presets
//! Persp/Front/Back/Left/Right/Top), mesma cena, seleção e shading da vista
//! principal, renderizada pelo caminho de software para não duplicar buffers de
//! GPU. Edição (seleção, transformação, pintura) continua só na vista principal.
//! O documento, o undo e a seleção são compartilhados; nada aqui os altera.

use std::hash::{Hash, Hasher};

use petunia_core::{Camera, ViewPreset};
use petunia_project::Project;

use crate::viewport_soft::Software3dViewport;
use crate::{PetuniaViewport, ViewportRenderState};

/// Largura mínima (px) por vista: abaixo disso a divisão é recusada (cap. 36).
pub const MIN_SPLIT_VIEW_WIDTH: f32 = 480.0;

pub struct SplitView {
    pub enabled: bool,
    pub camera: Camera,
    viewport: Software3dViewport,
    size: [u32; 2],
    last_key: Option<u64>,
}

impl Default for SplitView {
    fn default() -> Self {
        Self {
            enabled: false,
            camera: Camera::default(),
            viewport: Software3dViewport::new(2, 2),
            size: [2, 2],
            last_key: None,
        }
    }
}

impl SplitView {
    /// Abre a vista secundária a partir da câmera principal, em Front ortográfica.
    pub fn open(&mut self, main: &Camera) {
        self.camera = main.clone();
        self.camera.set_preset(ViewPreset::Front);
        self.camera.aspect = self.size[0] as f32 / self.size[1].max(1) as f32;
        self.enabled = true;
        self.last_key = None;
    }

    pub fn close(&mut self) {
        self.enabled = false;
        self.last_key = None;
    }

    pub fn set_preset(&mut self, preset: ViewPreset) {
        self.camera.set_preset(preset);
        self.last_key = None;
    }

    pub fn orbit(&mut self, dx: f32, dy: f32) {
        self.camera.orbit(dx, dy);
    }

    pub fn pan(&mut self, dx: f32, dy: f32) {
        self.camera.pan(dx, dy);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.camera.zoom(delta);
    }

    /// Identificador estável do preset ativo (`custom` quando orbitado).
    pub fn preset_id(&self) -> &'static str {
        match self.camera.view_preset() {
            Some(ViewPreset::Persp) => "persp",
            Some(ViewPreset::Front) => "front",
            Some(ViewPreset::Back) => "back",
            Some(ViewPreset::Left) => "left",
            Some(ViewPreset::Right) => "right",
            Some(ViewPreset::Top) => "top",
            _ => "custom",
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let size = [width.max(1), height.max(1)];
        if size == self.size {
            return;
        }
        self.size = size;
        self.viewport.resize(size[0], size[1]);
        self.camera.aspect = size[0] as f32 / size[1] as f32;
        self.last_key = None;
    }

    fn key(&self, project: &Project, state: &ViewportRenderState) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        for revision in [
            project.topology_revision,
            project.position_revision,
            project.selection_revision,
            project.material_revision,
            project.texture_revision,
            project.transform_revision,
            project.normal_revision,
            project.uv_revision,
            project.color_revision,
            project.assets.len() as u64,
        ] {
            revision.hash(&mut hasher);
        }
        let c = &self.camera;
        for value in [
            c.target.x,
            c.target.y,
            c.target.z,
            c.distance,
            c.yaw,
            c.pitch,
            c.fov_y,
            c.aspect,
            c.ortho_half_h,
            state.xray_opacity,
        ] {
            value.to_bits().hash(&mut hasher);
        }
        (c.proj as u8).hash(&mut hasher);
        self.size.hash(&mut hasher);
        format!("{:?}", state.shading).hash(&mut hasher);
        (
            state.xray,
            state.show_wireframe_overlay,
            state.show_grid,
            state.textured,
        )
            .hash(&mut hasher);
        hasher.finish()
    }

    /// Renderiza só quando cena, câmera ou opções mudaram desde o último quadro.
    pub fn render(
        &mut self,
        project: &Project,
        refs: &[petunia_core::ReferenceImage],
        state: ViewportRenderState,
    ) -> Option<slint::Image> {
        if !self.enabled {
            return None;
        }
        let key = self.key(project, &state);
        if self.last_key == Some(key) {
            return None;
        }
        self.last_key = Some(key);
        self.viewport
            .render_frame(project, refs, &self.camera, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_project::Asset;

    fn scene() -> Project {
        let mut project = Project::default();
        project
            .assets
            .push(Asset::new("Cube", petunia_mesh::Mesh::cube(1.0)));
        project
    }

    #[test]
    fn opens_in_front_view_and_renders_only_when_something_changed() {
        let mut split = SplitView::default();
        assert!(!split.enabled);
        assert!(
            split
                .render(&scene(), &[], ViewportRenderState::default())
                .is_none()
        );

        split.open(&Camera::default());
        split.resize(200, 120);
        assert_eq!(split.preset_id(), "front");
        let project = scene();
        assert!(
            split
                .render(&project, &[], ViewportRenderState::default())
                .is_some()
        );
        assert!(
            split
                .render(&project, &[], ViewportRenderState::default())
                .is_none(),
            "sem mudança não re-renderiza"
        );
        split.set_preset(ViewPreset::Top);
        assert_eq!(split.preset_id(), "top");
        assert!(
            split
                .render(&project, &[], ViewportRenderState::default())
                .is_some()
        );
        split.orbit(20.0, -40.0);
        assert_eq!(split.preset_id(), "custom");
    }

    #[test]
    fn close_stops_rendering_and_keeps_the_document_untouched() {
        let mut split = SplitView::default();
        split.open(&Camera::default());
        let project = scene();
        let before = project.assets.len();
        split.close();
        assert!(
            split
                .render(&project, &[], ViewportRenderState::default())
                .is_none()
        );
        assert_eq!(project.assets.len(), before);
    }
}
