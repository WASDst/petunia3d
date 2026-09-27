//! Draw Profile (§9) — desenha silhueta 2D sobre referência/vista ortográfica,
//! triangula (ear clipping) e gera malha por extrusão ou revolve.
//!
//! Fluxo: ative numa vista ortográfica → cliques adicionam pontos → clique
//! perto do 1º ponto (ou botão) fecha → Gerar (extrude/revolve).

use petunia_core::{AppState, ProfileState};
use petunia_mesh::Mesh;

use super::Tool;

/// Modo interativo de geração de volume a partir do perfil 2D.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileVolumeMode {
    Extrude,
    Revolve,
    Sweep,
}

/// Converte cursor NDC em coords 2D do plano do perfil (com snap opcional).
pub fn profile_screen_to_plane(state: &AppState, nx: f32, ny: f32) -> Option<[f32; 2]> {
    let (origin, dir) = state.session.camera.ray(nx, ny);
    let p = &state.profile;
    let n = glam::Vec3::from(p.normal);
    let denom = dir.dot(n);
    if denom.abs() < 1e-6 {
        return None; // raio paralelo ao plano
    }
    let o = glam::Vec3::from(p.origin);
    let t = (o - origin).dot(n) / denom;
    if t < 0.0 {
        return None;
    }
    let hit = origin + dir * t;
    let d = hit - o;
    let mut x = d.dot(glam::Vec3::from(p.right));
    let mut y = d.dot(glam::Vec3::from(p.up));
    if p.snap {
        x = (x * 4.0).round() / 4.0;
        y = (y * 4.0).round() / 4.0;
    }
    Some([x, y])
}

/// Adiciona ponto (clique no viewport). Fecha se perto do primeiro.
pub fn profile_add_point(state: &mut AppState, nx: f32, ny: f32) {
    if state.profile.closed {
        return;
    }
    let Some([x, y]) = profile_screen_to_plane(state, nx, ny) else {
        return;
    };
    let n = state.profile.points.len();
    if n >= 3 {
        let f = state.profile.points[0];
        if (f[0] - x).hypot(f[1] - y) < 0.25 {
            state.profile.closed = true;
            state.set_status(state.t("profile.closed"));
            state.mark_dirty();
            return;
        }
    }
    if state.profile.points.len() >= 512 {
        state.set_status("max 512 pts".to_string());
        return;
    }
    state.profile.points.push([x, y]);
    state
        .profile
        .nodes
        .push(petunia_mesh::curve::BezierNode::new([x, y]));
    state.mark_dirty();
}

/// Inicia a inserção de um nó que pode se tornar Bézier se houver arraste contínuo.
pub fn profile_begin_drag_node(state: &mut AppState, nx: f32, ny: f32) -> bool {
    if state.profile.closed {
        return false;
    }
    let Some([x, y]) = profile_screen_to_plane(state, nx, ny) else {
        return false;
    };
    let n = state.profile.points.len();
    if n >= 3 {
        let f = state.profile.points[0];
        if (f[0] - x).hypot(f[1] - y) < 0.25 {
            state.profile.closed = true;
            state.set_status(state.t("profile.closed"));
            state.mark_dirty();
            return true;
        }
    }
    if state.profile.points.len() >= 512 {
        state.set_status("max 512 pts".to_string());
        return false;
    }
    state.profile.points.push([x, y]);
    state
        .profile
        .nodes
        .push(petunia_mesh::curve::BezierNode::new([x, y]));
    state.mark_dirty();
    true
}

/// Atualiza as alças tangentes do último nó durante o arraste do ponteiro.
/// `break_tangent` (Alt) quebra a simetria criando uma quina (Sharp) com alça assimétrica.
pub fn profile_update_drag_handle(
    state: &mut AppState,
    nx: f32,
    ny: f32,
    break_tangent: bool,
) -> bool {
    if state.profile.closed || state.profile.nodes.is_empty() {
        return false;
    }
    let Some([curr_x, curr_y]) = profile_screen_to_plane(state, nx, ny) else {
        return false;
    };
    let last_idx = state.profile.nodes.len() - 1;
    let anchor = state.profile.nodes[last_idx].point;
    let dx = curr_x - anchor[0];
    let dy = curr_y - anchor[1];
    if dx.hypot(dy) > 0.02 {
        let node = &mut state.profile.nodes[last_idx];
        node.kind = if break_tangent {
            petunia_mesh::curve::BezierNodeKind::Sharp
        } else {
            petunia_mesh::curve::BezierNodeKind::Symmetric
        };
        node.handle_out = Some([dx, dy]);
        node.handle_in = if break_tangent {
            None
        } else {
            Some([-dx, -dy])
        };
        state.mark_dirty();
        return true;
    }
    false
}

