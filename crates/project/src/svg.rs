//! SVG como decalque de pintura e como fonte de perfis DRAW (P3D-133, P3D-156/158).
//!
//! Camada de domínio sobre `resvg`/`usvg` **sem** os recursos `text`, `svgz` e
//! `raster-images`. Consequências deliberadas (anti-bloat, cap. 42–44):
//!
//! - elementos `<text>` são **ignorados** (sem fontes, sem acesso ao sistema);
//! - `<image>` é ignorado: nenhuma imagem embutida (data URL) ou externa
//!   (arquivo/rede) é carregada — o resolvedor de href devolve sempre `None`;
//! - SVGZ (gzip) não é aceito: não há descompressão, logo não há "zip bomb";
//! - DTD com `<!ENTITY` é recusado antes do parse (expansão de entidades);
//! - a expansão de `<use>` (que o `usvg` faz no parse, sem teto próprio) é
//!   medida **antes**, com memoização por `id`: documentos cujo número de
//!   elementos expandidos ou cuja profundidade de aninhamento (grupos e
//!   cadeias de `<use>`, que geram uma camada de render por nível) passa dos
//!   tetos devolvem [`SvgError::TooLarge`] sem chamar o `usvg`.
//!
//! Duas saídas:
//!
//! 1. [`rasterize_svg`]: `Canvas` RGBA **straight** (não pré-multiplicado, como
//!    o resto de `Canvas`) para o `DecalLayer`;
//! 2. [`svg_subpaths`] / [`svg_to_splines`]: geometria de caminhos como nós
//!    Bézier com alças **relativas** (convenção de `SplinePoint`), normalizada
//!    (centrada, y para cima, escala uniforme) para virar perfil/spline DRAW.
//!
//! Tudo é determinístico: mesma entrada, mesma saída (exceto os `Uuid` novos
//! das splines, que são identidade e não geometria).

use std::collections::{HashMap, HashSet};

use resvg::tiny_skia::{PathSegment, Pixmap, Point, Transform};
use resvg::usvg::{self, ImageHrefResolver, Node, Options};

use crate::Canvas;
use crate::spline::{SplineInterpolation, SplinePoint, SplineResource};

/// Tamanho máximo do texto SVG aceito (bytes), checado antes de parsear.
pub const MAX_SVG_BYTES: usize = 2 * 1024 * 1024;
/// Maior lado (px) que [`rasterize_svg`] produz, mesmo se `max_px` for maior.
pub const MAX_RASTER_PX: u32 = 1024;
/// Teto de nós Bézier extraídos por [`svg_subpaths`].
pub const MAX_SVG_NODES: usize = 20_000;

/// Teto de elementos **após** a expansão de `<use>` (anti "bomba de `<use>`").
pub const MAX_SVG_ELEMENTS: u64 = 250_000;
/// Teto de profundidade expandida (grupos + cadeias de `<use>`): cada nível
/// com opacidade/máscara pode custar uma camada do tamanho do canvas.
pub const MAX_SVG_DEPTH: u32 = 64;
/// Teto de nós XML do documento de origem (checado pelo próprio parser).
const MAX_XML_NODES: u32 = 1_000_000;

/// Distância² abaixo da qual dois pontos (em unidades do SVG) são o mesmo nó.
const SAME_POINT_EPS_SQ: f64 = 1.0e-12;

/// Erros de leitura/rasterização de SVG.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SvgError {
    /// Entrada acima de [`MAX_SVG_BYTES`] (ou geometria acima de [`MAX_SVG_NODES`]).
    #[error("SVG grande demais")]
    TooLarge,
    /// XML/SVG malformado (mensagem do parser).
    #[error("SVG inválido: {0}")]
    Parse(String),
    /// Texto vazio ou documento sem nenhum conteúdo desenhável.
    #[error("SVG sem conteúdo")]
    Empty,
    /// Tamanho do documento nulo/ausente ou geometria sem extensão.
    #[error("SVG com tamanho zero")]
    ZeroSize,
    /// Falha ao alocar/renderizar o raster.
    #[error("falha ao rasterizar o SVG")]
    Render,
}

/// Tamanho intrínseco do documento (unidades de usuário, após `viewBox`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgInfo {
    pub width: f32,
    pub height: f32,
}

/// Nó Bézier de um subcaminho. Alças **relativas** a `position`
/// (mesma convenção de [`SplinePoint`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SvgNode {
    pub position: [f64; 2],
    pub handle_in: [f64; 2],
    pub handle_out: [f64; 2],
}

/// Sequência de nós de um `M ... [Z]` do SVG.
#[derive(Debug, Clone, PartialEq)]
pub struct SvgSubpath {
    pub nodes: Vec<SvgNode>,
    pub closed: bool,
}

/// Opções seguras: nenhum recurso externo ou embutido é carregado.
fn safe_options() -> Options<'static> {
    Options {
        resources_dir: None,
        image_href_resolver: ImageHrefResolver {
            resolve_data: Box::new(|_, _, _| None),
            resolve_string: Box::new(|_, _| None),
        },
        ..Options::default()
    }
}

fn parse_tree(svg: &str) -> Result<usvg::Tree, SvgError> {
    if svg.len() > MAX_SVG_BYTES {
        return Err(SvgError::TooLarge);
    }
    if svg.trim().is_empty() {
        return Err(SvgError::Empty);
    }
    if svg.contains("<!ENTITY") {
        return Err(SvgError::Parse(
            "declarações <!ENTITY> não são suportadas".to_string(),
        ));
    }
    check_expansion(svg)?;
    usvg::Tree::from_str(svg, &safe_options()).map_err(|error| match error {
        usvg::Error::InvalidSize => SvgError::ZeroSize,
        other => SvgError::Parse(other.to_string()),
    })
}

/// Medida de um elemento já expandido: (elementos, altura da subárvore).
type Extent = (u64, u32);

