//! Testes de Composição Sólida Reeditável e Múltiplos Sólidos (H26).
//!
//! Valida:
//! - Multi-solid chaining (3 cubos encadeados sequencialmente: base + op1 + op2);
//! - Orçamento rígido (`ShapeBuilderBudget`: profundidade, triângulos de entrada e decompostos);
//! - Invalidação explícita (`check_invalidation`) e status `NeedsEvaluation` ao alterar malha ou transform;
//! - Invalidação (`Invalidated`) ao remover operando;
//! - Conversão explícita para malha estática via `bake()` ("sem bake silencioso");
//! - Cancelamento gracioso e reversibilidade.

use petunia_mesh::Mesh;
use petunia_mesh::solid_builder::{ShapeBuilderBudget, SolidOperation};
use petunia_project::{Asset, CompositionError, CompositionStatus, SolidCompositionRecipe};

fn make_cube_asset(name: &str, center: [f32; 3], size: f32) -> Asset {
    let mut mesh = Mesh::cube(size);
    for v in &mut mesh.verts {
        v.pos[0] += center[0];
        v.pos[1] += center[1];
        v.pos[2] += center[2];
    }
    Asset::new(name, mesh)
}

#[test]
fn test_multi_solid_chaining_three_cubes() {
    let base = make_cube_asset("BaseCube", [0.0, 0.0, 0.0], 2.0);
    let op1 = make_cube_asset("OperandX", [1.0, 0.0, 0.0], 2.0);
    let op2 = make_cube_asset("OperandY", [0.0, 1.0, 0.0], 2.0);
    let assets = vec![base.clone(), op1.clone(), op2.clone()];

    let mut recipe = SolidCompositionRecipe::new(base.id);
    recipe.add_step(op1.id, SolidOperation::UniteAll);
    recipe.add_step(op2.id, SolidOperation::UniteAll);

    let result = recipe.evaluate(&assets);
    assert!(result.is_ok(), "Evaluation of 3 chained cubes must succeed");
    let mesh = result.unwrap();
    assert!(!mesh.faces.is_empty(), "Resulting mesh must have faces");
    assert_eq!(recipe.status, CompositionStatus::Valid);
    assert_eq!(recipe.steps.len(), 2);
    assert!(recipe.is_source(base.id));
    assert!(recipe.is_source(op1.id));
    assert!(recipe.is_source(op2.id));
}

#[test]
fn test_budget_enforcement_max_steps() {
    let base = make_cube_asset("Base", [0.0, 0.0, 0.0], 2.0);
    let op1 = make_cube_asset("Op1", [1.0, 0.0, 0.0], 2.0);
    let op2 = make_cube_asset("Op2", [0.0, 1.0, 0.0], 2.0);
    let assets = vec![base.clone(), op1.clone(), op2.clone()];

    let budget = ShapeBuilderBudget {
        max_depth: 1,
        max_input_triangles: 100_000,
        max_decomposed_triangles: 250_000,
        max_components: 128,
    };

    let mut recipe = SolidCompositionRecipe::new(base.id).with_budget(budget);
    recipe.add_step(op1.id, SolidOperation::UniteAll);
    recipe.add_step(op2.id, SolidOperation::UniteAll);

    let result = recipe.evaluate(&assets);
    match result {
        Err(CompositionError::BudgetExceeded(msg)) => {
            assert!(
                msg.contains("depth") || msg.contains("budget"),
                "Message: {msg}"
            );
        }
        other => panic!("Expected BudgetExceeded error, got: {other:?}"),
    }
}

#[test]
fn test_budget_enforcement_max_input_triangles() {
    let base = make_cube_asset("Base", [0.0, 0.0, 0.0], 2.0);
    let op1 = make_cube_asset("Op1", [1.0, 0.0, 0.0], 2.0);
    let assets = vec![base.clone(), op1.clone()];

    // 2 cubos = 24 triângulos no total. Orçamento de 10 deve estourar.
    let budget = ShapeBuilderBudget {
        max_depth: 10,
        max_input_triangles: 10,
        max_decomposed_triangles: 250_000,
        max_components: 128,
    };

    let mut recipe = SolidCompositionRecipe::new(base.id).with_budget(budget);
    recipe.add_step(op1.id, SolidOperation::UniteAll);

    let result = recipe.evaluate(&assets);
    match result {
        Err(CompositionError::BudgetExceeded(msg)) => {
            assert!(
                msg.contains("input") || msg.contains("budget"),
                "Message: {msg}"
            );
        }
        other => panic!("Expected BudgetExceeded error, got: {other:?}"),
    }
}

