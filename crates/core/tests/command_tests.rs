//! Testes do sistema de comandos (CommandDispatcher e Comandos Básicos) do Petunia3D.
//! Garante a execução headless e o ciclo transacional de Undo/Redo sem qualquer dependência de UI.
#![allow(clippy::field_reassign_with_default)]

use petunia_core::ProjectService;
use petunia_core::command::{
    AddPrimitiveCmd, AddSplinePointCmd, AttachSplinePointCmd, BakeDecalCmd, BakePathGeneratorCmd,
    BoxSelectCmd, ClearSelectionCmd, CommandDispatcher, CommandError, ConvertSplineToPolylineCmd,
    CreatePathGeneratorCmd, CreateProfileCmd, CreateSplineCmd, DeleteAssetCmd,
    DeleteOrDissolveSelectionCmd, DeleteProfileCmd, DeleteSelectionCmd, DeleteSplineCmd,
    DeleteSplinePointCmd, DetachSplinePointCmd, DissolveCmd, DuplicateAssetCmd,
    DuplicateSelectionCmd, ExtrudeIndividualCmd, FlipDiagonalCmd, FlipNormalsCmd,
    InvertSelectionCmd, MergeCenterCmd, MoveSplinePointCmd, PrimitiveKind, ReorderAssetCmd,
    ReprojectSplinePointAttachmentCmd, ReverseSplineCmd, RevolveCmd,
    RotateSplinePointAttachmentCmd, SelectAllCmd, SelectLinkedCmd, SetAssetCollectionCmd,
    SetDecalTransformCmd, SetSplineClosedCmd, SetSplineHandlesCmd, SlideSplinePointAttachmentCmd,
    SubdivideSelectionCmd, ToggleCollectionLockCmd, ToggleCollectionVisibilityCmd,
    ToggleLockAssetCmd, ToggleVisibilityAssetCmd, UpdateProfileCmd, UpdateSplineCmd,
    UpdateSweepGeneratorCmd, UvRelaxCmd, UvStitchCmd,
};
use petunia_core::state::{ASSET_NAME_MAX_LEN, AppState, AssetRenameError, DirtyReason, EditMode};
use petunia_core::{
    PathGenerator, PathGeneratorEvaluationCache, PathGeneratorQuality, ProfileResource,
    ProfileWorkplane, SplineHandleMode, SplineInterpolation, SplinePoint, SplineResource,
    SurfaceAttachmentStatus, SweepGeneratorParameters, project_ray_to_surface_target,
};

fn sweep_fixture() -> (
    SplineResource,
    ProfileResource,
    SplineResource,
    PathGenerator,
) {
    let profile_spline = SplineResource::from_polyline(
        "Square curve",
        &[
            [-0.5, -0.5, 0.0],
            [0.5, -0.5, 0.0],
            [0.5, 0.5, 0.0],
            [-0.5, 0.5, 0.0],
        ],
        true,
    );
    let profile = ProfileResource::new("Square", profile_spline.id, ProfileWorkplane::default());
    let path = SplineResource::from_polyline(
        "Guide",
        &[[0.0, 0.0, 0.0], [0.0, 0.0, 2.0], [1.0, 0.0, 3.0]],
        false,
    );
    let generator = PathGenerator::sweep(
        "Square sweep",
        path.id,
        profile.id,
        SweepGeneratorParameters::default(),
    );
    (profile_spline, profile, path, generator)
}

#[test]
fn test_add_primitive_commands_and_undo_redo() {
    let mut state = AppState::default();
    assert_eq!(state.project.assets.len(), 1); // Cubo padrão inicial

    // Adiciona Sphere
    let add_sphere = AddPrimitiveCmd::new(PrimitiveKind::Sphere);
    state.dispatch(&add_sphere).expect("deve adicionar esfera");
    assert_eq!(state.project.assets.len(), 2);
    assert!(state.project.assets[1].name.contains("Sphere"));

    // Adiciona Cylinder
    let add_cyl = AddPrimitiveCmd::new(PrimitiveKind::Cylinder);
    state.dispatch(&add_cyl).expect("deve adicionar cilindro");
    assert_eq!(state.project.assets.len(), 3);

    // Adiciona Plane
    let add_plane = AddPrimitiveCmd::new(PrimitiveKind::Plane);
    state.dispatch(&add_plane).expect("deve adicionar plano");
    assert_eq!(state.project.assets.len(), 4);

    // Adiciona Cone
    let add_cone = AddPrimitiveCmd::new(PrimitiveKind::Cone);
    state.dispatch(&add_cone).expect("deve adicionar cone");
    assert_eq!(state.project.assets.len(), 5);

    // Adiciona Capsule
    let add_capsule = AddPrimitiveCmd::new(PrimitiveKind::Capsule);
    state
        .dispatch(&add_capsule)
        .expect("deve adicionar cápsula");
    assert_eq!(state.project.assets.len(), 6);

    // Testa Undo em cadeia
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 5);

    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 4);

    // Testa Redo em cadeia
    assert!(state.redo());
    assert_eq!(state.project.assets.len(), 5);

    assert!(state.redo());
    assert_eq!(state.project.assets.len(), 6);
}

#[test]
fn test_duplicate_and_delete_asset_cmd() {
    let mut state = AppState::default();
    assert_eq!(state.project.assets.len(), 1);

    // Duplica o cubo ativo
    let dup_cmd = DuplicateAssetCmd { asset_index: None };
    state.dispatch(&dup_cmd).expect("deve duplicar asset ativo");
    assert_eq!(state.project.assets.len(), 2);
    assert_eq!(state.project.assets[1].name, "Cube copy");

    // Undo restaura para 1 asset
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 1);

    // Redo volta para 2 assets
    assert!(state.redo());
    assert_eq!(state.project.assets.len(), 2);

    // Deleta o asset recém-duplicado
    let del_cmd = DeleteAssetCmd {
        asset_index: Some(1),
    };
    state
        .dispatch(&del_cmd)
        .expect("deve deletar asset no índice 1");
    assert_eq!(state.project.assets.len(), 1);

    // Undo restaura o asset deletado
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 2);

    // Erro ao tentar deletar índice inexistente
    let invalid_del = DeleteAssetCmd {
        asset_index: Some(999),
    };
    let err = state.dispatch(&invalid_del).unwrap_err();
    assert!(
        matches!(
            err,
            CommandError::Execution(_) | CommandError::InvalidAssetIndex(999)
        ),
        "esperava erro ao tentar deletar índice inexistente, obteve: {:?}",
        err
    );
}

#[test]
fn test_reorder_asset_cmd_and_undo() {
    // Reorders assets in the scene and verifies active index tracking and undo/redo
    // Reordena assets na cena e valida rastreamento do índice ativo e undo/redo
    let mut state = AppState::default();
    let add_sphere = AddPrimitiveCmd::new(PrimitiveKind::Sphere);
    state.dispatch(&add_sphere).expect("adiciona esfera");
    assert_eq!(state.project.assets.len(), 2);
    assert_eq!(state.project.assets[0].name, "Cube");
    assert_eq!(state.project.assets[1].name, "Sphere");
    assert_eq!(state.project.active, 1);

    // Reordena 0 -> 1 (Cube vai para a posição 1, Sphere vai para a posição 0)
    let reorder_cmd = ReorderAssetCmd { from: 0, to: 1 };
    state.dispatch(&reorder_cmd).expect("reordena assets");
    assert_eq!(state.project.assets[0].name, "Sphere");
    assert_eq!(state.project.assets[1].name, "Cube");
    // O asset ativo era Sphere, que agora está no índice 0
    // Active asset was Sphere, which is now at index 0
    assert_eq!(state.project.active, 0);

    // Undo restaura a ordem e o índice ativo original
    // Undo restores original order and active index
    assert!(state.undo());
    assert_eq!(state.project.assets[0].name, "Cube");
    assert_eq!(state.project.assets[1].name, "Sphere");
    assert_eq!(state.project.active, 1);

    // Redo reaplica a reordenação
    // Redo reapplies the reorder
    assert!(state.redo());
    assert_eq!(state.project.assets[0].name, "Sphere");
    assert_eq!(state.project.assets[1].name, "Cube");
    assert_eq!(state.project.active, 0);

    // Tentativa com índices inválidos é rejeitada
    // Attempt with invalid indices is rejected
    let invalid_reorder = ReorderAssetCmd { from: 0, to: 0 };
    assert!(state.dispatch(&invalid_reorder).is_err());
    let out_of_bounds = ReorderAssetCmd { from: 0, to: 99 };
    assert!(state.dispatch(&out_of_bounds).is_err());
}

