//! Two-operand solid-region sessions. Geometry preparation and commit are
//! separated from pointer picking and region selection.
use crate::{Command, CommandContext, CommandError, EditorView};
use glam::Vec3;
use petunia_mesh::{
    Mesh,
    boolean::{BooleanOp, BooleanSolid, SolidRegionClass},
};
use petunia_project::{Asset, Project, ProjectChanges};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolidShapeIntent {
    Unite,
    Remove,
    Extract,
}

#[derive(Debug, Clone)]
pub struct SolidRegionPreview {
    pub id: Uuid,
    pub class: SolidRegionClass,
    pub mesh: Mesh,
    bvh: crate::bvh::TriangleBvh,
}

#[derive(Debug, Clone)]
pub struct SolidRegionGroup {
    pub owner: Option<Uuid>,
    pub regions: BTreeSet<usize>,
    pub changed: bool,
}

#[derive(Debug, Clone)]
pub struct SolidShapeBuilderSession {
    pub first: Asset,
    pub second: Asset,
    pub regions: Vec<SolidRegionPreview>,
    pub groups: Vec<SolidRegionGroup>,
    pub selected: BTreeSet<usize>,
    pub intent: SolidShapeIntent,
    revision: [u64; 11],
    surface_materials: Vec<petunia_project::SurfaceMaterial>,
}

#[derive(Debug, Clone)]
pub struct CommitSolidShapeBuilderCmd {
    pub session: SolidShapeBuilderSession,
}

