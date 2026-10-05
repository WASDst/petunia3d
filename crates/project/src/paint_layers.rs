//! Pilha de camadas de pintura (P3D-061, P3D-133, P3D-134).
//!
//! Vive no crate de projeto porque é **dado persistente do asset**
//! (serialize + undo via snapshot do `Project`), não estado de UI.
//! `module-paint` re-exporta estes tipos e implementa as operações.
//!
//! Modelo V1: camadas Raster ordenadas com visibility/opacity/active,
//! composição determinística alpha-normal sobre o canvas base. Decal e
//! SVG decals, effects and raster layers participate in composition. Projection
//! commits raster pixels; the source decal remains a live, reusable layer.

use serde::{Deserialize, Serialize};

use glam::Vec3;
use petunia_mesh::Mesh;

use crate::Canvas;
use crate::svg::{SvgError, rasterize_svg, svg_info};

/// Tamanho de tile para composição parcial (P3D-061: composite cacheável).
pub const TILE_SIZE: u32 = 32;

/// Modo de mesclagem de camadas de pintura (P3D-061).
///
/// V1 usa `Normal`; demais modos existem para compatibilidade de
/// serialização e testes, sem UI dedicada.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LayerBlendMode {
    #[default]
    Normal,
    Multiply,
    Add,
    Screen,
}

/// Efeitos não-destrutivos sobre texturas / camadas (P3D-134, cap. 42).
///
/// Discriminantes **append-only**: variantes novas entram no fim para
/// preservar valores serializados. A lista segue os nodes iniciais do
/// cap. 42 (presets antes de graphs).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum PaintEffect {
    /// Pixelização com tamanho de bloco especificado (P3D-134).
    Pixelate { cell_size: u32 },
    /// Quantização / posterização de tons por canal de cor (P3D-134).
    Posterize { levels: u8 },
    /// Inversão de cores RGB.
    Invert,
    /// Ruído aditivo determinístico por pixel (cap. 42: Pixel Noise / Grain).
    /// `intensity` 0..=1; mesmo `seed` ⇒ mesmo resultado (avaliador determinístico).
    Grain { intensity: f32, seed: u32 },
    /// Remapeamento de tons: `[in_min, in_max]` → `[out_min, out_max]` com gamma.
    Levels {
        in_min: f32,
        in_max: f32,
        gamma: f32,
        out_min: f32,
        out_max: f32,
    },
    /// Brilho (`-1..=1`) e contraste (`-1..=1`; `-1` achata em cinza médio).
    BrightnessContrast { brightness: f32, contrast: f32 },
    /// Rotação de matiz em graus e escala de saturação (`-1..=1`).
    HueSaturation { hue_shift_deg: f32, saturation: f32 },
}

/// Fixação do decalque na superfície (cap. 39, "surface attachment"): um
/// projetor ortogonal no espaço do objeto. Cada texel recebe a cor pela sua
/// posição 3D projetada no plano do decalque, então o decalque fica onde foi
/// colocado, atravessa costuras e mantém a proporção da imagem.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecalAnchor {
    /// Centro do decalque na superfície (mundo).
    pub point: [f32; 3],
    /// Direção de projeção (normal da superfície no ponto, unitária).
    pub normal: [f32; 3],
    /// Direção "direita" da imagem antes da rotação (unitária, ⟂ normal).
    pub tangent: [f32; 3],
    /// Largura da imagem em unidades de mundo; a altura segue a proporção.
    pub width: f32,
    /// Alcance do projetor ao longo da normal, para os dois lados (mundo).
    pub depth: f32,
}

impl DecalAnchor {
    /// Fixação em `point` com `normal`, a imagem "em pé" segundo `up` (ex.:
    /// o vetor cima da câmera); `up` paralelo à normal cai num eixo do mundo.
    pub fn new(point: Vec3, normal: Vec3, up: Vec3, width: f32) -> Option<Self> {
        let normal = normal.normalize_or_zero();
        if normal == Vec3::ZERO || !point.is_finite() || !(width.is_finite() && width > 0.0) {
            return None;
        }
        let tangent = [up, Vec3::Y, Vec3::Z, Vec3::X]
            .into_iter()
            .map(|reference| reference.cross(normal))
            .find(|t| t.length_squared() > 1.0e-6)?
            .normalize();
        Some(Self {
            point: point.to_array(),
            normal: normal.to_array(),
            tangent: tangent.to_array(),
            width,
            depth: width * 0.5,
        })
    }

    /// Base do projetor com a rotação aplicada: (direita, cima, normal).
    pub fn frame(&self, rotation_rad: f32) -> (Vec3, Vec3, Vec3) {
        let n = Vec3::from(self.normal).normalize_or_zero();
        let t0 = Vec3::from(self.tangent).normalize_or_zero();
        let b0 = n.cross(t0);
        let (sin, cos) = rotation_rad.sin_cos();
        (t0 * cos + b0 * sin, b0 * cos - t0 * sin, n)
    }
}

/// Amostra bilinear (`u`, `v` em 0..1, origem no canto superior esquerdo).
fn sample_bilinear(image: &Canvas, u: f32, v: f32) -> Option<[u8; 4]> {
    if image.w == 0 || image.h == 0 || !(u.is_finite() && v.is_finite()) {
        return None;
    }
    let x = (u * image.w as f32 - 0.5).clamp(0.0, image.w as f32 - 1.0);
    let y = (v * image.h as f32 - 0.5).clamp(0.0, image.h as f32 - 1.0);
    let (x0, y0) = (x.floor() as u32, y.floor() as u32);
    let (x1, y1) = ((x0 + 1).min(image.w - 1), (y0 + 1).min(image.h - 1));
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let (a, b, c, d) = (
        image.get(x0, y0)?,
        image.get(x1, y0)?,
        image.get(x0, y1)?,
        image.get(x1, y1)?,
    );
    let mut out = [0u8; 4];
    for i in 0..4 {
        let top = a[i] as f32 * (1.0 - fx) + b[i] as f32 * fx;
        let bottom = c[i] as f32 * (1.0 - fx) + d[i] as f32 * fx;
        out[i] = (top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8;
    }
    Some(out)
}

/// Uma variante de um Decal Set (cap. 39): olhos abertos/fechados, bocas,
/// estados de dano. Variantes são equivalentes: mesma fixação, outra imagem.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecalVariant {
    pub name: String,
    pub image: Canvas,
    /// SVG de origem desta variante (re-rasterizável sem perder nitidez).
    #[serde(default)]
    pub source_svg: Option<String>,
}

/// Chave da trilha de variantes: a partir de `frame`, mostra `variant`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecalVariantKey {
    pub frame: u32,
    pub variant: u32,
}

/// Trilha `variant_index` (cap. 39, "step animation"): troca de variante em
/// degraus, sem interpolação. Chaves ordenadas por quadro e únicas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DecalVariantTrack {
    pub fps: f32,
    /// Duração do ciclo em quadros (o último quadro é `length_frames - 1`).
    pub length_frames: u32,
    pub looping: bool,
    pub keys: Vec<DecalVariantKey>,
}

impl Default for DecalVariantTrack {
    fn default() -> Self {
        Self {
            fps: 12.0,
            length_frames: 24,
            looping: true,
            keys: Vec::new(),
        }
    }
}

impl DecalVariantTrack {
    /// Quadro mostrado no instante `time` (segundos), dando a volta no ciclo.
    pub fn frame_at(&self, time: f32) -> u32 {
        if !(time.is_finite() && self.fps.is_finite() && self.fps > 0.0) || time <= 0.0 {
            return 0;
        }
        let frame = (time * self.fps).floor() as u64;
        let length = u64::from(self.length_frames.max(1));
        if self.looping {
            (frame % length) as u32
        } else {
            frame.min(length - 1) as u32
        }
    }

    /// Variante no quadro `frame`: a da última chave em ou antes dele; antes
    /// da primeira chave, vale a primeira. `None` sem chaves.
    pub fn variant_at_frame(&self, frame: u32) -> Option<u32> {
        let first = self.keys.first()?;
        Some(
            self.keys
                .iter()
                .take_while(|key| key.frame <= frame)
                .last()
                .unwrap_or(first)
                .variant,
        )
    }

    /// Grava (ou substitui) a chave do quadro `frame`.
    pub fn set_key(&mut self, frame: u32, variant: u32) {
        match self.keys.binary_search_by_key(&frame, |key| key.frame) {
            Ok(index) => self.keys[index].variant = variant,
            Err(index) => self.keys.insert(index, DecalVariantKey { frame, variant }),
        }
        self.length_frames = self.length_frames.max(frame + 1);
    }

    /// Remove a chave do quadro `frame`. `false` se não havia.
    pub fn remove_key(&mut self, frame: u32) -> bool {
        match self.keys.binary_search_by_key(&frame, |key| key.frame) {
            Ok(index) => {
                self.keys.remove(index);
                true
            }
            Err(_) => false,
        }
    }
}

/// Decalque / projeção 2D parametrizada e reposicionável (P3D-133, MVP).
#[derive(Clone, Debug, PartialEq)]
pub struct DecalLayer {
    pub image: Canvas,
    /// Centro da projeção no espaço UV [0.0..1.0]
    pub center_uv: [f32; 2],
    /// Escala relativa da estampa no espaço UV [0.0..1.0]
    pub scale_uv: [f32; 2],
    /// Rotação do decalque em radianos
    pub rotation_rad: f32,
    /// Texto SVG de origem, quando o decalque veio de um vetor. Permite
    /// re-rasterizar em outra resolução sem perder nitidez (P3D-133).
    /// Projetos antigos não têm o campo e abrem como `None`.
    pub source_svg: Option<String>,
    /// Fixação na superfície. `None` = decalque em espaço UV (legado):
    /// `center_uv`/`scale_uv` posicionam a imagem no atlas.
    pub anchor: Option<DecalAnchor>,
    /// Decal Set: todas as variantes, inclusive a mostrada. Vazio = uma
    /// variante só (`image`). Com variantes, `image` e `source_svg` são a
    /// cópia da variante `variant_index`.
    pub variants: Vec<DecalVariant>,
    pub variant_index: usize,
    /// Trilha de variantes em degraus (`None` = decalque estático).
    pub track: Option<DecalVariantTrack>,
}

