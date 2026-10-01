// Screen-space projection, gizmo geometry, raycasting, and viewport math.
// Projeção em espaço de tela, geometria do gizmo, raycasting e matemática de viewport.

use crate::*;
use petunia_core::{AppState, Camera, PivotPoint, SelectionDomain, Workspace};

/// Projeta um ponto do mundo para pixels da viewport; `None` atrás da câmera
/// ou fora do frustum de profundidade.
pub(crate) fn project_world_point(
    camera: &Camera,
    viewport: [f32; 2],
    point: glam::Vec3,
) -> Option<[f32; 2]> {
    let clip = camera.view_proj() * point.extend(1.0);
    if !clip.is_finite() || clip.w <= 0.05 || clip.z < 0.0 || clip.z > clip.w {
        return None;
    }
    Some([
        (clip.x / clip.w * 0.5 + 0.5) * viewport[0],
        (0.5 - clip.y / clip.w * 0.5) * viewport[1],
    ])
}

/// Menor `w` aceito no clip-space (plano de recorte próximo).
const NEAR_W: f32 = 0.05;

/// Interseção da aresta `a-b` (clip-space) com o plano `w = NEAR_W`.
fn clip_near_edge(a: glam::Vec4, b: glam::Vec4) -> glam::Vec4 {
    let t = (NEAR_W - a.w) / (b.w - a.w);
    a + (b - a) * t.clamp(0.0, 1.0)
}

fn clip_to_screen(clip: glam::Vec4, viewport: [f32; 2]) -> [f32; 2] {
    [
        (clip.x / clip.w * 0.5 + 0.5) * viewport[0],
        (0.5 - clip.y / clip.w * 0.5) * viewport[1],
    ]
}

/// Projeta um anel (ou polilinha) de mundo recortando no plano próximo.
///
/// Diferente de `project_world_point` por ponto, que descarta o desenho inteiro
/// (ou liga pontos que não são vizinhos), o recorte mantém a parte visível:
/// orbitar com a forma parcialmente atrás da câmera não a faz piscar nem
/// espirrar por cima da cena. `closed` aplica Sutherland–Hodgman; aberto devolve
/// trechos (`M` novo a cada descontinuidade). Retorna `false` se nada é visível.
pub(crate) fn write_clipped_path(
    camera: &Camera,
    viewport: [f32; 2],
    points: impl IntoIterator<Item = glam::Vec3>,
    closed: bool,
    out: &mut String,
) -> bool {
    use std::fmt::Write as _;
    let matrix = camera.view_proj();
    let clip: Vec<glam::Vec4> = points.into_iter().map(|p| matrix * p.extend(1.0)).collect();
    if clip.len() < 2 || clip.iter().any(|c| !c.is_finite()) {
        return false;
    }
    if closed && clip.len() >= 3 {
        let mut polygon: Vec<glam::Vec4> = Vec::with_capacity(clip.len() + 2);
        for i in 0..clip.len() {
            let (current, next) = (clip[i], clip[(i + 1) % clip.len()]);
            let (current_in, next_in) = (current.w > NEAR_W, next.w > NEAR_W);
            if current_in {
                polygon.push(current);
            }
            if current_in != next_in {
                polygon.push(clip_near_edge(current, next));
            }
        }
        if polygon.len() < 3 {
            return false;
        }
        for (i, c) in polygon.iter().enumerate() {
            let [x, y] = clip_to_screen(*c, viewport);
            let action = if i == 0 { 'M' } else { 'L' };
            let _ = write!(out, "{action} {x:.2} {y:.2} ");
        }
        out.push_str("Z ");
        return true;
    }
    let mut wrote = false;
    let mut pen_down = false;
    let mut segments: Vec<[glam::Vec4; 2]> = clip.windows(2).map(|w| [w[0], w[1]]).collect();
    if closed {
        segments.push([clip[clip.len() - 1], clip[0]]);
    }
    for [a, b] in segments {
        let (a_in, b_in) = (a.w > NEAR_W, b.w > NEAR_W);
        if !a_in && !b_in {
            pen_down = false;
            continue;
        }
        let start = if a_in { a } else { clip_near_edge(a, b) };
        let end = if b_in { b } else { clip_near_edge(a, b) };
        let [sx, sy] = clip_to_screen(start, viewport);
        let [ex, ey] = clip_to_screen(end, viewport);
        if !pen_down || !a_in {
            let _ = write!(out, "M {sx:.2} {sy:.2} ");
        }
        let _ = write!(out, "L {ex:.2} {ey:.2} ");
        pen_down = b_in;
        wrote = true;
    }
    wrote
}

pub(crate) fn project_preview_segment(
    camera: &Camera,
    viewport: [f32; 2],
    a: glam::Vec3,
    b: glam::Vec3,
    commands: &mut String,
) {
    if let (Some(a), Some(b)) = (
        project_world_point(camera, viewport, a),
        project_world_point(camera, viewport, b),
    ) {
        use std::fmt::Write as _;
        let _ = write!(
            commands,
            "M {:.2} {:.2} L {:.2} {:.2} ",
            a[0], a[1], b[0], b[1]
        );
    }
}

pub(crate) fn pivot_id(pivot: PivotPoint) -> &'static str {
    match pivot {
        PivotPoint::MedianPoint => "median",
        PivotPoint::BoundingBoxCenter => "bounds",
        PivotPoint::Cursor3D => "cursor",
        PivotPoint::IndividualOrigins => "individual",
    }
}

pub(crate) fn pivot_from_id(id: &str) -> Option<PivotPoint> {
    match id {
        "median" => Some(PivotPoint::MedianPoint),
        "bounds" => Some(PivotPoint::BoundingBoxCenter),
        "cursor" => Some(PivotPoint::Cursor3D),
        "individual" => Some(PivotPoint::IndividualOrigins),
        _ => None,
    }
}

/// Projeta o pivô da seleção e os três eixos do mundo para o overlay Slint.
/// Distância de um ponto a um segmento, em espaço de tela.
pub fn point_segment_distance(point: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1]];
    let ap = [point[0] - a[0], point[1] - a[1]];
    let length_squared = ab[0] * ab[0] + ab[1] * ab[1];
    if length_squared <= 1.0e-6 {
        return (ap[0] * ap[0] + ap[1] * ap[1]).sqrt();
    }
    let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / length_squared).clamp(0.0, 1.0);
    let closest = [a[0] + ab[0] * t, a[1] + ab[1] * t];
    ((point[0] - closest[0]).powi(2) + (point[1] - closest[1]).powi(2)).sqrt()
}

pub(crate) fn parse_lasso_path(path: &str) -> Option<Vec<[f32; 2]>> {
    if path.len() > 131_072 {
        return None;
    }
    let mut polygon = Vec::new();
    for part in path.split(';').filter(|part| !part.is_empty()) {
        let (x, y) = part.split_once(',')?;
        let x: f32 = x.parse().ok()?;
        let y: f32 = y.parse().ok()?;
        if !x.is_finite() || !y.is_finite() || polygon.len() >= 4096 {
            return None;
        }
        // O grab entrega valores fora da viewport. Projetar na borda mantém
        // a forma do laço, em vez de descartar pontos e abrir um corte.
        polygon.push([x.clamp(0.0, 1.0) * 2.0 - 1.0, 1.0 - y.clamp(0.0, 1.0) * 2.0]);
    }
    (polygon.len() >= 3).then_some(polygon)
}

/// Comprimento das hastes do gizmo de transformação, em px lógicos.
pub(crate) const GIZMO_ROD_LENGTH: f32 = 72.0;
/// Raio do anel externo de View Roll da ferramenta Rotate, em px lógicos.
/// Desenho e hit-test usam a mesma constante.
pub(crate) const GIZMO_VIEW_ROLL_RADIUS: f32 = GIZMO_ROD_LENGTH * 1.18;
/// Meia-largura da faixa clicável de anéis: 24 px lógicos no total (WCAG 2.5.8).
pub(crate) const GIZMO_RING_HIT_HALF_WIDTH: f32 = 12.0;

