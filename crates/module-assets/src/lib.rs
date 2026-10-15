//! module-assets — Asset Library (§13): múltiplos assets com UUID,
//! rename/duplicate/delete/search. Reage a `ActiveAssetChanged`.

use petunia_core::{
    AddMeshAssetCmd, AppEvent, AppState, DeleteAssetCmd, DuplicateAssetCmd, JoinAssetIntoActiveCmd,
    Module,
};
use petunia_mesh::Mesh;

#[derive(Default)]
pub struct AssetsModule {
    pub filter: String,
}

impl AssetsModule {
    pub fn new() -> Self {
        Self {
            filter: String::new(),
        }
    }

    pub fn add_primitive(state: &mut AppState, name: &str, mut mesh: Mesh) {
        let cursor = state.session.cursor_3d;
        for v in &mut mesh.verts {
            v.pos[0] += cursor[0];
            v.pos[1] += cursor[1];
            v.pos[2] += cursor[2];
        }
        if let Err(err) = state.dispatch(&AddMeshAssetCmd {
            label: "add asset",
            name: name.to_string(),
            mesh,
        }) {
            state.set_status(format!("add asset: {err}"));
        }
    }

    pub fn duplicate_active(state: &mut AppState) {
        if let Err(err) = state.dispatch(&DuplicateAssetCmd::default()) {
            state.set_status(format!("duplicate asset: {err}"));
        }
    }

    pub fn delete_active(state: &mut AppState) {
        if let Err(err) = state.dispatch(&DeleteAssetCmd::default()) {
            state.set_status(format!("delete asset: {err}"));
        }
    }

    /// Mescla a malha de outro asset com o asset ativo e remove o asset de origem.
    pub fn join_asset(state: &mut AppState, source_idx: usize) {
        if let Err(err) = state.dispatch(&JoinAssetIntoActiveCmd {
            source_index: source_idx,
        }) {
            state.set_status(format!("join asset: {err}"));
        }
    }

    /// Assets visíveis após o filtro (índices reais).
    pub fn filtered(state: &AppState, filter: &str) -> Vec<usize> {
        let f = filter.to_lowercase();
        state
            .project
            .assets
            .iter()
            .enumerate()
            .filter(|(_, a)| f.is_empty() || a.name.to_lowercase().contains(&f))
            .map(|(i, _)| i)
            .collect()
    }
}

impl Module for AssetsModule {
    fn id(&self) -> &'static str {
        "assets"
    }
    fn on_event(&mut self, _event: &AppEvent, _state: &mut AppState) {}

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
    fn test_assets_filter_and_operations() {
        let mut state = AppState::new("en");
        assert_eq!(state.project.assets.len(), 1);

        // Add primitives
        AssetsModule::add_primitive(&mut state, "Sphere", Mesh::cube(1.0));
        AssetsModule::add_primitive(&mut state, "Cylinder", Mesh::cube(1.0));
        assert_eq!(state.project.assets.len(), 3);

        // Filter
        let all = AssetsModule::filtered(&state, "");
        assert_eq!(all.len(), 3);

        let cyl = AssetsModule::filtered(&state, "cyl");
        assert_eq!(cyl.len(), 1);
        assert_eq!(state.project.assets[cyl[0]].name, "Cylinder");

        // Duplicate active
        state.project.active = 1;
        AssetsModule::duplicate_active(&mut state);
        assert_eq!(state.project.assets.len(), 4);
        assert!(state.project.assets.last().unwrap().name.contains("copy"));

        // Join asset into active
        state.project.active = 0;
        let v0 = state.project.assets[0].mesh.verts.len();
        let v1 = state.project.assets[1].mesh.verts.len();
        AssetsModule::join_asset(&mut state, 1);
        assert_eq!(state.project.assets.len(), 3);
        assert_eq!(state.project.assets[0].mesh.verts.len(), v0 + v1);
    }
}