/// Mede, sem alocar a árvore expandida, quanto o `usvg` geraria ao resolver
/// os `<use>`. Erro de XML vira [`SvgError::Parse`]; estouro de qualquer teto
/// vira [`SvgError::TooLarge`].
fn check_expansion(svg: &str) -> Result<(), SvgError> {
    let options = roxmltree::ParsingOptions {
        allow_dtd: true,
        nodes_limit: MAX_XML_NODES,
        ..Default::default()
    };
    let doc =
        roxmltree::Document::parse_with_options(svg, options).map_err(|error| match error {
            roxmltree::Error::NodesLimitReached => SvgError::TooLarge,
            other => SvgError::Parse(other.to_string()),
        })?;
    let mut ids: HashMap<&str, Vec<roxmltree::NodeId>> = HashMap::new();
    for node in doc.descendants().filter(|n| n.is_element()) {
        if let Some(id) = node.attribute("id") {
            ids.entry(id).or_default().push(node.id());
        }
    }
    let mut walker = Expansion {
        doc: &doc,
        ids,
        memo: HashMap::new(),
        visiting: HashSet::new(),
    };
    walker.measure(doc.root_element(), 0)?;
    Ok(())
}

struct Expansion<'a, 'input> {
    doc: &'a roxmltree::Document<'input>,
    ids: HashMap<&'a str, Vec<roxmltree::NodeId>>,
    memo: HashMap<roxmltree::NodeId, Extent>,
    visiting: HashSet<roxmltree::NodeId>,
}

impl<'a, 'input> Expansion<'a, 'input> {
    /// Medida de `node` (e do que ele expande) a profundidade expandida `depth`.
    /// A recursão é limitada por `MAX_SVG_DEPTH`, então não estoura a pilha.
    fn measure(
        &mut self,
        node: roxmltree::Node<'a, 'input>,
        depth: u32,
    ) -> Result<Extent, SvgError> {
        if depth > MAX_SVG_DEPTH {
            return Err(SvgError::TooLarge);
        }
        if let Some(&extent) = self.memo.get(&node.id()) {
            return Self::fit(extent, depth);
        }
        if !self.visiting.insert(node.id()) {
            // ciclo de `<use>`: o usvg descarta; aqui não soma nada
            return Ok((0, 0));
        }
        let mut count = 1u64;
        let mut height = 1u32;
        let mut children = Vec::new();
        for child in node.children().filter(|c| c.is_element()) {
            children.push(child);
        }
        if node.tag_name().name() == "use" {
            // alvos com `id` repetido: vale o pior caso
            for target in self.use_targets(node) {
                children.push(target);
            }
        }
        for child in children {
            let (c, h) = self.measure(child, depth + 1)?;
            count = count.saturating_add(c);
            height = height.max(h.saturating_add(1));
            if count > MAX_SVG_ELEMENTS {
                return Err(SvgError::TooLarge);
            }
        }
        self.visiting.remove(&node.id());
        let extent = (count, height);
        self.memo.insert(node.id(), extent);
        Self::fit(extent, depth)
    }

    fn fit(extent: Extent, depth: u32) -> Result<Extent, SvgError> {
        if extent.0 > MAX_SVG_ELEMENTS || depth.saturating_add(extent.1) > MAX_SVG_DEPTH + 1 {
            Err(SvgError::TooLarge)
        } else {
            Ok(extent)
        }
    }

    /// Elementos referenciados por `href="#id"` / `xlink:href="#id"` de um `<use>`.
    fn use_targets(&self, node: roxmltree::Node<'a, 'input>) -> Vec<roxmltree::Node<'a, 'input>> {
        let Some(href) = node
            .attributes()
            .find(|a| a.name() == "href")
            .map(|a| a.value())
        else {
            return Vec::new();
        };
        let Some(id) = href.trim().strip_prefix('#') else {
            return Vec::new();
        };
        self.ids
            .get(id)
            .map(|nodes| nodes.iter().filter_map(|&n| self.doc.get_node(n)).collect())
            .unwrap_or_default()
    }
}

/// Lê só o tamanho intrínseco do SVG.
pub fn svg_info(svg: &str) -> Result<SvgInfo, SvgError> {
    let tree = parse_tree(svg)?;
    let size = tree.size();
    Ok(SvgInfo {
        width: size.width(),
        height: size.height(),
    })
}

/// Rasteriza o SVG em RGBA **straight** (não pré-multiplicado).
///
/// A proporção é preservada: o maior lado vira `min(max_px, MAX_RASTER_PX)`
/// (mínimo 1 px) e o outro é arredondado (mínimo 1 px).
pub fn rasterize_svg(svg: &str, max_px: u32) -> Result<Canvas, SvgError> {
    let tree = parse_tree(svg)?;
    if !tree.root().has_children() {
        return Err(SvgError::Empty);
    }
    let size = tree.size();
    let (w, h) = (f64::from(size.width()), f64::from(size.height()));
    if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
        return Err(SvgError::ZeroSize);
    }
    let target = max_px.clamp(1, MAX_RASTER_PX);
    let (out_w, out_h) = if w >= h {
        (target, scaled_side(target, h / w))
    } else {
        (scaled_side(target, w / h), target)
    };
    let mut pixmap = Pixmap::new(out_w, out_h).ok_or(SvgError::Render)?;
    let transform =
        Transform::from_scale((f64::from(out_w) / w) as f32, (f64::from(out_h) / h) as f32);
    resvg::render(&tree, transform, &mut pixmap.as_mut());
    Ok(Canvas {
        w: out_w,
        h: out_h,
        pixels: pixmap.take_demultiplied(),
    })
}

/// Lado menor proporcional, arredondado e limitado a `1..=long`.
fn scaled_side(long: u32, ratio: f64) -> u32 {
    let side = (f64::from(long) * ratio).round();
    if side.is_finite() {
        (side as u32).clamp(1, long)
    } else {
        1
    }
}

/// Extrai a geometria de todos os caminhos (formas incluídas) do SVG,
/// normalizada: transformações aplicadas, y invertido (y para cima), centrada
/// no centro da caixa envolvente e escalada uniformemente para que o maior
/// lado da caixa valha `fit_size`.
///
/// Segmentos quadráticos viram cúbicos; nós consecutivos coincidentes e
/// degenerados são fundidos; caminhos ocultos e `<text>`/`<image>` são
/// ignorados. Estouro de [`MAX_SVG_NODES`] devolve [`SvgError::TooLarge`];
/// `fit_size` não positivo/não finito ou geometria sem extensão devolve
/// [`SvgError::ZeroSize`]; nenhuma geometria devolve [`SvgError::Empty`].
pub fn svg_subpaths(svg: &str, fit_size: f64) -> Result<Vec<SvgSubpath>, SvgError> {
    if !(fit_size.is_finite() && fit_size > 0.0) {
        return Err(SvgError::ZeroSize);
    }
    let tree = parse_tree(svg)?;
    let mut subpaths = Vec::new();
    let mut total = 0usize;
    collect_group(tree.root(), &mut subpaths, &mut total)?;
    if subpaths.is_empty() {
        return Err(SvgError::Empty);
    }
    normalize(&mut subpaths, fit_size)?;
    Ok(subpaths)
}

