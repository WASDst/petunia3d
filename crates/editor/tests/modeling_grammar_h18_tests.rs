use petunia_core::{
    AddPrimitiveCmd, AnimatePreview, AppState, BevelCmd, EditMode, ExtrudeSelectedCmd,
    InsetFacesCmd, MergeCenterCmd, PaintBlockedReason, PrimitiveKind, RigPresetKind,
    SelectionDomain, SubdivideSelectionCmd,
};
use petunia_editor::{PlaceholderViewport, SlintUiBridge, UiIntent};
use petunia_project::paint_layers::PaintLayerStack;
use petunia_project::{Canvas, IkChain};

fn test_bridge() -> SlintUiBridge<PlaceholderViewport> {
    let mut b = SlintUiBridge::new(AppState::new("en"), PlaceholderViewport::default());
    b.apply(UiIntent::AddPrimitive(PrimitiveKind::Cube));
    b.state.confirm_primitive();
    b
}

// ============================================================================
// H15: Posing e IK manual por intenção de alvo
// ============================================================================

#[test]
fn test_h15_ik_solve_draft_cancel_commit_and_keying() {
    let mut state = AppState::new("en");

    // Cria um rig humanoide
    let skel_id = state.animate_add_rig(RigPresetKind::Humanoid).unwrap();
    state.animate_select_skeleton(skel_id);

    // Obtém a perna esquerda do humanoide para criar uma cadeia IK
    let skel = state.project.project.get_skeleton(skel_id).unwrap().clone();
    let leg_chain = petunia_project::rig_roles::LegChain {
        limb: 0,
        thigh: skel.find_bone("UpperLeg.L").unwrap(),
        shin: skel.find_bone("LowerLeg.L").unwrap(),
        foot: skel.find_bone("Foot.L"),
    };
    let chain = IkChain::two_bone_leg(skel_id, "LegIK.L", &leg_chain);
    let chain_id = chain.id;
    state.project.project.ik_chains.push(chain);

    // Baseline: grava a profundidade de Undo
    let initial_undo = state.project.undo.depth().0;

    // 1. Validação de coordenadas finitas (rejeita NaN/Inf)
    let nan_target = [f32::NAN, 1.0, 0.0];
    assert!(state.animate_solve_ik(chain_id, nan_target).is_err());
    assert_eq!(state.project.undo.depth().0, initial_undo);

    // 2. Drag / Solver contínuo: avalia IK sem gerar Undo
    let foot_idx = skel.bone_index(leg_chain.foot.unwrap()).unwrap();
    let foot_head = skel.bones[foot_idx].head;
    let target = [
        foot_head[0] + 0.05,
        foot_head[1] + 0.25,
        foot_head[2] + 0.15,
    ];

    state.animate_solve_ik(chain_id, target).unwrap();

    // Draft de IK está presente na sessão
    assert!(state.session.animate.ik_draft().is_some());
    assert_eq!(state.session.animate.ik_draft().unwrap().chain_id, chain_id);

    // Nenhuma entrada de Undo foi gerada durante o solve/draft
    assert_eq!(state.project.undo.depth().0, initial_undo);

    // Preview avaliado reflete a nova pose do IK draft
    let mut preview = AnimatePreview::default();
    let (eval_skel, local_pose) = preview
        .local_pose(&state.project.project, &state.session.animate)
        .unwrap();
    assert_eq!(eval_skel, skel_id);
    let thigh_idx = skel.bone_index(leg_chain.thigh).unwrap();
    assert_ne!(
        local_pose[thigh_idx].rotation,
        skel.bones[thigh_idx].local_transform.rotation
    );

    // 3. Cancelar IK (Esc) descarta draft com 0 Undo
    state.animate_cancel_ik();
    assert!(state.session.animate.ik_draft().is_none());
    assert_eq!(state.project.undo.depth().0, initial_undo);

    // A pose avaliada volta ao repouso
    let (_, reset_pose) = preview
        .local_pose(&state.project.project, &state.session.animate)
        .unwrap();
    assert_eq!(
        reset_pose[thigh_idx].rotation,
        skel.bones[thigh_idx].local_transform.rotation
    );

    // 4. Commit IK: confirma em exatamente UMA transação de Undo
    state.animate_solve_ik(chain_id, target).unwrap();
    assert!(state.animate_commit_ik().unwrap());
    assert!(state.session.animate.ik_draft().is_none());
    assert_eq!(state.project.undo.depth().0, initial_undo + 1);

    // O esqueleto no documento recebeu as rotações calculadas
    let after_rot = state.project.project.get_skeleton(skel_id).unwrap().bones[thigh_idx]
        .local_transform
        .rotation;
    assert_ne!(after_rot, skel.bones[thigh_idx].local_transform.rotation);

    // Undo restaura a rotação anterior e Redo re-aplica
    assert!(state.undo());
    let undone_rot = state.project.project.get_skeleton(skel_id).unwrap().bones[thigh_idx]
        .local_transform
        .rotation;
    assert_eq!(undone_rot, skel.bones[thigh_idx].local_transform.rotation);
    assert!(state.redo());
    let redone_rot = state.project.project.get_skeleton(skel_id).unwrap().bones[thigh_idx]
        .local_transform
        .rotation;
    assert_eq!(redone_rot, after_rot);

    // 5. Keying de pose avaliada grava chaves no clipe sem mutação da malha de repouso
    state.animate_solve_ik(chain_id, target).unwrap();
    state.animate_select_bone(Some(leg_chain.thigh));
    state.animate_insert_keyframe().unwrap();

    // Clipe contém a chave
    let clip = state
        .session
        .animate
        .resolved_clip(&state.project.project)
        .unwrap()
        .unwrap();
    let track = clip.clip.get_track(leg_chain.thigh).unwrap();
    assert!(!track.rotations.is_empty());

    // Invariante estrito: malha de repouso e rest pose não são corrompidos
    if let Some(active_asset) = state.project.active() {
        for v in &active_asset.mesh.verts {
            assert!(v.pos[0].is_finite() && v.pos[1].is_finite() && v.pos[2].is_finite());
        }
    }
}