/// Alinha a câmera ortogonalmente e perpendicular ao plano de trabalho do perfil.
pub fn profile_align_camera_to_workplane(state: &mut AppState) {
    let normal = glam::Vec3::from(state.profile.normal).normalize_or_zero();
    let origin = glam::Vec3::from(state.profile.origin);
    state.session.camera.target = origin;
    state
        .session
        .camera
        .set_projection(petunia_core::Projection::Ortho);

    // Se a normal for próxima de +Y (Ground)
    if (normal - glam::Vec3::Y).length_squared() < 1e-3 {
        state
            .session
            .camera
            .set_preset(petunia_core::ViewPreset::Top);
    } else if (normal + glam::Vec3::Y).length_squared() < 1e-3 {
        state
            .session
            .camera
            .set_preset(petunia_core::ViewPreset::Bottom);
    } else if (normal - glam::Vec3::Z).length_squared() < 1e-3 {
        state
            .session
            .camera
            .set_preset(petunia_core::ViewPreset::Front);
    } else if (normal + glam::Vec3::Z).length_squared() < 1e-3 {
        state
            .session
            .camera
            .set_preset(petunia_core::ViewPreset::Back);
    } else if (normal - glam::Vec3::X).length_squared() < 1e-3 {
        state
            .session
            .camera
            .set_preset(petunia_core::ViewPreset::Right);
    } else if (normal + glam::Vec3::X).length_squared() < 1e-3 {
        state
            .session
            .camera
            .set_preset(petunia_core::ViewPreset::Left);
    } else if normal.length_squared() > 1e-4 {
        let pitch = normal.y.clamp(-1.0, 1.0).asin();
        let yaw = normal.x.atan2(normal.z);
        state.session.camera.pitch = pitch;
        state.session.camera.yaw = yaw;
    }
    state.camera = state.session.camera.clone();
}

/// Captura o frame 2D alinhado ao plano do chão (XZ, normal +Y) centrado no 3D Cursor.
pub fn profile_capture_ground(state: &mut AppState) {
    state.profile.right = [1.0, 0.0, 0.0];
    state.profile.up = [0.0, 0.0, -1.0];
    state.profile.normal = [0.0, 1.0, 0.0];
    state.profile.origin = state.session.cursor_3d;
    state.profile.points.clear();
    state.profile.nodes.clear();
    state.profile.closed = false;
    state.mark_dirty();
}

/// Captura o frame 2D alinhado à face selecionada da malha ativa (ou chão se nenhuma selecionada).
pub fn profile_capture_face(state: &mut AppState) -> bool {
    let Some(mesh) = state.project.active_mesh() else {
        profile_capture_ground(state);
        return false;
    };
    let Some(face) = mesh.faces.iter().find(|f| f.selected) else {
        profile_capture_ground(state);
        return false;
    };
    if face.verts.len() < 3 {
        profile_capture_ground(state);
        return false;
    }
    let p0 = mesh.verts[face.verts[0] as usize].vec();
    let p1 = mesh.verts[face.verts[1] as usize].vec();
    let p2 = mesh.verts[face.verts[2] as usize].vec();
    let mut center = glam::Vec3::ZERO;
    for &idx in &face.verts {
        center += mesh.verts[idx as usize].vec();
    }
    center /= face.verts.len() as f32;

    let normal = (p1 - p0).cross(p2 - p0).normalize_or_zero();
    let normal = if normal.length_squared() < 1e-4 {
        glam::Vec3::Y
    } else {
        normal
    };

    let right = (p1 - p0).normalize_or_zero();
    let right = if right.length_squared() < 1e-4 || right.abs_diff_eq(normal, 0.1) {
        if normal.y.abs() < 0.9 {
            glam::Vec3::Y.cross(normal).normalize_or_zero()
        } else {
            glam::Vec3::X.cross(normal).normalize_or_zero()
        }
    } else {
        right
    };
    let up = normal.cross(right).normalize_or_zero();

    state.profile.right = right.to_array();
    state.profile.up = up.to_array();
    state.profile.normal = normal.to_array();
    state.profile.origin = center.to_array();
    state.profile.points.clear();
    state.profile.nodes.clear();
    state.profile.closed = false;
    state.mark_dirty();
    true
}