#[test]
fn test_delete_selection_in_edit_and_object_modes() {
    let mut state = AppState::default();

    // 1. Em Object Mode, DeleteSelectionCmd remove o asset
    state.set_edit_mode(EditMode::Object);
    let add_cyl = AddPrimitiveCmd::new(PrimitiveKind::Cylinder);
    state.dispatch(&add_cyl).expect("adiciona cilindro");
    assert_eq!(state.project.assets.len(), 2);

    let del_sel = DeleteSelectionCmd;
    state.dispatch(&del_sel).expect("deleta asset ativo");
    assert_eq!(state.project.assets.len(), 1);

    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 2);

    // 2. Em Edit Mode, DeleteSelectionCmd remove sub-elementos da malha
    state.set_edit_mode(EditMode::Edit);
    let initial_verts = state.project.active_mesh().unwrap().verts.len();
    assert!(initial_verts > 0);

    // Seleciona tudo e deleta
    state.dispatch(&SelectAllCmd).expect("seleciona tudo");
    state
        .dispatch(&del_sel)
        .expect("deleta geometria selecionada");

    assert_eq!(state.project.active_mesh().unwrap().verts.len(), 0);

    // Undo restaura a geometria da malha ativa
    assert!(state.undo());
    assert_eq!(
        state.project.active_mesh().unwrap().verts.len(),
        initial_verts
    );
}

#[test]
fn test_duplicate_selection_in_edit_mode() {
    let mut state = AppState::default();
    state.set_edit_mode(EditMode::Edit);

    let initial_verts = state.project.active_mesh().unwrap().verts.len();
    assert_eq!(initial_verts, 8); // Cubo

    // Seleciona tudo
    state.dispatch(&SelectAllCmd).expect("seleciona tudo");

    // Duplica geometria selecionada
    let dup_sel = DuplicateSelectionCmd;
    state.dispatch(&dup_sel).expect("duplica geometria");

    let new_verts = state.project.active_mesh().unwrap().verts.len();
    assert_eq!(new_verts, 16); // 8 originais + 8 duplicados

    // Undo restaura para 8
    assert!(state.undo());
    assert_eq!(state.project.active_mesh().unwrap().verts.len(), 8);

    // Redo re-aplica duplicação para 16
    assert!(state.redo());
    assert_eq!(state.project.active_mesh().unwrap().verts.len(), 16);
}

#[test]
fn test_selection_commands_are_non_destructive() {
    let mut state = AppState::default();
    state.set_edit_mode(EditMode::Edit);

    let mesh = state.project.active_mesh_mut().unwrap();
    mesh.deselect_all();
    assert!(mesh.verts.iter().all(|v| !v.selected));

    // SelectAllCmd
    state.dispatch(&SelectAllCmd).expect("select all");
    let mesh = state.project.active_mesh().unwrap();
    assert!(mesh.verts.iter().all(|v| v.selected));

    // ClearSelectionCmd
    state.dispatch(&ClearSelectionCmd).expect("clear selection");
    let mesh = state.project.active_mesh().unwrap();
    assert!(mesh.verts.iter().all(|v| !v.selected));

    // InvertSelectionCmd
    let mesh = state.project.active_mesh_mut().unwrap();
    mesh.verts[0].selected = true;
    state
        .dispatch(&InvertSelectionCmd)
        .expect("invert selection");
    let mesh = state.project.active_mesh().unwrap();
    assert!(!mesh.verts[0].selected);
    assert!(mesh.verts[1..].iter().all(|v| v.selected));

    // Seleção não é destrutiva: undo() retorna false porque nenhuma operação destrutiva foi feita
    assert!(!state.undo());
}

#[test]
fn selection_command_advances_only_selection_revision() {
    let mut state = AppState::default();
    state.set_edit_mode(EditMode::Edit);
    state.sync_selection();
    let before = state.project.project.revision_clock();

    state.dispatch(&SelectAllCmd).expect("select all");
    let after = state.project.project.revision_clock();

    assert_eq!(after[0], before[0], "topology revision");
    assert_eq!(after[1], before[1], "position revision");
    assert_eq!(after[2], before[2], "normal revision");
    assert_eq!(after[3], before[3] + 1, "selection revision");
    assert_eq!(after[4], before[4], "uv revision");
    assert_eq!(after[5], before[5], "color revision");
    assert_eq!(after[6], before[6], "material revision");
    assert_eq!(after[7], before[7], "texture revision");
    assert_eq!(after[8], before[8], "transform revision");
    assert_eq!(after[9], before[9], "spline revision");
    assert_eq!(after[10], before[10], "procedural revision");
    assert_eq!(state.project.undo.depth(), (0, 0));
}

#[test]
fn uv_commands_commit_once_and_advance_only_uv_revision() {
    for command in [
        &UvStitchCmd as &dyn petunia_core::command::Command,
        &UvRelaxCmd { iterations: 2 } as &dyn petunia_core::command::Command,
    ] {
        let mut state = AppState::default();
        state.sync_selection();
        let before = state.project.project.revision_clock();

        state.dispatch(command).expect("uv command");
        let after = state.project.project.revision_clock();

        assert_eq!(state.project.undo.depth(), (1, 0), "{}", command.label());
        assert_eq!(after[0], before[0], "{} topology", command.label());
        assert_eq!(after[1], before[1], "{} positions", command.label());
        assert_eq!(after[2], before[2], "{} normals", command.label());
        assert_eq!(after[3], before[3], "{} selection", command.label());
        assert_eq!(after[4], before[4] + 1, "{} uv", command.label());
        assert_eq!(after[5], before[5], "{} colors", command.label());
        assert_eq!(after[6], before[6], "{} materials", command.label());
        assert_eq!(after[7], before[7], "{} textures", command.label());
        assert_eq!(after[8], before[8], "{} transforms", command.label());
        assert_eq!(after[9], before[9], "{} splines", command.label());
        assert_eq!(after[10], before[10], "{} procedural", command.label());
    }
}

#[test]
fn flip_normals_advances_only_normal_revision() {
    let mut state = AppState::default();
    state.sync_selection();
    let before = state.project.project.revision_clock();

    state.dispatch(&FlipNormalsCmd).expect("flip normals");
    let after = state.project.project.revision_clock();

    assert_eq!(state.project.undo.depth(), (1, 0));
    for index in [0, 1, 3, 4, 5, 6, 7, 8, 9, 10] {
        assert_eq!(after[index], before[index], "revision index {index}");
    }
    assert_eq!(after[2], before[2] + 1);
}

#[test]
fn spline_create_move_undo_redo_is_transactional_and_domain_scoped() {
    let mut state = AppState::default();
    state.sync_selection();
    let spline = SplineResource::from_polyline(
        "Guide",
        &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [2.0, 0.5, 0.0]],
        false,
    );
    let spline_id = spline.id;
    let point_id = spline.points[1].id;
    let before = state.project.project.revision_clock();

    state.dispatch(&CreateSplineCmd { spline }).unwrap();

    let after_create = state.project.project.revision_clock();
    assert_eq!(state.project.undo.depth(), (1, 0));
    assert_eq!(state.render.last_dirty_reason, Some(DirtyReason::CurveEdit));
    for index in 0..9 {
        assert_eq!(after_create[index], before[index], "revision index {index}");
    }
    assert_eq!(after_create[9], before[9] + 1);
    assert_eq!(after_create[10], before[10]);

    state
        .dispatch(&MoveSplinePointCmd {
            spline_id,
            point_id,
            position: [1.0, 1.0, 0.0],
        })
        .unwrap();
    assert_eq!(state.project.undo.depth(), (2, 0));
    assert_eq!(
        state
            .project
            .project
            .get_spline(spline_id)
            .unwrap()
            .point(point_id)
            .unwrap()
            .position,
        [1.0, 1.0, 0.0]
    );

    let before_no_op = state.project.project.revision_clock();
    let depth_before_no_op = state.project.undo.depth();
    assert!(
        state
            .dispatch(&MoveSplinePointCmd {
                spline_id,
                point_id,
                position: [1.0, 1.0, 0.0],
            })
            .is_err()
    );
    assert_eq!(state.project.undo.depth(), depth_before_no_op);
    assert_eq!(state.project.project.revision_clock(), before_no_op);

    assert!(state.undo());
    assert_eq!(
        state.render.last_dirty_reason,
        Some(DirtyReason::MaterialEdit)
    );
    assert_eq!(
        state
            .project
            .project
            .get_spline(spline_id)
            .unwrap()
            .point(point_id)
            .unwrap()
            .position,
        [1.0, 0.0, 0.0]
    );
    assert!(state.redo());
    assert_eq!(
        state
            .project
            .project
            .get_spline(spline_id)
            .unwrap()
            .point(point_id)
            .unwrap()
            .position,
        [1.0, 1.0, 0.0]
    );
}

