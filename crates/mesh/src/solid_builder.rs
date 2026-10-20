//! ShapeBuilder 3D — Decomposição em regiões sólidas conexas e plano de ownership transitório (`RegionPlan`).
//!
//! Conforme especificação em `website/docs/03-geometry/shape-builder-3d.md`:
//! - Decomposição em células de partição: A-only, Overlap, B-only;
//! - Separação em componentes conectados por aresta (edge-connected);
//! - Proveniência de UV e material preservados por face;
//! - Preservação estrita de objetos intocados via `RegionPlan`;
//! - Operações `Unite`, `Remove`, `Extract` transitórias;
//! - Sucesso com volume vazio tratado como caso legítimo sem erro de kernel.

use std::collections::BTreeSet;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::Mesh;
use crate::boolean::{BooleanError, BooleanOp, PreparedBooleanOperands, boolean_meshes_clean};
use crate::boolean_cleanup::{SourceIndex, annotate_mesh_provenance};

/// Operação do ShapeBuilder sólido sobre o RegionPlan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolidOperation {
    /// Une todas as regiões ativas em um único sólido (equivalente a A ∪ B).
    UniteAll,
    /// Remove as regiões de overlap de todos os operandos.
    RemoveOverlap,
    /// Extrai o overlap como um novo objeto na cena, retirando-o dos operandos de origem.
    ExtractOverlap,
}

/// Orçamento e limites de complexidade para o ShapeBuilder e composição sólida.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShapeBuilderBudget {
    /// Limite de triângulos nas malhas de entrada somadas.
    pub max_input_triangles: usize,
    /// Limite de triângulos nas malhas decompostas/resultantes.
    pub max_decomposed_triangles: usize,
    /// Limite de componentes conexos por aresta (edge-connected).
    pub max_components: usize,
    /// Profundidade máxima de operações encadeadas em uma composição.
    pub max_depth: usize,
}

impl Default for ShapeBuilderBudget {
    fn default() -> Self {
        Self {
            max_input_triangles: 250_000,
            max_decomposed_triangles: 500_000,
            max_components: 128,
            max_depth: 32,
        }
    }
}

impl ShapeBuilderBudget {
    pub fn check_input(&self, triangle_count: usize) -> Result<(), BooleanError> {
        if triangle_count > self.max_input_triangles {
            return Err(BooleanError::InvalidInput(
                "input triangle count exceeds ShapeBuilderBudget",
            ));
        }
        Ok(())
    }

    pub fn check_decomposed(
        &self,
        triangle_count: usize,
        component_count: usize,
    ) -> Result<(), BooleanError> {
        if triangle_count > self.max_decomposed_triangles {
            return Err(BooleanError::InvalidOutput(
                "decomposed triangle count exceeds ShapeBuilderBudget",
            ));
        }
        if component_count > self.max_components {
            return Err(BooleanError::InvalidOutput(
                "connected component count exceeds ShapeBuilderBudget",
            ));
        }
        Ok(())
    }

    pub fn check_depth(&self, depth: usize) -> Result<(), BooleanError> {
        if depth > self.max_depth {
            return Err(BooleanError::InvalidInput(
                "operation depth exceeds ShapeBuilderBudget",
            ));
        }
        Ok(())
    }
}

/// Proveniência da região sólida particionada.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolidRegionProvenance {
    /// Pertence exclusivamente ao primeiro operando (A - B).
    FirstOnly,
    /// Pertence ao volume de interseção compartilhada (A ∩ B).
    Overlap,
    /// Pertence exclusivamente ao segundo operando (B - A).
    SecondOnly,
}

/// Componente conectado por aresta de um sólido resultante da partição.
#[derive(Clone, Debug)]
pub struct SolidRegion {
    pub id: usize,
    pub provenance: SolidRegionProvenance,
    pub mesh: Mesh,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
}

