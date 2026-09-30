//! module-uv — workspace UV (§12): projeção planar, seleção sincronizada
//! via eventos e transformações simples de ilhas (mover/escalar).

use petunia_core::{AppEvent, AppState, Module};

#[derive(Default)]
pub struct UvModule {
    /// faces com UV "suja" (precisam re-projeção) — dirty flag simples.
    pub dirty: bool,
}

impl UvModule {
    pub fn new() -> Self {
        Self { dirty: false }
    }

    /// Re-projeta planar o asset ativo (checkpoint + evento).
    pub fn reproject(state: &mut AppState) {
        state.checkpoint("uv planar");
        if let Some(m) = state.project.active_mesh_mut() {
            m.project_planar();
        }
        state.uv_selected.clear();
        state.emit_uv_changed();
    }

    /// Move UVs das faces selecionadas (ou todas se vazio), preservando vértices fixados (pinned).
    pub fn move_selected(state: &mut AppState, du: f32, dv: f32) {
        let sel_empty = state.session.uv_selected.is_empty();
        let uv_selected = &state.session.uv_selected;
        let mut changed = false;
        if let Some(m) = state.project.active_mesh_mut() {
            for (fi, f) in m.faces.iter_mut().enumerate() {
                if sel_empty || uv_selected.contains(&fi) {
                    for (c_idx, uv) in f.uv.iter_mut().enumerate() {
                        if !m.uv_pinned.contains(&(fi, c_idx)) {
                            uv[0] += du;
                            uv[1] += dv;
                        }
                    }
                }
            }
            changed = true;
        }
        if changed {
            state.emit_uv_changed();
        }
    }

    /// Escala UVs selecionadas em torno do centroide, preservando vértices fixados (pinned).
    pub fn scale_selected(state: &mut AppState, s: f32) {
        let sel_empty = state.session.uv_selected.is_empty();
        let uv_selected = &state.session.uv_selected;
        let mut changed = false;
        if let Some(m) = state.project.active_mesh_mut() {
            let mut c = [0.0f32; 2];
            let mut n = 0;
            for (fi, f) in m.faces.iter().enumerate() {
                if sel_empty || uv_selected.contains(&fi) {
                    for uv in &f.uv {
                        c[0] += uv[0];
                        c[1] += uv[1];
                        n += 1;
                    }
                }
            }
            if n == 0 {
                return;
            }
            c[0] /= n as f32;
            c[1] /= n as f32;
            for (fi, f) in m.faces.iter_mut().enumerate() {
                if sel_empty || uv_selected.contains(&fi) {
                    for (c_idx, uv) in f.uv.iter_mut().enumerate() {
                        if !m.uv_pinned.contains(&(fi, c_idx)) {
                            uv[0] = c[0] + (uv[0] - c[0]) * s;
                            uv[1] = c[1] + (uv[1] - c[1]) * s;
                        }
                    }
                }
            }
            changed = true;
        }
        if changed {
            state.emit_uv_changed();
        }
    }

    /// Rotaciona UVs selecionadas em torno do centroide em radianos, preservando vértices fixados (pinned).
    pub fn rotate_selected(state: &mut AppState, angle_rad: f32) {
        let sel_empty = state.session.uv_selected.is_empty();
        let uv_selected = &state.session.uv_selected;
        let (sin_a, cos_a) = angle_rad.sin_cos();
        let mut changed = false;

        if let Some(m) = state.project.active_mesh_mut() {
            let mut c = [0.0f32; 2];
            let mut n = 0;
            for (fi, f) in m.faces.iter().enumerate() {
                if sel_empty || uv_selected.contains(&fi) {
                    for uv in &f.uv {
                        c[0] += uv[0];
                        c[1] += uv[1];
                        n += 1;
                    }
                }
            }
            if n == 0 {
                return;
            }
            c[0] /= n as f32;
            c[1] /= n as f32;

            for (fi, f) in m.faces.iter_mut().enumerate() {
                if sel_empty || uv_selected.contains(&fi) {
                    for (c_idx, uv) in f.uv.iter_mut().enumerate() {
                        if !m.uv_pinned.contains(&(fi, c_idx)) {
                            let du = uv[0] - c[0];
                            let dv = uv[1] - c[1];
                            uv[0] = c[0] + du * cos_a - dv * sin_a;
                            uv[1] = c[1] + du * sin_a + dv * cos_a;
                        }
                    }
                }
            }
            changed = true;
        }
        if changed {
            state.emit_uv_changed();
        }
    }