// Legacy postcard files predate SVG. Their fixed decal layout has four fields;
// canonical ZIP/JSON stores the editable SVG source as the fifth optional field.
impl Serialize for DecalLayer {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let human = serializer.is_human_readable();
        let mut value = serializer.serialize_struct("DecalLayer", if human { 9 } else { 4 })?;
        value.serialize_field("image", &self.image)?;
        value.serialize_field("center_uv", &self.center_uv)?;
        value.serialize_field("scale_uv", &self.scale_uv)?;
        value.serialize_field("rotation_rad", &self.rotation_rad)?;
        if human {
            value.serialize_field("source_svg", &self.source_svg)?;
            value.serialize_field("anchor", &self.anchor)?;
            value.serialize_field("variants", &self.variants)?;
            value.serialize_field("variant_index", &self.variant_index)?;
            value.serialize_field("track", &self.track)?;
        }
        value.end()
    }
}
impl<'de> Deserialize<'de> for DecalLayer {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Old {
            image: Canvas,
            center_uv: [f32; 2],
            scale_uv: [f32; 2],
            rotation_rad: f32,
        }
        #[derive(Deserialize)]
        struct Current {
            image: Canvas,
            center_uv: [f32; 2],
            scale_uv: [f32; 2],
            rotation_rad: f32,
            #[serde(default)]
            source_svg: Option<String>,
            #[serde(default)]
            anchor: Option<DecalAnchor>,
            #[serde(default)]
            variants: Vec<DecalVariant>,
            #[serde(default)]
            variant_index: usize,
            #[serde(default)]
            track: Option<DecalVariantTrack>,
        }
        if deserializer.is_human_readable() {
            let c = Current::deserialize(deserializer)?;
            let mut decal = Self {
                image: c.image,
                center_uv: c.center_uv,
                scale_uv: c.scale_uv,
                rotation_rad: c.rotation_rad,
                source_svg: c.source_svg,
                anchor: c.anchor,
                variants: c.variants,
                variant_index: c.variant_index,
                track: c.track,
            };
            // Arquivo editado à mão ou corrompido: índice dentro da lista.
            if decal.variant_index >= decal.variants.len().max(1) {
                decal.variant_index = 0;
            }
            Ok(decal)
        } else {
            let c = Old::deserialize(deserializer)?;
            Ok(Self::new(c.image, c.center_uv, c.scale_uv, c.rotation_rad))
        }
    }
}

impl DecalLayer {
    pub fn new(image: Canvas, center_uv: [f32; 2], scale_uv: [f32; 2], rotation_rad: f32) -> Self {
        Self {
            image,
            center_uv,
            scale_uv,
            rotation_rad,
            source_svg: None,
            anchor: None,
            variants: Vec::new(),
            variant_index: 0,
            track: None,
        }
    }

    /// Número de variantes (um decalque simples tem uma).
    pub fn variant_count(&self) -> usize {
        self.variants.len().max(1)
    }

    /// Nome da variante `index` (`base` para o decalque de variante única).
    pub fn variant_name<'a>(&'a self, index: usize, base: &'a str) -> &'a str {
        self.variants.get(index).map_or(base, |v| v.name.as_str())
    }

    /// Imagem da variante `index`.
    pub fn variant_image(&self, index: usize) -> &Canvas {
        self.variants.get(index).map_or(&self.image, |v| &v.image)
    }

    /// Transforma o decalque simples num Decal Set de uma variante (`name`).
    fn ensure_variants(&mut self, name: &str) {
        if self.variants.is_empty() {
            self.variants.push(DecalVariant {
                name: name.to_owned(),
                image: self.image.clone(),
                source_svg: self.source_svg.clone(),
            });
            self.variant_index = 0;
        }
    }

    /// Acrescenta uma variante; `base_name` nomeia a imagem atual se o
    /// decalque ainda não era um Decal Set. Devolve o índice da nova.
    pub fn add_variant(&mut self, base_name: &str, variant: DecalVariant) -> usize {
        self.ensure_variants(base_name);
        self.variants.push(variant);
        self.variants.len() - 1
    }

    /// Mostra a variante `index` (estado parado do decalque).
    pub fn set_variant(&mut self, index: usize) -> bool {
        let Some(variant) = self.variants.get(index) else {
            return index == 0 && self.variants.is_empty();
        };
        self.image = variant.image.clone();
        self.source_svg = variant.source_svg.clone();
        self.variant_index = index;
        true
    }

    /// Remove a variante `index` e as chaves que a mostravam; as chaves das
    /// seguintes descem um índice. A última variante não pode sair.
    pub fn remove_variant(&mut self, index: usize) -> bool {
        if self.variants.len() < 2 || index >= self.variants.len() {
            return false;
        }
        self.variants.remove(index);
        if let Some(track) = self.track.as_mut() {
            let removed = index as u32;
            track.keys.retain(|key| key.variant != removed);
            for key in &mut track.keys {
                if key.variant > removed {
                    key.variant -= 1;
                }
            }
        }
        let shown = if self.variant_index > index {
            self.variant_index - 1
        } else {
            self.variant_index.min(self.variants.len() - 1)
        };
        self.set_variant(shown)
    }

    /// Renomeia a variante `index`.
    pub fn rename_variant(&mut self, index: usize, name: &str) -> bool {
        let name = name.trim();
        match self.variants.get_mut(index) {
            Some(variant) if !name.is_empty() => {
                variant.name = name.to_owned();
                true
            }
            _ => false,
        }
    }

    /// Variante mostrada no instante `time` (segundos da reprodução).
    /// Parado (`None`), sem trilha ou sem chaves: `variant_index`.
    pub fn variant_at(&self, time: Option<f32>) -> usize {
        let keyed = time
            .zip(self.track.as_ref())
            .and_then(|(time, track)| track.variant_at_frame(track.frame_at(time)));
        match keyed {
            Some(variant) if (variant as usize) < self.variants.len() => variant as usize,
            _ => self.variant_index,
        }
    }

    /// Imagem mostrada no instante `time` (ver [`Self::variant_at`]).
    pub fn image_at(&self, time: Option<f32>) -> &Canvas {
        if self.variants.is_empty() {
            return &self.image;
        }
        let index = self.variant_at(time);
        if index == self.variant_index {
            &self.image
        } else {
            self.variant_image(index)
        }
    }

    /// Altura em mundo de um decalque de superfície (proporção da imagem).
    pub fn surface_height(&self, width: f32) -> f32 {
        width * self.image.h.max(1) as f32 / self.image.w.max(1) as f32
    }

    /// Cantos do retângulo do projetor (mundo), na ordem superior-esquerdo,
    /// superior-direito, inferior-direito, inferior-esquerdo. `None` sem fixação.
    pub fn surface_corners(&self) -> Option<[Vec3; 4]> {
        let anchor = self.anchor?;
        let (right, up, _) = anchor.frame(self.rotation_rad);
        let center = Vec3::from(anchor.point);
        let (hw, hh) = (anchor.width * 0.5, self.surface_height(anchor.width) * 0.5);
        Some([
            center - right * hw + up * hh,
            center + right * hw + up * hh,
            center + right * hw - up * hh,
            center - right * hw - up * hh,
        ])
    }

    /// Cores projetadas pelo decalque de superfície: `(x, y, rgba)` por texel
    /// de uma textura `w × h` da malha. Só faces voltadas para o projetor e
    /// dentro do seu alcance; amostragem bilinear. Vazio sem fixação.
    pub fn surface_samples(&self, mesh: &Mesh, w: u32, h: u32) -> Vec<(u32, u32, [u8; 4])> {
        self.surface_samples_of(&self.image, mesh, w, h)
    }

    /// Como [`Self::surface_samples`], com a imagem de uma variante.
    pub fn surface_samples_of(
        &self,
        image: &Canvas,
        mesh: &Mesh,
        w: u32,
        h: u32,
    ) -> Vec<(u32, u32, [u8; 4])> {
        let Some(anchor) = self.anchor else {
            return Vec::new();
        };
        let (right, up, normal) = anchor.frame(self.rotation_rad);
        let half_w = anchor.width * 0.5;
        let half_h = self.surface_height(anchor.width) * 0.5;
        if normal == Vec3::ZERO || !(half_w > 0.0 && half_h > 0.0) {
            return Vec::new();
        }
        let center = Vec3::from(anchor.point);
        let reach = (half_w * half_w + half_h * half_h + anchor.depth * anchor.depth).sqrt();
        let mut written = std::collections::HashSet::new();
        let mut out = Vec::new();
        for fi in 0..mesh.faces.len() {
            // Superfície de costas para o projetor não recebe o decalque.
            if mesh.face_normal(fi).normalize_or_zero().dot(normal) < 0.1 {
                continue;
            }
            mesh.rasterize_face_near(fi, center, reach, w, h, 1.0, |x, y, _, pos, _| {
                let local = pos - center;
                if local.dot(normal).abs() > anchor.depth {
                    return;
                }
                let u = local.dot(right) / (2.0 * half_w) + 0.5;
                let v = 0.5 - local.dot(up) / (2.0 * half_h);
                if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
                    return;
                }
                if !written.insert((x, y)) {
                    return;
                }
                if let Some(color) = sample_bilinear(image, u, v) {
                    out.push((x, y, color));
                }
            });
        }
        out
    }

    /// Cor do decalque UV (legado) no texel `(x, y)` de uma textura `w × h`.
    fn uv_sample(&self, x: u32, y: u32, w: u32, h: u32) -> Option<[u8; 4]> {
        self.uv_sample_of(&self.image, x, y, w, h)
    }

    fn uv_sample_of(&self, image: &Canvas, x: u32, y: u32, w: u32, h: u32) -> Option<[u8; 4]> {
        if self.scale_uv[0].abs() < 1e-5 || self.scale_uv[1].abs() < 1e-5 {
            return None;
        }
        let (sin_rot, cos_rot) = (-self.rotation_rad).sin_cos();
        let dx = (x as f32 + 0.5) / w as f32 - self.center_uv[0];
        let dy = (y as f32 + 0.5) / h as f32 - self.center_uv[1];
        let decal_u = (dx * cos_rot - dy * sin_rot) / self.scale_uv[0] + 0.5;
        let decal_v = (dx * sin_rot + dy * cos_rot) / self.scale_uv[1] + 0.5;
        if !(0.0..=1.0).contains(&decal_u) || !(0.0..=1.0).contains(&decal_v) {
            return None;
        }
        sample_bilinear(image, decal_u, decal_v)
    }

    /// Retângulo de texels `[x0, y0, x1, y1)` que o decalque cobre numa
    /// textura `w × h` (pela imagem parada). `None` se não cobre nada.
    pub fn texel_bounds(&self, mesh: Option<&Mesh>, w: u32, h: u32) -> Option<[u32; 4]> {
        if w == 0 || h == 0 {
            return None;
        }
        if self.anchor.is_some() {
            let samples = self.surface_samples(mesh?, w, h);
            let mut bounds = [u32::MAX, u32::MAX, 0, 0];
            for &(x, y, _) in &samples {
                bounds = [
                    bounds[0].min(x),
                    bounds[1].min(y),
                    bounds[2].max(x + 1),
                    bounds[3].max(y + 1),
                ];
            }
            return (!samples.is_empty()).then_some(bounds);
        }
        // Retângulo UV girado: caixa dos quatro cantos.
        let (sin, cos) = self.rotation_rad.sin_cos();
        let (hu, hv) = (self.scale_uv[0] * 0.5, self.scale_uv[1] * 0.5);
        let corners = [(-hu, -hv), (hu, -hv), (hu, hv), (-hu, hv)].map(|(du, dv)| {
            (
                self.center_uv[0] + du * cos - dv * sin,
                self.center_uv[1] + du * sin + dv * cos,
            )
        });
        let (mut u0, mut v0, mut u1, mut v1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (u, v) in corners {
            (u0, v0, u1, v1) = (u0.min(u), v0.min(v), u1.max(u), v1.max(v));
        }
        let x0 = (u0.max(0.0) * w as f32).floor() as u32;
        let y0 = (v0.max(0.0) * h as f32).floor() as u32;
        let x1 = ((u1.min(1.0) * w as f32).ceil() as u32 + 1).min(w);
        let y1 = ((v1.min(1.0) * h as f32).ceil() as u32 + 1).min(h);
        (x0 < x1 && y0 < y1).then_some([x0, y0, x1, y1])
    }

    /// Lado (px) que a imagem precisa para não perder nitidez na textura
    /// `w × h`: o maior lado, em texels, da área que o decalque ocupa.
    pub fn needed_raster_px(&self, mesh: Option<&Mesh>, w: u32, h: u32) -> Option<u32> {
        let [x0, y0, x1, y1] = self.texel_bounds(mesh, w, h)?;
        Some((x1 - x0).max(y1 - y0))
    }

    /// Re-rasteriza as variantes vindas de SVG quando a área na textura pede
    /// bem mais pixels que a imagem tem (10% de tolerância: o rasterizador
    /// arredonda) ou menos da metade (memória). Posição, largura e rotação não
    /// mudam. `Ok(true)` se refez alguma imagem.
    pub fn fit_svg_resolution(&mut self, needed_px: u32) -> Result<bool, SvgError> {
        let target = needed_px.clamp(64, crate::svg::MAX_RASTER_PX);
        let refit = |image: &Canvas| {
            let side = image.w.max(image.h);
            side.saturating_mul(10) < target.saturating_mul(9) || side > target.saturating_mul(2)
        };
        let mut changed = false;
        if self.source_svg.is_some() && refit(&self.image) {
            changed |= self.rerasterize(target)?;
        }
        for (index, variant) in self.variants.iter_mut().enumerate() {
            if index == self.variant_index {
                if let Some(svg) = &self.source_svg {
                    variant.image = self.image.clone();
                    variant.source_svg = Some(svg.clone());
                }
                continue;
            }
            if let Some(svg) = variant.source_svg.as_deref()
                && refit(&variant.image)
            {
                variant.image = rasterize_svg(svg, target)?;
                changed = true;
            }
        }
        Ok(changed)
    }

    /// Cria um decalque a partir de SVG: rasteriza (maior lado = `max_px`,
    /// limitado por [`crate::svg::MAX_RASTER_PX`]) e guarda o texto de origem.
    ///
    /// `scale_uv_width` é a largura em UV; a altura segue a proporção do SVG.
    /// Largura não finita ou `<= 0` devolve [`SvgError::ZeroSize`].
    pub fn from_svg(
        svg: &str,
        max_px: u32,
        center_uv: [f32; 2],
        scale_uv_width: f32,
    ) -> Result<Self, SvgError> {
        if !(scale_uv_width.is_finite() && scale_uv_width > 0.0) {
            return Err(SvgError::ZeroSize);
        }
        let info = svg_info(svg)?;
        let image = rasterize_svg(svg, max_px)?;
        let aspect = info.height / info.width;
        let mut decal = Self::new(
            image,
            center_uv,
            [scale_uv_width, scale_uv_width * aspect],
            0.0,
        );
        decal.source_svg = Some(svg.to_owned());
        Ok(decal)
    }

    /// Re-rasteriza a partir de `source_svg` (maior lado = `max_px`).
    ///
    /// `Ok(false)` quando não há SVG de origem (decalque raster comum, nada
    /// muda); `Ok(true)` quando a imagem foi refeita. Em erro o decalque fica
    /// intacto. Posição, escala e rotação não mudam.
    pub fn rerasterize(&mut self, max_px: u32) -> Result<bool, SvgError> {
        let Some(source) = self.source_svg.as_deref() else {
            return Ok(false);
        };
        self.image = rasterize_svg(source, max_px)?;
        if let Some(variant) = self.variants.get_mut(self.variant_index) {
            variant.image = self.image.clone();
        }
        Ok(true)
    }
}