/// Captura o frame 2D da câmera atual centrado no 3D Cursor.
pub fn profile_capture_view(state: &mut AppState) {
    let right = state.session.camera.right().to_array();
    let up = state.session.camera.up().to_array();
    let origin = state.session.cursor_3d;
    let normal = (-state.session.camera.forward()).to_array();
    state.profile.right = right;
    state.profile.up = up;
    state.profile.origin = origin;
    state.profile.normal = normal;
    state.profile.points.clear();
    state.profile.nodes.clear();
    state.profile.closed = false;
    state.mark_dirty();
}

/// Captura automaticamente o melhor workplane para o contexto atual:
/// 1. Se houver face selecionada -> Face
/// 2. Se a câmera estiver alinhada com eixo ortogonal -> View
/// 3. Caso contrário (perspectiva geral) -> Ground (XZ)
pub fn profile_capture_auto(state: &mut AppState) {
    let has_selected_face = state
        .project
        .active_mesh()
        .map(|m| m.faces.iter().any(|f| f.selected))
        .unwrap_or(false);
    if has_selected_face {
        profile_capture_face(state);
        return;
    }
    let fwd = state.session.camera.forward();
    if fwd.x.abs() > 0.95 || fwd.y.abs() > 0.95 || fwd.z.abs() > 0.95 {
        profile_capture_view(state);
    } else {
        profile_capture_ground(state);
    }
}

/// Alias para manter compatibilidade com chamadas existentes.
pub fn profile_capture_frame(state: &mut AppState) {
    profile_capture_auto(state);
}

/// Rótulo descritivo do workplane atualmente configurado no ProfileState.
pub fn profile_workplane_label(state: &AppState) -> &'static str {
    let n = glam::Vec3::from(state.profile.normal);
    if (n - glam::Vec3::Y).length_squared() < 0.01 {
        "Ground"
    } else {
        let has_selected_face = state
            .project
            .active_mesh()
            .map(|m| m.faces.iter().any(|f| f.selected))
            .unwrap_or(false);
        if has_selected_face { "Face" } else { "View" }
    }
}

/// Constrói a malha 3D de extrusão a partir do ProfileState sem alterar o projeto ou descartar pontos.
pub fn build_extrude_mesh(p: &ProfileState) -> Result<Mesh, String> {
    let effective = p.effective_points();
    if effective.len() < 3 || !p.closed {
        return Err("profile requires at least 3 points and must be closed".to_string());
    }
    let mut m = Mesh::from_polygon(&effective, p.depth.max(0.05)).map_err(|e| e.to_string())?;
    let r = glam::Vec3::from(p.right);
    let u = glam::Vec3::from(p.up);
    let o = glam::Vec3::from(p.origin);
    let n = glam::Vec3::from(p.normal);
    for v in &mut m.verts {
        let q = glam::Vec3::from(v.pos);
        v.pos = (o + r * q.x + u * q.y + n * q.z).to_array();
    }
    Ok(m)
}

pub fn generate_extrude(state: &mut AppState) {
    match build_extrude_mesh(&state.profile) {
        Ok(m) => {
            state.checkpoint("draw profile");
            state.project.add("Profile", m);
            state.profile.clear();
            state.sync_selection();
            state.emit_mesh_changed();
            state.set_status(state.t("profile.generated"));
        }
        Err(e) => state.set_status(format!("profile: {e}")),
    }
}

/// Constrói a malha 3D de revolução a partir do ProfileState sem alterar o projeto ou descartar pontos.
pub fn build_revolve_mesh(p: &ProfileState) -> Result<Mesh, String> {
    let effective = p.effective_points();
    if effective.len() < 2 {
        return Err("profile requires at least 2 points for revolve".to_string());
    }
    let angle = if p.revolve_angle <= 0.0 {
        360.0
    } else {
        p.revolve_angle
    };
    let mut m = Mesh::revolve_angle(&effective, p.revolve_segments.max(3), angle)
        .map_err(|e| e.to_string())?;
    let r = glam::Vec3::from(p.right);
    let u = glam::Vec3::from(p.up);
    let n = glam::Vec3::from(p.normal);
    let o = glam::Vec3::from(p.origin);
    for v in &mut m.verts {
        let q = glam::Vec3::from(v.pos);
        v.pos = (o + r * q.x + u * q.y + n * q.z).to_array();
    }
    Ok(m)
}

