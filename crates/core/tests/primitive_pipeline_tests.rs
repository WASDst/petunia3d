//! Pipeline por primitiva (§59): Create → Confirm → Select → Transform →
//! Edit Mode → Save → Load → Export OBJ/GLB. Malha válida na memória não basta;
//! cada espécie atravessa persistência e exportadores de verdade.

use petunia_core::{
    AppState, DuplicateSelectionCmd, EditMode, PrimitiveKind, ProjectService, SubdivideSelectionCmd,
};

fn all_kinds() -> [PrimitiveKind; 10] {
    use PrimitiveKind as K;
    [
        K::Cube,
        K::Plane,
        K::Wedge,
        K::Cylinder,
        K::Cone,
        K::Circle,
        K::Torus,
        K::Sphere,
        K::Icosphere,
        K::Capsule,
    ]
}

#[test]
fn primitive_full_pipeline_per_species() {
    for kind in all_kinds() {
        let mut state = AppState::new("en");
        // Create → Confirm via sessão (transação única).
        assert!(state.begin_primitive(kind, None), "{kind:?} begin");
        assert!(state.confirm_primitive(), "{kind:?} confirm");
        let asset_idx = state.project.assets.len() - 1;
        let verts_before = state.project.assets[asset_idx].source_mesh().verts.len();
        assert!(verts_before > 0, "{kind:?} sem vértices");

        // Select + Duplicate + ferramenta de modelagem sobre a malha criada.
        state.select_object(Some(asset_idx), false);
        state.dispatch(&DuplicateSelectionCmd).expect("duplicate");
        assert_eq!(state.project.assets.len(), asset_idx + 2);
        state.project.active = asset_idx + 1;
        assert!(state.project.active().unwrap().geometry_source.is_some());
        assert!(state.dispatch(&SubdivideSelectionCmd).is_err());
        assert!(state.freeze_active_primitive());
        // Edit Mode sobre a duplicata após conversão explícita.
        state.set_edit_mode(EditMode::Edit);
        state
            .dispatch(&petunia_core::SelectAllCmd)
            .expect("select editable mesh");
        state
            .dispatch(&SubdivideSelectionCmd)
            .expect("subdivide editable mesh");
        assert_eq!(state.edit_mode(), EditMode::Edit);

        // Save → Load round-trip.
        let dir = tempfile::tempdir().expect("tempdir");
        let proj_path = dir.path().join("scene.petunia");
        ProjectService::save_project(&mut state, &proj_path).expect("save");
        let mut loaded = AppState::new("en");
        ProjectService::load_project(&mut loaded, &proj_path).expect("load");
        assert_eq!(
            loaded.project.assets.len(),
            asset_idx + 2,
            "{kind:?} assets"
        );
        assert_eq!(
            loaded.project.assets[asset_idx].source_mesh().verts.len(),
            verts_before,
            "{kind:?} verts round-trip"
        );

        // Export OBJ + GLB da original confirmada.
        let obj_path = dir.path().join("mesh.obj");
        ProjectService::export_obj(&loaded, asset_idx, &obj_path).expect("obj");
        assert!(obj_path.metadata().expect("obj meta").len() > 0);

        let glb_path = dir.path().join("mesh.glb");
        ProjectService::export_glb(&loaded, &[asset_idx], &glb_path).expect("glb");
        assert!(glb_path.metadata().expect("glb meta").len() > 0);
    }
}

#[test]
fn canceled_primitive_never_reaches_save() {
    let mut state = AppState::new("en");
    assert!(state.begin_primitive(PrimitiveKind::Torus, None));
    assert!(state.cancel_primitive());
    let dir = tempfile::tempdir().expect("tempdir");
    let proj_path = dir.path().join("scene.petunia");
    ProjectService::save_project(&mut state, &proj_path).expect("save");
    let mut loaded = AppState::new("en");
    ProjectService::load_project(&mut loaded, &proj_path).expect("load");
    assert!(
        loaded.project.assets.iter().all(|a| a.name != "Torus"),
        "preview cancelado vazou para o save"
    );
}