#[test]
fn persistent_profile_sweep_and_bake_are_transactional() {
    let mut state = AppState::default();
    let initial_assets = state.project.assets.len();
    let (profile_spline, profile, path, generator) = sweep_fixture();
    let profile_spline_id = profile_spline.id;
    let profile_id = profile.id;
    let path_id = path.id;
    let generator_id = generator.id;

    let clock_before = state.project.project.revision_clock();
    state
        .dispatch(&CreateProfileCmd {
            spline: profile_spline,
            profile,
        })
        .unwrap();
    assert_eq!(state.project.profiles.len(), 1);
    assert_eq!(state.render.last_dirty_reason, Some(DirtyReason::CurveEdit));
    assert_eq!(
        state.project.project.revision_clock()[9],
        clock_before[9] + 1
    );
    assert_eq!(
        state.project.project.revision_clock()[10],
        clock_before[10] + 1
    );

    state.dispatch(&CreateSplineCmd { spline: path }).unwrap();
    state
        .dispatch(&CreatePathGeneratorCmd { generator })
        .unwrap();
    assert_eq!(state.project.path_generators.len(), 1);
    assert_eq!(state.render.last_dirty_reason, Some(DirtyReason::CurveEdit));

    let depth_before_blocked_delete = state.project.undo.depth();
    assert!(
        state
            .dispatch(&DeleteSplineCmd { spline_id: path_id })
            .is_err()
    );
    assert_eq!(state.project.undo.depth(), depth_before_blocked_delete);
    assert!(state.dispatch(&DeleteProfileCmd { profile_id }).is_err());
    assert_eq!(state.project.undo.depth(), depth_before_blocked_delete);

    let mut parameters = state
        .project
        .project
        .get_path_generator(generator_id)
        .unwrap()
        .sweep_parameters();
    parameters.miter = false;
    state
        .dispatch(&UpdateSweepGeneratorCmd {
            generator_id,
            parameters,
        })
        .unwrap();
    let depth_after_update = state.project.undo.depth();
    assert!(
        state
            .dispatch(&UpdateSweepGeneratorCmd {
                generator_id,
                parameters,
            })
            .is_err()
    );
    assert_eq!(state.project.undo.depth(), depth_after_update);

    let expected = {
        let mut cache = PathGeneratorEvaluationCache::default();
        state
            .project
            .project
            .evaluate_path_generator(generator_id, PathGeneratorQuality::Final, &mut cache)
            .unwrap()
            .mesh
            .clone()
    };
    state
        .dispatch(&BakePathGeneratorCmd {
            generator_id,
            asset_name: "Editable sweep".to_string(),
        })
        .unwrap();

    assert_eq!(state.project.assets.len(), initial_assets + 1);
    assert!(
        state
            .project
            .project
            .get_path_generator(generator_id)
            .is_none()
    );
    assert!(state.project.project.get_profile(profile_id).is_some());
    assert!(
        state
            .project
            .project
            .get_spline(profile_spline_id)
            .is_some()
    );
    assert!(state.project.project.get_spline(path_id).is_some());
    let baked = state.project.active().unwrap();
    assert_eq!(baked.name, "Editable sweep");
    assert_eq!(baked.mesh.vert_count(), expected.vert_count());
    assert_eq!(baked.mesh.tri_count(), expected.tri_count());
    assert!(
        baked
            .mesh
            .verts
            .iter()
            .zip(&expected.verts)
            .all(|(left, right)| left.pos == right.pos)
    );

    assert!(state.undo());
    assert_eq!(state.project.assets.len(), initial_assets);
    assert!(
        state
            .project
            .project
            .get_path_generator(generator_id)
            .is_some()
    );
    assert!(state.redo());
    assert_eq!(state.project.assets.len(), initial_assets + 1);
    assert!(
        state
            .project
            .project
            .get_path_generator(generator_id)
            .is_none()
    );
}

#[test]
fn profile_draft_and_batch_updates_are_single_entry_roundtrips() {
    let mut state = AppState::default();
    let mut spline = SplineResource::new("Draft curve", SplineInterpolation::CubicBezier);
    spline.add_point(SplinePoint::new([0.0, 0.0, 0.0])).unwrap();
    let spline_id = spline.id;
    let profile = ProfileResource::new("Draft", spline_id, ProfileWorkplane::default());
    let profile_id = profile.id;
    state
        .dispatch(&CreateProfileCmd { spline, profile })
        .unwrap();
    assert_eq!(state.project.undo.depth(), (1, 0));

    let mut updated_spline = state.project.project.get_spline(spline_id).unwrap().clone();
    updated_spline
        .add_point(SplinePoint::new([1.0, 0.0, 0.0]))
        .unwrap();
    state
        .dispatch(&UpdateSplineCmd {
            spline: updated_spline,
        })
        .unwrap();
    assert_eq!(state.project.undo.depth(), (2, 0));

    let mut updated_profile = state
        .project
        .project
        .get_profile(profile_id)
        .unwrap()
        .clone();
    updated_profile.wall_thickness = 0.25;
    updated_profile.revision = updated_profile.revision.wrapping_add(1);
    state
        .dispatch(&UpdateProfileCmd {
            profile: updated_profile,
        })
        .unwrap();
    assert_eq!(state.project.undo.depth(), (3, 0));
    assert_eq!(
        state
            .project
            .project
            .get_profile(profile_id)
            .unwrap()
            .wall_thickness,
        0.25
    );

    assert!(state.undo());
    assert_eq!(
        state
            .project
            .project
            .get_profile(profile_id)
            .unwrap()
            .wall_thickness,
        0.0
    );
    assert!(state.redo());
    assert_eq!(
        state
            .project
            .project
            .get_profile(profile_id)
            .unwrap()
            .wall_thickness,
        0.25
    );
}

#[test]
fn spline_authoring_commands_cover_point_handles_loop_reverse_convert_and_delete() {
    let mut state = AppState::default();
    let mut spline = SplineResource::new("Bezier guide", SplineInterpolation::CubicBezier);
    for position in [[0.0, 0.0, 0.0], [1.0, 1.0, 0.0], [2.0, 0.0, 0.0]] {
        spline.add_point(SplinePoint::new(position)).unwrap();
    }
    let spline_id = spline.id;
    let first_point = spline.points[0].id;
    state.dispatch(&CreateSplineCmd { spline }).unwrap();

    let inserted = SplinePoint::new([0.5, 0.25, 0.0]);
    let inserted_id = inserted.id;
    state
        .dispatch(&AddSplinePointCmd {
            spline_id,
            index: Some(1),
            point: inserted,
        })
        .unwrap();
    assert_eq!(
        state.project.project.get_spline(spline_id).unwrap().points[1].id,
        inserted_id
    );
    state
        .dispatch(&DeleteSplinePointCmd {
            spline_id,
            point_id: inserted_id,
        })
        .unwrap();

    state
        .dispatch(&SetSplineHandlesCmd {
            spline_id,
            point_id: first_point,
            handle_in: [-0.5, 0.0, 0.0],
            handle_out: [0.5, 0.0, 0.0],
            mode: SplineHandleMode::Mirrored,
        })
        .unwrap();
    state
        .dispatch(&SetSplineClosedCmd {
            spline_id,
            closed: true,
        })
        .unwrap();
    state.dispatch(&ReverseSplineCmd { spline_id }).unwrap();
    state
        .dispatch(&ConvertSplineToPolylineCmd {
            spline_id,
            spacing: 0.25,
            tolerance: 1.0e-4,
        })
        .unwrap();
    assert_eq!(
        state
            .project
            .project
            .get_spline(spline_id)
            .unwrap()
            .interpolation,
        SplineInterpolation::Polyline
    );

    state.dispatch(&DeleteSplineCmd { spline_id }).unwrap();
    assert!(state.project.project.get_spline(spline_id).is_none());
    assert!(state.undo());
    assert!(state.project.project.get_spline(spline_id).is_some());
}