pub(crate) fn compute_gizmo(state: &AppState, width: f32, height: f32) -> GizmoModel {
    const ROD_LENGTH: f32 = GIZMO_ROD_LENGTH;
    /// Tamanho da seta: recuo da ponta e meia-largura da base.
    const ARROW_BACK: f32 = 13.0;
    const ARROW_HALF: f32 = 5.5;
    /// Tripé de navegação: no alto à direita, abaixo dos controles da viewport.
    const VIEW_MARGIN: f32 = 54.0;
    const VIEW_TOP: f32 = 108.0;
    const VIEW_LENGTH: f32 = 38.0;

    if width <= 1.0 || height <= 1.0 {
        return GizmoModel::default();
    }

    let view_proj = state.session.camera.view_proj();
    let project = |point: glam::Vec3| -> Option<[f32; 2]> {
        let clip = view_proj * glam::Vec4::new(point.x, point.y, point.z, 1.0);
        if clip.w <= 0.05 {
            return None;
        }
        let inv_w = 1.0 / clip.w;
        Some([
            (clip.x * inv_w * 0.5 + 0.5) * width,
            (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height,
        ])
    };
    // Camera-space directions preserve foreshortening: an axis pointing at
    // the viewer should shrink, not turn into a full-length diagonal.
    let screen_direction = |axis: glam::Vec3| -> [f32; 2] {
        [
            axis.dot(state.session.camera.right()),
            -axis.dot(state.session.camera.up()),
        ]
    };

    let mut model = GizmoModel::default();

    // Tripé de navegação: sempre visível quando a viewport tem tamanho válido.
    // Ele mostra a orientação da câmera, não a cena: por isso as hastes partem
    // de uma âncora fixa no canto, não de um ponto projetado.
    {
        model.view_origin_x = width - VIEW_MARGIN;
        model.view_origin_y = VIEW_TOP.min(height - VIEW_MARGIN);
        let origin = [VIEW_MARGIN, VIEW_MARGIN];
        for (axis, slot, endpoint) in [
            (
                glam::Vec3::X,
                &mut model.view_x_commands,
                &mut model.view_x_end,
            ),
            (
                glam::Vec3::Y,
                &mut model.view_y_commands,
                &mut model.view_y_end,
            ),
            (
                glam::Vec3::Z,
                &mut model.view_z_commands,
                &mut model.view_z_end,
            ),
        ] {
            let direction = screen_direction(axis);
            let end = [
                origin[0] + direction[0] * VIEW_LENGTH,
                origin[1] + direction[1] * VIEW_LENGTH,
            ];
            *endpoint = end;
            *slot = format!(
                "M {:.2} {:.2} L {:.2} {:.2} ",
                origin[0], origin[1], end[0], end[1]
            );
        }
    }

    // 3D Cursor screen projection:
    if state.session.show_cursor && state.session.show_overlays {
        let cursor_world = glam::Vec3::from(state.session.cursor_3d);
        let clip = view_proj * glam::Vec4::new(cursor_world.x, cursor_world.y, cursor_world.z, 1.0);
        if clip.is_finite() && clip.w > 0.05 && clip.z >= 0.0 && clip.z <= clip.w {
            let inv_w = 1.0 / clip.w;
            let sx = (clip.x * inv_w * 0.5 + 0.5) * width;
            let sy = (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height;
            if sx >= 0.0 && sx <= width && sy >= 0.0 && sy <= height {
                model.cursor_screen = [sx, sy];
                model.cursor_visible = true;
            }
        }
    }

    // Hastes de transformação: tamanho fixo em tela. O modo combinado mantém
    // três famílias de handle simultâneas e selecionáveis.
    if state.workspace != Workspace::Model
        || !matches!(
            state.session.tools.active_tool.as_str(),
            "move" | "rotate" | "scale" | "transform"
        )
    {
        return model;
    }
    let Some(asset) = state.project.active() else {
        return model;
    };
    if asset.mesh.verts.is_empty() {
        return model;
    };
    let pivot = state.session.tools.modal.as_ref().map_or_else(
        || state.calculate_pivot(state.session.pivot_point),
        |modal| modal.pivot,
    );
    let Some(origin) = project(pivot) else {
        return model;
    };
    model.visible = true;
    model.origin_x = origin[0];
    model.origin_y = origin[1];

    for (axis, rod, arrow, scale, rotate) in [
        (
            glam::Vec3::X,
            &mut model.x_commands,
            &mut model.x_arrow_commands,
            &mut model.x_scale_commands,
            &mut model.x_rotate_commands,
        ),
        (
            glam::Vec3::Y,
            &mut model.y_commands,
            &mut model.y_arrow_commands,
            &mut model.y_scale_commands,
            &mut model.y_rotate_commands,
        ),
        (
            glam::Vec3::Z,
            &mut model.z_commands,
            &mut model.z_arrow_commands,
            &mut model.z_scale_commands,
            &mut model.z_rotate_commands,
        ),
    ] {
        let direction = screen_direction(axis);
        let end = [
            origin[0] + direction[0] * ROD_LENGTH,
            origin[1] + direction[1] * ROD_LENGTH,
        ];
        let label_pos = [
            origin[0] + direction[0] * (ROD_LENGTH + 14.0),
            origin[1] + direction[1] * (ROD_LENGTH + 14.0),
        ];
        if axis == glam::Vec3::X {
            model.x_end = end;
            model.x_label = label_pos;
        } else if axis == glam::Vec3::Y {
            model.y_end = end;
            model.y_label = label_pos;
        } else if axis == glam::Vec3::Z {
            model.z_end = end;
            model.z_label = label_pos;
        }

        if state.session.tools.active_tool == "rotate" {
            let tangent = if axis == glam::Vec3::X {
                glam::Vec3::Y
            } else {
                glam::Vec3::X
            };
            let bitangent = axis.cross(tangent);
            for segment in 0..=64 {
                let angle = segment as f32 * std::f32::consts::TAU / 64.0;
                let direction = screen_direction(tangent * angle.cos() + bitangent * angle.sin());
                rod.push_str(&format!(
                    "{} {:.2} {:.2} ",
                    if segment == 0 { "M" } else { "L" },
                    origin[0] + direction[0] * ROD_LENGTH,
                    origin[1] + direction[1] * ROD_LENGTH
                ));
            }
            continue;
        }
        *rod = format!(
            "M {:.2} {:.2} L {:.2} {:.2} ",
            origin[0], origin[1], end[0], end[1]
        );
        if state.session.tools.active_tool == "transform" {
            let scale_center = [
                origin[0] + direction[0] * 43.0,
                origin[1] + direction[1] * 43.0,
            ];
            let r = 5.0;
            *scale = format!(
                "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                scale_center[0] - r,
                scale_center[1] - r,
                scale_center[0] + r,
                scale_center[1] - r,
                scale_center[0] + r,
                scale_center[1] + r,
                scale_center[0] - r,
                scale_center[1] + r,
            );
            let tangent = if axis == glam::Vec3::X {
                glam::Vec3::Y
            } else {
                glam::Vec3::X
            };
            let bitangent = axis.cross(tangent);
            for segment in 0..=64 {
                let angle = segment as f32 * std::f32::consts::TAU / 64.0;
                let ring_direction =
                    screen_direction(tangent * angle.cos() + bitangent * angle.sin());
                rotate.push_str(&format!(
                    "{} {:.2} {:.2} ",
                    if segment == 0 { "M" } else { "L" },
                    origin[0] + ring_direction[0] * 91.0,
                    origin[1] + ring_direction[1] * 91.0
                ));
            }
        }
        if state.session.tools.active_tool == "scale" {
            let r = ARROW_HALF;
            *arrow = format!(
                "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
                end[0] - r,
                end[1] - r,
                end[0] + r,
                end[1] - r,
                end[0] + r,
                end[1] + r,
                end[0] - r,
                end[1] + r
            );
            continue;
        }
        // Seta: ponta em `end`, base recuada ao longo da haste.
        let base = [
            end[0] - direction[0] * ARROW_BACK,
            end[1] - direction[1] * ARROW_BACK,
        ];
        let perpendicular = [-direction[1], direction[0]];
        let left = [
            base[0] + perpendicular[0] * ARROW_HALF,
            base[1] + perpendicular[1] * ARROW_HALF,
        ];
        let right = [
            base[0] - perpendicular[0] * ARROW_HALF,
            base[1] - perpendicular[1] * ARROW_HALF,
        ];
        *arrow = format!(
            "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
            end[0], end[1], left[0], left[1], right[0], right[1]
        );
    }

    // Quadrantes dos planos (Plane Handles) para Move e Scale
    if matches!(state.session.tools.active_tool.as_str(), "move" | "scale") {
        let dir_x = screen_direction(glam::Vec3::X);
        let dir_y = screen_direction(glam::Vec3::Y);
        let dir_z = screen_direction(glam::Vec3::Z);

        // Plane 0: YZ (normal X)
        let p1 = [
            origin[0] + (dir_y[0] + dir_z[0]) * 18.0,
            origin[1] + (dir_y[1] + dir_z[1]) * 18.0,
        ];
        let p2 = [
            origin[0] + dir_y[0] * 32.0 + dir_z[0] * 18.0,
            origin[1] + dir_y[1] * 32.0 + dir_z[1] * 18.0,
        ];
        let p3 = [
            origin[0] + (dir_y[0] + dir_z[0]) * 32.0,
            origin[1] + (dir_y[1] + dir_z[1]) * 32.0,
        ];
        let p4 = [
            origin[0] + dir_y[0] * 18.0 + dir_z[0] * 32.0,
            origin[1] + dir_y[1] * 18.0 + dir_z[1] * 32.0,
        ];
        model.plane_yz_commands = format!(
            "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
            p1[0], p1[1], p2[0], p2[1], p3[0], p3[1], p4[0], p4[1]
        );

        // Plane 1: XZ (normal Y)
        let p1 = [
            origin[0] + (dir_x[0] + dir_z[0]) * 18.0,
            origin[1] + (dir_x[1] + dir_z[1]) * 18.0,
        ];
        let p2 = [
            origin[0] + dir_x[0] * 32.0 + dir_z[0] * 18.0,
            origin[1] + dir_x[1] * 32.0 + dir_z[1] * 18.0,
        ];
        let p3 = [
            origin[0] + (dir_x[0] + dir_z[0]) * 32.0,
            origin[1] + (dir_x[1] + dir_z[1]) * 32.0,
        ];
        let p4 = [
            origin[0] + dir_x[0] * 18.0 + dir_z[0] * 32.0,
            origin[1] + dir_x[1] * 18.0 + dir_z[1] * 32.0,
        ];
        model.plane_xz_commands = format!(
            "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
            p1[0], p1[1], p2[0], p2[1], p3[0], p3[1], p4[0], p4[1]
        );

        // Plane 2: XY (normal Z)
        let p1 = [
            origin[0] + (dir_x[0] + dir_y[0]) * 18.0,
            origin[1] + (dir_x[1] + dir_y[1]) * 18.0,
        ];
        let p2 = [
            origin[0] + dir_x[0] * 32.0 + dir_y[0] * 18.0,
            origin[1] + dir_x[1] * 32.0 + dir_y[1] * 18.0,
        ];
        let p3 = [
            origin[0] + (dir_x[0] + dir_y[0]) * 32.0,
            origin[1] + (dir_x[1] + dir_y[1]) * 32.0,
        ];
        let p4 = [
            origin[0] + dir_x[0] * 18.0 + dir_y[0] * 32.0,
            origin[1] + dir_x[1] * 18.0 + dir_y[1] * 32.0,
        ];
        model.plane_xy_commands = format!(
            "M {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} L {:.2} {:.2} Z ",
            p1[0], p1[1], p2[0], p2[1], p3[0], p3[1], p4[0], p4[1]
        );
    }

    // Anel externo de rotação da visão (View Roll) para Rotate
    if state.session.tools.active_tool == "rotate" {
        let roll_r = GIZMO_VIEW_ROLL_RADIUS;
        let mut roll = String::new();
        for segment in 0..=64 {
            let angle = segment as f32 * std::f32::consts::TAU / 64.0;
            let px = origin[0] + roll_r * angle.cos();
            let py = origin[1] + roll_r * angle.sin();
            roll.push_str(&format!(
                "{} {:.2} {:.2} ",
                if segment == 0 { "M" } else { "L" },
                px,
                py
            ));
        }
        model.view_roll_commands = roll;
    }

    model
}

/// Tracejado pontilhado do "cordão" entre a base da seleção (pivô) e o
/// mouse durante uma ferramenta de manipulação ativa (Move/Rotate/Scale por
/// arrasto ou modal de teclado, Extrude/Inset/Bevel paramétricos...).
/// O Path do Slint não tem dash nativo, então o padrão nasce aqui em Rust:
/// pontos de 2px com 4px de intervalo ao longo do segmento base→mouse.
/// A linha cresce/encolhe sozinha conforme o mouse se afasta/aproxima.
pub(crate) fn dotted_line_with_spacing(
    from: [f32; 2],
    to: [f32; 2],
    dash: f32,
    gap: f32,
) -> String {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let length = dx.hypot(dy);
    if length < 0.5 {
        return String::new();
    }
    let (ux, uy) = (dx / length, dy / length);
    let mut commands = String::new();
    let mut cursor = 0.0;
    while cursor < length {
        let end = (cursor + dash).min(length);
        use std::fmt::Write as _;
        let _ = write!(
            commands,
            "M {:.2} {:.2} L {:.2} {:.2} ",
            from[0] + ux * cursor,
            from[1] + uy * cursor,
            from[0] + ux * end,
            from[1] + uy * end,
        );
        cursor = end + gap;
    }
    commands
}

pub(crate) fn dotted_link_commands(from: [f32; 2], to: [f32; 2]) -> String {
    dotted_line_with_spacing(from, to, 2.0, 4.0)
}

/// Active tool pivot-to-pointer or guide-line feedback cord (P3D-131).
/// Cordão pivô→mouse ou guia de telemetria da ferramenta ativa (P3D-131).
pub(crate) fn compute_drag_link(
    state: &AppState,
    width: f32,
    height: f32,
    pointer: [f32; 2],
    link_active: bool,
) -> String {
    if !link_active || width <= 1.0 || height <= 1.0 {
        return String::new();
    }
    let view_proj = state.session.camera.view_proj();
    let fb = state.current_tool_feedback();
    let pivot = fb
        .as_ref()
        .map(|f| f.origin)
        .unwrap_or_else(|| state.calculate_pivot(state.session.pivot_point));
    let clip = view_proj * pivot.extend(1.0);
    if clip.w <= 0.05 {
        return String::new();
    }
    let inv_w = 1.0 / clip.w;
    let base = [
        (clip.x * inv_w * 0.5 + 0.5) * width,
        (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height,
    ];

    let target = if let Some(fb) = fb
        && fb.is_snapped
        && (fb.current - fb.origin).length() > 1e-4
    {
        // When snapped to magnetic target with non-zero delta, project the 3D snapped coordinate
        // Quando atraído por alvo magnético com delta não-nulo, projeta a coordenada 3D sob snap
        let clip_t = view_proj * fb.current.extend(1.0);
        if clip_t.w > 0.05 {
            let inv_t = 1.0 / clip_t.w;
            [
                (clip_t.x * inv_t * 0.5 + 0.5) * width,
                (1.0 - (clip_t.y * inv_t * 0.5 + 0.5)) * height,
            ]
        } else {
            pointer
        }
    } else {
        pointer
    };

    dotted_link_commands(base, target)
}

/// Círculo pontilhado do raio de influência do Proportional Editing (Soft Selection).
pub(crate) fn compute_proportional_circle(
    state: &AppState,
    width: f32,
    height: f32,
    active: bool,
) -> String {
    if !active || !state.session.proportional_editing || width <= 1.0 || height <= 1.0 {
        return String::new();
    }
    let radius = state.session.proportional_settings.radius;
    if radius <= 1e-4 {
        return String::new();
    }
    let view_proj = state.session.camera.view_proj();
    let fb = state.current_tool_feedback();
    let pivot = fb
        .as_ref()
        .map(|f| f.origin)
        .unwrap_or_else(|| state.calculate_pivot(state.session.pivot_point));
    let clip = view_proj * pivot.extend(1.0);
    if clip.w <= 0.05 {
        return String::new();
    }
    let inv_w = 1.0 / clip.w;
    let cx = (clip.x * inv_w * 0.5 + 0.5) * width;
    let cy = (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height;

    let edge_3d = pivot + state.session.camera.right() * radius;
    let clip_e = view_proj * edge_3d.extend(1.0);
    if clip_e.w <= 0.05 {
        return String::new();
    }
    let inv_e = 1.0 / clip_e.w;
    let ex = (clip_e.x * inv_e * 0.5 + 0.5) * width;
    let ey = (1.0 - (clip_e.y * inv_e * 0.5 + 0.5)) * height;

    let r = (ex - cx).hypot(ey - cy);
    if r < 2.0 {
        return String::new();
    }

    let mut commands = String::new();
    use std::fmt::Write as _;
    let segments = 36;
    for i in 0..segments {
        let theta_start = (i as f32) * std::f32::consts::TAU / (segments as f32);
        let theta_mid = ((i as f32) + 0.55) * std::f32::consts::TAU / (segments as f32);
        let p1x = cx + r * theta_start.cos();
        let p1y = cy + r * theta_start.sin();
        let p2x = cx + r * theta_mid.cos();
        let p2y = cy + r * theta_mid.sin();
        let _ = write!(commands, "M {:.2} {:.2} L {:.2} {:.2} ", p1x, p1y, p2x, p2y);
    }
    commands
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AxisGuideModel {
    pub visible: bool,
    pub commands: String,
    pub color: [u8; 3],
    pub label: String,
    pub label_x: f32,
    pub label_y: f32,
}

pub(crate) fn compute_axis_guide(state: &AppState, width: f32, height: f32) -> AxisGuideModel {
    if width <= 1.0 || height <= 1.0 {
        return AxisGuideModel::default();
    }
    let Some(modal) = state.modal.as_ref() else {
        return AxisGuideModel::default();
    };
    let axis_idx = match modal.constraint {
        petunia_core::ModalConstraint::Axis(i) => i.min(2),
        _ => return AxisGuideModel::default(),
    };

    let dir = match axis_idx {
        0 => glam::Vec3::X,
        1 => glam::Vec3::Y,
        _ => glam::Vec3::Z,
    };
    let color = match axis_idx {
        0 => [229, 77, 66], // Red
        1 => [70, 167, 88], // Green
        _ => [62, 99, 221], // Blue
    };

    let camera = &state.session.camera;
    let view_proj = camera.view_proj();
    let pivot = modal.pivot;

    // Project pivot to screen
    let clip_p = view_proj * pivot.extend(1.0);
    if clip_p.w <= 0.05 {
        return AxisGuideModel::default();
    }
    let p_screen = [
        (clip_p.x / clip_p.w * 0.5 + 0.5) * width,
        (1.0 - (clip_p.y / clip_p.w * 0.5 + 0.5)) * height,
    ];

    // Project a point along the axis to find screen direction
    let p_axis = pivot + dir * 5.0;
    let clip_a = view_proj * p_axis.extend(1.0);
    if clip_a.w <= 0.05 {
        return AxisGuideModel::default();
    }
    let a_screen = [
        (clip_a.x / clip_a.w * 0.5 + 0.5) * width,
        (1.0 - (clip_a.y / clip_a.w * 0.5 + 0.5)) * height,
    ];

    let dx = a_screen[0] - p_screen[0];
    let dy = a_screen[1] - p_screen[1];
    let len = dx.hypot(dy);
    if len < 0.5 {
        return AxisGuideModel::default();
    }
    let (ux, uy) = (dx / len, dy / len);

    // Compute infinite line spanning across the viewport
    let max_dim = width.max(height) * 2.0;
    let start = [p_screen[0] - ux * max_dim, p_screen[1] - uy * max_dim];
    let end = [p_screen[0] + ux * max_dim, p_screen[1] + uy * max_dim];

    let commands = dotted_line_with_spacing(start, end, 6.0, 4.0);

    let label = match axis_idx {
        0 => "X".to_string(),
        1 => "Y".to_string(),
        _ => "Z".to_string(),
    };
    let badge_dist = 64.0;
    let label_x = p_screen[0] + ux * badge_dist;
    let label_y = p_screen[1] + uy * badge_dist;

    AxisGuideModel {
        visible: true,
        commands,
        color,
        label,
        label_x,
        label_y,
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorldAxisLabelModel {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub color: [u8; 3],
}

pub(crate) fn compute_world_axis_labels(
    state: &AppState,
    width: f32,
    height: f32,
) -> Vec<WorldAxisLabelModel> {
    if !state.ui.colorblind_axes
        || width <= 1.0
        || height <= 1.0
        || state.workspace != Workspace::Model
    {
        return Vec::new();
    }

    let view_proj = state.session.camera.view_proj();
    let project = |pos: glam::Vec3| -> Option<[f32; 2]> {
        let clip = view_proj * pos.extend(1.0);
        if clip.w <= 0.05 {
            return None;
        }
        let inv_w = 1.0 / clip.w;
        let sx = (clip.x * inv_w * 0.5 + 0.5) * width;
        let sy = (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height;
        if sx >= 24.0 && sx <= width - 24.0 && sy >= 24.0 && sy <= height - 24.0 {
            Some([sx, sy])
        } else {
            None
        }
    };

    let mut labels = Vec::new();
    const EXTENT: f32 = 8.0;
    let candidates = [
        (glam::Vec3::new(EXTENT, 0.0, 0.0), "+X", [229, 77, 66]),
        (glam::Vec3::new(-EXTENT, 0.0, 0.0), "-X", [229, 77, 66]),
        (glam::Vec3::new(0.0, 0.0, EXTENT), "+Z", [62, 99, 221]),
        (glam::Vec3::new(0.0, 0.0, -EXTENT), "-Z", [62, 99, 221]),
    ];

    for (pos, text, color) in candidates {
        if let Some([x, y]) = project(pos) {
            labels.push(WorldAxisLabelModel {
                text: text.to_string(),
                x,
                y,
                color,
            });
        }
    }

    labels
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DimensionAnnotationModel {
    pub visible: bool,
    pub commands: String,
    pub text: String,
    pub label_x: f32,
    pub label_y: f32,
}

pub(crate) fn compute_dimension_annotation(
    state: &AppState,
    width: f32,
    height: f32,
) -> DimensionAnnotationModel {
    if width <= 1.0 || height <= 1.0 {
        return DimensionAnnotationModel::default();
    }
    let Some(modal) = state.modal.as_ref() else {
        return DimensionAnnotationModel::default();
    };

    // Uma cota linear só existe quando o valor é uma distância no mundo.
    // Rotate (graus), Scale (fator), Inset e Bevel aparecem no HUD, não aqui.
    let Some(p_end) = modal.current_point() else {
        return DimensionAnnotationModel::default();
    };
    let delta_val = (p_end - modal.pivot).length();

    if delta_val < 0.005 {
        return DimensionAnnotationModel::default();
    }

    let view_proj = state.session.camera.view_proj();
    let p_start = modal.pivot;

    let clip_start = view_proj * p_start.extend(1.0);
    let clip_end = view_proj * p_end.extend(1.0);
    if clip_start.w <= 0.05 || clip_end.w <= 0.05 {
        return DimensionAnnotationModel::default();
    }

    let a = [
        (clip_start.x / clip_start.w * 0.5 + 0.5) * width,
        (1.0 - (clip_start.y / clip_start.w * 0.5 + 0.5)) * height,
    ];
    let b = [
        (clip_end.x / clip_end.w * 0.5 + 0.5) * width,
        (1.0 - (clip_end.y / clip_end.w * 0.5 + 0.5)) * height,
    ];

    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let screen_dist = dx.hypot(dy);
    if screen_dist < 10.0 {
        return DimensionAnnotationModel::default();
    }

    let (ux, uy) = (dx / screen_dist, dy / screen_dist);
    let (nx, ny) = (-uy, ux);

    let mut commands = String::new();
    use std::fmt::Write as _;
    let _ = write!(
        commands,
        "M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} ",
        a[0] - nx * 6.0,
        a[1] - ny * 6.0,
        a[0] + nx * 6.0,
        a[1] + ny * 6.0,
        b[0] - nx * 6.0,
        b[1] - ny * 6.0,
        b[0] + nx * 6.0,
        b[1] + ny * 6.0,
        a[0],
        a[1],
        b[0],
        b[1],
    );

    let text = format!("{:.2} m", delta_val);

    let label_x = (a[0] + b[0]) * 0.5 + nx * 14.0;
    let label_y = (a[1] + b[1]) * 0.5 + ny * 14.0;

    DimensionAnnotationModel {
        visible: true,
        commands,
        text,
        label_x,
        label_y,
    }
}

/// Rótulo individual de medição projetado na viewport (ex.: em uma aresta).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeasureTagModel {
    pub text: String,
    pub x: f32,
    pub y: f32,
}

/// Modelo de fita métrica tridimensional e consulta dimensional rápida.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuickMeasureModel {
    pub visible: bool,
    pub commands: String,
    pub distance: f32,
    pub dx: f32,
    pub dy: f32,
    pub dz: f32,
    pub angle_deg: f32,
    pub text: String,
    pub label_x: f32,
    pub label_y: f32,
    pub hud_text: String,
    pub tags: Vec<MeasureTagModel>,
}

pub(crate) fn compute_quick_measure(
    state: &AppState,
    width: f32,
    height: f32,
) -> QuickMeasureModel {
    if width <= 1.0 || height <= 1.0 {
        return QuickMeasureModel::default();
    }

    use std::fmt::Write as _;
    let view_proj = state.session.camera.view_proj();

    // Helper closure para projetar ponto 3D para espaço de tela (pixels)
    let project_pt = |p: glam::Vec3| -> Option<[f32; 2]> {
        let clip = view_proj * p.extend(1.0);
        if clip.w <= 0.05 {
            return None;
        }
        Some([
            (clip.x / clip.w * 0.5 + 0.5) * width,
            (1.0 - (clip.y / clip.w * 0.5 + 0.5)) * height,
        ])
    };

    // Helper para calcular vetor perpendicular orientado para cima/direita com offset de 24px
    let compute_offset = |a: [f32; 2], b: [f32; 2]| -> Option<([f32; 2], [f32; 2], [f32; 2])> {
        let screen_dx = b[0] - a[0];
        let screen_dy = b[1] - a[1];
        let screen_dist = screen_dx.hypot(screen_dy);
        if screen_dist < 4.0 {
            return None;
        }
        let ux = screen_dx / screen_dist;
        let uy = screen_dy / screen_dist;
        let mut nx = -uy;
        let mut ny = ux;
        // Orientação preferencial para cima ou para a direita para não colidir com a linha
        if ny > 0.0 || (ny.abs() < 1e-4 && nx < 0.0) {
            nx = -nx;
            ny = -ny;
        }
        let tag_pos = [
            (a[0] + b[0]) * 0.5 + nx * 24.0,
            (a[1] + b[1]) * 0.5 + ny * 24.0,
        ];
        Some(([nx, ny], [ux, uy], tag_pos))
    };

    // Caso 1: medição ativa explícita no ToolState (Ferramenta Measure / Ruler)
    if let Some(m) = state.session.tools.active_measurement.as_ref() {
        let p_start = glam::Vec3::from_array(m.start);
        let p_end = glam::Vec3::from_array(m.end);
        let delta = p_end - p_start;
        let distance = delta.length();
        if distance < 1e-4 {
            return QuickMeasureModel::default();
        }
        let dx = delta.x.abs();
        let dy = delta.y.abs();
        let dz = delta.z.abs();
        let angle_deg = delta.y.atan2(delta.x).to_degrees().abs();

        let (Some(a), Some(b)) = (project_pt(p_start), project_pt(p_end)) else {
            return QuickMeasureModel::default();
        };

        let Some(([nx, ny], _, tag_pos)) = compute_offset(a, b) else {
            return QuickMeasureModel::default();
        };

        let mut commands = String::new();
        let _ = write!(
            commands,
            "M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} ",
            a[0] - nx * 6.0,
            a[1] - ny * 6.0,
            a[0] + nx * 6.0,
            a[1] + ny * 6.0,
            b[0] - nx * 6.0,
            b[1] - ny * 6.0,
            b[0] + nx * 6.0,
            b[1] + ny * 6.0,
            a[0],
            a[1],
            b[0],
            b[1],
        );

        let tag_text = format!("{:.3}m", distance);
        let hud_text = format!(
            "{:.3}m  ·  ΔX: {:.3}  ΔY: {:.3}  ΔZ: {:.3}  ·  {:.1}°",
            distance, dx, dy, dz, angle_deg
        );
        let text = format!(
            "{:.3}m  |  ΔX: {:.3}  ΔY: {:.3}  ΔZ: {:.3}  |  {:.1}°",
            distance, dx, dy, dz, angle_deg
        );

        let tags = vec![MeasureTagModel {
            text: tag_text,
            x: tag_pos[0],
            y: tag_pos[1],
        }];

        return QuickMeasureModel {
            visible: true,
            commands,
            distance,
            dx,
            dy,
            dz,
            angle_deg,
            text,
            label_x: tag_pos[0],
            label_y: tag_pos[1],
            hud_text,
            tags,
        };
    }

    // Caso 2: Arestas selecionadas (suporta 1 aresta e múltiplas arestas)
    let selected_edges: Vec<(u32, u32)> = if let Some(mesh) = state.project.active_mesh() {
        if !mesh.selected_edges.is_empty() {
            mesh.selected_edges.iter().copied().collect()
        } else if !state.selection.edges.is_empty() {
            state.selection.edges.clone()
        } else {
            Vec::new()
        }
    } else if !state.selection.edges.is_empty() {
        state.selection.edges.clone()
    } else {
        Vec::new()
    };

    if !selected_edges.is_empty() {
        let Some(mesh) = state.project.active_mesh() else {
            return QuickMeasureModel::default();
        };

        let mut sorted_edges = selected_edges;
        sorted_edges.sort_unstable();
        sorted_edges.dedup();

        if sorted_edges.len() == 1 {
            let (v0, v1) = sorted_edges[0];
            let idx0 = v0 as usize;
            let idx1 = v1 as usize;
            if idx0 >= mesh.verts.len() || idx1 >= mesh.verts.len() || idx0 == idx1 {
                return QuickMeasureModel::default();
            }
            let p0 = mesh.verts[idx0].vec();
            let p1 = mesh.verts[idx1].vec();
            let delta = p1 - p0;
            let distance = delta.length();
            if distance < 1e-4 {
                return QuickMeasureModel::default();
            }
            let dx = delta.x.abs();
            let dy = delta.y.abs();
            let dz = delta.z.abs();
            let angle_deg = delta.y.atan2(delta.x).to_degrees().abs();

            let (Some(a), Some(b)) = (project_pt(p0), project_pt(p1)) else {
                return QuickMeasureModel::default();
            };
            let Some(([nx, ny], _, tag_pos)) = compute_offset(a, b) else {
                return QuickMeasureModel::default();
            };

            let mut commands = String::new();
            let _ = write!(
                commands,
                "M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} ",
                a[0] - nx * 6.0,
                a[1] - ny * 6.0,
                a[0] + nx * 6.0,
                a[1] + ny * 6.0,
                b[0] - nx * 6.0,
                b[1] - ny * 6.0,
                b[0] + nx * 6.0,
                b[1] + ny * 6.0,
                a[0],
                a[1],
                b[0],
                b[1],
            );

            let tag_text = format!("{:.3}m", distance);
            let hud_text = format!(
                "{:.3}m  ·  ΔX: {:.3}  ΔY: {:.3}  ΔZ: {:.3}  ·  {:.1}°",
                distance, dx, dy, dz, angle_deg
            );
            let text = format!(
                "{:.3}m  |  ΔX: {:.3}  ΔY: {:.3}  ΔZ: {:.3}  |  {:.1}°",
                distance, dx, dy, dz, angle_deg
            );
            let tags = vec![MeasureTagModel {
                text: tag_text,
                x: tag_pos[0],
                y: tag_pos[1],
            }];

            return QuickMeasureModel {
                visible: true,
                commands,
                distance,
                dx,
                dy,
                dz,
                angle_deg,
                text,
                label_x: tag_pos[0],
                label_y: tag_pos[1],
                hud_text,
                tags,
            };
        } else {
            // Múltiplas arestas selecionadas
            let mut total_distance = 0.0f32;
            let mut edge_count = 0usize;
            let mut min_pt = glam::Vec3::splat(f32::INFINITY);
            let mut max_pt = glam::Vec3::splat(f32::NEG_INFINITY);
            let mut commands = String::new();
            let mut vert_counts: std::collections::HashMap<u32, usize> =
                std::collections::HashMap::new();

            for &(v0, v1) in &sorted_edges {
                let idx0 = v0 as usize;
                let idx1 = v1 as usize;
                if idx0 >= mesh.verts.len() || idx1 >= mesh.verts.len() || idx0 == idx1 {
                    continue;
                }
                let p0 = mesh.verts[idx0].vec();
                let p1 = mesh.verts[idx1].vec();
                let edge_dist = (p1 - p0).length();
                if edge_dist < 1e-4 {
                    continue;
                }
                total_distance += edge_dist;
                edge_count += 1;
                min_pt = min_pt.min(p0).min(p1);
                max_pt = max_pt.max(p0).max(p1);
                *vert_counts.entry(v0).or_insert(0) += 1;
                *vert_counts.entry(v1).or_insert(0) += 1;

                let (Some(a), Some(b)) = (project_pt(p0), project_pt(p1)) else {
                    continue;
                };
                let Some(([nx, ny], _, _)) = compute_offset(a, b) else {
                    continue;
                };
                let _ = write!(
                    commands,
                    "M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} ",
                    a[0] - nx * 5.0,
                    a[1] - ny * 5.0,
                    a[0] + nx * 5.0,
                    a[1] + ny * 5.0,
                    b[0] - nx * 5.0,
                    b[1] - ny * 5.0,
                    b[0] + nx * 5.0,
                    b[1] + ny * 5.0,
                    a[0],
                    a[1],
                    b[0],
                    b[1],
                );
            }

            if edge_count == 0 {
                return QuickMeasureModel::default();
            }

            let avg_distance = total_distance / edge_count as f32;
            let span = (max_pt - min_pt).abs();
            let hud_text = format!(
                "Total: {:.3}m ({} edges)  ·  Avg: {:.3}m  ·  Span: {:.3} × {:.3} × {:.3}m",
                total_distance, edge_count, avg_distance, span.x, span.y, span.z
            );
            let text = format!("Total: {:.3}m ({} edges)", total_distance, edge_count);

            // Ponto de interseção ou centróide geométrico das arestas selecionadas
            let center_pt =
                if let Some((&best_v, &max_c)) = vert_counts.iter().max_by_key(|entry| *entry.1) {
                    if max_c >= 2 && vert_counts.values().filter(|&&c| c == max_c).count() == 1 {
                        mesh.verts[best_v as usize].vec()
                    } else {
                        let sum: glam::Vec3 = vert_counts
                            .keys()
                            .map(|&v| mesh.verts[v as usize].vec())
                            .sum();
                        sum / (vert_counts.len() as f32)
                    }
                } else {
                    (min_pt + max_pt) * 0.5
                };

            let tag_screen_pos = project_pt(center_pt);
            let mut tags = Vec::new();
            if let (true, Some([sx, sy])) = (state.ui.multiselection_measure_tag, tag_screen_pos) {
                tags.push(MeasureTagModel {
                    text: format!("Ø {:.3}m", avg_distance),
                    x: sx,
                    y: sy - 14.0,
                });
            }

            let label_x = tag_screen_pos.map(|p| p[0]).unwrap_or(width * 0.5);
            let label_y = tag_screen_pos.map(|p| p[1] - 14.0).unwrap_or(height * 0.5);

            return QuickMeasureModel {
                visible: true,
                commands,
                distance: avg_distance,
                dx: span.x,
                dy: span.y,
                dz: span.z,
                angle_deg: 0.0,
                text,
                label_x,
                label_y,
                hud_text,
                tags,
            };
        }
    }

    // Caso 3: exatamente dois vértices selecionados na malha ativa (quando não há arestas selecionadas)
    if state.selection.verts.len() == 2 {
        let Some(mesh) = state.project.active_mesh() else {
            return QuickMeasureModel::default();
        };
        let mut iter = state.selection.verts.iter().copied();
        let (Some(v0), Some(v1)) = (iter.next(), iter.next()) else {
            return QuickMeasureModel::default();
        };
        let idx0 = v0 as usize;
        let idx1 = v1 as usize;
        if idx0 >= mesh.verts.len() || idx1 >= mesh.verts.len() || idx0 == idx1 {
            return QuickMeasureModel::default();
        }
        let p_start = mesh.verts[idx0].vec();
        let p_end = mesh.verts[idx1].vec();
        let delta = p_end - p_start;
        let distance = delta.length();
        if distance < 1e-4 {
            return QuickMeasureModel::default();
        }

        let dx = delta.x.abs();
        let dy = delta.y.abs();
        let dz = delta.z.abs();
        let angle_deg = delta.y.atan2(delta.x).to_degrees().abs();

        let (Some(a), Some(b)) = (project_pt(p_start), project_pt(p_end)) else {
            return QuickMeasureModel::default();
        };
        let Some(([nx, ny], _, tag_pos)) = compute_offset(a, b) else {
            return QuickMeasureModel::default();
        };

        let mut commands = String::new();
        let _ = write!(
            commands,
            "M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} M {:.2} {:.2} L {:.2} {:.2} ",
            a[0] - nx * 6.0,
            a[1] - ny * 6.0,
            a[0] + nx * 6.0,
            a[1] + ny * 6.0,
            b[0] - nx * 6.0,
            b[1] - ny * 6.0,
            b[0] + nx * 6.0,
            b[1] + ny * 6.0,
            a[0],
            a[1],
            b[0],
            b[1],
        );

        let tag_text = format!("{:.3}m", distance);
        let hud_text = format!(
            "{:.3}m  ·  ΔX: {:.3}  ΔY: {:.3}  ΔZ: {:.3}  ·  {:.1}°",
            distance, dx, dy, dz, angle_deg
        );
        let text = format!(
            "{:.3}m  |  ΔX: {:.3}  ΔY: {:.3}  ΔZ: {:.3}  |  {:.1}°",
            distance, dx, dy, dz, angle_deg
        );
        let tags = vec![MeasureTagModel {
            text: tag_text,
            x: tag_pos[0],
            y: tag_pos[1],
        }];

        return QuickMeasureModel {
            visible: true,
            commands,
            distance,
            dx,
            dy,
            dz,
            angle_deg,
            text,
            label_x: tag_pos[0],
            label_y: tag_pos[1],
            hud_text,
            tags,
        };
    }

    QuickMeasureModel::default()
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SnapMarkerModel {
    pub visible: bool,
    pub x: f32,
    pub y: f32,
    /// Rótulo do alvo (TextId traduzido): o tipo nunca é comunicado só por cor.
    pub label: String,
    /// Forma do marcador: quadrado para pontos, círculo para arestas, guias,
    /// faces e grade.
    pub round: bool,
}

pub(crate) fn compute_snap_marker(state: &AppState, width: f32, height: f32) -> SnapMarkerModel {
    let Some(fb) = state.current_tool_feedback() else {
        return SnapMarkerModel::default();
    };
    if !fb.is_snapped {
        return SnapMarkerModel::default();
    }
    snap_marker_at(state, fb.current, fb.snap_kind, width, height)
}

/// Marcador (posição, forma e rótulo) para um ponto encaixado em mundo.
pub(crate) fn snap_marker_at(
    state: &AppState,
    point: glam::Vec3,
    kind: Option<petunia_core::SnapKind>,
    width: f32,
    height: f32,
) -> SnapMarkerModel {
    if width <= 1.0 || height <= 1.0 {
        return SnapMarkerModel::default();
    }
    let view_proj = state.session.camera.view_proj();
    let clip = view_proj * point.extend(1.0);
    if clip.w <= 0.05 {
        return SnapMarkerModel::default();
    }

    let x = (clip.x / clip.w * 0.5 + 0.5) * width;
    let y = (1.0 - (clip.y / clip.w * 0.5 + 0.5)) * height;

    if x >= -20.0 && x <= width + 20.0 && y >= -20.0 && y <= height + 20.0 {
        SnapMarkerModel {
            visible: true,
            x,
            y,
            label: kind
                .map(|kind| state.t_id(kind.text_id()))
                .unwrap_or_default(),
            round: !matches!(
                kind,
                Some(petunia_core::SnapKind::Point | petunia_core::SnapKind::Midpoint)
            ),
        }
    } else {
        SnapMarkerModel::default()
    }
}

/// Resumo legível da seleção (Object Info do Blender): o que está
/// selecionado e quanto. Linha única para a pill da viewport e o inspector.
pub(crate) fn format_selection_summary(state: &AppState) -> String {
    fn plural(count: usize, singular: &str, plural: &str) -> Option<String> {
        if count == 0 {
            None
        } else if count == 1 {
            Some(format!("1 {singular}"))
        } else {
            Some(format!("{count} {plural}"))
        }
    }
    if state.selection_domain() == SelectionDomain::Object {
        // Espelha a regra da UI: ativo conta como selecionado.
        let active_id = state.project.active().map(|a| a.id);
        let total = state
            .project
            .assets
            .iter()
            .filter(|a| state.session.selection.assets.contains(&a.id) || active_id == Some(a.id))
            .count();
        return plural(total, "object selected", "objects selected")
            .unwrap_or_else(|| "No selection".to_string());
    }
    let details = state.query_selection_details();
    let mut parts = Vec::new();
    if let Some(text) = plural(details.selected_verts_count, "point", "points") {
        parts.push(text);
    }
    if let Some(text) = plural(details.selected_edges_count, "edge", "edges") {
        parts.push(text);
    }
    if let Some(text) = plural(details.selected_faces_count, "face", "faces") {
        parts.push(text);
    }
    if parts.is_empty() {
        "No selection".to_string()
    } else {
        format!("Selected: {}", parts.join(" · "))
    }
}

pub(crate) fn compute_selection_overlay(
    state: &AppState,
    width: f32,
    height: f32,
    backend_draws_guides: bool,
) -> SelectionOverlayModel {
    if state.selection_domain() != SelectionDomain::Object {
        return compute_asset_overlay(
            state,
            width,
            height,
            backend_draws_guides,
            state.project.active,
        );
    }
    let mut overlay = SelectionOverlayModel::default();
    for (index, asset) in state.project.assets.iter().enumerate() {
        if !asset.visible {
            continue;
        }
        let is_active = index == state.project.active;
        let selected = state.session.selection.assets.contains(&asset.id) || is_active;
        let hovered = state.session.tools.hover == petunia_core::HoverTarget::Object(index);
        if !selected && !hovered {
            continue;
        }
        let part = compute_asset_overlay(state, width, height, backend_draws_guides, index);
        if is_active {
            overlay
                .active_outline_commands
                .push_str(&part.outline_commands);
        } else if selected {
            overlay.outline_commands.push_str(&part.outline_commands);
        } else {
            overlay
                .unselected_outline_commands
                .push_str(&part.outline_commands);
        }
    }
    overlay.visible = !overlay.outline_commands.is_empty()
        || !overlay.active_outline_commands.is_empty()
        || !overlay.unselected_outline_commands.is_empty();
    overlay
}

pub(crate) fn compute_asset_overlay(
    state: &AppState,
    width: f32,
    height: f32,
    backend_draws_guides: bool,
    index: usize,
) -> SelectionOverlayModel {
    /// Teto de segmentos por frame: malhas grandes não podem gerar uma string
    /// gigante a cada sync de propriedades.
    const MAX_SEGMENTS: usize = 4_000;

    if width <= 1.0 || height <= 1.0 {
        return SelectionOverlayModel::default();
    }
    if state.workspace == Workspace::Paint {
        return SelectionOverlayModel::default();
    }
    let Some(asset) = state.project.assets.get(index) else {
        return SelectionOverlayModel::default();
    };
    let evaluated = asset.evaluated_mesh_ref();
    let mesh = &*evaluated;
    if mesh.verts.is_empty() {
        return SelectionOverlayModel::default();
    }

    let view_proj = state.session.camera.view_proj();
    let project = |point: glam::Vec3| -> Option<[f32; 2]> {
        let clip = view_proj * glam::Vec4::new(point.x, point.y, point.z, 1.0);
        if clip.w <= 0.05 || clip.z < 0.0 || clip.z > clip.w {
            return None;
        }
        let inv_w = 1.0 / clip.w;
        Some([
            (clip.x * inv_w * 0.5 + 0.5) * width,
            (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height,
        ])
    };

    let mut outline = String::new();
    let mut points = String::new();
    let mut unselected_outline = String::new();
    let mut unselected_points = String::new();
    let mut segments = 0usize;
    let push_segment = |commands: &mut String, a: [f32; 2], b: [f32; 2]| {
        commands.push_str(&format!(
            "M {:.2} {:.2} L {:.2} {:.2} ",
            a[0], a[1], b[0], b[1]
        ));
    };
    let push_disc = |commands: &mut String, center: [f32; 2], radius: f32| {
        use std::fmt::Write as _;
        for step in 0..12 {
            let angle = step as f32 * std::f32::consts::TAU / 12.0;
            let action = if step == 0 { 'M' } else { 'L' };
            let _ = write!(
                commands,
                "{action} {:.2} {:.2} ",
                center[0] + radius * angle.cos(),
                center[1] + radius * angle.sin()
            );
        }
        commands.push_str("Z ");
    };

    let domain = state.selection_domain();
    let mut truncated = false;

    match domain {
        SelectionDomain::Object => {
            // Silhueta: aresta entre face frontal e traseira (ou borda aberta
            // frontal). Não desenhar todas as arestas de faces frontais, o que
            // faria um objeto selecionado parecer uma caixa de arame gigante.
            let mut adjacent = std::collections::HashMap::<(u32, u32), (usize, usize)>::new();
            let eye = state.session.camera.eye();
            for (fi, face) in mesh.faces.iter().enumerate() {
                if face.verts.len() < 3 {
                    continue;
                }
                let face_center = face
                    .verts
                    .iter()
                    .filter_map(|&vi| mesh.verts.get(vi as usize))
                    .map(|vertex| vertex.vec())
                    .sum::<glam::Vec3>()
                    / face.verts.len() as f32;
                let front = mesh.face_normal(fi).dot(eye - face_center) > 0.0;
                for index in 0..face.verts.len() {
                    let a = face.verts[index];
                    let b = face.verts[(index + 1) % face.verts.len()];
                    let entry = adjacent.entry((a.min(b), a.max(b))).or_default();
                    if front {
                        entry.0 += 1;
                    } else {
                        entry.1 += 1;
                    }
                }
            }
            for ((a, b), (front, back)) in adjacent {
                if front == 0 || (back == 0 && front > 1) {
                    continue;
                }
                if segments >= MAX_SEGMENTS {
                    truncated = true;
                    break;
                }
                let (Some(va), Some(vb)) = (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                else {
                    continue;
                };
                if let (Some(pa), Some(pb)) = (project(va.vec()), project(vb.vec())) {
                    push_segment(&mut outline, pa, pb);
                    segments += 1;
                }
            }
        }
        SelectionDomain::Vertex => {
            if backend_draws_guides {
                return SelectionOverlayModel::default();
            }
            for (index, vertex) in mesh.verts.iter().enumerate() {
                if segments >= MAX_SEGMENTS {
                    truncated = true;
                    break;
                }
                let Some(sp) = project(vertex.vec()) else {
                    continue;
                };
                let hovered =
                    state.session.tools.hover == petunia_core::HoverTarget::Vertex(index as u32);
                let radius = if hovered {
                    (state.ui.selection_thickness * 2.5).clamp(6.5, 8.5)
                } else if vertex.selected {
                    (state.ui.selection_thickness * 2.0).clamp(5.0, 7.0)
                } else {
                    (state.ui.selection_thickness * 1.5).clamp(3.5, 5.5)
                };
                let target = if hovered || vertex.selected {
                    &mut points
                } else {
                    &mut unselected_points
                };
                push_disc(target, sp, radius);
                segments += 1;
            }
        }
        SelectionDomain::Edge => {
            if backend_draws_guides {
                return SelectionOverlayModel::default();
            }
            for (a, b) in mesh.edges_unique() {
                if segments >= MAX_SEGMENTS {
                    truncated = true;
                    break;
                }
                let selected =
                    mesh.selected_edges.contains(&(a, b)) || mesh.selected_edges.contains(&(b, a));
                if selected {
                    // O GPU desenha a aresta selecionada com depth test.
                    continue;
                }
                let (Some(va), Some(vb)) = (mesh.verts.get(a as usize), mesh.verts.get(b as usize))
                else {
                    continue;
                };
                if let (Some(pa), Some(pb)) = (project(va.vec()), project(vb.vec())) {
                    push_segment(&mut unselected_outline, pa, pb);
                    segments += 1;
                }
            }
        }
        SelectionDomain::Face => {
            // A seleção e o hover de faces são preenchidos pelo renderer.
            // Marcadores centrais em todas as faces poluíam a malha.
        }
    }

    let _ = truncated;
    let visible = !outline.is_empty()
        || !points.is_empty()
        || !unselected_outline.is_empty()
        || !unselected_points.is_empty();
    SelectionOverlayModel {
        visible,
        outline_commands: outline,
        // O roteamento ativo × selecionado acontece no chamador
        // (compute_selection_overlay); aqui nasce sempre vazio.
        active_outline_commands: String::new(),
        point_commands: points,
        unselected_outline_commands: unselected_outline,
        unselected_point_commands: unselected_points,
        accent: domain != SelectionDomain::Object,
    }
}

pub(crate) fn pick_face_hit(
    state: &AppState,
    origin: glam::Vec3,
    direction: glam::Vec3,
) -> Option<(usize, glam::Vec3)> {
    pick_face_hit_cull(state, origin, direction, true)
}

pub(crate) fn pick_face_hit_cull(
    state: &AppState,
    origin: glam::Vec3,
    direction: glam::Vec3,
    cull_backfaces: bool,
) -> Option<(usize, glam::Vec3)> {
    let mesh = state.project.active_mesh()?;
    let mut best = None;
    for (face_index, face) in mesh.faces.iter().enumerate() {
        for corners in mesh.face_triangle_corners(face_index) {
            let [p0, p1, p2] = corners.map(|i| mesh.verts[face.verts[i] as usize].vec());
            if let Some(distance) = ray_triangle_cull(origin, direction, p0, p1, p2, cull_backfaces)
                && best.is_none_or(|(_, current)| distance < current)
            {
                best = Some((face_index, distance));
            }
        }
    }
    best.map(|(face, distance)| (face, origin + direction * distance))
}

#[allow(dead_code)]
pub(crate) fn ray_triangle(
    origin: glam::Vec3,
    direction: glam::Vec3,
    p0: glam::Vec3,
    p1: glam::Vec3,
    p2: glam::Vec3,
) -> Option<f32> {
    ray_triangle_cull(origin, direction, p0, p1, p2, false)
}

pub(crate) fn ray_triangle_cull(
    origin: glam::Vec3,
    direction: glam::Vec3,
    p0: glam::Vec3,
    p1: glam::Vec3,
    p2: glam::Vec3,
    cull_backfaces: bool,
) -> Option<f32> {
    let edge1 = p1 - p0;
    let edge2 = p2 - p0;
    let pvec = direction.cross(edge2);
    let det = edge1.dot(pvec);
    if cull_backfaces {
        if det < 1e-7 {
            return None;
        }
    } else if det.abs() < 1e-7 {
        return None;
    }
    let inv = 1.0 / det;
    let tvec = origin - p0;
    let u = tvec.dot(pvec) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let qvec = tvec.cross(edge1);
    let v = direction.dot(qvec) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let distance = edge2.dot(qvec) * inv;
    (distance > 1e-4).then_some(distance)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProtractorModel {
    pub visible: bool,
    pub wedge_commands: String,
    pub ticks_commands: String,
    pub center_x: f32,
    pub center_y: f32,
    pub angle_degrees: f32,
}

pub(crate) fn compute_protractor(
    state: &AppState,
    width: f32,
    height: f32,
    drag: Option<&ViewportDrag>,
) -> ProtractorModel {
    if width <= 1.0 || height <= 1.0 {
        return ProtractorModel::default();
    }
    let modal = state.session.tools.modal.as_ref().or(state.modal.as_ref());
    let Some(modal) = modal else {
        return ProtractorModel::default();
    };
    if modal.kind != petunia_core::ModalKind::Rotate {
        return ProtractorModel::default();
    }

    let view_proj = state.session.camera.view_proj();
    let clip = view_proj * modal.pivot.extend(1.0);
    if clip.w <= 0.05 {
        return ProtractorModel::default();
    }
    let inv_w = 1.0 / clip.w;
    let cx = (clip.x * inv_w * 0.5 + 0.5) * width;
    let cy = (1.0 - (clip.y * inv_w * 0.5 + 0.5)) * height;

    let theta0 = if let Some(d) = drag {
        let dx = d.start[0] - cx;
        let dy = d.start[1] - cy;
        if dx.hypot(dy) > 5.0 {
            dy.atan2(dx)
        } else {
            0.0
        }
    } else {
        0.0
    };

    let angle_deg = modal.value;
    const R: f32 = 80.0;

    use std::fmt::Write as _;
    let mut wedge_commands = String::new();
    if angle_deg.abs() >= 0.2 {
        // Slint screen coordinates: Y increases downwards.
        // Rotation in standard coordinates is counterclockwise.
        let delta_rad = -angle_deg.to_radians();
        let steps = (angle_deg.abs() / 4.0).ceil().clamp(2.0, 72.0) as usize;
        let _ = write!(wedge_commands, "M {:.2} {:.2} ", cx, cy);
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            let th = theta0 + delta_rad * t;
            let px = cx + R * th.cos();
            let py = cy + R * th.sin();
            let _ = write!(wedge_commands, "L {:.2} {:.2} ", px, py);
        }
        wedge_commands.push('Z');
    } else {
        let px = cx + R * theta0.cos();
        let py = cy + R * theta0.sin();
        let _ = write!(
            wedge_commands,
            "M {:.2} {:.2} L {:.2} {:.2} ",
            cx, cy, px, py
        );
    }

    let mut ticks_commands = String::new();
    // Circular protractor dial ring
    const RING_SEGS: usize = 36;
    for i in 0..RING_SEGS {
        let a1 = i as f32 * std::f32::consts::TAU / (RING_SEGS as f32);
        let a2 = ((i as f32) + 0.6) * std::f32::consts::TAU / (RING_SEGS as f32);
        let x1 = cx + R * a1.cos();
        let y1 = cy + R * a1.sin();
        let x2 = cx + R * a2.cos();
        let y2 = cy + R * a2.sin();
        let _ = write!(
            ticks_commands,
            "M {:.2} {:.2} L {:.2} {:.2} ",
            x1, y1, x2, y2
        );
    }

    // 24 radial tick marks (every 15 degrees, with 45 degrees being major ticks)
    for step in 0..24 {
        let tick_angle = theta0 + (step as f32 * std::f32::consts::TAU / 24.0);
        let is_major = step % 3 == 0;
        let r_in = if is_major { R - 8.0 } else { R - 4.0 };
        let x1 = cx + r_in * tick_angle.cos();
        let y1 = cy + r_in * tick_angle.sin();
        let x2 = cx + R * tick_angle.cos();
        let y2 = cy + R * tick_angle.sin();
        let _ = write!(
            ticks_commands,
            "M {:.2} {:.2} L {:.2} {:.2} ",
            x1, y1, x2, y2
        );
    }

    ProtractorModel {
        visible: true,
        wedge_commands,
        ticks_commands,
        center_x: cx,
        center_y: cy,
        angle_degrees: angle_deg,
    }
}

#[cfg(test)]
mod clipped_path_tests {
    use super::*;
    use glam::Vec3;
    use petunia_core::Camera;

    fn camera() -> Camera {
        Camera::default()
    }

    fn numbers(commands: &str) -> Vec<f32> {
        commands
            .split_whitespace()
            .filter_map(|t| t.parse::<f32>().ok())
            .collect()
    }

    #[test]
    fn closed_ring_in_front_of_the_camera_is_written_whole() {
        let camera = camera();
        let ring = [
            Vec3::new(-0.5, 0.0, -0.5),
            Vec3::new(0.5, 0.0, -0.5),
            Vec3::new(0.5, 0.0, 0.5),
            Vec3::new(-0.5, 0.0, 0.5),
        ];
        let mut out = String::new();
        assert!(write_clipped_path(
            &camera,
            [800.0, 600.0],
            ring,
            true,
            &mut out
        ));
        assert_eq!(out.matches('M').count(), 1);
        assert_eq!(out.matches('L').count(), 3);
        assert!(out.trim_end().ends_with('Z'));
    }

    #[test]
    fn ring_crossing_the_near_plane_is_clipped_not_dropped() {
        let camera = camera();
        let eye = camera.eye();
        let forward = camera.forward();
        let right = camera.right();
        // Grande quadrado que atravessa o plano da câmera: metade na frente, metade atrás.
        let center = eye;
        let ring = [
            center + right * -3.0 + forward * -4.0,
            center + right * 3.0 + forward * -4.0,
            center + right * 3.0 + forward * 6.0,
            center + right * -3.0 + forward * 6.0,
        ];
        let mut out = String::new();
        assert!(write_clipped_path(
            &camera,
            [800.0, 600.0],
            ring,
            true,
            &mut out
        ));
        let values = numbers(&out);
        assert!(values.len() >= 6, "restou polígono visível: {out}");
        assert!(
            values.iter().all(|v| v.is_finite() && v.abs() < 1.0e6),
            "sem coordenadas explodidas por w<0: {out}"
        );
    }

    #[test]
    fn ring_entirely_behind_the_camera_writes_nothing() {
        let camera = camera();
        let eye = camera.eye();
        let forward = camera.forward();
        let ring = [
            eye - forward * 2.0,
            eye - forward * 3.0 + camera.right(),
            eye - forward * 3.0 - camera.right(),
        ];
        let mut out = String::new();
        assert!(!write_clipped_path(
            &camera,
            [800.0, 600.0],
            ring,
            true,
            &mut out
        ));
        assert!(out.is_empty());
    }

    #[test]
    fn open_polyline_restarts_after_a_hidden_stretch() {
        let camera = camera();
        let eye = camera.eye();
        let forward = camera.forward();
        let a = eye + forward * 3.0;
        let behind = eye - forward * 3.0;
        let b = eye + forward * 3.0 + camera.right();
        let mut out = String::new();
        assert!(write_clipped_path(
            &camera,
            [800.0, 600.0],
            [a, behind, b],
            false,
            &mut out
        ));
        assert!(out.matches('M').count() >= 2, "traço interrompido: {out}");
    }
}
