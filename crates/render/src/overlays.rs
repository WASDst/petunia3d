//! Tipos neutros de overlay entre Application/UI e renderer (sem dependência de
//! GPU) e a tesselação de CPU dos overlays simples (gizmo de tela e workplane).
//!
//! Moram aqui para que nenhum backend gráfico seja requisito de quem os produz e
//! para a tesselação ser testável sem contexto GL.

use crate::frame::{CameraMatrices, OverlayFaceVertex, OverlayLine};

/// Aparência das arestas no viewport (capítulo 05, aparência por workspace).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum EdgeMode {
    /// Faces limpas; arestas só com o overlay de wireframe (PAINT, UV).
    #[default]
    Overlay,
    /// DRAW: leitura de forma — só arestas de feição (bordas e dobras).
    Features,
    /// POLY: leitura de topologia — todas as arestas finas, as de feição
    /// reforçadas.
    Topology,
}

/// Visibilidade de uma aresta no overlay (cap. 05, "Overlays, não novos
/// modos" + Princípio de UX).
///
/// O toggle do overlay (`show_wireframe_overlay`) é o mestre das arestas
/// finas **em qualquer modo**: com ele desligado, o modo contribui só com a
/// leitura de forma/topologia (arestas de feição em DRAW/POLY, nada em
/// `Overlay`). Forçar todas as arestas no modo POLY tornava o toggle um
/// no-op justamente no workspace padrão — o modo continua legível (feições
/// reforçadas) e o usuário recupera o controle.
pub fn edge_overlay_visible(
    show_wireframe_overlay: bool,
    mode: EdgeMode,
    is_feature: bool,
) -> bool {
    if is_feature {
        show_wireframe_overlay || mode != EdgeMode::Overlay
    } else {
        show_wireframe_overlay
    }
}

/// Plano de trabalho do DRAW em destaque (capítulo 05): origem e eixos do
/// frame do perfil, em coordenadas de mundo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkplaneOverlay {
    pub origin: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
}

/// Forma do overlay de tela em px lógicos: polilinha (traço de largura
/// constante) e/ou polígono convexo preenchido.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlayShape {
    pub points: Vec<[f32; 2]>,
    /// Fecha o traço no primeiro ponto.
    pub closed: bool,
    /// Preenchimento (polígono convexo, leque a partir do primeiro ponto).
    pub fill: Option<[f32; 4]>,
    /// Traço: cor e largura em px lógicos.
    pub stroke: Option<([f32; 4], f32)>,
}

/// Vértice do overlay de tela em px físicos (origem no topo-esquerdo).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

/// Triângulos das formas do overlay em px físicos. Traços viram faixas com
/// pontas estendidas em meia largura (juntas sem frestas).
pub fn tessellate_screen_overlay(shapes: &[OverlayShape], pixel_ratio: f32) -> Vec<ScreenVertex> {
    let mut out = Vec::new();
    for shape in shapes {
        let points: Vec<[f32; 2]> = shape
            .points
            .iter()
            .filter(|p| p[0].is_finite() && p[1].is_finite())
            .map(|p| [p[0] * pixel_ratio, p[1] * pixel_ratio])
            .collect();
        if let Some(color) = shape.fill
            && points.len() >= 3
        {
            for i in 1..points.len() - 1 {
                for position in [points[0], points[i], points[i + 1]] {
                    out.push(ScreenVertex { position, color });
                }
            }
        }
        if let Some((color, width)) = shape.stroke
            && points.len() >= 2
        {
            let half = (width * pixel_ratio * 0.5).max(0.5);
            let count = if shape.closed {
                points.len()
            } else {
                points.len() - 1
            };
            for i in 0..count {
                let (a, b) = (points[i], points[(i + 1) % points.len()]);
                let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
                let length = (dx * dx + dy * dy).sqrt();
                if length < 1e-6 {
                    continue;
                }
                let along = [dx / length * half, dy / length * half];
                let a = [a[0] - along[0], a[1] - along[1]];
                let b = [b[0] + along[0], b[1] + along[1]];
                let side = [-along[1], along[0]];
                let corner = |p: [f32; 2], s: f32| [p[0] + side[0] * s, p[1] + side[1] * s];
                for position in [
                    corner(a, 1.0),
                    corner(a, -1.0),
                    corner(b, 1.0),
                    corner(b, 1.0),
                    corner(a, -1.0),
                    corner(b, -1.0),
                ] {
                    out.push(ScreenVertex { position, color });
                }
            }
        }
    }
    out
}

