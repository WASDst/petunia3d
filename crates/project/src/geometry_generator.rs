//! Persistent shape-first sources. Evaluated meshes are disposable, never authoring data.
use glam::Vec3;
use petunia_mesh::{Mesh, PrimitiveDescriptor, SweepOptions};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{ProfileWorkplane, Project};

pub const MAX_GENERATOR_VERTICES: usize = 262_144;
const CURVE_TOLERANCE: f64 = 0.005;
fn default_cap() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeometryGenerator {
    pub id: Uuid,
    pub revision: u64,
    pub kind: GeometryGeneratorKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeometryGeneratorKind {
    Primitive(PrimitiveDescriptor),
    Extrude {
        shape_object: Uuid,
        distance: f32,
        cap_start: bool,
        cap_end: bool,
    },
    Revolve {
        profile_object: Uuid,
        axis: RevolutionAxis,
        angle: f32,
        segments: u32,
        #[serde(default = "default_cap")]
        cap_start: bool,
        #[serde(default = "default_cap")]
        cap_end: bool,
    },
    Sweep {
        shape_object: Uuid,
        path_object: Uuid,
        cap_start: bool,
        cap_end: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RevolutionAxis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GeneratorError {
    #[error("Generator source is missing: {0}")]
    MissingSource(Uuid),
    #[error("Invalid generator parameters: {0}")]
    Invalid(&'static str),
    #[error("Generator evaluation failed: {0}")]
    Geometry(String),
    #[error("Generator exceeds the safe geometry budget")]
    Budget,
}

impl GeometryGenerator {
    pub fn new(kind: GeometryGeneratorKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            revision: 0,
            kind,
        }
    }

    pub fn depends_on_shape(&self, id: Uuid) -> bool {
        match self.kind {
            GeometryGeneratorKind::Extrude { shape_object, .. }
            | GeometryGeneratorKind::Sweep { shape_object, .. } => shape_object == id,
            GeometryGeneratorKind::Revolve { profile_object, .. } => profile_object == id,
            GeometryGeneratorKind::Primitive(_) => false,
        }
    }

    pub fn depends_on_spline(&self, id: Uuid) -> bool {
        matches!(self.kind, GeometryGeneratorKind::Sweep { path_object, .. } if path_object == id)
    }

    pub fn evaluate(&self, project: &Project) -> Result<Mesh, GeneratorError> {
        let mut mesh = match self.kind {
            GeometryGeneratorKind::Primitive(descriptor) => {
                validate_primitive_descriptor(descriptor)?;
                descriptor.build()
            }
            GeometryGeneratorKind::Extrude {
                shape_object,
                distance,
                cap_start,
                cap_end,
            } => {
                if !distance.is_finite() || distance.abs() < 1.0e-6 || distance.abs() > 100_000.0 {
                    return Err(GeneratorError::Invalid(
                        "distance must be finite and non-zero",
                    ));
                }
                let (outer, holes, plane, closed) = source_contours(project, shape_object)?;
                if !closed || outer.len() < 3 {
                    return Err(GeneratorError::Invalid("extrusion requires a closed shape"));
                }
                let mut mesh = petunia_mesh::imprint::region_sheet(
                    &outer,
                    &holes,
                    Vec3::ZERO,
                    Vec3::X,
                    Vec3::Y,
                )
                .map_err(|e| GeneratorError::Geometry(e.to_string()))?;
                mesh.extrude_selected(distance.abs());
                if !cap_start || !cap_end {
                    let (cap_start, cap_end) = if distance < 0.0 {
                        (cap_end, cap_start)
                    } else {
                        (cap_start, cap_end)
                    };
                    let keep: Vec<_> = (0..mesh.faces.len())
                        .map(|i| {
                            let normal = mesh.face_normal(i);
                            (cap_start || normal.z > -0.9) && (cap_end || normal.z < 0.9)
                        })
                        .collect();
                    mesh.faces = mesh
                        .faces
                        .into_iter()
                        .zip(keep)
                        .filter_map(|(face, keep)| keep.then_some(face))
                        .collect();
                }
                let offset = if distance < 0.0 { distance } else { 0.0 };
                map_to_plane(&mut mesh, plane, |p| Vec3::new(p.x, p.y, p.z + offset));
                mesh
            }
            GeometryGeneratorKind::Revolve {
                profile_object,
                axis,
                angle,
                segments,
                cap_start,
                cap_end,
            } => {
                if !angle.is_finite()
                    || !(0.01..=360.0).contains(&angle)
                    || !(3..=512).contains(&segments)
                {
                    return Err(GeneratorError::Invalid("revolve angle or segment count"));
                }
                let (outer, holes, plane, closed) = source_contours(project, profile_object)?;
                if outer.len() < 2 || !holes.is_empty() {
                    return Err(GeneratorError::Invalid(
                        "revolve requires a simple planar spline",
                    ));
                }
                if outer
                    .len()
                    .checked_mul(segments as usize)
                    .is_none_or(|n| n > MAX_GENERATOR_VERTICES)
                {
                    return Err(GeneratorError::Budget);
                }
                let points: Vec<_> = outer
                    .iter()
                    .map(|p| match axis {
                        RevolutionAxis::Vertical => [p[0] as f32, p[1] as f32],
                        RevolutionAxis::Horizontal => [p[1] as f32, p[0] as f32],
                    })
                    .collect();
                let mut mesh =
                    Mesh::revolve_profile(&points, segments, angle, closed, [cap_start, cap_end])
                        .map_err(GeneratorError::Geometry)?;
                map_to_plane(&mut mesh, plane, |p| match axis {
                    RevolutionAxis::Vertical => p,
                    RevolutionAxis::Horizontal => Vec3::new(p.y, p.x, -p.z),
                });
                mesh
            }
            GeometryGeneratorKind::Sweep {
                shape_object,
                path_object,
                cap_start,
                cap_end,
            } => {
                let (outer, holes, _, closed) = source_contours(project, shape_object)?;
                if !closed || outer.len() < 3 {
                    return Err(GeneratorError::Invalid("sweep requires a closed shape"));
                }
                let path = project
                    .resolved_spline(path_object)
                    .map_err(|e| GeneratorError::Geometry(e.to_string()))?;
                let table = path
                    .arc_length_table(CURVE_TOLERANCE)
                    .map_err(|e| GeneratorError::Geometry(e.to_string()))?;
                let path_plane = project
                    .profiles
                    .iter()
                    .find(|p| p.spline_id == path_object || p.hole_splines.contains(&path_object))
                    .map(|p| p.workplane);
                let mut path_points: Vec<_> = table
                    .polyline()
                    .into_iter()
                    .map(|p| {
                        let local = Vec3::from_array(p.map(|v| v as f32));
                        path_plane.map_or(local, |plane| {
                            Vec3::from_array(plane.origin.map(|v| v as f32))
                                + Vec3::from_array(plane.right.map(|v| v as f32)) * local.x
                                + Vec3::from_array(plane.up.map(|v| v as f32)) * local.y
                                + Vec3::from_array(plane.normal.map(|v| v as f32)) * local.z
                        })
                    })
                    .collect();
                path_points.dedup_by(|a, b| a.abs_diff_eq(*b, 1.0e-8));
                if path.closed
                    && path_points
                        .first()
                        .zip(path_points.last())
                        .is_some_and(|(a, b)| a.abs_diff_eq(*b, 1.0e-8))
                {
                    path_points.pop();
                }
                if path_points.len() < 2 {
                    return Err(GeneratorError::Invalid(
                        "select an explicit spline path with at least two points",
                    ));
                }
                if outer
                    .len()
                    .checked_mul(path_points.len())
                    .is_none_or(|n| n > MAX_GENERATOR_VERTICES)
                {
                    return Err(GeneratorError::Budget);
                }
                compound_sweep(
                    &outer,
                    &holes,
                    &path_points,
                    path.closed,
                    cap_start,
                    cap_end,
                )?
            }
        };
        if mesh.verts.len() > MAX_GENERATOR_VERTICES
            || mesh.faces.len() > MAX_GENERATOR_VERTICES.saturating_mul(2)
        {
            return Err(GeneratorError::Budget);
        }
        mesh.validate_invariants()
            .map_err(|e| GeneratorError::Geometry(e.to_string()))?;
        mesh.deselect_all();
        Ok(mesh)
    }
}

type SourceContours = (Vec<[f64; 2]>, Vec<Vec<[f64; 2]>>, ProfileWorkplane, bool);
fn source_contours(project: &Project, id: Uuid) -> Result<SourceContours, GeneratorError> {
    let shape = project
        .get_profile(id)
        .ok_or(GeneratorError::MissingSource(id))?;
    let spline = project
        .get_spline(shape.spline_id)
        .ok_or(GeneratorError::MissingSource(shape.spline_id))?;
    shape
        .validate_authoring(spline)
        .map_err(|e| GeneratorError::Geometry(e.to_string()))?;
    let plane = shape
        .workplane
        .try_normalized()
        .map_err(|e| GeneratorError::Geometry(e.to_string()))?;
    let table = spline
        .arc_length_table(CURVE_TOLERANCE)
        .map_err(|e| GeneratorError::Geometry(e.to_string()))?;
    let mut outer: Vec<_> = table.polyline().into_iter().map(|p| [p[0], p[1]]).collect();
    clean_contour(&mut outer, spline.closed);
    let holes = project
        .shape_hole_contours(shape, CURVE_TOLERANCE)
        .map_err(|e| GeneratorError::Geometry(e.to_string()))?;
    let count = holes
        .iter()
        .try_fold(outer.len(), |n, ring| n.checked_add(ring.len()))
        .ok_or(GeneratorError::Budget)?;
    if count > MAX_GENERATOR_VERTICES / 4 {
        return Err(GeneratorError::Budget);
    }
    if spline.closed {
        validate_closed_contours(&outer, &holes)?;
    }
    Ok((outer, holes, plane, spline.closed))
}
fn map_to_plane(mesh: &mut Mesh, plane: ProfileWorkplane, map: impl Fn(Vec3) -> Vec3) {
    let origin = Vec3::from_array(plane.origin.map(|v| v as f32));
    let right = Vec3::from_array(plane.right.map(|v| v as f32));
    let up = Vec3::from_array(plane.up.map(|v| v as f32));
    let normal = Vec3::from_array(plane.normal.map(|v| v as f32));
    for vertex in &mut mesh.verts {
        let p = map(vertex.vec());
        vertex.pos = (origin + right * p.x + up * p.y + normal * p.z).to_array();
    }
}
impl Project {
    /// Resolve all contours through the same persistent spline representation.
    pub fn shape_hole_contours(
        &self,
        shape: &crate::PlanarShape,
        tolerance: f64,
    ) -> Result<Vec<Vec<[f64; 2]>>, crate::ProfileError> {
        let mut contours =
            Vec::with_capacity(shape.hole_splines.len().saturating_add(shape.holes.len()));
        for id in &shape.hole_splines {
            let spline = self
                .get_spline(*id)
                .ok_or(crate::ProfileError::SplineNotFound(*id))?;
            spline
                .validate_authoring()
                .map_err(|_| crate::ProfileError::InvalidSpline)?;
            if !spline.closed
                || spline.points.len() < 3
                || spline
                    .points
                    .iter()
                    .any(|p| p.position[2].abs() > 1.0e-8 || p.attachment.is_some())
            {
                return Err(crate::ProfileError::InvalidSpline);
            }
            let table = spline
                .arc_length_table(tolerance)
                .map_err(|_| crate::ProfileError::InvalidSpline)?;
            contours.push(table.polyline().into_iter().map(|p| [p[0], p[1]]).collect());
        }
        // Only the legacy decoder and legacy commands may supply raw contours.
        contours.extend(shape.holes.iter().cloned());
        for contour in &mut contours {
            clean_contour(contour, true);
        }
        Ok(contours)
    }

    /// Version-compatible migration at load/command boundaries. Raw rings and
    /// thickness become explicit editable spline contours, exactly once.
    pub fn promote_planar_contours(&mut self) -> Result<(), crate::ProfileError> {
        let mut replacements = Vec::new();
        for shape in &self.profiles {
            let mut rings = shape.holes.clone();
            if shape.wall_thickness > 0.0 {
                let spline = self
                    .get_spline(shape.spline_id)
                    .ok_or(crate::ProfileError::SplineNotFound(shape.spline_id))?;
                if !spline.closed {
                    continue;
                }
                let table = spline
                    .arc_length_table(CURVE_TOLERANCE)
                    .map_err(|_| crate::ProfileError::InvalidSpline)?;
                let points: Vec<_> = table
                    .polyline()
                    .into_iter()
                    .map(|p| [p[0] as f32, p[1] as f32])
                    .collect();
                let inner =
                    petunia_mesh::curve::offset_polygon(&points, -(shape.wall_thickness as f32));
                if inner.len() < 3 {
                    return Err(crate::ProfileError::InvalidSpline);
                }
                rings.push(inner.into_iter().map(|p| p.map(f64::from)).collect());
            }
            if rings.is_empty() {
                continue;
            }
            let mut splines = Vec::new();
            for ring in rings {
                if ring.len() < 3
                    || ring.len() > 4096
                    || ring.iter().flatten().any(|v| !v.is_finite())
                {
                    return Err(crate::ProfileError::InvalidSpline);
                }
                let points: Vec<_> = ring.into_iter().map(|p| [p[0], p[1], 0.0]).collect();
                splines.push(crate::SplineResource::from_polyline(
                    format!("{} · inner contour", shape.name),
                    &points,
                    true,
                ));
            }
            replacements.push((shape.id, splines));
        }
        for (shape_id, splines) in replacements {
            let ids: Vec<_> = splines.iter().map(|s| s.id).collect();
            self.splines.extend(splines);
            if let Some(shape) = self.get_profile_mut(shape_id) {
                shape.hole_splines.extend(ids);
                shape.holes.clear();
                shape.wall_thickness = 0.0;
                shape.primitive = None;
                shape.revision = shape.revision.wrapping_add(1);
            }
        }
        Ok(())
    }

    /// Re-evaluate each dependent object only when its authoring revisions change.
    /// A failed generator remains present with an empty cache and diagnostic.
    pub fn refresh_generated_geometry(&mut self) -> bool {
        let mut results = Vec::new();
        for asset in &self.assets {
            let Some(generator) = &asset.geometry_source else {
                continue;
            };
            let mut key = vec![generator.revision];
            for shape in self
                .profiles
                .iter()
                .filter(|s| generator.depends_on_shape(s.id))
            {
                key.extend([
                    shape.revision,
                    self.get_spline(shape.spline_id)
                        .map_or(u64::MAX, |s| s.revision),
                ]);
                for id in &shape.hole_splines {
                    key.push(self.get_spline(*id).map_or(u64::MAX, |s| s.revision));
                }
            }
            for spline in self
                .splines
                .iter()
                .filter(|s| generator.depends_on_spline(s.id))
            {
                key.push(spline.revision);
                for owner in self
                    .profiles
                    .iter()
                    .filter(|p| p.spline_id == spline.id || p.hole_splines.contains(&spline.id))
                {
                    key.push(owner.revision);
                }
            }
            if asset.generated_key.as_ref() == Some(&key) {
                continue;
            }
            results.push((asset.id, key, generator.evaluate(self)));
        }
        let changed = !results.is_empty();
        for (id, key, result) in results {
            if let Some(asset) = self.assets.iter_mut().find(|a| a.id == id) {
                asset.mesh = Mesh::default().into();
                asset.generated_key = Some(key);
                asset.eval_cache = None;
                match result {
                    Ok(mesh) => {
                        asset.generated_mesh = mesh.into();
                        asset.generator_diagnostic = None;
                    }
                    Err(error) => {
                        asset.generated_mesh = Mesh::default().into();
                        asset.generator_diagnostic = Some(error.to_string());
                    }
                }
            }
        }
        changed
    }
}

fn compound_sweep(
    outer: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
    path: &[Vec3],
    closed_path: bool,
    cap_start: bool,
    cap_end: bool,
) -> Result<Mesh, GeneratorError> {
    use std::collections::HashMap;
    let contours: Vec<_> = std::iter::once(outer)
        .chain(holes.iter().map(Vec::as_slice))
        .collect();
    let total = contours
        .iter()
        .try_fold(0usize, |n, c| n.checked_add(c.len()))
        .ok_or(GeneratorError::Budget)?;
    if total
        .checked_mul(path.len())
        .is_none_or(|n| n > MAX_GENERATOR_VERTICES)
    {
        return Err(GeneratorError::Budget);
    }
    let mut mesh = Mesh::default();
    let mut start_map = HashMap::new();
    let mut end_map = HashMap::new();
    let key = |p: [f32; 2]| p.map(|v| if v == 0.0 { 0 } else { v.to_bits() });
    for (ring, contour) in contours.iter().enumerate() {
        let mut points: Vec<_> = contour.iter().map(|p| p.map(|v| v as f32)).collect();
        if petunia_mesh::arrangement::signed_area(contour) < 0.0 {
            points.reverse();
        }
        let mut surface = Mesh::from_sweep(
            &points,
            path,
            SweepOptions {
                closed_path,
                closed_profile: true,
                cap_start: false,
                cap_end: false,
                miter: true,
                miter_limit: 3.0,
            },
        )
        .map_err(GeneratorError::Geometry)?;
        if ring > 0 {
            for face in &mut surface.faces {
                face.reverse_winding();
            }
        }
        let offset = mesh.verts.len();
        for (index, point) in points.iter().enumerate() {
            start_map.insert(
                key(*point),
                u32::try_from(offset.saturating_add(index)).map_err(|_| GeneratorError::Budget)?,
            );
            let last = path
                .len()
                .saturating_sub(1)
                .checked_mul(points.len())
                .and_then(|n| n.checked_add(offset))
                .and_then(|n| n.checked_add(index))
                .ok_or(GeneratorError::Budget)?;
            end_map.insert(
                key(*point),
                u32::try_from(last).map_err(|_| GeneratorError::Budget)?,
            );
        }
        mesh.join(&surface);
    }
    if !closed_path && (cap_start || cap_end) {
        let caps = petunia_mesh::imprint::region_sheet(outer, holes, Vec3::ZERO, Vec3::X, Vec3::Y)
            .map_err(|e| GeneratorError::Geometry(e.to_string()))?;
        for source in &caps.faces {
            let is_end = source.selected;
            if (is_end && !cap_end) || (!is_end && !cap_start) {
                continue;
            }
            let map = if is_end { &end_map } else { &start_map };
            let mut face = source.clone();
            for id in &mut face.verts {
                let vertex = caps
                    .verts
                    .get(*id as usize)
                    .ok_or(GeneratorError::Invalid("cap vertex"))?;
                *id = *map
                    .get(&key([vertex.pos[0], vertex.pos[1]]))
                    .ok_or(GeneratorError::Invalid("cap boundary does not match sweep"))?;
            }
            mesh.faces.push(face);
        }
    }
    Ok(mesh)
}

fn clean_contour(points: &mut Vec<[f64; 2]>, closed: bool) {
    points.dedup_by(|a, b| *a == *b);
    if closed
        && points
            .first()
            .zip(points.last())
            .is_some_and(|(a, b)| a == b)
    {
        points.pop();
    }
}

pub fn validate_primitive_descriptor(
    descriptor: PrimitiveDescriptor,
) -> Result<(), GeneratorError> {
    let dimension = |v: f32| v.is_finite() && (0.05..=100.0).contains(&v);
    let valid = match descriptor {
        PrimitiveDescriptor::Box {
            width,
            height,
            depth,
        }
        | PrimitiveDescriptor::Wedge {
            width,
            height,
            depth,
        } => [width, height, depth].into_iter().all(dimension),
        PrimitiveDescriptor::Plane { width, height } => dimension(width) && dimension(height),
        PrimitiveDescriptor::Cylinder {
            radius,
            height,
            sides,
            ..
        } => dimension(radius) && dimension(height) && (3..=32).contains(&sides),
        PrimitiveDescriptor::Cone {
            bottom_radius,
            top_radius,
            height,
            sides,
            ..
        } => {
            (0.0..=100.0).contains(&bottom_radius)
                && (0.0..=100.0).contains(&top_radius)
                && bottom_radius.max(top_radius) > 0.0
                && dimension(height)
                && (3..=32).contains(&sides)
        }
        PrimitiveDescriptor::Circle {
            radius, vertices, ..
        } => dimension(radius) && (3..=64).contains(&vertices),
        PrimitiveDescriptor::Torus {
            major_radius,
            minor_radius,
            major_segments,
            minor_segments,
        } => {
            dimension(major_radius)
                && (0.01..=100.0).contains(&minor_radius)
                && minor_radius <= major_radius * 0.99
                && (3..=64).contains(&major_segments)
                && (3..=32).contains(&minor_segments)
        }
        PrimitiveDescriptor::LowSphere {
            radius,
            segments,
            rings,
        } => dimension(radius) && (3..=32).contains(&segments) && (2..=24).contains(&rings),
        PrimitiveDescriptor::Icosphere { radius, subdiv } => dimension(radius) && subdiv <= 3,
        PrimitiveDescriptor::Capsule {
            radius,
            height,
            radial_segments,
            cap_segments,
        } => {
            dimension(radius)
                && dimension(height)
                && height >= radius * 2.0
                && (3..=32).contains(&radial_segments)
                && (1..=8).contains(&cap_segments)
        }
    };
    if valid {
        Ok(())
    } else {
        Err(GeneratorError::Invalid(
            "primitive parameter is outside its supported range",
        ))
    }
}

fn validate_closed_contours(
    outer: &[[f64; 2]],
    holes: &[Vec<[f64; 2]>],
) -> Result<(), GeneratorError> {
    let rings: Vec<_> = std::iter::once(outer)
        .chain(holes.iter().map(Vec::as_slice))
        .collect();
    let mut checks = 0usize;
    let edges = |ring: &[[f64; 2]]| {
        ring.iter()
            .copied()
            .zip(ring.iter().copied().cycle().skip(1))
            .take(ring.len())
            .collect::<Vec<_>>()
    };
    let intersects = |a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]| {
        let cross = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| {
            (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
        };
        if a[0].max(b[0]) < c[0].min(d[0])
            || c[0].max(d[0]) < a[0].min(b[0])
            || a[1].max(b[1]) < c[1].min(d[1])
            || c[1].max(d[1]) < a[1].min(b[1])
        {
            return false;
        }
        let (ab_c, ab_d, cd_a, cd_b) = (
            cross(a, b, c),
            cross(a, b, d),
            cross(c, d, a),
            cross(c, d, b),
        );
        ab_c * ab_d <= 0.0 && cd_a * cd_b <= 0.0
    };
    for (index, ring) in rings.iter().enumerate() {
        if ring.len() < 3 || petunia_mesh::arrangement::signed_area(ring).abs() < 1.0e-10 {
            return Err(GeneratorError::Invalid("degenerate contour"));
        }
        let segments = edges(ring);
        for (i, (a, b)) in segments.iter().copied().enumerate() {
            for (j, (c, d)) in segments.iter().copied().enumerate().skip(i + 1) {
                if j == i + 1 || (i == 0 && j + 1 == segments.len()) {
                    continue;
                }
                checks = checks.saturating_add(1);
                if checks > 1_000_000 {
                    return Err(GeneratorError::Budget);
                }
                if intersects(a, b, c, d) {
                    return Err(GeneratorError::Invalid("contour self-intersects"));
                }
            }
        }
        for other in rings.iter().take(index) {
            for (a, b) in &segments {
                for (c, d) in edges(other) {
                    checks = checks.saturating_add(1);
                    if checks > 1_000_000 {
                        return Err(GeneratorError::Budget);
                    }
                    if intersects(*a, *b, c, d) {
                        return Err(GeneratorError::Invalid("contours intersect or touch"));
                    }
                }
            }
        }
        if index > 0
            && ring
                .iter()
                .any(|p| !petunia_mesh::arrangement::point_in_polygon(outer, *p))
        {
            return Err(GeneratorError::Invalid(
                "inner contour is outside the shape",
            ));
        }
        if index > 0
            && rings.iter().skip(1).take(index - 1).any(|other| {
                ring.first()
                    .is_some_and(|p| petunia_mesh::arrangement::point_in_polygon(other, *p))
                    || other
                        .first()
                        .is_some_and(|p| petunia_mesh::arrangement::point_in_polygon(ring, *p))
            })
        {
            return Err(GeneratorError::Invalid("inner contours overlap"));
        }
    }
    Ok(())
}
