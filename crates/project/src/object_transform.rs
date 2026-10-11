//! Transform autoritativo do objeto (P01/G10; website/docs/03-geometry/
//! transforms-origin-pivot-gizmos.md, "Transform canônico", "TRS e shear",
//! "Negative scale").
//!
//! `world(object) = world(parent) × local(object)`; sem parent, `world =
//! local`. A geometria autoral fica no frame local do objeto e a origem do
//! objeto é o `(0,0,0)` local.
//!
//! **Fase 1 da migração:** todo objeto tem transform identidade (a geometria
//! ainda vive em mundo), mas render, picking, snapping, export e demais
//! consumidores em mundo passam a usar [`crate::Asset::world_matrix`] /
//! [`crate::Asset::world_mesh`]. Na fase 2 o Object Mode passa a editar só este
//! transform e arquivos antigos convertem `local = T⁻¹ · mundo`.

use glam::{Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};

/// Translation + Rotation + Scale (sem shear).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ObjectTransform {
    pub translation: [f32; 3],
    /// Quaternion `[x, y, z, w]`; Euler é só representação de UI.
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl Default for ObjectTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Uma matriz não pôde virar TRS sem distorcer o objeto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum TransformError {
    #[error("matrix is not finite")]
    NonFinite,
    #[error("matrix is singular (zero scale)")]
    Singular,
    #[error("matrix has shear; it cannot be represented as translation/rotation/scale")]
    Shear,
}

impl ObjectTransform {
    pub const IDENTITY: Self = Self {
        translation: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0; 3],
    };

    pub fn from_trs(translation: Vec3, rotation: Quat, scale: Vec3) -> Self {
        Self {
            translation: translation.to_array(),
            rotation: rotation.normalize().to_array(),
            scale: scale.to_array(),
        }
    }

    pub fn translation(&self) -> Vec3 {
        Vec3::from_array(self.translation)
    }

    pub fn rotation(&self) -> Quat {
        let q = Quat::from_array(self.rotation);
        if q.is_finite() && q.length_squared() > 1.0e-12 {
            q.normalize()
        } else {
            Quat::IDENTITY
        }
    }

    pub fn scale(&self) -> Vec3 {
        Vec3::from_array(self.scale)
    }

    pub fn is_identity(&self) -> bool {
        *self == Self::IDENTITY
    }

    pub fn is_finite(&self) -> bool {
        self.translation().is_finite()
            && self.scale().is_finite()
            && Quat::from_array(self.rotation).is_finite()
    }

    /// Escala negativa em número ímpar de eixos inverte o winding.
    pub fn flips_winding(&self) -> bool {
        self.scale.iter().filter(|s| **s < 0.0).count() % 2 == 1
    }

    pub fn has_uniform_scale(&self) -> bool {
        let [x, y, z] = self.scale.map(f32::abs);
        (x - y).abs() <= 1.0e-5 * x.max(1.0) && (y - z).abs() <= 1.0e-5 * y.max(1.0)
    }

    /// Matriz local → parent/mundo.
    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale(), self.rotation(), self.translation())
    }

    /// TRS exato de `matrix`, ou erro quando ela tem shear / é singular. Nunca
    /// aproxima em silêncio.
    pub fn from_mat4(matrix: Mat4) -> Result<Self, TransformError> {
        if !matrix.is_finite() {
            return Err(TransformError::NonFinite);
        }
        let det = matrix.determinant();
        if det.abs() < 1.0e-12 {
            return Err(TransformError::Singular);
        }
        let (scale, rotation, translation) = matrix.to_scale_rotation_translation();
        let candidate = Self::from_trs(translation, rotation, scale);
        let rebuilt = candidate.to_mat4();
        let tolerance = 1.0e-4
            * matrix
                .abs()
                .to_cols_array()
                .iter()
                .fold(1.0f32, |m, v| m.max(*v));
        if rebuilt.abs_diff_eq(matrix, tolerance) {
            Ok(candidate)
        } else {
            Err(TransformError::Shear)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_round_trips() {
        assert_eq!(ObjectTransform::default().to_mat4(), Mat4::IDENTITY);
        assert!(ObjectTransform::default().is_identity());
        assert_eq!(
            ObjectTransform::from_mat4(Mat4::IDENTITY).unwrap(),
            ObjectTransform::IDENTITY
        );
    }

    #[test]
    fn trs_round_trips_through_the_matrix() {
        let t = ObjectTransform::from_trs(
            Vec3::new(1.0, 2.0, 3.0),
            Quat::from_rotation_y(0.7),
            Vec3::new(2.0, 2.0, 2.0),
        );
        let back = ObjectTransform::from_mat4(t.to_mat4()).unwrap();
        assert!(back.to_mat4().abs_diff_eq(t.to_mat4(), 1e-5));
    }

    #[test]
    fn shear_and_singular_matrices_are_refused() {
        let shear = Mat4::from_cols_array(&[
            1.0, 0.0, 0.0, 0.0, //
            0.5, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0, //
            0.0, 0.0, 0.0, 1.0,
        ]);
        assert_eq!(
            ObjectTransform::from_mat4(shear),
            Err(TransformError::Shear)
        );
        assert_eq!(
            ObjectTransform::from_mat4(Mat4::from_scale(Vec3::new(1.0, 0.0, 1.0))),
            Err(TransformError::Singular)
        );
    }

    #[test]
    fn negative_scale_flags_winding_flip() {
        let mirrored =
            ObjectTransform::from_trs(Vec3::ZERO, Quat::IDENTITY, Vec3::new(-1.0, 1.0, 1.0));
        assert!(mirrored.flips_winding());
        let double =
            ObjectTransform::from_trs(Vec3::ZERO, Quat::IDENTITY, Vec3::new(-1.0, -1.0, 1.0));
        assert!(!double.flips_winding());
    }
}