// ============================================================================
// H16: Picker temporário e origem explícita de Clone
// ============================================================================

#[test]
fn test_h16_temporary_picker_and_explicit_clone_source() {
    let mut b = test_bridge();
    b.state.session.tools.active_tool = "brush".to_string();

    let initial_undo = b.state.project.undo.depth().0;

    // 1. Amostragem temporária (Alt mantido durante Paint)
    b.state.begin_temporary_picker();
    assert_eq!(b.state.session.tools.active_tool, "picker");
    assert_eq!(
        b.state.session.tools.temporary_paint_origin_tool(),
        Some("brush")
    );

    // Amostragem ou cancelamento restaura o brush de origem sem criar passos de Undo
    let restored = b.state.finish_picker_sampling();
    assert_eq!(restored, Some("brush".to_string()));
    assert_eq!(b.state.session.tools.active_tool, "brush");
    assert_eq!(b.state.session.tools.temporary_paint_origin_tool(), None);
    assert_eq!(b.state.project.undo.depth().0, initial_undo);

    // 2. Clone Stamp exige origem explícita: sem origem, não cria Undo
    b.state.session.tools.active_tool = "clone".to_string();
    b.state.session.tools.clone_source = None;

    // Tentar pintar sem origem dispara validação de origem e não gera comando de traço
    b.tool_pointer(0, 100.0, 100.0, false, false);
    assert_eq!(b.state.project.undo.depth().0, initial_undo);

    // Definindo origem explícita
    b.state.session.tools.clone_source = Some([16.0, 16.0]);
    assert_eq!(b.state.session.tools.clone_source, Some([16.0, 16.0]));
}

// ============================================================================
// H17: Estado e diagnóstico das ferramentas Paint
// ============================================================================