impl SolidRegion {
    pub fn new(id: usize, provenance: SolidRegionProvenance, mesh: Mesh) -> Self {
        let mut min = Vec3::splat(f32::INFINITY);
        let mut max = Vec3::splat(f32::NEG_INFINITY);
        for v in &mesh.verts {
            let p = v.vec();
            min = min.min(p);
            max = max.max(p);
        }
        let bounds_min = if min.is_finite() {
            min.to_array()
        } else {
            [0.0; 3]
        };
        let bounds_max = if max.is_finite() {
            max.to_array()
        } else {
            [0.0; 3]
        };
        Self {
            id,
            provenance,
            mesh,
            bounds_min,
            bounds_max,
        }
    }
}

/// Origem do grupo de regiões no plano de ownership.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionGroupOrigin {
    FirstOperand,
    SecondOperand,
    Created,
}

/// Grupo de regiões atribuídas a um objeto de saída.
#[derive(Clone, Debug)]
pub struct RegionGroup {
    pub id: usize,
    pub origin: RegionGroupOrigin,
    pub regions: BTreeSet<usize>,
}

/// Resultado de commit para um grupo do RegionPlan.
#[derive(Clone, Debug)]
pub enum CommittedObject {
    /// O primeiro operando original permaneceu intocado: preservar SceneObject, id e transform.
    PreserveFirst,
    /// O segundo operando original permaneceu intocado: preservar SceneObject, id e transform.
    PreserveSecond,
    /// O grupo resultou em volume vazio (removido ou diferença total).
    Empty,
    /// Nova malha materializada para o grupo.
    Mesh {
        mesh: Box<Mesh>,
        origin: RegionGroupOrigin,
    },
}

/// Plano de ownership transitório para ShapeBuilder sólido de dois operandos.
#[derive(Clone, Debug)]
pub struct RegionPlan {
    pub regions: Vec<SolidRegion>,
    pub first_initial: BTreeSet<usize>,
    pub second_initial: BTreeSet<usize>,
    pub groups: Vec<RegionGroup>,
    next_group_id: usize,
}

impl RegionPlan {
    /// Constrói a partição de sólidos e inicializa o RegionPlan a partir dos operandos A e B.
    pub fn build(a: &Mesh, b: &Mesh) -> Result<Self, BooleanError> {
        let operands = PreparedBooleanOperands::new(a, b)?;
        let partition = operands.partition()?;
        let source_index = SourceIndex::from_operands(a, b);

        let mut regions = Vec::new();
        let mut first_initial = BTreeSet::new();
        let mut second_initial = BTreeSet::new();

        // 1. A-only (FirstOnly)
        for mut comp in partition.first_only.connected_regions()? {
            annotate_mesh_provenance(&mut comp, a, b, &source_index);
            let id = regions.len();
            regions.push(SolidRegion::new(id, SolidRegionProvenance::FirstOnly, comp));
            first_initial.insert(id);
        }

        // 2. Overlap (Interseção)
        for mut comp in partition.overlap.connected_regions()? {
            annotate_mesh_provenance(&mut comp, a, b, &source_index);
            let id = regions.len();
            regions.push(SolidRegion::new(id, SolidRegionProvenance::Overlap, comp));
            first_initial.insert(id);
            second_initial.insert(id);
        }

        // 3. B-only (SecondOnly)
        for mut comp in partition.second_only.connected_regions()? {
            annotate_mesh_provenance(&mut comp, a, b, &source_index);
            let id = regions.len();
            regions.push(SolidRegion::new(
                id,
                SolidRegionProvenance::SecondOnly,
                comp,
            ));
            second_initial.insert(id);
        }

        let groups = vec![
            RegionGroup {
                id: 0,
                origin: RegionGroupOrigin::FirstOperand,
                regions: first_initial.clone(),
            },
            RegionGroup {
                id: 1,
                origin: RegionGroupOrigin::SecondOperand,
                regions: second_initial.clone(),
            },
        ];

        Ok(Self {
            regions,
            first_initial,
            second_initial,
            groups,
            next_group_id: 2,
        })
    }

