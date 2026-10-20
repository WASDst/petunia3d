//! Rebase an object's local origin without moving its authored surface in world space.
use crate::{Asset, ObjectTransform};
use glam::{EulerRot, Vec3};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OriginError {
    #[error("Asset is locked")]
    Locked,
    #[error("Asset mesh is empty")]
    Empty,
    #[error("Origin or object frame is non-finite, singular or loses surface precision")]
    InvalidFrame,
    #[error("Origin rebase requires Make Editable for a parametric primitive")]
    Parametric,
    #[error("Origin rebase requires explicitly applying or removing modifiers first")]
    Modifiers,
    #[error("Origin rebase of a rigged object requires bind compensation, not yet supported")]
    Rigged,
}

impl Asset {
    /// Compatibility classification, not a second persistent transform. Older
    /// Object Mode wrote vertices and accumulated these fields without a TRS.
    pub fn has_legacy_baked_transform(&self) -> bool {
        self.origin.is_some()
            || (self.transform.is_identity()
                && (self.position != [0.0; 3]
                    || self.rotation != [0.0; 3]
                    || self.scale != [1.0; 3]))
    }

    /// Translate an entire object frame without touching its authored sources.
    pub fn translate_world(
        &mut self,
        delta: Vec3,
    ) -> Result<(), crate::object_transform::TransformError> {
        use crate::object_transform::TransformError;
        let mut transform = self.transform;
        transform.translation = (transform.translation() + delta).to_array();
        let matrix = transform.to_mat4();
        if !delta.is_finite()
            || !transform.is_finite()
            || !matrix.is_finite()
            || self
                .mesh
                .verts
                .iter()
                .any(|v| !v.vec().is_finite() || !matrix.transform_point3(v.vec()).is_finite())
        {
            return Err(TransformError::NonFinite);
        }
        if !matrix.inverse().is_finite() {
            return Err(TransformError::Singular);
        }
        if self.has_legacy_baked_transform() {
            let position = Vec3::from(self.position) + delta;
            let origin = self
                .origin
                .unwrap_or_else(|| self.to_local_point(self.origin_world()).to_array());
            if !position.is_finite()
                || !Vec3::from(origin).is_finite()
                || !Vec3::from(self.rotation).is_finite()
                || !Vec3::from(self.scale).is_finite()
            {
                return Err(TransformError::NonFinite);
            }
            self.transform = transform;
            self.position = position.to_array();
            self.origin = Some(origin);
        } else {
            self.set_object_transform(transform);
        }
        Ok(())
    }

    /// Place the copied origin, rather than a baked vertex centroid, at `point`.
    pub fn duplicate_at_world(
        &self,
        point: Vec3,
    ) -> Result<Self, crate::object_transform::TransformError> {
        let mut copy = self.duplicate();
        copy.translate_world(point - self.origin_world())?;
        Ok(copy)
    }

    /// New objects use local zero. The old optional offset is read only for
    /// compatibility until an explicit origin rebase clears it.
    pub fn origin_world(&self) -> Vec3 {
        let local = self.origin.map_or_else(
            || {
                if self.has_legacy_baked_transform() {
                    (self
                        .mesh
                        .verts
                        .iter()
                        .map(|v| v.vec().as_dvec3())
                        .sum::<glam::DVec3>()
                        / self.mesh.verts.len().max(1) as f64)
                        .as_vec3()
                } else {
                    Vec3::ZERO
                }
            },
            Vec3::from,
        );
        self.to_world_point(local)
    }

    pub fn validate_origin_rebase(&self) -> Result<(), OriginError> {
        if self.locked {
            return Err(OriginError::Locked);
        }
        if self.parametric.is_some() {
            return Err(OriginError::Parametric);
        }
        // Disabled modifiers also retain local planes and may later be enabled.
        if !self.modifiers.is_empty() {
            return Err(OriginError::Modifiers);
        }
        if self.skeleton_id.is_some() || self.skin_data.is_some() {
            return Err(OriginError::Rigged);
        }
        if self.mesh.verts.is_empty() {
            return Err(OriginError::Empty);
        }
        let matrix = self.world_matrix();
        if !self.transform.is_finite()
            || !matrix.is_finite()
            || matrix.determinant().abs() < 1.0e-12
            || !matrix.inverse().is_finite()
            || self
                .mesh
                .verts
                .iter()
                .any(|v| !v.vec().is_finite() || !matrix.transform_point3(v.vec()).is_finite())
        {
            return Err(OriginError::InvalidFrame);
        }
        Ok(())
    }

    /// `M' = M T(d)`, `p' = p - d`, therefore `M' p' = M p`.
    /// Validate every result before writing so an error leaves the asset intact.
    pub fn rebase_origin_world(&mut self, point: Vec3) -> Result<(), OriginError> {
        self.validate_origin_rebase()?;
        if !point.is_finite() {
            return Err(OriginError::InvalidFrame);
        }
        let source_matrix = self.world_matrix();
        let local_offset = source_matrix.inverse().transform_point3(point);
        let mut transform = self.transform;
        transform.translation = point.to_array();
        let matrix = transform.to_mat4();
        if !local_offset.is_finite()
            || self.mesh.verts.iter().any(|v| {
                let p = v.vec() - local_offset;
                let before = source_matrix.transform_point3(v.vec());
                let after = matrix.transform_point3(p);
                let tolerance = before.abs().max_element().max(1.0) * 1.0e-4;
                !p.is_finite()
                    || !after.is_finite()
                    || (after - before).abs().max_element() > tolerance
            })
        {
            return Err(OriginError::InvalidFrame);
        }
        for vertex in &mut self.mesh.verts {
            vertex.pos = (vertex.vec() - local_offset).to_array();
        }
        self.set_object_transform(transform);
        self.origin = None;
        Ok(())
    }

    /// Compatibility metadata is a projection of the authoritative TRS, never
    /// an additional transform applied to geometry.
    pub fn set_object_transform(&mut self, transform: ObjectTransform) {
        self.transform = transform;
        self.position = transform.translation;
        let (x, y, z) = transform.rotation().to_euler(EulerRot::XYZ);
        self.rotation = [x.to_degrees(), y.to_degrees(), z.to_degrees()];
        self.scale = transform.scale;
    }

    /// Set authoritative TRS and explicitly authored Euler representation in degrees.
    pub fn set_object_transform_with_euler(
        &mut self,
        transform: ObjectTransform,
        euler_deg: [f32; 3],
    ) {
        self.transform = transform;
        self.position = transform.translation;
        self.rotation = euler_deg;
        self.scale = transform.scale;
    }
}
