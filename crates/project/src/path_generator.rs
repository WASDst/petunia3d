//! Geradores procedurais baseados em path para o fluxo shape-first (P3D-168).

use glam::Vec3;
use petunia_mesh::{Mesh, SweepOptions, triangulate};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Project;
use crate::profile::{ProfileError, ProfileResource};
use crate::spline::{SplineError, SplineInterpolation, SplineResource};

pub const MAX_GENERATED_VERTICES: usize = 1_000_000;
const DEFAULT_VERTEX_BUDGET: u32 = 262_144;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PathGeneratorQuality {
    Preview,
    #[default]
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SweepGeneratorParameters {
    pub profile_spacing: f64,
    pub path_spacing: f64,
    pub tolerance: f64,
    pub preview_spacing_multiplier: f64,
    pub cap_start: bool,
    pub cap_end: bool,
    pub miter: bool,
    pub miter_limit: f32,
    pub vertex_budget: u32,
}

impl Default for SweepGeneratorParameters {
    fn default() -> Self {
        Self {
            profile_spacing: 0.05,
            path_spacing: 0.1,
            tolerance: 1.0e-4,
            preview_spacing_multiplier: 4.0,
            cap_start: true,
            cap_end: true,
            miter: true,
            miter_limit: 3.0,
            vertex_budget: DEFAULT_VERTEX_BUDGET,
        }
    }
}

impl SweepGeneratorParameters {
    pub fn validate(self) -> Result<(), PathGeneratorError> {
        if !self.profile_spacing.is_finite() || self.profile_spacing <= 0.0 {
            return Err(PathGeneratorError::InvalidProfileSpacing);
        }
        if !self.path_spacing.is_finite() || self.path_spacing <= 0.0 {
            return Err(PathGeneratorError::InvalidPathSpacing);
        }
        if !self.tolerance.is_finite() || self.tolerance <= 0.0 {
            return Err(PathGeneratorError::InvalidTolerance);
        }
        if !self.preview_spacing_multiplier.is_finite() || self.preview_spacing_multiplier < 1.0 {
            return Err(PathGeneratorError::InvalidPreviewMultiplier);
        }
        if !self.miter_limit.is_finite() || self.miter_limit < 1.0 {
            return Err(PathGeneratorError::InvalidMiterLimit);
        }
        if self.vertex_budget < 6 || self.vertex_budget as usize > MAX_GENERATED_VERTICES {
            return Err(PathGeneratorError::InvalidVertexBudget);
        }
        Ok(())
    }

    fn spacing_for(self, quality: PathGeneratorQuality) -> (f64, f64) {
        let multiplier = match quality {
            PathGeneratorQuality::Preview => self.preview_spacing_multiplier,
            PathGeneratorQuality::Final => 1.0,
        };
        (
            self.profile_spacing * multiplier,
            self.path_spacing * multiplier,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PathGeneratorKind {
    Sweep(SweepGeneratorParameters),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PathGenerator {
    pub id: Uuid,
    pub name: String,
    pub path_id: Uuid,
    pub profile_id: Uuid,
    pub kind: PathGeneratorKind,
    #[serde(default)]
    pub revision: u64,
}

impl PathGenerator {
    pub fn sweep(
        name: impl Into<String>,
        path_id: Uuid,
        profile_id: Uuid,
        parameters: SweepGeneratorParameters,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            path_id,
            profile_id,
            kind: PathGeneratorKind::Sweep(parameters),
            revision: 0,
        }
    }

    pub fn sweep_parameters(&self) -> SweepGeneratorParameters {
        match self.kind {
            PathGeneratorKind::Sweep(parameters) => parameters,
        }
    }

    pub fn set_sweep_parameters(
        &mut self,
        parameters: SweepGeneratorParameters,
    ) -> Result<bool, PathGeneratorError> {
        parameters.validate()?;
        if self.sweep_parameters() == parameters {
            return Ok(false);
        }
        self.kind = PathGeneratorKind::Sweep(parameters);
        self.revision = self.revision.wrapping_add(1);
        Ok(true)
    }

    pub fn validate_loaded(&mut self) {
        if self.name.trim().is_empty() {
            self.name = "Sweep".to_string();
        }
        let parameters = self.sweep_parameters();
        if parameters.validate().is_err() {
            self.kind = PathGeneratorKind::Sweep(SweepGeneratorParameters::default());
        }
    }

    pub fn content_fingerprint(&self) -> u128 {
        const FNV_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
        const FNV_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;
        let mut fingerprint = FNV_OFFSET;
        let mut mix = |value: u64| {
            fingerprint ^= u128::from(value);
            fingerprint = fingerprint.wrapping_mul(FNV_PRIME);
        };
        for id in [self.id, self.path_id, self.profile_id] {
            mix(id.as_u128() as u64);
            mix((id.as_u128() >> 64) as u64);
        }
        mix(self.revision);
        let parameters = self.sweep_parameters();
        mix(parameters.profile_spacing.to_bits());
        mix(parameters.path_spacing.to_bits());
        mix(parameters.tolerance.to_bits());
        mix(parameters.preview_spacing_multiplier.to_bits());
        mix(parameters.cap_start as u64);
        mix(parameters.cap_end as u64);
        mix(parameters.miter as u64);
        mix(u64::from(parameters.miter_limit.to_bits()));
        mix(u64::from(parameters.vertex_budget));
        fingerprint
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathGeneratorDependencies {
    pub profile_id: Uuid,
    pub profile_spline_id: Uuid,
    pub path_spline_id: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathGeneratorWarning {
    PreviewDecimated,
    StartCapDisabled,
    EndCapDisabled,
    BudgetAboveEightyPercent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathGeneratorDiagnostics {
    pub quality: PathGeneratorQuality,
    pub vertices: usize,
    pub triangles: usize,
    pub vertex_budget: usize,
    pub dependencies: PathGeneratorDependencies,
    pub warnings: Vec<PathGeneratorWarning>,
}

#[derive(Debug, Clone)]
pub struct PathGeneratorEvaluation {
    pub mesh: Mesh,
    pub diagnostics: PathGeneratorDiagnostics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PathGeneratorCacheKey {
    generator: u128,
    profile: u128,
    profile_spline: u128,
    path_spline: u128,
    quality: PathGeneratorQuality,
}

#[derive(Debug, Clone, Default)]
pub struct PathGeneratorEvaluationCache {
    key: Option<PathGeneratorCacheKey>,
    evaluation: Option<PathGeneratorEvaluation>,
    hits: u64,
    misses: u64,
}

impl PathGeneratorEvaluationCache {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn hits(&self) -> u64 {
        self.hits
    }

    pub fn misses(&self) -> u64 {
        self.misses
    }

    pub fn evaluate<'a>(
        &'a mut self,
        project: &Project,
        generator_id: Uuid,
        quality: PathGeneratorQuality,
    ) -> Result<&'a PathGeneratorEvaluation, PathGeneratorError> {
        let generator = project
            .get_path_generator(generator_id)
            .ok_or(PathGeneratorError::GeneratorNotFound(generator_id))?;
        let profile = project
            .get_profile(generator.profile_id)
            .ok_or(PathGeneratorError::ProfileNotFound(generator.profile_id))?;
        let profile_spline = project
            .get_spline(profile.spline_id)
            .ok_or(PathGeneratorError::SplineNotFound(profile.spline_id))?;
        validate_dependencies(generator, profile, profile_spline, project)?;
        let path_spline = project.resolved_spline(generator.path_id)?;
        let key = PathGeneratorCacheKey {
            generator: generator.content_fingerprint(),
            profile: profile.content_fingerprint(),
            profile_spline: profile_spline.geometry_fingerprint(),
            path_spline: path_spline.geometry_fingerprint(),
            quality,
        };
        if self.key != Some(key) {
            self.evaluation = Some(evaluate_sweep(
                generator,
                profile,
                profile_spline,
                &path_spline,
                quality,
            )?);
            self.key = Some(key);
            self.misses = self.misses.wrapping_add(1);
        } else {
            self.hits = self.hits.wrapping_add(1);
        }
        self.evaluation
            .as_ref()
            .ok_or(PathGeneratorError::EvaluationUnavailable)
    }
}

pub(crate) fn validate_path_generator(
    project: &Project,
    generator: &PathGenerator,
) -> Result<(), PathGeneratorError> {
    let profile = project
        .get_profile(generator.profile_id)
        .ok_or(PathGeneratorError::ProfileNotFound(generator.profile_id))?;
    let profile_spline = project
        .get_spline(profile.spline_id)
        .ok_or(PathGeneratorError::SplineNotFound(profile.spline_id))?;
    validate_dependencies(generator, profile, profile_spline, project)
}

fn validate_dependencies(
    generator: &PathGenerator,
    profile: &ProfileResource,
    profile_spline: &SplineResource,
    project: &Project,
) -> Result<(), PathGeneratorError> {
    if generator.name.trim().is_empty() {
        return Err(PathGeneratorError::EmptyName);
    }
    generator.sweep_parameters().validate()?;
    profile.validate_authoring(profile_spline)?;
    if !profile_spline.closed {
        return Err(PathGeneratorError::ProfileMustBeClosed);
    }
    if profile_spline.points.len() < 3 {
        return Err(PathGeneratorError::ProfileNeedsThreePoints);
    }
    if profile.wall_thickness > 0.0 {
        return Err(PathGeneratorError::ProfileThicknessUnsupported);
    }
    let path = project
        .get_spline(generator.path_id)
        .ok_or(PathGeneratorError::SplineNotFound(generator.path_id))?;
    path.validate_authoring()?;
    if path.points.len() < 2 {
        return Err(PathGeneratorError::PathNeedsTwoPoints);
    }
    Ok(())
}

fn evaluate_sweep(
    generator: &PathGenerator,
    profile: &ProfileResource,
    profile_spline: &SplineResource,
    path_spline: &SplineResource,
    quality: PathGeneratorQuality,
) -> Result<PathGeneratorEvaluation, PathGeneratorError> {
    let parameters = generator.sweep_parameters();
    let (profile_spacing, path_spacing) = parameters.spacing_for(quality);
    let profile_points = sampled_positions(profile_spline, profile_spacing, parameters.tolerance)?;
    let path_points = sampled_positions(path_spline, path_spacing, parameters.tolerance)?;
    if profile_points.len() < 3 {
        return Err(PathGeneratorError::ProfileNeedsThreePoints);
    }
    if path_points.len() < 2 {
        return Err(PathGeneratorError::PathNeedsTwoPoints);
    }
    let vertices = profile_points.len().checked_mul(path_points.len()).ok_or(
        PathGeneratorError::BudgetExceeded {
            estimated: usize::MAX,
            budget: parameters.vertex_budget as usize,
        },
    )?;
    let budget = parameters.vertex_budget as usize;
    if vertices > budget || vertices > MAX_GENERATED_VERTICES {
        return Err(PathGeneratorError::BudgetExceeded {
            estimated: vertices,
            budget,
        });
    }

    let profile_2d: Vec<_> = profile_points
        .into_iter()
        .map(|position| finite_f32_2(position[0], position[1]))
        .collect::<Result<_, _>>()?;
    if !path_spline.closed && (parameters.cap_start || parameters.cap_end) {
        triangulate::ear_clip(&profile_2d).map_err(PathGeneratorError::InvalidCapProfile)?;
    }
    let path_3d: Vec<_> = path_points
        .into_iter()
        .map(finite_vec3)
        .collect::<Result<_, _>>()?;
    let mesh = Mesh::from_sweep(
        &profile_2d,
        &path_3d,
        SweepOptions {
            closed_path: path_spline.closed,
            closed_profile: true,
            cap_start: parameters.cap_start,
            cap_end: parameters.cap_end,
            miter: parameters.miter,
            miter_limit: parameters.miter_limit,
        },
    )
    .map_err(PathGeneratorError::MeshGeneration)?;

    let mut warnings = Vec::new();
    if quality == PathGeneratorQuality::Preview && parameters.preview_spacing_multiplier > 1.0 {
        warnings.push(PathGeneratorWarning::PreviewDecimated);
    }
    if !path_spline.closed && !parameters.cap_start {
        warnings.push(PathGeneratorWarning::StartCapDisabled);
    }
    if !path_spline.closed && !parameters.cap_end {
        warnings.push(PathGeneratorWarning::EndCapDisabled);
    }
    if vertices.saturating_mul(5) >= budget.saturating_mul(4) {
        warnings.push(PathGeneratorWarning::BudgetAboveEightyPercent);
    }
    let diagnostics = PathGeneratorDiagnostics {
        quality,
        vertices: mesh.vert_count(),
        triangles: mesh.tri_count(),
        vertex_budget: budget,
        dependencies: PathGeneratorDependencies {
            profile_id: profile.id,
            profile_spline_id: profile.spline_id,
            path_spline_id: generator.path_id,
        },
        warnings,
    };
    Ok(PathGeneratorEvaluation { mesh, diagnostics })
}

fn sampled_positions(
    spline: &SplineResource,
    spacing: f64,
    tolerance: f64,
) -> Result<Vec<[f64; 3]>, PathGeneratorError> {
    // Uma curva Bézier cujos handles são todos nulos é geometricamente uma
    // polilinha (ex.: Rectangle e cantos vivos do editor de Profile). Reamostrar
    // por espaçamento cortaria os cantos; usar os vértices preserva a forma.
    let is_degenerate_bezier = spline.interpolation == SplineInterpolation::CubicBezier
        && spline
            .points
            .iter()
            .all(|point| point.handle_in == [0.0; 3] && point.handle_out == [0.0; 3]);
    match spline.interpolation {
        SplineInterpolation::Polyline => {
            Ok(spline.points.iter().map(|point| point.position).collect())
        }
        SplineInterpolation::CubicBezier if is_degenerate_bezier => {
            Ok(spline.points.iter().map(|point| point.position).collect())
        }
        SplineInterpolation::CubicBezier => Ok(spline
            .resample_by_spacing(spacing, tolerance)?
            .into_iter()
            .map(|sample| sample.position)
            .collect()),
    }
}

fn finite_f32_2(x: f64, y: f64) -> Result<[f32; 2], PathGeneratorError> {
    let point = [x as f32, y as f32];
    point
        .iter()
        .all(|component| component.is_finite())
        .then_some(point)
        .ok_or(PathGeneratorError::CoordinateOutsideMeshRange)
}

fn finite_vec3(position: [f64; 3]) -> Result<Vec3, PathGeneratorError> {
    let point = Vec3::new(position[0] as f32, position[1] as f32, position[2] as f32);
    point
        .is_finite()
        .then_some(point)
        .ok_or(PathGeneratorError::CoordinateOutsideMeshRange)
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PathGeneratorError {
    #[error("path generator id is duplicated: {0}")]
    DuplicateGenerator(Uuid),
    #[error("path generator was not found: {0}")]
    GeneratorNotFound(Uuid),
    #[error("profile was not found: {0}")]
    ProfileNotFound(Uuid),
    #[error("spline was not found: {0}")]
    SplineNotFound(Uuid),
    #[error("path generator name cannot be empty")]
    EmptyName,
    #[error("profile sampling spacing must be positive and finite")]
    InvalidProfileSpacing,
    #[error("path sampling spacing must be positive and finite")]
    InvalidPathSpacing,
    #[error("sampling tolerance must be positive and finite")]
    InvalidTolerance,
    #[error("preview spacing multiplier must be finite and at least one")]
    InvalidPreviewMultiplier,
    #[error("miter limit must be finite and at least one")]
    InvalidMiterLimit,
    #[error("vertex budget is outside the supported range")]
    InvalidVertexBudget,
    #[error("profile needs at least three points")]
    ProfileNeedsThreePoints,
    #[error("profile must be closed")]
    ProfileMustBeClosed,
    #[error("path needs at least two points")]
    PathNeedsTwoPoints,
    #[error("hollow profiles are not supported by Sweep yet")]
    ProfileThicknessUnsupported,
    #[error("generated mesh needs {estimated} vertices, above budget {budget}")]
    BudgetExceeded { estimated: usize, budget: usize },
    #[error("profile cannot be triangulated for caps: {0}")]
    InvalidCapProfile(String),
    #[error("profile or path coordinate is outside the mesh numeric range")]
    CoordinateOutsideMeshRange,
    #[error("sweep mesh generation failed: {0}")]
    MeshGeneration(String),
    #[error("path generator evaluation did not produce a result")]
    EvaluationUnavailable,
    #[error(transparent)]
    Profile(#[from] ProfileError),
    #[error(transparent)]
    Spline(#[from] SplineError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProfileWorkplane, SplineResource};

    #[test]
    fn zero_handle_bezier_keeps_its_corners() {
        let mut square = SplineResource::from_polyline(
            "Square bezier",
            &[
                [-0.5, -0.5, 0.0],
                [0.5, -0.5, 0.0],
                [0.5, 0.5, 0.0],
                [-0.5, 0.5, 0.0],
            ],
            true,
        );
        square.interpolation = SplineInterpolation::CubicBezier;
        // Espaçamento que não divide o perímetro: reamostrar cortaria cantos.
        let sampled = sampled_positions(&square, 0.37, 1e-3).unwrap();
        assert_eq!(sampled.len(), 4);
        for corner in [[-0.5, -0.5, 0.0], [0.5, 0.5, 0.0]] {
            assert!(sampled.contains(&corner));
        }
    }

    fn project_with_sweep() -> (Project, Uuid) {
        let mut project = Project::default();
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
        let profile =
            ProfileResource::new("Square", profile_spline.id, ProfileWorkplane::default());
        let profile_id = profile.id;
        project.add_spline(profile_spline).unwrap();
        project.add_profile(profile).unwrap();
        let path = SplineResource::from_polyline(
            "Path",
            &[[0.0, 0.0, 0.0], [0.0, 0.0, 2.0], [1.0, 0.0, 3.0]],
            false,
        );
        let path_id = path.id;
        project.add_spline(path).unwrap();
        let generator = PathGenerator::sweep(
            "Sweep",
            path_id,
            profile_id,
            SweepGeneratorParameters::default(),
        );
        let generator_id = generator.id;
        project.add_path_generator(generator).unwrap();
        (project, generator_id)
    }

    #[test]
    fn sweep_evaluation_is_deterministic_and_reports_dependencies() {
        let (project, generator_id) = project_with_sweep();
        let mut first_cache = PathGeneratorEvaluationCache::default();
        let first = first_cache
            .evaluate(&project, generator_id, PathGeneratorQuality::Final)
            .unwrap()
            .clone();
        let mut second_cache = PathGeneratorEvaluationCache::default();
        let second = second_cache
            .evaluate(&project, generator_id, PathGeneratorQuality::Final)
            .unwrap();

        assert_eq!(first.mesh.vert_count(), 12);
        assert_eq!(first.mesh.tri_count(), 20);
        assert_eq!(first.mesh.verts.len(), second.mesh.verts.len());
        assert!(
            first
                .mesh
                .verts
                .iter()
                .zip(&second.mesh.verts)
                .all(|(left, right)| left.pos == right.pos)
        );
        assert_eq!(
            first.diagnostics.dependencies.path_spline_id,
            project.get_path_generator(generator_id).unwrap().path_id
        );
    }

    #[test]
    fn evaluation_cache_invalidates_after_dependency_edit() {
        let (mut project, generator_id) = project_with_sweep();
        let mut cache = PathGeneratorEvaluationCache::default();
        cache
            .evaluate(&project, generator_id, PathGeneratorQuality::Final)
            .unwrap();
        cache
            .evaluate(&project, generator_id, PathGeneratorQuality::Final)
            .unwrap();
        assert_eq!((cache.hits(), cache.misses()), (1, 1));

        let path_id = project.get_path_generator(generator_id).unwrap().path_id;
        let point_id = project.get_spline(path_id).unwrap().points[1].id;
        project
            .get_spline_mut(path_id)
            .unwrap()
            .move_point(point_id, [0.5, 0.0, 2.0])
            .unwrap();
        cache
            .evaluate(&project, generator_id, PathGeneratorQuality::Final)
            .unwrap();
        assert_eq!((cache.hits(), cache.misses()), (1, 2));
    }

    #[test]
    fn evaluation_cache_tracks_resolved_surface_attachment_geometry() {
        let (mut project, generator_id) = project_with_sweep();
        project.add("Attachment target", Mesh::cube(2.0));
        let target_id = project.assets[0].id;
        let hit = crate::project_ray_to_surface_target(
            &project,
            target_id,
            [0.0, 3.0, 0.0],
            [0.0, -1.0, 0.0],
            10.0,
        )
        .unwrap()
        .unwrap();
        let path_id = project.get_path_generator(generator_id).unwrap().path_id;
        let point_id = project.get_spline(path_id).unwrap().points[0].id;
        project
            .get_spline_mut(path_id)
            .unwrap()
            .set_attachment(
                point_id,
                Some(hit.attachment),
                hit.frame.position.map(f64::from),
            )
            .unwrap();

        let mut cache = PathGeneratorEvaluationCache::default();
        cache
            .evaluate(&project, generator_id, PathGeneratorQuality::Final)
            .unwrap();
        cache
            .evaluate(&project, generator_id, PathGeneratorQuality::Final)
            .unwrap();
        assert_eq!((cache.hits(), cache.misses()), (1, 1));

        let face_index = hit.attachment.triangle.face_index as usize;
        let triangle_index = hit.attachment.triangle.triangle_index as usize;
        let mesh = &mut project.assets[0].mesh;
        let triangle = mesh.face_triangle_corners(face_index)[triangle_index];
        let dominant_corner = hit
            .attachment
            .barycentric
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .unwrap()
            .0;
        let vertex_index = mesh.faces[face_index].verts[triangle[dominant_corner]] as usize;
        mesh.verts[vertex_index].pos[1] += 0.25;

        cache
            .evaluate(&project, generator_id, PathGeneratorQuality::Final)
            .unwrap();
        assert_eq!((cache.hits(), cache.misses()), (1, 2));
    }

    #[test]
    fn open_profile_draft_is_valid_authoring_but_not_a_sweep_dependency() {
        let (mut project, generator_id) = project_with_sweep();
        let profile_id = project.get_path_generator(generator_id).unwrap().profile_id;
        let spline_id = project.get_profile(profile_id).unwrap().spline_id;
        project
            .get_spline_mut(spline_id)
            .unwrap()
            .set_closed(false)
            .unwrap();

        let error = PathGeneratorEvaluationCache::default()
            .evaluate(&project, generator_id, PathGeneratorQuality::Final)
            .unwrap_err();
        assert_eq!(error, PathGeneratorError::ProfileMustBeClosed);
    }

    #[test]
    fn configured_budget_rejects_oversized_mesh_before_generation() {
        let (mut project, generator_id) = project_with_sweep();
        let generator = project.get_path_generator_mut(generator_id).unwrap();
        let mut parameters = generator.sweep_parameters();
        parameters.vertex_budget = 6;
        generator.set_sweep_parameters(parameters).unwrap();

        let error = PathGeneratorEvaluationCache::default()
            .evaluate(&project, generator_id, PathGeneratorQuality::Final)
            .unwrap_err();
        assert!(matches!(error, PathGeneratorError::BudgetExceeded { .. }));
    }
}