impl SolidShapeBuilderSession {
    pub fn begin(project: &Project, first: Uuid, second: Uuid) -> Result<Self, CommandError> {
        if first == second {
            return Err(CommandError::Execution(
                "Choose two distinct solid objects".into(),
            ));
        }
        let a = project
            .assets
            .iter()
            .find(|a| a.id == first)
            .ok_or(CommandError::NoActiveAsset)?;
        let b = project
            .assets
            .iter()
            .find(|a| a.id == second)
            .ok_or(CommandError::NoActiveAsset)?;
        for asset in [a, b] {
            if asset.locked
                || !asset.visible
                || asset.skeleton_id.is_some()
                || asset.skin_data.is_some()
            {
                return Err(CommandError::Execution(
                    "Operands must be visible unlocked static solids".into(),
                ));
            }
            let matrix = asset.world_matrix();
            if !matrix.is_finite() || matrix.determinant().abs() < 1.0e-12 {
                return Err(CommandError::Execution(
                    "Operand transform is singular".into(),
                ));
            }
        }
        let mut surface_materials = Vec::new();
        let mut first_mesh = a.evaluated_mesh();
        bind_surfaces(project, a, &mut first_mesh, &mut surface_materials)?;
        let mut second_mesh = b.evaluated_mesh();
        bind_surfaces(project, b, &mut second_mesh, &mut surface_materials)?;
        second_mesh.transform_affine(a.world_matrix().inverse() * b.world_matrix());
        let partition = petunia_mesh::boolean::partition_solids(&first_mesh, &second_mesh)
            .map_err(|e| CommandError::Execution(e.to_string()))?;
        let regions: Vec<_> = partition
            .regions
            .into_iter()
            .map(|region| {
                let mut triangles = Vec::new();
                for face in 0..region.mesh.faces.len() {
                    let Some(source) = region.mesh.faces.get(face) else {
                        continue;
                    };
                    for corners in region.mesh.face_triangle_corners(face) {
                        let points = corners.map(|corner| {
                            source
                                .verts
                                .get(corner)
                                .and_then(|v| region.mesh.verts.get(*v as usize))
                                .map(|v| v.vec())
                        });
                        if let [Some(a), Some(b), Some(c)] = points {
                            triangles.push(([a, b, c], face as u32));
                        }
                    }
                }
                SolidRegionPreview {
                    id: Uuid::new_v4(),
                    class: region.class,
                    mesh: region.mesh,
                    bvh: crate::bvh::TriangleBvh::build(triangles),
                }
            })
            .collect();
        let first_regions = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.class != SolidRegionClass::SecondOnly)
            .map(|(i, _)| i)
            .collect();
        let second_regions = regions
            .iter()
            .enumerate()
            .filter(|(_, r)| r.class != SolidRegionClass::FirstOnly)
            .map(|(i, _)| i)
            .collect();
        Ok(Self {
            first: a.clone(),
            second: b.clone(),
            regions,
            groups: vec![
                SolidRegionGroup {
                    owner: Some(first),
                    regions: first_regions,
                    changed: false,
                },
                SolidRegionGroup {
                    owner: Some(second),
                    regions: second_regions,
                    changed: false,
                },
            ],
            selected: BTreeSet::new(),
            intent: SolidShapeIntent::Unite,
            revision: project.revision_clock(),
            surface_materials,
        })
    }

    pub fn validate(&self, project: &Project) -> Result<(), CommandError> {
        let current = project.revision_clock();
        if current
            .iter()
            .zip(self.revision)
            .enumerate()
            .any(|(i, (a, b))| i != 2 && *a != b)
        {
            return Err(CommandError::StaleReference(
                "Solid operands changed; restart Shape Builder".into(),
            ));
        }
        for id in [self.first.id, self.second.id] {
            if !project
                .assets
                .iter()
                .any(|a| a.id == id && a.visible && !a.locked)
            {
                return Err(CommandError::StaleReference(
                    "Solid operand is unavailable".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn hits(&self, origin: Vec3, direction: Vec3) -> Vec<(f32, usize)> {
        let inverse = self.first.world_matrix().inverse();
        let origin = inverse.transform_point3(origin);
        let direction = inverse.transform_vector3(direction);
        let mut hits: Vec<_> = self
            .regions
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                r.bvh
                    .nearest(origin, direction, f32::INFINITY, |_| true)
                    .map(|(depth, _)| (depth, i))
            })
            .collect();
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        hits
    }

    pub fn apply_selected(&mut self) -> Result<(), CommandError> {
        if self.selected.is_empty() {
            return Err(CommandError::EmptySelection);
        }
        if self.selected.iter().any(|i| self.regions.get(*i).is_none()) {
            return Err(CommandError::StaleReference("Unknown solid region".into()));
        }
        for group in &mut self.groups {
            let before = group.regions.len();
            group.regions.retain(|i| !self.selected.contains(i));
            group.changed |= before != group.regions.len();
        }
        match self.intent {
            SolidShapeIntent::Remove => {}
            SolidShapeIntent::Unite => self.groups.push(SolidRegionGroup {
                owner: None,
                regions: self.selected.clone(),
                changed: true,
            }),
            SolidShapeIntent::Extract => {
                for region in &self.selected {
                    self.groups.push(SolidRegionGroup {
                        owner: None,
                        regions: BTreeSet::from([*region]),
                        changed: true,
                    });
                }
            }
        }
        self.selected.clear();
        Ok(())
    }

    fn group_mesh(&self, group: &SolidRegionGroup) -> Result<Mesh, CommandError> {
        let mut ids = group.regions.iter();
        let first = ids
            .next()
            .and_then(|i| self.regions.get(*i))
            .ok_or(CommandError::EmptySelection)?;
        let mut mesh = first.mesh.clone();
        for id in ids {
            let next = self.regions.get(*id).ok_or(CommandError::EmptySelection)?;
            // Final union removes internal surfaces exactly once at commit.
            mesh = match petunia_mesh::boolean::boolean_solid(&mesh, &next.mesh, BooleanOp::Union)
                .map_err(|e| CommandError::Execution(e.to_string()))?
            {
                BooleanSolid::Mesh(mesh) => *mesh,
                BooleanSolid::Empty => {
                    return Err(CommandError::Execution(
                        "Union unexpectedly produced no volume".into(),
                    ));
                }
            };
        }
        Ok(mesh)
    }
}

impl Command for CommitSolidShapeBuilderCmd {
    fn label(&self) -> &'static str {
        "solid shape builder"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::GEOMETRY
            | ProjectChanges::PROCEDURAL
            | ProjectChanges::SELECTION
            | ProjectChanges::MATERIALS
    }
    fn can_execute(&self, state: &EditorView<'_>) -> Result<(), &'static str> {
        self.session
            .validate(&state.document.project)
            .map_err(|_| "Solid session is obsolete")?;
        if !self.session.selected.is_empty() || self.session.groups.iter().any(|g| g.changed) {
            Ok(())
        } else {
            Err("Choose one or more solid regions")
        }
    }
    fn execute(&self, state: &mut CommandContext<'_>) -> Result<(), CommandError> {
        let mut session = self.session.clone();
        if !session.selected.is_empty() {
            session.apply_selected()?;
        }
        let mut outputs = Vec::new();
        let mut removed = BTreeSet::new();
        for group in &session.groups {
            if !group.changed {
                continue;
            }
            if let Some(owner) = group.owner {
                removed.insert(owner);
            }
            if group.regions.is_empty() {
                continue;
            }
            let mut mesh = session.group_mesh(group)?;
            let mut asset = match group.owner {
                Some(id) if id == session.first.id => session.first.clone(),
                Some(_) => session.second.clone(),
                None => {
                    let mut a = Asset::new(
                        match session.intent {
                            SolidShapeIntent::Extract => "Extracted region",
                            _ => "United regions",
                        },
                        Mesh::default(),
                    );
                    a.transform = session.first.transform;
                    a.material_id = session.first.material_id;
                    a
                }
            };
            mesh.transform_affine(asset.world_matrix().inverse() * session.first.world_matrix());
            mesh.deselect_all();
            asset.mesh = mesh.into();
            asset.surface_materials = session.surface_materials.clone();
            asset.texture = None;
            asset.paint_stack = None;
            asset.geometry_source = None;
            asset.generated_mesh = Mesh::default().into();
            asset.parametric = None;
            asset.modifiers.clear();
            asset.generator_diagnostic = None;
            outputs.push(asset);
        }
        let project = &mut state.document.project;
        project.assets.retain(|a| !removed.contains(&a.id));
        let selected: Vec<_> = outputs.iter().map(|a| a.id).collect();
        project.assets.extend(outputs);
        let active = selected
            .first()
            .copied()
            .or_else(|| project.assets.first().map(|a| a.id));
        project.active = active
            .and_then(|id| project.assets.iter().position(|a| a.id == id))
            .unwrap_or(usize::MAX);
        state.editor.selection.assets = if selected.is_empty() {
            active.into_iter().collect()
        } else {
            selected
        };
        state.editor.selection.asset = active;
        state.editor.selection.clear();
        project.history_selection = state.editor.selection.assets.clone();
        Ok(())
    }
}

/// Remap both operands into a deterministic local table before partitioning;
/// the Boolean provenance mapper then carries these slots and original UVs.
fn bind_surfaces(
    project: &Project,
    asset: &Asset,
    mesh: &mut Mesh,
    table: &mut Vec<petunia_project::SurfaceMaterial>,
) -> Result<(), CommandError> {
    for face in &mut mesh.faces {
        let binding = if asset.surface_materials.is_empty() {
            let material = face
                .material_slot
                .and_then(|slot| project.materials.get(slot));
            petunia_project::SurfaceMaterial {
                material_id: material.map(|m| m.id).or(asset.material_id),
                base_color: asset.base_color,
                texture: if material.is_some() {
                    None
                } else {
                    asset.texture.clone()
                },
            }
        } else {
            face.material_slot
                .and_then(|slot| asset.surface_materials.get(slot))
                .or_else(|| asset.surface_materials.first())
                .cloned()
                .ok_or_else(|| CommandError::Execution("Missing surface material binding".into()))?
        };
        let slot = match table.iter().position(|existing| *existing == binding) {
            Some(slot) => slot,
            None if table.len() < 256 => {
                table.push(binding);
                table.len() - 1
            }
            None => return Err(CommandError::Execution("Too many surface materials".into())),
        };
        face.material_slot = Some(slot);
    }
    Ok(())
}
