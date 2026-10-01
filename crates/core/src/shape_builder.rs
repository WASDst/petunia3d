//! Shape Builder e Pathfinder do DRAW (polígonos).
//!
//! Os perfis fechados de um plano formam um **arranjo planar**: cada região
//! limitada é uma *face*. O Shape Builder trabalha nessas faces, como a
//! ferramenta homônima da arte vetorial:
//!
//! - **Merge** (arrastar por cima de várias faces): une as faces tocadas numa só;
//! - **Delete** (Ctrl + arrastar): remove as faces tocadas;
//! - **clique**: extrai a face como forma independente (é um Merge de uma face).
//!
//! O **Pathfinder** age sobre a pilha de perfis fechados do plano: Unite,
//! Subtract (a forma mais ao fundo menos as da frente), Intersect e Exclude.
//!
//! Em todos os casos o resultado substitui os perfis fechados do plano por laços
//! poligonais que ladrilham as faces resultantes: o arranjo recalculado dá
//! exatamente as faces esperadas. Tudo vira **uma** entrada de Undo (o comando é
//! despachado normalmente). Perfis usados por um Path Generator ficam de fora, e
//! os perfis abertos continuam onde estão.
//!
//! Só polígonos: as curvas chegam tesseladas e saem como polilinhas.

use glam::Vec3;
use petunia_mesh::arrangement::{Region2, region_at};
use petunia_mesh::shape_ops::{face_loops, pathfinder, regions_union};
use petunia_project::{ProfileResource, ProfileWorkplane, SplineResource};

pub use petunia_mesh::shape_ops::PathfinderOp;

use crate::command::{Command, CommandError};
use crate::region_push::{ProfileGroup, RegionPlane};
use crate::state::AppState;

/// Teto de laços gravados por operação: um plano com mais faces que isto quase
/// certamente veio de um gesto descontrolado.
const MAX_LOOPS: usize = 2_000;

/// Plano editável do Shape Builder.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapePlane {
    pub plane: RegionPlane,
    /// Perfis (fechados e abertos) do plano que o Shape Builder pode editar.
    pub profiles: Vec<uuid::Uuid>,
    pub regions: Vec<Region2>,
}

/// Edição pedida ao Shape Builder.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeEdit {
    /// Une as faces sob os pontos do caminho (mundo, sobre o plano).
    Merge { samples: Vec<Vec3> },
    /// Remove as faces sob os pontos do caminho.
    Delete { samples: Vec<Vec3> },
    /// Operação do Pathfinder sobre os perfis fechados do plano.
    Pathfinder(PathfinderOp),
}

/// Resultado de uma edição, para a mensagem de status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeEditReport {
    pub closed_before: usize,
    pub closed_after: usize,
    pub faces_before: usize,
    pub faces_after: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ShapeEditError {
    #[error("Draw at least one closed shape first")]
    NoShapes,
    #[error("Pass over the regions you want to change")]
    NothingTouched,
    #[error("Pathfinder needs two or more closed shapes on the same plane")]
    NeedsTwoShapes,
    #[error("The operation would leave nothing")]
    Empty,
    #[error("Too many pieces for one operation")]
    TooManyPieces,
}

impl AppState {
    /// Planos com perfis que o Shape Builder pode editar, e suas faces.
    pub fn shape_planes(&self) -> Vec<ShapePlane> {
        self.profile_groups(true)
            .into_iter()
            .map(|group| ShapePlane {
                plane: group.plane,
                profiles: group.profiles.clone(),
                regions: petunia_mesh::arrangement::planar_regions(&group.lines),
            })
            .filter(|plane| !plane.regions.is_empty())
            .collect()
    }

    fn same_plane(a: &RegionPlane, b: &RegionPlane) -> bool {
        a.normal.dot(b.normal).abs() > 0.9999 && (a.origin - b.origin).dot(a.normal).abs() < 1.0e-4
    }

