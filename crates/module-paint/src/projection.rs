//! Projeção / Stencil e pintura de polígono preenchido em espaço de tela
//! (P3D-133, P3D-134, P3D-062; foundations 43/44).
//!
//! Núcleo de domínio, sem UI: uma imagem posicionada na tela (o *stencil*) é
//! "assada" na textura da superfície pintada, só nos texels que estão visíveis
//! da câmera. O mesmo caminho serve a Decal e a Projection (Surface
//! Manipulator compartilhado; vocabulário Apply / Keep Live / Bake — aqui só o
//! *Bake*; quem chama decide quando aplicar).
//!
//! Peças:
//!
//! - [`DepthBuffer`]: z-buffer por software da malha vista pela câmera, usado
//!   para oclusão (`visible`);
//! - [`StencilPlacement`]: retângulo rotacionável/espelhável em pixels de tela
//!   e o mapeamento tela → UV da imagem;
//! - [`sample_bilinear`] / [`stencil_alpha`]: amostragem da imagem;
//! - [`project_image_onto_mesh`]: bake da imagem projetada no canvas alvo;
//! - [`fill_screen_polygon`]: pinta cor sólida nos texels visíveis cuja posição
//!   projetada cai dentro de um polígono (paint de "filled path").
//!
//! Convenções:
//!
//! - Pixel de tela: origem no canto superior esquerdo, `y` para baixo, centro
//!   do pixel em `+0.5`. Mesma convenção da imagem do stencil: `uv = (0, 0)` é
//!   o canto superior esquerdo da imagem (V cresce para baixo — diferente da UV
//!   da malha, que é invertida na conversão para texel).
//! - Câmera: matriz `view_proj` com profundidade `0..1` (`glam::Mat4::*_rh`,
//!   a mesma de `Camera::view_proj`). O `bias` de oclusão é em unidades de
//!   profundidade NDC (`z/w`), não em unidades de mundo.
//! - Rotação do stencil positiva = sentido horário na tela (eixo `y` para
//!   baixo).

use std::collections::BTreeSet;

use glam::{Mat4, Vec2, Vec3, Vec4};
use petunia_mesh::Mesh;
use petunia_project::Canvas;
use petunia_project::paint_layers::TILE_SIZE;

/// Teto de pixels do z-buffer (`4096 × 4096`, ~64 MiB). Acima disso o buffer
/// vem vazio: nada fica visível e nada é alocado.
pub const MAX_DEPTH_PIXELS: u64 = 4096 * 4096;

/// Limite do tamanho do stencil em pixels (evita overflow em escala repetida).
const MAX_STENCIL_SIZE_PX: f32 = 1.0e6;

// ---------------------------------------------------------------------------
// Z-buffer
// ---------------------------------------------------------------------------

/// Z-buffer por software (profundidade NDC `0..1`, menor = mais perto).
///
/// Construído a partir dos triângulos de mundo da cena com a mesma
/// `view_proj` usada para projetar. Triângulos que cruzam o plano near são
/// recortados; triângulos com coordenada não finita são ignorados.
#[derive(Clone, Debug)]
pub struct DepthBuffer {
    view_proj: Mat4,
    inverse: Option<Mat4>,
    width: u32,
    height: u32,
    depth: Vec<f32>,
}

impl DepthBuffer {
    /// Buffer vazio: nada é visível. Resultado de dimensões inválidas.
    fn empty(view_proj: Mat4) -> Self {
        Self {
            view_proj,
            inverse: None,
            width: 0,
            height: 0,
            depth: Vec::new(),
        }
    }

    /// Rasteriza `triangles` (posições de mundo) num buffer `width × height`.
    ///
    /// Dimensão zero ou `width * height > MAX_DEPTH_PIXELS` devolve um buffer
    /// vazio ([`DepthBuffer::is_empty`]) sem alocar.
    pub fn build(
        view_proj: Mat4,
        width: u32,
        height: u32,
        triangles: impl IntoIterator<Item = [Vec3; 3]>,
    ) -> Self {
        if width == 0 || height == 0 || (width as u64) * (height as u64) > MAX_DEPTH_PIXELS {
            return Self::empty(view_proj);
        }
        let det = view_proj.determinant();
        let inverse = if det.is_finite() && det.abs() > 1e-30 {
            let inv = view_proj.inverse();
            inv.is_finite().then_some(inv)
        } else {
            None
        };
        let mut buf = Self {
            view_proj,
            inverse,
            width,
            height,
            depth: vec![f32::INFINITY; (width as usize) * (height as usize)],
        };
        for tri in triangles {
            buf.raster_world_triangle(tri);
        }
        buf
    }

    /// Atalho: z-buffer de todas as faces de uma malha (mesma triangulação do
    /// render e do picking).
    pub fn from_mesh(view_proj: Mat4, width: u32, height: u32, mesh: &Mesh) -> Self {
        Self::build(view_proj, width, height, mesh_world_triangles(mesh))
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// `true` quando o buffer não tem pixels (dimensões inválidas/enormes).
    pub fn is_empty(&self) -> bool {
        self.depth.is_empty()
    }

    /// Matriz com que o buffer foi construído.
    pub fn view_proj(&self) -> Mat4 {
        self.view_proj
    }

    /// Profundidade NDC guardada no pixel (`INFINITY` = nada desenhado).
    pub fn depth_at(&self, x: u32, y: u32) -> Option<f32> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.depth
            .get((y as usize) * (self.width as usize) + x as usize)
            .copied()
    }

    /// Posição em pixels de `world` quando o ponto está dentro do viewport,
    /// entre os planos near/far e não ocluído (`z <= z_guardado + bias`).
    ///
    /// `bias` não finito ou negativo vale `0`. Além do `bias`, soma-se
    /// automaticamente a inclinação local do z-buffer (superfícies oblíquas).
    pub fn visible(&self, world: Vec3, bias: f32) -> Option<[f32; 2]> {
        if self.is_empty() || !world.is_finite() {
            return None;
        }
        let clip = self.view_proj * world.extend(1.0);
        if !clip.is_finite() || clip.w <= 1e-9 {
            return None;
        }
        let ndc = clip.truncate() / clip.w;
        if !(0.0..=1.0).contains(&ndc.z) {
            return None;
        }
        let px = [
            (ndc.x * 0.5 + 0.5) * self.width as f32,
            (0.5 - ndc.y * 0.5) * self.height as f32,
        ];
        if !(px[0] >= 0.0
            && px[1] >= 0.0
            && px[0] < self.width as f32
            && px[1] < self.height as f32)
        {
            return None;
        }
        let bias = if bias.is_finite() { bias.max(0.0) } else { 0.0 };
        let (x, y) = (px[0] as usize, px[1] as usize);
        let stored = *self.depth.get(y * (self.width as usize) + x)?;
        if ndc.z <= stored + bias {
            return Some(px);
        }
        // Bias proporcional à inclinação: em superfícies oblíquas a
        // profundidade varia dentro do pixel e o ponto exato está até ~1 px do
        // centro onde o z foi guardado.
        let slope = self.local_slope(x, y, stored);
        (ndc.z <= stored + bias + slope).then_some(px)
    }

    /// Variação de profundidade por pixel em torno de `(x, y)`. Em cada eixo
    /// vale o menor degrau entre os vizinhos desenhados, então uma silhueta
    /// (salto de um lado só) não vira "inclinação" e não deixa tinta vazar para
    /// trás do oclusor.
    fn local_slope(&self, x: usize, y: usize, center: f32) -> f32 {
        if !center.is_finite() {
            return 0.0;
        }
        let w = self.width as usize;
        let step = |nx: Option<usize>, ny: Option<usize>| -> Option<f32> {
            let z = *self.depth.get(ny? * w + nx?)?;
            z.is_finite().then(|| (z - center).abs())
        };
        let axis = |a: Option<f32>, b: Option<f32>| match (a, b) {
            (Some(a), Some(b)) => a.min(b),
            _ => 0.0,
        };
        let sx = axis(step(x.checked_sub(1), Some(y)), step(Some(x + 1), Some(y)));
        let sy = axis(step(Some(x), y.checked_sub(1)), step(Some(x), Some(y + 1)));
        sx.max(sy)
    }

    /// Direção (unitária, da câmera para a cena) do raio que passa pelo pixel
    /// `px`. Serve ao teste de face voltada para trás. `None` se a matriz não é
    /// invertível ou o buffer está vazio.
    pub fn view_direction_at(&self, px: [f32; 2]) -> Option<Vec3> {
        let inv = self.inverse?;
        if self.is_empty() || !(px[0].is_finite() && px[1].is_finite()) {
            return None;
        }
        let nx = px[0] / self.width as f32 * 2.0 - 1.0;
        let ny = 1.0 - px[1] / self.height as f32 * 2.0;
        let unproject = |z: f32| {
            let p = inv * Vec4::new(nx, ny, z, 1.0);
            (p.w.abs() > 1e-20).then(|| p.truncate() / p.w)
        };
        let dir = (unproject(1.0)? - unproject(0.0)?).normalize_or_zero();
        (dir != Vec3::ZERO && dir.is_finite()).then_some(dir)
    }