    /// Constrói a partição de sólidos aplicando um orçamento de complexidade (`ShapeBuilderBudget`).
    pub fn build_with_budget(
        a: &Mesh,
        b: &Mesh,
        budget: &ShapeBuilderBudget,
    ) -> Result<Self, BooleanError> {
        budget.check_input(a.faces.len().saturating_add(b.faces.len()))?;
        let plan = Self::build(a, b)?;
        let total_triangles: usize = plan.regions.iter().map(|r| r.mesh.faces.len()).sum();
        budget.check_decomposed(total_triangles, plan.regions.len())?;
        Ok(plan)
    }

    /// Remove as regiões selecionadas de todos os grupos existentes.
    pub fn remove(&mut self, region_ids: &[usize]) {
        for group in &mut self.groups {
            for &id in region_ids {
                group.regions.remove(&id);
            }
        }
    }

    /// Extrai as regiões selecionadas dos grupos atuais e cria um novo grupo de saída contendo-as.
    pub fn extract(&mut self, region_ids: &[usize]) -> Option<usize> {
        let mut extracted = BTreeSet::new();
        for &id in region_ids {
            if id < self.regions.len() {
                extracted.insert(id);
            }
        }
        if extracted.is_empty() {
            return None;
        }
        self.remove(region_ids);
        let group_id = self.next_group_id;
        self.next_group_id = self.next_group_id.saturating_add(1);
        self.groups.push(RegionGroup {
            id: group_id,
            origin: RegionGroupOrigin::Created,
            regions: extracted,
        });
        Some(group_id)
    }

    /// Une as regiões selecionadas em um único grupo de saída contendo-as.
    pub fn unite(&mut self, region_ids: &[usize]) -> Option<usize> {
        let mut united = BTreeSet::new();
        for &id in region_ids {
            if id < self.regions.len() {
                united.insert(id);
            }
        }
        if united.is_empty() {
            return None;
        }
        self.remove(region_ids);
        let group_id = self.next_group_id;
        self.next_group_id = self.next_group_id.saturating_add(1);
        self.groups.push(RegionGroup {
            id: group_id,
            origin: RegionGroupOrigin::Created,
            regions: united,
        });
        Some(group_id)
    }

    /// Retorna `true` se o primeiro operando mantém rigorosamente seu volume inicial intacto.
    pub fn is_first_untouched(&self) -> bool {
        self.groups
            .iter()
            .any(|g| g.origin == RegionGroupOrigin::FirstOperand && g.regions == self.first_initial)
    }

    /// Retorna `true` se o segundo operando mantém rigorosamente seu volume inicial intacto.
    pub fn is_second_untouched(&self) -> bool {
        self.groups.iter().any(|g| {
            g.origin == RegionGroupOrigin::SecondOperand && g.regions == self.second_initial
        })
    }

