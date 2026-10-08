//! Rectangle and circle use the same ToolSession as all viewport drags.
use petunia_project::profile::ProfilePrimitive;

use crate::{PetuniaViewport, SlintUiBridge, ToolGesture};

impl<V: PetuniaViewport> SlintUiBridge<V> {
    /// Starting or cancelling a shape clears geometry, not the chosen plane.
    pub(crate) fn clear_profile_draft(&mut self) {
        let previous = self.state.profile.clone();
        self.state.profile.clear();
        self.state.profile.origin = previous.origin;
        self.state.profile.right = previous.right;
        self.state.profile.up = previous.up;
        self.state.profile.normal = previous.normal;
        self.state.profile.snap = previous.snap;
    }

    pub(crate) fn begin_profile_primitive(
        &mut self,
        primitive: ProfilePrimitive,
        pixel: [f32; 2],
    ) -> bool {
        let [width, height] = self.viewport_size;
        if width <= 1.0 || height <= 1.0 || self.profile_pick_face {
            return false;
        }
        self.active_profile_id = None;
        self.profile_selected_point = None;
        self.clear_profile_draft();
        let ndc = [pixel[0] / width * 2.0 - 1.0, 1.0 - pixel[1] / height * 2.0];
        self.resolve_auto_workplane_at(ndc[0], ndc[1]);
        let Some(anchor) = self.profile_screen_to_plane(ndc[0], ndc[1]) else {
            return false;
        };
        self.tool_gesture = Some(ToolGesture::ProfilePrimitive { primitive, anchor });
        true
    }

    pub(crate) fn update_profile_primitive(
        &mut self,
        primitive: ProfilePrimitive,
        anchor: [f64; 2],
        pixel: [f32; 2],
    ) -> bool {
        let [width, height] = self.viewport_size;
        let Some(end) = self
            .profile_screen_to_plane(pixel[0] / width * 2.0 - 1.0, 1.0 - pixel[1] / height * 2.0)
        else {
            return false;
        };
        let points = match primitive {
            ProfilePrimitive::Rectangle => vec![
                [anchor[0], anchor[1]],
                [end[0], anchor[1]],
                [end[0], end[1]],
                [anchor[0], end[1]],
            ],
            ProfilePrimitive::Ellipse => {
                let center = anchor;
                let radius = (end[0] - anchor[0]).hypot(end[1] - anchor[1]);
                let segments = 16;
                (0..segments)
                    .map(|index| {
                        let angle = std::f64::consts::TAU * index as f64 / segments as f64;
                        [
                            center[0] + radius * angle.cos(),
                            center[1] + radius * angle.sin(),
                        ]
                    })
                    .collect()
            }
        };
        self.state.profile.points = points.iter().map(|point| point.map(|v| v as f32)).collect();
        self.state.profile.nodes.clear();
        self.state.profile.closed = true;
        self.state.mark_dirty();
        true
    }

    pub(crate) fn commit_profile_primitive(&mut self, primitive: ProfilePrimitive) -> bool {
        let points: Vec<_> = self
            .state
            .profile
            .points
            .iter()
            .map(|point| [f64::from(point[0]), f64::from(point[1]), 0.0])
            .collect();
        let Some(first) = points.first() else {
            return false;
        };
        let min = points.iter().fold([first[0], first[1]], |a, p| {
            [a[0].min(p[0]), a[1].min(p[1])]
        });
        let max = points.iter().fold([first[0], first[1]], |a, p| {
            [a[0].max(p[0]), a[1].max(p[1])]
        });
        if max[0] - min[0] < 1e-6 || max[1] - min[1] < 1e-6 {
            self.clear_profile_draft();
            return false;
        }
        let tool = self.state.session.tools.active_tool.clone();
        let name = match primitive {
            ProfilePrimitive::Rectangle => self.state.t("profile.rectangle"),
            ProfilePrimitive::Ellipse => self.state.t("profile.circle"),
        };
        let changed = self.replace_profile_shape(&name, points, primitive);
        // Creating another shape remains one drag away; selection and the card
        // describe the completed profile until the next creation gesture.
        self.state.session.tools.active_tool = tool;
        changed
    }
}
