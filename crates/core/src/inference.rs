//! Motor de inferência de snap em espaço de tela (P3D-040, ADR 007 Onda 3).
//!
//! Uma única passada por gesto: a tolerância é medida em pixels lógicos da
//! viewport (lei de Fitts), nunca em unidades de mundo, então o alvo tem o
//! mesmo tamanho em qualquer zoom. Classes de alvo seguem prioridade fixa
//! (ponto > aresta > guia de eixo > face > grade); dentro da classe vence o
//! mais próximo do cursor ("bubble cursor"), e a profundidade desempata.
//!
//! Coordenadas de cursor e viewport seguem [`crate::transform_projection`]:
//! origem no canto superior esquerdo, `y` para baixo.

use glam::{Vec2, Vec3};
use petunia_config::TextId;
use petunia_mesh::Mesh;

use crate::bvh::TriangleBvh;
use crate::picking;
use crate::{Camera, SnapTarget};

/// Raio padrão do snap (px lógicos). Ajustável por preferência.
pub const DEFAULT_SNAP_RADIUS_PIXELS: f32 = 12.0;
/// Limites do raio configurável: abaixo de 4 px o alvo é inalcançável com
/// tremor; acima de 48 px o snap "rouba" o cursor.
pub const SNAP_RADIUS_RANGE: (f32, f32) = (4.0, 48.0);

/// O que encaixou. Cada tipo tem forma, cor **e** rótulo na UI (nunca só cor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SnapKind {
    /// Ponto existente da malha (vocabulário de usuário: Point).
    Point,
    /// Ponto médio de uma aresta.
    Midpoint,
    /// Ponto mais próximo sobre uma aresta.
    OnEdge,
    /// Guia a partir da âncora, paralela ao eixo `i` do referencial.
    Axis(usize),
    /// Ponto projetado sobre a face sob o cursor (não o centroide).
    OnFace,
    /// Centro (centroide) de uma face.
    FaceCenter,
    /// Cruzamento real de duas arestas.
    Intersection,
    /// Guia a partir da âncora, paralela à direção de referência.
    Parallel,
    /// Guia a partir da âncora, perpendicular à direção de referência.
    Perpendicular,
    /// Guia a partir da âncora num múltiplo do passo angular (graus).
    Angle(u16),
    /// Cruzamento da grade do plano de trabalho.
    Grid,
}

impl SnapKind {
    /// Rótulo exibido no HUD ao lado do marcador.
    pub fn text_id(self) -> TextId {
        use petunia_config::text_id as t;
        match self {
            Self::Point => t::SNAP_KIND_POINT,
            Self::Midpoint => t::SNAP_KIND_MIDPOINT,
            Self::OnEdge => t::SNAP_KIND_ON_EDGE,
            Self::Axis(0) => t::SNAP_KIND_AXIS_X,
            Self::Axis(1) => t::SNAP_KIND_AXIS_Y,
            Self::Axis(_) => t::SNAP_KIND_AXIS_Z,
            Self::OnFace => t::SNAP_KIND_ON_FACE,
            Self::FaceCenter => t::SNAP_KIND_FACE_CENTER,
            Self::Intersection => t::SNAP_KIND_INTERSECTION,
            Self::Parallel => t::SNAP_KIND_PARALLEL,
            Self::Perpendicular => t::SNAP_KIND_PERPENDICULAR,
            Self::Angle(_) => t::SNAP_KIND_ANGLE,
            Self::Grid => t::SNAP_KIND_GRID,
        }
    }

    /// Classe de prioridade: menor vence, independentemente da distância.
    fn class(self) -> u8 {
        match self {
            Self::Point | Self::Midpoint | Self::FaceCenter | Self::Intersection => 0,
            Self::OnEdge => 1,
            Self::Axis(_) | Self::Parallel | Self::Perpendicular => 2,
            Self::Angle(_) => 3,
            Self::OnFace => 4,
            Self::Grid => 5,
        }
    }