pub fn generate_revolve(state: &mut AppState) {
    match build_revolve_mesh(&state.profile) {
        Ok(m) => {
            state.checkpoint("revolve profile");
            state.project.add("Revolved", m);
            state.profile.clear();
            state.sync_selection();
            state.emit_mesh_changed();
            state.set_status(state.t("profile.generated"));
        }
        Err(e) => state.set_status(format!("revolve: {e}")),
    }
}

/// Extrai caminho guia 3D a partir da malha ativa (arestas selecionadas em cadeia ou vértices selecionados).
pub fn extract_sweep_path_from_mesh(mesh: &Mesh) -> Option<(Vec<glam::Vec3>, bool)> {
    if !mesh.selected_edges.is_empty() {
        use std::collections::HashMap;
        let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
        for &(a, b) in &mesh.selected_edges {
            adj.entry(a).or_default().push(b);
            adj.entry(b).or_default().push(a);
        }

        let start = adj
            .iter()
            .find(|(_, neighbors)| neighbors.len() == 1)
            .map(|(&v, _)| v)
            .or_else(|| adj.keys().copied().next())?;

        let mut chain = vec![start];
        let mut visited_edges = std::collections::HashSet::new();
        let mut curr = start;

        loop {
            let next_opt = adj.get(&curr).and_then(|neighbors| {
                neighbors.iter().copied().find(|&n| {
                    let key = petunia_mesh::edge_key(curr, n);
                    !visited_edges.contains(&key)
                })
            });

            if let Some(next) = next_opt {
                visited_edges.insert(petunia_mesh::edge_key(curr, next));
                chain.push(next);
                curr = next;
                if curr == start {
                    break;
                }
            } else {
                break;
            }
        }

        if chain.len() >= 2 {
            let is_closed = chain.len() > 2 && chain.first() == chain.last();
            let pts: Vec<glam::Vec3> = chain
                .into_iter()
                .filter_map(|idx| mesh.verts.get(idx as usize).map(|v| v.vec()))
                .collect();
            if pts.len() >= 2 {
                return Some((pts, is_closed));
            }
        }
    }

    let selected_verts: Vec<glam::Vec3> = mesh
        .verts
        .iter()
        .filter(|v| v.selected)
        .map(|v| v.vec())
        .collect();

    if selected_verts.len() >= 2 {
        Some((selected_verts, false))
    } else {
        None
    }
}

/// Constrói a malha 3D de sweep a partir do ProfileState sem alterar o projeto ou descartar pontos.
pub fn build_sweep_mesh(p: &ProfileState, mesh_guide: Option<&Mesh>) -> Result<Mesh, String> {
    let effective = p.effective_points();
    if effective.len() < 2 {
        return Err("profile requires at least 2 points for sweep".to_string());
    }

    let extracted = mesh_guide.and_then(extract_sweep_path_from_mesh);
    let (path, closed_path) = if let Some((path_pts, is_closed)) = extracted {
        (path_pts, is_closed)
    } else {
        let r = glam::Vec3::from(p.right).normalize_or_zero();
        let u = glam::Vec3::from(p.up).normalize_or_zero();
        let n = glam::Vec3::from(p.normal).normalize_or_zero();
        let o = glam::Vec3::from(p.origin);
        let l = p.depth.max(1.0);
        let default_path = vec![
            o,
            o + n * (l * 0.3) + u * (l * 0.15),
            o + n * (l * 0.7) + u * (l * 0.2) + r * (l * 0.15),
            o + n * l + r * (l * 0.3),
        ];
        (default_path, false)
    };

    let options = petunia_mesh::sweep::SweepOptions {
        closed_path,
        closed_profile: p.closed,
        cap_start: true,
        cap_end: true,
        miter: true,
        miter_limit: 3.0,
    };

    Mesh::from_sweep(&effective, &path, options).map_err(|e| e.to_string())
}

/// Gera uma malha por varredura 3D (Sweep) do perfil 2D ao longo de um caminho guia 3D.
pub fn generate_sweep(state: &mut AppState) {
    let guide = state.project.active_mesh().cloned();
    match build_sweep_mesh(&state.profile, guide.as_ref()) {
        Ok(m) => {
            state.checkpoint("sweep profile");
            state.project.add("Sweep", m);
            state.profile.clear();
            state.sync_selection();
            state.emit_mesh_changed();
            state.set_status(state.t("profile.generated"));
        }
        Err(e) => state.set_status(format!("sweep: {e}")),
    }
}

