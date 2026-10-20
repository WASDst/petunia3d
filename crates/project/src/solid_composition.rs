//! Receita de Composição Sólida Reeditável (H26).
//!
//! Conforme especificação em `website/docs/02-application/contextual-editor-contract.md`
//! e `website/docs/03-geometry/shape-builder-3d.md`:
//! - Fontes com identidade estável (referenciadas por Uuid/AssetId);
//! - Receita persistente de operações (UniteAll, RemoveOverlap, ExtractOverlap);
//! - Malha avaliada derivada, recalculada sob demanda;
//! - Sem bake silencioso: conversão para malha poligonal editável exige comando explícito;
//! - Invalidação explícita de referências e status para revisão se fontes mudarem;
//! - Orçamento estrito (`ShapeBuilderBudget`) para proteção de hardware low-end.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use petunia_mesh::Mesh;
use petunia_mesh::solid_builder::{ShapeBuilderBudget, SolidOperation, evaluate_multi_solid};

use crate::Asset;

/// Versão do schema de composição sólida.
pub const SOLID_COMPOSITION_SCHEMA_VERSION: u32 = 1;

/// Erros na validação e avaliação da composição sólida.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompositionError {
    #[error("Base asset with id {0} not found in project")]
    MissingBase(Uuid),
    #[error("Operand asset with id {0} not found in project")]
    MissingOperand(Uuid),
    #[error("Solid composition budget exceeded: {0}")]
    BudgetExceeded(String),
    #[error("Boolean kernel error during composition: {0}")]
    KernelError(String),
    #[error("Composition produced an empty solid")]
    EmptyResult,
    #[error("Composition recipe is invalidated: {0}")]
    Invalidated(String),
}

pub type SolidCompositionError = CompositionError;

/// Status de integridade e revisão da receita.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompositionStatus {
    /// A receita foi avaliada com sucesso e suas fontes estão síncronas.
    Valid,
    /// Alguma fonte sofreu alteração e a receita requer reavaliação.
    NeedsEvaluation,
    /// A receita foi invalidada (ex: fonte removida ou topologia incompatível) e requer revisão do usuário.
    Invalidated { reason: String },
}

pub type RecipeStatus = CompositionStatus;

/// Registro de uma fonte estável que participa da composição.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolidSource {
    pub id: Uuid,
    pub name: String,
    pub topology_hash: u64,
}

impl SolidSource {
    pub fn new(id: Uuid, name: &str, topology_hash: u64) -> Self {
        Self {
            id,
            name: name.to_string(),
            topology_hash,
        }
    }
}

/// Passo ordenado de composição sólida.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SolidCompositionStep {
    pub id: Uuid,
    /// Operando secundário associado a esta etapa.
    pub operand_id: Uuid,
    /// Operação a aplicar sobre o acumulado até este ponto.
    pub operation: SolidOperation,
    /// Se esta etapa está ativa na avaliação.
    pub enabled: bool,
}

impl SolidCompositionStep {
    pub fn new(operand_id: Uuid, operation: SolidOperation) -> Self {
        Self {
            id: Uuid::new_v4(),
            operand_id,
            operation,
            enabled: true,
        }
    }
}

/// Receita persistente de composição sólida associada a um SceneObject / Asset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SolidCompositionRecipe {
    pub id: Uuid,
    /// Identidade estável do asset base.
    pub base_id: Uuid,
    /// Fontes estáveis participantes com hashes topológicos.
    pub sources: Vec<SolidSource>,
    /// Sequência ordenada de passos de composição.
    pub steps: Vec<SolidCompositionStep>,
    /// Malha avaliada em cache.
    #[serde(default)]
    pub cached_mesh: Mesh,
    /// Orçamento de complexidade para segurança em hardware modesto.
    pub budget: ShapeBuilderBudget,
    /// Status de integridade da receita.
    pub status: CompositionStatus,
}