/// Conteúdo específico da camada (Raster, Decal ou Efeito).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayerKind {
    /// Camada de pintura raster comum com canvas próprio (P3D-061).
    Raster(Canvas),
    /// Camada de decalque / estampa projetada sobre o UV (P3D-133, pós-V1).
    Decal(DecalLayer),
    /// Camada de efeito não-destrutivo aplicada sobre a composição inferior (P3D-134, pós-V1).
    Effect(PaintEffect),
}

/// Camada de pintura unificada com suporte a raster, decalques e efeitos (P3D-061, P3D-133, P3D-134).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PaintLayer {
    pub id: uuid::Uuid,
    pub name: String,
    pub visible: bool,
    pub opacity: f32,
    pub blend: LayerBlendMode,
    pub kind: LayerKind,
    #[serde(default)]
    pub locked: bool,
    /// Parent group id; `None` = root. Simple tree, not a DAG.
    #[serde(default)]
    pub group_id: Option<uuid::Uuid>,
    #[serde(default)]
    pub is_group: bool,
}

impl PaintLayer {
    pub fn new(name: impl Into<String>, w: u32, h: u32, fill: [u8; 4]) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: LayerBlendMode::Normal,
            kind: LayerKind::Raster(Canvas::new(w, h, fill)),
            locked: false,
            group_id: None,
            is_group: false,
        }
    }

    pub fn new_raster(name: impl Into<String>, canvas: Canvas) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: LayerBlendMode::Normal,
            kind: LayerKind::Raster(canvas),
            locked: false,
            group_id: None,
            is_group: false,
        }
    }

    pub fn new_decal(name: impl Into<String>, decal: DecalLayer) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: LayerBlendMode::Normal,
            kind: LayerKind::Decal(decal),
            locked: false,
            group_id: None,
            is_group: false,
        }
    }

    pub fn new_effect(name: impl Into<String>, effect: PaintEffect) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: LayerBlendMode::Normal,
            kind: LayerKind::Effect(effect),
            locked: false,
            group_id: None,
            is_group: false,
        }
    }

    pub fn new_group(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: LayerBlendMode::Normal,
            kind: LayerKind::Raster(Canvas::new(1, 1, [0, 0, 0, 0])),
            locked: false,
            group_id: None,
            is_group: true,
        }
    }

    pub fn canvas(&self) -> Option<&Canvas> {
        match &self.kind {
            LayerKind::Raster(c) => Some(c),
            LayerKind::Decal(d) => Some(&d.image),
            LayerKind::Effect(_) => None,
        }
    }

    pub fn canvas_mut(&mut self) -> Option<&mut Canvas> {
        match &mut self.kind {
            LayerKind::Raster(c) => Some(c),
            LayerKind::Decal(d) => Some(&mut d.image),
            LayerKind::Effect(_) => None,
        }
    }

    /// Só camadas Raster aceitam pinceladas (decal/efeito são pós-V1).
    pub fn is_paintable(&self) -> bool {
        matches!(&self.kind, LayerKind::Raster(_))
    }
}

/// Mistura dois pixels com modo de mesclagem e opacidade.
pub fn blend_pixels(dst: [u8; 4], src: [u8; 4], opacity: f32, mode: LayerBlendMode) -> [u8; 4] {
    let alpha = (src[3] as f32 / 255.0) * opacity.clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return dst;
    }

    let (sr, sg, sb) = (src[0] as f32, src[1] as f32, src[2] as f32);
    let (dr, dg, db) = (dst[0] as f32, dst[1] as f32, dst[2] as f32);

    let (mr, mg, mb) = match mode {
        LayerBlendMode::Normal => (sr, sg, sb),
        LayerBlendMode::Multiply => (sr * dr / 255.0, sg * dg / 255.0, sb * db / 255.0),
        LayerBlendMode::Add => (
            (sr + dr).min(255.0),
            (sg + dg).min(255.0),
            (sb + db).min(255.0),
        ),
        LayerBlendMode::Screen => (
            255.0 - ((255.0 - sr) * (255.0 - dr) / 255.0),
            255.0 - ((255.0 - sg) * (255.0 - dg) / 255.0),
            255.0 - ((255.0 - sb) * (255.0 - db) / 255.0),
        ),
    };

    let out_r = (dr * (1.0 - alpha) + mr * alpha).round().clamp(0.0, 255.0) as u8;
    let out_g = (dg * (1.0 - alpha) + mg * alpha).round().clamp(0.0, 255.0) as u8;
    let out_b = (db * (1.0 - alpha) + mb * alpha).round().clamp(0.0, 255.0) as u8;
    let out_a = (dst[3] as f32 * (1.0 - alpha) + src[3] as f32 * alpha)
        .round()
        .clamp(0.0, 255.0) as u8;

    [out_r, out_g, out_b, out_a]
}

/// Pilha unificada de camadas de pintura, decalques e efeitos (P3D-061, P3D-133, P3D-134).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct PaintLayerStack {
    pub layers: Vec<PaintLayer>,
    pub active_layer: usize,
}