#[test]
fn spline_surface_attachment_commands_are_transactional() {
    let mut state = AppState::default();
    state.project.project.assets[0].mesh = petunia_mesh::Mesh::plane(2.0);
    let target_id = state.project.project.assets[0].id;
    let spline = SplineResource::from_polyline(
        "Surface guide",
        &[[0.0, 0.0, 0.0], [0.75, 0.0, 0.75]],
        false,
    );
    let spline_id = spline.id;
    let point_id = spline.points[0].id;
    state.dispatch(&CreateSplineCmd { spline }).unwrap();
    let hit = project_ray_to_surface_target(
        &state.project.project,
        target_id,
        [0.0, 2.0, 0.0],
        [0.0, -1.0, 0.0],
        10.0,
    )
    .unwrap()
    .unwrap();

    let mut attachment = hit.attachment;
    attachment.normal_offset = 0.2;
    state
        .dispatch(&AttachSplinePointCmd {
            spline_id,
            point_id,
            attachment,
        })
        .unwrap();
    assert_eq!(state.project.undo.depth(), (2, 0));
    let point = state
        .project
        .project
        .get_spline(spline_id)
        .unwrap()
        .point(point_id)
        .unwrap();
    let stored_attachment = point.attachment.unwrap();
    assert_eq!(stored_attachment.normal_offset, 0.2);
    assert_eq!(stored_attachment.last_world_position, [0.0, 0.2, 0.0]);
    assert!((point.position[1] - 0.2).abs() < 1.0e-6);

    let depth_before_no_op = state.project.undo.depth();
    assert!(
        state
            .dispatch(&AttachSplinePointCmd {
                spline_id,
                point_id,
                attachment: stored_attachment,
            })
            .is_err()
    );
    assert_eq!(state.project.undo.depth(), depth_before_no_op);
    assert!(
        state
            .dispatch(&MoveSplinePointCmd {
                spline_id,
                point_id,
                position: [0.25, 0.0, 0.0],
            })
            .is_err()
    );
    assert_eq!(state.project.undo.depth(), depth_before_no_op);

    state
        .dispatch(&RotateSplinePointAttachmentCmd {
            spline_id,
            point_id,
            delta_radians: 0.5,
        })
        .unwrap();
    assert_eq!(state.project.undo.depth(), (3, 0));
    assert_eq!(
        state
            .project
            .project
            .get_spline(spline_id)
            .unwrap()
            .point(point_id)
            .unwrap()
            .attachment
            .unwrap()
            .tangent_rotation,
        0.5
    );

    state
        .dispatch(&SlideSplinePointAttachmentCmd {
            spline_id,
            point_id,
            ray_origin: [0.4, 2.0, -0.3],
            ray_direction: [0.0, -1.0, 0.0],
            max_distance: 10.0,
        })
        .unwrap();
    let slid = state
        .project
        .project
        .get_spline(spline_id)
        .unwrap()
        .point(point_id)
        .unwrap();
    assert!((slid.position[0] - 0.4).abs() < 1.0e-6);
    assert!((slid.position[2] + 0.3).abs() < 1.0e-6);

    let attached_before_detach = slid.attachment;
    state
        .dispatch(&DetachSplinePointCmd {
            spline_id,
            point_id,
        })
        .unwrap();
    assert!(
        state
            .project
            .project
            .get_spline(spline_id)
            .unwrap()
            .point(point_id)
            .unwrap()
            .attachment
            .is_none()
    );
    assert!(state.undo());
    assert_eq!(
        state
            .project
            .project
            .get_spline(spline_id)
            .unwrap()
            .point(point_id)
            .unwrap()
            .attachment,
        attached_before_detach
    );
}

#[test]
fn spline_attachment_reprojection_repairs_topology_and_can_change_target() {
    let mut state = AppState::default();
    state.project.project.assets[0].mesh = petunia_mesh::Mesh::plane(2.0);
    let first_target = state.project.project.assets[0].id;
    let mut second_mesh = petunia_mesh::Mesh::plane(2.0);
    for vertex in &mut second_mesh.verts {
        vertex.pos[0] += 3.0;
    }
    state.project.project.add("Second surface", second_mesh);
    let second_target = state.project.project.assets[1].id;
    let spline = SplineResource::from_polyline(
        "Reproject guide",
        &[[0.0, 0.0, 0.0], [0.5, 0.0, 0.0]],
        false,
    );
    let spline_id = spline.id;
    let point_id = spline.points[0].id;
    state.dispatch(&CreateSplineCmd { spline }).unwrap();
    let hit = project_ray_to_surface_target(
        &state.project.project,
        first_target,
        [0.0, 2.0, 0.0],
        [0.0, -1.0, 0.0],
        10.0,
    )
    .unwrap()
    .unwrap();
    state
        .dispatch(&AttachSplinePointCmd {
            spline_id,
            point_id,
            attachment: hit.attachment,
        })
        .unwrap();

    state.project.project.assets[0].mesh.faces[0]
        .verts
        .rotate_left(1);
    state.project.project.bump_topology();
    let stale = state
        .project
        .project
        .get_spline(spline_id)
        .unwrap()
        .point(point_id)
        .unwrap()
        .attachment
        .unwrap();
    assert_eq!(
        stale.status(&state.project.project),
        SurfaceAttachmentStatus::NeedsReattach
    );

    state
        .dispatch(&ReprojectSplinePointAttachmentCmd {
            spline_id,
            point_id,
            target_id: second_target,
            ray_origin: [3.25, 2.0, 0.25],
            ray_direction: [0.0, -1.0, 0.0],
            max_distance: 10.0,
        })
        .unwrap();
    let repaired = state
        .project
        .project
        .get_spline(spline_id)
        .unwrap()
        .point(point_id)
        .unwrap();
    assert_eq!(repaired.attachment.unwrap().target, second_target);
    assert_eq!(
        repaired.attachment.unwrap().status(&state.project.project),
        SurfaceAttachmentStatus::Valid
    );
    assert!((repaired.position[0] - 3.25).abs() < 1.0e-6);
}

#[test]
fn spline_conversion_resolves_attached_points_before_baking() {
    let mut state = AppState::default();
    state.project.project.assets[0].mesh = petunia_mesh::Mesh::plane(2.0);
    let target_id = state.project.project.assets[0].id;
    let spline = SplineResource::from_polyline(
        "Bake attached guide",
        &[[0.0, 0.0, 0.0], [0.75, 0.0, 0.75]],
        false,
    );
    let spline_id = spline.id;
    let point_id = spline.points[0].id;
    state.dispatch(&CreateSplineCmd { spline }).unwrap();
    let hit = project_ray_to_surface_target(
        &state.project.project,
        target_id,
        [0.0, 2.0, 0.0],
        [0.0, -1.0, 0.0],
        10.0,
    )
    .unwrap()
    .unwrap();
    state
        .dispatch(&AttachSplinePointCmd {
            spline_id,
            point_id,
            attachment: hit.attachment,
        })
        .unwrap();
    for vertex in &mut state.project.project.assets[0].mesh.verts {
        vertex.pos[1] += 1.0;
    }

    state
        .dispatch(&ConvertSplineToPolylineCmd {
            spline_id,
            spacing: 0.2,
            tolerance: 1.0e-4,
        })
        .unwrap();
    let converted = state.project.project.get_spline(spline_id).unwrap();
    assert!(
        converted
            .points
            .iter()
            .all(|point| point.attachment.is_none())
    );
    assert!((converted.points[0].position[1] - 1.0).abs() < 1.0e-6);
}

#[test]
fn dissolve_and_contextual_delete_each_commit_one_roundtrip() {
    for contextual in [false, true] {
        let mut state = AppState::default();
        state.set_edit_mode(EditMode::Edit);
        let edge = {
            let face = &state.project.active_mesh().unwrap().faces[0];
            let a = face.verts[0];
            let b = face.verts[1];
            (a.min(b), a.max(b))
        };
        state
            .project
            .active_mesh_mut()
            .unwrap()
            .selected_edges
            .insert(edge);
        state.sync_selection();
        let before_faces = state.project.active_mesh().unwrap().faces.len();
        let before_depth = state.project.undo.depth().0;

        if contextual {
            state
                .dispatch(&DeleteOrDissolveSelectionCmd)
                .expect("contextual delete");
        } else {
            state.dispatch(&DissolveCmd).expect("dissolve");
        }
        let after_faces = state.project.active_mesh().unwrap().faces.len();

        assert!(after_faces < before_faces);
        assert_eq!(state.project.undo.depth(), (before_depth + 1, 0));
        assert!(state.undo());
        assert_eq!(
            state.project.active_mesh().unwrap().faces.len(),
            before_faces
        );
        assert!(state.redo());
        assert_eq!(
            state.project.active_mesh().unwrap().faces.len(),
            after_faces
        );
    }
}

#[test]
fn project_history_budget_uses_deep_snapshot_size() {
    let mut state = AppState::default();
    state.project.active_mut().unwrap().texture =
        Some(petunia_project::Canvas::new(256, 256, [12, 34, 56, 255]));
    let snapshot_bytes = state.project.project.estimated_bytes();
    assert!(snapshot_bytes > std::mem::size_of::<petunia_project::Project>() + 256 * 256 * 4);

    state
        .project
        .undo
        .set_byte_budget(snapshot_bytes.saturating_add(1));
    state.checkpoint("first deep snapshot");
    state.project.active_mut().unwrap().name = "Changed".into();
    state.checkpoint("second deep snapshot");

    let metrics = state.project.history_metrics();
    assert_eq!(metrics.history_entries, 1);
    assert!(metrics.largest_entry >= snapshot_bytes);
}

#[test]
fn test_command_dispatcher_registry() {
    let mut state = AppState::default();
    let mut dispatcher = CommandDispatcher::new();

    dispatcher.register(
        "add_sphere",
        Box::new(AddPrimitiveCmd::new(PrimitiveKind::Sphere)),
    );
    dispatcher.register("select_all", Box::new(SelectAllCmd));

    // Executa comando registrado via dispatcher
    dispatcher
        .execute("add_sphere", &mut state)
        .expect("deve executar add_sphere");
    assert_eq!(state.project.assets.len(), 2);

    // Executa seleção via dispatcher (em Edit Mode para malha ativa)
    state.set_edit_mode(EditMode::Edit);
    dispatcher
        .execute("select_all", &mut state)
        .expect("deve executar select_all");
    let mesh = state.project.active_mesh().unwrap();
    assert!(mesh.verts.iter().all(|v| v.selected));

    // Comando não registrado deve retornar erro
    let err = dispatcher
        .execute("unknown_command", &mut state)
        .unwrap_err();
    assert!(matches!(err, CommandError::Execution(_)));
}