#[test]
fn test_invalidation_when_source_geometry_modified() {
    let base = make_cube_asset("Base", [0.0, 0.0, 0.0], 2.0);
    let mut op1 = make_cube_asset("Op1", [1.0, 0.0, 0.0], 2.0);
    let mut assets = vec![base.clone(), op1.clone()];

    let mut recipe = SolidCompositionRecipe::new(base.id);
    recipe.add_step(op1.id, SolidOperation::UniteAll);

    let eval_res = recipe.evaluate(&assets);
    assert!(eval_res.is_ok());
    assert_eq!(recipe.status, CompositionStatus::Valid);

    // Nenhuma alteração ainda: check_invalidation retorna false
    assert!(!recipe.check_invalidation(&assets));

    // Modifica a geometria de op1
    if let Some(v) = op1.mesh.verts.get_mut(0) {
        v.pos[0] += 5.0;
    }
    assets[1] = op1.clone();

    // Invalidação detectada
    assert!(recipe.check_invalidation(&assets));
    assert_eq!(recipe.status, CompositionStatus::NeedsEvaluation);

    // Re-avaliação restaura a validade
    let reeval_res = recipe.evaluate(&assets);
    assert!(reeval_res.is_ok());
    assert_eq!(recipe.status, CompositionStatus::Valid);
}

#[test]
fn test_invalidation_when_operand_removed() {
    let base = make_cube_asset("Base", [0.0, 0.0, 0.0], 2.0);
    let op1 = make_cube_asset("Op1", [1.0, 0.0, 0.0], 2.0);
    let assets = vec![base.clone(), op1.clone()];

    let mut recipe = SolidCompositionRecipe::new(base.id);
    recipe.add_step(op1.id, SolidOperation::UniteAll);

    assert!(recipe.evaluate(&assets).is_ok());
    assert_eq!(recipe.status, CompositionStatus::Valid);

    // Remove op1 do projeto
    let assets_without_op = vec![base.clone()];
    assert!(recipe.check_invalidation(&assets_without_op));
    match recipe.status {
        CompositionStatus::Invalidated { ref reason } => {
            assert!(reason.contains("missing"), "Reason: {reason}");
        }
        other => panic!("Expected Invalidated status, got: {other:?}"),
    }

    // Avaliação deve falhar com MissingOperand
    let eval_res = recipe.evaluate(&assets_without_op);
    assert!(matches!(eval_res, Err(CompositionError::MissingOperand(id)) if id == op1.id));
}

#[test]
fn test_explicit_bake_preserves_static_mesh_without_recipe() {
    let mut base = make_cube_asset("Base", [0.0, 0.0, 0.0], 2.0);
    let op1 = make_cube_asset("Op1", [1.0, 0.0, 0.0], 2.0);
    let assets = vec![base.clone(), op1.clone()];

    let mut recipe = SolidCompositionRecipe::new(base.id);
    recipe.add_step(op1.id, SolidOperation::UniteAll);

    let evaluated = recipe
        .evaluate(&assets)
        .expect("Evaluation must succeed")
        .clone();
    base.mesh = evaluated.into();
    base.composition = Some(recipe);

    assert!(base.is_composition());

    // Bake explícito
    assert!(base.bake_composition());
    assert!(!base.is_composition());
    assert!(!base.mesh.faces.is_empty());
}

#[test]
fn test_toggle_and_reorder_steps() {
    let base = make_cube_asset("Base", [0.0, 0.0, 0.0], 2.0);
    let op1 = make_cube_asset("Op1", [1.0, 0.0, 0.0], 2.0);
    let op2 = make_cube_asset("Op2", [0.0, 1.0, 0.0], 2.0);

    let mut recipe = SolidCompositionRecipe::new(base.id);
    let step1_id = recipe.add_step(op1.id, SolidOperation::UniteAll);
    let step2_id = recipe.add_step(op2.id, SolidOperation::RemoveOverlap);

    assert_eq!(recipe.steps[0].id, step1_id);
    assert_eq!(recipe.steps[1].id, step2_id);

    // Reorder
    assert!(recipe.reorder_step(0, 1));
    assert_eq!(recipe.steps[0].id, step2_id);
    assert_eq!(recipe.steps[1].id, step1_id);

    // Toggle
    assert!(recipe.toggle_step(step1_id, false));
    assert!(!recipe.steps[1].enabled);
    assert!(recipe.toggle_step(step1_id, true));
    assert!(recipe.steps[1].enabled);
}