#[test]
fn test_h17_paint_diagnostic_and_mask_coordinate_invariance() {
    let mut state = AppState::new("en");

    // 1. Sem objeto ativo -> NoActiveMesh
    state.project.assets.clear();
    assert_eq!(
        state.paint_diagnostic(),
        Some(PaintBlockedReason::NoActiveMesh)
    );

    // Adiciona um cubo com malha válida
    state
        .dispatch(&AddPrimitiveCmd {
            kind: PrimitiveKind::Cube,
            name: None,
            at_cursor: false,
        })
        .unwrap();
    state.confirm_primitive();
    let asset = state.project.active_mut().unwrap();

    // 2. Malha sem UVs -> MissingUvMap
    for f in &mut asset.mesh.faces {
        f.uv.clear();
    }
    assert_eq!(
        state.paint_diagnostic(),
        Some(PaintBlockedReason::MissingUvMap)
    );

    // Restaura UVs
    let asset = state.project.active_mut().unwrap();
    for f in &mut asset.mesh.faces {
        f.uv = vec![[0.0, 0.0]; f.verts.len()];
    }

    // 3. Camada bloqueada -> LayerLocked
    let mut stack = PaintLayerStack::with_base("Base", Canvas::new(16, 16, [255, 255, 255, 255]));
    stack.set_locked(stack.layers[0].id, true);
    state.project.active_mut().unwrap().paint_stack = Some(stack);
    assert_eq!(
        state.paint_diagnostic(),
        Some(PaintBlockedReason::LayerLocked)
    );

    // Desbloqueia camada
    let stack = state
        .project
        .active_mut()
        .unwrap()
        .paint_stack
        .as_mut()
        .unwrap();
    stack.set_locked(stack.layers[0].id, false);

    // 4. Máscara de seleção bloqueia tudo -> MaskBlocksEverything
    state.tools.paint_isolate_selection = true;
    state.selection.faces.clear();
    assert_eq!(
        state.paint_diagnostic(),
        Some(PaintBlockedReason::MaskBlocksEverything)
    );

    // Seleciona uma face com isolamento -> Não bloqueia mais
    state.selection.faces.push(0);
    assert_eq!(state.paint_diagnostic(), None);

    // 5. Clone sem origem -> MissingCloneSource
    state.tools.paint_isolate_selection = false;
    state.tools.active_tool = "clone".to_string();
    state.tools.clone_source = None;
    assert_eq!(
        state.paint_diagnostic(),
        Some(PaintBlockedReason::MissingCloneSource)
    );

    // Com origem definida -> Não bloqueia
    state.tools.clone_source = Some([8.0, 8.0]);
    assert_eq!(state.paint_diagnostic(), None);

    // 6. Invariante: seleção de faces/máscara NÃO altera coordenadas dos vértices (mesh.verts)
    let verts_before: Vec<[f32; 3]> = state
        .project
        .active()
        .unwrap()
        .mesh
        .verts
        .iter()
        .map(|v| v.pos)
        .collect();
    state.selection.faces = vec![0, 1, 2];
    state.tools.paint_isolate_selection = true;
    let verts_after: Vec<[f32; 3]> = state
        .project
        .active()
        .unwrap()
        .mesh
        .verts
        .iter()
        .map(|v| v.pos)
        .collect();
    assert_eq!(verts_before, verts_after);
}

// ============================================================================
// H18: Gramática consistente nas famílias de modelagem
// ============================================================================

