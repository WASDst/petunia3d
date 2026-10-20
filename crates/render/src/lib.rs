//! Renderer do Petunia3D: boundary neutro (`frame`), adaptador do documento
//! (`extract`), tipos/tesselação de overlay (`overlays`) e o renderer OpenGL 3.3
//! Core concreto (`gl`) — o único backend do viewport. Tipos compartilhados de
//! shading, stats e capabilities (§29-30) também vivem aqui.
//!
//! Matemática de cena (grid, quads de referência, luz) vive em `scene` para o
//! renderer e o viewport por software usarem a mesma fonte.

pub mod extract;
pub mod frame;
pub mod gl;
pub mod overlays;
pub mod scene;

pub use overlays::{EdgeMode, OverlayShape, WorkplaneOverlay, edge_overlay_visible};

/// Viewport shading. Defined in core (session state); re-exported here so
/// existing `petunia_render::Shading` call sites keep compiling.
pub use petunia_core::Shading;

/// Contadores do frame para overlay e benchmarks.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameStats {
    pub tris: usize,
    pub draws: usize,
    pub tex_uploads: usize,
}

/// Capacidades detectadas na inicialização (§29).
#[derive(Debug, Clone)]
pub struct GpuCaps {
    pub gl_version: String,
    pub glsl_version: String,
    pub vendor: String,
    pub renderer: String,
    pub max_texture_size: i32,
    /// Amostras máximas de MSAA em renderbuffer.
    pub max_samples: i32,
    pub major: u32,
    pub minor: u32,
    /// Contexto OpenGL ES (não suportado como alvo; só diagnóstico).
    pub embedded: bool,
}

impl GpuCaps {
    /// Baseline do renderer: OpenGL 3.3 Core (desktop).
    pub fn supports_core_33(&self) -> bool {
        !self.embedded && (self.major, self.minor) >= (3, 3)
    }

    /// Detecta se está rodando sobre o Zink (implementação OpenGL sobre Vulkan do Mesa).
    pub fn is_zink(&self) -> bool {
        self.renderer.to_ascii_lowercase().contains("zink")
    }

    /// Detecta se está rodando sobre o driver Vulkan NVK (NVIDIA open-source no Mesa).
    pub fn is_nvk(&self) -> bool {
        self.renderer.to_ascii_lowercase().contains("nvk")
    }

    /// Detecta rasterizador por software puro (llvmpipe, softpipe, swrast, lavapipe).
    pub fn is_software(&self) -> bool {
        let r = self.renderer.to_ascii_lowercase();
        r.contains("llvmpipe")
            || r.contains("softpipe")
            || r.contains("swrast")
            || r.contains("lavapipe")
    }

    /// Resumo de uma linha para log de startup e relatórios.
    pub fn summary(&self) -> String {
        let mut notes = Vec::new();
        if self.is_zink() {
            notes.push("Zink");
        }
        if self.is_nvk() {
            notes.push("NVK");
        }
        if self.is_software() {
            notes.push("software rasterizer");
        }
        let note_suffix = if notes.is_empty() {
            String::new()
        } else {
            format!(" [{}]", notes.join(", "))
        };
        format!(
            "OpenGL {} | GLSL {} | {} — {}{} | max texture {} | max MSAA {}x",
            self.gl_version,
            self.glsl_version,
            self.vendor,
            self.renderer,
            note_suffix,
            self.max_texture_size,
            self.max_samples
        )
    }
}

impl Default for GpuCaps {
    fn default() -> Self {
        Self {
            gl_version: "desconhecido".into(),
            glsl_version: "desconhecido".into(),
            vendor: "desconhecido".into(),
            renderer: "desconhecido".into(),
            max_texture_size: 2048,
            max_samples: 1,
            major: 0,
            minor: 0,
            embedded: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_33_is_the_floor_and_gles_is_rejected() {
        let caps = |major, minor, embedded| GpuCaps {
            major,
            minor,
            embedded,
            ..GpuCaps::default()
        };
        assert!(caps(3, 3, false).supports_core_33());
        assert!(caps(4, 6, false).supports_core_33());
        assert!(!caps(3, 2, false).supports_core_33());
        assert!(!caps(3, 3, true).supports_core_33());
    }
}

mod surface_materials;
