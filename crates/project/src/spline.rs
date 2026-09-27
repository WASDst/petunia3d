//! Authoring e avaliação headless de splines persistentes (P3D-161).

use std::collections::HashSet;

use glam::DVec3;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use petunia_mesh::PathFrame as SplineFrame;

const POSITION_EPSILON_SQUARED: f64 = 1.0e-18;
const DEFAULT_TOLERANCE: f64 = 1.0e-4;
const MAX_TESSELLATION_DEPTH: u8 = 16;
const MAX_EVALUATION_POINTS: usize = 262_144;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplineInterpolation {
    #[default]
    Polyline,
    CubicBezier,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplineHandleMode {
    #[default]
    Broken,
    Aligned,
    Mirrored,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplinePoint {
    pub id: Uuid,
    pub position: [f64; 3],
    /// Vetor relativo ao ponto de controle.
    pub handle_in: [f64; 3],
    /// Vetor relativo ao ponto de controle.
    pub handle_out: [f64; 3],
    pub handle_mode: SplineHandleMode,
}

impl SplinePoint {
    pub fn new(position: [f64; 3]) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            handle_in: [0.0; 3],
            handle_out: [0.0; 3],
            handle_mode: SplineHandleMode::Broken,
        }
    }

    pub fn with_handles(
        position: [f64; 3],
        handle_in: [f64; 3],
        handle_out: [f64; 3],
        handle_mode: SplineHandleMode,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            position,
            handle_in,
            handle_out,
            handle_mode,
        }
    }

    pub fn set_handle_in(&mut self, handle: [f64; 3]) -> Result<(), SplineError> {
        let handle = finite_vec(handle)?;
        self.handle_in = handle.to_array();
        match self.handle_mode {
            SplineHandleMode::Broken => {}
            SplineHandleMode::Aligned => {
                self.handle_out =
                    aligned_opposite(handle, DVec3::from_array(self.handle_out)).to_array();
            }
            SplineHandleMode::Mirrored => self.handle_out = (-handle).to_array(),
        }
        Ok(())
    }

    pub fn set_handle_out(&mut self, handle: [f64; 3]) -> Result<(), SplineError> {
        let handle = finite_vec(handle)?;
        self.handle_out = handle.to_array();
        match self.handle_mode {
            SplineHandleMode::Broken => {}
            SplineHandleMode::Aligned => {
                self.handle_in =
                    aligned_opposite(handle, DVec3::from_array(self.handle_in)).to_array();
            }
            SplineHandleMode::Mirrored => self.handle_in = (-handle).to_array(),
        }
        Ok(())
    }

    pub fn resolved_handles(
        handle_in: [f64; 3],
        handle_out: [f64; 3],
        mode: SplineHandleMode,
    ) -> Result<([f64; 3], [f64; 3]), SplineError> {
        let handle_in = finite_vec(handle_in)?;
        let handle_out = finite_vec(handle_out)?;
        let handle_out = match mode {
            SplineHandleMode::Broken => handle_out,
            SplineHandleMode::Aligned => aligned_opposite(handle_in, handle_out),
            SplineHandleMode::Mirrored => -handle_in,
        };
        Ok((handle_in.to_array(), handle_out.to_array()))
    }

    fn validate(&mut self) {
        if !DVec3::from_array(self.position).is_finite() {
            self.position = [0.0; 3];
        }
        if !DVec3::from_array(self.handle_in).is_finite() {
            self.handle_in = [0.0; 3];
        }
        if !DVec3::from_array(self.handle_out).is_finite() {
            self.handle_out = [0.0; 3];
        }
        match self.handle_mode {
            SplineHandleMode::Broken => {}
            SplineHandleMode::Aligned => {
                let input = DVec3::from_array(self.handle_in);
                let output = DVec3::from_array(self.handle_out);
                self.handle_out = aligned_opposite(input, output).to_array();
            }
            SplineHandleMode::Mirrored => {
                self.handle_out = (-DVec3::from_array(self.handle_in)).to_array();
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplineResource {
    pub id: Uuid,
    pub name: String,
    pub points: Vec<SplinePoint>,
    pub closed: bool,
    pub interpolation: SplineInterpolation,
    pub revision: u64,
}

impl SplineResource {
    pub fn new(name: impl Into<String>, interpolation: SplineInterpolation) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            points: Vec::new(),
            closed: false,
            interpolation,
            revision: 0,
        }
    }

    pub fn from_polyline(name: impl Into<String>, points: &[[f64; 3]], closed: bool) -> Self {
        let mut spline = Self::new(name, SplineInterpolation::Polyline);
        spline.points = points.iter().copied().map(SplinePoint::new).collect();
        spline.closed = closed && spline.points.len() >= 3;
        spline
    }

    pub fn point(&self, point_id: Uuid) -> Option<&SplinePoint> {
        self.points.iter().find(|point| point.id == point_id)
    }

    pub fn point_mut(&mut self, point_id: Uuid) -> Option<&mut SplinePoint> {
        self.points.iter_mut().find(|point| point.id == point_id)
    }

    pub fn add_point(&mut self, point: SplinePoint) -> Result<(), SplineError> {
        self.insert_point(self.points.len(), point)
    }

    pub fn insert_point(&mut self, index: usize, point: SplinePoint) -> Result<(), SplineError> {
        validate_point_data(&point)?;
        if index > self.points.len() {
            return Err(SplineError::InvalidPointIndex(index));
        }
        if self.point(point.id).is_some() {
            return Err(SplineError::DuplicatePoint(point.id));
        }
        self.points.insert(index, point);
        self.bump_revision();
        Ok(())
    }

    pub fn move_point(&mut self, point_id: Uuid, position: [f64; 3]) -> Result<(), SplineError> {
        let position = finite_vec(position)?.to_array();
        let point = self
            .point_mut(point_id)
            .ok_or(SplineError::PointNotFound(point_id))?;
        if point.position != position {
            point.position = position;
            self.bump_revision();
        }
        Ok(())
    }

    pub fn set_handles(
        &mut self,
        point_id: Uuid,
        handle_in: [f64; 3],
        handle_out: [f64; 3],
        mode: SplineHandleMode,
    ) -> Result<(), SplineError> {
        let (handle_in, handle_out) = SplinePoint::resolved_handles(handle_in, handle_out, mode)?;
        let point = self
            .point_mut(point_id)
            .ok_or(SplineError::PointNotFound(point_id))?;
        let before = (point.handle_in, point.handle_out, point.handle_mode);
        point.handle_mode = mode;
        point.handle_in = handle_in;
        point.handle_out = handle_out;
        if before != (point.handle_in, point.handle_out, point.handle_mode) {
            self.bump_revision();
        }
        Ok(())
    }

    pub fn delete_point(&mut self, point_id: Uuid) -> Result<SplinePoint, SplineError> {
        let index = self
            .points
            .iter()
            .position(|point| point.id == point_id)
            .ok_or(SplineError::PointNotFound(point_id))?;
        let point = self.points.remove(index);
        if self.points.len() < 3 {
            self.closed = false;
        }
        self.bump_revision();
        Ok(point)
    }

    pub fn set_closed(&mut self, closed: bool) -> Result<(), SplineError> {
        if closed && self.points.len() < 3 {
            return Err(SplineError::ClosedNeedsThreePoints);
        }
        if self.closed != closed {
            self.closed = closed;
            self.bump_revision();
        }
        Ok(())
    }

    pub fn reverse(&mut self) {
        self.points.reverse();
        for point in &mut self.points {
            std::mem::swap(&mut point.handle_in, &mut point.handle_out);
        }
        if self.points.len() > 1 {
            self.bump_revision();
        }
    }

    pub fn segment_count(&self) -> usize {
        if self.points.len() < 2 {
            0
        } else if self.closed {
            self.points.len()
        } else {
            self.points.len() - 1
        }
    }

    pub fn evaluate_segment(
        &self,
        segment: usize,
        parameter: f64,
    ) -> Result<[f64; 3], SplineError> {
        let (start, end) = self.segment(segment)?;
        let parameter = finite_parameter(parameter)?;
        let start_position = DVec3::from_array(start.position);
        let end_position = DVec3::from_array(end.position);
        let position = match self.interpolation {
            SplineInterpolation::Polyline => start_position.lerp(end_position, parameter),
            SplineInterpolation::CubicBezier => cubic_position(
                start_position,
                start_position + DVec3::from_array(start.handle_out),
                end_position + DVec3::from_array(end.handle_in),
                end_position,
                parameter,
            ),
        };
        Ok(position.to_array())
    }

    pub fn tangent_segment(&self, segment: usize, parameter: f64) -> Result<[f64; 3], SplineError> {
        let (start, end) = self.segment(segment)?;
        let parameter = finite_parameter(parameter)?;
        let start_position = DVec3::from_array(start.position);
        let end_position = DVec3::from_array(end.position);
        let tangent = match self.interpolation {
            SplineInterpolation::Polyline => end_position - start_position,
            SplineInterpolation::CubicBezier => cubic_tangent(
                start_position,
                start_position + DVec3::from_array(start.handle_out),
                end_position + DVec3::from_array(end.handle_in),
                end_position,
                parameter,
            ),
        };
        let tangent = if tangent.length_squared() > POSITION_EPSILON_SQUARED {
            tangent.normalize()
        } else {
            (end_position - start_position).normalize_or_zero()
        };
        if tangent.length_squared() <= POSITION_EPSILON_SQUARED {
            Err(SplineError::DegenerateSpline)
        } else {
            Ok(tangent.to_array())
        }
    }

    pub fn arc_length_table(&self, tolerance: f64) -> Result<ArcLengthTable, SplineError> {
        ArcLengthTable::build(self, tolerance)
    }

    pub fn resample_by_spacing(
        &self,
        spacing: f64,
        tolerance: f64,
    ) -> Result<Vec<SplineSample>, SplineError> {
        self.arc_length_table(tolerance)?.resample(spacing)
    }

    pub fn convert_to_polyline(&mut self, spacing: f64, tolerance: f64) -> Result<(), SplineError> {
        let samples = self.resample_by_spacing(spacing, tolerance)?;
        self.points = samples
            .into_iter()
            .map(|sample| SplinePoint::new(sample.position))
            .collect();
        self.interpolation = SplineInterpolation::Polyline;
        self.bump_revision();
        Ok(())
    }

    pub fn validate_authoring(&self) -> Result<(), SplineError> {
        if self.closed && self.points.len() < 3 {
            return Err(SplineError::ClosedNeedsThreePoints);
        }
        let mut point_ids = HashSet::with_capacity(self.points.len());
        for point in &self.points {
            if !point_ids.insert(point.id) {
                return Err(SplineError::DuplicatePoint(point.id));
            }
            validate_point_data(point)?;
        }
        Ok(())
    }

    pub fn validate(&mut self) {
        if self.name.trim().is_empty() {
            self.name = "Spline".to_string();
        }
        let mut ids = HashSet::with_capacity(self.points.len());
        for point in &mut self.points {
            if !ids.insert(point.id) {
                point.id = Uuid::new_v4();
                ids.insert(point.id);
            }
            point.validate();
        }
        if self.points.len() < 3 {
            self.closed = false;
        }
    }

    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            .saturating_add(self.name.capacity())
            .saturating_add(
                self.points
                    .capacity()
                    .saturating_mul(std::mem::size_of::<SplinePoint>()),
            )
    }

    fn geometry_fingerprint(&self) -> u128 {
        const FNV_OFFSET: u128 = 0x6c62_272e_07bb_0142_62b8_2175_6295_c58d;
        const FNV_PRIME: u128 = 0x0000_0000_0100_0000_0000_0000_0000_013b;

        let mut fingerprint = FNV_OFFSET;
        let mut mix = |value: u64| {
            fingerprint ^= u128::from(value);
            fingerprint = fingerprint.wrapping_mul(FNV_PRIME);
        };
        mix(self.closed as u64);
        mix(self.interpolation as u64);
        mix(self.points.len() as u64);
        for point in &self.points {
            for component in point
                .position
                .iter()
                .chain(&point.handle_in)
                .chain(&point.handle_out)
            {
                mix(component.to_bits());
            }
        }
        fingerprint
    }

    fn segment(&self, segment: usize) -> Result<(&SplinePoint, &SplinePoint), SplineError> {
        if segment >= self.segment_count() {
            return Err(SplineError::InvalidSegment(segment));
        }
        Ok((
            &self.points[segment],
            &self.points[(segment + 1) % self.points.len()],
        ))
    }

    fn tessellated_positions(&self, tolerance: f64) -> Result<Vec<DVec3>, SplineError> {
        if self.segment_count() == 0 {
            return Err(SplineError::NeedsTwoPoints);
        }
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(SplineError::InvalidTolerance);
        }
        self.points.iter().try_for_each(validate_point_data)?;
        let mut positions = vec![DVec3::from_array(self.points[0].position)];
        for segment in 0..self.segment_count() {
            let (start, end) = self.segment(segment)?;
            let start_position = DVec3::from_array(start.position);
            let end_position = DVec3::from_array(end.position);
            match self.interpolation {
                SplineInterpolation::Polyline => {
                    push_evaluation_point(&mut positions, end_position)?;
                }
                SplineInterpolation::CubicBezier => {
                    tessellate_cubic(
                        start_position,
                        start_position + DVec3::from_array(start.handle_out),
                        end_position + DVec3::from_array(end.handle_in),
                        end_position,
                        tolerance,
                        0,
                        &mut positions,
                    )?;
                }
            }
        }
        positions
            .dedup_by(|left, right| (*left - *right).length_squared() <= POSITION_EPSILON_SQUARED);
        if self.closed
            && positions.len() > 1
            && (positions[0] - positions[positions.len() - 1]).length_squared()
                <= POSITION_EPSILON_SQUARED
        {
            positions.pop();
        }
        if positions.len() < 2 {
            Err(SplineError::DegenerateSpline)
        } else {
            Ok(positions)
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplineSample {
    pub position: [f64; 3],
    pub tangent: [f64; 3],
    pub distance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArcLengthTable {
    positions: Vec<DVec3>,
    cumulative_lengths: Vec<f64>,
    total_length: f64,
    closed: bool,
}

impl ArcLengthTable {
    fn build(spline: &SplineResource, tolerance: f64) -> Result<Self, SplineError> {
        if !tolerance.is_finite() || tolerance <= 0.0 {
            return Err(SplineError::InvalidTolerance);
        }
        let positions = spline.tessellated_positions(tolerance.max(DEFAULT_TOLERANCE * 0.01))?;
        let mut cumulative_lengths = Vec::with_capacity(positions.len());
        cumulative_lengths.push(0.0);
        let mut total_length = 0.0;
        for pair in positions.windows(2) {
            total_length += pair[0].distance(pair[1]);
            cumulative_lengths.push(total_length);
        }
        if spline.closed {
            total_length += positions[positions.len() - 1].distance(positions[0]);
        }
        if !total_length.is_finite() || total_length <= f64::EPSILON {
            return Err(SplineError::DegenerateSpline);
        }
        Ok(Self {
            positions,
            cumulative_lengths,
            total_length,
            closed: spline.closed,
        })
    }

    pub fn total_length(&self) -> f64 {
        self.total_length
    }

    pub fn polyline(&self) -> Vec<[f64; 3]> {
        self.positions
            .iter()
            .map(|point| point.to_array())
            .collect()
    }

    pub fn sample_at_distance(&self, distance: f64) -> Result<SplineSample, SplineError> {
        if !distance.is_finite() {
            return Err(SplineError::InvalidDistance);
        }
        let distance = if self.closed {
            distance.rem_euclid(self.total_length)
        } else {
            distance.clamp(0.0, self.total_length)
        };
        if !self.closed && distance >= self.total_length {
            let end = self.positions.len() - 1;
            let tangent = (self.positions[end] - self.positions[end - 1]).normalize();
            return Ok(SplineSample {
                position: self.positions[end].to_array(),
                tangent: tangent.to_array(),
                distance: self.total_length,
            });
        }

        for index in 0..self.positions.len() {
            let next = (index + 1) % self.positions.len();
            if next == 0 && !self.closed {
                break;
            }
            let start_distance = self.cumulative_lengths[index];
            let end_distance = if next == 0 {
                self.total_length
            } else {
                self.cumulative_lengths[next]
            };
            if distance <= end_distance {
                let segment_length = end_distance - start_distance;
                let parameter = if segment_length > f64::EPSILON {
                    (distance - start_distance) / segment_length
                } else {
                    0.0
                };
                let tangent = (self.positions[next] - self.positions[index]).normalize();
                return Ok(SplineSample {
                    position: self.positions[index]
                        .lerp(self.positions[next], parameter)
                        .to_array(),
                    tangent: tangent.to_array(),
                    distance,
                });
            }
        }
        Err(SplineError::DegenerateSpline)
    }

    pub fn resample(&self, spacing: f64) -> Result<Vec<SplineSample>, SplineError> {
        if !spacing.is_finite() || spacing <= 0.0 {
            return Err(SplineError::InvalidSpacing);
        }
        let minimum_intervals = if self.closed { 3 } else { 1 };
        let requested_intervals = (self.total_length / spacing)
            .ceil()
            .max(minimum_intervals as f64);
        let max_intervals = if self.closed {
            MAX_EVALUATION_POINTS
        } else {
            MAX_EVALUATION_POINTS - 1
        };
        if !requested_intervals.is_finite() || requested_intervals > max_intervals as f64 {
            return Err(SplineError::EvaluationLimitExceeded);
        }
        let intervals = requested_intervals as usize;
        let step = self.total_length / intervals as f64;
        let sample_count = if self.closed {
            intervals
        } else {
            intervals + 1
        };
        (0..sample_count)
            .map(|index| self.sample_at_distance(index as f64 * step))
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SplineEvaluationCache {
    table_key: Option<(Uuid, u64, u128, u64)>,
    table: Option<ArcLengthTable>,
    sample_key: Option<(Uuid, u64, u128, u64, u64)>,
    samples: Vec<SplineSample>,
    frames: Vec<SplineFrame>,
}

impl SplineEvaluationCache {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn arc_length_table(
        &mut self,
        spline: &SplineResource,
        tolerance: f64,
    ) -> Result<&ArcLengthTable, SplineError> {
        let key = (
            spline.id,
            spline.revision,
            spline.geometry_fingerprint(),
            tolerance.to_bits(),
        );
        if self.table_key != Some(key) {
            self.table = Some(spline.arc_length_table(tolerance)?);
            self.table_key = Some(key);
            self.sample_key = None;
            self.samples.clear();
            self.frames.clear();
        }
        self.table.as_ref().ok_or(SplineError::DegenerateSpline)
    }

    pub fn samples_by_spacing(
        &mut self,
        spline: &SplineResource,
        spacing: f64,
        tolerance: f64,
    ) -> Result<&[SplineSample], SplineError> {
        let key = (
            spline.id,
            spline.revision,
            spline.geometry_fingerprint(),
            tolerance.to_bits(),
            spacing.to_bits(),
        );
        if self.sample_key != Some(key) {
            self.samples = self
                .arc_length_table(spline, tolerance)?
                .resample(spacing)?;
            self.frames.clear();
            self.sample_key = Some(key);
        }
        Ok(&self.samples)
    }

    pub fn frames_by_spacing(
        &mut self,
        spline: &SplineResource,
        spacing: f64,
        tolerance: f64,
    ) -> Result<&[SplineFrame], SplineError> {
        let key = (
            spline.id,
            spline.revision,
            spline.geometry_fingerprint(),
            tolerance.to_bits(),
            spacing.to_bits(),
        );
        if self.sample_key != Some(key) || self.frames.is_empty() {
            let path: Vec<_> = self
                .samples_by_spacing(spline, spacing, tolerance)?
                .iter()
                .map(|sample| DVec3::from_array(sample.position))
                .collect();
            self.frames = petunia_mesh::compute_parallel_transport_frames(&path, spline.closed)
                .map_err(|_| SplineError::DegenerateSpline)?;
        }
        Ok(&self.frames)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplineSnapSettings {
    pub grid_step: Option<f64>,
    pub point_radius: f64,
}

impl Default for SplineSnapSettings {
    fn default() -> Self {
        Self {
            grid_step: None,
            point_radius: 0.0,
        }
    }
}

pub fn snap_spline_position(
    position: [f64; 3],
    settings: SplineSnapSettings,
    candidates: &[[f64; 3]],
) -> Result<[f64; 3], SplineError> {
    let position = finite_vec(position)?;
    if !settings.point_radius.is_finite() || settings.point_radius < 0.0 {
        return Err(SplineError::InvalidSnapSettings);
    }
    if settings
        .grid_step
        .is_some_and(|step| !step.is_finite() || step <= 0.0)
    {
        return Err(SplineError::InvalidSnapSettings);
    }
    if settings.point_radius > 0.0 {
        let radius_squared = settings.point_radius * settings.point_radius;
        if let Some(candidate) = candidates
            .iter()
            .filter_map(|candidate| finite_vec(*candidate).ok())
            .filter_map(|candidate| {
                let distance_squared = position.distance_squared(candidate);
                (distance_squared <= radius_squared).then_some((candidate, distance_squared))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
        {
            return Ok(candidate.0.to_array());
        }
    }
    if let Some(step) = settings.grid_step {
        return Ok(((position / step).round() * step).to_array());
    }
    Ok(position.to_array())
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SplineError {
    #[error("spline id is duplicated: {0}")]
    DuplicateSpline(Uuid),
    #[error("spline was not found: {0}")]
    SplineNotFound(Uuid),
    #[error("spline needs at least two points")]
    NeedsTwoPoints,
    #[error("a closed spline needs at least three points")]
    ClosedNeedsThreePoints,
    #[error("spline point was not found: {0}")]
    PointNotFound(Uuid),
    #[error("spline point id is duplicated: {0}")]
    DuplicatePoint(Uuid),
    #[error("invalid spline point index: {0}")]
    InvalidPointIndex(usize),
    #[error("invalid spline segment: {0}")]
    InvalidSegment(usize),
    #[error("spline data must be finite")]
    NonFinite,
    #[error("spline parameter must be finite")]
    InvalidParameter,
    #[error("arc-length tolerance must be positive and finite")]
    InvalidTolerance,
    #[error("sample spacing must be positive and finite")]
    InvalidSpacing,
    #[error("sample distance must be finite")]
    InvalidDistance,
    #[error("spline snap settings must be non-negative and finite")]
    InvalidSnapSettings,
    #[error("spline has no measurable length")]
    DegenerateSpline,
    #[error("spline evaluation exceeds the safety point limit")]
    EvaluationLimitExceeded,
}

fn validate_point_data(point: &SplinePoint) -> Result<(), SplineError> {
    finite_vec(point.position)?;
    finite_vec(point.handle_in)?;
    finite_vec(point.handle_out)?;
    Ok(())
}

fn finite_vec(value: [f64; 3]) -> Result<DVec3, SplineError> {
    let value = DVec3::from_array(value);
    value
        .is_finite()
        .then_some(value)
        .ok_or(SplineError::NonFinite)
}

fn finite_parameter(parameter: f64) -> Result<f64, SplineError> {
    parameter
        .is_finite()
        .then_some(parameter.clamp(0.0, 1.0))
        .ok_or(SplineError::InvalidParameter)
}

fn aligned_opposite(changed: DVec3, previous_opposite: DVec3) -> DVec3 {
    if changed.length_squared() <= POSITION_EPSILON_SQUARED {
        return DVec3::ZERO;
    }
    let length = if previous_opposite.length_squared() > POSITION_EPSILON_SQUARED {
        previous_opposite.length()
    } else {
        changed.length()
    };
    -changed.normalize() * length
}

fn cubic_position(p0: DVec3, p1: DVec3, p2: DVec3, p3: DVec3, parameter: f64) -> DVec3 {
    let inverse = 1.0 - parameter;
    p0 * inverse.powi(3)
        + p1 * 3.0 * inverse.powi(2) * parameter
        + p2 * 3.0 * inverse * parameter.powi(2)
        + p3 * parameter.powi(3)
}

fn cubic_tangent(p0: DVec3, p1: DVec3, p2: DVec3, p3: DVec3, parameter: f64) -> DVec3 {
    let inverse = 1.0 - parameter;
    (p1 - p0) * 3.0 * inverse.powi(2)
        + (p2 - p1) * 6.0 * inverse * parameter
        + (p3 - p2) * 3.0 * parameter.powi(2)
}

fn tessellate_cubic(
    p0: DVec3,
    p1: DVec3,
    p2: DVec3,
    p3: DVec3,
    tolerance: f64,
    depth: u8,
    output: &mut Vec<DVec3>,
) -> Result<(), SplineError> {
    if depth >= MAX_TESSELLATION_DEPTH || cubic_flatness(p0, p1, p2, p3) <= tolerance {
        return push_evaluation_point(output, p3);
    }
    let p01 = (p0 + p1) * 0.5;
    let p12 = (p1 + p2) * 0.5;
    let p23 = (p2 + p3) * 0.5;
    let p012 = (p01 + p12) * 0.5;
    let p123 = (p12 + p23) * 0.5;
    let midpoint = (p012 + p123) * 0.5;
    tessellate_cubic(p0, p01, p012, midpoint, tolerance, depth + 1, output)?;
    tessellate_cubic(midpoint, p123, p23, p3, tolerance, depth + 1, output)
}

fn push_evaluation_point(output: &mut Vec<DVec3>, point: DVec3) -> Result<(), SplineError> {
    if output.len() >= MAX_EVALUATION_POINTS {
        return Err(SplineError::EvaluationLimitExceeded);
    }
    output.push(point);
    Ok(())
}

fn cubic_flatness(p0: DVec3, p1: DVec3, p2: DVec3, p3: DVec3) -> f64 {
    let chord = p3 - p0;
    let chord_length = chord.length();
    if chord_length <= f64::EPSILON {
        return p1.distance(p0).max(p2.distance(p0));
    }
    let distance = |point: DVec3| (point - p0).cross(chord).length() / chord_length;
    distance(p1).max(distance(p2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polyline_arc_length_and_resampling_are_uniform() {
        let spline = SplineResource::from_polyline(
            "L",
            &[[0.0, 0.0, 0.0], [3.0, 0.0, 0.0], [3.0, 4.0, 0.0]],
            false,
        );
        let table = spline.arc_length_table(DEFAULT_TOLERANCE).unwrap();
        assert!((table.total_length() - 7.0).abs() < 1.0e-9);
        let samples = table.resample(2.0).unwrap();
        assert_eq!(samples.first().unwrap().position, [0.0, 0.0, 0.0]);
        assert_eq!(samples.last().unwrap().position, [3.0, 4.0, 0.0]);
        for pair in samples.windows(2) {
            let distance =
                DVec3::from_array(pair[0].position).distance(DVec3::from_array(pair[1].position));
            assert!(distance <= 2.0 + 1.0e-9);
        }
    }

    #[test]
    fn cubic_evaluation_and_tangent_are_finite() {
        let mut spline = SplineResource::new("Bezier", SplineInterpolation::CubicBezier);
        spline
            .add_point(SplinePoint::with_handles(
                [0.0, 0.0, 0.0],
                [0.0; 3],
                [1.0, 1.0, 0.0],
                SplineHandleMode::Broken,
            ))
            .unwrap();
        spline
            .add_point(SplinePoint::with_handles(
                [2.0, 0.0, 0.0],
                [-1.0, 1.0, 0.0],
                [0.0; 3],
                SplineHandleMode::Broken,
            ))
            .unwrap();
        let midpoint = DVec3::from_array(spline.evaluate_segment(0, 0.5).unwrap());
        assert!((midpoint.x - 1.0).abs() < 1.0e-9);
        assert!(midpoint.y > 0.7);
        let tangent = DVec3::from_array(spline.tangent_segment(0, 0.5).unwrap());
        assert!((tangent.length() - 1.0).abs() < 1.0e-9);
    }

    #[test]
    fn reverse_preserves_bezier_geometry_with_opposite_parameterization() {
        let mut spline = SplineResource::new("Bezier", SplineInterpolation::CubicBezier);
        spline
            .add_point(SplinePoint::with_handles(
                [0.0, 0.0, 0.0],
                [0.0; 3],
                [1.0, 2.0, 0.0],
                SplineHandleMode::Broken,
            ))
            .unwrap();
        spline
            .add_point(SplinePoint::with_handles(
                [3.0, 0.0, 0.0],
                [-1.0, 1.0, 0.0],
                [0.0; 3],
                SplineHandleMode::Broken,
            ))
            .unwrap();
        let before = spline.evaluate_segment(0, 0.25).unwrap();
        spline.reverse();
        let after = spline.evaluate_segment(0, 0.75).unwrap();
        assert!(DVec3::from_array(before).distance(DVec3::from_array(after)) < 1.0e-9);
    }

    #[test]
    fn handle_modes_enforce_alignment_and_mirroring() {
        let mut point = SplinePoint::with_handles(
            [0.0; 3],
            [-2.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            SplineHandleMode::Aligned,
        );
        point.set_handle_out([0.0, 1.0, 0.0]).unwrap();
        assert_eq!(point.handle_in, [0.0, -2.0, 0.0]);
        point.handle_mode = SplineHandleMode::Mirrored;
        point.set_handle_out([1.0, 2.0, 0.0]).unwrap();
        assert_eq!(point.handle_in, [-1.0, -2.0, 0.0]);
    }

    #[test]
    fn cache_invalidates_only_when_spline_revision_changes() {
        let mut spline =
            SplineResource::from_polyline("Line", &[[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]], false);
        let point_id = spline.points[1].id;
        let mut cache = SplineEvaluationCache::default();
        let before = cache
            .samples_by_spacing(&spline, 0.5, DEFAULT_TOLERANCE)
            .unwrap()
            .to_vec();
        let same = cache
            .samples_by_spacing(&spline, 0.5, DEFAULT_TOLERANCE)
            .unwrap()
            .to_vec();
        assert_eq!(before, same);
        spline.move_point(point_id, [4.0, 0.0, 0.0]).unwrap();
        let after = cache
            .samples_by_spacing(&spline, 0.5, DEFAULT_TOLERANCE)
            .unwrap();
        assert_ne!(before.len(), after.len());
    }

    #[test]
    fn cache_rejects_same_revision_with_different_geometry_after_restore() {
        let mut spline =
            SplineResource::from_polyline("Line", &[[0.0, 0.0, 0.0], [2.0, 0.0, 0.0]], false);
        let mut cache = SplineEvaluationCache::default();
        let before = cache
            .samples_by_spacing(&spline, 0.5, DEFAULT_TOLERANCE)
            .unwrap()
            .to_vec();

        spline.points[1].position = [4.0, 0.0, 0.0];
        let after = cache
            .samples_by_spacing(&spline, 0.5, DEFAULT_TOLERANCE)
            .unwrap();

        assert_ne!(before.len(), after.len());
    }

    #[test]
    fn resampled_frames_survive_near_vertical_and_repeated_control_points() {
        let spline = SplineResource::from_polyline(
            "Vertical",
            &[
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 0.0],
                [0.0, 2.0, 0.01],
                [1.0, 4.0, 0.0],
            ],
            false,
        );
        let mut cache = SplineEvaluationCache::default();
        let frames = cache
            .frames_by_spacing(&spline, 0.5, DEFAULT_TOLERANCE)
            .unwrap();
        assert!(frames.len() >= 2);
        assert!(frames.iter().all(|frame| {
            frame.origin.is_finite()
                && frame.tangent.is_finite()
                && frame.normal.is_finite()
                && frame.binormal.is_finite()
        }));
    }

    #[test]
    fn snapping_prioritizes_nearby_points_then_grid() {
        let settings = SplineSnapSettings {
            grid_step: Some(1.0),
            point_radius: 0.25,
        };
        assert_eq!(
            snap_spline_position([0.9, 1.1, 0.0], settings, &[[1.0, 1.0, 0.0]]).unwrap(),
            [1.0, 1.0, 0.0]
        );
        assert_eq!(
            snap_spline_position([1.4, 1.6, 0.0], settings, &[]).unwrap(),
            [1.0, 2.0, 0.0]
        );
        assert_eq!(
            snap_spline_position(
                [0.0; 3],
                SplineSnapSettings {
                    grid_step: Some(0.0),
                    point_radius: 0.0,
                },
                &[],
            ),
            Err(SplineError::InvalidSnapSettings)
        );
    }

    #[test]
    fn evaluation_rejects_non_finite_and_unbounded_requests() {
        let mut spline =
            SplineResource::from_polyline("Line", &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0]], false);
        assert_eq!(
            spline.arc_length_table(f64::NAN),
            Err(SplineError::InvalidTolerance)
        );
        assert_eq!(
            spline.resample_by_spacing(f64::MIN_POSITIVE, DEFAULT_TOLERANCE),
            Err(SplineError::EvaluationLimitExceeded)
        );

        spline.points[0].position[0] = f64::NAN;
        assert_eq!(
            spline.arc_length_table(DEFAULT_TOLERANCE),
            Err(SplineError::NonFinite)
        );
    }

    #[test]
    fn spline_serialization_preserves_ids_and_authoring_data() {
        let mut spline = SplineResource::from_polyline(
            "Path",
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0]],
            true,
        );
        spline.interpolation = SplineInterpolation::CubicBezier;
        let json = serde_json::to_string(&spline).unwrap();
        let restored: SplineResource = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, spline);
    }
}
