//! Operações adicionais de DRAW usando comandos e controles numéricos existentes.
use super::*;
use petunia_project::profile_tools;
#[derive(Clone, Copy)]
pub(crate) struct DrawParameters {
    pub radius: f64,
    pub sides: usize,
    pub tolerance: f64,
    pub length: f64,
    pub angle: f64,
    pub constrain: bool,
    pub mirror: bool,
    pub export_padding: u32,
    pub trace_threshold: u8,
}
impl Default for DrawParameters {
    fn default() -> Self {
        Self {
            radius: 0.1,
            sides: 6,
            tolerance: 0.01,
            length: 1.0,
            angle: 0.0,
            constrain: false,
            mirror: false,
            export_padding: 2,
            trace_threshold: 128,
        }
    }
}
pub(crate) const DRAW_ACTIONS: &[(&str, &str)] = &[
    ("draw.round_corners", "extensions.round"),
    ("draw.simplify", "extensions.simplify"),
    ("draw.resample", "extensions.resample"),
    ("draw.regular_polygon", "extensions.polygon"),
    ("draw.rounded_rectangle", "extensions.rectangle"),
    ("draw.ellipse", "extensions.ellipse"),
    ("draw.arc_three_points", "extensions.arc"),
    ("draw.slot", "extensions.slot"),
    ("draw.mirror_creation", "extensions.mirror"),
    ("draw.segment_dimensions", "extensions.dimensions"),
    ("draw.trace_reference", "extensions.trace"),
];
impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn execute_draw_extension(&mut self, id: &str) -> bool {
        if !DRAW_ACTIONS.iter().any(|(key, _)| *key == id)
            || self.modeling_mode != ModelingMode::Draw
            || self.state.workspace != Workspace::Model
            || self.profile_volume_mode.is_some()
            || self.state.modal.is_some()
        {
            return false;
        }
        let params = self.draw_parameters;
        let name = self
            .state
            .t_id(petunia_config::TextId::new("draw.shape_name"));
        match id {
            "draw.mirror_creation" => {
                self.draw_parameters.mirror = !params.mirror;
            }
            "draw.segment_dimensions" => {
                self.draw_parameters.constrain = !params.constrain;
            }
            "draw.regular_polygon" => {
                let points = profile_tools::regular_polygon(params.length, params.sides);
                self.create_extension_shape(&name, points, true);
            }
            "draw.rounded_rectangle" => {
                let points = profile_tools::rounded_rectangle(
                    params.length,
                    params.length * 0.5,
                    params.radius,
                );
                self.create_extension_shape(&name, points, true);
            }
            "draw.ellipse" => {
                let points = profile_tools::ellipse(
                    params.length,
                    params.length * 0.5,
                    params.sides.max(12),
                );
                self.create_extension_shape(&name, points, true);
            }
            "draw.slot" => {
                let points = profile_tools::rounded_rectangle(
                    params.length,
                    params.length * 0.4,
                    params.length * 0.2,
                );
                self.create_extension_shape(&name, points, true);
            }
            "draw.trace_reference" => {
                let Some(reference) = self.state.project.refs.iter().find(|r| r.visible).cloned()
                else {
                    return false;
                };
                let mut mask = reference.rgba.clone();
                if mask.chunks_exact(4).all(|p| p[3] == 255) {
                    for p in mask.chunks_exact_mut(4) {
                        let luma =
                            0.2126 * p[0] as f32 + 0.7152 * p[1] as f32 + 0.0722 * p[2] as f32;
                        p[3] = if luma <= params.trace_threshold as f32 {
                            255
                        } else {
                            0
                        };
                    }
                }
                let rings = profile_tools::trace_alpha(
                    reference.width,
                    reference.height,
                    &mask,
                    if reference.rgba.chunks_exact(4).all(|p| p[3] == 255) {
                        128
                    } else {
                        params.trace_threshold
                    },
                    params.tolerance * reference.width.max(reference.height) as f64,
                );
                let scale =
                    reference.size as f64 / reference.width.max(reference.height).max(1) as f64;
                let mut regions: Vec<petunia_mesh::arrangement::Region2> = rings
                    .iter()
                    .filter(|ring| petunia_mesh::arrangement::signed_area(ring) > 0.0)
                    .map(|ring| petunia_mesh::arrangement::Region2 {
                        outer: ring.clone(),
                        holes: vec![],
                    })
                    .collect();
                for ring in rings
                    .iter()
                    .filter(|ring| petunia_mesh::arrangement::signed_area(ring) < 0.0)
                {
                    let parent = regions
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| {
                            petunia_mesh::arrangement::point_in_polygon(&r.outer, ring[0])
                        })
                        .min_by(|(_, a), (_, b)| a.area().total_cmp(&b.area()))
                        .map(|(i, _)| i);
                    if let Some(i) = parent {
                        regions[i].holes.push(ring.clone());
                    }
                }
                let workplane = self.draft_profile_workplane();
                let profiles = regions
                    .into_iter()
                    .map(|region| {
                        let points: Vec<_> = region
                            .outer
                            .iter()
                            .map(|p| [p[0] * scale, p[1] * scale, 0.0])
                            .collect();
                        let spline =
                            petunia_project::SplineResource::from_polyline(&name, &points, true);
                        let mut profile =
                            petunia_project::ProfileResource::new(&name, spline.id, workplane);
                        profile.holes = region
                            .holes
                            .into_iter()
                            .map(|ring| {
                                ring.into_iter()
                                    .map(|p| [p[0] * scale, p[1] * scale])
                                    .collect()
                            })
                            .collect();
                        (spline, profile)
                    })
                    .collect();
                if let Err(error) = self.state.dispatch(&petunia_core::DrawBatchCmd {
                    update: None,
                    profiles,
                }) {
                    self.state.set_status(error.to_string());
                    return false;
                }
            }
            "draw.round_corners" | "draw.simplify" | "draw.resample" | "draw.arc_three_points" => {
                let Some((_, spline)) = self.active_profile_resources() else {
                    return false;
                };
                let mut spline = spline.clone();
                let points: Vec<_> = spline
                    .points
                    .iter()
                    .map(|p| [p.position[0], p.position[1]])
                    .collect();
                let result = match id {
                    "draw.round_corners" => {
                        if !spline.closed {
                            return false;
                        }
                        profile_tools::round_corners_at(
                            &points,
                            params.radius,
                            6,
                            self.profile_selected_point
                                .and_then(|id| spline.points.iter().position(|p| p.id == id)),
                        )
                    }
                    "draw.resample" => {
                        let Ok(table) = spline.arc_length_table(params.tolerance * 0.25) else {
                            return false;
                        };
                        let n = params.sides;
                        let count = if spline.closed { n } else { n + 1 };
                        let Ok(points) = (0..count)
                            .map(|i| {
                                table
                                    .sample_at_distance(table.total_length() * i as f64 / n as f64)
                                    .map(|p| [p.position[0], p.position[1]])
                            })
                            .collect::<Result<Vec<_>, _>>()
                        else {
                            return false;
                        };
                        points
                    }
                    "draw.simplify" => {
                        let Ok(table) = spline.arc_length_table(params.tolerance * 0.25) else {
                            return false;
                        };
                        let points: Vec<_> =
                            table.polyline().into_iter().map(|p| [p[0], p[1]]).collect();
                        profile_tools::simplify(&points, params.tolerance, spline.closed)
                    }
                    _ => {
                        if points.len() < 3 {
                            return false;
                        }
                        spline.closed = false;
                        profile_tools::arc_three_points(
                            points[0],
                            points[1],
                            points[2],
                            params.sides.max(12),
                        )
                    }
                };
                spline.points = result
                    .into_iter()
                    .map(|p| petunia_project::SplinePoint::new([p[0], p[1], 0.0]))
                    .collect();
                spline.interpolation = petunia_project::SplineInterpolation::Polyline;
                if let Err(error) = self
                    .state
                    .dispatch(&petunia_core::UpdateSplineCmd { spline })
                {
                    self.state.set_status(error.to_string());
                    return false;
                }
            }
            _ => {}
        }
        self.state.mark_dirty();
        true
    }
    fn create_extension_shape(&mut self, name: &str, points: Vec<[f64; 2]>, closed: bool) -> bool {
        if points.len() < 3 {
            return false;
        }
        let points3: Vec<_> = points.iter().map(|p| [p[0], p[1], 0.0]).collect();
        let spline = petunia_project::SplineResource::from_polyline(name, &points3, closed);
        let profile =
            petunia_project::ProfileResource::new(name, spline.id, self.draft_profile_workplane());
        let id = profile.id;
        let mut profiles = vec![(spline, profile)];
        if self.draw_parameters.mirror {
            let (spline, profile) = &profiles[0];
            profiles.push(mirrored_profile(profile, spline));
        }
        if let Err(error) = self.state.dispatch(&petunia_core::DrawBatchCmd {
            update: None,
            profiles,
        }) {
            self.state.set_status(error.to_string());
            return false;
        }
        self.active_profile_id = Some(id);
        self.state.session.tools.active_tool = "draw_profile".into();
        true
    }
    pub(crate) fn close_profile_with_mirror(&mut self) -> bool {
        let Some((profile, spline)) = self.active_profile_resources() else {
            return false;
        };
        let mut spline = spline.clone();
        spline.closed = true;
        let profiles = if self.draw_parameters.mirror {
            vec![mirrored_profile(profile, &spline)]
        } else {
            vec![]
        };
        self.state
            .dispatch(&petunia_core::DrawBatchCmd {
                update: Some(spline),
                profiles,
            })
            .is_ok()
    }
    pub(crate) fn set_extension_parameter(&mut self, key: &str, text: &str) -> bool {
        let Ok(value) = numeric::parse_numeric(text) else {
            return false;
        };
        let value = f64::from(value);
        if !value.is_finite() {
            return false;
        }
        match key {
            "threshold" => {
                self.draw_parameters.trace_threshold = value.round().clamp(0.0, 254.0) as u8
            }
            "padding" => {
                self.draw_parameters.export_padding = value.round().clamp(0.0, 64.0) as u32
            }
            "radius" => self.draw_parameters.radius = value.clamp(0.0001, 10000.0),
            "sides" => self.draw_parameters.sides = value.round().clamp(3.0, 512.0) as usize,
            "tolerance" => self.draw_parameters.tolerance = value.clamp(0.000001, 100.0),
            "length" => self.draw_parameters.length = value.clamp(0.0001, 10000.0),
            "angle" => self.draw_parameters.angle = value.rem_euclid(360.0),
            _ => return false,
        }
        self.state.mark_dirty();
        true
    }
}

fn mirrored_profile(
    profile: &petunia_project::ProfileResource,
    spline: &petunia_project::SplineResource,
) -> (
    petunia_project::SplineResource,
    petunia_project::ProfileResource,
) {
    let mut copy = spline.clone();
    copy.id = uuid::Uuid::new_v4();
    for p in &mut copy.points {
        p.id = uuid::Uuid::new_v4();
        p.position[0] = -p.position[0];
        p.handle_in[0] = -p.handle_in[0];
        p.handle_out[0] = -p.handle_out[0];
    }
    let mut profile = profile.clone();
    profile.id = uuid::Uuid::new_v4();
    profile.spline_id = copy.id;
    for hole in &mut profile.holes {
        for p in hole.iter_mut() {
            p[0] = -p[0];
        }
        hole.reverse();
    }
    (copy, profile)
}