#[test]
fn unavailable_command_does_not_create_history_or_dirty_document() {
    let mut state = AppState::default();
    let original = state.project.project.clone();
    let mut dispatcher = CommandDispatcher::new();
    dispatcher.register(
        "delete.invalid",
        DeleteAssetCmd {
            asset_index: Some(999),
        },
    );

    let error = dispatcher
        .execute("delete.invalid", &mut state)
        .unwrap_err();

    assert!(matches!(error, CommandError::Execution(_)));
    assert_eq!(state.project.assets.len(), original.assets.len());
    assert!(!state.project.undo.can_undo());
    assert!(!state.is_document_dirty());
}

#[test]
fn test_headless_full_modeling_session() {
    // Prova de execução 100% headless sem qualquer binding de UI
    let mut state = AppState::default();

    // 1. Adiciona um cilindro
    state
        .dispatch(&AddPrimitiveCmd::new(PrimitiveKind::Cylinder))
        .expect("adiciona cilindro");
    assert_eq!(state.project.assets.len(), 2);

    // 2. Entra em modo de edição
    state.set_edit_mode(EditMode::Edit);

    // 3. Seleciona toda a malha e duplica
    state.dispatch(&SelectAllCmd).expect("seleciona");
    let initial_count = state.project.active_mesh().unwrap().verts.len();
    state.dispatch(&DuplicateSelectionCmd).expect("duplica");
    assert_eq!(
        state.project.active_mesh().unwrap().verts.len(),
        initial_count * 2
    );

    // 4. Inverte seleção e limpa
    state.dispatch(&InvertSelectionCmd).expect("inverte");
    state.dispatch(&ClearSelectionCmd).expect("limpa");
    assert_eq!(
        state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .filter(|v| v.selected)
            .count(),
        0
    );

    // 5. Undo desfaz a duplicação
    assert!(state.undo());
    assert_eq!(
        state.project.active_mesh().unwrap().verts.len(),
        initial_count
    );

    // 6. Undo desfaz a criação do cilindro
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 1);
}

#[test]
fn test_mesh_editing_commands_subdivide_merge_flip() {
    let mut state = AppState::default();
    state.set_edit_mode(EditMode::Edit);

    // Subdivisão da malha ativa
    state.dispatch(&SelectAllCmd).expect("seleciona tudo");
    let initial_faces = state.project.active_mesh().unwrap().faces.len();
    assert_eq!(initial_faces, 6); // Cubo

    state.dispatch(&SubdivideSelectionCmd).expect("subdivide");
    let subdivided_faces = state.project.active_mesh().unwrap().faces.len();
    assert!(subdivided_faces > initial_faces);

    // Undo restaura contagem de faces original
    assert!(state.undo());
    assert_eq!(
        state.project.active_mesh().unwrap().faces.len(),
        initial_faces
    );

    // Merge center
    state.dispatch(&SelectAllCmd).expect("seleciona tudo");
    state.dispatch(&MergeCenterCmd).expect("merge center");
    assert_eq!(state.project.active_mesh().unwrap().verts.len(), 1);

    // Undo restaura os 8 vértices
    assert!(state.undo());
    assert_eq!(state.project.active_mesh().unwrap().verts.len(), 8);

    // Flip normals
    let normal_before = state.project.active_mesh().unwrap().face_normal(0);
    state.dispatch(&FlipNormalsCmd).expect("flip normals");
    let normal_after = state.project.active_mesh().unwrap().face_normal(0);
    assert!((normal_before + normal_after).length() < 1e-4);

    // Undo restaura a normal original
    assert!(state.undo());
    let normal_restored = state.project.active_mesh().unwrap().face_normal(0);
    assert!((normal_before - normal_restored).length() < 1e-4);
}

#[test]
fn test_flip_diagonal_command_and_undo() {
    let mut state = AppState::default();
    state.set_edit_mode(EditMode::Edit);

    // Seleciona a primeira face (quad do cubo)
    state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    let initial_verts = state.project.active_mesh().unwrap().faces[0].verts.clone();

    // Executa FlipDiagonalCmd
    state.dispatch(&FlipDiagonalCmd).expect("flip diagonal");

    // Vértices do quad devem ter rotacionado cíclica para inverter a diagonal
    let flipped_verts = state.project.active_mesh().unwrap().faces[0].verts.clone();
    assert_ne!(initial_verts, flipped_verts);
    assert_eq!(flipped_verts[0], initial_verts[1]);

    // Undo restaura a orientação original
    assert!(state.undo());
    let restored_verts = state.project.active_mesh().unwrap().faces[0].verts.clone();
    assert_eq!(initial_verts, restored_verts);
}

#[test]
fn test_revolve_command_and_undo() {
    let mut state = AppState::default();
    state.set_edit_mode(EditMode::Edit);

    // Cria perfil aberto no mesh ativo
    let mesh = state.project.active_mesh_mut().unwrap();
    mesh.verts.clear();
    mesh.faces.clear();
    mesh.selected_edges.clear();

    mesh.verts.push(petunia_mesh::Vertex::new(1.0, 0.0, 0.0));
    mesh.verts.push(petunia_mesh::Vertex::new(1.5, 1.0, 0.0));
    mesh.verts.push(petunia_mesh::Vertex::new(1.0, 2.0, 0.0));
    for v in &mut mesh.verts {
        v.selected = true;
    }
    mesh.selected_edges.insert(petunia_mesh::edge_key(0, 1));
    mesh.selected_edges.insert(petunia_mesh::edge_key(1, 2));

    // Executa RevolveCmd (8 segmentos, 360°, eixo Y)
    let revolve_cmd = RevolveCmd {
        segments: 8,
        angle_deg: 360.0,
        axis: 1,
        center: [0.0, 0.0, 0.0],
    };
    state.dispatch(&revolve_cmd).expect("revolve profile");

    assert_eq!(state.project.active_mesh().unwrap().faces.len(), 16);

    // Undo restaura o perfil original com 0 faces
    assert!(state.undo());
    assert_eq!(state.project.active_mesh().unwrap().faces.len(), 0);
    assert_eq!(state.project.active_mesh().unwrap().verts.len(), 3);
}

#[test]
fn test_extrude_individual_command_and_undo() {
    let mut state = AppState::default();
    state.set_edit_mode(EditMode::Edit);

    // Seleciona duas faces do cubo
    state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    state.project.active_mesh_mut().unwrap().faces[1].selected = true;

    let initial_faces = state.project.active_mesh().unwrap().faces.len();
    let initial_verts = state.project.active_mesh().unwrap().verts.len();

    let extrude_cmd = ExtrudeIndividualCmd { dist: 1.0 };
    state.dispatch(&extrude_cmd).expect("extrude individual");

    let mesh_after = state.project.active_mesh().unwrap();
    // 2 faces quadrangulares extrudadas individualmente geram 8 novos vértices e 8 novas faces laterais
    assert_eq!(mesh_after.verts.len(), initial_verts + 8);
    assert_eq!(mesh_after.faces.len(), initial_faces + 8);

    // Undo restaura o cubo original
    assert!(state.undo());
    let mesh_restored = state.project.active_mesh().unwrap();
    assert_eq!(mesh_restored.verts.len(), initial_verts);
    assert_eq!(mesh_restored.faces.len(), initial_faces);
}

#[test]
fn test_select_linked_and_box_select_commands() {
    let mut state = AppState::default();
    state.set_edit_mode(EditMode::Edit);

    // Deseleciona tudo
    state.dispatch(&ClearSelectionCmd).expect("clear");
    assert_eq!(
        state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .filter(|v| v.selected)
            .count(),
        0
    );

    // Seleciona um vértice e executa SelectLinkedCmd
    state.project.active_mesh_mut().unwrap().verts[0].selected = true;
    state.dispatch(&SelectLinkedCmd).expect("select linked");

    // Todo o cubo conectado deve estar selecionado
    assert_eq!(
        state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .filter(|v| v.selected)
            .count(),
        8
    );

    // BoxSelectCmd cobrindo toda a tela NDC [-1, 1]
    state.dispatch(&ClearSelectionCmd).expect("clear");
    let vp = state.session.camera.view_proj().to_cols_array();
    let box_cmd = BoxSelectCmd {
        p0: [-1.0, -1.0],
        p1: [1.0, 1.0],
        view_proj: vp,
        add: false,
    };
    state.dispatch(&box_cmd).expect("box select");
    assert!(
        state
            .project
            .active_mesh()
            .unwrap()
            .verts
            .iter()
            .any(|v| v.selected)
    );
}

