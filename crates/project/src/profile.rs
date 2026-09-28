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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProfileResource {
    pub id: Uuid,
    pub name: String,
    pub spline_id: Uuid,
    pub workplane: ProfileWorkplane,
    #[serde(default)]
    pub wall_thickness: f64,
    #[serde(default)]
    pub revision: u64,
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
        }
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
}