impl PaintLayerStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pilha inicial V1: uma camada Raster sobre o canvas base.
    pub fn with_base(name: impl Into<String>, canvas: Canvas) -> Self {
        let mut stack = Self::new();
        stack.add_layer(PaintLayer::new_raster(name, canvas));
        stack
    }

    pub fn add_layer(&mut self, layer: PaintLayer) -> uuid::Uuid {
        let id = layer.id;
        self.layers.push(layer);
        self.active_layer = self.layers.len() - 1;
        id
    }

    pub fn add_group(&mut self, name: impl Into<String>) -> uuid::Uuid {
        self.add_layer(PaintLayer::new_group(name))
    }

    pub fn set_parent(&mut self, child: uuid::Uuid, parent: Option<uuid::Uuid>) -> bool {
        if parent == Some(child) {
            return false;
        }
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == child) {
            layer.group_id = parent;
            true
        } else {
            false
        }
    }

    pub fn set_locked(&mut self, id: uuid::Uuid, locked: bool) -> bool {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id) {
            layer.locked = locked;
            true
        } else {
            false
        }
    }

    pub fn remove_layer(&mut self, id: uuid::Uuid) -> bool {
        if let Some(pos) = self.layers.iter().position(|l| l.id == id) {
            self.layers.remove(pos);
            if self.active_layer >= self.layers.len() && !self.layers.is_empty() {
                self.active_layer = self.layers.len() - 1;
            }
            true
        } else {
            false
        }
    }

    pub fn move_layer(&mut self, from: usize, to: usize) -> bool {
        if from < self.layers.len() && to < self.layers.len() && from != to {
            let l = self.layers.remove(from);
            self.layers.insert(to, l);
            self.active_layer = to;
            true
        } else {
            false
        }
    }

    /// Funde a camada na posição `pos` com a camada imediatamente abaixo (`pos - 1`).
    pub fn merge_down(&mut self, pos: usize) -> bool {
        self.merge_down_on(pos, None)
    }

    /// Como [`Self::merge_down`], com a malha para decalques de superfície.
    pub fn merge_down_on(&mut self, pos: usize, mesh: Option<&Mesh>) -> bool {
        if pos == 0 || pos >= self.layers.len() {
            return false;
        }
        let lower_idx = pos - 1;
        if self.layers[lower_idx].locked {
            return false;
        }
        let (w, h) = if let Some(cv) = self.layers[lower_idx].canvas() {
            (cv.w, cv.h)
        } else if let Some(cv) = self.layers[pos].canvas() {
            (cv.w, cv.h)
        } else {
            (256, 256)
        };
        if self.layers[lower_idx].canvas().is_none() {
            self.layers[lower_idx].kind = LayerKind::Raster(Canvas::new(w, h, [0, 0, 0, 0]));
        }

        let upper = self.layers.remove(pos);
        let lower = &mut self.layers[lower_idx];
        if let Some(lower_cv) = lower.canvas_mut()
            && upper.visible
            && upper.opacity > 0.0
        {
            match &upper.kind {
                LayerKind::Raster(upper_cv) => {
                    let blend_w = lower_cv.w.min(upper_cv.w);
                    let blend_h = lower_cv.h.min(upper_cv.h);
                    for y in 0..blend_h {
                        for x in 0..blend_w {
                            if let (Some(dst), Some(src)) = (lower_cv.get(x, y), upper_cv.get(x, y))
                            {
                                let blended = blend_pixels(dst, src, upper.opacity, upper.blend);
                                lower_cv.set(x, y, blended);
                            }
                        }
                    }
                }
                LayerKind::Decal(decal) => {
                    let (w, h) = (lower_cv.w, lower_cv.h);
                    if decal.anchor.is_some() {
                        for (x, y, src) in mesh
                            .map(|mesh| decal.surface_samples(mesh, w, h))
                            .unwrap_or_default()
                        {
                            if let Some(dst) = lower_cv.get(x, y) {
                                lower_cv.set(
                                    x,
                                    y,
                                    blend_pixels(dst, src, upper.opacity, upper.blend),
                                );
                            }
                        }
                    } else {
                        for y in 0..h {
                            for x in 0..w {
                                if let (Some(dst), Some(src)) =
                                    (lower_cv.get(x, y), decal.uv_sample(x, y, w, h))
                                {
                                    lower_cv.set(
                                        x,
                                        y,
                                        blend_pixels(dst, src, upper.opacity, upper.blend),
                                    );
                                }
                            }
                        }
                    }
                }
                LayerKind::Effect(effect) => {
                    if upper.opacity >= 1.0 {
                        apply_effect(lower_cv, effect);
                    } else {
                        let before = lower_cv.clone();
                        apply_effect(lower_cv, effect);
                        for y in 0..lower_cv.h {
                            for x in 0..lower_cv.w {
                                if let (Some(dst), Some(src)) =
                                    (before.get(x, y), lower_cv.get(x, y))
                                {
                                    let blended = blend_pixels(
                                        dst,
                                        src,
                                        upper.opacity,
                                        LayerBlendMode::Normal,
                                    );
                                    lower_cv.set(x, y, blended);
                                }
                            }
                        }
                    }
                }
            }
        }
        self.active_layer = lower_idx;
        true
    }

    pub fn active(&self) -> Option<&PaintLayer> {
        self.layers.get(self.active_layer)
    }

    pub fn active_mut(&mut self) -> Option<&mut PaintLayer> {
        self.layers.get_mut(self.active_layer)
    }

    pub fn set_active(&mut self, id: uuid::Uuid) -> bool {
        if let Some(pos) = self.layers.iter().position(|l| l.id == id) {
            self.active_layer = pos;
            true
        } else {
            false
        }
    }

    /// Updates the coordinates, scale and rotation of a Decal layer (P3D-133).
    /// Atualiza as coordenadas, escala e rotação de uma camada Decal (P3D-133).
    pub fn set_decal_transform(
        &mut self,
        id: uuid::Uuid,
        center_uv: [f32; 2],
        scale_uv: [f32; 2],
        rotation_rad: f32,
    ) -> bool {
        if !center_uv[0].is_finite()
            || !center_uv[1].is_finite()
            || !scale_uv[0].is_finite()
            || !scale_uv[1].is_finite()
            || !rotation_rad.is_finite()
            || scale_uv[0] <= 0.0
            || scale_uv[1] <= 0.0
        {
            return false;
        }
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id)
            && let LayerKind::Decal(ref mut decal) = layer.kind
        {
            decal.rotation_rad = rotation_rad;
            if let Some(anchor) = decal.anchor.as_mut() {
                // Decalque de superfície: a posição vem da fixação 3D e a
                // altura da proporção da imagem; `scale_uv[0]` é a largura.
                anchor.width = scale_uv[0];
                anchor.depth = scale_uv[0] * 0.5;
            } else {
                decal.center_uv = center_uv;
                decal.scale_uv = scale_uv;
            }
            return true;
        }
        false
    }

    /// Converts a Decal layer into a static Raster layer by baking its projection (P3D-160).
    /// Converte uma camada de Decal em Raster aplicando sua projeção estaticamente (P3D-160).
    pub fn bake_decal_to_raster(&mut self, id: uuid::Uuid, target_w: u32, target_h: u32) -> bool {
        self.bake_decal_to_raster_on(id, target_w, target_h, None)
    }

    /// Como [`Self::bake_decal_to_raster`], com a malha para decalques de
    /// superfície (sem malha eles não têm onde cair e ficam transparentes).
    pub fn bake_decal_to_raster_on(
        &mut self,
        id: uuid::Uuid,
        target_w: u32,
        target_h: u32,
        mesh: Option<&Mesh>,
    ) -> bool {
        if target_w == 0 || target_h == 0 {
            return false;
        }
        let Some(pos) = self.layers.iter().position(|l| l.id == id) else {
            return false;
        };
        let LayerKind::Decal(decal) = &self.layers[pos].kind else {
            return false;
        };

        let mut baked = Canvas::new(target_w, target_h, [0, 0, 0, 0]);
        if decal.anchor.is_some() {
            if let Some(mesh) = mesh {
                for (x, y, src) in decal.surface_samples(mesh, target_w, target_h) {
                    baked.set(x, y, src);
                }
            }
        } else {
            for y in 0..target_h {
                for x in 0..target_w {
                    if let Some(src) = decal.uv_sample(x, y, target_w, target_h) {
                        baked.set(x, y, src);
                    }
                }
            }
        }

        self.layers[pos].kind = LayerKind::Raster(baked);
        true
    }

    /// Executa a composição determinística de todas as camadas sobre o canvas base.
    pub fn composite(&self, base: &mut Canvas) {
        self.composite_on(base, None, None);
    }

    /// Composição com a malha do asset: decalques de superfície caem nos
    /// texels pela posição 3D (sem malha, são ignorados). `time` (segundos)
    /// escolhe a variante dos decalques com trilha; `None` = estado parado.
    pub fn composite_on(&self, base: &mut Canvas, mesh: Option<&Mesh>, time: Option<f32>) {
        for layer in &self.layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }

            match &layer.kind {
                LayerKind::Raster(canvas) => {
                    let w = base.w.min(canvas.w);
                    let h = base.h.min(canvas.h);
                    for y in 0..h {
                        for x in 0..w {
                            if let (Some(dst), Some(src)) = (base.get(x, y), canvas.get(x, y)) {
                                let blended = blend_pixels(dst, src, layer.opacity, layer.blend);
                                base.set(x, y, blended);
                            }
                        }
                    }
                }
                LayerKind::Decal(decal) => {
                    let (w, h) = (base.w, base.h);
                    let image = decal.image_at(time);
                    if decal.anchor.is_some() {
                        if let Some(mesh) = mesh {
                            for (x, y, src) in decal.surface_samples_of(image, mesh, w, h) {
                                if let Some(dst) = base.get(x, y) {
                                    base.set(
                                        x,
                                        y,
                                        blend_pixels(dst, src, layer.opacity, layer.blend),
                                    );
                                }
                            }
                        }
                        continue;
                    }
                    for y in 0..h {
                        for x in 0..w {
                            if let (Some(dst), Some(src)) =
                                (base.get(x, y), decal.uv_sample_of(image, x, y, w, h))
                            {
                                base.set(x, y, blend_pixels(dst, src, layer.opacity, layer.blend));
                            }
                        }
                    }
                }
                LayerKind::Effect(effect) => {
                    // Efeito a 100%: aplica direto. Com opacidade parcial:
                    // aplica numa cópia e re-mescla antes/depois (idempotente).
                    if layer.opacity >= 1.0 {
                        apply_effect(&mut *base, effect);
                    } else {
                        let before = base.clone();
                        apply_effect(&mut *base, effect);
                        for y in 0..base.h {
                            for x in 0..base.w {
                                if let (Some(orig), Some(current)) =
                                    (before.get(x, y), base.get(x, y))
                                {
                                    let blended = blend_pixels(
                                        orig,
                                        current,
                                        layer.opacity,
                                        LayerBlendMode::Normal,
                                    );
                                    base.set(x, y, blended);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// True se a composição parcial por tiles é **equivalente** à completa.
    ///
    /// P3D-061: "composite deve ser cacheável e evitar recomposição integral".
    /// Efeitos com amostragem de vizinhança além do pixel (Pixelate) impedem
    /// tiling seguro; os per-pixel (Grain/Levels/BrightnessContrast/
    /// HueSaturation/Invert/Posterize) são locais e permanecem tileáveis.
    pub fn is_tileable(&self) -> bool {
        !self
            .layers
            .iter()
            .any(|l| matches!(l.kind, LayerKind::Effect(PaintEffect::Pixelate { .. })))
    }

    /// Ajusta a resolução das variantes SVG do decalque `id` à área que ele
    /// ocupa numa textura `w × h` (ver [`DecalLayer::fit_svg_resolution`]).
    pub fn fit_decal_svg(
        &mut self,
        id: uuid::Uuid,
        mesh: Option<&Mesh>,
        w: u32,
        h: u32,
    ) -> Result<bool, SvgError> {
        let Some(LayerKind::Decal(decal)) = self
            .layers
            .iter_mut()
            .find(|layer| layer.id == id)
            .map(|layer| &mut layer.kind)
        else {
            return Ok(false);
        };
        match decal.needed_raster_px(mesh, w, h) {
            Some(needed) => decal.fit_svg_resolution(needed),
            None => Ok(false),
        }
    }

    /// Tiles (indexação de [`Self::composite_tiles`]) que cobrem o retângulo
    /// de texels `[x0, y0, x1, y1)` numa textura `w × h`.
    pub fn tiles_for_bounds(bounds: [u32; 4], w: u32, h: u32) -> Vec<u32> {
        let [x0, y0, x1, y1] = bounds;
        if w == 0 || h == 0 || x0 >= x1.min(w) || y0 >= y1.min(h) {
            return Vec::new();
        }
        let tiles_x = w.div_ceil(TILE_SIZE).max(1);
        let (tx0, ty0) = (x0 / TILE_SIZE, y0 / TILE_SIZE);
        let (tx1, ty1) = ((x1.min(w) - 1) / TILE_SIZE, (y1.min(h) - 1) / TILE_SIZE);
        (ty0..=ty1)
            .flat_map(|ty| (tx0..=tx1).map(move |tx| ty * tiles_x + tx))
            .collect()
    }

    /// Recompõe apenas os tiles listados sobre `out`, que **deve conter o
    /// resultado da composição anterior** (completa ou parcial).
    ///
    /// `dirty_tiles` usa indexação por linha: `tile = ty * tiles_x + tx`,
    /// com tiles de [`TILE_SIZE`] a partir do canto superior esquerdo.
    /// Exige [`Self::is_tileable`] — em stacks com Pixelate use `composite`.
    pub fn composite_tiles(&self, out: &mut Canvas, dirty_tiles: &[u32]) {
        self.composite_tiles_on(out, dirty_tiles, None, None);
    }

    /// Composição parcial com a malha (decalques de superfície) e o instante
    /// da reprodução (variantes com trilha).
    pub fn composite_tiles_on(
        &self,
        out: &mut Canvas,
        dirty_tiles: &[u32],
        mesh: Option<&Mesh>,
        time: Option<f32>,
    ) {
        assert!(
            self.is_tileable(),
            "composição parcial exige stack tileável (sem Pixelate)"
        );
        if dirty_tiles.is_empty() {
            return;
        }
        let tiles_x = out.w.div_ceil(TILE_SIZE).max(1);
        // Projeção de cada decalque de superfície, uma vez por chamada.
        let surface: Vec<Vec<(u32, u32, [u8; 4])>> = self
            .layers
            .iter()
            .map(|layer| match (&layer.kind, mesh) {
                (LayerKind::Decal(decal), Some(mesh)) if decal.anchor.is_some() => {
                    decal.surface_samples_of(decal.image_at(time), mesh, out.w, out.h)
                }
                _ => Vec::new(),
            })
            .collect();
        for tile in dirty_tiles {
            let tx = tile % tiles_x;
            let ty = tile / tiles_x;
            let x0 = tx * TILE_SIZE;
            let y0 = ty * TILE_SIZE;
            let x1 = (x0 + TILE_SIZE).min(out.w);
            let y1 = (y0 + TILE_SIZE).min(out.h);
            self.composite_tile_region(out, [x0, y0, x1, y1], &surface, time);
        }
    }

    /// Recompõe a região de um tile: reset da base (primeiro layer) sobre
    /// transparente, depois os layers restantes com blend — a mesma
    /// aritmética do `composite` completo sobre scratch transparente.
    fn composite_tile_region(
        &self,
        out: &mut Canvas,
        [x0, y0, x1, y1]: [u32; 4],
        surface: &[Vec<(u32, u32, [u8; 4])>],
        time: Option<f32>,
    ) {
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for (li, layer) in self.layers.iter().enumerate() {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            match &layer.kind {
                LayerKind::Raster(canvas) => {
                    for y in y0..y1 {
                        for x in x0..x1 {
                            let Some(src) = canvas.get(x, y) else {
                                continue;
                            };
                            let v = if li == 0 {
                                blend_pixels([0, 0, 0, 0], src, layer.opacity, layer.blend)
                            } else {
                                let Some(dst) = out.get(x, y) else {
                                    continue;
                                };
                                blend_pixels(dst, src, layer.opacity, layer.blend)
                            };
                            out.set(x, y, v);
                        }
                    }
                }
                LayerKind::Decal(decal) => {
                    if decal.anchor.is_some() {
                        for &(x, y, src) in surface.get(li).map_or(&[][..], Vec::as_slice) {
                            if (x0..x1).contains(&x) && (y0..y1).contains(&y) {
                                let dst = out.get(x, y).unwrap_or([0, 0, 0, 0]);
                                out.set(x, y, blend_pixels(dst, src, layer.opacity, layer.blend));
                            }
                        }
                        continue;
                    }
                    let (w, h) = (out.w, out.h);
                    let image = decal.image_at(time);
                    for y in y0..y1 {
                        for x in x0..x1 {
                            let Some(src) = decal.uv_sample_of(image, x, y, w, h) else {
                                continue;
                            };
                            let dst = out.get(x, y).unwrap_or([0, 0, 0, 0]);
                            out.set(x, y, blend_pixels(dst, src, layer.opacity, layer.blend));
                        }
                    }
                }
                LayerKind::Effect(effect) => {
                    // Só efeitos per-pixel chegam aqui (guardado por `is_tileable`).
                    for y in y0..y1 {
                        for x in x0..x1 {
                            if let Some(c) = out.get(x, y) {
                                let value = effect_pixel(effect, x, y, c);
                                out.set(
                                    x,
                                    y,
                                    blend_pixels(c, value, layer.opacity, LayerBlendMode::Normal),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Versão per-pixel de um efeito tileável (sem amostragem de vizinhança).
/// Espelha a aritmética de [`apply_effect`] para os efeitos locais.
fn effect_pixel(effect: &PaintEffect, x: u32, y: u32, c: [u8; 4]) -> [u8; 4] {
    match effect {
        PaintEffect::Posterize { levels } => {
            let n = (*levels).max(2) as f32;
            let step = 255.0 / (n - 1.0);
            let q = |v: u8| -> u8 {
                ((v as f32 / 255.0 * (n - 1.0)).round() * step).clamp(0.0, 255.0) as u8
            };
            [q(c[0]), q(c[1]), q(c[2]), c[3]]
        }
        PaintEffect::Invert => [255 - c[0], 255 - c[1], 255 - c[2], c[3]],
        PaintEffect::Grain { intensity, seed } => {
            let k = intensity.clamp(0.0, 1.0);
            let n = hash_noise(x, y, *seed);
            let d = (n * k * 255.0) as i32;
            [
                (c[0] as i32 + d).clamp(0, 255) as u8,
                (c[1] as i32 + d).clamp(0, 255) as u8,
                (c[2] as i32 + d).clamp(0, 255) as u8,
                c[3],
            ]
        }
        PaintEffect::Levels {
            in_min,
            in_max,
            gamma,
            out_min,
            out_max,
        } => {
            let (lo, hi) = (
                in_min.clamp(0.0, 1.0),
                in_max.clamp(0.0, 1.0).max(in_min.clamp(0.0, 1.0) + 1e-3),
            );
            let (olo, ohi) = (out_min.clamp(0.0, 1.0), out_max.clamp(0.0, 1.0));
            let g = gamma.clamp(0.1, 10.0);
            let mapped = |v: u8| -> u8 {
                let t = v as f32 / 255.0;
                let out = if t <= lo {
                    olo
                } else if t >= hi {
                    ohi
                } else {
                    let n = (t - lo) / (hi - lo);
                    olo + n.powf(g) * (ohi - olo)
                };
                (out * 255.0).round().clamp(0.0, 255.0) as u8
            };
            [mapped(c[0]), mapped(c[1]), mapped(c[2]), c[3]]
        }
        PaintEffect::BrightnessContrast {
            brightness,
            contrast,
        } => {
            let b = brightness.clamp(-1.0, 1.0);
            let ct = contrast.clamp(-1.0, 1.0);
            let adj = |v: u8| -> u8 {
                let t = (v as f32 - 127.5) * (1.0 + ct) + 127.5 + b * 127.5;
                t.round().clamp(0.0, 255.0) as u8
            };
            [adj(c[0]), adj(c[1]), adj(c[2]), c[3]]
        }
        PaintEffect::HueSaturation {
            hue_shift_deg,
            saturation,
        } => {
            let shift = hue_shift_deg % 360.0;
            let sat_scale = (1.0 + saturation.clamp(-1.0, 1.0)).max(0.0);
            let (hue, sat, val) = rgb_to_hsv(c[0], c[1], c[2]);
            let h2 = (hue + shift + 360.0) % 360.0;
            let s2 = (sat * sat_scale).clamp(0.0, 1.0);
            let (r2, g2, b2) = hsv_to_rgb(h2, s2, val);
            [r2, g2, b2, c[3]]
        }
        // Pixelate não é tileável — nunca chega aqui (guardado por `is_tileable`).
        PaintEffect::Pixelate { .. } => c,
    }
}

/// Aplica um `PaintEffect` sobre o canvas (opacidade 100%).
///
/// Função compartilhada entre a pilha de camadas (P3D-134) e o Surface
/// Recipe graph (P3D-113/cap. 42: "efeitos são node groups internamente;
/// presets são a superfície"). Determinística: mesma entrada ⇒ mesma saída.
pub fn apply_effect(canvas: &mut Canvas, effect: &PaintEffect) {
    let w = canvas.w;
    let h = canvas.h;
    match effect {
        PaintEffect::Pixelate { cell_size } => {
            let step = (*cell_size).max(1);
            for y_block in (0..h).step_by(step as usize) {
                for x_block in (0..w).step_by(step as usize) {
                    if let Some(sample) = canvas.get(x_block, y_block) {
                        for dy in 0..step {
                            for dx in 0..step {
                                let px = x_block + dx;
                                let py = y_block + dy;
                                if px < w && py < h {
                                    canvas.set(px, py, sample);
                                }
                            }
                        }
                    }
                }
            }
        }
        PaintEffect::Posterize { levels } => {
            let n = (*levels).max(2) as f32;
            let step = 255.0 / (n - 1.0);
            for y in 0..h {
                for x in 0..w {
                    if let Some(c) = canvas.get(x, y) {
                        let q = |v: u8| -> u8 {
                            ((v as f32 / 255.0 * (n - 1.0)).round() * step).clamp(0.0, 255.0) as u8
                        };
                        canvas.set(x, y, [q(c[0]), q(c[1]), q(c[2]), c[3]]);
                    }
                }
            }
        }
        PaintEffect::Invert => {
            for y in 0..h {
                for x in 0..w {
                    if let Some(c) = canvas.get(x, y) {
                        canvas.set(x, y, [255 - c[0], 255 - c[1], 255 - c[2], c[3]]);
                    }
                }
            }
        }
        PaintEffect::Grain { intensity, seed } => {
            let k = intensity.clamp(0.0, 1.0);
            for y in 0..h {
                for x in 0..w {
                    if let Some(c) = canvas.get(x, y) {
                        let n = hash_noise(x, y, *seed);
                        let d = (n * k * 255.0) as i32;
                        canvas.set(
                            x,
                            y,
                            [
                                (c[0] as i32 + d).clamp(0, 255) as u8,
                                (c[1] as i32 + d).clamp(0, 255) as u8,
                                (c[2] as i32 + d).clamp(0, 255) as u8,
                                c[3],
                            ],
                        );
                    }
                }
            }
        }
        PaintEffect::Levels {
            in_min,
            in_max,
            gamma,
            out_min,
            out_max,
        } => {
            let (lo, hi) = (
                in_min.clamp(0.0, 1.0),
                in_max.clamp(0.0, 1.0).max(in_min.clamp(0.0, 1.0) + 1e-3),
            );
            let (olo, ohi) = (out_min.clamp(0.0, 1.0), out_max.clamp(0.0, 1.0));
            let g = gamma.clamp(0.1, 10.0);
            for y in 0..h {
                for x in 0..w {
                    if let Some(c) = canvas.get(x, y) {
                        let mapped = |v: u8| -> u8 {
                            let t = v as f32 / 255.0;
                            let out = if t <= lo {
                                olo
                            } else if t >= hi {
                                ohi
                            } else {
                                let n = (t - lo) / (hi - lo);
                                olo + n.powf(g) * (ohi - olo)
                            };
                            (out * 255.0).round().clamp(0.0, 255.0) as u8
                        };
                        canvas.set(x, y, [mapped(c[0]), mapped(c[1]), mapped(c[2]), c[3]]);
                    }
                }
            }
        }
        PaintEffect::BrightnessContrast {
            brightness,
            contrast,
        } => {
            let b = brightness.clamp(-1.0, 1.0);
            let ct = contrast.clamp(-1.0, 1.0);
            for y in 0..h {
                for x in 0..w {
                    if let Some(c) = canvas.get(x, y) {
                        let adj = |v: u8| -> u8 {
                            let t = (v as f32 - 127.5) * (1.0 + ct) + 127.5 + b * 127.5;
                            t.round().clamp(0.0, 255.0) as u8
                        };
                        canvas.set(x, y, [adj(c[0]), adj(c[1]), adj(c[2]), c[3]]);
                    }
                }
            }
        }
        PaintEffect::HueSaturation {
            hue_shift_deg,
            saturation,
        } => {
            let shift = hue_shift_deg % 360.0;
            let sat_scale = (1.0 + saturation.clamp(-1.0, 1.0)).max(0.0);
            for y in 0..h {
                for x in 0..w {
                    if let Some(c) = canvas.get(x, y) {
                        let (hue, sat, val) = rgb_to_hsv(c[0], c[1], c[2]);
                        let h2 = (hue + shift + 360.0) % 360.0;
                        let s2 = (sat * sat_scale).clamp(0.0, 1.0);
                        let (r2, g2, b2) = hsv_to_rgb(h2, s2, val);
                        canvas.set(x, y, [r2, g2, b2, c[3]]);
                    }
                }
            }
        }
    }
}

/// Hash determinístico por pixel para Grain (mesmo seed ⇒ mesmo ruído).
fn hash_noise(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = seed ^ x.wrapping_mul(0x9E37_79B9) ^ y.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^= h >> 16;
    h as f32 / u32::MAX as f32 * 2.0 - 1.0
}

/// RGB [0..255] → HSV (hue 0..360, s/v 0..1).
fn rgb_to_hsv(r: u8, g: u8, b: u8) -> (f32, f32, f32) {
    let (r, g, b) = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let delta = max - min;
    let hue = if delta <= f32::EPSILON {
        0.0
    } else if max == r {
        60.0 * (((g - b) / delta) % 6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    let hue = if hue < 0.0 { hue + 360.0 } else { hue };
    let sat = if max <= f32::EPSILON {
        0.0
    } else {
        delta / max
    };
    (hue, sat, max)
}

/// HSV (hue 0..360, s/v 0..1) → RGB [0..255].
fn hsv_to_rgb(hue: f32, sat: f32, val: f32) -> (u8, u8, u8) {
    let c = val * sat;
    let hp = hue / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = val - c;
    (
        ((r + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).round().clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).round().clamp(0.0, 255.0) as u8,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Faces do cubo com a normal mais próxima de `dir`.
    fn cube_face(mesh: &Mesh, dir: Vec3) -> usize {
        (0..mesh.faces.len())
            .max_by(|&a, &b| {
                mesh.face_normal(a)
                    .dot(dir)
                    .total_cmp(&mesh.face_normal(b).dot(dir))
            })
            .unwrap()
    }

    fn surface_decal(width: f32, normal: Vec3, point: Vec3, image: Canvas) -> DecalLayer {
        let mut decal = DecalLayer::new(image, [0.5; 2], [0.3; 2], 0.0);
        decal.anchor = DecalAnchor::new(point, normal, Vec3::Y, width);
        decal
    }

    fn variant(name: &str, color: [u8; 4]) -> DecalVariant {
        DecalVariant {
            name: name.to_owned(),
            image: Canvas::new(8, 8, color),
            source_svg: None,
        }
    }

    #[test]
    fn variant_track_steps_without_interpolation_and_loops() {
        let mut track = DecalVariantTrack {
            fps: 12.0,
            length_frames: 12,
            looping: true,
            keys: Vec::new(),
        };
        assert_eq!(track.variant_at_frame(3), None);
        track.set_key(5, 1);
        track.set_key(0, 0);
        track.set_key(8, 2);
        track.set_key(5, 3); // substitui, não duplica
        assert_eq!(track.keys.len(), 3);
        assert_eq!(track.variant_at_frame(4), Some(0));
        assert_eq!(track.variant_at_frame(5), Some(3));
        assert_eq!(track.variant_at_frame(11), Some(2));
        // 1 s a 12 fps = quadro 12, que dá a volta para 0.
        assert_eq!(track.frame_at(1.0), 0);
        assert_eq!(track.frame_at(0.5), 6);
        track.looping = false;
        assert_eq!(track.frame_at(5.0), 11);
        assert!(track.remove_key(8));
        assert!(!track.remove_key(8));
        track.set_key(30, 1);
        assert_eq!(track.length_frames, 31, "a duração cresce até a chave");
    }

    #[test]
    fn decal_set_variants_switch_remove_and_follow_the_track() {
        let red = Canvas::new(8, 8, [255, 0, 0, 255]);
        let mut decal = DecalLayer::new(red.clone(), [0.5; 2], [0.3; 2], 0.0);
        assert_eq!(decal.variant_count(), 1);
        assert_eq!(
            decal.add_variant("Neutra", variant("Sorriso", [0, 255, 0, 255])),
            1
        );
        decal.add_variant("Neutra", variant("Aberta", [0, 0, 255, 255]));
        assert_eq!(decal.variant_count(), 3);
        assert_eq!(decal.variant_name(0, "x"), "Neutra");
        assert!(decal.set_variant(2));
        assert_eq!(decal.image.get(0, 0), Some([0, 0, 255, 255]));

        let mut track = DecalVariantTrack::default();
        track.set_key(0, 0);
        track.set_key(6, 1);
        track.set_key(9, 2);
        decal.track = Some(track);
        let fps = decal.track.as_ref().unwrap().fps;
        assert_eq!(decal.variant_at(None), 2, "parado: a variante escolhida");
        assert_eq!(decal.variant_at(Some(6.5 / fps)), 1);
        assert_eq!(
            decal.image_at(Some(0.5 / fps)).get(0, 0),
            Some([255, 0, 0, 255])
        );

        // Remover a variante 1 apaga a chave dela e desce as seguintes.
        assert!(decal.remove_variant(1));
        let keys = &decal.track.as_ref().unwrap().keys;
        assert_eq!(
            keys.iter()
                .map(|k| (k.frame, k.variant))
                .collect::<Vec<_>>(),
            vec![(0, 0), (9, 1)]
        );
        assert_eq!(decal.variant_index, 1);
        assert_eq!(decal.image.get(0, 0), Some([0, 0, 255, 255]));
        assert!(decal.remove_variant(0));
        assert!(!decal.remove_variant(0), "a última variante não sai");
    }

    #[test]
    fn composite_shows_the_variant_keyed_at_the_playback_time() {
        let mut decal =
            DecalLayer::new(Canvas::new(8, 8, [255, 0, 0, 255]), [0.5; 2], [0.5; 2], 0.0);
        decal.add_variant("A", variant("B", [0, 255, 0, 255]));
        let mut track = DecalVariantTrack::default();
        track.set_key(0, 0);
        track.set_key(3, 1);
        let fps = track.fps;
        decal.track = Some(track);
        let mut stack = PaintLayerStack::with_base("Base", Canvas::new(32, 32, [0, 0, 0, 255]));
        stack.add_layer(PaintLayer::new_decal("Boca", decal));
        let center = |time: Option<f32>| {
            let mut out = Canvas::new(32, 32, [0, 0, 0, 0]);
            stack.composite_on(&mut out, None, time);
            out.get(16, 16).unwrap()
        };
        assert_eq!(center(None), [255, 0, 0, 255]);
        assert_eq!(center(Some(1.5 / fps)), [255, 0, 0, 255]);
        assert_eq!(center(Some(3.5 / fps)), [0, 255, 0, 255]);
        // Composição por tiles no mesmo instante = completa.
        let mut full = Canvas::new(32, 32, [0, 0, 0, 0]);
        stack.composite_on(&mut full, None, Some(3.5 / fps));
        let mut tiles = Canvas::new(32, 32, [0, 0, 0, 0]);
        stack.composite_on(&mut tiles, None, None);
        stack.composite_tiles_on(&mut tiles, &[0], None, Some(3.5 / fps));
        assert_eq!(tiles.pixels.to_vec(), full.pixels.to_vec());
    }

    #[test]
    fn decal_set_round_trips_json_and_old_files_open_without_variants() {
        let mut decal = DecalLayer::new(Canvas::new(4, 4, [9, 9, 9, 255]), [0.5; 2], [0.3; 2], 0.0);
        decal.add_variant("A", variant("B", [1, 2, 3, 255]));
        decal.track = Some(DecalVariantTrack::default());
        decal.track.as_mut().unwrap().set_key(4, 1);
        let json = serde_json::to_string(&decal).unwrap();
        let back: DecalLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, decal);
        // JSON sem os campos novos (versão anterior) e índice fora da lista.
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("variants");
        object.remove("track");
        object.insert("variant_index".into(), 7.into());
        let old: DecalLayer = serde_json::from_value(value).unwrap();
        assert!(old.variants.is_empty() && old.track.is_none());
        assert_eq!(old.variant_index, 0);
    }

    #[test]
    fn svg_variants_are_rerasterized_to_the_area_they_cover() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="20" height="10" fill="red"/></svg>"#;
        let mut decal = DecalLayer::from_svg(svg, 64, [0.5; 2], 0.5).unwrap();
        decal.add_variant(
            "A",
            DecalVariant {
                name: "B".into(),
                image: crate::svg::rasterize_svg(svg, 64).unwrap(),
                source_svg: Some(svg.to_owned()),
            },
        );
        // Metade de uma textura de 1024: ~512 texels de largura.
        let needed = decal.needed_raster_px(None, 1024, 1024).unwrap();
        assert!((500..=530).contains(&needed), "{needed}");
        assert!(decal.fit_svg_resolution(needed).unwrap());
        let side = decal
            .image
            .w
            .abs_diff(needed.clamp(64, crate::svg::MAX_RASTER_PX));
        assert!(side <= 1, "o rasterizador arredonda no máximo 1 px");
        assert_eq!(
            decal.variants[1].image.w, decal.image.w,
            "a outra variante também"
        );
        assert_eq!(
            decal.variants[0].image, decal.image,
            "a cópia da mostrada segue a imagem"
        );
        // Já na resolução certa: nada muda.
        assert!(!decal.fit_svg_resolution(needed).unwrap());
        // Muito maior que o necessário: encolhe (memória).
        assert!(decal.fit_svg_resolution(100).unwrap());
        assert_eq!(decal.image.w, 100);
    }

    #[test]
    fn decal_bounds_map_to_the_tiles_they_touch() {
        assert_eq!(
            PaintLayerStack::tiles_for_bounds([0, 0, 1, 1], 128, 128),
            vec![0]
        );
        assert_eq!(
            PaintLayerStack::tiles_for_bounds([30, 30, 34, 34], 128, 128),
            vec![0, 1, 4, 5]
        );
        assert!(PaintLayerStack::tiles_for_bounds([5, 5, 5, 9], 128, 128).is_empty());
        let decal = DecalLayer::new(Canvas::new(4, 4, [0; 4]), [0.5; 2], [0.25; 2], 0.0);
        let [x0, y0, x1, y1] = decal.texel_bounds(None, 128, 128).unwrap();
        assert!(x0 <= 48 && y0 <= 48 && x1 >= 80 && y1 >= 80 && x1 - x0 <= 36);
        // Girado 45°: a caixa cresce (diagonal do quadrado).
        let turned = DecalLayer::new(
            Canvas::new(4, 4, [0; 4]),
            [0.5; 2],
            [0.25; 2],
            std::f32::consts::FRAC_PI_4,
        );
        let [a, _, b, _] = turned.texel_bounds(None, 128, 128).unwrap();
        assert!(b - a > x1 - x0);
    }

    #[test]
    fn merging_a_surface_decal_down_uses_the_mesh() {
        let mesh = Mesh::cube(2.0);
        let mut stack =
            PaintLayerStack::with_base("Base", Canvas::new(128, 128, [255, 255, 255, 255]));
        let decal = surface_decal(
            1.0,
            Vec3::Z,
            Vec3::new(0.0, 0.0, 1.0),
            Canvas::new(8, 8, [0, 0, 255, 255]),
        );
        stack.add_layer(PaintLayer::new_decal("Logo", decal));
        let mut expected = Canvas::new(128, 128, [0, 0, 0, 0]);
        stack.composite_on(&mut expected, Some(&mesh), None);
        assert!(stack.merge_down_on(1, Some(&mesh)));
        let mut merged = Canvas::new(128, 128, [0, 0, 0, 0]);
        stack.composite(&mut merged);
        assert_eq!(merged.pixels.to_vec(), expected.pixels.to_vec());
    }

    #[test]
    fn surface_decal_lands_only_where_it_was_placed() {
        let mesh = Mesh::cube(2.0);
        let (w, h) = (256, 256);
        let red = Canvas::new(16, 16, [255, 0, 0, 255]);
        let decal = surface_decal(1.0, Vec3::Z, Vec3::new(0.0, 0.0, 1.0), red);
        let samples = decal.surface_samples(&mesh, w, h);
        assert!(!samples.is_empty());
        let front = cube_face(&mesh, Vec3::Z);
        let front_mask = mesh.uv_coverage_mask([front], w, h, 1.0);
        for &(x, y, color) in &samples {
            assert!(
                front_mask.allows(x, y),
                "texel ({x},{y}) fora da face da frente"
            );
            assert_eq!(color, [255, 0, 0, 255]);
        }
        // Metade da largura e da altura da face (2 m): ~1/4 dos texels dela.
        let face_texels = mesh.uv_coverage_mask([front], w, h, 0.0).covered() as f32;
        let ratio = samples.len() as f32 / face_texels;
        assert!((0.18..0.36).contains(&ratio), "cobertura {ratio:.3}");
    }

    #[test]
    fn surface_decal_crosses_a_seam_and_keeps_the_image_aspect() {
        let mesh = Mesh::cube(2.0);
        let (w, h) = (256, 256);
        let wide = Canvas::new(32, 16, [0, 0, 255, 255]);
        // Centrado na aresta entre a frente (+Z) e a direita (+X).
        let normal = Vec3::new(1.0, 0.0, 1.0).normalize();
        let decal = surface_decal(1.2, normal, Vec3::new(1.0, 0.0, 1.0), wide.clone());
        let samples = decal.surface_samples(&mesh, w, h);
        let front = mesh.uv_coverage_mask([cube_face(&mesh, Vec3::Z)], w, h, 1.0);
        let right = mesh.uv_coverage_mask([cube_face(&mesh, Vec3::X)], w, h, 1.0);
        assert!(samples.iter().any(|&(x, y, _)| front.allows(x, y)));
        assert!(samples.iter().any(|&(x, y, _)| right.allows(x, y)));
        assert!(
            (decal.surface_height(1.2) - 0.6).abs() < 1e-6,
            "altura = largura × 16/32"
        );
        // De costas para o projetor (face de trás) nada recebe.
        let back = mesh.uv_coverage_mask([cube_face(&mesh, -Vec3::Z)], w, h, 0.0);
        assert!(
            !samples
                .iter()
                .any(|&(x, y, _)| back.allows(x, y) && !front.allows(x, y) && !right.allows(x, y))
        );
    }

    #[test]
    fn surface_decal_composites_bakes_and_round_trips() {
        let mesh = Mesh::cube(2.0);
        let mut stack =
            PaintLayerStack::with_base("Base", Canvas::new(128, 128, [255, 255, 255, 255]));
        let decal = surface_decal(
            1.0,
            Vec3::Z,
            Vec3::new(0.0, 0.0, 1.0),
            Canvas::new(8, 8, [0, 255, 0, 255]),
        );
        let id = stack.add_layer(PaintLayer::new_decal("Logo", decal.clone()));
        let mut composed = Canvas::new(128, 128, [0, 0, 0, 0]);
        stack.composite_on(&mut composed, Some(&mesh), None);
        let green = composed
            .pixels
            .chunks(4)
            .filter(|p| p[..3] == [0, 255, 0])
            .count();
        assert!(green > 0);
        // Composição parcial por tiles = completa.
        let mut tiles = Canvas::new(128, 128, [0, 0, 0, 0]);
        stack.composite_on(&mut tiles, Some(&mesh), None);
        let all: Vec<u32> = (0..(128 / TILE_SIZE) * (128 / TILE_SIZE)).collect();
        stack.composite_tiles_on(&mut tiles, &all, Some(&mesh), None);
        assert_eq!(tiles.pixels.to_vec(), composed.pixels.to_vec());
        // JSON guarda a fixação.
        let json = serde_json::to_string(&decal).unwrap();
        let back: DecalLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(back.anchor, decal.anchor);
        // Bake na mesma posição.
        assert!(stack.bake_decal_to_raster_on(id, 128, 128, Some(&mesh)));
        let mut baked = Canvas::new(128, 128, [0, 0, 0, 0]);
        stack.composite(&mut baked);
        let baked_green = baked
            .pixels
            .chunks(4)
            .filter(|p| p[..3] == [0, 255, 0])
            .count();
        assert_eq!(baked_green, green);
    }

    #[test]
    fn layer_stack_composition_is_deterministic() {
        let mut base = Canvas::new(8, 8, [0, 0, 0, 255]);
        let mut stack = PaintLayerStack::new();
        let mut layer1 = PaintLayer::new("Layer 1", 8, 8, [255, 0, 0, 255]);
        layer1.opacity = 0.5;
        stack.add_layer(layer1);
        stack.composite(&mut base);
        let px = base.get(0, 0).unwrap();
        assert!((px[0] as i32 - 128).abs() <= 2);
    }

    #[test]
    fn active_layer_tracking_survives_remove_and_move() {
        let mut stack = PaintLayerStack::new();
        let a = stack.add_layer(PaintLayer::new("A", 4, 4, [255, 0, 0, 255]));
        let b = stack.add_layer(PaintLayer::new("B", 4, 4, [0, 255, 0, 255]));
        assert!(stack.set_active(a));
        assert_eq!(stack.active().map(|l| l.id), Some(a));
        assert!(stack.move_layer(0, 1));
        assert_eq!(stack.active().map(|l| l.id), Some(a));
        assert!(stack.remove_layer(b));
        assert_eq!(stack.layers.len(), 1);
    }

    #[test]
    fn grain_is_deterministic_and_seeded() {
        let effect = PaintEffect::Grain {
            intensity: 0.5,
            seed: 42,
        };
        let mut base_a = Canvas::new(16, 16, [100, 100, 100, 255]);
        let mut base_b = Canvas::new(16, 16, [100, 100, 100, 255]);
        let mut stack = PaintLayerStack::new();
        stack.add_layer(PaintLayer::new_effect("Grain", effect));
        stack.composite(&mut base_a);
        let mut stack_b = PaintLayerStack::new();
        stack_b.add_layer(PaintLayer::new_effect("Grain", effect));
        stack_b.composite(&mut base_b);
        assert_eq!(base_a.pixels, base_b.pixels, "mesmo seed ⇒ mesmo ruído");
        assert!(
            base_a.pixels.chunks(4).any(|p| p[0] != 100),
            "intensidade > 0 deve alterar pixels"
        );
        // Seed diferente ⇒ padrão diferente (probabilidade de colisão desprezível).
        let mut base_c = Canvas::new(16, 16, [100, 100, 100, 255]);
        let mut stack_c = PaintLayerStack::new();
        stack_c.add_layer(PaintLayer::new_effect(
            "Grain",
            PaintEffect::Grain {
                intensity: 0.5,
                seed: 43,
            },
        ));
        stack_c.composite(&mut base_c);
        assert_ne!(base_a.pixels, base_c.pixels);
    }

    #[test]
    fn grain_zero_intensity_is_identity() {
        let mut base = Canvas::new(4, 4, [10, 20, 30, 255]);
        let before = base.pixels.clone();
        let mut stack = PaintLayerStack::new();
        stack.add_layer(PaintLayer::new_effect(
            "Grain",
            PaintEffect::Grain {
                intensity: 0.0,
                seed: 7,
            },
        ));
        stack.composite(&mut base);
        assert_eq!(before, base.pixels);
    }

    #[test]
    fn levels_maps_black_to_white_and_is_bounded() {
        let mut base = Canvas::new(2, 1, [0, 0, 0, 255]);
        base.set(1, 0, [255, 255, 255, 255]);
        let mut stack = PaintLayerStack::new();
        stack.add_layer(PaintLayer::new_effect(
            "Levels",
            PaintEffect::Levels {
                in_min: 0.0,
                in_max: 1.0,
                gamma: 1.0,
                out_min: 1.0,
                out_max: 0.0,
            },
        ));
        stack.composite(&mut base);
        assert_eq!(
            base.get(0, 0),
            Some([255, 255, 255, 255]),
            "preto vira branco"
        );
        assert_eq!(base.get(1, 0), Some([0, 0, 0, 255]), "branco vira preto");
    }

    #[test]
    fn brightness_contrast_flat_is_mid_gray() {
        let mut base = Canvas::new(2, 1, [0, 0, 0, 255]);
        base.set(1, 0, [255, 255, 255, 255]);
        let mut stack = PaintLayerStack::new();
        stack.add_layer(PaintLayer::new_effect(
            "Flat",
            PaintEffect::BrightnessContrast {
                brightness: 0.0,
                contrast: -1.0,
            },
        ));
        stack.composite(&mut base);
        let a = base.get(0, 0).unwrap();
        let b = base.get(1, 0).unwrap();
        assert_eq!(a, b, "contraste -1 achata tudo em cinza médio");
        assert!((a[0] as i32 - 128).abs() <= 2);
    }

    #[test]
    fn hue_shift_180_turns_red_into_cyan() {
        let mut base = Canvas::new(1, 1, [255, 0, 0, 255]);
        let mut stack = PaintLayerStack::new();
        stack.add_layer(PaintLayer::new_effect(
            "Hue 180",
            PaintEffect::HueSaturation {
                hue_shift_deg: 180.0,
                saturation: 0.0,
            },
        ));
        stack.composite(&mut base);
        let c = base.get(0, 0).unwrap();
        assert!(
            c[1] > 200 && c[2] > 200 && c[0] < 60,
            "vermelho +180° ≈ ciano: {c:?}"
        );
    }

    #[test]
    fn new_effects_serialize_roundtrip() {
        for e in [
            PaintEffect::Grain {
                intensity: 0.4,
                seed: 9,
            },
            PaintEffect::Levels {
                in_min: 0.1,
                in_max: 0.9,
                gamma: 1.5,
                out_min: 0.0,
                out_max: 1.0,
            },
            PaintEffect::BrightnessContrast {
                brightness: 0.2,
                contrast: 0.5,
            },
            PaintEffect::HueSaturation {
                hue_shift_deg: 30.0,
                saturation: -0.4,
            },
        ] {
            let json = serde_json::to_string(&e).expect("serializa");
            let back: PaintEffect = serde_json::from_str(&json).expect("desserializa");
            assert_eq!(e, back);
        }
    }

    #[test]
    fn composite_tiles_matches_full_composite() {
        // Stack com base + camada de detalhe + efeito per-pixel (tileável).
        let mut stack =
            PaintLayerStack::with_base("Base", Canvas::new(64, 64, [200, 200, 200, 255]));
        let mut detail = PaintLayer::new("Detail", 64, 64, [0, 0, 0, 0]);
        detail.opacity = 0.6;
        // Pincelada na região do tile (1,0).
        if let Some(cv) = detail.canvas_mut() {
            for y in 30..40 {
                for x in 35..50 {
                    cv.set(x, y, [255, 100, 0, 255]);
                }
            }
        }
        stack.add_layer(detail);
        stack.add_layer(PaintLayer::new_effect(
            "Grain",
            PaintEffect::Grain {
                intensity: 0.2,
                seed: 5,
            },
        ));
        assert!(stack.is_tileable());

        // Composição completa (referência).
        let mut full = Canvas::new(64, 64, [0, 0, 0, 0]);
        stack.composite(&mut full);

        // Composição parcial: todos os tiles sujos a partir do estado anterior
        // (que aqui é o resultado "antes da pincelada": compõe sem o detalhe).
        let mut prev_stack =
            PaintLayerStack::with_base("Base", Canvas::new(64, 64, [200, 200, 200, 255]));
        prev_stack.add_layer(PaintLayer::new_effect(
            "Grain",
            PaintEffect::Grain {
                intensity: 0.2,
                seed: 5,
            },
        ));
        let mut partial = Canvas::new(64, 64, [0, 0, 0, 0]);
        prev_stack.composite(&mut partial);
        // Tiles sujos: (1,0) e (1,1)? A pincelada toca x 35..50, y 30..40 →
        // tiles (1,0) e (1,1) com TILE_SIZE 32.
        stack.composite_tiles(&mut partial, &[1, 3]);

        assert_eq!(partial.pixels, full.pixels, "tiled == full");
    }

    #[test]
    fn composite_tiles_partial_keeps_clean_tiles() {
        let mut stack = PaintLayerStack::with_base("Base", Canvas::new(64, 64, [50, 50, 50, 255]));
        let mut top = PaintLayer::new("Top", 64, 64, [0, 0, 0, 0]);
        if let Some(cv) = top.canvas_mut() {
            cv.set(40, 40, [255, 0, 0, 255]);
        }
        stack.add_layer(top);

        let mut full = Canvas::new(64, 64, [0, 0, 0, 0]);
        stack.composite(&mut full);

        // Estado anterior: só a base. Suja apenas o tile (1,1) que contém (40,40).
        let mut prev = Canvas::new(64, 64, [0, 0, 0, 0]);
        PaintLayerStack::with_base("Base", Canvas::new(64, 64, [50, 50, 50, 255]))
            .composite(&mut prev);
        stack.composite_tiles(&mut prev, &[3]);

        assert_eq!(prev.pixels, full.pixels);
    }

    #[test]
    fn pixelate_makes_stack_not_tileable() {
        let mut stack = PaintLayerStack::new();
        stack.add_layer(PaintLayer::new_effect(
            "Pixelate",
            PaintEffect::Pixelate { cell_size: 4 },
        ));
        assert!(!stack.is_tileable());
    }

    #[test]
    fn test_decal_transform_and_bake_to_raster() {
        // Tests decal transform mutation and baking to a static raster layer
        // Testa a mutação de transformação do decalque e bake para camada raster estática
        let mut stack = PaintLayerStack::with_base("Base", Canvas::new(32, 32, [0, 0, 0, 255]));
        let decal_canvas = Canvas::new(8, 8, [255, 0, 0, 255]);
        let decal = DecalLayer::new(decal_canvas, [0.5, 0.5], [0.25, 0.25], 0.0);
        let decal_id = stack.add_layer(PaintLayer::new_decal("Sticker", decal));

        // Valid transform update / Atualização válida de transformação
        assert!(stack.set_decal_transform(decal_id, [0.4, 0.6], [0.3, 0.3], 0.5));
        if let Some(layer) = stack.layers.iter().find(|l| l.id == decal_id) {
            if let LayerKind::Decal(ref d) = layer.kind {
                assert_eq!(d.center_uv, [0.4, 0.6]);
                assert_eq!(d.scale_uv, [0.3, 0.3]);
                assert_eq!(d.rotation_rad, 0.5);
            } else {
                panic!("layer should be Decal / camada deveria ser Decal");
            }
        }

        // Invalid transform rejected / Transformação inválida rejeitada
        assert!(!stack.set_decal_transform(decal_id, [f32::NAN, 0.0], [0.1, 0.1], 0.0));
        assert!(!stack.set_decal_transform(decal_id, [0.0, 0.0], [-0.1, 0.1], 0.0));

        // Bake to raster / Bake para raster
        assert!(stack.bake_decal_to_raster(decal_id, 32, 32));
        if let Some(layer) = stack.layers.iter().find(|l| l.id == decal_id) {
            assert!(matches!(layer.kind, LayerKind::Raster(_)));
        } else {
            panic!("layer should exist after bake / camada deveria existir após bake");
        }
    }

    #[test]
    fn test_paint_layers_merge_down() {
        let base_canvas = Canvas::new(16, 16, [0, 0, 0, 255]);
        let mut stack = PaintLayerStack::with_base("Base", base_canvas);

        let mut top_canvas = Canvas::new(16, 16, [0, 0, 0, 0]);
        top_canvas.set(4, 4, [255, 0, 0, 255]);
        stack.add_layer(PaintLayer::new_raster("Top", top_canvas));

        assert_eq!(stack.layers.len(), 2);
        // Merge down of top layer (index 1) into base (index 0)
        assert!(stack.merge_down(1));
        assert_eq!(stack.layers.len(), 1);
        let merged_pixel = stack.layers[0].canvas().unwrap().get(4, 4).unwrap();
        assert_eq!(merged_pixel, [255, 0, 0, 255]);
        let unmodified_pixel = stack.layers[0].canvas().unwrap().get(0, 0).unwrap();
        assert_eq!(unmodified_pixel, [0, 0, 0, 255]);
    }

    const RECT_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="100">
        <rect width="200" height="100" fill="#ff0000"/></svg>"##;

    #[test]
    fn decal_from_svg_keeps_aspect_and_source() {
        let decal = DecalLayer::from_svg(RECT_SVG, 128, [0.25, 0.75], 0.4).unwrap();
        assert_eq!((decal.image.w, decal.image.h), (128, 64));
        assert_eq!(decal.center_uv, [0.25, 0.75]);
        assert_eq!(decal.scale_uv[0], 0.4);
        assert!((decal.scale_uv[1] - 0.2).abs() < 1.0e-6);
        assert_eq!(decal.rotation_rad, 0.0);
        assert_eq!(decal.source_svg.as_deref(), Some(RECT_SVG));
        assert_eq!(decal.image.get(10, 10), Some([255, 0, 0, 255]));
    }

    #[test]
    fn decal_from_svg_rejects_bad_input() {
        assert_eq!(
            DecalLayer::from_svg("", 64, [0.5; 2], 0.5).unwrap_err(),
            SvgError::Empty
        );
        assert!(matches!(
            DecalLayer::from_svg("<svg", 64, [0.5; 2], 0.5),
            Err(SvgError::Parse(_))
        ));
        for width in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            assert_eq!(
                DecalLayer::from_svg(RECT_SVG, 64, [0.5; 2], width).unwrap_err(),
                SvgError::ZeroSize
            );
        }
    }

    #[test]
    fn decal_rerasterize_changes_resolution_only() {
        let mut decal = DecalLayer::from_svg(RECT_SVG, 32, [0.5; 2], 0.5).unwrap();
        decal.rotation_rad = 0.7;
        let scale = decal.scale_uv;
        assert_eq!(decal.rerasterize(256), Ok(true));
        assert_eq!((decal.image.w, decal.image.h), (256, 128));
        assert_eq!(decal.scale_uv, scale);
        assert_eq!(decal.rotation_rad, 0.7);
    }

    #[test]
    fn decal_rerasterize_without_source_is_noop() {
        let canvas = Canvas::new(4, 4, [1, 2, 3, 255]);
        let mut decal = DecalLayer::new(canvas.clone(), [0.5; 2], [0.2; 2], 0.0);
        assert_eq!(decal.source_svg, None);
        assert_eq!(decal.rerasterize(512), Ok(false));
        assert_eq!(decal.image, canvas);
    }

    #[test]
    fn decal_rerasterize_error_leaves_decal_intact() {
        let mut decal = DecalLayer::from_svg(RECT_SVG, 32, [0.5; 2], 0.5).unwrap();
        decal.source_svg = Some("<svg".to_string());
        let before = decal.image.clone();
        assert!(matches!(decal.rerasterize(64), Err(SvgError::Parse(_))));
        assert_eq!(decal.image, before);
    }

    #[test]
    fn decal_json_without_source_svg_still_deserializes() {
        let decal = DecalLayer::new(Canvas::new(2, 2, [9, 8, 7, 255]), [0.5; 2], [0.3; 2], 0.0);
        let mut value = serde_json::to_value(&decal).unwrap();
        value.as_object_mut().unwrap().remove("source_svg");
        let json = value.to_string();
        assert!(!json.contains("source_svg"));
        let back: DecalLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, decal);
        assert_eq!(back.source_svg, None);
    }

    #[test]
    fn decal_with_source_svg_round_trips_json() {
        let decal = DecalLayer::from_svg(RECT_SVG, 16, [0.5; 2], 0.5).unwrap();
        let json = serde_json::to_string(&decal).unwrap();
        let back: DecalLayer = serde_json::from_str(&json).unwrap();
        assert_eq!(back, decal);
    }
}

#[cfg(test)]
mod svg_compatibility_tests {
    use super::*;
    #[test]
    fn legacy_postcard_decal_keeps_its_four_field_layout() {
        #[derive(Serialize)]
        struct Old {
            image: Canvas,
            center_uv: [f32; 2],
            scale_uv: [f32; 2],
            rotation_rad: f32,
        }
        let old = Old {
            image: Canvas::new(2, 2, [1, 2, 3, 255]),
            center_uv: [0.5; 2],
            scale_uv: [0.2; 2],
            rotation_rad: 0.4,
        };
        let bytes = postcard::to_allocvec(&old).unwrap();
        let decal: DecalLayer = postcard::from_bytes(&bytes).unwrap();
        assert!(decal.source_svg.is_none());
        assert_eq!(decal.image, old.image);
        assert_eq!(postcard::to_allocvec(&decal).unwrap(), bytes);
    }
    #[test]
    fn canonical_json_retains_svg_source() {
        let mut decal = DecalLayer::new(Canvas::new(2, 2, [1, 2, 3, 255]), [0.5; 2], [0.2; 2], 0.0);
        decal.source_svg = Some("<svg/>".into());
        assert_eq!(
            serde_json::from_str::<DecalLayer>(&serde_json::to_string(&decal).unwrap()).unwrap(),
            decal
        );
    }
}