/// Converte o SVG em splines DRAW (uma por subcaminho), nomeadas "SVG 1", ...
///
/// `CubicBezier` quando alguma alça é não nula, senão `Polyline`. `closed`
/// só vale com 3+ nós (exigência de `SplineResource`).
pub fn svg_to_splines(svg: &str, fit_size: f64) -> Result<Vec<SplineResource>, SvgError> {
    let subpaths = svg_subpaths(svg, fit_size)?;
    Ok(subpaths
        .iter()
        .enumerate()
        .map(|(index, subpath)| {
            let curved = subpath
                .nodes
                .iter()
                .any(|node| node.handle_in != [0.0; 2] || node.handle_out != [0.0; 2]);
            let interpolation = if curved {
                SplineInterpolation::CubicBezier
            } else {
                SplineInterpolation::Polyline
            };
            let mut spline = SplineResource::new(format!("SVG {}", index + 1), interpolation);
            spline.points = subpath
                .nodes
                .iter()
                .map(|node| {
                    let mut point = SplinePoint::new([node.position[0], node.position[1], 0.0]);
                    point.handle_in = [node.handle_in[0], node.handle_in[1], 0.0];
                    point.handle_out = [node.handle_out[0], node.handle_out[1], 0.0];
                    point
                })
                .collect();
            spline.closed = subpath.closed && spline.points.len() >= 3;
            spline
        })
        .collect())
}

fn collect_group(
    group: &usvg::Group,
    out: &mut Vec<SvgSubpath>,
    total: &mut usize,
) -> Result<(), SvgError> {
    for child in group.children() {
        match child {
            Node::Group(inner) => collect_group(inner, out, total)?,
            Node::Path(path) if path.is_visible() => {
                // Coordenadas locais → absolutas (grupos e viewBox aplicados).
                let Some(data) = path.data().clone().transform(path.abs_transform()) else {
                    continue; // coordenadas não finitas após a transformação
                };
                collect_path(&data, out, total)?;
            }
            // Path oculto, Image (nunca carregada) e Text (sem recurso `text`).
            _ => {}
        }
    }
    Ok(())
}

fn to_f64(point: Point) -> [f64; 2] {
    [f64::from(point.x), f64::from(point.y)]
}

fn sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

fn dist_sq(a: [f64; 2], b: [f64; 2]) -> f64 {
    let d = sub(a, b);
    d[0] * d[0] + d[1] * d[1]
}

fn finite2(p: [f64; 2]) -> bool {
    p[0].is_finite() && p[1].is_finite()
}

/// Fecha o subcaminho em construção, se tiver geometria (2+ nós).
fn finish(current: &mut Option<SvgSubpath>, out: &mut Vec<SvgSubpath>) {
    if let Some(mut subpath) = current.take() {
        // Fechado cujo último nó repete o primeiro: funde (a alça de entrada
        // do último vira a do primeiro).
        if subpath.closed && subpath.nodes.len() >= 2 {
            let first = subpath.nodes[0].position;
            let last_index = subpath.nodes.len() - 1;
            if dist_sq(subpath.nodes[last_index].position, first) <= SAME_POINT_EPS_SQ {
                let handle_in = subpath.nodes[last_index].handle_in;
                subpath.nodes[0].handle_in = handle_in;
                subpath.nodes.pop();
            }
        }
        if subpath.nodes.len() >= 2 {
            out.push(subpath);
        }
    }
}

fn collect_path(
    data: &resvg::tiny_skia::Path,
    out: &mut Vec<SvgSubpath>,
    total: &mut usize,
) -> Result<(), SvgError> {
    let mut current: Option<SvgSubpath> = None;
    // Subcaminho descartado por coordenada não finita: ignora até o próximo `M`.
    let mut poisoned = false;

    for segment in data.segments() {
        match segment {
            PathSegment::MoveTo(p) => {
                finish(&mut current, out);
                let position = to_f64(p);
                poisoned = !finite2(position);
                if !poisoned {
                    bump(total, 1)?;
                    current = Some(SvgSubpath {
                        nodes: vec![SvgNode {
                            position,
                            handle_in: [0.0; 2],
                            handle_out: [0.0; 2],
                        }],
                        closed: false,
                    });
                }
            }
            PathSegment::LineTo(p) => {
                push_segment(&mut current, &mut poisoned, total, None, None, to_f64(p))?;
            }
            PathSegment::QuadTo(q, p) => {
                let Some(start) = current.as_ref().and_then(last_position) else {
                    continue;
                };
                let (q, p) = (to_f64(q), to_f64(p));
                // Elevação de grau exata: c1 = p0 + 2/3 (q - p0), c2 = p + 2/3 (q - p).
                let c1 = [
                    start[0] + 2.0 / 3.0 * (q[0] - start[0]),
                    start[1] + 2.0 / 3.0 * (q[1] - start[1]),
                ];
                let c2 = [
                    p[0] + 2.0 / 3.0 * (q[0] - p[0]),
                    p[1] + 2.0 / 3.0 * (q[1] - p[1]),
                ];
                push_segment(&mut current, &mut poisoned, total, Some(c1), Some(c2), p)?;
            }
            PathSegment::CubicTo(c1, c2, p) => {
                push_segment(
                    &mut current,
                    &mut poisoned,
                    total,
                    Some(to_f64(c1)),
                    Some(to_f64(c2)),
                    to_f64(p),
                )?;
            }
            PathSegment::Close => {
                if let Some(subpath) = current.as_mut() {
                    subpath.closed = true;
                }
                finish(&mut current, out);
            }
        }
    }
    finish(&mut current, out);
    Ok(())
}

fn last_position(subpath: &SvgSubpath) -> Option<[f64; 2]> {
    subpath.nodes.last().map(|node| node.position)
}

fn bump(total: &mut usize, count: usize) -> Result<(), SvgError> {
    *total = total.saturating_add(count);
    if *total > MAX_SVG_NODES {
        Err(SvgError::TooLarge)
    } else {
        Ok(())
    }
}