/// Fração da altura visível coberta pelo recorte do plano (meia largura).
const WORKPLANE_HALF_EXTENT: f32 = 0.3;
/// Divisões da grade do recorte em cada metade.
const WORKPLANE_HALF_CELLS: i32 = 4;
const WORKPLANE_FILL: [f32; 4] = [0.36, 0.62, 0.95, 0.10];
const WORKPLANE_GRID: [f32; 4] = [0.46, 0.70, 0.98, 0.35];
const WORKPLANE_AXIS: [f32; 4] = [0.56, 0.78, 1.0, 0.75];

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn scale(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn normalized(a: [f32; 3]) -> Option<[f32; 3]> {
    let length = dot(a, a).sqrt();
    (length > 1e-6).then(|| scale(a, 1.0 / length))
}

/// Escala da perspectiva no ponto: 1 no plano focal, proporcional à profundidade.
pub fn perspective_scale(camera: &CameraMatrices, point: [f32; 3]) -> f32 {
    if camera.orthographic {
        1.0
    } else {
        (dot(sub(point, camera.eye), camera.forward) / camera.distance.max(0.01)).max(0.01)
    }
}

/// Recorte translúcido do plano de trabalho com grade, centrado na origem. O
/// tamanho acompanha a altura visível, então lê igual em qualquer zoom; o
/// recorte é empurrado um pouco para a câmera para não brigar com a face.
pub fn workplane_geometry(
    plane: WorkplaneOverlay,
    camera: &CameraMatrices,
) -> (Vec<OverlayFaceVertex>, Vec<OverlayLine>) {
    let (Some(right), Some(up)) = (normalized(plane.right), normalized(plane.up)) else {
        return (Vec::new(), Vec::new());
    };
    let Some(mut normal) = normalized(cross(right, up)) else {
        return (Vec::new(), Vec::new());
    };
    let visible = camera.visible_height * perspective_scale(camera, plane.origin);
    if dot(normal, sub(camera.eye, plane.origin)) < 0.0 {
        normal = scale(normal, -1.0);
    }
    let center = add(plane.origin, scale(normal, visible * 0.002));
    let half = visible * WORKPLANE_HALF_EXTENT;
    let corner = |u: f32, v: f32| add(add(center, scale(right, u)), scale(up, v));
    let quad = [
        corner(-half, -half),
        corner(half, -half),
        corner(half, half),
        corner(-half, half),
    ];
    let faces = [0usize, 1, 2, 0, 2, 3]
        .into_iter()
        .map(|index| OverlayFaceVertex {
            position: quad[index],
            color: WORKPLANE_FILL,
        })
        .collect();
    let step = half / WORKPLANE_HALF_CELLS as f32;
    let mut lines = Vec::new();
    for cell in -WORKPLANE_HALF_CELLS..=WORKPLANE_HALF_CELLS {
        let offset = cell as f32 * step;
        let (color, width_px) = if cell == 0 {
            (WORKPLANE_AXIS, 1.5)
        } else {
            (WORKPLANE_GRID, 1.0)
        };
        for (a, b) in [
            (corner(offset, -half), corner(offset, half)),
            (corner(-half, offset), corner(half, offset)),
        ] {
            lines.push(OverlayLine {
                a,
                b,
                color,
                width_px,
            });
        }
    }
    (faces, lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_toggle_is_master_for_thin_edges_in_every_mode() {
        // Regressão: `edge_mode == Topology` forçava todas as arestas e
        // tornava o toggle um no-op no workspace padrão (POLY).
        for mode in [EdgeMode::Overlay, EdgeMode::Features, EdgeMode::Topology] {
            assert!(edge_overlay_visible(true, mode, false));
            assert!(!edge_overlay_visible(false, mode, false));
        }
    }

    #[test]
    fn draw_and_poly_keep_feature_edges_as_mode_reading() {
        assert!(edge_overlay_visible(false, EdgeMode::Features, true));
        assert!(edge_overlay_visible(false, EdgeMode::Topology, true));
        assert!(!edge_overlay_visible(false, EdgeMode::Overlay, true));
        for mode in [EdgeMode::Overlay, EdgeMode::Features, EdgeMode::Topology] {
            assert!(edge_overlay_visible(true, mode, true));
        }
    }

    #[test]
    fn filled_square_becomes_two_triangles_in_physical_pixels() {
        let square = OverlayShape {
            points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
            closed: true,
            fill: Some([1.0, 0.0, 0.0, 1.0]),
            stroke: None,
        };
        let vertices = tessellate_screen_overlay(&[square], 2.0);
        assert_eq!(vertices.len(), 6);
        assert!(vertices.iter().any(|v| v.position == [20.0, 20.0]));
    }

    #[test]
    fn stroke_width_scales_with_the_pixel_ratio() {
        let line = OverlayShape {
            points: vec![[0.0, 0.0], [10.0, 0.0]],
            closed: false,
            fill: None,
            stroke: Some(([1.0; 4], 2.0)),
        };
        let at = |ratio: f32| {
            let vertices = tessellate_screen_overlay(std::slice::from_ref(&line), ratio);
            vertices
                .iter()
                .map(|v| v.position[1])
                .fold(f32::MIN, f32::max)
        };
        assert!((at(1.0) - 1.0).abs() < 1e-5);
        assert!((at(2.0) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn non_finite_points_are_dropped() {
        let shape = OverlayShape {
            points: vec![[f32::NAN, 0.0], [0.0, 0.0], [5.0, 5.0]],
            closed: false,
            fill: None,
            stroke: Some(([1.0; 4], 1.0)),
        };
        assert_eq!(tessellate_screen_overlay(&[shape], 1.0).len(), 6);
    }

    fn top_camera() -> CameraMatrices {
        let mut camera = petunia_core::Camera::default();
        camera.set_preset(petunia_core::ViewPreset::Top);
        CameraMatrices::from_camera(&camera)
    }

    #[test]
    fn workplane_has_fill_grid_and_a_stronger_axis() {
        let plane = WorkplaneOverlay {
            origin: [0.0; 3],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 0.0, -1.0],
        };
        let (faces, lines) = workplane_geometry(plane, &top_camera());
        assert_eq!(faces.len(), 6);
        assert_eq!(lines.len(), 18);
        assert_eq!(lines.iter().filter(|l| l.width_px > 1.0).count(), 2);
        // O recorte é empurrado para o lado da câmera (Y+ na vista de topo).
        assert!(faces.iter().all(|v| v.position[1] > 0.0));
    }

    #[test]
    fn degenerate_workplane_draws_nothing() {
        let plane = WorkplaneOverlay {
            origin: [0.0; 3],
            right: [1.0, 0.0, 0.0],
            up: [1.0, 0.0, 0.0],
        };
        let (faces, lines) = workplane_geometry(plane, &top_camera());
        assert!(faces.is_empty() && lines.is_empty());
    }
}
