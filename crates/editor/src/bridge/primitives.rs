//! Primitivas paramétricas e modos de extrusão/bevel.

use crate::*;

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub fn toggle_bevel_clamp_overlap(&mut self) -> bool {
        self.state.tools.bevel_clamp_overlap = !self.state.tools.bevel_clamp_overlap;
        let enabled = self.state.tools.bevel_clamp_overlap;
        self.state.set_status(if enabled {
            "Round Edge: Clamp Overlap ativado"
        } else {
            "Round Edge: Clamp Overlap desativado"
        });
        self.state.render.mark_dirty();
        enabled
    }

    pub fn toggle_bevel_affect_vertices(&mut self) -> bool {
        self.state.tools.bevel_affect_vertices = !self.state.tools.bevel_affect_vertices;
        let enabled = self.state.tools.bevel_affect_vertices;
        self.state.set_status(if enabled {
            "Round Edge: Point (Vertex) Mode ativado"
        } else {
            "Round Edge: Point (Vertex) Mode desativado"
        });
        self.state.render.mark_dirty();
        enabled
    }

    pub fn active_primitive_descriptor_or_session(
        &self,
    ) -> Option<petunia_core::PrimitiveDescriptor> {
        if let Some(session) = &self.state.session.primitive_session {
            Some(session.descriptor)
        } else {
            self.state.active_primitive_descriptor()
        }
    }

    pub fn freeze_active_primitive(&mut self) -> bool {
        let frozen = self.state.freeze_active_primitive();
        if frozen {
            self.state.render.mark_dirty();
        }
        frozen
    }

    pub fn update_primitive_param_float(&mut self, param: &str, value: f32) -> bool {
        if !value.is_finite() {
            return false;
        }
        if [
            "sides",
            "vertices",
            "major_segments",
            "minor_segments",
            "segments",
            "rings",
            "subdiv",
        ]
        .contains(&param)
            && (value.fract() != 0.0 || !(0.0..=512.0).contains(&value))
        {
            return false;
        }
        let Some(desc) = self.active_primitive_descriptor_or_session() else {
            return false;
        };
        use petunia_core::PrimitiveDescriptor::*;
        let new_desc = match desc {
            Box {
                width,
                height,
                depth,
            } => match param {
                "width" => Box {
                    width: value,
                    height,
                    depth,
                },
                "height" => Box {
                    width,
                    height: value,
                    depth,
                },
                "depth" => Box {
                    width,
                    height,
                    depth: value,
                },
                _ => return false,
            },
            Plane { width, height } => match param {
                "width" => Plane {
                    width: value,
                    height,
                },
                "height" => Plane {
                    width,
                    height: value,
                },
                _ => return false,
            },
            Wedge {
                width,
                height,
                depth,
            } => match param {
                "width" => Wedge {
                    width: value,
                    height,
                    depth,
                },
                "height" => Wedge {
                    width,
                    height: value,
                    depth,
                },
                "depth" => Wedge {
                    width,
                    height,
                    depth: value,
                },
                _ => return false,
            },
            Cylinder {
                radius,
                height,
                sides,
                cap_top,
                cap_bottom,
            } => match param {
                "radius" => Cylinder {
                    radius: value,
                    height,
                    sides,
                    cap_top,
                    cap_bottom,
                },
                "height" => Cylinder {
                    radius,
                    height: value,
                    sides,
                    cap_top,
                    cap_bottom,
                },
                "sides" => Cylinder {
                    radius,
                    height,
                    sides: value as u32,
                    cap_top,
                    cap_bottom,
                },
                _ => return false,
            },
            Cone {
                bottom_radius,
                top_radius,
                height,
                sides,
                cap_bottom,
                cap_top,
            } => match param {
                "radius" => Cone {
                    bottom_radius: value,
                    top_radius,
                    height,
                    sides,
                    cap_bottom,
                    cap_top,
                },
                "radius_b" => Cone {
                    bottom_radius,
                    top_radius: value,
                    height,
                    sides,
                    cap_bottom,
                    cap_top,
                },
                "height" => Cone {
                    bottom_radius,
                    top_radius,
                    height: value,
                    sides,
                    cap_bottom,
                    cap_top,
                },
                "sides" => Cone {
                    bottom_radius,
                    top_radius,
                    height,
                    sides: value as u32,
                    cap_bottom,
                    cap_top,
                },
                _ => return false,
            },
            Circle {
                radius,
                vertices,
                fill,
            } => match param {
                "radius" => Circle {
                    radius: value,
                    vertices,
                    fill,
                },
                "sides" => Circle {
                    radius,
                    vertices: value as u32,
                    fill,
                },
                _ => return false,
            },
            Torus {
                major_radius,
                minor_radius,
                major_segments,
                minor_segments,
            } => match param {
                "radius" => Torus {
                    major_radius: value,
                    minor_radius,
                    major_segments,
                    minor_segments,
                },
                "radius_b" => Torus {
                    major_radius,
                    minor_radius: value,
                    major_segments,
                    minor_segments,
                },
                "sides" => Torus {
                    major_radius,
                    minor_radius,
                    major_segments: value as u32,
                    minor_segments,
                },
                "rings" => Torus {
                    major_radius,
                    minor_radius,
                    major_segments,
                    minor_segments: value as u32,
                },
                _ => return false,
            },
            LowSphere {
                radius,
                segments,
                rings,
            } => match param {
                "radius" => LowSphere {
                    radius: value,
                    segments,
                    rings,
                },
                "sides" => LowSphere {
                    radius,
                    segments: value as u32,
                    rings,
                },
                "rings" => LowSphere {
                    radius,
                    segments,
                    rings: value as u32,
                },
                _ => return false,
            },
            Icosphere { radius, subdiv } => match param {
                "radius" => Icosphere {
                    radius: value,
                    subdiv,
                },
                "subdiv" => Icosphere {
                    radius,
                    subdiv: value as u32,
                },
                _ => return false,
            },
            Capsule {
                radius,
                height,
                radial_segments,
                cap_segments,
            } => match param {
                "radius" => Capsule {
                    radius: value,
                    height,
                    radial_segments,
                    cap_segments,
                },
                "height" => Capsule {
                    radius,
                    height: value,
                    radial_segments,
                    cap_segments,
                },
                "sides" => Capsule {
                    radius,
                    height,
                    radial_segments: value as u32,
                    cap_segments,
                },
                "rings" => Capsule {
                    radius,
                    height,
                    radial_segments,
                    cap_segments: value as u32,
                },
                _ => return false,
            },
        };
        if let Err(error) =
            petunia_project::geometry_generator::validate_primitive_descriptor(new_desc)
        {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state
            .update_active_primitive(new_desc, !self.state.primitive_session_valid())
    }

    pub fn update_primitive_param_bool(&mut self, param: &str, value: bool) -> bool {
        let Some(desc) = self.active_primitive_descriptor_or_session() else {
            return false;
        };
        use petunia_core::PrimitiveDescriptor::*;
        let new_desc = match desc {
            Cylinder {
                radius,
                height,
                sides,
                cap_top,
                cap_bottom,
            } => match param {
                "cap_top" => Cylinder {
                    radius,
                    height,
                    sides,
                    cap_top: value,
                    cap_bottom,
                },
                "cap_bottom" => Cylinder {
                    radius,
                    height,
                    sides,
                    cap_top,
                    cap_bottom: value,
                },
                _ => return false,
            },
            Cone {
                bottom_radius,
                top_radius,
                height,
                sides,
                cap_bottom,
                cap_top,
            } => match param {
                "cap_top" => Cone {
                    bottom_radius,
                    top_radius,
                    height,
                    sides,
                    cap_bottom,
                    cap_top: value,
                },
                "cap_bottom" => Cone {
                    bottom_radius,
                    top_radius,
                    height,
                    sides,
                    cap_bottom: value,
                    cap_top,
                },
                _ => return false,
            },
            Circle {
                radius, vertices, ..
            } => match param {
                "fill_disc" => Circle {
                    radius,
                    vertices,
                    fill: if value {
                        petunia_core::CircleFill::Disc
                    } else {
                        petunia_core::CircleFill::None
                    },
                },
                _ => return false,
            },
            _ => return false,
        };
        if let Err(error) =
            petunia_project::geometry_generator::validate_primitive_descriptor(new_desc)
        {
            self.state.set_status(error.to_string());
            return false;
        }
        self.state
            .update_active_primitive(new_desc, !self.state.primitive_session_valid())
    }

    pub fn confirm_primitive(&mut self) -> bool {
        self.state.confirm_primitive()
    }

    pub fn cancel_primitive(&mut self) -> bool {
        self.state.cancel_primitive()
    }

    /// Alterna o modo de extrusão entre Região e Faces Individuais mantendo a distância.
    pub fn switch_extrude_mode(&mut self) -> bool {
        let Some(current) = self.tool_modal else {
            return false;
        };
        let new_kind = match current {
            ToolModalKind::Extrude => ToolModalKind::ExtrudeIndividual,
            ToolModalKind::ExtrudeIndividual => ToolModalKind::Extrude,
            _ => return false,
        };
        let current_value = self.tool_modal_value;
        let _ = self.state.cancel_modal();
        if self.begin_tool_modal(new_kind) {
            self.tool_modal_value = current_value;
            let _ = self.state.update_modal(glam::Vec3::ZERO, current_value);
            self.state.mark_dirty();
            true
        } else {
            false
        }
    }
}