#[test]
fn test_outliner_asset_lock_and_visibility_commands_and_undo() {
    let mut state = AppState::default();
    assert!(!state.project.assets[0].locked);
    assert!(state.project.assets[0].visible);

    // 1. Toggle lock
    let toggle_lock = ToggleLockAssetCmd { asset_index: None };
    state.dispatch(&toggle_lock).expect("lock active asset");
    assert!(state.project.assets[0].locked);

    // Undo restaura lock para false
    assert!(state.undo());
    assert!(!state.project.assets[0].locked);

    // Redo re-aplica lock
    assert!(state.redo());
    assert!(state.project.assets[0].locked);

    // 2. Toggle visibility
    let toggle_vis = ToggleVisibilityAssetCmd {
        asset_index: Some(0),
    };
    state.dispatch(&toggle_vis).expect("hide active asset");
    assert!(!state.project.assets[0].visible);

    // Undo restaura visibilidade
    assert!(state.undo());
    assert!(state.project.assets[0].visible);
}

#[test]
fn test_outliner_collection_commands_and_undo() {
    let mut state = AppState::default();
    assert_eq!(state.project.assets[0].collection, None);

    // 1. Set collection
    let set_col = SetAssetCollectionCmd {
        asset_index: 0,
        collection: Some("Characters".to_string()),
    };
    state.dispatch(&set_col).expect("set collection");
    assert_eq!(
        state.project.assets[0].collection.as_deref(),
        Some("Characters")
    );

    // 2. Toggle collection lock
    let lock_col = ToggleCollectionLockCmd {
        collection: "Characters".to_string(),
    };
    state.dispatch(&lock_col).expect("lock collection");
    assert!(state.project.assets[0].locked);

    // 3. Toggle collection visibility
    let vis_col = ToggleCollectionVisibilityCmd {
        collection: "Characters".to_string(),
    };
    state.dispatch(&vis_col).expect("hide collection");
    assert!(!state.project.assets[0].visible);

    // 4. Undo reverte visibilidade da coleção
    assert!(state.undo());
    assert!(state.project.assets[0].visible);

    // Undo reverte lock da coleção
    assert!(state.undo());
    assert!(!state.project.assets[0].locked);

    // Undo reverte atribuição da coleção
    assert!(state.undo());
    assert_eq!(state.project.assets[0].collection, None);
}

#[test]
fn test_canonical_command_dispatcher_metadata_and_categories() {
    let dispatcher = petunia_core::command::CommandDispatcher::canonical();
    let all_meta = dispatcher.all_metadata();

    assert!(
        all_meta.len() >= 15,
        "Expected at least 15 canonical commands"
    );

    // All categories should be represented
    let categories: std::collections::HashSet<_> = all_meta.iter().map(|m| m.category).collect();
    assert!(categories.contains(&petunia_core::command::CommandCategory::File));
    assert!(categories.contains(&petunia_core::command::CommandCategory::Edit));
    assert!(categories.contains(&petunia_core::command::CommandCategory::Model));
    assert!(categories.contains(&petunia_core::command::CommandCategory::View));
    assert!(categories.contains(&petunia_core::command::CommandCategory::Help));

    for meta in &all_meta {
        assert!(!meta.id.is_empty());
        assert!(!meta.label.is_empty());
        assert!(!meta.description.is_empty());
    }

    // Test query and can_execute validation
    let mut state = AppState::default();

    // 1. Undo should be disabled when history is empty
    let undo_items = dispatcher.query("undo", &state);
    let undo_cmd = undo_items
        .iter()
        .find(|i| i.id == "edit.undo")
        .expect("undo command found");
    assert!(!undo_cmd.is_available);
    assert_eq!(undo_cmd.disabled_reason, Some("Nothing to undo"));

    // 2. Extrude should require Edit mode
    assert_eq!(state.edit_mode(), EditMode::Object);
    let extrude_items = dispatcher.query("extrude", &state);
    assert!(!extrude_items.is_empty());
    let extrude_cmd = extrude_items
        .iter()
        .find(|i| i.id == "model.extrude")
        .unwrap();
    assert!(!extrude_cmd.is_available);
    assert_eq!(extrude_cmd.disabled_reason, Some("Requires Edit mode"));

    // 3. Switch to Edit mode - now requires face selection
    state.set_edit_mode(EditMode::Edit);
    let extrude_items_edit = dispatcher.query("extrude", &state);
    let extrude_cmd_edit = extrude_items_edit
        .iter()
        .find(|i| i.id == "model.extrude")
        .unwrap();
    assert!(!extrude_cmd_edit.is_available);
    assert_eq!(extrude_cmd_edit.disabled_reason, Some("Select faces first"));

    // 4. Select a face
    state.project.active_mesh_mut().unwrap().faces[0].selected = true;
    let extrude_items_sel = dispatcher.query("extrude", &state);
    let extrude_cmd_sel = extrude_items_sel
        .iter()
        .find(|i| i.id == "model.extrude")
        .unwrap();
    assert!(extrude_cmd_sel.is_available);
    assert_eq!(extrude_cmd_sel.disabled_reason, None);
}

#[test]
fn test_app_state_dispatch_command_string_id() {
    let mut state = AppState::default();

    // Toggle wireframe
    assert_ne!(state.shading, petunia_core::Shading::Wireframe);
    state
        .dispatch_command("view.toggle_wireframe")
        .expect("toggle wireframe command");
    assert_eq!(state.shading, petunia_core::Shading::Wireframe);
    state
        .dispatch_command("view.toggle_wireframe")
        .expect("toggle wireframe command again");
    assert_ne!(state.shading, petunia_core::Shading::Wireframe);

    // Toggle command palette
    assert!(!state.ui.show_command_palette);
    state
        .dispatch_command("window.command_palette")
        .expect("toggle command palette");
    assert!(state.ui.show_command_palette);

    // Invalid command ID fails
    let err = state.dispatch_command("invalid.command.id");
    assert!(err.is_err());
}

#[test]
fn test_view_preset_commands_and_hud() {
    let mut state = AppState::default();

    // 1. View front
    state.dispatch_command("view.front").expect("view.front");
    let (name, _, _) = state.camera.nominal_view();
    assert_eq!(name, "Front Ortho");
    assert_eq!(state.camera.proj, petunia_core::camera::Projection::Ortho);

    // 2. View top
    state.dispatch_command("view.top").expect("view.top");
    let (name, _, _) = state.camera.nominal_view();
    assert_eq!(name, "Top Ortho");

    // 3. View isometric NE
    state
        .dispatch_command("view.isometric_ne")
        .expect("view.isometric_ne");
    let (name, _, _) = state.camera.nominal_view();
    assert_eq!(name, "Isometric NE");

    // 4. View isometric SW
    state
        .dispatch_command("view.isometric_sw")
        .expect("view.isometric_sw");
    let (name, _, _) = state.camera.nominal_view();
    assert_eq!(name, "Isometric SW");

    // 5. Toggle Nav HUD
    assert!(state.show_nav_hud);
    state
        .dispatch_command("view.toggle_nav_hud")
        .expect("toggle nav hud");
    assert!(!state.show_nav_hud);
    state
        .dispatch_command("view.toggle_nav_hud")
        .expect("toggle nav hud back");
    assert!(state.show_nav_hud);

    // 6. Toggle Reference Manager
    assert!(!state.ui.show_reference_manager);
    state
        .dispatch_command("window.reference_manager")
        .expect("toggle reference manager");
    assert!(state.ui.show_reference_manager);
}

#[test]
fn test_frame_all_and_selection_dont_dirty_project() {
    let mut state = AppState::default();
    assert!(!state.project.is_dirty());

    // Frame Selection
    state
        .dispatch_command("view.frame_selection")
        .expect("frame selection");
    assert!(state.camera_frame.is_some());
    assert!(
        !state.project.is_dirty(),
        "frame_selection must not mark project dirty"
    );

    // Frame All
    state.dispatch_command("view.frame_all").expect("frame all");
    assert!(state.camera_frame.is_some());
    assert!(
        !state.project.is_dirty(),
        "frame_all must not mark project dirty"
    );
}

#[test]
fn test_instantiate_asset_command_and_undo() {
    let mut state = AppState::default();
    let asset_id = state.project.assets[0].id;
    let initial_count = state.project.assets.len();

    // 1. Instancia asset em posição específica
    let cmd = petunia_core::InstantiateAssetCmd {
        asset_id,
        position: Some([5.0, 0.0, -2.0]),
    };
    state.dispatch(&cmd).expect("deve instanciar asset");
    assert_eq!(state.project.assets.len(), initial_count + 1);
    assert_eq!(state.project.active, initial_count);

    let instantiated = &state.project.assets[state.project.active];
    let center = instantiated.mesh.selection_center();
    assert!((center[0] - 5.0).abs() < 1e-4);
    assert!((center[1] - 0.0).abs() < 1e-4);
    assert!((center[2] - (-2.0)).abs() < 1e-4);

    // 2. Undo restaura estado anterior
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), initial_count);

    // 3. Redo restaura a instância
    assert!(state.redo());
    assert_eq!(state.project.assets.len(), initial_count + 1);

    // 4. Teste via método auxiliar instantiate_asset_by_id
    assert!(state.instantiate_asset_by_id(asset_id, Some([10.0, 2.0, 3.0])));
    assert_eq!(state.project.assets.len(), initial_count + 2);
    let center2 = state.project.assets.last().unwrap().mesh.selection_center();
    assert!((center2[0] - 10.0).abs() < 1e-4);

    // 5. Instanciação com ID inexistente falha graciosamente
    assert!(!state.instantiate_asset_by_id(uuid::Uuid::new_v4(), None));
}