/// Acrescenta o ponto final de um segmento ao subcaminho atual. Controle
/// `None` = reta. Segmento degenerado (mesmo ponto e sem curvatura) é fundido.
fn push_segment(
    current: &mut Option<SvgSubpath>,
    poisoned: &mut bool,
    total: &mut usize,
    c1: Option<[f64; 2]>,
    c2: Option<[f64; 2]>,
    end: [f64; 2],
) -> Result<(), SvgError> {
    if *poisoned {
        return Ok(());
    }
    let Some(subpath) = current.as_mut() else {
        return Ok(());
    };
    let Some(start) = last_position(subpath) else {
        return Ok(());
    };
    let finite = finite2(end) && c1.is_none_or(finite2) && c2.is_none_or(finite2);
    if !finite {
        *poisoned = true;
        *current = None;
        return Ok(());
    }
    let same_point = dist_sq(start, end) <= SAME_POINT_EPS_SQ;
    let flat = c1.is_none_or(|c| dist_sq(c, start) <= SAME_POINT_EPS_SQ)
        && c2.is_none_or(|c| dist_sq(c, end) <= SAME_POINT_EPS_SQ);
    if same_point && flat {
        return Ok(()); // nó duplicado consecutivo
    }
    bump(total, 1)?;
    if let (Some(c1), Some(last)) = (c1, subpath.nodes.last_mut()) {
        last.handle_out = sub(c1, start);
    }
    subpath.nodes.push(SvgNode {
        position: end,
        handle_in: c2.map_or([0.0; 2], |c| sub(c, end)),
        handle_out: [0.0; 2],
    });
    Ok(())
}

/// Raízes de `d/dt` de um cúbico 1D em `(0, 1)` (até 2), para a caixa exata.
fn cubic_extrema(p0: f64, p1: f64, p2: f64, p3: f64) -> [Option<f64>; 2] {
    // B'(t)/3 = a t² + b t + c
    let a = -p0 + 3.0 * p1 - 3.0 * p2 + p3;
    let b = 2.0 * (p0 - 2.0 * p1 + p2);
    let c = p1 - p0;
    let inside = |t: f64| (t.is_finite() && t > 0.0 && t < 1.0).then_some(t);
    if a.abs() < 1.0e-12 {
        if b.abs() < 1.0e-12 {
            return [None, None];
        }
        return [inside(-c / b), None];
    }
    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 {
        return [None, None];
    }
    let root = disc.sqrt();
    [
        inside((-b + root) / (2.0 * a)),
        inside((-b - root) / (2.0 * a)),
    ]
}

fn cubic_at(p0: f64, p1: f64, p2: f64, p3: f64, t: f64) -> f64 {
    let u = 1.0 - t;
    u * u * u * p0 + 3.0 * u * u * t * p1 + 3.0 * u * t * t * p2 + t * t * t * p3
}

/// Caixa envolvente exata (nós + extremos das curvas) de todos os subcaminhos.
fn bounding_box(subpaths: &[SvgSubpath]) -> Option<([f64; 2], [f64; 2])> {
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    let mut include = |p: [f64; 2]| {
        for axis in 0..2 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    };
    for subpath in subpaths {
        let count = subpath.nodes.len();
        for (index, node) in subpath.nodes.iter().enumerate() {
            include(node.position);
            let next = if index + 1 < count {
                Some(&subpath.nodes[index + 1])
            } else if subpath.closed {
                subpath.nodes.first()
            } else {
                None
            };
            let Some(next) = next else { continue };
            let p0 = node.position;
            let p3 = next.position;
            let p1 = [p0[0] + node.handle_out[0], p0[1] + node.handle_out[1]];
            let p2 = [p3[0] + next.handle_in[0], p3[1] + next.handle_in[1]];
            for axis in 0..2 {
                for t in cubic_extrema(p0[axis], p1[axis], p2[axis], p3[axis])
                    .into_iter()
                    .flatten()
                {
                    let mut point = [0.0; 2];
                    for (k, value) in point.iter_mut().enumerate() {
                        *value = cubic_at(p0[k], p1[k], p2[k], p3[k], t);
                    }
                    include(point);
                }
            }
        }
    }
    (finite2(min) && finite2(max)).then_some((min, max))
}

