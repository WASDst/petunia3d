//! Frames de transporte paralelo compartilhados por Sweep, Spline e generators.

use glam::DVec3;

const EPSILON_SQUARED: f64 = 1.0e-18;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PathFrame {
    pub origin: DVec3,
    pub tangent: DVec3,
    pub normal: DVec3,
    pub binormal: DVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PathFrameError {
    #[error("path needs at least two points")]
    TooFewPoints,
    #[error("path has no measurable segment")]
    DegeneratePath,
    #[error("path contains a non-finite point")]
    NonFinitePoint,
}

pub fn compute_parallel_transport_frames(
    path: &[DVec3],
    closed: bool,
) -> Result<Vec<PathFrame>, PathFrameError> {
    if path.len() < 2 {
        return Err(PathFrameError::TooFewPoints);
    }
    if path.iter().any(|point| !point.is_finite()) {
        return Err(PathFrameError::NonFinitePoint);
    }

    let tangents: Vec<_> = (0..path.len())
        .map(|index| tangent_at(path, index, closed))
        .collect::<Option<_>>()
        .ok_or(PathFrameError::DegeneratePath)?;
    let first_tangent = tangents[0];
    let first_normal = fallback_normal(first_tangent);
    let first_binormal = first_tangent.cross(first_normal).normalize();
    let mut frames = Vec::with_capacity(path.len());
    frames.push(PathFrame {
        origin: path[0],
        tangent: first_tangent,
        normal: first_normal,
        binormal: first_binormal,
    });

    for index in 0..path.len() - 1 {
        let current = frames[index];
        let next_origin = path[index + 1];
        let next_tangent = tangents[index + 1];
        let segment = next_origin - current.origin;
        let segment_length_squared = segment.length_squared();
        let candidate = if segment_length_squared <= EPSILON_SQUARED {
            current.normal
        } else {
            let reflected_normal = current.normal
                - (2.0 / segment_length_squared) * segment.dot(current.normal) * segment;
            let reflected_tangent = current.tangent
                - (2.0 / segment_length_squared) * segment.dot(current.tangent) * segment;
            let tangent_delta = next_tangent - reflected_tangent;
            let tangent_delta_length_squared = tangent_delta.length_squared();
            if tangent_delta_length_squared <= EPSILON_SQUARED {
                reflected_normal
            } else {
                reflected_normal
                    - (2.0 / tangent_delta_length_squared)
                        * tangent_delta.dot(reflected_normal)
                        * tangent_delta
            }
        };
        let normal = orthogonalized_normal(next_tangent, candidate);
        frames.push(PathFrame {
            origin: next_origin,
            tangent: next_tangent,
            normal,
            binormal: next_tangent.cross(normal).normalize(),
        });
    }

    if closed && path.len() > 2 {
        compensate_closed_twist(&mut frames);
    }
    Ok(frames)
}

fn tangent_at(path: &[DVec3], index: usize, closed: bool) -> Option<DVec3> {
    let previous = distinct_neighbor(path, index, false, closed);
    let next = distinct_neighbor(path, index, true, closed);
    let tangent = match (previous, next) {
        (Some(previous), Some(next)) if closed || (index > 0 && index + 1 < path.len()) => {
            (next - previous).normalize_or_zero()
        }
        (_, Some(next)) => (next - path[index]).normalize_or_zero(),
        (Some(previous), None) => (path[index] - previous).normalize_or_zero(),
        (None, None) => return None,
    };
    if tangent.length_squared() > EPSILON_SQUARED {
        Some(tangent)
    } else if let Some(next) = next {
        let fallback = (next - path[index]).normalize_or_zero();
        (fallback.length_squared() > EPSILON_SQUARED).then_some(fallback)
    } else if let Some(previous) = previous {
        let fallback = (path[index] - previous).normalize_or_zero();
        (fallback.length_squared() > EPSILON_SQUARED).then_some(fallback)
    } else {
        None
    }
}

fn distinct_neighbor(path: &[DVec3], index: usize, forward: bool, closed: bool) -> Option<DVec3> {
    for step in 1..path.len() {
        let candidate_index = if forward {
            let candidate = index + step;
            if !closed && candidate >= path.len() {
                break;
            }
            candidate % path.len()
        } else if index >= step {
            index - step
        } else if closed {
            path.len() - (step - index) % path.len()
        } else {
            break;
        };
        let candidate_index = candidate_index % path.len();
        let candidate = path[candidate_index];
        if (candidate - path[index]).length_squared() > EPSILON_SQUARED {
            return Some(candidate);
        }
    }
    None
}

fn fallback_normal(tangent: DVec3) -> DVec3 {
    let guide = if tangent.y.abs() < 0.9 {
        DVec3::Y
    } else {
        DVec3::X
    };
    tangent.cross(guide).normalize()
}

fn orthogonalized_normal(tangent: DVec3, candidate: DVec3) -> DVec3 {
    let projected = candidate - tangent * candidate.dot(tangent);
    if projected.length_squared() > EPSILON_SQUARED {
        projected.normalize()
    } else {
        fallback_normal(tangent)
    }
}

fn compensate_closed_twist(frames: &mut [PathFrame]) {
    let first = frames[0];
    let last = frames[frames.len() - 1];
    let closing_segment = first.origin - last.origin;
    let closing_length_squared = closing_segment.length_squared();
    let end_normal = if closing_length_squared <= EPSILON_SQUARED {
        last.normal
    } else {
        let reflected_normal = last.normal
            - (2.0 / closing_length_squared) * closing_segment.dot(last.normal) * closing_segment;
        let reflected_tangent = last.tangent
            - (2.0 / closing_length_squared) * closing_segment.dot(last.tangent) * closing_segment;
        let tangent_delta = first.tangent - reflected_tangent;
        let tangent_delta_length_squared = tangent_delta.length_squared();
        let candidate = if tangent_delta_length_squared <= EPSILON_SQUARED {
            reflected_normal
        } else {
            reflected_normal
                - (2.0 / tangent_delta_length_squared)
                    * tangent_delta.dot(reflected_normal)
                    * tangent_delta
        };
        orthogonalized_normal(first.tangent, candidate)
    };
    let cosine = end_normal.dot(first.normal).clamp(-1.0, 1.0);
    let sine = end_normal.dot(first.binormal);
    let correction = sine.atan2(cosine);
    let frame_count = frames.len() as f64;
    for (index, frame) in frames.iter_mut().enumerate() {
        let angle = -correction * index as f64 / frame_count;
        let (sine, cosine) = angle.sin_cos();
        let normal = frame.normal * cosine + frame.binormal * sine;
        frame.normal = orthogonalized_normal(frame.tangent, normal);
        frame.binormal = frame.tangent.cross(frame.normal).normalize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_orthonormal(frame: PathFrame) {
        assert!((frame.tangent.length() - 1.0).abs() < 1.0e-9);
        assert!((frame.normal.length() - 1.0).abs() < 1.0e-9);
        assert!((frame.binormal.length() - 1.0).abs() < 1.0e-9);
        assert!(frame.tangent.dot(frame.normal).abs() < 1.0e-9);
        assert!(frame.tangent.dot(frame.binormal).abs() < 1.0e-9);
        assert!(frame.normal.dot(frame.binormal).abs() < 1.0e-9);
    }

    #[test]
    fn straight_path_builds_stable_frames() {
        let frames =
            compute_parallel_transport_frames(&[DVec3::ZERO, DVec3::Z, DVec3::Z * 2.0], false)
                .unwrap();
        assert_eq!(frames.len(), 3);
        for frame in frames {
            assert_orthonormal(frame);
            assert!(frame.tangent.dot(DVec3::Z) > 0.999_999);
        }
    }

    #[test]
    fn repeated_points_preserve_cardinality_without_nan() {
        let path = [
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(1.0, 1.0, 0.0),
        ];
        let frames = compute_parallel_transport_frames(&path, false).unwrap();
        assert_eq!(frames.len(), path.len());
        frames.into_iter().for_each(assert_orthonormal);
    }

    #[test]
    fn closed_loop_compensation_remains_finite_and_deterministic() {
        let path = [
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.5),
            DVec3::new(-1.0, 0.0, 0.0),
            DVec3::new(0.0, -1.0, -0.5),
        ];
        let first = compute_parallel_transport_frames(&path, true).unwrap();
        let second = compute_parallel_transport_frames(&path, true).unwrap();
        assert_eq!(first, second);
        first.into_iter().for_each(assert_orthonormal);
    }

    #[test]
    fn s_curve_and_sharp_corner_do_not_flip_or_lose_orthogonality() {
        let path = [
            DVec3::new(-2.0, 0.0, 0.0),
            DVec3::new(-1.0, 1.0, 0.5),
            DVec3::ZERO,
            DVec3::new(1.0, -1.0, -0.5),
            DVec3::new(1.0, 2.0, 0.0),
        ];
        let frames = compute_parallel_transport_frames(&path, false).unwrap();
        frames.iter().copied().for_each(assert_orthonormal);
        for pair in frames.windows(2) {
            assert!(pair[0].normal.dot(pair[1].normal) > -0.5);
        }
    }

    #[test]
    fn mixed_extreme_scales_remain_finite() {
        let path = [
            DVec3::ZERO,
            DVec3::new(1.0e-6, 0.0, 0.0),
            DVec3::new(1.0e6, 1.0e3, -1.0e2),
        ];
        let frames = compute_parallel_transport_frames(&path, false).unwrap();
        frames.into_iter().for_each(assert_orthonormal);
    }
}
