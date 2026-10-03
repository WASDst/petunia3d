//! Perfis planares persistentes para modelagem shape-first (P3D-160/P3D-168).

use glam::DVec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::spline::SplineResource;

const AXIS_EPSILON_SQUARED: f64 = 1.0e-18;
const NORMAL_ALIGNMENT_MINIMUM: f64 = 0.5;
const PLANAR_EPSILON: f64 = 1.0e-8;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ProfileWorkplane {
    pub origin: [f64; 3],
    pub right: [f64; 3],
    pub up: [f64; 3],
    pub normal: [f64; 3],
}

impl Default for ProfileWorkplane {
    fn default() -> Self {
        Self {
            origin: [0.0, 0.0, 0.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            normal: [0.0, 0.0, 1.0],
        }
    }
}

impl ProfileWorkplane {
    pub fn try_normalized(self) -> Result<Self, ProfileError> {
        let origin = finite_vec(self.origin)?;
        let right = finite_vec(self.right)?;
        let input_up = finite_vec(self.up)?;
        let input_normal = finite_vec(self.normal)?;
        if right.length_squared() <= AXIS_EPSILON_SQUARED
            || input_up.length_squared() <= AXIS_EPSILON_SQUARED
            || input_normal.length_squared() <= AXIS_EPSILON_SQUARED
        {
            return Err(ProfileError::InvalidWorkplane);
        }

        let right = right.normalize();
        let up = input_up - right * input_up.dot(right);
        if up.length_squared() <= AXIS_EPSILON_SQUARED {
            return Err(ProfileError::InvalidWorkplane);
        }
        let mut up = up.normalize();
        let mut normal = right.cross(up).normalize();
        let input_normal = input_normal.normalize();
        if normal.dot(input_normal).abs() < NORMAL_ALIGNMENT_MINIMUM {
            return Err(ProfileError::InvalidWorkplane);
        }
        if normal.dot(input_normal) < 0.0 {
            up = -up;
            normal = -normal;
        }

        Ok(Self {
            origin: origin.to_array(),
            right: right.to_array(),
            up: up.to_array(),
            normal: normal.to_array(),
        })
    }

    pub fn to_world(self, point: [f64; 2]) -> Result<[f64; 3], ProfileError> {
        if point.iter().any(|component| !component.is_finite()) {
            return Err(ProfileError::NonFinite);
        }
        let normalized = self.try_normalized()?;
        let origin = DVec3::from_array(normalized.origin);
        let right = DVec3::from_array(normalized.right);
        let up = DVec3::from_array(normalized.up);
        Ok((origin + right * point[0] + up * point[1]).to_array())
    }

    /// Cria um plano de trabalho passando por 3 pontos no espaço 3D (origem = p0, eixo X em direção a p1, p2 define o semiplano positivo de Y).
    pub fn from_three_points(
        p0: [f64; 3],
        p1: [f64; 3],
        p2: [f64; 3],
    ) -> Result<Self, ProfileError> {
        let v0 = finite_vec(p0)?;
        let v1 = finite_vec(p1)?;
        let v2 = finite_vec(p2)?;
        let v01 = v1 - v0;
        let v02 = v2 - v0;
        let normal = v01.cross(v02);
        if normal.length_squared() <= AXIS_EPSILON_SQUARED
            || v01.length_squared() <= AXIS_EPSILON_SQUARED
        {
            return Err(ProfileError::InvalidWorkplane);
        }
        let normal = normal.normalize();
        let right = v01.normalize();
        let up = normal.cross(right).normalize();
        Ok(Self {
            origin: v0.to_array(),
            right: right.to_array(),
            up: up.to_array(),
            normal: normal.to_array(),
        })
    }