    /// Aplica `edit` ao plano `hint` (ou ao do perfil mais recente).
    pub fn apply_shape_edit(
        &mut self,
        hint: Option<&RegionPlane>,
        edit: &ShapeEdit,
    ) -> Result<ShapeEditReport, ShapeEditError> {
        let groups = self.profile_groups(true);
        let group: &ProfileGroup = match hint {
            Some(plane) => groups
                .iter()
                .find(|g| Self::same_plane(&g.plane, plane))
                .ok_or(ShapeEditError::NoShapes)?,
            // Sem plano indicado: o do perfil mais recente (o último criado).
            None => {
                let latest = self
                    .project
                    .project
                    .profiles
                    .iter()
                    .rev()
                    .find_map(|profile| groups.iter().find(|g| g.profiles.contains(&profile.id)));
                latest.ok_or(ShapeEditError::NoShapes)?
            }
        };
        let plane = group.plane;
        let regions = petunia_mesh::arrangement::planar_regions(&group.lines);
        if regions.is_empty() {
            return Err(ShapeEditError::NoShapes);
        }
        let closed: Vec<(uuid::Uuid, &petunia_mesh::arrangement::Polyline2)> = group
            .profiles
            .iter()
            .copied()
            .zip(&group.lines)
            .filter(|(_, line)| line.closed)
            .collect();

        let faces: Vec<Region2> = match edit {
            ShapeEdit::Merge { samples } | ShapeEdit::Delete { samples } => {
                let mut touched: Vec<usize> = samples
                    .iter()
                    .filter_map(|p| region_at(&regions, plane.to_plane(*p)))
                    .collect();
                touched.sort_unstable();
                touched.dedup();
                if touched.is_empty() {
                    return Err(ShapeEditError::NothingTouched);
                }
                let rest = regions
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| !touched.contains(i))
                    .map(|(_, r)| r.clone());
                let mut out: Vec<Region2> = Vec::new();
                if matches!(edit, ShapeEdit::Merge { .. }) {
                    let picked: Vec<Region2> =
                        touched.iter().map(|&i| regions[i].clone()).collect();
                    out.extend(regions_union(&picked));
                }
                out.extend(rest);
                out
            }
            ShapeEdit::Pathfinder(op) => {
                if closed.len() < 2 {
                    return Err(ShapeEditError::NeedsTwoShapes);
                }
                let shapes: Vec<Vec<[f64; 2]>> =
                    closed.iter().map(|(_, line)| line.points.clone()).collect();
                let result = pathfinder(&shapes, *op);
                if result.is_empty() {
                    return Err(ShapeEditError::Empty);
                }
                result
            }
        };
        let loops = face_loops(&faces);
        if loops.len() > MAX_LOOPS {
            return Err(ShapeEditError::TooManyPieces);
        }
        let report = ShapeEditReport {
            closed_before: closed.len(),
            closed_after: loops.len(),
            faces_before: regions.len(),
            faces_after: faces.len(),
        };

        // Escrita: remove os perfis fechados do plano e grava os laços novos.
        let removed: Vec<uuid::Uuid> = closed.iter().map(|(id, _)| *id).collect();
        let wall_thickness = removed
            .first()
            .and_then(|id| self.project.project.get_profile(*id))
            .map_or(0.0, |profile| profile.wall_thickness);
        for id in &removed {
            if let Some(profile) = self.project.project.get_profile(*id).cloned() {
                let _ = self.project.project.remove_profile(*id);
                let _ = self.project.project.remove_spline(profile.spline_id);
            }
        }
        let workplane = ProfileWorkplane {
            origin: plane.origin.to_array().map(f64::from),
            right: plane.right.to_array().map(f64::from),
            up: plane.up.to_array().map(f64::from),
            normal: plane.normal.to_array().map(f64::from),
        };
        let name = self.t_id(petunia_config::text_id::DRAW_SHAPE_NAME);
        for points in loops {
            let points3: Vec<[f64; 3]> = points.iter().map(|p| [p[0], p[1], 0.0]).collect();
            let spline = SplineResource::from_polyline(&name, &points3, true);
            let mut profile = ProfileResource::new(&name, spline.id, workplane);
            profile.wall_thickness = wall_thickness;
            // Falhas aqui são de dados inválidos (não finitos): o dispatcher
            // desfaz o comando inteiro ao receber o erro.
            if self.project.project.add_spline(spline).is_err()
                || self.project.project.add_profile(profile).is_err()
            {
                return Err(ShapeEditError::Empty);
            }
        }
        Ok(report)
    }
}

/// Comando de edição de formas (Shape Builder e Pathfinder).
#[derive(Debug, Clone)]
pub struct ShapeEditCmd {
    pub plane: Option<RegionPlane>,
    pub edit: ShapeEdit,
}

