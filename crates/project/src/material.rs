//! Material System (P3D-050 a P3D-054, P3D-140).
//!
//! Modelo canônico e unificado de materiais com canais PBR (Albedo, Normal,
//! Roughness, Metallic, Emission, Height) e perfis de sombreamento (Pbr, Unlit,
//! Toon, Glass, Emissive). Compartilhado entre Viewport, Paint, UV e Exporters.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Canvas;

/// Perfil de shader do material (P3D-140).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ShaderProfile {
    /// Sombreamento fisicamente baseado padrão (Cook-Torrance / Lambert PBR).
    #[default]
    Pbr,
    /// Sem iluminação nem sombras, exibe cores e texturas puras.
    Unlit,
    /// Sombreamento cel-shaded estilizado com bandas discretas de luz.
    Toon,
    /// Vidro / superfície translúcida com refração e specular suave.
    Glass,
    /// Material auto-iluminado / emissivo.
    Emissive,
}

impl ShaderProfile {
    pub const ALL: [Self; 5] = [
        Self::Pbr,
        Self::Unlit,
        Self::Toon,
        Self::Glass,
        Self::Emissive,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Pbr => "PBR Standard",
            Self::Unlit => "Unlit / Flat",
            Self::Toon => "Toon / Cel-Shading",
            Self::Glass => "Glass / Transparent",
            Self::Emissive => "Emissive",
        }
    }
}

/// Modo de transparência alfa do material.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AlphaMode {
    /// 100% opaco, ignora canal alfa de textura.
    #[default]
    Opaque,
    /// Teste de máscara: pixels com alfa >= `alpha_cutoff` são opacos, abaixo são descartados.
    Mask,
    /// Mesclagem alfa translúcida contínua.
    Blend,
}

/// Canais de textura disponíveis no material (P3D-050 a P3D-054).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureChannel {
    Albedo,
    Normal,
    Roughness,
    Metallic,
    Emission,
    Height,
}

/// Material canônico PBR com canais de textura e parâmetros escalares.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Material {
    pub id: Uuid,
    pub name: String,
    /// Cor base RGBA linear (padrão [0.75, 0.75, 0.78, 1.0]).
    pub base_color: [f32; 4],
    /// Rugosidade da superfície [0.0 = espelhado/liso, 1.0 = totalmente difuso].
    pub roughness: f32,
    /// Metacidade [0.0 = dielétrico/plástico, 1.0 = metal puro].
    pub metallic: f32,
    /// Escala/intensidade do mapa de normais.
    pub normal_scale: f32,
    /// Cor de emissão RGB.
    pub emission_color: [f32; 3],
    /// Intensidade da luz emitida.
    pub emission_strength: f32,
    /// Perfil de shader selecionado.
    pub profile: ShaderProfile,
    /// Modo de canal alfa.
    pub alpha_mode: AlphaMode,
    /// Limiar para corte de alfa no modo Mask.
    pub alpha_cutoff: f32,

    // Canais de textura opcionais
    pub albedo_texture: Option<Canvas>,
    pub normal_texture: Option<Canvas>,
    pub roughness_texture: Option<Canvas>,
    pub metallic_texture: Option<Canvas>,
    pub emission_texture: Option<Canvas>,
    pub height_texture: Option<Canvas>,
}

impl Default for Material {
    fn default() -> Self {
        Self::new("Default Material")
    }
}