    fn raster_world_triangle(&mut self, tri: [Vec3; 3]) {
        if !tri.iter().all(|v| v.is_finite()) {
            return;
        }
        let clip = tri.map(|v| self.view_proj * v.extend(1.0));
        if !clip.iter().all(|c| c.is_finite()) {
            return;
        }
        // Recorte contra z >= 0 (near, profundidade 0..1). Um triângulo vira
        // no máximo um quadrilátero.
        let mut poly = [Vec4::ZERO; 4];
        let mut n = 0usize;
        for i in 0..3 {
            let (a, b) = (clip[i], clip[(i + 1) % 3]);
            let (a_in, b_in) = (a.z >= 0.0, b.z >= 0.0);
            if a_in && n < 4 {
                poly[n] = a;
                n += 1;
            }
            if a_in != b_in && n < 4 {
                let t = a.z / (a.z - b.z);
                poly[n] = a + (b - a) * t;
                n += 1;
            }
        }
        if n < 3 || poly[..n].iter().any(|v| v.w <= 1e-9) {
            return;
        }
        let (w, h) = (self.width as f32, self.height as f32);
        let screen: Vec<Vec3> = poly[..n]
            .iter()
            .map(|v| {
                let ndc = v.truncate() / v.w;
                Vec3::new((ndc.x * 0.5 + 0.5) * w, (0.5 - ndc.y * 0.5) * h, ndc.z)
            })
            .collect();
        for i in 1..n - 1 {
            self.raster_screen_triangle(screen[0], screen[i], screen[i + 1]);
        }
    }

    /// Cobertura por centro de pixel; z interpolado linearmente em tela
    /// (`z/w` é afim no espaço da tela, logo exato sob perspectiva).
    fn raster_screen_triangle(&mut self, a: Vec3, b: Vec3, c: Vec3) {
        let area = edge(a, b, c.x, c.y);
        if !area.is_finite() || area.abs() < 1e-12 {
            return;
        }
        let (w, h) = (self.width as f32, self.height as f32);
        let lo = |m: f32, lim: f32| (m - 0.5).ceil().clamp(0.0, lim) as usize;
        let hi = |m: f32, lim: f32| (m - 0.5).floor().clamp(-1.0, lim - 1.0);
        let x0 = lo(a.x.min(b.x).min(c.x), w);
        let y0 = lo(a.y.min(b.y).min(c.y), h);
        let x1 = hi(a.x.max(b.x).max(c.x), w);
        let y1 = hi(a.y.max(b.y).max(c.y), h);
        if x1 < 0.0 || y1 < 0.0 {
            return;
        }
        let (x1, y1) = (x1 as usize, y1 as usize);
        let width = self.width as usize;
        const EPS: f32 = -1e-5;
        for y in y0..=y1 {
            for x in x0..=x1 {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let l0 = edge(b, c, px, py) / area;
                let l1 = edge(c, a, px, py) / area;
                let l2 = edge(a, b, px, py) / area;
                if l0 < EPS || l1 < EPS || l2 < EPS {
                    continue;
                }
                let z = l0 * a.z + l1 * b.z + l2 * c.z;
                if !z.is_finite() {
                    continue;
                }
                if let Some(slot) = self.depth.get_mut(y * width + x)
                    && z < *slot
                {
                    *slot = z;
                }
            }
        }
    }
}

/// Função de aresta 2D: positiva de um lado de `a → b`.
#[inline]
fn edge(a: Vec3, b: Vec3, px: f32, py: f32) -> f32 {
    (b.x - a.x) * (py - a.y) - (b.y - a.y) * (px - a.x)
}

/// Triângulos de mundo de todas as faces de `mesh` (triangulação do render).
pub fn mesh_world_triangles(mesh: &Mesh) -> impl Iterator<Item = [Vec3; 3]> + '_ {
    (0..mesh.faces.len()).flat_map(move |fi| {
        let face = &mesh.faces[fi];
        mesh.face_triangle_corners(fi)
            .into_iter()
            .filter_map(move |[a, b, c]| {
                let pos = |k: usize| {
                    face.verts
                        .get(k)
                        .and_then(|&v| mesh.verts.get(v as usize))
                        .map(|v| v.vec())
                };
                Some([pos(a)?, pos(b)?, pos(c)?])
            })
    })
}

// ---------------------------------------------------------------------------
// Stencil
// ---------------------------------------------------------------------------

/// Retângulo do stencil na tela: centro, tamanho, rotação e espelho.
///
/// O espelho (`mirror_x`) inverte a imagem horizontalmente no espaço local do
/// retângulo, antes da rotação.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StencilPlacement {
    pub center_px: [f32; 2],
    pub size_px: [f32; 2],
    /// Radianos; positivo = horário na tela.
    pub rotation_rad: f32,
    pub mirror_x: bool,
}

impl Default for StencilPlacement {
    fn default() -> Self {
        Self {
            center_px: [0.0, 0.0],
            size_px: [100.0, 100.0],
            rotation_rad: 0.0,
            mirror_x: false,
        }
    }
}

impl StencilPlacement {
    fn is_valid(&self) -> bool {
        self.center_px.iter().all(|v| v.is_finite())
            && self.size_px.iter().all(|v| v.is_finite() && *v > 0.0)
            && self.rotation_rad.is_finite()
    }

    /// UV da imagem (`0..1`, origem no canto superior esquerdo) do pixel `px`,
    /// ou `None` se cai fora do retângulo rotacionado (ou o placement é
    /// inválido: tamanho `<= 0`, valor não finito).
    pub fn uv_at(&self, px: [f32; 2]) -> Option<[f32; 2]> {
        if !self.is_valid() || !(px[0].is_finite() && px[1].is_finite()) {
            return None;
        }
        let d = Vec2::new(px[0] - self.center_px[0], px[1] - self.center_px[1]);
        let (sin, cos) = self.rotation_rad.sin_cos();
        // rotação inversa (tela → local)
        let local = Vec2::new(d.x * cos + d.y * sin, -d.x * sin + d.y * cos);
        let mut u = local.x / self.size_px[0] + 0.5;
        let v = local.y / self.size_px[1] + 0.5;
        if self.mirror_x {
            u = 1.0 - u;
        }
        ((0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v)).then_some([u, v])
    }

    /// Cantos do retângulo na tela, na ordem superior-esquerdo, superior-direito,
    /// inferior-direito, inferior-esquerdo (do retângulo, antes da rotação; o
    /// espelho não muda o contorno). Placement inválido: todos no centro.
    pub fn outline_px(&self) -> [[f32; 2]; 4] {
        if !self.is_valid() {
            return [self.center_px; 4];
        }
        let (hw, hh) = (self.size_px[0] * 0.5, self.size_px[1] * 0.5);
        let (sin, cos) = self.rotation_rad.sin_cos();
        [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)].map(|(x, y)| {
            [
                self.center_px[0] + x * cos - y * sin,
                self.center_px[1] + x * sin + y * cos,
            ]
        })
    }

    /// Move (arrastar). Delta não finito: sem efeito.
    pub fn translated(&self, delta_px: [f32; 2]) -> Self {
        if !(delta_px[0].is_finite() && delta_px[1].is_finite()) {
            return *self;
        }
        Self {
            center_px: [
                self.center_px[0] + delta_px[0],
                self.center_px[1] + delta_px[1],
            ],
            ..*self
        }
    }

    /// Escala uniforme em torno do centro (Shift + arrastar). Fator `<= 0` ou
    /// não finito: sem efeito. Tamanho limitado a `1e6` px.
    pub fn scaled_about_center(&self, factor: f32) -> Self {
        if !(factor.is_finite() && factor > 0.0) {
            return *self;
        }
        Self {
            size_px: self
                .size_px
                .map(|s| (s * factor).clamp(1e-3, MAX_STENCIL_SIZE_PX)),
            ..*self
        }
    }

    /// Gira em torno do centro (Ctrl + arrastar). O ângulo fica em `(-π, π]`.
    /// Delta não finito: sem efeito.
    pub fn rotated(&self, delta_rad: f32) -> Self {
        if !delta_rad.is_finite() || !self.rotation_rad.is_finite() {
            return *self;
        }
        let tau = std::f32::consts::TAU;
        let mut r = (self.rotation_rad + delta_rad).rem_euclid(tau);
        if r > std::f32::consts::PI {
            r -= tau;
        }
        Self {
            rotation_rad: r,
            ..*self
        }
    }
}

// ---------------------------------------------------------------------------
// Amostragem
// ---------------------------------------------------------------------------

/// `true` se o canvas tem `w`, `h` > 0 e bytes suficientes.
fn canvas_is_sane(c: &Canvas) -> bool {
    c.w > 0 && c.h > 0 && (c.w as u128) * (c.h as u128) * 4 <= c.pixels.len() as u128
}

/// Texel em alfa premultiplicado, `0..1`. Chamar só com coordenadas dentro.
fn texel_premultiplied(c: &Canvas, x: usize, y: usize) -> [f32; 4] {
    let i = (y * c.w as usize + x) * 4;
    let px = c.pixels.get(i..i + 4).unwrap_or(&[0, 0, 0, 0]);
    let a = px[3] as f32 / 255.0;
    [
        px[0] as f32 / 255.0 * a,
        px[1] as f32 / 255.0 * a,
        px[2] as f32 / 255.0 * a,
        a,
    ]
}