    /// Guias e grade são virtuais: não sofrem oclusão pela malha.
    fn occludable(self) -> bool {
        !matches!(
            self,
            Self::Axis(_) | Self::Parallel | Self::Perpendicular | Self::Angle(_) | Self::Grid
        )
    }
}

/// Tipos de alvo considerados numa consulta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapMask {
    pub points: bool,
    pub edges: bool,
    pub axes: bool,
    pub faces: bool,
    pub grid: bool,
}

impl SnapMask {
    pub const ALL: Self = Self {
        points: true,
        edges: true,
        axes: true,
        faces: true,
        grid: true,
    };

    /// Tradução da escolha de alvo do usuário.
    ///
    /// `Grid` (padrão) é o modo automático: pontos, arestas e guias de eixo,
    /// com a grade como último recurso. `Increment` só infere eixos; o passo
    /// relativo é aplicado por quem chama. Os demais restringem ao tipo.
    pub fn for_target(target: SnapTarget) -> Self {
        let none = Self {
            points: false,
            edges: false,
            axes: false,
            faces: false,
            grid: false,
        };
        match target {
            SnapTarget::Grid => Self {
                faces: false,
                ..Self::ALL
            },
            SnapTarget::Increment => Self { axes: true, ..none },
            SnapTarget::Vertex => Self {
                points: true,
                ..none
            },
            SnapTarget::Edge => Self {
                points: true,
                edges: true,
                ..none
            },
            SnapTarget::Face => Self {
                points: true,
                edges: true,
                faces: true,
                ..none
            },
        }
    }
}

/// Grade de um plano (mundo ou plano de trabalho).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapGrid {
    pub origin: Vec3,
    pub right: Vec3,
    pub up: Vec3,
    pub spacing: f32,
}

/// Ponto de partida das guias de inferência e seus eixos (mundo ou plano).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapAnchor {
    pub point: Vec3,
    pub axes: [Option<Vec3>; 3],
}

impl SnapAnchor {
    /// Âncora com os eixos X, Y e Z do mundo.
    pub fn world(point: Vec3) -> Self {
        Self {
            point,
            axes: [Some(Vec3::X), Some(Vec3::Y), Some(Vec3::Z)],
        }
    }
}

/// Consulta de snap em espaço de tela.
#[derive(Clone, Copy)]
pub struct ScreenSnapQuery<'a> {
    pub camera: &'a Camera,
    /// Tamanho da viewport em px lógicos.
    pub viewport_pixels: Vec2,
    /// Cursor em px lógicos (origem no canto superior esquerdo).
    pub cursor_pixels: Vec2,
    pub mesh: Option<&'a Mesh>,
    /// Vértices que se movem com o gesto (índice → `true`); não são alvos e
    /// suas faces não ocultam nem recebem snap. Vazio = nenhum.
    pub moving: &'a [bool],
    pub anchor: Option<SnapAnchor>,
    pub grid: Option<SnapGrid>,
    pub radius_pixels: f32,
    pub mask: SnapMask,
    /// Raio-X/wireframe: pontos e arestas atrás da malha também são alvos.
    pub xray: bool,
    /// Direção de referência (ex.: o último segmento desenhado) para as guias
    /// paralela e perpendicular a partir da âncora.
    pub reference: Option<Vec3>,
    /// Passo das guias angulares no plano dos dois primeiros eixos da âncora
    /// (ex.: 15°). `None` desliga.
    pub angle_step_degrees: Option<f32>,
}

/// Uma malha participante do snap e os vértices que se movem com o gesto.
#[derive(Clone, Copy)]
pub struct SnapSource<'a> {
    pub mesh: &'a Mesh,
    /// Índice → `true` para vértices em movimento (vazio = nenhum).
    pub moving: &'a [bool],
}