/// Define um perfil 2D retangular centralizado no plano ativo.
pub fn profile_set_rectangle(state: &mut AppState, width: f32, height: f32) {
    if state.profile.points.is_empty() {
        profile_capture_auto(state);
    }
    let w = if width.is_finite() && width > 0.0 {
        width
    } else {
        2.0
    };
    let h = if height.is_finite() && height > 0.0 {
        height
    } else {
        1.5
    };
    state.profile.points = vec![
        [-w * 0.5, -h * 0.5],
        [w * 0.5, -h * 0.5],
        [w * 0.5, h * 0.5],
        [-w * 0.5, h * 0.5],
    ];
    state.profile.nodes = state
        .profile
        .points
        .iter()
        .map(|&p| petunia_mesh::curve::BezierNode::new(p))
        .collect();
    state.profile.closed = true;
    state.session.tools.active_tool = "draw_profile".to_string();
    state.set_status(format!(
        "Shape: Rectangle 2D ({:.1} x {:.1}) on {}: drag Depth or press Enter to create volume",
        w,
        h,
        profile_workplane_label(state)
    ));
    state.mark_dirty();
}

/// Define um perfil 2D circular centralizado no plano ativo.
pub fn profile_set_circle(state: &mut AppState, radius: f32, segments: usize) {
    if state.profile.points.is_empty() {
        profile_capture_auto(state);
    }
    let r = if radius.is_finite() && radius > 0.0 {
        radius
    } else {
        1.0
    };
    let segs = segments.clamp(6, 64);
    let mut points = Vec::with_capacity(segs);
    for i in 0..segs {
        let angle = std::f32::consts::TAU * (i as f32) / (segs as f32);
        points.push([r * angle.cos(), r * angle.sin()]);
    }
    state.profile.points = points.clone();
    state.profile.nodes = points
        .into_iter()
        .map(petunia_mesh::curve::BezierNode::new)
        .collect();
    state.profile.closed = true;
    state.session.tools.active_tool = "draw_profile".to_string();
    state.set_status(format!(
        "Shape: Circle 2D (r={:.1}, {} segs) on {}: drag Depth or press Enter to create volume",
        r,
        segs,
        profile_workplane_label(state)
    ));
    state.mark_dirty();
}

/// Converte todos os nós do perfil atual em curvas Bézier suaves (G1/C1 contínuas).
pub fn profile_smooth_curves(state: &mut AppState) {
    if state.profile.nodes.is_empty() && !state.profile.points.is_empty() {
        state.profile.nodes = state
            .profile
            .points
            .iter()
            .map(|&p| petunia_mesh::curve::BezierNode::new(p))
            .collect();
    }
    let mut path = petunia_mesh::curve::BezierPath {
        nodes: state.profile.nodes.clone(),
        closed: state.profile.closed,
    };
    path.auto_smooth(0.25);
    state.profile.nodes = path.nodes;
    state.mark_dirty();
    state.set_status("Profile curves smoothed (Cubic Bézier)".to_string());
}

/// Converte todos os nós do perfil atual em cantos retos (Sharp).
pub fn profile_clear_curves(state: &mut AppState) {
    for node in &mut state.profile.nodes {
        node.handle_in = None;
        node.handle_out = None;
        node.kind = petunia_mesh::curve::BezierNodeKind::Sharp;
    }
    state.mark_dirty();
    state.set_status("Profile corners sharpened".to_string());
}

/// Ajusta a espessura de parede (Wall Thickness) para perfis ocos.
pub fn profile_set_wall_thickness(state: &mut AppState, thickness: f32) {
    state.profile.wall_thickness = thickness.max(0.0);
    state.mark_dirty();
}

/// Ajusta a tolerância de suavização de tesselação das curvas Bézier.
pub fn profile_set_curve_smoothness(state: &mut AppState, smoothness: f32) {
    state.profile.curve_smoothness = smoothness.clamp(0.005, 0.2);
    state.mark_dirty();
}

#[derive(Default)]
pub struct DrawProfileTool;

impl Tool for DrawProfileTool {
    fn id(&self) -> &'static str {
        "draw_profile"
    }
    fn label_key(&self) -> &'static str {
        "tools.draw_profile"
    }
    fn hint_key(&self) -> &'static str {
        "hints.draw_profile"
    }
    fn icon(&self) -> &'static str {
        "✎"
    }
    fn shortcut(&self) -> &'static str {
        "Shift+P"
    }
    fn on_activate(&self, state: &mut AppState) {
        profile_capture_frame(state);
        state.set_status(state.t("hints.draw_profile"));
        state.mark_dirty();
    }
}