    /// Consolida o plano de regiões em objetos de saída concretos (`CommittedObject`).
    pub fn commit(&self, a: &Mesh, b: &Mesh) -> Result<Vec<CommittedObject>, BooleanError> {
        let mut results = Vec::new();

        for group in &self.groups {
            if group.regions.is_empty() {
                results.push(CommittedObject::Empty);
                continue;
            }

            if group.origin == RegionGroupOrigin::FirstOperand
                && group.regions == self.first_initial
            {
                results.push(CommittedObject::PreserveFirst);
                continue;
            }

            if group.origin == RegionGroupOrigin::SecondOperand
                && group.regions == self.second_initial
            {
                results.push(CommittedObject::PreserveSecond);
                continue;
            }

            // Fast paths baseados nas regiões do grupo:
            // 1. Todas as regiões de A e B -> equivalente exato de A ∪ B
            let total_regions: BTreeSet<usize> = (0..self.regions.len()).collect();
            if group.regions == total_regions && !total_regions.is_empty() {
                let (union_mesh, _) = boolean_meshes_clean(a, b, BooleanOp::Union)?;
                results.push(CommittedObject::Mesh {
                    mesh: Box::new(union_mesh),
                    origin: group.origin,
                });
                continue;
            }

            // 2. Apenas 1 região no grupo -> sem novo cálculo booleano
            if group.regions.len() == 1 {
                let reg_id = *group.regions.iter().next().unwrap_or(&0);
                if let Some(region) = self.regions.get(reg_id) {
                    results.push(CommittedObject::Mesh {
                        mesh: Box::new(region.mesh.clone()),
                        origin: group.origin,
                    });
                }
                continue;
            }

            // 3. Combinação arbitrária de múltiplas regiões no mesmo grupo
            let mut region_meshes: Vec<Mesh> = group
                .regions
                .iter()
                .filter_map(|&id| self.regions.get(id).map(|r| r.mesh.clone()))
                .collect();

            if region_meshes.is_empty() {
                results.push(CommittedObject::Empty);
                continue;
            }

            let mut combined = region_meshes.remove(0);
            for next_mesh in region_meshes {
                let (merged, _) = boolean_meshes_clean(&combined, &next_mesh, BooleanOp::Union)?;
                combined = merged;
            }

            results.push(CommittedObject::Mesh {
                mesh: Box::new(combined),
                origin: group.origin,
            });
        }

        Ok(results)
    }
}

/// Avalia uma sequência ordenada de múltiplos operandos sólidos sob um orçamento de segurança.
pub fn evaluate_multi_solid(
    base: &Mesh,
    steps: &[(&Mesh, SolidOperation)],
    budget: &ShapeBuilderBudget,
) -> Result<Mesh, BooleanError> {
    budget.check_depth(steps.len())?;
    let mut current = base.clone();
    current.triangulate();

    for (depth, &(operand, op)) in steps.iter().enumerate() {
        budget.check_depth(depth.saturating_add(1))?;
        budget.check_input(current.faces.len().saturating_add(operand.faces.len()))?;

        let mut operand_tri = operand.clone();
        operand_tri.triangulate();

        let plan = RegionPlan::build_with_budget(&current, &operand_tri, budget)?;
        match op {
            SolidOperation::UniteAll => {
                let (union_mesh, _) =
                    boolean_meshes_clean(&current, &operand_tri, BooleanOp::Union)?;
                current = union_mesh;
                current.triangulate();
            }
            SolidOperation::RemoveOverlap => {
                let overlap_ids: Vec<usize> = plan
                    .regions
                    .iter()
                    .filter(|r| r.provenance == SolidRegionProvenance::Overlap)
                    .map(|r| r.id)
                    .collect();
                let mut p = plan;
                p.remove(&overlap_ids);
                let results = p.commit(&current, &operand_tri)?;
                if let Some(first_res) = results.into_iter().next() {
                    match first_res {
                        CommittedObject::PreserveFirst => {}
                        CommittedObject::Mesh { mesh, .. } => {
                            current = *mesh;
                            current.triangulate();
                        }
                        CommittedObject::Empty => {
                            current = Mesh::default();
                        }
                        CommittedObject::PreserveSecond => {}
                    }
                }
            }
            SolidOperation::ExtractOverlap => {
                let overlap_ids: Vec<usize> = plan
                    .regions
                    .iter()
                    .filter(|r| r.provenance == SolidRegionProvenance::Overlap)
                    .map(|r| r.id)
                    .collect();
                let mut p = plan;
                if let Some(group_id) = p.extract(&overlap_ids) {
                    let results = p.commit(&current, &operand_tri)?;
                    if let Some(res) = results.get(group_id) {
                        match res {
                            CommittedObject::Mesh { mesh, .. } => {
                                current = *(mesh.clone());
                                current.triangulate();
                            }
                            _ => {
                                current = Mesh::default();
                            }
                        }
                    }
                }
            }
        }
        budget.check_decomposed(current.faces.len(), 1)?;
    }

    Ok(current)
}