/// Geometria parada de uma ou mais malhas, pronta para várias consultas.
///
/// Construída uma vez por gesto (ou por revisão de geometria): a passada de
/// snap deixa de triangular a malha e montar `edges_unique()` a cada evento,
/// e a oclusão de cada candidato custa O(log triângulos) pela BVH.
#[derive(Debug, Clone, Default)]
pub struct SnapAccel {
    points: Vec<Vec3>,
    edges: Vec<(Vec3, Vec3)>,
    face_centers: Vec<Vec3>,
    occluder: TriangleBvh,
}

impl SnapAccel {
    pub fn build(sources: &[SnapSource<'_>]) -> Self {
        let mut accel = Self::default();
        let mut solid = Vec::new();
        for source in sources {
            let mesh = source.mesh;
            let moving = |index: usize| source.moving.get(index).copied().unwrap_or(false);
            for (index, vertex) in mesh.verts.iter().enumerate() {
                if !moving(index) {
                    accel.points.push(vertex.vec());
                }
            }
            for (a, b) in mesh.edges_unique() {
                let (ia, ib) = (a as usize, b as usize);
                if moving(ia) || moving(ib) {
                    continue;
                }
                if let (Some(va), Some(vb)) = (mesh.verts.get(ia), mesh.verts.get(ib)) {
                    accel.edges.push((va.vec(), vb.vec()));
                }
            }
            for (face_index, face) in mesh.faces.iter().enumerate() {
                if face.verts.len() < 3
                    || face.verts.iter().any(|&v| moving(v as usize))
                    || face
                        .verts
                        .iter()
                        .any(|&v| mesh.verts.get(v as usize).is_none())
                {
                    continue;
                }
                let sum: Vec3 = face
                    .verts
                    .iter()
                    .map(|&v| mesh.verts[v as usize].vec())
                    .sum();
                accel.face_centers.push(sum / face.verts.len() as f32);
                for corners in mesh.face_triangle_corners(face_index) {
                    solid.push((
                        corners.map(|corner| mesh.verts[face.verts[corner] as usize].vec()),
                        face_index as u32,
                    ));
                }
            }
        }
        accel.occluder = TriangleBvh::build(solid);
        accel
    }
}

/// Resultado: o ponto encaixado, o tipo e a distância ao cursor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenSnapHit {
    pub point: Vec3,
    pub kind: SnapKind,
    pub pixel_distance: f32,
}

struct Candidate {
    hit: ScreenSnapHit,
    depth: f32,
}

/// Raio saneado: valores inválidos voltam ao padrão; o resto é limitado à faixa.
pub fn clamp_snap_radius(radius: f32) -> f32 {
    if radius.is_finite() {
        radius.clamp(SNAP_RADIUS_RANGE.0, SNAP_RADIUS_RANGE.1)
    } else {
        DEFAULT_SNAP_RADIUS_PIXELS
    }
}

/// Executa a consulta. `None` = nada encaixou (o chamador usa o ponto livre).
///
/// Conveniência para uma consulta isolada sobre `query.mesh`; quem consulta a
/// cada evento deve guardar um [`SnapAccel`] e usar [`snap_screen_with`].
pub fn snap_screen(query: &ScreenSnapQuery) -> Option<ScreenSnapHit> {
    let accel = query.mesh.map_or_else(SnapAccel::default, |mesh| {
        SnapAccel::build(&[SnapSource {
            mesh,
            moving: query.moving,
        }])
    });
    snap_screen_with(query, &accel)
}

/// Candidatos de aresta próximos ao cursor considerados para interseções.
const INTERSECTION_EDGE_BUDGET: usize = 32;