/// Amostra bilinear de `image` em `uv` (`0..1`, origem no canto superior
/// esquerdo, centro do texel em `+0.5`), clamp-to-edge.
///
/// Interpola em alfa premultiplicado (sem halo escuro nas bordas
/// transparentes) e devolve RGBA **reto** em `0..1`. Canvas inválido ou `uv`
/// não finito: `[0; 4]`.
pub fn sample_bilinear(image: &Canvas, uv: [f32; 2]) -> [f32; 4] {
    if !canvas_is_sane(image) || !(uv[0].is_finite() && uv[1].is_finite()) {
        return [0.0; 4];
    }
    let (w, h) = (image.w as usize, image.h as usize);
    let fx = (uv[0] * w as f32 - 0.5).clamp(0.0, (w - 1) as f32);
    let fy = (uv[1] * h as f32 - 0.5).clamp(0.0, (h - 1) as f32);
    let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
    let (p00, p10) = (
        texel_premultiplied(image, x0, y0),
        texel_premultiplied(image, x1, y0),
    );
    let (p01, p11) = (
        texel_premultiplied(image, x0, y1),
        texel_premultiplied(image, x1, y1),
    );
    let mut out = [0.0f32; 4];
    for k in 0..4 {
        let top = p00[k] + (p10[k] - p00[k]) * tx;
        let bottom = p01[k] + (p11[k] - p01[k]) * tx;
        out[k] = top + (bottom - top) * ty;
    }
    let a = out[3];
    if a <= 1e-6 {
        return [0.0; 4];
    }
    [
        (out[0] / a).clamp(0.0, 1.0),
        (out[1] / a).clamp(0.0, 1.0),
        (out[2] / a).clamp(0.0, 1.0),
        a.clamp(0.0, 1.0),
    ]
}

/// Canal do stencil usado como máscara de opacidade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StencilChannel {
    /// Alfa da imagem.
    Alpha,
    /// Luminância (Rec. 709) multiplicada pelo alfa.
    Luma,
}

/// Valor `0..1` do stencil no pixel de tela `px`; `0` fora do retângulo.
pub fn stencil_alpha(
    image: &Canvas,
    placement: &StencilPlacement,
    px: [f32; 2],
    channel: StencilChannel,
) -> f32 {
    let Some(uv) = placement.uv_at(px) else {
        return 0.0;
    };
    let [r, g, b, a] = sample_bilinear(image, uv);
    match channel {
        StencilChannel::Alpha => a,
        StencilChannel::Luma => (0.2126 * r + 0.7152 * g + 0.0722 * b) * a,
    }
}

// ---------------------------------------------------------------------------
// Bake
// ---------------------------------------------------------------------------

/// Parâmetros da projeção de uma imagem na superfície.
#[derive(Clone, Copy, Debug)]
pub struct ProjectionParams {
    /// Mesma matriz com que o [`DepthBuffer`] foi construído.
    pub view_proj: Mat4,
    /// Mesmo tamanho do [`DepthBuffer`].
    pub viewport: [u32; 2],
    pub placement: StencilPlacement,
    /// `0..1`; fora disso é limitado.
    pub opacity: f32,
    /// Ignora faces voltadas para longe da câmera.
    pub back_face_cull: bool,
    /// Tolerância de oclusão em profundidade NDC.
    pub depth_bias: f32,
}