    /// Cria um plano a partir de uma face (centro e normal) alinhado com uma aresta de referência (direção X).
    pub fn from_face_and_edge(
        center: [f64; 3],
        normal: [f64; 3],
        edge_dir: [f64; 3],
    ) -> Result<Self, ProfileError> {
        let origin = finite_vec(center)?;
        let norm = finite_vec(normal)?;
        let edge = finite_vec(edge_dir)?;
        if norm.length_squared() <= AXIS_EPSILON_SQUARED
            || edge.length_squared() <= AXIS_EPSILON_SQUARED
        {
            return Err(ProfileError::InvalidWorkplane);
        }
        let norm = norm.normalize();
        // Projeta edge no plano da normal
        let right = edge - norm * edge.dot(norm);
        if right.length_squared() <= AXIS_EPSILON_SQUARED {
            return Err(ProfileError::InvalidWorkplane);
        }
        let right = right.normalize();
        let up = norm.cross(right).normalize();
        Ok(Self {
            origin: origin.to_array(),
            right: right.to_array(),
            up: up.to_array(),
            normal: norm.to_array(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ProfilePrimitive {
    Rectangle,
    Ellipse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileResource {
    pub id: Uuid,
    pub name: String,
    pub spline_id: Uuid,
    pub workplane: ProfileWorkplane,
    pub wall_thickness: f64,
    pub revision: u64,
    /// Contornos internos poligonais no mesmo plano; não são formas preenchidas.
    pub holes: Vec<Vec<[f64; 2]>>,
    pub primitive: Option<ProfilePrimitive>,
}

// Preserve the pre-compound layout when reading legacy postcard. New compound
// and parametric shapes use canonical ZIP/JSON; never silently discard authoring data.
impl Serialize for ProfileResource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let human = serializer.is_human_readable();
        if !human && (!self.holes.is_empty() || self.primitive.is_some()) {
            return Err(serde::ser::Error::custom(
                "Compound or parametric profiles require ZIP/JSON",
            ));
        }
        let mut value =
            serializer.serialize_struct("ProfileResource", if human { 8 } else { 6 })?;
        value.serialize_field("id", &self.id)?;
        value.serialize_field("name", &self.name)?;
        value.serialize_field("spline_id", &self.spline_id)?;
        value.serialize_field("workplane", &self.workplane)?;
        value.serialize_field("wall_thickness", &self.wall_thickness)?;
        value.serialize_field("revision", &self.revision)?;
        if human {
            value.serialize_field("holes", &self.holes)?;
            value.serialize_field("primitive", &self.primitive)?;
        }
        value.end()
    }
}
impl<'de> Deserialize<'de> for ProfileResource {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Old {
            id: Uuid,
            name: String,
            spline_id: Uuid,
            workplane: ProfileWorkplane,
            wall_thickness: f64,
            revision: u64,
        }
        #[derive(Deserialize)]
        struct Current {
            id: Uuid,
            name: String,
            spline_id: Uuid,
            workplane: ProfileWorkplane,
            #[serde(default)]
            wall_thickness: f64,
            #[serde(default)]
            revision: u64,
            #[serde(default)]
            holes: Vec<Vec<[f64; 2]>>,
            #[serde(default)]
            primitive: Option<ProfilePrimitive>,
        }
        if deserializer.is_human_readable() {
            let c = Current::deserialize(deserializer)?;
            Ok(Self {
                id: c.id,
                name: c.name,
                spline_id: c.spline_id,
                workplane: c.workplane,
                wall_thickness: c.wall_thickness,
                revision: c.revision,
                holes: c.holes,
                primitive: c.primitive,
            })
        } else {
            let c = Old::deserialize(deserializer)?;
            Ok(Self {
                id: c.id,
                name: c.name,
                spline_id: c.spline_id,
                workplane: c.workplane,
                wall_thickness: c.wall_thickness,
                revision: c.revision,
                holes: Vec::new(),
                primitive: None,
            })
        }
    }
}

impl ProfileResource {
    pub fn new(name: impl Into<String>, spline_id: Uuid, workplane: ProfileWorkplane) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            spline_id,
            workplane,
            wall_thickness: 0.0,
            revision: 0,
            holes: Vec::new(),
            primitive: None,
        }
    }

    /// Recover the affine primitive frame only while its authored points still
    /// match the generator. Node edits must never be overwritten by reediting.
    pub fn primitive_frame(&self, spline: &SplineResource) -> Option<(DVec3, DVec3, DVec3)> {
        if !spline.closed || !self.holes.is_empty() || spline.points.len() < 4 {
            return None;
        }
        let points: Vec<_> = spline
            .points
            .iter()
            .map(|p| DVec3::from_array(p.position))
            .collect();
        let center = points.iter().copied().sum::<DVec3>() / points.len() as f64;
        let (x, y) = match self.primitive? {
            ProfilePrimitive::Rectangle if points.len() == 4 => {
                ((points[1] - points[0]) * 0.5, (points[3] - points[0]) * 0.5)
            }
            ProfilePrimitive::Rectangle => return None,
            ProfilePrimitive::Ellipse => {
                let mut x = DVec3::ZERO;
                let mut y = DVec3::ZERO;
                for (i, p) in points.iter().enumerate() {
                    let angle = std::f64::consts::TAU * i as f64 / points.len() as f64;
                    x += (*p - center) * angle.cos();
                    y += (*p - center) * angle.sin();
                }
                (
                    x * (2.0 / points.len() as f64),
                    y * (2.0 / points.len() as f64),
                )
            }
        };
        if !x.is_finite() || !y.is_finite() || x.cross(y).length_squared() < 1e-18 {
            return None;
        }
        let tolerance = 1e-8 * (1.0 + x.length() + y.length());
        for (i, p) in points.iter().enumerate() {
            let (a, b) = match self.primitive? {
                ProfilePrimitive::Rectangle => {
                    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)][i]
                }
                ProfilePrimitive::Ellipse => {
                    let angle = std::f64::consts::TAU * i as f64 / points.len() as f64;
                    (angle.cos(), angle.sin())
                }
            };
            if p.distance(center + x * a + y * b) > tolerance
                || spline.points[i].handle_in != [0.0; 3]
                || spline.points[i].handle_out != [0.0; 3]
            {
                return None;
            }
        }
        Some((center, x, y))
    }

