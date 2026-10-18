//! Authoring previews live in the controller, outside the document and Undo.
use crate::bridge::tool_preview::TOOL_PREVIEW_REVISION_BIT;
use crate::*;
use petunia_project::PoseOverride;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub(crate) struct ProfileAuthoringPreview {
    pub profile: petunia_project::PlanarShape,
    pub spline: petunia_project::SplineResource,
    pub inner: Vec<petunia_project::SplineResource>,
    pub source: Option<(u64, u64)>,
    pub previous_shape: Option<uuid::Uuid>,
    pub evaluation_project: Box<petunia_project::Project>,
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn begin_profile_authoring_preview(&mut self) -> bool {
        if self.profile_authoring_preview.is_some() {
            return true;
        }
        let Some((profile, spline)) = self.active_profile_resources() else {
            return false;
        };
        if profile.locked || !profile.visible {
            self.state
                .set_status(self.state.t_id(petunia_config::TextId::new(
                    "draw_status.shape_is_locked_or_hidden",
                )));
            return false;
        }
        let inner = profile
            .hole_splines
            .iter()
            .filter_map(|id| self.state.project.project.get_spline(*id).cloned())
            .collect();
        self.profile_authoring_preview = Some(ProfileAuthoringPreview {
            profile: profile.clone(),
            spline: spline.clone(),
            inner,
            source: Some((profile.revision, spline.revision)),
            evaluation_project: Box::new(self.state.project.project.clone()),
            previous_shape: self.state.session.active_shape,
        });
        true
    }

    pub(crate) fn commit_profile_authoring_preview(&mut self) -> bool {
        let Some(draft) = self.profile_authoring_preview.take() else {
            return false;
        };
        self.clear_tool_preview();
        let result = if let Some((profile_revision, spline_revision)) = draft.source {
            self.state.dispatch(&petunia_core::EditPlanarShapeCmd {
                profile: draft.profile,
                spline: draft.spline,
                inner: draft.inner,
                expected_profile_revision: profile_revision,
                expected_spline_revision: spline_revision,
            })
        } else {
            self.state.dispatch(&petunia_core::CreateProfileCmd {
                profile: draft.profile,
                spline: draft.spline,
            })
        };
        match result {
            Ok(()) => {
                self.state.mark_dirty();
                true
            }
            Err(error) => {
                self.state.set_status(error.to_string());
                false
            }
        }
    }

    pub(crate) fn cancel_profile_authoring_preview(&mut self) -> bool {
        let Some(draft) = self.profile_authoring_preview.take() else {
            return false;
        };
        self.clear_tool_preview();
        if draft.source.is_none() {
            self.state.session.active_shape = draft.previous_shape;
        }
        self.state.mark_dirty();
        true
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn active_contour_resources(
        &self,
    ) -> Option<(
        &petunia_project::PlanarShape,
        &petunia_project::SplineResource,
    )> {
        let (profile, outer) = self.active_profile_resources()?;
        let Some(id) = self
            .state
            .session
            .planar_selection
            .contour
            .filter(|id| *id != outer.id)
        else {
            return Some((profile, outer));
        };
        if !profile.hole_splines.contains(&id) {
            return Some((profile, outer));
        }
        let contour = self
            .profile_authoring_preview
            .as_ref()
            .and_then(|d| d.inner.iter().find(|s| s.id == id))
            .or_else(|| self.state.project.project.get_spline(id))?;
        Some((profile, contour))
    }

    pub(crate) fn authoring_contour_mut(
        &mut self,
        id: uuid::Uuid,
    ) -> Option<&mut petunia_project::SplineResource> {
        let draft = self.profile_authoring_preview.as_mut()?;
        if draft.spline.id == id {
            Some(&mut draft.spline)
        } else {
            draft.inner.iter_mut().find(|s| s.id == id)
        }
    }

    pub fn select_draw_contour(&mut self, id: uuid::Uuid) -> bool {
        if self.profile_authoring_preview.is_some() || self.profile_volume_mode.is_some() {
            return false;
        }
        let Some((shape, outer)) = self.active_profile_resources() else {
            return false;
        };
        if id != outer.id && !shape.hole_splines.contains(&id) {
            return false;
        }
        self.state.session.planar_selection.contour = Some(id);
        self.state.session.planar_selection.point = None;
        self.state.session.planar_selection.segment = None;
        self.state.mark_dirty();
        true
    }
}

impl<V: PetuniaViewport> SlintUiBridge<V> {
    pub(crate) fn refresh_profile_authoring_preview(&mut self) {
        let Some(draft) = self.profile_authoring_preview.as_mut() else {
            return;
        };
        let candidate = &mut draft.evaluation_project;
        if let Some(profile) = candidate.get_profile_mut(draft.profile.id) {
            *profile = draft.profile.clone();
        }
        if let Some(spline) = candidate.get_spline_mut(draft.spline.id) {
            *spline = draft.spline.clone();
        }
        for contour in &draft.inner {
            if let Some(target) = candidate.get_spline_mut(contour.id) {
                *target = contour.clone();
            }
        }
        self.tool_preview_revision = self.tool_preview_revision.wrapping_add(1);
        let mut preview = PoseOverride::new(TOOL_PREVIEW_REVISION_BIT | self.tool_preview_revision);
        for asset in &candidate.assets {
            if let Some(generator) = asset.geometry_source.as_ref().filter(|g| {
                g.depends_on_shape(draft.profile.id)
                    || g.depends_on_spline(draft.spline.id)
                    || draft
                        .profile
                        .hole_splines
                        .iter()
                        .any(|id| g.depends_on_spline(*id))
            }) {
                preview.insert(
                    asset.id,
                    match generator.evaluate(candidate) {
                        Ok(mesh) => mesh,
                        Err(error) => {
                            self.state.set_status(error.to_string());
                            petunia_core::Mesh::default()
                        }
                    },
                );
            }
        }
        self.tool_preview = Some(Arc::new(preview));
        self.state.mark_dirty();
    }
}