/// Executa a consulta sobre uma geometria já preparada (`query.mesh` e
/// `query.moving` são ignorados: a geometria vem de `accel`).
pub fn snap_screen_with(query: &ScreenSnapQuery, accel: &SnapAccel) -> Option<ScreenSnapHit> {
    let viewport = query.viewport_pixels;
    if !viewport.is_finite() || viewport.min_element() <= 1.0 || !query.cursor_pixels.is_finite() {
        return None;
    }
    let matrix = query.camera.view_proj();
    let inverse = matrix.inverse();
    if !inverse.is_finite() {
        return None;
    }
    let radius = clamp_snap_radius(query.radius_pixels);
    let cursor = query.cursor_pixels;
    let cursor_ndc = Vec2::new(
        cursor.x / viewport.x * 2.0 - 1.0,
        1.0 - cursor.y / viewport.y * 2.0,
    );
    let to_pixels = |ndc: Vec2| Vec2::new(ndc.x + 1.0, 1.0 - ndc.y) * viewport * 0.5;
    let solid = &accel.occluder;

    let mut best: Option<Candidate> = None;
    // `probe` mede a distância ao cursor; `position` é o ponto entregue (a
    // guia quantizada pela grade é medida pelo ponto antes do passo).
    let mut consider = |kind: SnapKind, probe: Vec3, position: Vec3, limit: f32| {
        let Some(ndc) = picking::project(matrix, probe) else {
            return;
        };
        let pixel_distance = (to_pixels(ndc.truncate()) - cursor).length();
        if pixel_distance > limit {
            return;
        }
        let better = best.as_ref().is_none_or(|current| {
            let (a, b) = (kind.class(), current.hit.kind.class());
            a < b
                || (a == b
                    && (pixel_distance < current.hit.pixel_distance - 0.01
                        || ((pixel_distance - current.hit.pixel_distance).abs() <= 0.01
                            && ndc.z < current.depth)))
        });
        // A oclusão (o teste caro) só roda para quem venceria.
        if !better
            || (kind.occludable()
                && !query.xray
                && picking::occluded(solid, inverse, ndc.truncate(), position))
        {
            return;
        }
        best = Some(Candidate {
            hit: ScreenSnapHit {
                point: position,
                kind,
                pixel_distance,
            },
            depth: ndc.z,
        });
    };

    if query.mask.points {
        for &point in &accel.points {
            consider(SnapKind::Point, point, point, radius);
        }
        for &center in &accel.face_centers {
            consider(SnapKind::FaceCenter, center, center, radius);
        }
    }
    let mut near_edges: Vec<(Vec3, Vec3)> = Vec::new();
    if query.mask.points || query.mask.edges {
        let pixel_scale = viewport * 0.5;
        for &(pa, pb) in &accel.edges {
            if query.mask.points {
                consider(SnapKind::Midpoint, (pa + pb) * 0.5, (pa + pb) * 0.5, radius);
            }
            if !query.mask.edges {
                continue;
            }
            let Some((ca, cb)) = picking::clip_depth(matrix, pa, pb) else {
                continue;
            };
            let (Some(sa), Some(sb)) = (picking::project(matrix, ca), picking::project(matrix, cb))
            else {
                continue;
            };
            let start = sa.truncate() * pixel_scale;
            let segment = (sb.truncate() - sa.truncate()) * pixel_scale;
            let fraction = if segment.length_squared() > 1e-8 {
                ((cursor_ndc * pixel_scale - start).dot(segment) / segment.length_squared())
                    .clamp(0.0, 1.0)
            } else {
                0.0
            };
            // Interpolação com correção de perspectiva, como no picking.
            let wa = (matrix * ca.extend(1.0)).w;
            let wb = (matrix * cb.extend(1.0)).w;
            let fraction = fraction * wa / ((1.0 - fraction) * wb + fraction * wa);
            let on_edge = ca.lerp(cb, fraction);
            if let Some(ndc) = picking::project(matrix, on_edge)
                && (to_pixels(ndc.truncate()) - cursor).length() <= radius
                && near_edges.len() < INTERSECTION_EDGE_BUDGET
            {
                near_edges.push((pa, pb));
            }
            consider(SnapKind::OnEdge, on_edge, on_edge, radius);
        }
    }
    if query.mask.edges {
        for (i, &(a0, a1)) in near_edges.iter().enumerate() {
            for &(b0, b1) in &near_edges[i + 1..] {
                if let Some(point) = segment_intersection(a0, a1, b0, b1) {
                    consider(SnapKind::Intersection, point, point, radius);
                }
            }
        }
    }

    let cursor_ray = picking::ray(inverse, cursor_ndc);

    if query.mask.axes
        && let (Some(anchor), Some((origin, direction))) = (query.anchor, cursor_ray)
        && let Some(anchor_ndc) = picking::project(matrix, anchor.point)
    {
        let anchor_pixels = to_pixels(anchor_ndc.truncate());
        // Perto da própria âncora todas as guias coincidem: ambíguo.
        let near_anchor = |point: Vec3| {
            picking::project(matrix, point)
                .is_none_or(|ndc| (to_pixels(ndc.truncate()) - anchor_pixels).length() < radius)
        };
        let quantize = |point: Vec3, axis: Vec3| {
            if query.mask.grid
                && let Some(grid) = query.grid
                && grid.spacing.is_finite()
            {
                let spacing = grid.spacing.max(1.0e-4);
                let along = ((point - anchor.point).dot(axis) / spacing).round() * spacing;
                anchor.point + axis * along
            } else {
                point
            }
        };
        let mut guides: Vec<(SnapKind, Vec3)> = Vec::new();
        for (index, axis) in anchor.axes.iter().enumerate() {
            if let Some(axis) = axis.map(Vec3::normalize_or_zero)
                && axis != Vec3::ZERO
            {
                guides.push((SnapKind::Axis(index), axis));
            }
        }
        let plane_normal = match (anchor.axes[0], anchor.axes[1]) {
            (Some(a), Some(b)) => Some(a.cross(b).normalize_or_zero()).filter(|n| *n != Vec3::ZERO),
            _ => None,
        };
        if let Some(reference) = query.reference.map(Vec3::normalize_or_zero)
            && reference != Vec3::ZERO
        {
            guides.push((SnapKind::Parallel, reference));
            if let Some(normal) = plane_normal {
                let perpendicular = normal.cross(reference).normalize_or_zero();
                if perpendicular != Vec3::ZERO {
                    guides.push((SnapKind::Perpendicular, perpendicular));
                }
            }
        }
        if let (Some(step), Some(normal), Some(u)) =
            (query.angle_step_degrees, plane_normal, anchor.axes[0])
            && step.is_finite()
            && step >= 1.0
        {
            let u = u.normalize_or_zero();
            let v = normal.cross(u).normalize_or_zero();
            let denominator = direction.dot(normal);
            if u != Vec3::ZERO && v != Vec3::ZERO && denominator.abs() > 1.0e-6 {
                let t = (anchor.point - origin).dot(normal) / denominator;
                let offset = origin + direction * t - anchor.point;
                if offset.length_squared() > 1.0e-10 {
                    let angle = offset.dot(v).atan2(offset.dot(u)).to_degrees();
                    let snapped = (angle / step).round() * step;
                    let degrees = snapped.rem_euclid(360.0);
                    // Múltiplos de 90° já são os eixos: não duplicar a guia.
                    if (degrees % 90.0).abs() > 1.0e-3 {
                        let radians = snapped.to_radians();
                        let guide = u * radians.cos() + v * radians.sin();
                        guides.push((SnapKind::Angle(degrees.round() as u16), guide));
                    }
                }
            }
        }
        for (kind, axis) in guides {
            let Some(closest) = closest_on_line(anchor.point, axis, origin, direction) else {
                continue;
            };
            if near_anchor(closest) {
                continue;
            }
            consider(kind, closest, quantize(closest, axis), radius);
        }
    }

    if query.mask.faces
        && let Some((origin, direction)) = cursor_ray
        && let Some((distance, _)) = solid.nearest(origin, direction, f32::INFINITY, |_| true)
    {
        let on_face = origin + direction * distance;
        consider(SnapKind::OnFace, on_face, on_face, f32::INFINITY);
    }

    if query.mask.grid
        && let (Some(grid), Some((origin, direction))) = (query.grid, cursor_ray)
        && let Some(point) = grid_point(grid, origin, direction)
    {
        consider(SnapKind::Grid, point, point, f32::INFINITY);
    }

    best.map(|candidate| candidate.hit)
}