#[test]
fn test_ui_state_defaults_wave_6() {
    let state = AppState::default();
    assert_eq!(state.ui.asset_thumbnail_size, 64.0);
    assert!(!state.ui.inspector_detached);
}

#[test]
fn test_rename_active_asset_validates_and_commits_one_undo_entry() {
    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let original = state.project.active().unwrap().name.clone();
    assert_eq!(original, "Cube");

    // Nome vazio (ou só espaços) é recusado sem tocar no histórico.
    assert_eq!(
        state.rename_active_asset("   "),
        Err(AssetRenameError::EmptyName)
    );
    assert_eq!(state.project.undo.depth(), (0, 0));

    // Nome acima do limite canônico também.
    let long = "x".repeat(ASSET_NAME_MAX_LEN + 1);
    assert_eq!(
        state.rename_active_asset(&long),
        Err(AssetRenameError::NameTooLong)
    );
    assert_eq!(state.project.undo.depth(), (0, 0));

    // Espaços nas pontas são removidos e o rename vira uma única entrada.
    assert_eq!(state.rename_active_asset("  Turret Base  "), Ok(true));
    assert_eq!(state.project.active().unwrap().name, "Turret Base");
    assert_eq!(state.project.undo.depth(), (1, 0));

    // Confirmar o mesmo nome é idempotente: sem entrada nova.
    assert_eq!(state.rename_active_asset("Turret Base"), Ok(false));
    assert_eq!(state.project.undo.depth(), (1, 0));

    // O limite exato é aceito.
    let exact = "y".repeat(ASSET_NAME_MAX_LEN);
    assert_eq!(state.rename_active_asset(&exact), Ok(true));
    assert_eq!(state.project.active().unwrap().name, exact);

    // Undo volta para o nome anterior, não para o original.
    assert!(state.undo());
    assert_eq!(state.project.active().unwrap().name, "Turret Base");
    assert!(state.undo());
    assert_eq!(state.project.active().unwrap().name, original);
}

#[test]
fn test_alias_does_not_duplicate_command_ids_in_the_catalog() {
    let dispatcher = petunia_core::command::CommandDispatcher::canonical();
    let ids: Vec<&str> = dispatcher
        .all_metadata()
        .iter()
        .map(|meta| meta.id.as_str())
        .collect();
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        ids.len(),
        unique.len(),
        "o catálogo publica ids repetidos: {ids:?}"
    );

    // O alias existe e aponta para o mesmo comando, mas com id próprio.
    assert!(dispatcher.contains("model.delete"));
    assert!(dispatcher.contains("edit.delete"));
    let alias_meta = dispatcher.get_metadata("model.delete").unwrap();
    assert_eq!(alias_meta.id, "model.delete");
    let canonical_meta = dispatcher.get_metadata("edit.delete").unwrap();
    assert_eq!(canonical_meta.id, "edit.delete");
    assert_eq!(alias_meta.label, canonical_meta.label);
}

#[test]
fn test_boolean_fuse_and_cut_between_active_and_operand() {
    use petunia_core::BooleanOpCmd;
    use petunia_mesh::boolean::BooleanOp;

    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    state.project.assets[0].mesh = petunia_mesh::Mesh::cube(2.0);
    state.project.assets[0].name = "A".to_string();
    let a_id = state.project.assets[0].id;

    // Sem operando escolhido a operação precisa ser recusada.
    assert!(
        state
            .dispatch(&BooleanOpCmd::new(BooleanOp::Union))
            .is_err()
    );
    assert_eq!(state.project.assets.len(), 1);

    // Cria B deslocado no eixo X para que a união tenha volume maior.
    let mut b = petunia_mesh::Mesh::cube(2.0);
    b.select_all();
    b.translate_selected([1.0, 0.0, 0.0]);
    b.deselect_all();
    state.project.add("B", b);
    let b_id = state.project.assets[1].id;
    state.project.active = 0;
    state.session.tools.boolean_operand = Some(b_id);

    let before = state.project.assets[0].mesh.verts.len();
    let undo_before = state.project.undo.depth().0;
    assert!(state.dispatch(&BooleanOpCmd::new(BooleanOp::Union)).is_ok());
    assert_eq!(
        state.project.undo.depth().0,
        undo_before + 1,
        "Fuse é exatamente uma entrada de undo"
    );
    assert_eq!(state.project.assets.len(), 1, "B é consumido");
    assert!(state.project.assets[0].mesh.verts.len() > before);
    assert!(state.session.tools.boolean_operand.is_none());
    assert_eq!(state.project.active, 0);
    assert_eq!(state.project.assets[0].id, a_id);

    // Undo restaura os dois objetos.
    assert!(state.undo());
    assert_eq!(state.project.assets.len(), 2);
    assert!(state.project.assets.iter().any(|a| a.id == b_id));

    // Cut: o operando precisa existir de novo.
    state.project.active = 0;
    state.session.tools.boolean_operand = Some(b_id);
    assert!(
        state
            .dispatch(&BooleanOpCmd::new(BooleanOp::Difference))
            .is_ok()
    );
    assert_eq!(state.project.assets.len(), 1);
    assert_eq!(state.project.assets[0].id, a_id);
}

#[test]
fn test_boolean_refuses_the_active_asset_as_operand_and_missing_operand() {
    use petunia_core::BooleanOpCmd;
    use petunia_mesh::boolean::BooleanOp;

    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    let active = state.project.assets[0].id;

    state.session.tools.boolean_operand = Some(active);
    let err = state
        .dispatch(&BooleanOpCmd::new(BooleanOp::Union))
        .unwrap_err()
        .to_string();
    assert!(err.contains("active"), "veio: {err}");
    assert_eq!(state.project.assets.len(), 1);

    state.session.tools.boolean_operand = Some(uuid::Uuid::new_v4());
    assert!(
        state
            .dispatch(&BooleanOpCmd::new(BooleanOp::Difference))
            .is_err()
    );
    assert_eq!(state.project.assets.len(), 1);
}

#[test]
fn test_keep_parts_keeps_the_operand_in_the_scene() {
    use petunia_core::BooleanOpCmd;
    use petunia_mesh::boolean::BooleanOp;

    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    state.project.assets[0].mesh = petunia_mesh::Mesh::cube(2.0);

    let mut b = petunia_mesh::Mesh::cube(2.0);
    b.select_all();
    b.translate_selected([1.6, 0.0, 0.0]);
    b.deselect_all();
    state.project.add("B", b);
    let b_id = state.project.assets[1].id;
    state.project.active = 0;
    state.session.tools.boolean_operand = Some(b_id);
    state.session.tools.boolean_keep_parts = true;

    assert!(state.dispatch(&BooleanOpCmd::new(BooleanOp::Union)).is_ok());
    assert_eq!(
        state.project.assets.len(),
        2,
        "Keep Parts mantém o operando na cena"
    );
    assert!(state.project.assets.iter().any(|a| a.id == b_id));
    assert_eq!(state.project.active, 0, "o ativo continua sendo A");
    assert!(state.session.tools.boolean_operand.is_none());
}

#[test]
fn test_join_merges_both_topologies_without_a_boolean_kernel() {
    use petunia_core::JoinObjectsCmd;

    let mut state = AppState::new("en");
    ProjectService::new_project(&mut state);
    state.project.assets[0].mesh = petunia_mesh::Mesh::cube(2.0);
    let a_verts = state.project.assets[0].mesh.verts.len();
    let a_faces = state.project.assets[0].mesh.faces.len();

    let mut b = petunia_mesh::Mesh::cube(2.0);
    b.select_all();
    b.translate_selected([5.0, 0.0, 0.0]);
    b.deselect_all();
    state.project.add("B", b);
    let b_id = state.project.assets[1].id;
    let b_verts = state.project.assets[1].mesh.verts.len();
    let b_faces = state.project.assets[1].mesh.faces.len();
    state.project.active = 0;
    state.session.tools.boolean_operand = Some(b_id);

    let undo_before = state.project.undo.depth().0;
    assert!(state.dispatch(&JoinObjectsCmd).is_ok());
    assert_eq!(
        state.project.undo.depth().0,
        undo_before + 1,
        "Join é exatamente uma entrada de undo"
    );
    assert_eq!(state.project.assets.len(), 1);
    let mesh = state.project.active_mesh().unwrap();
    assert_eq!(
        mesh.verts.len(),
        a_verts + b_verts,
        "Join preserva os vértices das duas malhas"
    );
    assert_eq!(mesh.faces.len(), a_faces + b_faces);

    assert!(state.undo());
    assert_eq!(
        state.project.assets.len(),
        2,
        "undo restaura os dois objetos"
    );
}