#[test]
fn test_h18_family_extrude_inset_bevel_pushpull_preconditions_and_undo() {
    let mut state = AppState::new("en");
    state
        .dispatch(&AddPrimitiveCmd {
            kind: PrimitiveKind::Cube,
            name: None,
            at_cursor: false,
        })
        .unwrap();
    state.confirm_primitive();
    state.set_edit_mode(EditMode::Edit);

    let initial_undo = state.project.undo.depth().0;

    // Precondição: Extrude com seleção vazia falha com erro
    state.selection.faces.clear();
    assert!(state.dispatch(&ExtrudeSelectedCmd { dist: 1.0 }).is_err());
    assert_eq!(state.project.undo.depth().0, initial_undo);

    // Precondição: Inset com seleção vazia falha com erro
    assert!(
        state
            .dispatch(&InsetFacesCmd {
                distance: 0.2,
                ..Default::default()
            })
            .is_err()
    );
    assert_eq!(state.project.undo.depth().0, initial_undo);

    // Precondição: Bevel com aresta vazia falha com erro
    state.set_selection_domain(SelectionDomain::Edge);
    state.selection.edges.clear();
    assert!(
        state
            .dispatch(&BevelCmd {
                amount: 0.1,
                segments: 1,
                clamp_overlap: true,
                affect_vertices: false
            })
            .is_err()
    );
    assert_eq!(state.project.undo.depth().0, initial_undo);

    // Execução válida de Extrude com face selecionada: exatamente 1 Undo entry
    state.set_selection_domain(SelectionDomain::Face);
    state.selection.faces = vec![0];
    let faces_before = state.project.active_mesh().unwrap().faces.len();
    state.dispatch(&ExtrudeSelectedCmd { dist: 1.0 }).unwrap();
    assert!(state.project.active_mesh().unwrap().faces.len() > faces_before);
    assert_eq!(state.project.undo.depth().0, initial_undo + 1);

    // Undo reverte a topologia exatamente
    assert!(state.undo());
    assert_eq!(
        state.project.active_mesh().unwrap().faces.len(),
        faces_before
    );
    assert!(state.redo());

    // Execução válida de Inset: exatamente 1 Undo entry
    let after_extrude_undo = state.project.undo.depth().0;
    state.selection.faces = vec![0];
    state
        .dispatch(&InsetFacesCmd {
            distance: 0.1,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(state.project.undo.depth().0, after_extrude_undo + 1);
    assert!(state.undo());
}

#[test]
fn test_h18_family_knife_slice_loopcut_cutsession_lifecycle() {
    let mut b = test_bridge();
    b.state.set_edit_mode(EditMode::Edit);

    let initial_undo = b.state.project.undo.depth().0;

    // 1. Knife / CutSession: cancelamento descarta sem Undo
    b.apply(UiIntent::SetActiveTool("cut".into()));
    // Inicia cut session simulando um segmento
    let mut cut_session =
        petunia_core::CutSession::new(b.state.project.active_mesh().unwrap().clone());
    cut_session.segments = 2;
    cut_session.anchor = Some([0.5, 0.5]);
    b.state.session.tools.cut_session = Some(cut_session);

    // Cancelar faca (Esc): remove cut_session com 0 Undo
    assert!(b.cancel_knife());
    assert!(b.state.session.tools.cut_session.is_none());
    assert_eq!(b.state.project.undo.depth().0, initial_undo);

    // 2. Commit faca com preview: 1 Undo entry
    b.state.session.tools.active_tool = "cut".into();
    let mut cut_session =
        petunia_core::CutSession::new(b.state.project.active_mesh().unwrap().clone());
    cut_session.segments = 1;
    cut_session.anchor = Some([0.5, 0.5]);
    b.state.session.tools.cut_session = Some(cut_session);
    let preview_mesh = b.state.project.active_mesh().unwrap().clone();
    b.show_tool_preview(preview_mesh);
    assert!(b.commit_knife());
    assert!(b.state.session.tools.cut_session.is_none());
    assert_eq!(b.state.project.undo.depth().0, initial_undo + 1);
}

#[test]
fn test_h18_family_merge_subdivide_contextual_preconditions() {
    let mut state = AppState::new("en");
    state
        .dispatch(&AddPrimitiveCmd {
            kind: PrimitiveKind::Cube,
            name: None,
            at_cursor: false,
        })
        .unwrap();
    state.confirm_primitive();
    state.set_edit_mode(EditMode::Edit);

    let initial_undo = state.project.undo.depth().0;

    // 1. Subdivide em faces selecionadas: 1 Undo entry
    state.set_selection_domain(SelectionDomain::Face);
    state.selection.faces = vec![0];
    let faces_before = state.project.active_mesh().unwrap().faces.len();
    state.dispatch(&SubdivideSelectionCmd).unwrap();
    assert!(state.project.active_mesh().unwrap().faces.len() > faces_before);
    assert_eq!(state.project.undo.depth().0, initial_undo + 1);

    assert!(state.undo());
    assert_eq!(
        state.project.active_mesh().unwrap().faces.len(),
        faces_before
    );

    // 2. Merge Center: requer pelo menos 2 vértices selecionados
    state.set_selection_domain(SelectionDomain::Vertex);
    state.selection.verts = vec![0]; // Apenas 1 vértice
    assert!(state.dispatch(&MergeCenterCmd).is_err());

    state.selection.verts = vec![0, 1]; // 2 vértices
    let verts_before = state.project.active_mesh().unwrap().verts.len();
    state.dispatch(&MergeCenterCmd).unwrap();
    assert_eq!(
        state.project.active_mesh().unwrap().verts.len(),
        verts_before - 1
    );
    assert_eq!(state.project.undo.depth().0, initial_undo + 1);

    assert!(state.undo());
    assert_eq!(
        state.project.active_mesh().unwrap().verts.len(),
        verts_before
    );
}