impl ShapeEditCmd {
    pub fn pathfinder(op: PathfinderOp) -> Self {
        Self {
            plane: None,
            edit: ShapeEdit::Pathfinder(op),
        }
    }

    fn title(&self) -> &'static str {
        match &self.edit {
            ShapeEdit::Merge { .. } => "Shape Builder: merge",
            ShapeEdit::Delete { .. } => "Shape Builder: delete",
            ShapeEdit::Pathfinder(PathfinderOp::Unite) => "Unite",
            ShapeEdit::Pathfinder(PathfinderOp::Subtract) => "Subtract",
            ShapeEdit::Pathfinder(PathfinderOp::Intersect) => "Intersect",
            ShapeEdit::Pathfinder(PathfinderOp::Exclude) => "Exclude",
        }
    }
}

impl Command for ShapeEditCmd {
    fn label(&self) -> &'static str {
        match &self.edit {
            ShapeEdit::Merge { .. } => "shape builder merge",
            ShapeEdit::Delete { .. } => "shape builder delete",
            ShapeEdit::Pathfinder(PathfinderOp::Unite) => "shape unite",
            ShapeEdit::Pathfinder(PathfinderOp::Subtract) => "shape subtract",
            ShapeEdit::Pathfinder(PathfinderOp::Intersect) => "shape intersect",
            ShapeEdit::Pathfinder(PathfinderOp::Exclude) => "shape exclude",
        }
    }

    fn changes(&self) -> petunia_project::ProjectChanges {
        petunia_project::ProjectChanges::SPLINES | petunia_project::ProjectChanges::PROCEDURAL
    }

    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if state.project.project.profiles.is_empty() {
            return Err("Draw at least one closed shape first");
        }
        Ok(())
    }

    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        match state.apply_shape_edit(self.plane.as_ref(), &self.edit) {
            Ok(report) => {
                state.set_status(format!(
                    "{}: {} faces → {} faces",
                    self.title(),
                    report.faces_before,
                    report.faces_after
                ));
                Ok(())
            }
            Err(error) => Err(CommandError::Execution(error.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ViewPreset;

    fn plane() -> ProfileWorkplane {
        ProfileWorkplane {
            origin: [0.0, 0.0, 5.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            normal: [0.0, 0.0, 1.0],
        }
    }

    fn add_square(state: &mut AppState, x: f64, y: f64, size: f64) {
        let points = [
            [x, y, 0.0],
            [x + size, y, 0.0],
            [x + size, y + size, 0.0],
            [x, y + size, 0.0],
        ];
        let spline = SplineResource::from_polyline("curve", &points, true);
        let profile = ProfileResource::new("profile", spline.id, plane());
        state.project.project.splines.push(spline);
        state.project.project.profiles.push(profile);
    }

    fn two_squares() -> AppState {
        let mut state = AppState::default();
        add_square(&mut state, 0.0, 0.0, 2.0);
        add_square(&mut state, 1.0, 1.0, 2.0);
        state.session.camera.set_preset(ViewPreset::Front);
        state
    }

    fn sample(x: f32, y: f32) -> Vec3 {
        Vec3::new(x, y, 5.0)
    }

    fn face_areas(state: &AppState) -> Vec<f64> {
        let mut areas: Vec<f64> = state
            .shape_planes()
            .iter()
            .flat_map(|p| p.regions.iter().map(Region2::area))
            .collect();
        areas.sort_by(f64::total_cmp);
        areas
    }

    #[test]
    fn merge_drag_over_two_faces_fuses_them_and_keeps_the_rest() {
        let mut state = two_squares();
        assert_eq!(face_areas(&state).len(), 3);
        let cmd = ShapeEditCmd {
            plane: None,
            edit: ShapeEdit::Merge {
                samples: vec![sample(0.5, 0.5), sample(1.5, 1.5)],
            },
        };
        state.dispatch(&cmd).unwrap();
        let areas = face_areas(&state);
        assert_eq!(areas.len(), 2, "{areas:?}");
        assert!(
            (areas[0] - 3.0).abs() < 1e-6,
            "sobra do segundo quadrado: {areas:?}"
        );
        assert!(
            (areas[1] - 4.0).abs() < 1e-6,
            "primeiro quadrado fundido com a sobreposição: {areas:?}"
        );
    }

    #[test]
    fn click_on_one_face_extracts_it_without_changing_the_silhouette() {
        let mut state = two_squares();
        let cmd = ShapeEditCmd {
            plane: None,
            edit: ShapeEdit::Merge {
                samples: vec![sample(1.5, 1.5)],
            },
        };
        state.dispatch(&cmd).unwrap();
        let total: f64 = face_areas(&state).iter().sum();
        assert!((total - 7.0).abs() < 1e-6);
        assert_eq!(face_areas(&state).len(), 3);
    }

    #[test]
    fn delete_removes_only_the_touched_faces() {
        let mut state = two_squares();
        // Apaga a parte só do primeiro quadrado (o "L" de área 3): sobram a
        // sobreposição e a parte só do segundo, que juntas são o segundo quadrado.
        let cmd = ShapeEditCmd {
            plane: None,
            edit: ShapeEdit::Delete {
                samples: vec![sample(0.5, 0.5)],
            },
        };
        state.dispatch(&cmd).unwrap();
        let areas = face_areas(&state);
        assert_eq!(areas, vec![1.0, 3.0], "{areas:?}");
    }

    #[test]
    fn deleting_an_enclosed_face_leaves_its_outline_as_a_void_region() {
        // Limite conhecido: o arranjo planar devolve como região todo espaço
        // fechado por contornos; apagar uma face cercada deixa o vazio como região.
        let mut state = two_squares();
        let cmd = ShapeEditCmd {
            plane: None,
            edit: ShapeEdit::Delete {
                samples: vec![sample(1.5, 1.5)],
            },
        };
        state.dispatch(&cmd).unwrap();
        let areas = face_areas(&state);
        assert_eq!(areas, vec![1.0, 3.0, 3.0], "{areas:?}");
    }

    #[test]
    fn nothing_touched_is_an_error_and_leaves_the_document_alone() {
        let mut state = two_squares();
        let before = state.project.project.profiles.len();
        let cmd = ShapeEditCmd {
            plane: None,
            edit: ShapeEdit::Merge {
                samples: vec![sample(10.0, 10.0)],
            },
        };
        assert!(state.dispatch(&cmd).is_err());
        assert_eq!(state.project.project.profiles.len(), before);
    }

    #[test]
    fn pathfinder_operations_match_set_algebra_in_one_undo_step() {
        // Área total das faces do plano depois da operação. Exclude deixa o
        // vazio da sobreposição como região (ver o teste do vazio): 3 + 3 + 1.
        for (op, expected) in [
            (PathfinderOp::Unite, 7.0),
            (PathfinderOp::Subtract, 3.0),
            (PathfinderOp::Intersect, 1.0),
            (PathfinderOp::Exclude, 7.0),
        ] {
            let mut state = two_squares();
            let depth = state.project.undo.depth().0;
            state.dispatch(&ShapeEditCmd::pathfinder(op)).unwrap();
            let total: f64 = face_areas(&state).iter().sum();
            assert!((total - expected).abs() < 1e-6, "{op:?}: {total}");
            assert_eq!(state.project.undo.depth().0, depth + 1, "{op:?}: um Undo");
            state.undo();
            assert_eq!(
                state.project.project.profiles.len(),
                2,
                "{op:?}: Undo restaura"
            );
            assert_eq!(face_areas(&state).len(), 3);
        }
    }

    #[test]
    fn pathfinder_needs_two_shapes() {
        let mut state = AppState::default();
        add_square(&mut state, 0.0, 0.0, 2.0);
        assert!(
            state
                .dispatch(&ShapeEditCmd::pathfinder(PathfinderOp::Unite))
                .is_err()
        );
    }

    #[test]
    fn profiles_used_by_a_generator_are_not_touched() {
        let mut state = two_squares();
        // O segundo perfil passa a ser usado por um gerador: sai do Shape Builder.
        let used = state.project.project.profiles[1].id;
        let path =
            SplineResource::from_polyline("path", &[[0.0, 0.0, 0.0], [0.0, 0.0, 3.0]], false);
        let path_id = path.id;
        state.project.project.splines.push(path);
        state
            .project
            .project
            .path_generators
            .push(petunia_project::PathGenerator::sweep(
                "sweep",
                path_id,
                used,
                petunia_project::SweepGeneratorParameters::default(),
            ));
        let planes = state.shape_planes();
        assert_eq!(planes.len(), 1);
        assert_eq!(
            planes[0].profiles.len(),
            1,
            "só o primeiro perfil é editável"
        );
        assert_eq!(planes[0].regions.len(), 1);
    }
}
