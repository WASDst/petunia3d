//! module-uv — workspace UV (§12; website/docs/07-uv/architecture-decision.md).
//!
//! Camada fina sobre a Application: algoritmos vivem em `petunia_mesh`, a
//! seleção UV (cantos, modo, Sync explícito) em `petunia_core::UvSelectionState`
//! e as mutações discretas em comandos com Undo. Continuous gestures use the
//! explicit core UV session; compatibility helpers never bypass the dispatcher.

use petunia_core::{
    AppEvent, AppState, Module, PinOp, SeamOp, UnwrapAutoCmd, UvEqualizeTexelDensityCmd,
    UvNormalizeTexelDensityCmd, UvPackIslandsCmd, UvPinCmd, UvProjectCubeCmd, UvProjectFromViewCmd,
    UvProjectPlanarCmd, UvSeamCmd,
};
use petunia_mesh::UvTransform;

#[derive(Default)]
pub struct UvModule {
    /// UV mudou desde a última leitura do workspace (derivados a recalcular).
    pub dirty: bool,
}

impl UvModule {
    pub fn new() -> Self {
        Self { dirty: false }
    }

    /// Re-projeta planar o asset ativo (checkpoint + evento).
    pub fn reproject(state: &mut AppState) {
        if let Err(err) = state.dispatch(&UvProjectPlanarCmd) {
            state.set_status(format!("uv planar: {err}"));
        }
    }

    /// Compatibility entry point: update an explicit UV preview when active,
    /// otherwise dispatch one discrete command with Undo. Never mutate the mesh directly.
    pub fn transform_selected(state: &mut AppState, transform: UvTransform) -> usize {
        let corners = state.uv_selection.corner_set();
        let count = state.project.active_mesh().map_or(0, |mesh| {
            corners
                .iter()
                .filter(|corner| {
                    mesh.faces
                        .get(corner.face)
                        .and_then(|face| face.uv.get(corner.corner))
                        .is_some()
                })
                .count()
        });
        if count == 0 {
            return 0;
        }
        let result = if state.session.tools.uv_transform.is_some() {
            state.update_uv_transform(transform)
        } else {
            state.dispatch(&petunia_core::UvTransformCmd { transform })
        };
        match result {
            Ok(()) => count,
            Err(error) => {
                state.set_status(error.to_string());
                0
            }
        }
    }

    /// Move selected UV corners through the preview/command boundary.
    pub fn move_selected(state: &mut AppState, du: f32, dv: f32) {
        Self::transform_selected(state, UvTransform::Translate(glam::Vec2::new(du, dv)));
    }

    /// Escala os cantos UV selecionados em torno do centroide deles.
    pub fn scale_selected(state: &mut AppState, s: f32) {
        Self::transform_selected(state, UvTransform::Scale(glam::Vec2::splat(s)));
    }

    /// Rotaciona os cantos UV selecionados em torno do centroide (radianos).
    pub fn rotate_selected(state: &mut AppState, angle_rad: f32) {
        Self::transform_selected(state, UvTransform::Rotate(angle_rad));
    }

    /// Faces totalmente selecionadas no UV Editor.
    pub fn selected_faces(state: &AppState) -> std::collections::HashSet<usize> {
        state
            .project
            .active_mesh()
            .map(|m| state.uv_selection.faces(m))
            .unwrap_or_default()
    }

    /// Fixa (pin) todos os cantos das faces selecionadas no UV.
    pub fn pin_selected(state: &mut AppState) {
        if let Err(err) = state.dispatch(&UvPinCmd {
            op: PinOp::Pin,
            corners: None,
        }) {
            state.set_status(format!("pin uv: {err}"));
        }
    }

    /// Desfixa (unpin) todos os cantos das faces selecionadas no UV.
    pub fn unpin_selected(state: &mut AppState) {
        if let Err(err) = state.dispatch(&UvPinCmd {
            op: PinOp::Unpin,
            corners: None,
        }) {
            state.set_status(format!("unpin uv: {err}"));
        }
    }