impl Material {
    /// Cria um novo material com parâmetros padrão neutros.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            base_color: [0.75, 0.75, 0.78, 1.0],
            roughness: 0.5,
            metallic: 0.0,
            normal_scale: 1.0,
            emission_color: [0.0, 0.0, 0.0],
            emission_strength: 0.0,
            profile: ShaderProfile::Pbr,
            alpha_mode: AlphaMode::Opaque,
            alpha_cutoff: 0.5,
            albedo_texture: None,
            normal_texture: None,
            roughness_texture: None,
            metallic_texture: None,
            emission_texture: None,
            height_texture: None,
        }
    }

    /// Cria um novo material com uma cor base sólida específica.
    pub fn with_color(name: impl Into<String>, color: [f32; 4]) -> Self {
        let mut mat = Self::new(name);
        mat.base_color = color;
        mat
    }

    /// Duplica o material com um novo identificador único UUID.
    pub fn duplicate(&self) -> Self {
        let mut dup = self.clone();
        dup.id = Uuid::new_v4();
        dup.name = format!("{} Copy", self.name);
        dup
    }

    /// Obtém a referência ao canvas de um canal de textura.
    pub fn channel_texture(&self, channel: TextureChannel) -> Option<&Canvas> {
        match channel {
            TextureChannel::Albedo => self.albedo_texture.as_ref(),
            TextureChannel::Normal => self.normal_texture.as_ref(),
            TextureChannel::Roughness => self.roughness_texture.as_ref(),
            TextureChannel::Metallic => self.metallic_texture.as_ref(),
            TextureChannel::Emission => self.emission_texture.as_ref(),
            TextureChannel::Height => self.height_texture.as_ref(),
        }
    }

    /// Obtém a referência mutável ao canvas de um canal de textura.
    pub fn channel_texture_mut(&mut self, channel: TextureChannel) -> Option<&mut Canvas> {
        match channel {
            TextureChannel::Albedo => self.albedo_texture.as_mut(),
            TextureChannel::Normal => self.normal_texture.as_mut(),
            TextureChannel::Roughness => self.roughness_texture.as_mut(),
            TextureChannel::Metallic => self.metallic_texture.as_mut(),
            TextureChannel::Emission => self.emission_texture.as_mut(),
            TextureChannel::Height => self.height_texture.as_mut(),
        }
    }

    /// Define o canvas de um canal de textura.
    pub fn set_channel_texture(&mut self, channel: TextureChannel, canvas: Option<Canvas>) {
        match channel {
            TextureChannel::Albedo => self.albedo_texture = canvas,
            TextureChannel::Normal => self.normal_texture = canvas,
            TextureChannel::Roughness => self.roughness_texture = canvas,
            TextureChannel::Metallic => self.metallic_texture = canvas,
            TextureChannel::Emission => self.emission_texture = canvas,
            TextureChannel::Height => self.height_texture = canvas,
        }
    }

    /// Garante que o canvas de um canal existe com dimensões e cor de preenchimento dadas.
    pub fn ensure_channel_canvas(
        &mut self,
        channel: TextureChannel,
        w: u32,
        h: u32,
        fill: [u8; 4],
    ) -> &mut Canvas {
        let slot = match channel {
            TextureChannel::Albedo => &mut self.albedo_texture,
            TextureChannel::Normal => &mut self.normal_texture,
            TextureChannel::Roughness => &mut self.roughness_texture,
            TextureChannel::Metallic => &mut self.metallic_texture,
            TextureChannel::Emission => &mut self.emission_texture,
            TextureChannel::Height => &mut self.height_texture,
        };
        slot.get_or_insert_with(|| Canvas::new(w, h, fill))
    }

    /// Amostra a cor efetiva do canal albedo para uma coordenada UV dada.
    pub fn sample_albedo(&self, uv: [f32; 2]) -> [f32; 4] {
        if let Some(canvas) = &self.albedo_texture {
            let u = uv[0].rem_euclid(1.0);
            let v = uv[1].rem_euclid(1.0);
            let px = ((u * canvas.w as f32) as u32).min(canvas.w.saturating_sub(1));
            let py = (((1.0 - v) * canvas.h as f32) as u32).min(canvas.h.saturating_sub(1));
            if let Some(rgba) = canvas.get(px, py) {
                let r = (rgba[0] as f32 / 255.0) * self.base_color[0];
                let g = (rgba[1] as f32 / 255.0) * self.base_color[1];
                let b = (rgba[2] as f32 / 255.0) * self.base_color[2];
                let a = (rgba[3] as f32 / 255.0) * self.base_color[3];
                return [r, g, b, a];
            }
        }
        self.base_color
    }

    /// Converte glossiness para roughness (P3D-053).
    pub fn set_glossiness(&mut self, glossiness: f32) {
        self.roughness = (1.0 - glossiness.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    }

    /// Retorna glossiness derivado da roughness (P3D-053).
    pub fn glossiness(&self) -> f32 {
        (1.0 - self.roughness).clamp(0.0, 1.0)
    }

    /// Valida e sanitiza todos os campos numéricos e dimensões de canvas.
    pub fn validate(&mut self) {
        for c in &mut self.base_color {
            if !c.is_finite() {
                *c = 1.0;
            }
            *c = c.clamp(0.0, 1.0);
        }
        if !self.roughness.is_finite() {
            self.roughness = 0.5;
        }
        self.roughness = self.roughness.clamp(0.0, 1.0);

        if !self.metallic.is_finite() {
            self.metallic = 0.0;
        }
        self.metallic = self.metallic.clamp(0.0, 1.0);

        if !self.normal_scale.is_finite() {
            self.normal_scale = 1.0;
        }
        self.normal_scale = self.normal_scale.clamp(0.0, 10.0);

        for c in &mut self.emission_color {
            if !c.is_finite() {
                *c = 0.0;
            }
            *c = c.clamp(0.0, 1.0);
        }
        if !self.emission_strength.is_finite() {
            self.emission_strength = 0.0;
        }
        self.emission_strength = self.emission_strength.max(0.0);

        if !self.alpha_cutoff.is_finite() {
            self.alpha_cutoff = 0.5;
        }
        self.alpha_cutoff = self.alpha_cutoff.clamp(0.0, 1.0);

        if let Some(cv) = &mut self.albedo_texture {
            cv.validate();
        }
        if let Some(cv) = &mut self.normal_texture {
            cv.validate();
        }
        if let Some(cv) = &mut self.roughness_texture {
            cv.validate();
        }
        if let Some(cv) = &mut self.metallic_texture {
            cv.validate();
        }
        if let Some(cv) = &mut self.emission_texture {
            cv.validate();
        }
        if let Some(cv) = &mut self.height_texture {
            cv.validate();
        }
    }
}