/// Centra na caixa, inverte y e escala uniformemente (maior lado = `fit`).
fn normalize(subpaths: &mut [SvgSubpath], fit: f64) -> Result<(), SvgError> {
    let (min, max) = bounding_box(subpaths).ok_or(SvgError::ZeroSize)?;
    let extent = (max[0] - min[0]).max(max[1] - min[1]);
    if !(extent.is_finite() && extent > 0.0) {
        return Err(SvgError::ZeroSize);
    }
    let scale = fit / extent;
    let center = [(min[0] + max[0]) * 0.5, (min[1] + max[1]) * 0.5];
    let map_point = |p: [f64; 2]| [(p[0] - center[0]) * scale, -(p[1] - center[1]) * scale];
    let map_vec = |v: [f64; 2]| [v[0] * scale, -v[1] * scale];
    for subpath in subpaths {
        for node in &mut subpath.nodes {
            node.position = map_point(node.position);
            node.handle_in = map_vec(node.handle_in);
            node.handle_out = map_vec(node.handle_out);
            // -0.0 vira 0.0: mantém a comparação `!= [0.0; 2]` e a serialização limpas.
            for value in node
                .position
                .iter_mut()
                .chain(node.handle_in.iter_mut())
                .chain(node.handle_out.iter_mut())
            {
                if *value == 0.0 {
                    *value = 0.0;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1.0e-3;

    fn svg(body: &str) -> String {
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 100 100">{body}</svg>"#
        )
    }

    fn pixel(canvas: &Canvas, x: u32, y: u32) -> [u8; 4] {
        let i = ((y * canvas.w + x) * 4) as usize;
        [
            canvas.pixels[i],
            canvas.pixels[i + 1],
            canvas.pixels[i + 2],
            canvas.pixels[i + 3],
        ]
    }

    fn bounds(subpaths: &[SvgSubpath]) -> ([f64; 2], [f64; 2]) {
        bounding_box(subpaths).expect("caixa")
    }

    #[test]
    fn info_reads_size_and_viewbox() {
        let info = svg_info(&svg("<rect width='10' height='10'/>")).unwrap();
        assert_eq!((info.width, info.height), (100.0, 100.0));
        let wide = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="50"/>"#;
        let info = svg_info(wide).unwrap();
        assert_eq!((info.width, info.height), (200.0, 50.0));
    }

    #[test]
    fn rasterize_rect_fills_expected_pixels() {
        let canvas = rasterize_svg(
            &svg("<rect x='0' y='0' width='50' height='100' fill='#00ff00'/>"),
            64,
        )
        .unwrap();
        assert_eq!((canvas.w, canvas.h), (64, 64));
        assert_eq!(canvas.pixels.len(), 64 * 64 * 4);
        assert_eq!(pixel(&canvas, 8, 32), [0, 255, 0, 255]);
        assert_eq!(pixel(&canvas, 56, 32)[3], 0);
    }

    #[test]
    fn rasterize_circle_is_round() {
        let canvas =
            rasterize_svg(&svg("<circle cx='50' cy='50' r='40' fill='red'/>"), 100).unwrap();
        assert_eq!(pixel(&canvas, 50, 50), [255, 0, 0, 255]);
        assert_eq!(pixel(&canvas, 2, 2)[3], 0, "canto fora do círculo");
        assert_eq!(pixel(&canvas, 50, 50 + 38)[3], 255);
    }

    #[test]
    fn rasterize_alpha_is_straight() {
        let canvas = rasterize_svg(
            &svg("<rect width='100' height='100' fill='red' fill-opacity='0.5'/>"),
            16,
        )
        .unwrap();
        let [r, g, b, a] = pixel(&canvas, 8, 8);
        assert_eq!((r, g, b), (255, 0, 0));
        assert_eq!(a, 128);
    }

    #[test]
    fn rasterize_preserves_aspect_and_longest_side() {
        let wide = r#"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="50">
            <rect width="200" height="50" fill="blue"/></svg>"#;
        let canvas = rasterize_svg(wide, 128).unwrap();
        assert_eq!((canvas.w, canvas.h), (128, 32));
        assert_eq!(pixel(&canvas, 64, 16), [0, 0, 255, 255]);
        let tall = r#"<svg xmlns="http://www.w3.org/2000/svg" width="50" height="200">
            <rect width="50" height="200" fill="blue"/></svg>"#;
        let canvas = rasterize_svg(tall, 128).unwrap();
        assert_eq!((canvas.w, canvas.h), (32, 128));
    }

    #[test]
    fn rasterize_clamps_max_px() {
        let source = svg("<rect width='100' height='100'/>");
        let big = rasterize_svg(&source, u32::MAX).unwrap();
        assert_eq!((big.w, big.h), (MAX_RASTER_PX, MAX_RASTER_PX));
        let tiny = rasterize_svg(&source, 0).unwrap();
        assert_eq!((tiny.w, tiny.h), (1, 1));
    }

    #[test]
    fn rasterize_huge_document_size_is_clamped() {
        let huge = r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000000000" height="1000000000"
            viewBox="0 0 10 10"><rect width="10" height="10" fill="red"/></svg>"#;
        let canvas = rasterize_svg(huge, 4096).unwrap();
        assert_eq!((canvas.w, canvas.h), (MAX_RASTER_PX, MAX_RASTER_PX));
        assert_eq!(pixel(&canvas, 512, 512), [255, 0, 0, 255]);
        let extreme = r#"<svg xmlns="http://www.w3.org/2000/svg" width="1000000000" height="1">
            <rect width="1000000000" height="1" fill="red"/></svg>"#;
        let canvas = rasterize_svg(extreme, 256).unwrap();
        assert_eq!((canvas.w, canvas.h), (256, 1));
    }

    #[test]
    fn rasterize_is_deterministic() {
        let source = svg(
            "<g transform='rotate(20 50 50)'><ellipse cx='50' cy='50' rx='30' ry='15' fill='url(#g)'/></g>\
             <defs><linearGradient id='g'><stop offset='0' stop-color='red'/><stop offset='1' stop-color='blue'/></linearGradient></defs>",
        );
        assert_eq!(
            rasterize_svg(&source, 96).unwrap(),
            rasterize_svg(&source, 96).unwrap()
        );
        assert_eq!(
            svg_subpaths(&source, 2.0).unwrap(),
            svg_subpaths(&source, 2.0).unwrap()
        );
    }

    #[test]
    fn text_is_ignored_without_panic() {
        let source =
            svg("<text x='10' y='50' font-size='20'>Olá</text><rect width='10' height='10'/>");
        let canvas = rasterize_svg(&source, 32).unwrap();
        assert_eq!(canvas.pixels.len(), 32 * 32 * 4);
        // Só o retângulo vira geometria.
        let subpaths = svg_subpaths(&source, 1.0).unwrap();
        assert_eq!(subpaths.len(), 1);
        // Só texto: sem conteúdo desenhável.
        let only_text = svg("<text x='10' y='50'>abc</text>");
        assert_eq!(rasterize_svg(&only_text, 32), Err(SvgError::Empty));
        assert_eq!(svg_subpaths(&only_text, 1.0), Err(SvgError::Empty));
    }

    #[test]
    fn malformed_xml_is_parse_error() {
        for bad in ["<svg", "<svg><rect></svg>", "not xml at all", "<a/><b/>"] {
            assert!(
                matches!(rasterize_svg(bad, 16), Err(SvgError::Parse(_))),
                "{bad:?}"
            );
            assert!(matches!(svg_info(bad), Err(SvgError::Parse(_))), "{bad:?}");
        }
    }

    #[test]
    fn empty_input_is_empty() {
        assert_eq!(rasterize_svg("", 16), Err(SvgError::Empty));
        assert_eq!(rasterize_svg("  \n\t ", 16), Err(SvgError::Empty));
        assert_eq!(svg_info(""), Err(SvgError::Empty));
        assert_eq!(svg_subpaths("", 1.0), Err(SvgError::Empty));
        // Documento válido, mas sem nada desenhável.
        assert_eq!(rasterize_svg(&svg(""), 16), Err(SvgError::Empty));
        assert_eq!(svg_subpaths(&svg(""), 1.0), Err(SvgError::Empty));
    }

    #[test]
    fn zero_size_is_rejected() {
        let zero = r#"<svg xmlns="http://www.w3.org/2000/svg" width="0" height="0">
            <rect width="10" height="10"/></svg>"#;
        assert_eq!(rasterize_svg(zero, 16), Err(SvgError::ZeroSize));
        assert_eq!(svg_info(zero), Err(SvgError::ZeroSize));
        let no_size =
            r#"<svg xmlns="http://www.w3.org/2000/svg"><rect width="10" height="10"/></svg>"#;
        assert!(rasterize_svg(no_size, 16).is_ok() || svg_info(no_size).is_err());
    }

    #[test]
    fn oversize_input_is_rejected_before_parsing() {
        let big = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><!--{}--></svg>",
            "x".repeat(3 * 1024 * 1024)
        );
        assert_eq!(rasterize_svg(&big, 16), Err(SvgError::TooLarge));
        assert_eq!(svg_info(&big), Err(SvgError::TooLarge));
        assert_eq!(svg_subpaths(&big, 1.0), Err(SvgError::TooLarge));
        // No limite exato (preenchido com espaços) ainda é aceito para parse.
        let ok = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='1' height='1'><rect width='1' height='1'/></svg>{}",
            " ".repeat(MAX_SVG_BYTES - 100)
        );
        assert!(ok.len() <= MAX_SVG_BYTES);
        assert!(rasterize_svg(&ok, 8).is_ok());
    }

    #[test]
    fn entity_declarations_are_rejected() {
        let bomb = r#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY a "aaaa"><!ENTITY b "&a;&a;&a;">]>
            <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>"#;
        assert!(matches!(rasterize_svg(bomb, 8), Err(SvgError::Parse(_))));
    }

    #[test]
    fn external_href_is_ignored() {
        // Mesmo apontando para um arquivo existente, nada é carregado.
        let source = svg("<image href='/etc/passwd' width='100' height='100'/>\
             <image href='https://example.com/a.png' width='100' height='100'/>\
             <image href='data:image/png;base64,iVBORw0KGgo=' width='100' height='100'/>\
             <use href='file:///etc/hosts#x'/>");
        assert_eq!(rasterize_svg(&source, 16), Err(SvgError::Empty));
        // Com conteúdo vetorial ao lado, só ele aparece.
        let mixed = svg(
            "<image href='/etc/passwd' width='100' height='100'/><rect width='100' height='100' fill='red'/>",
        );
        let canvas = rasterize_svg(&mixed, 16).unwrap();
        assert_eq!(pixel(&canvas, 8, 8), [255, 0, 0, 255]);
    }

    #[test]
    fn group_transform_and_viewbox_scaling() {
        // viewBox 0..10 em documento de 100: escala 10x. Grupo translada 2 em x.
        let source = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100" viewBox="0 0 10 10">
            <g transform="translate(2 0)"><rect x="0" y="0" width="4" height="10" fill="red"/></g></svg>"#;
        let canvas = rasterize_svg(source, 100).unwrap();
        assert_eq!(pixel(&canvas, 30, 50), [255, 0, 0, 255]);
        assert_eq!(pixel(&canvas, 10, 50)[3], 0);
        assert_eq!(pixel(&canvas, 70, 50)[3], 0);
        // Geometria: retângulo 4x10 centrado, maior lado (altura) = 10.
        let subpaths = svg_subpaths(source, 10.0).unwrap();
        let (min, max) = bounds(&subpaths);
        assert!((max[0] - min[0] - 4.0).abs() < EPS);
        assert!((max[1] - min[1] - 10.0).abs() < EPS);
    }

    #[test]
    fn nested_group_transforms_compose() {
        let source = svg(
            "<g transform='translate(50 0)'><g transform='scale(2)'><rect width='10' height='10'/></g></g>\
             <rect x='0' y='0' width='1' height='1'/>",
        );
        let subpaths = svg_subpaths(&source, 100.0).unwrap();
        assert_eq!(subpaths.len(), 2);
        let (min, max) = bounds(&subpaths);
        // Caixa original: x 0..70, y 0..20 -> lado maior 70.
        assert!((max[0] - min[0] - 100.0).abs() < EPS);
        assert!((max[1] - min[1] - 100.0 * 20.0 / 70.0).abs() < EPS);
    }

    #[test]
    fn rect_subpath_is_closed_polyline_with_four_nodes() {
        let subpaths =
            svg_subpaths(&svg("<rect x='10' y='10' width='40' height='20'/>"), 2.0).unwrap();
        assert_eq!(subpaths.len(), 1);
        let sp = &subpaths[0];
        assert!(sp.closed);
        assert_eq!(sp.nodes.len(), 4);
        assert!(
            sp.nodes
                .iter()
                .all(|n| n.handle_in == [0.0; 2] && n.handle_out == [0.0; 2])
        );
        let (min, max) = bounds(&subpaths);
        assert!((max[0] - min[0] - 2.0).abs() < EPS);
        assert!((max[1] - min[1] - 1.0).abs() < EPS);
    }

    #[test]
    fn y_axis_is_flipped_to_y_up() {
        // Triângulo com o vértice no topo do SVG (y pequeno) deve ficar com y > 0.
        let source = svg("<path d='M 50 10 L 90 90 L 10 90 Z'/>");
        let sp = &svg_subpaths(&source, 2.0).unwrap()[0];
        let apex = sp
            .nodes
            .iter()
            .max_by(|a, b| a.position[1].total_cmp(&b.position[1]))
            .unwrap();
        assert!(apex.position[1] > 0.0);
        assert!(apex.position[0].abs() < EPS, "vértice no meio em x");
        let base: Vec<_> = sp.nodes.iter().filter(|n| n.position[1] < 0.0).collect();
        assert_eq!(base.len(), 2);
    }

    #[test]
    fn centred_and_fit_to_size() {
        let source = svg("<rect x='20' y='30' width='60' height='30'/>");
        for fit in [1.0, 5.0, 123.5] {
            let subpaths = svg_subpaths(&source, fit).unwrap();
            let (min, max) = bounds(&subpaths);
            assert!(((min[0] + max[0]) * 0.5).abs() < EPS);
            assert!(((min[1] + max[1]) * 0.5).abs() < EPS);
            assert!((max[0] - min[0] - fit).abs() < EPS * fit.max(1.0));
            assert!((max[1] - min[1] - fit * 0.5).abs() < EPS * fit.max(1.0));
        }
    }

    #[test]
    fn cubic_path_keeps_relative_handles() {
        // S simétrico: handles em y para baixo do SVG (y+) viram y negativo.
        let source = svg("<path d='M 0 0 C 0 50 100 50 100 0' fill='none' stroke='black'/>");
        let sp = &svg_subpaths(&source, 100.0).unwrap()[0];
        assert!(!sp.closed);
        assert_eq!(sp.nodes.len(), 2);
        let (a, b) = (sp.nodes[0], sp.nodes[1]);
        // Alças relativas: out do início = (0,50)->(0,-50); in do fim = (0,50)->(0,-50).
        assert!(a.handle_out[0].abs() < EPS);
        assert!(b.handle_in[0].abs() < EPS);
        assert!(a.handle_out[1] < 0.0 && b.handle_in[1] < 0.0);
        assert_eq!(a.handle_in, [0.0; 2]);
        assert_eq!(b.handle_out, [0.0; 2]);
        // A caixa usa o extremo da curva (y máx. real 37.5), não os controles (50).
        let (min, max) = bounds(&[sp.clone()]);
        assert!((max[0] - min[0] - 100.0).abs() < EPS);
        assert!((max[1] - min[1] - 37.5).abs() < 0.01);
    }

    #[test]
    fn quadratic_is_converted_to_equivalent_cubic() {
        let source = svg("<path d='M 0 0 Q 50 100 100 0' fill='none' stroke='black'/>");
        let sp = &svg_subpaths(&source, 100.0).unwrap()[0];
        assert_eq!(sp.nodes.len(), 2);
        // Meio da quadrática original = (50, 50) em SVG; em y-up centrado:
        // bbox x 0..100 (centro 50), y 0..50 (centro 25); y invertido → ponto (0, -25).
        let (a, b) = (sp.nodes[0], sp.nodes[1]);
        let p1 = [
            a.position[0] + a.handle_out[0],
            a.position[1] + a.handle_out[1],
        ];
        let p2 = [
            b.position[0] + b.handle_in[0],
            b.position[1] + b.handle_in[1],
        ];
        let t = 0.5;
        let mid = |i: usize| cubic_at(a.position[i], p1[i], p2[i], b.position[i], t);
        assert!(mid(0).abs() < EPS);
        assert!((mid(1) + 25.0).abs() < EPS);
    }

    #[test]
    fn circle_becomes_closed_cubic_spline() {
        let splines = svg_to_splines(&svg("<circle cx='50' cy='50' r='40'/>"), 8.0).unwrap();
        assert_eq!(splines.len(), 1);
        let s = &splines[0];
        assert_eq!(s.name, "SVG 1");
        assert_eq!(s.interpolation, SplineInterpolation::CubicBezier);
        assert!(s.closed);
        assert!(s.points.len() >= 4);
        // Todos os nós ficam no círculo de raio 4.
        for p in &s.points {
            let r = (p.position[0].powi(2) + p.position[1].powi(2)).sqrt();
            assert!((r - 4.0).abs() < 0.01, "r = {r}");
        }
    }

    #[test]
    fn open_and_closed_subpaths_and_names() {
        let source = svg(
            "<path d='M 0 0 L 10 0 L 10 10'/><path d='M 20 20 L 30 20 L 30 30 Z'/>\
             <path d='M 40 40 L 50 40' />",
        );
        let splines = svg_to_splines(&source, 10.0).unwrap();
        assert_eq!(splines.len(), 3);
        assert_eq!(splines[0].name, "SVG 1");
        assert_eq!(splines[2].name, "SVG 3");
        assert!(!splines[0].closed);
        assert!(splines[1].closed);
        assert!(!splines[2].closed);
        assert!(
            splines
                .iter()
                .all(|s| s.interpolation == SplineInterpolation::Polyline)
        );
    }

    #[test]
    fn closed_with_two_nodes_is_not_closed_spline() {
        let sp = &svg_to_splines(&svg("<path d='M 0 0 L 10 10 Z' stroke='black'/>"), 1.0).unwrap();
        assert_eq!(sp.len(), 1);
        assert!(!sp[0].closed);
        assert!(sp[0].validate_authoring().is_ok());
    }

    #[test]
    fn consecutive_duplicate_nodes_are_removed() {
        let source = svg("<path d='M 0 0 L 0 0 L 10 0 L 10 0 L 10 10 L 10 10 Z'/>");
        let sp = &svg_subpaths(&source, 10.0).unwrap()[0];
        assert_eq!(sp.nodes.len(), 3);
        assert!(sp.closed);
        // Fechamento explícito sobre o ponto inicial também funde.
        let source = svg("<path d='M 0 0 L 10 0 L 10 10 L 0 0 Z'/>");
        let sp = &svg_subpaths(&source, 10.0).unwrap()[0];
        assert_eq!(sp.nodes.len(), 3);
    }

    #[test]
    fn degenerate_subpaths_are_dropped() {
        // Apenas um M, ou M seguido de L no mesmo ponto: sem geometria.
        let source = svg(
            "<path d='M 5 5 M 7 7 L 7 7' stroke='black'/><path d='M 0 0 L 10 0' stroke='black'/>",
        );
        let subpaths = svg_subpaths(&source, 1.0).unwrap();
        assert_eq!(subpaths.len(), 1);
        // Tudo no mesmo ponto: sem extensão.
        let dot = svg("<path d='M 5 5 L 5 5' stroke='black'/>");
        assert!(matches!(
            svg_subpaths(&dot, 1.0),
            Err(SvgError::Empty) | Err(SvgError::ZeroSize)
        ));
    }

    #[test]
    fn hidden_paths_are_skipped() {
        let source = svg(
            "<rect width='10' height='10' visibility='hidden'/><rect x='20' width='10' height='10'/>",
        );
        assert_eq!(svg_subpaths(&source, 1.0).unwrap().len(), 1);
    }

    #[test]
    fn invalid_fit_size_is_zero_size() {
        let source = svg("<rect width='10' height='10'/>");
        for fit in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert_eq!(svg_subpaths(&source, fit), Err(SvgError::ZeroSize), "{fit}");
        }
    }

    #[test]
    fn huge_coordinates_do_not_panic() {
        let source = svg(
            "<path d='M 1e30 1e30 L -1e30 5 L 3e38 3e38 Z' stroke='black'/>\
             <path d='M 0 0 L 1 1' stroke='black'/>",
        );
        // Qualquer resultado é aceitável, desde que tipado e sem panic.
        let _ = svg_subpaths(&source, 1.0);
        let _ = rasterize_svg(&source, 16);
        let _ = svg_to_splines(&source, 1.0);
    }

    #[test]
    fn node_cap_is_enforced() {
        let mut d = String::from("M 0 0");
        for i in 1..=(MAX_SVG_NODES + 10) {
            d.push_str(&format!(" L {} {}", i, i % 7));
        }
        let source = svg(&format!("<path d='{d}' stroke='black' fill='none'/>"));
        assert!(source.len() < MAX_SVG_BYTES);
        assert_eq!(svg_subpaths(&source, 1.0), Err(SvgError::TooLarge));
        assert_eq!(svg_to_splines(&source, 1.0), Err(SvgError::TooLarge));
        // Logo abaixo do teto passa.
        let mut d = String::from("M 0 0");
        for i in 1..(MAX_SVG_NODES - 5) {
            d.push_str(&format!(" L {} {}", i, i % 7));
        }
        let ok = svg(&format!("<path d='{d}' stroke='black' fill='none'/>"));
        assert!(svg_subpaths(&ok, 1.0).is_ok());
    }

    #[test]
    fn round_trip_to_splines_validates() {
        let source = svg("<circle cx='30' cy='30' r='20'/>\
             <path d='M 10 80 C 20 60 40 100 60 80 S 90 60 95 85' fill='none' stroke='black'/>\
             <rect x='60' y='10' width='30' height='20' rx='5'/>\
             <path d='M 5 5 L 15 5 L 15 15 Z'/>");
        let splines = svg_to_splines(&source, 4.0).unwrap();
        assert!(splines.len() >= 4);
        for spline in &splines {
            assert!(spline.validate_authoring().is_ok(), "{}", spline.name);
            assert!(spline.points.iter().all(|p| p.position[2] == 0.0));
            assert!(
                spline
                    .points
                    .iter()
                    .all(|p| p.position.iter().all(|v| v.is_finite()))
            );
        }
        // Os ids são únicos entre os pontos de cada spline.
        for spline in &splines {
            let mut ids: Vec<_> = spline.points.iter().map(|p| p.id).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), spline.points.len());
        }
    }

    /// `<use>` dobrando a cada nível: 2^levels elementos expandidos.
    fn use_bomb(levels: usize) -> String {
        let mut body = String::from("<g id='g0'><rect width='1' height='1'/></g>");
        for level in 1..=levels {
            let prev = level - 1;
            body.push_str(&format!(
                "<g id='g{level}'><use href='#g{prev}'/><use href='#g{prev}'/></g>"
            ));
        }
        body.push_str(&format!("<use href='#g{levels}'/>"));
        svg(&body)
    }

    #[test]
    fn use_bomb_is_rejected_fast_everywhere() {
        let start = std::time::Instant::now();
        let bomb = use_bomb(40);
        assert!(bomb.len() < 10_000);
        assert_eq!(svg_info(&bomb), Err(SvgError::TooLarge));
        assert_eq!(rasterize_svg(&bomb, 64).unwrap_err(), SvgError::TooLarge);
        assert_eq!(svg_subpaths(&bomb, 1.0).unwrap_err(), SvgError::TooLarge);
        assert_eq!(svg_to_splines(&bomb, 1.0).unwrap_err(), SvgError::TooLarge);
        assert!(start.elapsed().as_secs() < 5);
    }

    #[test]
    fn use_bomb_30_levels_with_xlink_href_is_rejected() {
        let bomb = use_bomb(30)
            .replace("href=", "xlink:href=")
            .replace("<svg ", "<svg xmlns:xlink='http://www.w3.org/1999/xlink' ");
        assert_eq!(svg_info(&bomb), Err(SvgError::TooLarge));
    }

    #[test]
    fn use_bomb_with_wide_fanout_is_rejected() {
        // 1 + 1000 usos de um grupo com 1000 usos de um grupo com 1000 retângulos
        let rects = "<rect width='1' height='1'/>".repeat(1000);
        let uses = |id: &str| format!("<use href='#{id}'/>").repeat(1000);
        let body = format!(
            "<g id='a'>{rects}</g><g id='b'>{}</g><g id='c'>{}</g>{}",
            uses("a"),
            uses("b"),
            uses("c")
        );
        assert_eq!(svg_info(&svg(&body)), Err(SvgError::TooLarge));
    }

    #[test]
    fn moderate_use_expansion_still_works() {
        let body = "<defs><g id='dot'><rect width='4' height='4'/></g></defs>\
            <use href='#dot' x='0'/><use href='#dot' x='10'/><use href='#dot' x='20'/>";
        assert!(rasterize_svg(&svg(body), 32).is_ok());
        assert!(!svg_subpaths(&svg(body), 10.0).unwrap().is_empty());
        // 10 níveis dobrando (1024 retângulos) é legítimo
        assert!(svg_info(&use_bomb(10)).is_ok());
    }

    #[test]
    fn deep_group_nesting_is_rejected_but_moderate_is_fine() {
        let nest = |n: usize| {
            svg(&format!(
                "{}<rect width='10' height='10'/>{}",
                "<g opacity='0.9'>".repeat(n),
                "</g>".repeat(n)
            ))
        };
        assert!(rasterize_svg(&nest(20), 16).is_ok());
        assert_eq!(svg_info(&nest(200)), Err(SvgError::TooLarge));
        assert_eq!(
            rasterize_svg(&nest(200), 16).unwrap_err(),
            SvgError::TooLarge
        );
    }

    #[test]
    fn long_linear_use_chain_hits_depth_cap_without_overflowing_stack() {
        let mut body = String::from("<rect id='u0' width='1' height='1'/>");
        for level in 1..=5000 {
            body.push_str(&format!("<use id='u{level}' href='#u{}'/>", level - 1));
        }
        body.push_str("<use href='#u5000'/>");
        assert_eq!(svg_info(&svg(&body)), Err(SvgError::TooLarge));
    }

    #[test]
    fn use_cycles_and_dangling_refs_do_not_hang_the_check() {
        let cycle = "<g id='a'><use href='#b'/></g><g id='b'><use href='#a'/></g><use href='#a'/>";
        // o veredito (ok ou erro de parse) é do usvg; o que importa é terminar
        let _ = svg_info(&svg(cycle));
        let dangling =
            "<use href='#nope'/><use href='http://x/y.svg#z'/><use/><rect width='5' height='5'/>";
        assert!(svg_info(&svg(dangling)).is_ok());
    }

    #[test]
    fn duplicate_ids_use_worst_case() {
        let rects = "<rect width='1' height='1'/>".repeat(1000);
        let big = format!("<g id='x'/><g id='x'>{rects}</g>");
        let uses = "<use href='#x'/>".repeat(300);
        assert_eq!(
            svg_info(&svg(&format!("{big}{uses}"))),
            Err(SvgError::TooLarge)
        );
    }

    #[test]
    fn malformed_xml_is_parse_error_before_expansion() {
        assert!(matches!(
            svg_info("<svg xmlns='http://www.w3.org/2000/svg'><g></svg>"),
            Err(SvgError::Parse(_))
        ));
    }
}