    /// Alterna estado de fixação (pin/unpin) das faces selecionadas.
    pub fn toggle_pin_selected(state: &mut AppState) -> bool {
        let faces = Self::selected_faces(state);
        if state
            .dispatch(&UvPinCmd {
                op: PinOp::Toggle,
                corners: None,
            })
            .is_err()
        {
            return false;
        }
        state.project.active_mesh().is_some_and(|m| {
            faces.iter().any(|&fi| {
                m.faces
                    .get(fi)
                    .is_some_and(|f| (0..f.uv.len()).any(|ci| m.is_uv_pinned(fi, ci)))
            })
        })
    }

    /// Limpa todas as fixações de UV da malha ativa.
    pub fn clear_all_pins(state: &mut AppState) {
        if let Err(err) = state.dispatch(&UvPinCmd {
            op: PinOp::ClearAll,
            corners: None,
        }) {
            state.set_status(format!("clear all uv pins: {err}"));
        }
    }

    /// Executa projeção cúbica (Box Mapping) sobre o asset ativo (P3D-064).
    pub fn project_cube(state: &mut AppState) {
        if let Err(err) = state.dispatch(&UvProjectCubeCmd) {
            state.set_status(format!("uv cube: {err}"));
        }
    }

    /// Executa unwrap automático genérico usando o provider xatlas (P1-06 / P3D-064).
    pub fn unwrap_auto(state: &mut AppState) -> Result<usize, String> {
        state.dispatch(&UnwrapAutoCmd).map_err(|e| e.to_string())?;
        Ok(state
            .project
            .active_mesh()
            .map(|m| m.uv_islands().len())
            .unwrap_or(0))
    }

    /// Pack com padding em pixels da textura ativa.
    pub fn pack_islands(state: &mut AppState, padding_px: f32) -> Result<usize, String> {
        state
            .dispatch(&UvPackIslandsCmd {
                padding_px,
                ..UvPackIslandsCmd::default()
            })
            .map_err(|e| e.to_string())?;
        Ok(state
            .project
            .active_mesh()
            .map(|m| m.uv_islands().len())
            .unwrap_or(0))
    }

    /// Equaliza a densidade de texels entre as ilhas UV (ou ilhas selecionadas).
    pub fn equalize_texel_density(state: &mut AppState) -> Result<usize, String> {
        state
            .dispatch(&UvEqualizeTexelDensityCmd)
            .map_err(|e| e.to_string())?;
        Ok(state
            .project
            .active_mesh()
            .map(|m| m.uv_islands().len())
            .unwrap_or(0))
    }

    pub fn project_from_view(state: &mut AppState) -> Result<(), String> {
        state
            .dispatch(&UvProjectFromViewCmd)
            .map_err(|e| e.to_string())
    }

    pub fn mark_selected_seams(state: &mut AppState) {
        if let Err(err) = state.dispatch(&UvSeamCmd {
            op: SeamOp::Mark,
            edges: None,
        }) {
            state.set_status(format!("mark seam: {err}"));
        }
    }

    pub fn clear_selected_seams(state: &mut AppState) {
        if let Err(err) = state.dispatch(&UvSeamCmd {
            op: SeamOp::Clear,
            edges: None,
        }) {
            state.set_status(format!("clear seam: {err}"));
        }
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
        if let Err(err) = state.dispatch(&UvNormalizeTexelDensityCmd { texture_w, target }) {
            state.set_status(format!("normalize texel density: {err}"));
        }
    }

    /// Face cuja ilha contém (u, v) na malha ativa.
    pub fn uv_hit(state: &AppState, u: f32, v: f32) -> Option<usize> {
        state
            .project
            .active_mesh()?
            .uv_face_at(glam::Vec2::new(u, v))
    }
}