    /// Fixa (pin) todos os vértices UV das faces selecionadas.
    pub fn pin_selected(state: &mut AppState) {
        state.checkpoint("pin uv");
        if let Some(m) = state.project.active_mesh_mut() {
            m.pin_selected_faces_uv(&state.session.uv_selected);
        }
        state.emit_uv_changed();
    }

    /// Desfixa (unpin) todos os vértices UV das faces selecionadas.
    pub fn unpin_selected(state: &mut AppState) {
        state.checkpoint("unpin uv");
        if let Some(m) = state.project.active_mesh_mut() {
            m.unpin_selected_faces_uv(&state.session.uv_selected);
        }
        state.emit_uv_changed();
    }

    /// Alterna estado de fixação (pin/unpin) das faces selecionadas.
    pub fn toggle_pin_selected(state: &mut AppState) -> bool {
        state.checkpoint("toggle pin uv");
        let res = if let Some(m) = state.project.active_mesh_mut() {
            m.toggle_pin_selected_faces_uv(&state.session.uv_selected)
        } else {
            false
        };
        state.emit_uv_changed();
        res
    }

    /// Limpa todas as fixações de UV da malha ativa.
    pub fn clear_all_pins(state: &mut AppState) {
        state.checkpoint("clear all uv pins");
        if let Some(m) = state.project.active_mesh_mut() {
            m.clear_all_pins();
        }
        state.emit_uv_changed();
    }

    /// Executa projeção cúbica (Box Mapping) sobre o asset ativo (P3D-064).
    pub fn project_cube(state: &mut AppState) {
        state.checkpoint("uv cube projection");
        if let Some(m) = state.project.active_mesh_mut() {
            m.project_cube();
        }
        state.uv_selected.clear();
        state.emit_uv_changed();
    }

    /// Executa unwrap automático genérico usando o provider xatlas (P1-06 / P3D-064).
    pub fn unwrap_auto(state: &mut AppState) -> Result<usize, String> {
        state
            .dispatch(&petunia_core::UnwrapAutoCmd)
            .map_err(|e| e.to_string())?;
        Ok(state
            .project
            .active_mesh()
            .map(|m| m.uv_islands().len())
            .unwrap_or(0))
    }

    pub fn pack_islands(state: &mut AppState, padding: f32) -> Result<usize, String> {
        state
            .dispatch(&petunia_core::UvPackIslandsCmd { padding })
            .map_err(|e| e.to_string())?;
        Ok(state
            .project
            .active_mesh()
            .map(|m| m.uv_islands().len())
            .unwrap_or(0))
    }

    /// Equaliza a densidade de texels entre as ilhas UV (ou ilhas selecionadas).
    pub fn equalize_texel_density(state: &mut AppState) -> Result<usize, String> {
        state.checkpoint("equalize texel density");
        let modified = if let Some(m) = state.project.active_mesh_mut() {
            let count = m.equalize_texel_density(&state.session.uv_selected);
            state.mark_dirty();
            count
        } else {
            0
        };
        state.emit_uv_changed();
        state.set_status(format!("Equalized texel density on {modified} island(s)"));
        Ok(modified)
    }

    pub fn project_from_view(state: &mut AppState) -> Result<(), String> {
        state
            .dispatch(&petunia_core::UvProjectFromViewCmd)
            .map_err(|e| e.to_string())
    }

    pub fn mark_selected_seams(state: &mut AppState) {
        state.checkpoint("mark seam");
        if let Some(m) = state.project.active_mesh_mut() {
            let edges: Vec<(u32, u32)> = m.selected_edges.iter().copied().collect();
            for (a, b) in edges {
                m.mark_seam(a, b);
            }
        }
        state.emit_uv_changed();
    }