    pub fn resize_primitive(
        &self,
        spline: &SplineResource,
        width: f64,
        height: f64,
        segments: usize,
    ) -> Option<SplineResource> {
        if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
            return None;
        }
        let (center, x, y) = self.primitive_frame(spline)?;
        let x = x.normalize() * width * 0.5;
        let y = y.normalize() * height * 0.5;
        let count = match self.primitive? {
            ProfilePrimitive::Rectangle => 4,
            ProfilePrimitive::Ellipse => segments.clamp(6, 512),
        };
        let kind = self.primitive?;
        let mut next = spline.clone();
        next.points = (0..count)
            .map(|i| {
                let (a, b) = match kind {
                    ProfilePrimitive::Rectangle => {
                        [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)][i]
                    }
                    ProfilePrimitive::Ellipse => {
                        let angle = std::f64::consts::TAU * i as f64 / count as f64;
                        (angle.cos(), angle.sin())
                    }
                };
                let mut point =
                    crate::spline::SplinePoint::new((center + x * a + y * b).to_array());
                if count == spline.points.len() {
                    point.id = spline.points[i].id;
                }
                point
            })
            .collect();
        Some(next)
    }

    pub fn validate_authoring(&self, spline: &SplineResource) -> Result<(), ProfileError> {
        if self.name.trim().is_empty() {
            return Err(ProfileError::EmptyName);
        }
        if spline.id != self.spline_id {
            return Err(ProfileError::SplineMismatch {
                expected: self.spline_id,
                actual: spline.id,
            });
        }
        spline
            .validate_authoring()
            .map_err(|_| ProfileError::InvalidSpline)?;
        self.workplane.try_normalized()?;
        if !self.wall_thickness.is_finite() || self.wall_thickness < 0.0 {
            return Err(ProfileError::InvalidWallThickness);
        }
        if self
            .holes
            .iter()
            .any(|hole| hole.len() < 3 || hole.iter().flatten().any(|v| !v.is_finite()))
        {
            return Err(ProfileError::InvalidSpline);
        }
        for point in &spline.points {
            if point.attachment.is_some() {
                return Err(ProfileError::SurfaceAttachmentUnsupported(point.id));
            }
            if point.position[2].abs() > PLANAR_EPSILON
                || point.handle_in[2].abs() > PLANAR_EPSILON
                || point.handle_out[2].abs() > PLANAR_EPSILON
            {
                return Err(ProfileError::NonPlanarPoint(point.id));
            }
        }
        Ok(())
    }

    pub fn validate_loaded(&mut self) {
        if self.name.trim().is_empty() {
            self.name = "Profile".to_string();
        }
        self.holes
            .retain(|hole| hole.len() >= 3 && hole.iter().flatten().all(|v| v.is_finite()));
        self.workplane = self.workplane.try_normalized().unwrap_or_default();
        if !self.wall_thickness.is_finite() || self.wall_thickness < 0.0 {
            self.wall_thickness = 0.0;
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
        mix(self.id.as_u128() as u64);
        mix((self.id.as_u128() >> 64) as u64);
        mix(self.spline_id.as_u128() as u64);
        mix((self.spline_id.as_u128() >> 64) as u64);
        mix(self.revision);
        for component in self
            .workplane
            .origin
            .iter()
            .chain(&self.workplane.right)
            .chain(&self.workplane.up)
            .chain(&self.workplane.normal)
        {
            mix(component.to_bits());
        }
        mix(self.wall_thickness.to_bits());
        mix(self.holes.len() as u64);
        for hole in &self.holes {
            mix(hole.len() as u64);
            for value in hole.iter().flatten() {
                mix(value.to_bits());
            }
        }
        fingerprint
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProfileError {
    #[error("profile id is duplicated: {0}")]
    DuplicateProfile(Uuid),
    #[error("profile was not found: {0}")]
    ProfileNotFound(Uuid),
    #[error("profile spline was not found: {0}")]
    SplineNotFound(Uuid),
    #[error("profile is used by path generator: {0}")]
    ProfileInUse(Uuid),
    #[error("profile spline is already owned by profile: {0}")]
    SplineAlreadyOwned(Uuid),
    #[error("profile name cannot be empty")]
    EmptyName,
    #[error("profile workplane is invalid")]
    InvalidWorkplane,
    #[error("profile data must be finite")]
    NonFinite,
    #[error("profile wall thickness must be non-negative and finite")]
    InvalidWallThickness,
    #[error("profile spline reference mismatch: expected {expected}, got {actual}")]
    SplineMismatch { expected: Uuid, actual: Uuid },
    #[error("profile spline data is invalid")]
    InvalidSpline,
    #[error("profile spline must be closed")]
    SplineMustBeClosed,
    #[error("profile needs at least three points")]
    NeedsThreePoints,
    #[error("profile point is not planar: {0}")]
    NonPlanarPoint(Uuid),
    #[error("profile points cannot use surface attachment: {0}")]
    SurfaceAttachmentUnsupported(Uuid),
}

fn finite_vec(value: [f64; 3]) -> Result<DVec3, ProfileError> {
    let value = DVec3::from_array(value);
    value
        .is_finite()
        .then_some(value)
        .ok_or(ProfileError::NonFinite)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SplineResource;

    #[test]
    fn legacy_profile_layout_does_not_consume_the_next_record() {
        let profile = ProfileResource::new("Square", Uuid::new_v4(), ProfileWorkplane::default());
        let record = (&profile, 1234u32);
        let bytes = postcard::to_allocvec(&record).unwrap();
        let (back, tail): (ProfileResource, u32) = postcard::from_bytes(&bytes).unwrap();
        assert_eq!(back, profile);
        assert_eq!(tail, 1234);
        let mut compound = profile;
        compound
            .holes
            .push(vec![[0.0, 0.0], [0.1, 0.0], [0.0, 0.1]]);
        assert!(postcard::to_allocvec(&compound).is_err());
    }

    #[test]
    fn workplane_normalizes_axes_and_preserves_handedness() {
        let workplane = ProfileWorkplane {
            origin: [1.0, 2.0, 3.0],
            right: [2.0, 0.0, 0.0],
            up: [0.2, 4.0, 0.0],
            normal: [0.0, 0.0, 5.0],
        }
        .try_normalized()
        .unwrap();

        assert_eq!(workplane.right, [1.0, 0.0, 0.0]);
        assert_eq!(workplane.up, [0.0, 1.0, 0.0]);
        assert_eq!(workplane.normal, [0.0, 0.0, 1.0]);
        assert_eq!(workplane.to_world([2.0, -1.0]).unwrap(), [3.0, 1.0, 3.0]);
    }

    #[test]
    fn profile_accepts_open_authoring_drafts_but_requires_planar_unattached_points() {
        let draft = SplineResource::new("Draft curve", crate::SplineInterpolation::CubicBezier);
        let draft_profile = ProfileResource::new("Draft", draft.id, ProfileWorkplane::default());
        assert!(draft_profile.validate_authoring(&draft).is_ok());

        let mut spline = SplineResource::from_polyline(
            "Profile curve",
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            true,
        );
        let profile = ProfileResource::new("Triangle", spline.id, ProfileWorkplane::default());
        assert!(profile.validate_authoring(&spline).is_ok());

        spline.points[0].position[2] = 0.1;
        assert!(matches!(
            profile.validate_authoring(&spline),
            Err(ProfileError::NonPlanarPoint(_))
        ));
    }

    #[test]
    fn workplane_from_three_points_and_face_edge() {
        let p0 = [0.0, 0.0, 0.0];
        let p1 = [2.0, 0.0, 0.0];
        let p2 = [0.0, 3.0, 0.0];
        let wp = ProfileWorkplane::from_three_points(p0, p1, p2).unwrap();
        assert_eq!(wp.origin, [0.0, 0.0, 0.0]);
        assert_eq!(wp.right, [1.0, 0.0, 0.0]);
        assert_eq!(wp.up, [0.0, 1.0, 0.0]);
        assert_eq!(wp.normal, [0.0, 0.0, 1.0]);

        let wp2 =
            ProfileWorkplane::from_face_and_edge([1.0, 1.0, 1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0])
                .unwrap();
        assert_eq!(wp2.origin, [1.0, 1.0, 1.0]);
        assert_eq!(wp2.normal, [0.0, 1.0, 0.0]);
        assert_eq!(wp2.right, [1.0, 0.0, 0.0]);
        assert_eq!(wp2.up, [0.0, 0.0, -1.0]);
    }
}

#[cfg(test)]
mod compound_tests {
    use super::*;
    #[test]
    fn holes_survive_json_and_old_profiles_default_to_no_holes() {
        let mut profile = ProfileResource::new("Ring", Uuid::new_v4(), ProfileWorkplane::default());
        profile.holes = vec![vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]];
        let value = serde_json::to_value(&profile).unwrap();
        let loaded: ProfileResource = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(loaded.holes, profile.holes);
        let mut old = value;
        old.as_object_mut().unwrap().remove("holes");
        let loaded: ProfileResource = serde_json::from_value(old).unwrap();
        assert!(loaded.holes.is_empty());
    }
}

#[cfg(test)]
mod primitive_tests {
    use super::*;
    #[test]
    fn dimensions_preserve_transformed_frame_and_legacy_data_is_explicit() {
        let spline = SplineResource::from_polyline(
            "rectangle",
            &[
                [3.0, 1.0, 0.0],
                [3.0, 3.0, 0.0],
                [2.0, 3.0, 0.0],
                [2.0, 1.0, 0.0],
            ],
            true,
        );
        let mut profile = ProfileResource::new("rectangle", spline.id, ProfileWorkplane::default());
        profile.primitive = Some(ProfilePrimitive::Rectangle);
        let next = profile.resize_primitive(&spline, 4.0, 3.0, 16).unwrap();
        let (center, x, y) = profile.primitive_frame(&next).unwrap();
        assert!((center - DVec3::new(2.5, 2.0, 0.0)).length() < 1e-10);
        assert!((x - DVec3::new(0.0, 2.0, 0.0)).length() < 1e-10);
        assert!((y - DVec3::new(-1.5, 0.0, 0.0)).length() < 1e-10);
        assert_eq!(next.points[0].id, spline.points[0].id);
        assert!(postcard::to_allocvec(&profile).is_err());
        let mut project = crate::Project::new();
        project.add_spline(next.clone()).unwrap();
        project.add_profile(profile.clone()).unwrap();
        let bytes = crate::format::encode_zip(&project).unwrap();
        let loaded = crate::format::load_bytes(&bytes).unwrap();
        assert_eq!(
            loaded.profiles[0].primitive,
            Some(ProfilePrimitive::Rectangle)
        );
        assert!(
            loaded.profiles[0]
                .resize_primitive(&loaded.splines[0], 5.0, 2.0, 16)
                .is_some()
        );
        let mut value = serde_json::to_value(&profile).unwrap();
        value.as_object_mut().unwrap().remove("primitive");
        let old: ProfileResource = serde_json::from_value(value).unwrap();
        assert!(old.primitive.is_none());
    }
}