impl Module for UvModule {
    fn id(&self) -> &'static str {
        "uv"
    }

    fn on_event(&mut self, event: &AppEvent, state: &mut AppState) {
        // O Sync 3D → UV é da Application (`sync_selection`), e só age com Sync
        // ligado; aqui resta descartar cantos que deixaram de existir.
        if let AppEvent::MeshChanged { .. } | AppEvent::ActiveAssetChanged { .. } = event {
            let project = &state.project;
            if let Some(o) = project.assets.get(project.active) {
                state.session.tools.uv_selection.retain_valid(&o.mesh);
            }
            self.dirty = true;
        }
    }

    fn as_any(&self) -> &(dyn std::any::Any + 'static) {
        self
    }

    fn as_any_mut(&mut self) -> &mut (dyn std::any::Any + 'static) {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn transforms_only_touch_the_uv_selection() {
        let mut state = AppState::new("en");
        UvModule::reproject(&mut state);
        let untouched = state.project.active_mesh().unwrap().faces[1].uv.clone();
        let original = state.project.active_mesh().unwrap().faces[0].uv[0];

        let mesh = state.project.active_mesh().unwrap().clone();
        state.session.uv_selection.set_faces(&mesh, [0]);
        UvModule::move_selected(&mut state, 0.2, 0.1);
        let moved = state.project.active_mesh().unwrap().faces[0].uv[0];
        assert!((moved[0] - (original[0] + 0.2)).abs() < 1e-4);
        assert!((moved[1] - (original[1] + 0.1)).abs() < 1e-4);
        // A seleção é por canto: os cantos da face 1 não são tocados.
        assert_eq!(state.project.active_mesh().unwrap().faces[1].uv, untouched);

        UvModule::scale_selected(&mut state, 2.0);
        UvModule::rotate_selected(&mut state, std::f32::consts::FRAC_PI_2);
        assert!(
            state.project.active_mesh().unwrap().faces[0]
                .uv
                .iter()
                .all(|uv| uv[0].is_finite() && uv[1].is_finite())
        );
    }

    #[test]
    fn test_uv_auto_unwrap_xatlas() {
        let mut state = AppState::new("en");
        let charts = UvModule::unwrap_auto(&mut state).unwrap();
        assert!(charts >= 1);
        let mesh = state.project.active_mesh().unwrap();
        assert!(
            mesh.faces
                .iter()
                .all(|f| f.uv.iter().all(|uv| uv[0].is_finite() && uv[1].is_finite()))
        );
    }

    #[test]
    fn direct_moves_carry_pinned_corners_and_pins_follow_the_selection() {
        let mut state = AppState::new("en");
        let active = state.project.active;
        state.project.assets[active].mesh = petunia_mesh::Mesh::cube(2.0).into();
        UvModule::project_cube(&mut state);
        state.project.active_mesh_mut().unwrap().pin_uv(0, 0);
        let uv0 = state.project.active_mesh().unwrap().faces[0].uv[0];

        let mesh = state.project.active_mesh().unwrap().clone();
        state.session.uv_selection.set_faces(&mesh, [0]);
        UvModule::move_selected(&mut state, 0.5, 0.5);
        // Gesto explícito move o canto fixado; ele continua fixado.
        let mesh = state.project.active_mesh().unwrap();
        assert!((mesh.faces[0].uv[0][0] - (uv0[0] + 0.5)).abs() < 1e-4);
        assert!(mesh.is_uv_pinned(0, 0));

        let mesh = state.project.active_mesh().unwrap().clone();
        state.session.uv_selection.set_faces(&mesh, [1]);
        UvModule::pin_selected(&mut state);
        assert!(state.project.active_mesh().unwrap().is_uv_pinned(1, 0));

        UvModule::clear_all_pins(&mut state);
        assert_eq!(state.project.active_mesh().unwrap().uv_pinned.len(), 0);
    }
}