    pub fn clear_selected_seams(state: &mut AppState) {
        state.checkpoint("clear seam");
        if let Some(m) = state.project.active_mesh_mut() {
            let edges: Vec<(u32, u32)> = m.selected_edges.iter().copied().collect();
            for (a, b) in edges {
                m.clear_seam(a, b);
            }
        }
        state.emit_uv_changed();
    }

    pub fn diagnostics(state: &AppState) -> Option<petunia_mesh::uv_tools::UvDiagnostics> {
        state.project.active_mesh().map(|m| m.uv_diagnostics())
    }

    pub fn texel_density(state: &AppState, texture_w: u32) -> f32 {
        state
            .project
            .active_mesh()
            .map(|m| m.texel_density(texture_w, false))
            .unwrap_or(0.0)
    }

    pub fn normalize_texel_density(state: &mut AppState, texture_w: u32, target: f32) {
        state.checkpoint("normalize texel density");
        if let Some(m) = state.project.active_mesh_mut() {
            m.normalize_texel_density(texture_w, target);
        }
        state.emit_uv_changed();
    }
}

impl Module for UvModule {
    fn id(&self) -> &'static str {
        "uv"
    }
    fn on_event(&mut self, event: &AppEvent, state: &mut AppState) {
        match event {
            AppEvent::MeshChanged { .. } | AppEvent::ActiveAssetChanged { .. } => {
                // seleção UV pode ter morrido com a malha — limpa órfãs
                if let Some(o) = state.project.assets.get(state.project.active) {
                    let max_len = o.mesh.faces.len();
                    state.session.uv_selected.retain(|&fi| fi < max_len);
                }
                self.dirty = true;
            }
            AppEvent::SelectionChanged(sel) => {
                // espelha seleção de faces do viewport no editor UV
                state.uv_selected = sel.faces.iter().copied().collect();
            }
            _ => {}
        }
    }

    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn std::any::Any + 'static) {
        self
    }
}

impl UvModule {
    pub fn uv_hit(state: &AppState, u: f32, v: f32) -> Option<usize> {
        uv_hit(state, u, v)
    }
}

/// Face cuja ilha contém (u,v). Testa tris do polígono UV.
pub fn uv_hit(state: &AppState, u: f32, v: f32) -> Option<usize> {
    let o = state.project.assets.get(state.project.active)?;
    for (fi, f) in o.mesh.faces.iter().enumerate() {
        if f.uv.len() < 3 {
            continue;
        }
        // A ilha UV pode ser côncava mesmo quando a face 3D não é. O hit test
        // precisa respeitar o polígono UV, sem selecionar seu espaço vazio.
        let Ok(triangles) = petunia_mesh::triangulate::ear_clip(&f.uv) else {
            continue;
        };
        for [a, b, c] in triangles {
            if point_in_tri_uv([u, v], f.uv[a], f.uv[b], f.uv[c]) {
                return Some(fi);
            }
        }
    }
    None
}

