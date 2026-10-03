//! Attachment geométrico persistente compartilhado por paths e generators.

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::Project;

const BARYCENTRIC_EPSILON: f32 = 1.0e-4;
const FRAME_EPSILON_SQUARED: f32 = 1.0e-16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceTriangleHandle {
    pub face_index: u32,
    pub triangle_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SurfaceAttachment {
    pub target: Uuid,
    pub triangle: SurfaceTriangleHandle,
    pub barycentric: [f32; 3],
    pub normal_offset: f32,
    pub tangent_rotation: f32,
    pub source_topology_revision: u64,
    pub source_topology_fingerprint: u64,
    pub last_world_position: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceAttachmentStatus {
    Valid,
    NeedsReattach,
    MissingTarget,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceFrame {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
    pub uv: Option<[f32; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceHit {
    pub distance: f32,
    pub attachment: SurfaceAttachment,
    pub frame: SurfaceFrame,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SurfaceAttachmentError {
    #[error("surface attachment data is invalid")]
    InvalidData,
    #[error("surface attachment target does not exist")]
    MissingTarget,
    #[error("surface attachment target is locked")]
    TargetLocked,
    #[error("surface attachment topology no longer matches")]
    NeedsReattach,
    #[error("surface triangle is degenerate")]
    DegenerateTriangle,
    #[error("ray did not hit the requested surface")]
    NoSurfaceHit,
}

impl SurfaceAttachment {
    pub fn status(&self, project: &Project) -> SurfaceAttachmentStatus {
        surface_attachment_status(project, self)
    }

    pub fn rotated(self, delta_radians: f32) -> Result<Self, SurfaceAttachmentError> {
        if !delta_radians.is_finite() {
            return Err(SurfaceAttachmentError::InvalidData);
        }
        let mut attachment = self;
        let rotation =
            (attachment.tangent_rotation + delta_radians).rem_euclid(std::f32::consts::TAU);
        attachment.tangent_rotation = if rotation > std::f32::consts::PI {
            rotation - std::f32::consts::TAU
        } else {
            rotation
        };
        Ok(attachment)
    }

    pub fn is_well_formed(&self) -> bool {
        let barycentric = Vec3::from_array(self.barycentric);
        barycentric.is_finite()
            && self.normal_offset.is_finite()
            && self.tangent_rotation.is_finite()
            && Vec3::from_array(self.last_world_position).is_finite()
            && (barycentric.x + barycentric.y + barycentric.z - 1.0).abs() <= BARYCENTRIC_EPSILON
            && self
                .barycentric
                .iter()
                .all(|weight| (-BARYCENTRIC_EPSILON..=1.0 + BARYCENTRIC_EPSILON).contains(weight))
    }
}

pub fn surface_attachment_status(
    project: &Project,
    attachment: &SurfaceAttachment,
) -> SurfaceAttachmentStatus {
    if !attachment.is_well_formed() {
        return SurfaceAttachmentStatus::Invalid;
    }
    let Some(asset) = project
        .assets
        .iter()
        .find(|asset| asset.id == attachment.target)
    else {
        return SurfaceAttachmentStatus::MissingTarget;
    };
    if asset.mesh.topology_fingerprint() != attachment.source_topology_fingerprint
        || triangle_data(&asset.mesh, attachment.triangle).is_err()
    {
        SurfaceAttachmentStatus::NeedsReattach
    } else {
        SurfaceAttachmentStatus::Valid
    }
}

pub fn evaluate_surface_attachment(
    project: &Project,
    attachment: &SurfaceAttachment,
) -> Result<SurfaceFrame, SurfaceAttachmentError> {
    match surface_attachment_status(project, attachment) {
        SurfaceAttachmentStatus::Valid => {}
        SurfaceAttachmentStatus::NeedsReattach => {
            return Err(SurfaceAttachmentError::NeedsReattach);
        }
        SurfaceAttachmentStatus::MissingTarget => {
            return Err(SurfaceAttachmentError::MissingTarget);
        }
        SurfaceAttachmentStatus::Invalid => return Err(SurfaceAttachmentError::InvalidData),
    }
    let asset = project
        .assets
        .iter()
        .find(|asset| asset.id == attachment.target)
        .ok_or(SurfaceAttachmentError::MissingTarget)?;
    let triangle = triangle_data(&asset.mesh, attachment.triangle)?;
    evaluate_triangle_frame(triangle, attachment)
}

/// Evaluates a path while computing each target's topology fingerprint once.
pub fn evaluate_surface_attachments(
    project: &Project,
    attachments: &[SurfaceAttachment],
) -> Result<Vec<SurfaceFrame>, SurfaceAttachmentError> {
    let mut fingerprints = std::collections::HashMap::new();
    attachments
        .iter()
        .map(|a| {
            if !a.is_well_formed() {
                return Err(SurfaceAttachmentError::InvalidData);
            }
            let asset = project
                .assets
                .iter()
                .find(|asset| asset.id == a.target)
                .ok_or(SurfaceAttachmentError::MissingTarget)?;
            let fingerprint = *fingerprints
                .entry(asset.id)
                .or_insert_with(|| asset.mesh.topology_fingerprint());
            if fingerprint != a.source_topology_fingerprint {
                return Err(SurfaceAttachmentError::NeedsReattach);
            }
            evaluate_triangle_frame(triangle_data(&asset.mesh, a.triangle)?, a)
        })
        .collect()
}

pub fn detach_surface_attachment_keep_world(
    project: &Project,
    attachment: &SurfaceAttachment,
) -> Result<[f32; 3], SurfaceAttachmentError> {
    match evaluate_surface_attachment(project, attachment) {
        Ok(frame) => Ok(frame.position),
        Err(SurfaceAttachmentError::MissingTarget | SurfaceAttachmentError::NeedsReattach)
            if Vec3::from_array(attachment.last_world_position).is_finite() =>
        {
            Ok(attachment.last_world_position)
        }
        Err(error) => Err(error),
    }
}

pub fn project_ray_to_surface(
    project: &Project,
    origin: [f32; 3],
    direction: [f32; 3],
    max_distance: f32,
) -> Result<Option<SurfaceHit>, SurfaceAttachmentError> {
    project_ray(project, None, origin, direction, max_distance)
}

pub fn project_ray_to_surface_target(
    project: &Project,
    target: Uuid,
    origin: [f32; 3],
    direction: [f32; 3],
    max_distance: f32,
) -> Result<Option<SurfaceHit>, SurfaceAttachmentError> {
    let asset = project
        .assets
        .iter()
        .find(|asset| asset.id == target)
        .ok_or(SurfaceAttachmentError::MissingTarget)?;
    if asset.locked {
        return Err(SurfaceAttachmentError::TargetLocked);
    }
    project_ray(project, Some(target), origin, direction, max_distance)
}

pub fn reproject_surface_attachment(
    project: &Project,
    attachment: &SurfaceAttachment,
    origin: [f32; 3],
    direction: [f32; 3],
    max_distance: f32,
) -> Result<SurfaceAttachment, SurfaceAttachmentError> {
    reproject_surface_attachment_to_target(
        project,
        attachment,
        attachment.target,
        origin,
        direction,
        max_distance,
    )
}

pub fn reproject_surface_attachment_to_target(
    project: &Project,
    attachment: &SurfaceAttachment,
    target: Uuid,
    origin: [f32; 3],
    direction: [f32; 3],
    max_distance: f32,
) -> Result<SurfaceAttachment, SurfaceAttachmentError> {
    let hit = project_ray_to_surface_target(project, target, origin, direction, max_distance)?
        .ok_or(SurfaceAttachmentError::NoSurfaceHit)?;
    let mut projected = hit.attachment;
    projected.normal_offset = attachment.normal_offset;
    projected.tangent_rotation = attachment.tangent_rotation;
    projected.last_world_position = evaluate_surface_attachment(project, &projected)?.position;
    Ok(projected)
}

pub fn slide_surface_attachment(
    project: &Project,
    attachment: &SurfaceAttachment,
    origin: [f32; 3],
    direction: [f32; 3],
    max_distance: f32,
) -> Result<SurfaceAttachment, SurfaceAttachmentError> {
    match attachment.status(project) {
        SurfaceAttachmentStatus::Valid => {}
        SurfaceAttachmentStatus::NeedsReattach => {
            return Err(SurfaceAttachmentError::NeedsReattach);
        }
        SurfaceAttachmentStatus::MissingTarget => {
            return Err(SurfaceAttachmentError::MissingTarget);
        }
        SurfaceAttachmentStatus::Invalid => return Err(SurfaceAttachmentError::InvalidData),
    }
    reproject_surface_attachment(project, attachment, origin, direction, max_distance)
}

fn project_ray(
    project: &Project,
    target: Option<Uuid>,
    origin: [f32; 3],
    direction: [f32; 3],
    max_distance: f32,
) -> Result<Option<SurfaceHit>, SurfaceAttachmentError> {
    let origin = Vec3::from_array(origin);
    let direction = Vec3::from_array(direction);
    if !origin.is_finite()
        || !direction.is_finite()
        || direction.length_squared() <= FRAME_EPSILON_SQUARED
        || !max_distance.is_finite()
        || max_distance <= 0.0
    {
        return Err(SurfaceAttachmentError::InvalidData);
    }
    let direction = direction.normalize();
    let mut nearest: Option<SurfaceHit> = None;
    for asset in project.assets.iter().filter(|asset| {
        asset.visible && !asset.locked && target.is_none_or(|target| asset.id == target)
    }) {
        let topology_fingerprint = asset.mesh.topology_fingerprint();
        for (face_index, _) in asset.mesh.faces.iter().enumerate() {
            for (triangle_index, corners) in asset
                .mesh
                .face_triangle_corners(face_index)
                .into_iter()
                .enumerate()
            {
                let face = &asset.mesh.faces[face_index];
                let points =
                    corners.map(|corner| asset.mesh.verts[face.verts[corner] as usize].vec());
                let Some(ray_hit) = petunia_mesh::triangulate::ray_tri_hit(
                    origin, direction, points[0], points[1], points[2],
                ) else {
                    continue;
                };
                if ray_hit.distance > max_distance
                    || nearest
                        .as_ref()
                        .is_some_and(|nearest| ray_hit.distance >= nearest.distance)
                {
                    continue;
                }
                let handle = SurfaceTriangleHandle {
                    face_index: u32::try_from(face_index)
                        .map_err(|_| SurfaceAttachmentError::InvalidData)?,
                    triangle_index: u32::try_from(triangle_index)
                        .map_err(|_| SurfaceAttachmentError::InvalidData)?,
                };
                let base_position = points[0] * ray_hit.barycentric[0]
                    + points[1] * ray_hit.barycentric[1]
                    + points[2] * ray_hit.barycentric[2];
                let mut attachment = SurfaceAttachment {
                    target: asset.id,
                    triangle: handle,
                    barycentric: ray_hit.barycentric,
                    normal_offset: 0.0,
                    tangent_rotation: 0.0,
                    source_topology_revision: project.topology_revision,
                    source_topology_fingerprint: topology_fingerprint,
                    last_world_position: base_position.to_array(),
                };
                let frame =
                    evaluate_triangle_frame(triangle_data(&asset.mesh, handle)?, &attachment)?;
                attachment.last_world_position = frame.position;
                nearest = Some(SurfaceHit {
                    distance: ray_hit.distance,
                    attachment,
                    frame,
                });
            }
        }
    }
    Ok(nearest)
}

struct TriangleData {
    points: [Vec3; 3],
    uvs: Option<[Vec2; 3]>,
}

fn triangle_data(
    mesh: &petunia_mesh::Mesh,
    handle: SurfaceTriangleHandle,
) -> Result<TriangleData, SurfaceAttachmentError> {
    let face_index = handle.face_index as usize;
    let triangle_index = handle.triangle_index as usize;
    let face = mesh
        .faces
        .get(face_index)
        .ok_or(SurfaceAttachmentError::NeedsReattach)?;
    let corners = mesh
        .face_triangle_corners(face_index)
        .get(triangle_index)
        .copied()
        .ok_or(SurfaceAttachmentError::NeedsReattach)?;
    let points = corners.map(|corner| mesh.verts[face.verts[corner] as usize].vec());
    let uvs = (face.uv.len() == face.verts.len())
        .then(|| corners.map(|corner| Vec2::from_array(face.uv[corner])))
        .filter(|uvs| uvs.iter().all(|uv| uv.is_finite()));
    Ok(TriangleData { points, uvs })
}

fn evaluate_triangle_frame(
    triangle: TriangleData,
    attachment: &SurfaceAttachment,
) -> Result<SurfaceFrame, SurfaceAttachmentError> {
    let [a, b, c] = triangle.points;
    let edge_ab = b - a;
    let edge_ac = c - a;
    let normal = edge_ab.cross(edge_ac).normalize_or_zero();
    if normal.length_squared() <= FRAME_EPSILON_SQUARED {
        return Err(SurfaceAttachmentError::DegenerateTriangle);
    }
    let barycentric = Vec3::from_array(attachment.barycentric);
    let surface_position = a * barycentric.x + b * barycentric.y + c * barycentric.z;
    let tangent = triangle
        .uvs
        .and_then(|[uv_a, uv_b, uv_c]| {
            let delta_ab = uv_b - uv_a;
            let delta_ac = uv_c - uv_a;
            let determinant = delta_ab.x * delta_ac.y - delta_ab.y * delta_ac.x;
            (determinant.abs() > 1.0e-8)
                .then(|| (edge_ab * delta_ac.y - edge_ac * delta_ab.y) / determinant)
        })
        .unwrap_or(edge_ab);
    let tangent = (tangent - normal * tangent.dot(normal)).normalize_or_zero();
    let tangent = if tangent.length_squared() > FRAME_EPSILON_SQUARED {
        tangent
    } else {
        let guide = if normal.y.abs() < 0.9 {
            Vec3::Y
        } else {
            Vec3::X
        };
        normal.cross(guide).normalize()
    };
    let bitangent = normal.cross(tangent).normalize();
    let (sine, cosine) = attachment.tangent_rotation.sin_cos();
    let tangent = (tangent * cosine + bitangent * sine).normalize();
    let bitangent = normal.cross(tangent).normalize();
    let uv = triangle.uvs.map(|uvs| {
        (uvs[0] * barycentric.x + uvs[1] * barycentric.y + uvs[2] * barycentric.z).to_array()
    });
    Ok(SurfaceFrame {
        position: (surface_position + normal * attachment.normal_offset).to_array(),
        normal: normal.to_array(),
        tangent: tangent.to_array(),
        bitangent: bitangent.to_array(),
        uv,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_mesh::{Face, Mesh, Vertex};

    fn plane_project() -> (Project, Uuid) {
        let mut project = Project::new();
        project.assets.clear();
        project.add("Surface", Mesh::plane(2.0));
        let target = project.assets[0].id;
        (project, target)
    }

    fn hit_plane(project: &Project, target: Uuid, x: f32, z: f32) -> SurfaceHit {
        project_ray_to_surface_target(project, target, [x, 2.0, z], [0.0, -1.0, 0.0], 10.0)
            .unwrap()
            .unwrap()
    }

    #[test]
    fn raycast_builds_barycentric_attachment_and_surface_frame() {
        let (project, target) = plane_project();
        let hit = hit_plane(&project, target, 0.25, -0.25);
        assert_eq!(hit.attachment.target, target);
        assert!((hit.distance - 2.0).abs() < 1.0e-6);
        assert!((hit.attachment.barycentric.iter().sum::<f32>() - 1.0).abs() < 1.0e-6);
        assert_eq!(
            hit.attachment.status(&project),
            SurfaceAttachmentStatus::Valid
        );
        assert!(
            (Vec3::from_array(hit.frame.position) - Vec3::new(0.25, 0.0, -0.25)).length() < 1.0e-6
        );
        assert!(Vec3::from_array(hit.frame.normal).dot(Vec3::Y) > 0.999);
        assert!(hit.frame.uv.is_some());
    }

    #[test]
    fn position_edits_and_unrelated_revisions_preserve_attachment() {
        let (mut project, target) = plane_project();
        let attachment = hit_plane(&project, target, 0.0, 0.0).attachment;
        project.bump_topology();
        assert_eq!(attachment.status(&project), SurfaceAttachmentStatus::Valid);

        for vertex in &mut project.assets[0].mesh.verts {
            vertex.pos[1] += 1.5;
        }
        let frame = evaluate_surface_attachment(&project, &attachment).unwrap();
        assert_eq!(attachment.status(&project), SurfaceAttachmentStatus::Valid);
        assert!((frame.position[1] - 1.5).abs() < 1.0e-6);
    }

    #[test]
    fn topology_change_requires_explicit_reattach_and_detach_keeps_fallback() {
        let (mut project, target) = plane_project();
        let hit = hit_plane(&project, target, 0.2, 0.2);
        project.assets[0].mesh.faces[0].verts.swap(1, 2);
        assert_eq!(
            hit.attachment.status(&project),
            SurfaceAttachmentStatus::NeedsReattach
        );
        assert_eq!(
            evaluate_surface_attachment(&project, &hit.attachment),
            Err(SurfaceAttachmentError::NeedsReattach)
        );
        assert_eq!(
            detach_surface_attachment_keep_world(&project, &hit.attachment).unwrap(),
            hit.attachment.last_world_position
        );
        project.assets.clear();
        assert_eq!(
            hit.attachment.status(&project),
            SurfaceAttachmentStatus::MissingTarget
        );
        assert_eq!(
            detach_surface_attachment_keep_world(&project, &hit.attachment).unwrap(),
            hit.attachment.last_world_position
        );
        assert_eq!(
            slide_surface_attachment(
                &project,
                &hit.attachment,
                [0.0, 2.0, 0.0],
                [0.0, -1.0, 0.0],
                10.0,
            ),
            Err(SurfaceAttachmentError::MissingTarget)
        );
    }

    #[test]
    fn reprojection_is_deterministic_and_preserves_authored_offsets() {
        let (project, target) = plane_project();
        let mut attachment = hit_plane(&project, target, -0.25, -0.25).attachment;
        attachment.normal_offset = 0.125;
        attachment.tangent_rotation = 0.75;
        let first = reproject_surface_attachment(
            &project,
            &attachment,
            [0.3, 2.0, 0.4],
            [0.0, -1.0, 0.0],
            10.0,
        )
        .unwrap();
        let second = reproject_surface_attachment(
            &project,
            &attachment,
            [0.3, 2.0, 0.4],
            [0.0, -1.0, 0.0],
            10.0,
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.normal_offset, 0.125);
        assert_eq!(first.tangent_rotation, 0.75);
        assert!((first.last_world_position[1] - 0.125).abs() < 1.0e-6);
    }

    #[test]
    fn face_corner_uvs_keep_seam_sides_distinct() {
        let mesh = Mesh {
            verts: vec![
                Vertex::new(-1.0, 0.0, -1.0),
                Vertex::new(1.0, 0.0, -1.0),
                Vertex::new(1.0, 0.0, 1.0),
                Vertex::new(-1.0, 0.0, 1.0),
            ],
            faces: vec![
                Face::with_uv(vec![0, 2, 1], vec![[0.0, 0.0], [1.0, 1.0], [1.0, 0.0]]),
                Face::with_uv(
                    vec![0, 3, 2],
                    vec![[10.0, 10.0], [10.0, 11.0], [11.0, 11.0]],
                ),
            ],
            ..Mesh::default()
        };
        let mut project = Project::new();
        project.assets.clear();
        project.add("UV seam", mesh);
        let target = project.assets[0].id;
        let topology_fingerprint = project.assets[0].mesh.topology_fingerprint();
        let attachment = |face_index| SurfaceAttachment {
            target,
            triangle: SurfaceTriangleHandle {
                face_index,
                triangle_index: 0,
            },
            barycentric: [0.25, 0.25, 0.5],
            normal_offset: 0.0,
            tangent_rotation: 0.0,
            source_topology_revision: project.topology_revision,
            source_topology_fingerprint: topology_fingerprint,
            last_world_position: [0.0; 3],
        };
        let first = evaluate_surface_attachment(&project, &attachment(0)).unwrap();
        let second = evaluate_surface_attachment(&project, &attachment(1)).unwrap();
        assert!(first.uv.unwrap()[0] < 2.0);
        assert!(second.uv.unwrap()[0] > 9.0);
    }

    #[test]
    fn queries_reject_invalid_rays_and_respect_visibility_and_locking() {
        let (mut project, target) = plane_project();
        assert_eq!(
            project_ray_to_surface(&project, [0.0; 3], [0.0; 3], 10.0),
            Err(SurfaceAttachmentError::InvalidData)
        );
        project.assets[0].visible = false;
        assert_eq!(
            project_ray_to_surface_target(
                &project,
                target,
                [0.0, 2.0, 0.0],
                [0.0, -1.0, 0.0],
                10.0,
            ),
            Ok(None)
        );
        project.assets[0].visible = true;
        project.assets[0].mesh.faces[0].uv[0] = [f32::NAN, 0.0];
        assert!(hit_plane(&project, target, 0.0, 0.0).frame.uv.is_none());
        project.assets[0].locked = true;
        assert_eq!(
            project_ray_to_surface_target(
                &project,
                target,
                [0.0, 2.0, 0.0],
                [0.0, -1.0, 0.0],
                10.0,
            ),
            Err(SurfaceAttachmentError::TargetLocked)
        );
    }
}