#[test]
fn test_project_from_reference_and_bake_commands() {
    let mut state = AppState::default();

    // 1. Project from reference falls back to view when no reference images exist
    assert!(state.dispatch_command("uv.project_reference").is_ok());
    assert_eq!(state.project.undo.depth().0, 1);

    // 2. Add reference image
    let ref_rgba = vec![
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 0, 255,
    ];
    state.project.refs.push(petunia_core::ReferenceImage {
        name: "Test Ref".into(),
        width: 2,
        height: 2,
        rgba: ref_rgba,
        axis: petunia_core::RefAxis::Front,
        offset: 0.0,
        size: 2.0,
        opacity: 1.0,
        visible: true,
        locked: false,
        rotation: 0.0,
        xray: false,
        revision: petunia_core::ReferenceImage::next_revision(),
    });

    // 3. Project from reference with active reference image
    assert!(state.dispatch_command("uv.project_reference").is_ok());
    assert_eq!(state.project.undo.depth().0, 2);

    // 4. Bake reference to texture
    assert!(state.dispatch_command("paint.bake_reference").is_ok());
    assert_eq!(state.project.undo.depth().0, 3);
    let asset = state.project.active().unwrap();
    assert!(asset.texture.is_some());
    let tex = asset.texture.as_ref().unwrap();
    assert_eq!(tex.w, 2);
    assert_eq!(tex.h, 2);
    // Texel (0,0) must have baked the reference red color
    assert_eq!(tex.pixels[0], 255);
    assert_eq!(tex.pixels[1], 0);
    assert_eq!(tex.pixels[2], 0);

    // 5. Undo restores previous state
    assert!(state.undo());
    assert_eq!(state.project.undo.depth().0, 2);
}

#[test]
fn test_decal_commands_transform_and_bake() {
    // Tests SetDecalTransformCmd and BakeDecalCmd transactional execution and undo/redo
    // Testa execução transacional e desfazer/refazer de SetDecalTransformCmd e BakeDecalCmd
    let mut state = AppState::default();
    let decal_canvas = petunia_project::Canvas::new(8, 8, [255, 128, 0, 255]);
    let decal =
        petunia_project::paint_layers::DecalLayer::new(decal_canvas, [0.5, 0.5], [0.25, 0.25], 0.0);
    let decal_id = {
        let asset = state.project.active_mut().unwrap();
        let stack = asset.paint_stack.get_or_insert_with(|| {
            petunia_project::paint_layers::PaintLayerStack::with_base(
                "Base",
                petunia_project::Canvas::new(32, 32, [0, 0, 0, 255]),
            )
        });
        stack.add_layer(petunia_project::paint_layers::PaintLayer::new_decal(
            "Decal 1", decal,
        ))
    };

    // 1. Dispatch SetDecalTransformCmd / Despacha SetDecalTransformCmd
    assert!(
        state
            .dispatch(&SetDecalTransformCmd {
                layer_id: decal_id,
                center_uv: [0.6, 0.4],
                scale_uv: [0.3, 0.3],
                rotation_rad: 1.57,
            })
            .is_ok()
    );

    let asset = state.project.active().unwrap();
    let stack = asset.paint_stack.as_ref().unwrap();
    if let petunia_project::paint_layers::LayerKind::Decal(d) =
        &stack.layers.iter().find(|l| l.id == decal_id).unwrap().kind
    {
        assert_eq!(d.center_uv, [0.6, 0.4]);
        assert_eq!(d.scale_uv, [0.3, 0.3]);
        assert_eq!(d.rotation_rad, 1.57);
    } else {
        panic!("expected Decal layer / esperava camada Decal");
    }

    // 2. Dispatch BakeDecalCmd / Despacha BakeDecalCmd
    assert!(state.dispatch(&BakeDecalCmd { layer_id: decal_id }).is_ok());
    let asset = state.project.active().unwrap();
    let stack = asset.paint_stack.as_ref().unwrap();
    assert!(matches!(
        stack.layers.iter().find(|l| l.id == decal_id).unwrap().kind,
        petunia_project::paint_layers::LayerKind::Raster(_)
    ));

    // 3. Undo restores decal layer / Undo restaura camada decal
    assert!(state.undo());
    let asset = state.project.active().unwrap();
    let stack = asset.paint_stack.as_ref().unwrap();
    assert!(matches!(
        stack.layers.iter().find(|l| l.id == decal_id).unwrap().kind,
        petunia_project::paint_layers::LayerKind::Decal(_)
    ));
}

#[test]
fn profile_owned_spline_rejects_edits_that_break_planarity() {
    let mut state = AppState::default();
    let mut spline = SplineResource::new("Profile curve", SplineInterpolation::CubicBezier);
    let first = SplinePoint::new([0.0, 0.0, 0.0]);
    let first_id = first.id;
    spline.add_point(first).unwrap();
    let spline_id = spline.id;
    let profile = ProfileResource::new("Planar", spline_id, ProfileWorkplane::default());
    state
        .dispatch(&CreateProfileCmd { spline, profile })
        .unwrap();
    let depth = state.project.undo.depth();

    // Sair do plano do Profile por qualquer comando de spline deve ser recusado.
    assert!(
        state
            .dispatch(&MoveSplinePointCmd {
                spline_id,
                point_id: first_id,
                position: [0.0, 0.0, 0.5],
            })
            .is_err()
    );
    assert!(
        state
            .dispatch(&AddSplinePointCmd {
                spline_id,
                index: None,
                point: SplinePoint::new([1.0, 0.0, 0.25]),
            })
            .is_err()
    );
    assert!(
        state
            .dispatch(&SetSplineHandlesCmd {
                spline_id,
                point_id: first_id,
                handle_in: [0.0, 0.0, 0.0],
                handle_out: [0.5, 0.0, 0.5],
                mode: SplineHandleMode::Broken,
            })
            .is_err()
    );
    assert_eq!(state.project.undo.depth(), depth);

    // Edições planares continuam funcionando.
    state
        .dispatch(&MoveSplinePointCmd {
            spline_id,
            point_id: first_id,
            position: [0.5, 0.5, 0.0],
        })
        .unwrap();
    assert_eq!(state.project.undo.depth().0, depth.0 + 1);
}

#[test]
fn no_op_command_creates_no_history_entry_and_keeps_session() {
    use petunia_core::command::PrimitiveKind;
    let mut state = AppState::default();
    assert!(state.begin_primitive(PrimitiveKind::Cube, None));
    let had_session = state.session.primitive_session.is_some();
    assert!(had_session);
    state.finalize_primitive_session();
    // Com uma única face selecionada nenhuma aresta compartilhada entra no
    // filtro de costura, então o comando não tem o que alterar.
    state.session.uv_selected.insert(0);
    let depth = state.project.undo.depth();
    let clock = state.project.project.revision_clock();
    let result = state.dispatch(&UvStitchCmd);
    assert!(
        matches!(result, Err(CommandError::NoChange(_))),
        "{result:?}"
    );
    assert_eq!(state.project.undo.depth(), depth);
    assert_eq!(state.project.project.revision_clock(), clock);
}

#[test]
fn failed_dispatch_restores_primitive_creation_session() {
    use petunia_core::command::PrimitiveKind;
    let mut state = AppState::default();
    assert!(state.begin_primitive(PrimitiveKind::Cube, None));
    assert!(state.session.primitive_session.is_some());
    let last = state.session.last_primitive;
    // Dispatch destrutivo que falha depois de congelar a primitiva: a sessão
    // de criação precisa voltar junto com o projeto.
    let result = state.dispatch(&DissolveCmd);
    if result.is_err() {
        assert!(state.session.primitive_session.is_some());
        assert_eq!(state.session.last_primitive, last);
    }
}

#[test]
fn shade_smooth_is_per_object_transactional_and_persistent() {
    use petunia_core::command::SetShadeSmoothCmd;
    let mut state = AppState::default();
    let id = state.project.active().unwrap().id;
    assert!(!state.project.project.is_smooth_shaded(id));
    let normals_before = state.project.project.normal_revision;

    state.dispatch(&SetShadeSmoothCmd { smooth: true }).unwrap();
    assert!(state.project.project.is_smooth_shaded(id));
    assert!(state.project.project.normal_revision > normals_before);
    // Repetir é no-op: sem entrada de histórico.
    let depth = state.project.undo.depth();
    assert!(state.dispatch(&SetShadeSmoothCmd { smooth: true }).is_err());
    assert_eq!(state.project.undo.depth(), depth);

    // Persistência por objeto.
    let bytes = petunia_project::format::encode_zip(&state.project.project).unwrap();
    let back = petunia_project::format::load_bytes(&bytes).unwrap();
    assert!(back.is_smooth_shaded(id));

    assert!(state.undo());
    assert!(!state.project.project.is_smooth_shaded(id));
    assert!(state.redo());
    assert!(state.project.project.is_smooth_shaded(id));
    state
        .dispatch(&SetShadeSmoothCmd { smooth: false })
        .unwrap();
    assert!(!state.project.project.is_smooth_shaded(id));
}