impl SolidCompositionRecipe {
    pub fn new(base_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            base_id,
            sources: Vec::new(),
            steps: Vec::new(),
            cached_mesh: Mesh::default(),
            budget: ShapeBuilderBudget::default(),
            status: CompositionStatus::NeedsEvaluation,
        }
    }

    pub fn with_budget(mut self, budget: ShapeBuilderBudget) -> Self {
        self.budget = budget;
        self
    }

    pub fn add_step(&mut self, operand_id: Uuid, operation: SolidOperation) -> Uuid {
        let step = SolidCompositionStep::new(operand_id, operation);
        let id = step.id;
        self.steps.push(step);
        self.status = CompositionStatus::NeedsEvaluation;
        id
    }

    pub fn remove_step(&mut self, step_id: Uuid) -> bool {
        let prev_len = self.steps.len();
        self.steps.retain(|s| s.id != step_id);
        if self.steps.len() != prev_len {
            self.status = CompositionStatus::NeedsEvaluation;
            true
        } else {
            false
        }
    }

    pub fn toggle_step(&mut self, step_id: Uuid, enabled: bool) -> bool {
        if let Some(step) = self.steps.iter_mut().find(|s| s.id == step_id)
            && step.enabled != enabled
        {
            step.enabled = enabled;
            self.status = CompositionStatus::NeedsEvaluation;
            return true;
        }
        false
    }

    pub fn reorder_step(&mut self, from: usize, to: usize) -> bool {
        if from < self.steps.len() && to < self.steps.len() && from != to {
            let step = self.steps.remove(from);
            self.steps.insert(to, step);
            self.status = CompositionStatus::NeedsEvaluation;
            true
        } else {
            false
        }
    }

    /// Retorna todos os IDs estáveis de fontes desta receita (base + operandos).
    pub fn source_ids(&self) -> Vec<Uuid> {
        let mut ids = vec![self.base_id];
        for s in &self.steps {
            if !ids.contains(&s.operand_id) {
                ids.push(s.operand_id);
            }
        }
        ids
    }

    /// Retorna `true` se o asset_id participa desta receita.
    pub fn is_source(&self, asset_id: Uuid) -> bool {
        self.base_id == asset_id || self.steps.iter().any(|s| s.operand_id == asset_id)
    }

    /// Retorna a malha avaliada da composição para congelamento explícito (bake).
    pub fn bake(self) -> Mesh {
        self.cached_mesh
    }

    /// Retorna referência à malha avaliada em cache.
    pub fn evaluated_mesh(&self) -> &Mesh {
        &self.cached_mesh
    }

    /// Registra ou atualiza o hash de uma fonte.
    pub fn record_source(&mut self, id: Uuid, name: &str, hash: u64) {
        if let Some(s) = self.sources.iter_mut().find(|s| s.id == id) {
            s.name = name.to_string();
            s.topology_hash = hash;
        } else {
            self.sources.push(SolidSource::new(id, name, hash));
        }
    }

    /// Inspeciona se as fontes atuais no documento ainda são válidas e síncronas.
    pub fn check_invalidation(&mut self, current_assets: &[Asset]) -> bool {
        // 1. Verifica se a base ainda existe
        let base_opt = current_assets.iter().find(|a| a.id == self.base_id);
        let Some(base) = base_opt else {
            self.status = CompositionStatus::Invalidated {
                reason: format!("Base asset {} missing from project", self.base_id),
            };
            return true;
        };

        // 2. Verifica se todos os operandos ainda existem
        for step in &self.steps {
            if !current_assets.iter().any(|a| a.id == step.operand_id) {
                self.status = CompositionStatus::Invalidated {
                    reason: format!("Operand asset {} missing from project", step.operand_id),
                };
                return true;
            }
        }

        // 3. Verifica integridade de hash de topologia
        let mut changed = false;
        let base_hash = base.source_mesh_hash();
        let last_base_hash = self
            .sources
            .iter()
            .find(|s| s.id == self.base_id)
            .map(|s| s.topology_hash);
        if last_base_hash != Some(base_hash) {
            changed = true;
        }

        for step in &self.steps {
            if let Some(op_asset) = current_assets.iter().find(|a| a.id == step.operand_id) {
                let op_hash = op_asset.source_mesh_hash();
                let last_op_hash = self
                    .sources
                    .iter()
                    .find(|s| s.id == step.operand_id)
                    .map(|s| s.topology_hash);
                if last_op_hash != Some(op_hash) {
                    changed = true;
                }
            }
        }

        if changed && self.status == CompositionStatus::Valid {
            self.status = CompositionStatus::NeedsEvaluation;
            return true;
        }

        false
    }

    /// Avalia deterministicamente a receita de composição sólida a partir das fontes do projeto.
    pub fn evaluate(&mut self, assets: &[Asset]) -> Result<&Mesh, CompositionError> {
        let base = assets
            .iter()
            .find(|a| a.id == self.base_id)
            .ok_or(CompositionError::MissingBase(self.base_id))?;

        let mut transformed_operands = Vec::new();
        for step in &self.steps {
            if !step.enabled {
                continue;
            }
            let operand = assets
                .iter()
                .find(|a| a.id == step.operand_id)
                .ok_or(CompositionError::MissingOperand(step.operand_id))?;

            let in_base_frame = operand.mesh_in_frame_of(base);
            transformed_operands.push((in_base_frame, step.operation));
        }

        if transformed_operands.is_empty() {
            self.cached_mesh = base.mesh.to_owned_value();
            self.record_source(base.id, &base.name, base.source_mesh_hash());
            self.status = CompositionStatus::Valid;
            return Ok(&self.cached_mesh);
        }

        let steps_refs: Vec<(&Mesh, SolidOperation)> = transformed_operands
            .iter()
            .map(|(m, op)| (m, *op))
            .collect();

        let evaluated =
            evaluate_multi_solid(&base.mesh, &steps_refs, &self.budget).map_err(|e| match e {
                petunia_mesh::boolean::BooleanError::InvalidInput(s) => {
                    CompositionError::BudgetExceeded(s.to_string())
                }
                petunia_mesh::boolean::BooleanError::InvalidOutput(s) => {
                    CompositionError::BudgetExceeded(s.to_string())
                }
                petunia_mesh::boolean::BooleanError::Kernel => CompositionError::KernelError(
                    "kernel error during solid evaluation".to_string(),
                ),
            })?;

        if evaluated.faces.is_empty() {
            return Err(CompositionError::EmptyResult);
        }

        self.cached_mesh = evaluated;

        // Atualiza fontes gravadas
        self.record_source(base.id, &base.name, base.source_mesh_hash());
        let op_info: Vec<(Uuid, String, u64)> = self
            .steps
            .iter()
            .filter_map(|step| {
                assets
                    .iter()
                    .find(|a| a.id == step.operand_id)
                    .map(|op_asset| {
                        (
                            op_asset.id,
                            op_asset.name.clone(),
                            op_asset.source_mesh_hash(),
                        )
                    })
            })
            .collect();
        for (id, name, hash) in op_info {
            self.record_source(id, &name, hash);
        }

        self.status = CompositionStatus::Valid;
        Ok(&self.cached_mesh)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_cube(offset: [f32; 3]) -> Mesh {
        let mut m = Mesh::cube(1.0);
        m.triangulate();
        for v in &mut m.verts {
            v.pos[0] += offset[0];
            v.pos[1] += offset[1];
            v.pos[2] += offset[2];
        }
        m
    }

    #[test]
    fn solid_composition_evaluate_invalidation_and_bake() {
        let mut base_asset = Asset::new("Base", test_cube([0.0; 3]));
        let op1_asset = Asset::new("Op1", test_cube([0.5, 0.0, 0.0]));
        let op2_asset = Asset::new("Op2", test_cube([1.0, 0.0, 0.0]));

        let mut recipe = SolidCompositionRecipe::new(base_asset.id);
        recipe.add_step(op1_asset.id, SolidOperation::UniteAll);
        recipe.add_step(op2_asset.id, SolidOperation::UniteAll);

        assert_eq!(recipe.status, CompositionStatus::NeedsEvaluation);

        let assets = vec![base_asset.clone(), op1_asset.clone(), op2_asset.clone()];

        // 1. Evaluate
        let eval_mesh = recipe.evaluate(&assets).expect("Evaluation should succeed");
        assert!(!eval_mesh.faces.is_empty());
        assert_eq!(recipe.status, CompositionStatus::Valid);

        // 2. Invalidation check without changes
        assert!(!recipe.check_invalidation(&assets));
        assert_eq!(recipe.status, CompositionStatus::Valid);

        // 3. Invalidation check when an operand mesh changes topology
        let mut modified_assets = assets.clone();
        modified_assets[1].mesh = Mesh::cube(2.0).into(); // Op1 changed
        assert!(recipe.check_invalidation(&modified_assets));
        assert_eq!(recipe.status, CompositionStatus::NeedsEvaluation);

        // 4. Invalidation check when an operand is deleted
        let incomplete_assets = vec![assets[0].clone()]; // Op1 and Op2 missing
        assert!(recipe.check_invalidation(&incomplete_assets));
        assert!(matches!(
            recipe.status,
            CompositionStatus::Invalidated { .. }
        ));

        // 5. Bake composition onto base_asset
        base_asset.composition = Some(recipe);
        assert!(base_asset.is_composition());
        let baked = base_asset.bake_composition();
        assert!(baked);
        assert!(!base_asset.is_composition());
        assert!(!base_asset.mesh.faces.is_empty());
    }

    #[test]
    fn serialization_roundtrip_preserves_recipe() {
        let base_id = Uuid::new_v4();
        let op_id = Uuid::new_v4();
        let mut recipe = SolidCompositionRecipe::new(base_id);
        recipe.add_step(op_id, SolidOperation::RemoveOverlap);

        let json = serde_json::to_string(&recipe).expect("Serialization must succeed");
        let restored: SolidCompositionRecipe =
            serde_json::from_str(&json).expect("Deserialization must succeed");

        assert_eq!(restored.id, recipe.id);
        assert_eq!(restored.base_id, base_id);
        assert_eq!(restored.steps.len(), 1);
        assert_eq!(restored.steps[0].operand_id, op_id);
        assert_eq!(restored.steps[0].operation, SolidOperation::RemoveOverlap);
    }
}