/// Composição "over" em alfa reto: `src` (com `opacity` aplicada ao alfa)
/// sobre `dst`. Em destino opaco equivale à mistura normal; em destino
/// transparente a cor não escurece.
fn over(dst: [u8; 4], src: [u8; 4], opacity: f32) -> [u8; 4] {
    let sa = src[3] as f32 / 255.0 * opacity.clamp(0.0, 1.0);
    if sa <= 0.0 {
        return dst;
    }
    let da = dst[3] as f32 / 255.0;
    let oa = sa + da * (1.0 - sa);
    if oa <= 0.0 {
        return dst;
    }
    let mix = |s: u8, d: u8| {
        ((s as f32 * sa + d as f32 * da * (1.0 - sa)) / oa)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    [
        mix(src[0], dst[0]),
        mix(src[1], dst[1]),
        mix(src[2], dst[2]),
        (oa * 255.0).round().clamp(0.0, 255.0) as u8,
    ]
}

/// Núcleo comum do bake: para cada texel visível de cada face permitida pede a
/// cor a `shade(px_de_tela)` e compõe no canvas.
///
/// Cada texel é escrito no máximo uma vez (diagonal de quad e dilatação não
/// compõem duas vezes com opacidade < 1), e a dilatação de 1 px nunca invade
/// texels de faces vetadas ou de outras faces.
#[allow(clippy::too_many_arguments)]
fn bake_texels(
    target: &mut Canvas,
    mesh: &Mesh,
    face_allowed: &dyn Fn(usize) -> bool,
    depth: &DepthBuffer,
    depth_bias: f32,
    back_face_cull: bool,
    opacity: f32,
    mut shade: impl FnMut([f32; 2]) -> Option<[u8; 4]>,
) -> Vec<u32> {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if opacity <= 0.0
        || depth.is_empty()
        || !canvas_is_sane(target)
        || (back_face_cull && depth.inverse.is_none())
    {
        return Vec::new();
    }
    let (w, h) = (target.w, target.h);
    let tiles_x = w.div_ceil(TILE_SIZE).max(1);
    let allowed: Vec<bool> = (0..mesh.faces.len()).map(face_allowed).collect();
    // 1 = texel já decidido (pintado, descartado ou reservado a outra face)
    let mut owned = vec![0u8; (w as usize) * (h as usize)];
    let mut dirty = BTreeSet::new();

    // Passo 0: texels de faces vetadas ficam reservados (a dilatação não os toca).
    for (fi, ok) in allowed.iter().enumerate() {
        if !*ok {
            mesh.rasterize_face_texels(fi, w, h, 0.0, |x, y, _, _| {
                if let Some(slot) = owned.get_mut((y * w + x) as usize) {
                    *slot = 1;
                }
            });
        }
    }
    // Passo 1: interior das UVs; passo 2: dilatação de 1 px nos texels livres.
    for dilate in [0.0f32, 1.0] {
        for (fi, ok) in allowed.iter().enumerate() {
            if !*ok {
                continue;
            }
            mesh.rasterize_face_texels(fi, w, h, dilate, |x, y, pos, normal| {
                let idx = (y * w + x) as usize;
                let Some(slot) = owned.get_mut(idx) else {
                    return;
                };
                if *slot != 0 {
                    return;
                }
                *slot = 1;
                let Some(px) = depth.visible(pos, depth_bias) else {
                    return;
                };
                if back_face_cull {
                    match depth.view_direction_at(px) {
                        Some(dir) if normal.dot(dir) < 0.0 => {}
                        _ => return,
                    }
                }
                let Some(src) = shade(px) else {
                    return;
                };
                let i = idx * 4;
                let Some(dst) = target.pixels.get(i..i + 4) else {
                    return;
                };
                let dst = [dst[0], dst[1], dst[2], dst[3]];
                let out = over(dst, src, opacity);
                if out != dst {
                    target.pixels[i..i + 4].copy_from_slice(&out);
                    dirty.insert((y / TILE_SIZE) * tiles_x + x / TILE_SIZE);
                }
            });
        }
    }
    dirty.into_iter().collect()
}

/// Projeção da tela deve concordar com o z-buffer (mesma matriz e viewport).
fn depth_matches(depth: &DepthBuffer, view_proj: Mat4, viewport: [u32; 2]) -> bool {
    depth.view_proj == view_proj && [depth.width, depth.height] == viewport
}

/// Assa `image`, projetada pelo `placement` na tela, no canvas `target`.
///
/// - mistura "over" com `opacity` aplicada ao alfa da imagem;
/// - texels fora do retângulo do stencil ficam intactos;
/// - só texels visíveis (oclusão pelo `depth`, com `depth_bias`);
/// - `back_face_cull`: faces voltadas para longe da câmera são puladas;
/// - `face_allowed(i) == false`: a face não é tocada (nem pela dilatação);
/// - dilatação de 1 px nas bordas das UVs para evitar costuras.
///
/// Devolve os ids dos tiles alterados (ordenados, únicos; convenção de
/// [`crate::DirtyTiles`]: `ty * tiles_x + tx`, `tiles_x = ceil(w / TILE_SIZE)`).
/// `depth` precisa ter sido construído com `params.view_proj` e
/// `params.viewport`; se não bater, buffer vazio, canvas inválido ou opacidade
/// `0`, nada é pintado e a lista vem vazia.
pub fn project_image_onto_mesh(
    target: &mut Canvas,
    mesh: &Mesh,
    face_allowed: &dyn Fn(usize) -> bool,
    depth: &DepthBuffer,
    image: &Canvas,
    params: &ProjectionParams,
) -> Vec<u32> {
    if !canvas_is_sane(image)
        || !params.placement.is_valid()
        || !depth_matches(depth, params.view_proj, params.viewport)
    {
        return Vec::new();
    }
    bake_texels(
        target,
        mesh,
        face_allowed,
        depth,
        params.depth_bias,
        params.back_face_cull,
        params.opacity,
        |px| {
            let uv = params.placement.uv_at(px)?;
            let [r, g, b, a] = sample_bilinear(image, uv);
            if a <= 0.0 {
                return None;
            }
            let q = |v: f32| (v * 255.0).round().clamp(0.0, 255.0) as u8;
            Some([q(r), q(g), q(b), q(a)])
        },
    )
}

/// Teste par-ímpar de ponto em polígono (pontos já finitos, `len >= 3`).
fn point_in_polygon_even_odd(p: [f32; 2], poly: &[[f32; 2]]) -> bool {
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (pi, pj) = (poly[i], poly[j]);
        if (pi[1] > p[1]) != (pj[1] > p[1])
            && p[0] < (pj[0] - pi[0]) * (p[1] - pi[1]) / (pj[1] - pi[1]) + pi[0]
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Pinta `color` (RGBA reto; o alfa da cor multiplica `opacity`) em todo texel
/// visível cuja posição projetada em tela cai dentro do polígono `polygon_px`
/// (regra par-ímpar). É o paint de "filled path": só a parte visível da
/// superfície, sem atravessar oclusores.
///
/// Mesmas garantias de [`project_image_onto_mesh`] (escrita única por texel,
/// dilatação de 1 px, faces vetadas intactas, tiles devolvidos), exceto que
/// não há descarte de faces voltadas para trás: a oclusão pelo z-buffer decide.
/// Polígono com menos de 3 pontos ou ponto não finito: nada é pintado.
#[allow(clippy::too_many_arguments)]
pub fn fill_screen_polygon(
    target: &mut Canvas,
    mesh: &Mesh,
    face_allowed: &dyn Fn(usize) -> bool,
    depth: &DepthBuffer,
    polygon_px: &[[f32; 2]],
    color: [u8; 4],
    opacity: f32,
    view_proj: Mat4,
    viewport: [u32; 2],
    depth_bias: f32,
) -> Vec<u32> {
    if polygon_px.len() < 3
        || polygon_px
            .iter()
            .any(|p| !(p[0].is_finite() && p[1].is_finite()))
        || !depth_matches(depth, view_proj, viewport)
    {
        return Vec::new();
    }
    bake_texels(
        target,
        mesh,
        face_allowed,
        depth,
        depth_bias,
        false,
        opacity,
        |px| point_in_polygon_even_odd(px, polygon_px).then_some(color),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: [u8; 4] = [255, 0, 0, 255];
    const GREEN: [u8; 4] = [0, 255, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 255, 255];
    const YELLOW: [u8; 4] = [255, 255, 0, 255];
    const CLEAR: [u8; 4] = [0, 0, 0, 0];
    const BIAS: f32 = 1e-4;

    /// Câmera em (0, 0, 5) olhando para a origem; viewport 64 × 64.
    fn camera() -> (Mat4, [u32; 2]) {
        let view = Mat4::look_at_rh(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, Vec3::Y);
        let proj = Mat4::perspective_rh(60f32.to_radians(), 1.0, 0.1, 100.0);
        (proj * view, [64, 64])
    }

    /// Metade do lado, em pixels, de um quadrado de lado 2 a distância 5.
    fn half_px_at_5() -> f32 {
        1.0 / (5.0 * 30f32.to_radians().tan()) * 32.0
    }

    /// Placement que cobre exatamente o quad `z = 0` de lado 2.
    fn fit_placement() -> StencilPlacement {
        let s = half_px_at_5() * 2.0;
        StencilPlacement {
            center_px: [32.0, 32.0],
            size_px: [s, s],
            rotation_rad: 0.0,
            mirror_x: false,
        }
    }

    /// Quad paralelo ao plano XY em `z`, lado `2 * half`, UV com `u` em
    /// `u0..u1`; `flipped` inverte o winding (normal para `-z`).
    fn push_quad(m: &mut Mesh, z: f32, half: f32, u0: f32, u1: f32, flipped: bool) {
        let base = m.verts.len() as u32;
        for (x, y) in [(-half, -half), (half, -half), (half, half), (-half, half)] {
            m.verts.push(petunia_mesh::Vertex::new(x, y, z));
        }
        let mut verts = vec![base, base + 1, base + 2, base + 3];
        let mut uv = vec![[u0, 0.0], [u1, 0.0], [u1, 1.0], [u0, 1.0]];
        if flipped {
            verts.reverse();
            uv.reverse();
        }
        m.faces.push(petunia_mesh::Face::with_uv(verts, uv));
    }

    fn single_quad(flipped: bool) -> Mesh {
        let mut m = Mesh::default();
        push_quad(&mut m, 0.0, 1.0, 0.0, 1.0, flipped);
        m
    }

    /// Face 0 = quad de trás (UV esquerda), face 1 = quad menor na frente
    /// (UV direita), que esconde o centro do de trás.
    fn occluded_pair() -> Mesh {
        let mut m = Mesh::default();
        push_quad(&mut m, 0.0, 1.0, 0.0, 0.5, false);
        push_quad(&mut m, 1.5, 0.4, 0.5, 1.0, false);
        m
    }

    fn corner_image() -> Canvas {
        let mut c = Canvas::new(2, 2, CLEAR);
        c.set(0, 0, RED);
        c.set(1, 0, GREEN);
        c.set(0, 1, BLUE);
        c.set(1, 1, YELLOW);
        c
    }

    fn solid(w: u32, h: u32, c: [u8; 4]) -> Canvas {
        Canvas::new(w, h, c)
    }

    fn params(placement: StencilPlacement) -> ProjectionParams {
        let (view_proj, viewport) = camera();
        ProjectionParams {
            view_proj,
            viewport,
            placement,
            opacity: 1.0,
            back_face_cull: true,
            depth_bias: BIAS,
        }
    }

    fn depth_for(mesh: &Mesh) -> DepthBuffer {
        let (vp, [w, h]) = camera();
        DepthBuffer::from_mesh(vp, w, h, mesh)
    }

    fn always(_: usize) -> bool {
        true
    }

    // ---- DepthBuffer -------------------------------------------------------

    #[test]
    fn depth_visible_in_front_and_hidden_behind() {
        let mut m = Mesh::default();
        push_quad(&mut m, 1.0, 1.0, 0.0, 1.0, false);
        let d = depth_for(&m);
        // ponto na frente do quad: visível; atrás dele (mesmo pixel): ocluído
        assert!(d.visible(Vec3::new(0.0, 0.0, 2.0), BIAS).is_some());
        assert!(d.visible(Vec3::new(0.0, 0.0, 1.0), BIAS).is_some());
        assert!(d.visible(Vec3::new(0.0, 0.0, 0.0), BIAS).is_none());
        // fora do quad: nada desenhado, logo visível
        assert!(d.visible(Vec3::new(1.8, 0.0, 0.0), BIAS).is_some());
    }

    #[test]
    fn depth_visible_returns_pixel_position() {
        let d = depth_for(&Mesh::default());
        let px = d.visible(Vec3::ZERO, 0.0).unwrap();
        assert!((px[0] - 32.0).abs() < 1e-3 && (px[1] - 32.0).abs() < 1e-3);
        // +x do mundo vai para a direita, +y para cima (y de pixel diminui)
        let px = d.visible(Vec3::new(1.0, 1.0, 0.0), 0.0).unwrap();
        assert!(px[0] > 32.0 && px[1] < 32.0);
    }

    #[test]
    fn depth_rejects_points_outside_viewport_and_behind_camera() {
        let d = depth_for(&Mesh::default());
        assert!(d.visible(Vec3::new(50.0, 0.0, 0.0), 0.0).is_none());
        assert!(d.visible(Vec3::new(0.0, 0.0, 6.0), 0.0).is_none()); // atrás da câmera
        assert!(d.visible(Vec3::new(0.0, 0.0, 5.0), 0.0).is_none()); // no olho
        assert!(d.visible(Vec3::new(0.0, 0.0, -500.0), 0.0).is_none()); // além do far
        assert!(d.visible(Vec3::NAN, 0.0).is_none());
        assert!(d.visible(Vec3::splat(f32::INFINITY), 0.0).is_none());
    }

    #[test]
    fn depth_bias_is_clamped_and_nan_safe() {
        let mut m = Mesh::default();
        push_quad(&mut m, 1.0, 1.0, 0.0, 1.0, false);
        let d = depth_for(&m);
        let behind = Vec3::new(0.0, 0.0, 0.0);
        assert!(d.visible(behind, f32::NAN).is_none());
        assert!(d.visible(behind, -1.0).is_none());
        // bias enorme perdoa qualquer oclusão
        assert!(d.visible(behind, 10.0).is_some());
    }

    #[test]
    fn depth_handles_triangles_crossing_the_near_plane() {
        // chão y = -1 do z = +10 (atrás da câmera) ao z = -20
        let tris = [
            [
                Vec3::new(-5.0, -1.0, 10.0),
                Vec3::new(5.0, -1.0, 10.0),
                Vec3::new(5.0, -1.0, -20.0),
            ],
            [
                Vec3::new(-5.0, -1.0, 10.0),
                Vec3::new(5.0, -1.0, -20.0),
                Vec3::new(-5.0, -1.0, -20.0),
            ],
        ];
        let (vp, _) = camera();
        let d = DepthBuffer::build(vp, 64, 64, tris);
        let px = d.visible(Vec3::new(0.0, -1.0, 0.0), BIAS).unwrap();
        assert!(px[1] > 32.0);
        // o chão cobriu a metade de baixo com profundidades finitas
        let covered = (0..64)
            .filter(|&y| d.depth_at(32, y).is_some_and(|z| z.is_finite()))
            .count();
        assert!(covered > 5, "{covered}");
        // algo abaixo do chão, no mesmo pixel, fica ocluído
        assert!(d.visible(Vec3::new(0.0, -1.2, 0.0), BIAS).is_none());
    }

    #[test]
    fn depth_skips_hostile_triangles() {
        let (vp, _) = camera();
        let tris = [
            [Vec3::NAN, Vec3::X, Vec3::Y],
            [Vec3::splat(f32::INFINITY), Vec3::X, Vec3::Y],
            [Vec3::ZERO, Vec3::ZERO, Vec3::ZERO],
            [
                Vec3::new(0.0, 0.0, 9.0),
                Vec3::new(1.0, 0.0, 9.0),
                Vec3::new(0.0, 1.0, 9.0),
            ], // atrás
            [
                Vec3::splat(1e30),
                Vec3::splat(-1e30),
                Vec3::new(1e30, -1e30, 0.0),
            ],
        ];
        let d = DepthBuffer::build(vp, 16, 16, tris);
        assert!(!d.is_empty());
        for y in 0..16 {
            for x in 0..16 {
                let z = d.depth_at(x, y).unwrap();
                assert!(!z.is_nan());
            }
        }
        // matriz inválida também é inofensiva
        let d = DepthBuffer::build(Mat4::ZERO, 8, 8, [[Vec3::X, Vec3::Y, Vec3::Z]]);
        assert!(d.visible(Vec3::ZERO, 0.0).is_none());
        assert!(d.view_direction_at([1.0, 1.0]).is_none());
        let d = DepthBuffer::build(Mat4::from_cols_array(&[f32::NAN; 16]), 8, 8, []);
        assert!(d.visible(Vec3::ZERO, 0.0).is_none());
    }

    #[test]
    fn depth_rejects_huge_or_zero_viewports_without_allocating() {
        let (vp, _) = camera();
        for (w, h) in [
            (0, 10),
            (10, 0),
            (100_000, 100_000),
            (u32::MAX, u32::MAX),
            (4097, 4097),
        ] {
            let d = DepthBuffer::build(vp, w, h, []);
            assert!(d.is_empty(), "{w}x{h}");
            assert_eq!((d.width(), d.height()), (0, 0));
            assert!(d.visible(Vec3::ZERO, 1.0).is_none());
            assert!(d.depth_at(0, 0).is_none());
        }
        // o limite exato ainda é aceito
        assert!(!DepthBuffer::build(vp, 4096, 4096, []).is_empty());
    }

    #[test]
    fn depth_view_direction_points_into_the_scene() {
        let d = depth_for(&Mesh::default());
        let dir = d.view_direction_at([32.0, 32.0]).unwrap();
        assert!(dir.distance(-Vec3::Z) < 1e-3, "{dir:?}");
        let off = d.view_direction_at([0.0, 0.0]).unwrap();
        assert!(off.z < 0.0 && off.x < 0.0 && off.y > 0.0, "{off:?}");
        assert!(d.view_direction_at([f32::NAN, 0.0]).is_none());
    }

    // ---- StencilPlacement --------------------------------------------------

    #[test]
    fn placement_maps_center_and_corners() {
        let p = StencilPlacement {
            center_px: [100.0, 50.0],
            size_px: [40.0, 20.0],
            rotation_rad: 0.0,
            mirror_x: false,
        };
        assert_eq!(p.uv_at([100.0, 50.0]), Some([0.5, 0.5]));
        let tl = p.uv_at([80.5, 40.5]).unwrap();
        assert!(tl[0] < 0.02 && tl[1] < 0.05);
        let br = p.uv_at([119.5, 59.5]).unwrap();
        assert!(br[0] > 0.98 && br[1] > 0.95);
        assert!(p.uv_at([79.0, 50.0]).is_none());
        assert!(p.uv_at([100.0, 61.0]).is_none());
        assert!(p.uv_at([f32::NAN, 50.0]).is_none());
    }

    #[test]
    fn placement_rotation_moves_the_image_corners_clockwise() {
        let p = StencilPlacement {
            center_px: [50.0, 50.0],
            size_px: [20.0, 20.0],
            rotation_rad: std::f32::consts::FRAC_PI_2,
            mirror_x: false,
        };
        // o canto superior esquerdo da imagem agora está no canto superior direito
        let uv = p.uv_at([59.0, 41.0]).unwrap();
        assert!(uv[0] < 0.1 && uv[1] < 0.1, "{uv:?}");
        let uv = p.uv_at([59.0, 59.0]).unwrap();
        assert!(uv[0] > 0.9 && uv[1] < 0.1, "{uv:?}");
        let outline = p.outline_px();
        assert!((outline[0][0] - 60.0).abs() < 1e-3 && (outline[0][1] - 40.0).abs() < 1e-3);
        // rotação de 45°: o retângulo vira um losango (quina do canto cai em x = c)
        let d = StencilPlacement {
            rotation_rad: std::f32::consts::FRAC_PI_4,
            ..p
        };
        assert!(d.uv_at([50.0 + 13.0, 50.0]).is_some());
        assert!(d.uv_at([50.0 + 14.5, 50.0 + 14.5]).is_none());
    }

    #[test]
    fn placement_mirror_flips_horizontally_only() {
        let p = StencilPlacement {
            center_px: [50.0, 50.0],
            size_px: [20.0, 20.0],
            rotation_rad: 0.0,
            mirror_x: true,
        };
        let tl = p.uv_at([41.0, 41.0]).unwrap();
        assert!(tl[0] > 0.9 && tl[1] < 0.1, "{tl:?}");
        // o contorno não muda com o espelho
        let plain = StencilPlacement {
            mirror_x: false,
            ..p
        };
        assert_eq!(p.outline_px(), plain.outline_px());
    }

    #[test]
    fn placement_outline_corners_are_the_uv_corners() {
        for (rot, mirror) in [(0.0, false), (0.7, false), (-2.0, true), (3.0, true)] {
            let p = StencilPlacement {
                center_px: [30.0, 40.0],
                size_px: [50.0, 30.0],
                rotation_rad: rot,
                mirror_x: mirror,
            };
            let inward = |c: [f32; 2]| {
                // 1% do caminho do canto até o centro
                [c[0] + (30.0 - c[0]) * 0.01, c[1] + (40.0 - c[1]) * 0.01]
            };
            let [tl, tr, br, bl] = p.outline_px().map(|c| p.uv_at(inward(c)).unwrap());
            let (left, right) = if mirror { (1.0, 0.0) } else { (0.0, 1.0) };
            assert!((tl[0] - left).abs() < 0.03 && tl[1] < 0.03, "{rot} {tl:?}");
            assert!((tr[0] - right).abs() < 0.03 && tr[1] < 0.03, "{rot} {tr:?}");
            assert!((br[0] - right).abs() < 0.03 && br[1] > 0.97, "{rot} {br:?}");
            assert!((bl[0] - left).abs() < 0.03 && bl[1] > 0.97, "{rot} {bl:?}");
        }
    }

    #[test]
    fn placement_invalid_values_are_harmless() {
        let ok = StencilPlacement::default();
        for bad in [
            StencilPlacement {
                size_px: [0.0, 10.0],
                ..ok
            },
            StencilPlacement {
                size_px: [10.0, -1.0],
                ..ok
            },
            StencilPlacement {
                size_px: [f32::NAN, 10.0],
                ..ok
            },
            StencilPlacement {
                center_px: [f32::INFINITY, 0.0],
                ..ok
            },
            StencilPlacement {
                rotation_rad: f32::NAN,
                ..ok
            },
        ] {
            assert!(bad.uv_at([0.0, 0.0]).is_none());
            assert!(bad.uv_at(bad.center_px).is_none());
            assert_eq!(bad.outline_px(), [bad.center_px; 4]);
        }
    }

    #[test]
    fn placement_manipulation_helpers() {
        let p = StencilPlacement {
            center_px: [10.0, 20.0],
            size_px: [40.0, 20.0],
            rotation_rad: 0.0,
            mirror_x: true,
        };
        let t = p.translated([5.0, -5.0]);
        assert_eq!(t.center_px, [15.0, 15.0]);
        assert_eq!(t.size_px, p.size_px);
        assert!(t.mirror_x);
        let s = p.scaled_about_center(2.0);
        assert_eq!((s.center_px, s.size_px), (p.center_px, [80.0, 40.0]));
        let r = p.rotated(1.0);
        assert!((r.rotation_rad - 1.0).abs() < 1e-6);
        // normaliza para (-π, π]
        let wrapped = p.rotated(7.0);
        assert!(wrapped.rotation_rad > -std::f32::consts::PI - 1e-4);
        assert!(wrapped.rotation_rad <= std::f32::consts::PI + 1e-4);
        assert!((wrapped.rotation_rad - (7.0 - std::f32::consts::TAU)).abs() < 1e-4);
        // entradas hostis: sem efeito
        assert_eq!(p.translated([f32::NAN, 0.0]), p);
        assert_eq!(p.scaled_about_center(0.0), p);
        assert_eq!(p.scaled_about_center(-2.0), p);
        assert_eq!(p.scaled_about_center(f32::INFINITY), p);
        assert_eq!(p.rotated(f32::NAN), p);
        assert_eq!(p.rotated(f32::INFINITY), p);
        // escala repetida não estoura nem zera
        let mut big = p;
        for _ in 0..200 {
            big = big.scaled_about_center(10.0);
        }
        assert!(big.size_px.iter().all(|v| v.is_finite() && *v <= 1.0e6));
        let mut tiny = p;
        for _ in 0..200 {
            tiny = tiny.scaled_about_center(0.1);
        }
        assert!(tiny.size_px.iter().all(|v| *v > 0.0));
    }

    // ---- Amostragem --------------------------------------------------------

    #[test]
    fn bilinear_clamps_to_edge_and_averages_in_between() {
        let mut c = Canvas::new(2, 1, CLEAR);
        c.set(0, 0, [0, 0, 0, 255]);
        c.set(1, 0, [255, 255, 255, 255]);
        let at = |u: f32| sample_bilinear(&c, [u, 0.5]);
        assert_eq!(at(0.0), [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(at(1.0), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(at(-5.0), at(0.0));
        assert_eq!(at(9.0), at(1.0));
        let mid = at(0.5);
        assert!((mid[0] - 0.5).abs() < 1e-5 && (mid[3] - 1.0).abs() < 1e-6);
        let quarter = at(0.375);
        assert!((quarter[0] - 0.25).abs() < 1e-5);
    }

    #[test]
    fn bilinear_is_premultiplied_so_transparent_edges_do_not_darken() {
        let mut c = Canvas::new(2, 1, CLEAR);
        c.set(0, 0, RED);
        // o vizinho é transparente com RGB preto
        let mid = sample_bilinear(&c, [0.5, 0.5]);
        assert!((mid[0] - 1.0).abs() < 1e-5, "{mid:?}");
        assert!(mid[1].abs() < 1e-5 && mid[2].abs() < 1e-5);
        assert!((mid[3] - 0.5).abs() < 1e-5);
        // totalmente transparente: zero
        assert_eq!(sample_bilinear(&c, [1.0, 0.5]), [0.0; 4]);
    }

    #[test]
    fn bilinear_rejects_bad_images_and_uvs() {
        let c = solid(2, 2, RED);
        assert_eq!(sample_bilinear(&c, [f32::NAN, 0.0]), [0.0; 4]);
        assert_eq!(sample_bilinear(&c, [0.0, f32::INFINITY]), [0.0; 4]);
        let empty = Canvas {
            w: 0,
            h: 0,
            pixels: Vec::new(),
        };
        assert_eq!(sample_bilinear(&empty, [0.5, 0.5]), [0.0; 4]);
        let short = Canvas {
            w: 100,
            h: 100,
            pixels: vec![255; 8],
        };
        assert_eq!(sample_bilinear(&short, [0.5, 0.5]), [0.0; 4]);
        let huge = Canvas {
            w: u32::MAX,
            h: u32::MAX,
            pixels: vec![255; 8],
        };
        assert_eq!(sample_bilinear(&huge, [0.5, 0.5]), [0.0; 4]);
        // uv absurdo é preso à borda
        assert_eq!(sample_bilinear(&c, [1e30, -1e30]), [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn stencil_alpha_channels() {
        let p = StencilPlacement {
            center_px: [10.0, 10.0],
            size_px: [10.0, 10.0],
            ..StencilPlacement::default()
        };
        let half = solid(2, 2, [255, 255, 255, 128]);
        let a = stencil_alpha(&half, &p, [10.0, 10.0], StencilChannel::Alpha);
        assert!((a - 128.0 / 255.0).abs() < 1e-3);
        let l = stencil_alpha(&half, &p, [10.0, 10.0], StencilChannel::Luma);
        assert!((l - 128.0 / 255.0).abs() < 1e-3);
        let black = solid(2, 2, [0, 0, 0, 255]);
        assert_eq!(
            stencil_alpha(&black, &p, [10.0, 10.0], StencilChannel::Luma),
            0.0
        );
        assert_eq!(
            stencil_alpha(&black, &p, [10.0, 10.0], StencilChannel::Alpha),
            1.0
        );
        // verde puro pesa 0,7152
        let green = solid(2, 2, GREEN);
        let g = stencil_alpha(&green, &p, [10.0, 10.0], StencilChannel::Luma);
        assert!((g - 0.7152).abs() < 1e-3);
        // fora do retângulo e placement inválido: zero
        assert_eq!(
            stencil_alpha(&green, &p, [50.0, 10.0], StencilChannel::Alpha),
            0.0
        );
        let bad = StencilPlacement {
            size_px: [0.0, 0.0],
            ..p
        };
        assert_eq!(
            stencil_alpha(&green, &bad, [10.0, 10.0], StencilChannel::Alpha),
            0.0
        );
    }

    // ---- project_image_onto_mesh -------------------------------------------

    fn project(mesh: &Mesh, target: &mut Canvas, image: &Canvas, p: &ProjectionParams) -> Vec<u32> {
        project_image_onto_mesh(target, mesh, &always, &depth_for(mesh), image, p)
    }

    #[test]
    fn quad_facing_the_camera_receives_the_image_upright() {
        let mesh = single_quad(false);
        let mut target = Canvas::new(16, 16, CLEAR);
        let dirty = project(
            &mesh,
            &mut target,
            &corner_image(),
            &params(fit_placement()),
        );
        assert_eq!(target.get(0, 0), Some(RED));
        assert_eq!(target.get(15, 0), Some(GREEN));
        assert_eq!(target.get(0, 15), Some(BLUE));
        assert_eq!(target.get(15, 15), Some(YELLOW));
        // o centro mistura as quatro cores (opaco)
        let mid = target.get(8, 8).unwrap();
        assert_eq!(mid[3], 255);
        assert!(dirty == vec![0]);
        // a quase totalidade da textura foi coberta
        let painted = target.pixels.chunks(4).filter(|p| p[3] > 0).count();
        assert_eq!(painted, 256);
    }

    #[test]
    fn rotation_moves_the_projected_corners() {
        let mesh = single_quad(false);
        let mut target = Canvas::new(16, 16, CLEAR);
        let placement = StencilPlacement {
            rotation_rad: std::f32::consts::FRAC_PI_2,
            ..fit_placement()
        };
        project(&mesh, &mut target, &corner_image(), &params(placement));
        // girar 90° no sentido horário: vermelho (TL) vai para TR, verde para BR
        assert_eq!(target.get(15, 0), Some(RED));
        assert_eq!(target.get(15, 15), Some(GREEN));
        assert_eq!(target.get(0, 15), Some(YELLOW));
        assert_eq!(target.get(0, 0), Some(BLUE));
    }

    #[test]
    fn mirror_swaps_left_and_right() {
        let mesh = single_quad(false);
        let mut target = Canvas::new(16, 16, CLEAR);
        let placement = StencilPlacement {
            mirror_x: true,
            ..fit_placement()
        };
        project(&mesh, &mut target, &corner_image(), &params(placement));
        assert_eq!(target.get(0, 0), Some(GREEN));
        assert_eq!(target.get(15, 0), Some(RED));
        assert_eq!(target.get(0, 15), Some(YELLOW));
        assert_eq!(target.get(15, 15), Some(BLUE));
    }

    #[test]
    fn quad_behind_another_quad_is_not_painted() {
        let mesh = occluded_pair();
        let image = solid(2, 2, RED);
        let only_back = |fi: usize| fi == 0;
        let mut target = Canvas::new(32, 16, CLEAR);
        let depth = depth_for(&mesh);
        let dirty = project_image_onto_mesh(
            &mut target,
            &mesh,
            &only_back,
            &depth,
            &image,
            &params(fit_placement()),
        );
        assert!(!dirty.is_empty());
        // cantos do quad de trás: visíveis
        assert_eq!(target.get(2, 2), Some(RED));
        assert_eq!(target.get(13, 13), Some(RED));
        // centro: escondido pelo quad da frente
        assert_eq!(target.get(7, 7), Some(CLEAR));
        assert_eq!(target.get(5, 5), Some(CLEAR));
        assert_eq!(target.get(10, 10), Some(CLEAR));
        // face vetada (a da frente) intacta, inclusive na borda da dilatação
        for y in 0..16 {
            for x in 16..32 {
                assert_eq!(target.get(x, y), Some(CLEAR), "({x},{y})");
            }
        }
        // controle: sem o oclusor no z-buffer o centro é pintado
        let mut back_only = Mesh::default();
        push_quad(&mut back_only, 0.0, 1.0, 0.0, 0.5, false);
        let mut control = Canvas::new(32, 16, CLEAR);
        let (vp, [w, h]) = camera();
        let depth = DepthBuffer::from_mesh(vp, w, h, &back_only);
        project_image_onto_mesh(
            &mut control,
            &mesh,
            &only_back,
            &depth,
            &image,
            &params(fit_placement()),
        );
        assert_eq!(control.get(7, 7), Some(RED));
    }

    #[test]
    fn back_facing_face_is_skipped_only_with_culling() {
        let mesh = single_quad(true);
        let image = solid(2, 2, RED);
        let mut culled = Canvas::new(16, 16, CLEAR);
        let dirty = project(&mesh, &mut culled, &image, &params(fit_placement()));
        assert!(dirty.is_empty());
        assert!(culled.pixels.iter().all(|b| *b == 0));

        let mut open = Canvas::new(16, 16, CLEAR);
        let mut p = params(fit_placement());
        p.back_face_cull = false;
        let dirty = project(&mesh, &mut open, &image, &p);
        assert!(!dirty.is_empty());
        assert_eq!(open.get(8, 8), Some(RED));
    }

    #[test]
    fn opacity_blends_once_per_texel() {
        let mesh = single_quad(false);
        let image = solid(2, 2, RED);
        let mut target = Canvas::new(16, 16, [255, 255, 255, 255]);
        let mut p = params(fit_placement());
        p.opacity = 0.5;
        project(&mesh, &mut target, &image, &p);
        let first = target.get(0, 0).unwrap();
        assert_eq!(first[0], 255);
        assert!((127..=129).contains(&first[1]) && (127..=129).contains(&first[2]));
        assert_eq!(first[3], 255);
        // diagonal do quad e dilatação não compõem duas vezes
        for y in 0..16 {
            for x in 0..16 {
                assert_eq!(target.get(x, y), Some(first), "({x},{y})");
            }
        }
    }

    #[test]
    fn opacity_zero_nan_and_overshoot() {
        let mesh = single_quad(false);
        let image = solid(2, 2, RED);
        for bad in [0.0, -1.0, f32::NAN] {
            let mut target = Canvas::new(16, 16, CLEAR);
            let mut p = params(fit_placement());
            p.opacity = bad;
            assert!(project(&mesh, &mut target, &image, &p).is_empty());
            assert!(target.pixels.iter().all(|b| *b == 0));
        }
        let mut target = Canvas::new(16, 16, CLEAR);
        let mut p = params(fit_placement());
        p.opacity = 7.0;
        project(&mesh, &mut target, &image, &p);
        assert_eq!(target.get(4, 4), Some(RED));
    }

    #[test]
    fn transparent_destination_keeps_the_colour() {
        let mesh = single_quad(false);
        let image = solid(2, 2, RED);
        let mut target = Canvas::new(16, 16, CLEAR);
        let mut p = params(fit_placement());
        p.opacity = 0.5;
        project(&mesh, &mut target, &image, &p);
        let px = target.get(8, 8).unwrap();
        assert_eq!(&px[..3], &[255, 0, 0]);
        assert!((127..=129).contains(&px[3]));
    }

    #[test]
    fn disallowed_faces_are_untouched() {
        let mesh = single_quad(false);
        let image = solid(2, 2, RED);
        let mut target = Canvas::new(16, 16, [9, 9, 9, 255]);
        let before = target.clone();
        let depth = depth_for(&mesh);
        let dirty = project_image_onto_mesh(
            &mut target,
            &mesh,
            &|_| false,
            &depth,
            &image,
            &params(fit_placement()),
        );
        assert!(dirty.is_empty());
        assert_eq!(target, before);
    }

    #[test]
    fn texels_outside_the_stencil_are_untouched() {
        let mesh = single_quad(false);
        let image = solid(2, 2, RED);
        let mut target = Canvas::new(16, 16, CLEAR);
        // stencil de 8 px no meio do quad (que ocupa ~22 px)
        let placement = StencilPlacement {
            center_px: [32.0, 32.0],
            size_px: [8.0, 8.0],
            ..StencilPlacement::default()
        };
        project(&mesh, &mut target, &image, &params(placement));
        assert_eq!(target.get(8, 8), Some(RED));
        assert_eq!(target.get(7, 7), Some(RED));
        assert_eq!(target.get(0, 0), Some(CLEAR));
        assert_eq!(target.get(15, 15), Some(CLEAR));
        assert_eq!(target.get(8, 1), Some(CLEAR));
        let painted = target.pixels.chunks(4).filter(|p| p[3] > 0).count();
        // 8 px de tela ≈ 3,2 texels de 16 → bem menos que a textura toda
        assert!(painted > 4 && painted < 40, "{painted}");
        // stencil totalmente fora do quad: nada muda
        let mut miss = Canvas::new(16, 16, CLEAR);
        let away = StencilPlacement {
            center_px: [5.0, 5.0],
            size_px: [4.0, 4.0],
            ..StencilPlacement::default()
        };
        assert!(project(&mesh, &mut miss, &image, &params(away)).is_empty());
    }

    #[test]
    fn transparent_image_pixels_do_not_paint() {
        let mesh = single_quad(false);
        let mut image = Canvas::new(2, 2, CLEAR);
        image.set(0, 0, RED);
        let mut target = Canvas::new(16, 16, CLEAR);
        project(&mesh, &mut target, &image, &params(fit_placement()));
        assert_eq!(target.get(0, 0), Some(RED));
        assert_eq!(target.get(15, 15), Some(CLEAR));
    }

    #[test]
    fn dirty_tiles_follow_the_dirty_tiles_convention() {
        let mesh = single_quad(false);
        let image = solid(2, 2, RED);
        // canvas 64 × 64 = 2 × 2 tiles de 32
        let mut all = Canvas::new(64, 64, CLEAR);
        let dirty = project(&mesh, &mut all, &image, &params(fit_placement()));
        assert_eq!(dirty, vec![0, 1, 2, 3]);
        // só o quadrante superior esquerdo da tela → só o tile 0
        let mut one = Canvas::new(64, 64, CLEAR);
        let q = StencilPlacement {
            center_px: [26.5, 26.5],
            size_px: [11.0, 11.0],
            ..StencilPlacement::default()
        };
        let dirty = project(&mesh, &mut one, &image, &params(q));
        assert_eq!(dirty, vec![0]);
        // canvas não múltiplo do tile: tiles_x = ceil(40 / 32) = 2
        let mut odd = Canvas::new(40, 8, CLEAR);
        let dirty = project(&mesh, &mut odd, &image, &params(fit_placement()));
        assert_eq!(dirty, vec![0, 1]);
        // pintar a mesma cor de novo não altera nada
        let again = project(&mesh, &mut odd, &image, &params(fit_placement()));
        assert!(again.is_empty());
    }

    #[test]
    fn projection_is_deterministic() {
        let mesh = occluded_pair();
        let image = corner_image();
        let run = || {
            let mut t = Canvas::new(32, 16, [10, 20, 30, 255]);
            let mut p = params(fit_placement().rotated(0.3));
            p.opacity = 0.7;
            let d = project(&mesh, &mut t, &image, &p);
            (t, d)
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn projection_degenerate_inputs_are_harmless() {
        let image = solid(2, 2, RED);
        let mesh = single_quad(false);
        let ok = params(fit_placement());
        let depth = depth_for(&mesh);
        let run = |target: &mut Canvas,
                   mesh: &Mesh,
                   depth: &DepthBuffer,
                   image: &Canvas,
                   p: &ProjectionParams| {
            project_image_onto_mesh(target, mesh, &always, depth, image, p)
        };

        // malha vazia
        let mut t = Canvas::new(16, 16, CLEAR);
        assert!(run(&mut t, &Mesh::default(), &depth, &image, &ok).is_empty());
        // canvas alvo inválido
        let mut bad = Canvas {
            w: 0,
            h: 0,
            pixels: Vec::new(),
        };
        assert!(run(&mut bad, &mesh, &depth, &image, &ok).is_empty());
        let mut short = Canvas {
            w: 16,
            h: 16,
            pixels: vec![0; 4],
        };
        assert!(run(&mut short, &mesh, &depth, &image, &ok).is_empty());
        // imagem inválida
        let empty_img = Canvas {
            w: 0,
            h: 0,
            pixels: Vec::new(),
        };
        let mut t = Canvas::new(16, 16, CLEAR);
        assert!(run(&mut t, &mesh, &depth, &empty_img, &ok).is_empty());
        // placement inválido
        let mut broken = ok;
        broken.placement.size_px = [f32::NAN, 1.0];
        assert!(run(&mut t, &mesh, &depth, &image, &broken).is_empty());
        // z-buffer vazio / de outro viewport / de outra câmera
        let (vp, _) = camera();
        let none = DepthBuffer::build(vp, 100_000, 100_000, []);
        assert!(run(&mut t, &mesh, &none, &image, &ok).is_empty());
        let mut other_vp = ok;
        other_vp.viewport = [32, 32];
        assert!(run(&mut t, &mesh, &depth, &image, &other_vp).is_empty());
        let mut other_cam = ok;
        other_cam.view_proj = Mat4::IDENTITY;
        assert!(run(&mut t, &mesh, &depth, &image, &other_cam).is_empty());
        assert!(t.pixels.iter().all(|b| *b == 0));

        // malha hostil: vértice NaN, UV NaN, índice fora de faixa, UV degenerada
        let mut nan_vert = single_quad(false);
        nan_vert.verts[0].pos[0] = f32::NAN;
        let mut nan_uv = single_quad(false);
        nan_uv.faces[0].uv[0] = [f32::NAN, 0.0];
        let mut bad_idx = single_quad(false);
        bad_idx.faces[0].verts[2] = 77;
        let mut flat_uv = single_quad(false);
        flat_uv.faces[0].uv = vec![[0.5, 0.5]; 4];
        for hostile in [nan_vert, nan_uv, bad_idx, flat_uv] {
            let d = depth_for(&hostile);
            let mut t = Canvas::new(16, 16, CLEAR);
            run(&mut t, &hostile, &d, &image, &ok);
        }
    }

    #[test]
    fn view_proj_without_inverse_disables_only_culling() {
        let mesh = single_quad(false);
        let image = solid(2, 2, RED);
        // matriz que ainda projeta algo mas não tem inversa
        let (vp, _) = camera();
        let singular = Mat4::from_cols(vp.x_axis, vp.y_axis, vp.z_axis, vp.z_axis);
        let depth = DepthBuffer::from_mesh(singular, 64, 64, &mesh);
        let mut p = params(fit_placement());
        p.view_proj = singular;
        let mut t = Canvas::new(16, 16, CLEAR);
        assert!(project_image_onto_mesh(&mut t, &mesh, &always, &depth, &image, &p).is_empty());
        p.back_face_cull = false;
        project_image_onto_mesh(&mut t, &mesh, &always, &depth, &image, &p);
    }

    // ---- fill_screen_polygon -----------------------------------------------

    fn fill(
        mesh: &Mesh,
        target: &mut Canvas,
        face_allowed: &dyn Fn(usize) -> bool,
        polygon: &[[f32; 2]],
    ) -> Vec<u32> {
        let (vp, viewport) = camera();
        fill_screen_polygon(
            target,
            mesh,
            face_allowed,
            &depth_for(mesh),
            polygon,
            RED,
            1.0,
            vp,
            viewport,
            BIAS,
        )
    }

    #[test]
    fn polygon_fill_covers_exactly_the_visible_part() {
        let mesh = occluded_pair();
        let mut target = Canvas::new(32, 16, CLEAR);
        // metade direita da tela
        let polygon = [[32.0, 0.0], [64.0, 0.0], [64.0, 64.0], [32.0, 64.0]];
        let dirty = fill(&mesh, &mut target, &|fi| fi == 0, &polygon);
        assert!(!dirty.is_empty());
        let hp = half_px_at_5();
        let hidden_half = 0.4 * 32.0 / (3.5 * 30f32.to_radians().tan());
        let mut expected_count = 0;
        for y in 0..16u32 {
            for x in 0..16u32 {
                let wx = -1.0 + (x as f32 + 0.5) / 8.0;
                let wy = 1.0 - (y as f32 + 0.5) / 8.0;
                let (px, py) = (32.0 + wx * hp, 32.0 - wy * hp);
                // a oclusão vale por pixel: conta o centro do pixel que contém o ponto
                let (cx, cy) = (px.floor() + 0.5, py.floor() + 0.5);
                let occluded = (cx - 32.0).abs() < hidden_half && (cy - 32.0).abs() < hidden_half;
                let expected = px >= 32.0 && !occluded;
                expected_count += expected as u32;
                let got = target.get(x, y) == Some(RED);
                assert_eq!(got, expected, "texel ({x},{y}) px=({px},{py})");
            }
        }
        assert!(expected_count > 30);
        // face da frente vetada: intacta
        for y in 0..16 {
            for x in 16..32 {
                assert_eq!(target.get(x, y), Some(CLEAR));
            }
        }
    }

    #[test]
    fn polygon_fill_uses_the_even_odd_rule() {
        let mesh = single_quad(false);
        // quadrado externo + quadrado interno no mesmo caminho = furo no meio
        let polygon = [
            [10.0, 10.0],
            [54.0, 10.0],
            [54.0, 54.0],
            [10.0, 54.0],
            [10.0, 10.0],
            [26.0, 26.0],
            [38.0, 26.0],
            [38.0, 38.0],
            [26.0, 38.0],
            [26.0, 26.0],
        ];
        let mut target = Canvas::new(16, 16, CLEAR);
        fill(&mesh, &mut target, &always, &polygon);
        assert_eq!(target.get(1, 1), Some(RED));
        assert_eq!(target.get(14, 14), Some(RED));
        assert_eq!(target.get(8, 8), Some(CLEAR));
    }

    #[test]
    fn polygon_fill_applies_color_alpha_and_opacity() {
        let mesh = single_quad(false);
        let (vp, viewport) = camera();
        let depth = depth_for(&mesh);
        let polygon = [[0.0, 0.0], [64.0, 0.0], [64.0, 64.0], [0.0, 64.0]];
        let mut target = Canvas::new(16, 16, [255, 255, 255, 255]);
        fill_screen_polygon(
            &mut target,
            &mesh,
            &always,
            &depth,
            &polygon,
            [0, 0, 0, 128],
            0.5,
            vp,
            viewport,
            BIAS,
        );
        let px = target.get(8, 8).unwrap();
        // alfa efetivo ≈ 0,25
        assert!((190..=194).contains(&px[0]), "{px:?}");
        // opacidade 0 e NaN: nada
        let mut untouched = Canvas::new(16, 16, CLEAR);
        for o in [0.0, f32::NAN] {
            assert!(
                fill_screen_polygon(
                    &mut untouched,
                    &mesh,
                    &always,
                    &depth,
                    &polygon,
                    RED,
                    o,
                    vp,
                    viewport,
                    BIAS,
                )
                .is_empty()
            );
        }
    }

    #[test]
    fn polygon_fill_degenerate_inputs_are_harmless() {
        let mesh = single_quad(false);
        let (vp, viewport) = camera();
        let depth = depth_for(&mesh);
        let mut t = Canvas::new(16, 16, CLEAR);
        let go = |t: &mut Canvas, poly: &[[f32; 2]], d: &DepthBuffer, vp: Mat4, v: [u32; 2]| {
            fill_screen_polygon(t, &mesh, &always, d, poly, RED, 1.0, vp, v, BIAS)
        };
        let full = [[0.0, 0.0], [64.0, 0.0], [64.0, 64.0], [0.0, 64.0]];
        assert!(go(&mut t, &[], &depth, vp, viewport).is_empty());
        assert!(go(&mut t, &[[1.0, 1.0], [5.0, 5.0]], &depth, vp, viewport).is_empty());
        let nan = [[0.0, 0.0], [f32::NAN, 0.0], [64.0, 64.0]];
        assert!(go(&mut t, &nan, &depth, vp, viewport).is_empty());
        let inf = [[0.0, 0.0], [f32::INFINITY, 0.0], [64.0, 64.0]];
        assert!(go(&mut t, &inf, &depth, vp, viewport).is_empty());
        // polígono sem área (colinear) ou fora da tela: nada pintado
        let line = [[0.0, 0.0], [10.0, 10.0], [20.0, 20.0]];
        assert!(go(&mut t, &line, &depth, vp, viewport).is_empty());
        let away = [[-100.0, -100.0], [-50.0, -100.0], [-50.0, -50.0]];
        assert!(go(&mut t, &away, &depth, vp, viewport).is_empty());
        // coordenadas gigantes (porém finitas) não travam
        let huge = [[-1e30, -1e30], [1e30, -1e30], [1e30, 1e30], [-1e30, 1e30]];
        assert!(!go(&mut t, &huge, &depth, vp, viewport).is_empty());
        // viewport/câmera divergentes e z-buffer vazio
        assert!(go(&mut t, &full, &depth, vp, [32, 32]).is_empty());
        assert!(go(&mut t, &full, &depth, Mat4::IDENTITY, viewport).is_empty());
        let none = DepthBuffer::build(vp, 0, 0, []);
        assert!(go(&mut t, &full, &none, vp, viewport).is_empty());
        // canvas alvo inválido
        let mut bad = Canvas {
            w: 5,
            h: 5,
            pixels: Vec::new(),
        };
        assert!(go(&mut bad, &full, &depth, vp, viewport).is_empty());
    }

    #[test]
    fn polygon_fill_is_deterministic_and_skips_vetoed_faces() {
        let mesh = occluded_pair();
        let polygon = [[0.0, 0.0], [64.0, 0.0], [64.0, 64.0], [0.0, 64.0]];
        let run = || {
            let mut t = Canvas::new(32, 16, CLEAR);
            let d = fill(&mesh, &mut t, &always, &polygon);
            (t, d)
        };
        assert_eq!(run(), run());
        let mut none = Canvas::new(32, 16, CLEAR);
        assert!(fill(&mesh, &mut none, &|_| false, &polygon).is_empty());
        assert!(none.pixels.iter().all(|b| *b == 0));
    }

    #[test]
    fn point_in_polygon_basics() {
        let tri = [[0.0, 0.0], [10.0, 0.0], [0.0, 10.0]];
        assert!(point_in_polygon_even_odd([2.0, 2.0], &tri));
        assert!(!point_in_polygon_even_odd([8.0, 8.0], &tri));
        assert!(!point_in_polygon_even_odd([-1.0, 2.0], &tri));
    }
}