/// Ponto de cruzamento real de dois segmentos (dentro de uma tolerância
/// relativa ao tamanho deles); extremidades compartilhadas não contam, pois
/// já são pontos da malha.
fn segment_intersection(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> Option<Vec3> {
    let (da, db) = (a1 - a0, b1 - b0);
    let r = a0 - b0;
    let (aa, bb, ab) = (da.dot(da), db.dot(db), da.dot(db));
    let denominator = aa * bb - ab * ab;
    if aa <= 1.0e-12 || bb <= 1.0e-12 || denominator.abs() <= 1.0e-12 * aa * bb {
        return None;
    }
    let (ar, br) = (da.dot(r), db.dot(r));
    let s = (ab * br - bb * ar) / denominator;
    let t = (aa * br - ab * ar) / denominator;
    let interior = 1.0e-4;
    if !(interior..=1.0 - interior).contains(&s) || !(interior..=1.0 - interior).contains(&t) {
        return None;
    }
    let (pa, pb) = (a0 + da * s, b0 + db * t);
    let tolerance = 1.0e-4 * aa.sqrt().max(bb.sqrt()).max(1.0);
    (pa.distance(pb) <= tolerance).then(|| (pa + pb) * 0.5)
}

/// Ponto da reta `anchor + t·axis` mais próximo do raio do cursor.
/// `None` quando a reta é quase paralela ao raio (aponta para o observador).
fn closest_on_line(anchor: Vec3, axis: Vec3, origin: Vec3, direction: Vec3) -> Option<Vec3> {
    let b = axis.dot(direction);
    let denominator = 1.0 - b * b;
    if denominator < 1.0e-4 {
        return None;
    }
    let w = anchor - origin;
    let t = (b * direction.dot(w) - axis.dot(w)) / denominator;
    let point = anchor + axis * t;
    point.is_finite().then_some(point)
}

/// Cruzamento de grade mais próximo do ponto em que o raio cruza o plano.
fn grid_point(grid: SnapGrid, origin: Vec3, direction: Vec3) -> Option<Vec3> {
    let normal = grid.right.cross(grid.up).normalize_or_zero();
    let denominator = direction.dot(normal);
    if normal == Vec3::ZERO || denominator.abs() < 1.0e-6 || !grid.spacing.is_finite() {
        return None;
    }
    let spacing = grid.spacing.max(1.0e-4);
    let distance = (grid.origin - origin).dot(normal) / denominator;
    let offset = origin + direction * distance - grid.origin;
    let u = (offset.dot(grid.right) / spacing).round() * spacing;
    let v = (offset.dot(grid.up) / spacing).round() * spacing;
    let point = grid.origin + grid.right * u + grid.up * v;
    point.is_finite().then_some(point)
}

/// Projeção usada pelos testes para mirar o cursor sem depender da UI.
#[cfg(test)]
fn project_pixels(camera: &Camera, viewport: Vec2, point: Vec3) -> Vec2 {
    let matrix = camera.view_proj();
    let ndc = picking::project(matrix, point).unwrap().truncate();
    Vec2::new(ndc.x + 1.0, 1.0 - ndc.y) * viewport * 0.5
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ViewPreset;
    use petunia_mesh::{Face, Vertex};

    const VIEWPORT: Vec2 = Vec2::new(800.0, 800.0);

    fn camera(half_height: f32) -> Camera {
        let mut camera = Camera::default();
        camera.set_preset(ViewPreset::Front);
        camera.aspect = 1.0;
        camera.ortho_half_h = half_height;
        camera
    }

    /// Quadrado 2×2 no plano XY (z = 0), de frente para a câmera Front.
    fn square() -> Mesh {
        let mut mesh = Mesh::default();
        for [x, y] in [[-1.0f32, -1.0], [1.0, -1.0], [1.0, 1.0], [-1.0, 1.0]] {
            mesh.verts.push(Vertex::new(x, y, 0.0));
        }
        mesh.faces.push(Face::new(vec![0, 1, 2, 3]));
        mesh
    }

    fn query<'a>(camera: &'a Camera, mesh: Option<&'a Mesh>, cursor: Vec2) -> ScreenSnapQuery<'a> {
        ScreenSnapQuery {
            camera,
            viewport_pixels: VIEWPORT,
            cursor_pixels: cursor,
            mesh,
            moving: &[],
            anchor: None,
            grid: None,
            radius_pixels: DEFAULT_SNAP_RADIUS_PIXELS,
            mask: SnapMask::ALL,
            xray: false,
            reference: None,
            angle_step_degrees: None,
        }
    }

    #[test]
    fn tolerance_is_measured_in_pixels_at_any_zoom() {
        let mesh = square();
        for half_height in [2.0, 50.0] {
            let camera = camera(half_height);
            let corner = project_pixels(&camera, VIEWPORT, Vec3::new(1.0, 1.0, 0.0));
            let near = snap_screen(&query(&camera, Some(&mesh), corner + Vec2::new(8.0, 0.0)));
            assert_eq!(
                near.map(|hit| hit.kind),
                Some(SnapKind::Point),
                "zoom {half_height}"
            );
            assert!((near.unwrap().point - Vec3::new(1.0, 1.0, 0.0)).length() < 1e-4);
            let mut far = query(&camera, Some(&mesh), corner + Vec2::new(30.0, 30.0));
            far.mask.faces = false;
            assert!(snap_screen(&far).is_none(), "zoom {half_height}");
        }
    }

    #[test]
    fn points_beat_edges_and_edges_beat_faces() {
        let camera = camera(2.0);
        let mesh = square();
        let midpoint = project_pixels(&camera, VIEWPORT, Vec3::new(0.0, 1.0, 0.0));
        let hit = snap_screen(&query(&camera, Some(&mesh), midpoint + Vec2::new(4.0, 2.0)));
        assert_eq!(hit.unwrap().kind, SnapKind::Midpoint);

        let on_edge = project_pixels(&camera, VIEWPORT, Vec3::new(0.5, 1.0, 0.0));
        let hit = snap_screen(&query(&camera, Some(&mesh), on_edge + Vec2::new(0.0, 3.0))).unwrap();
        assert_eq!(hit.kind, SnapKind::OnEdge);
        assert!((hit.point - Vec3::new(0.5, 1.0, 0.0)).length() < 1e-3);

        let inside = project_pixels(&camera, VIEWPORT, Vec3::new(0.3, -0.4, 0.0));
        let hit = snap_screen(&query(&camera, Some(&mesh), inside)).unwrap();
        assert_eq!(hit.kind, SnapKind::OnFace);
        // Ponto projetado sob o cursor, não o centroide da face.
        assert!((hit.point - Vec3::new(0.3, -0.4, 0.0)).length() < 1e-3);
    }

    #[test]
    fn moving_vertices_are_never_targets() {
        let camera = camera(2.0);
        let mesh = square();
        let corner = project_pixels(&camera, VIEWPORT, Vec3::new(1.0, 1.0, 0.0));
        let moving = [false, false, true, false];
        let mut q = query(&camera, Some(&mesh), corner);
        q.moving = &moving;
        let hit = snap_screen(&q);
        assert!(hit.is_none_or(|hit| hit.kind != SnapKind::Point
            || (hit.point - Vec3::new(1.0, 1.0, 0.0)).length() > 1e-3));
        assert!(hit.is_none_or(|hit| hit.kind != SnapKind::OnFace));
    }

    #[test]
    fn axis_guide_infers_direction_from_anchor() {
        let camera = camera(2.0);
        let anchor = Vec3::new(-1.5, 0.0, 0.0);
        let target = project_pixels(&camera, VIEWPORT, Vec3::new(0.7, 0.0, 0.0));
        let mut q = query(&camera, None, target + Vec2::new(0.0, 5.0));
        q.anchor = Some(SnapAnchor::world(anchor));
        let hit = snap_screen(&q).unwrap();
        assert_eq!(hit.kind, SnapKind::Axis(0));
        assert!((hit.point - Vec3::new(0.7, 0.0, 0.0)).length() < 1e-3);
        assert!(hit.point.y.abs() < 1e-5);
    }

    #[test]
    fn axis_guide_steps_by_grid_spacing() {
        let camera = camera(2.0);
        let anchor = Vec3::new(-1.5, 0.0, 0.0);
        // 2,05 a partir da âncora → 8 passos de 0,25.
        let target = project_pixels(&camera, VIEWPORT, Vec3::new(0.55, 0.0, 0.0));
        let mut q = query(&camera, None, target + Vec2::new(0.0, 4.0));
        q.anchor = Some(SnapAnchor::world(anchor));
        q.grid = Some(SnapGrid {
            origin: Vec3::ZERO,
            right: Vec3::X,
            up: Vec3::Y,
            spacing: 0.25,
        });
        let hit = snap_screen(&q).unwrap();
        assert_eq!(hit.kind, SnapKind::Axis(0));
        // -1.5 + 8 × 0.25 = 0.5: comprimento exato a partir da âncora.
        assert!(
            (hit.point - Vec3::new(0.5, 0.0, 0.0)).length() < 1e-4,
            "{:?}",
            hit.point
        );
    }

    #[test]
    fn grid_is_the_last_resort() {
        let camera = camera(2.0);
        let cursor = project_pixels(&camera, VIEWPORT, Vec3::new(0.62, 0.38, 0.0));
        let mut q = query(&camera, None, cursor);
        q.grid = Some(SnapGrid {
            origin: Vec3::ZERO,
            right: Vec3::X,
            up: Vec3::Y,
            spacing: 0.25,
        });
        let hit = snap_screen(&q).unwrap();
        assert_eq!(hit.kind, SnapKind::Grid);
        assert!((hit.point - Vec3::new(0.5, 0.5, 0.0)).length() < 1e-4);

        let mesh = square();
        q.mesh = Some(&mesh);
        q.cursor_pixels = project_pixels(&camera, VIEWPORT, Vec3::new(1.0, 1.0, 0.0));
        assert_eq!(snap_screen(&q).unwrap().kind, SnapKind::Point);
    }

    #[test]
    fn hidden_points_need_xray() {
        let mut camera = camera(2.0);
        camera.set_preset(ViewPreset::Front);
        // Quadrado na frente (z = 1) cobre um ponto solto atrás (z = -1).
        let mut mesh = square();
        for v in &mut mesh.verts {
            v.pos[2] = 1.0;
        }
        let hidden = mesh.verts.len();
        mesh.verts.push(Vertex::new(0.2, 0.2, -1.0));
        let cursor = project_pixels(&camera, VIEWPORT, Vec3::new(0.2, 0.2, -1.0));
        let mut q = query(&camera, Some(&mesh), cursor);
        q.mask.faces = false;
        assert!(snap_screen(&q).is_none(), "oculto pela face da frente");
        q.xray = true;
        let hit = snap_screen(&q).unwrap();
        assert_eq!(hit.kind, SnapKind::Point);
        assert!((hit.point - mesh.verts[hidden].vec()).length() < 1e-4);
    }

    #[test]
    fn invalid_input_is_ignored() {
        let camera = camera(2.0);
        let mesh = square();
        let mut q = query(&camera, Some(&mesh), Vec2::new(f32::NAN, 0.0));
        assert!(snap_screen(&q).is_none());
        q.cursor_pixels = Vec2::ZERO;
        q.viewport_pixels = Vec2::ZERO;
        assert!(snap_screen(&q).is_none());
        assert_eq!(clamp_snap_radius(f32::NAN), DEFAULT_SNAP_RADIUS_PIXELS);
        assert_eq!(clamp_snap_radius(1000.0), SNAP_RADIUS_RANGE.1);
    }
}