fn point_in_tri_uv(p: [f32; 2], a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> bool {
    let v0 = [c[0] - a[0], c[1] - a[1]];
    let v1 = [b[0] - a[0], b[1] - a[1]];
    let v2 = [p[0] - a[0], p[1] - a[1]];
    let (d00, d01, d11, d20, d21) = (
        v0[0] * v0[0] + v0[1] * v0[1],
        v0[0] * v1[0] + v0[1] * v1[1],
        v1[0] * v1[0] + v1[1] * v1[1],
        v2[0] * v0[0] + v2[1] * v0[1],
        v2[0] * v1[0] + v2[1] * v1[1],
    );
    let den = d00 * d11 - d01 * d01;
    if den.abs() < 1e-12 {
        return false;
    }
    let v = (d11 * d20 - d01 * d21) / den;
    let w = (d00 * d21 - d01 * d20) / den;
    v >= -1e-6 && w >= -1e-6 && v + w <= 1.0 + 1e-6
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uv_hit_respects_a_concave_island() {
        use petunia_mesh::{Face, Vertex};

        let mut state = AppState::new("en");
        let mesh = state.project.active_mesh_mut().expect("active mesh");
        mesh.verts = (0..5).map(|x| Vertex::new(x as f32, 0.0, 0.0)).collect();
        mesh.faces = vec![Face::with_uv(
            vec![0, 1, 2, 3, 4],
            vec![
                [0.0, 0.0],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.5, 1.0 / 3.0],
                [0.0, 1.0],
            ],
        )];

        assert_eq!(uv_hit(&state, 0.25, 0.1), Some(0));
        assert_eq!(uv_hit(&state, 0.5, 0.5), None);
    }

    #[test]
    fn test_uv_planar_and_cube_projections() {
        let mut state = AppState::new("en");
        UvModule::reproject(&mut state);
        let mesh = state.project.active_mesh().unwrap();
        assert!(mesh.faces.iter().all(|f| f.uv.len() == f.verts.len()));

        UvModule::project_cube(&mut state);
        let mesh = state.project.active_mesh().unwrap();
        assert!(
            mesh.faces
                .iter()
                .all(|f| f.uv.iter().all(|uv| uv[0].is_finite() && uv[1].is_finite()))
        );
    }

    #[test]
    fn test_uv_transforms() {
        let mut state = AppState::new("en");
        UvModule::reproject(&mut state);

        let original_uv0 = state.project.active_mesh().unwrap().faces[0].uv[0];

        // Move
        UvModule::move_selected(&mut state, 0.2, 0.1);
        let moved_uv0 = state.project.active_mesh().unwrap().faces[0].uv[0];
        assert!((moved_uv0[0] - (original_uv0[0] + 0.2)).abs() < 1e-4);
        assert!((moved_uv0[1] - (original_uv0[1] + 0.1)).abs() < 1e-4);

        // Scale
        UvModule::scale_selected(&mut state, 2.0);

        // Rotate
        UvModule::rotate_selected(&mut state, std::f32::consts::FRAC_PI_2);
        let rotated_mesh = state.project.active_mesh().unwrap();
        assert!(
            rotated_mesh
                .faces
                .iter()
                .all(|f| f.uv.iter().all(|uv| uv[0].is_finite() && uv[1].is_finite()))
        );
    }

    #[test]
    fn test_uv_auto_unwrap_xatlas() {
        let mut state = AppState::new("en");
        let res = UvModule::unwrap_auto(&mut state);
        assert!(res.is_ok());
        let charts = res.unwrap();
        assert!(charts >= 1);
        let mesh = state.project.active_mesh().unwrap();
        assert!(
            mesh.faces
                .iter()
                .all(|f| f.uv.iter().all(|uv| uv[0].is_finite() && uv[1].is_finite()))
        );
    }

    #[test]
    fn test_uv_pinning_in_transforms() {
        let mut state = AppState::new("en");
        let active = state.project.active;
        state.project.assets[active].mesh = petunia_mesh::Mesh::cube(2.0);
        UvModule::project_cube(&mut state);

        let uv0_orig = state.project.active_mesh().unwrap().faces[0].uv[0];
        let uv1_orig = state.project.active_mesh().unwrap().faces[0].uv[1];

        // Fixa apenas o canto (0, 0)
        state.project.active_mesh_mut().unwrap().pin_uv(0, 0);

        // Move todas as UVs da malha em (0.5, 0.5)
        UvModule::move_selected(&mut state, 0.5, 0.5);

        let mesh = state.project.active_mesh().unwrap();
        // Canto fixado não moveu
        assert_eq!(mesh.faces[0].uv[0], uv0_orig);
        // Canto não fixado moveu
        assert!((mesh.faces[0].uv[1][0] - (uv1_orig[0] + 0.5)).abs() < 1e-4);
        assert!((mesh.faces[0].uv[1][1] - (uv1_orig[1] + 0.5)).abs() < 1e-4);

        // Teste de pin_selected e clear_all_pins
        state.session.uv_selected.insert(1);
        UvModule::pin_selected(&mut state);
        assert!(state.project.active_mesh().unwrap().is_uv_pinned(1, 0));

        UvModule::clear_all_pins(&mut state);
        assert_eq!(state.project.active_mesh().unwrap().uv_pinned.len(), 0);
    }
}
