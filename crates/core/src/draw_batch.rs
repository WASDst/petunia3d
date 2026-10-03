//! Um gesto DRAW pode produzir vários perfis (espelho, SVG, vetorização).
use crate::{AppState, Command, CommandError, CreateProfileCmd, UpdateSplineCmd};
use petunia_project::{ProfileResource, ProjectChanges, SplineResource};
#[derive(Clone)]
pub struct DrawBatchCmd {
    pub update: Option<SplineResource>,
    pub profiles: Vec<(SplineResource, ProfileResource)>,
}
impl Command for DrawBatchCmd {
    fn label(&self) -> &'static str {
        "draw profiles"
    }
    fn changes(&self) -> ProjectChanges {
        ProjectChanges::SPLINES | ProjectChanges::PROCEDURAL
    }
    fn execute(&self, state: &mut AppState) -> Result<(), CommandError> {
        if let Some(spline) = &self.update {
            UpdateSplineCmd {
                spline: spline.clone(),
            }
            .execute(state)?;
        }
        for (spline, profile) in &self.profiles {
            CreateProfileCmd {
                spline: spline.clone(),
                profile: profile.clone(),
            }
            .execute(state)?;
        }
        Ok(())
    }
    fn can_execute(&self, state: &AppState) -> Result<(), &'static str> {
        if self.profiles.len() > 2000 {
            return Err("Too many profiles");
        }
        if let Some(spline) = &self.update {
            UpdateSplineCmd {
                spline: spline.clone(),
            }
            .can_execute(state)?;
        }
        let mut splines = std::collections::HashSet::new();
        let mut profiles = std::collections::HashSet::new();
        for (spline, profile) in &self.profiles {
            if !splines.insert(spline.id) || !profiles.insert(profile.id) {
                return Err("Duplicate profile identifiers");
            }
            CreateProfileCmd {
                spline: spline.clone(),
                profile: profile.clone(),
            }
            .can_execute(state)?;
        }
        if self.update.is_none() && self.profiles.is_empty() {
            return Err("No profiles to create");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petunia_project::{ProfileResource, ProfileWorkplane, SplineResource};
    fn profile() -> (SplineResource, ProfileResource) {
        let spline = SplineResource::from_polyline(
            "Shape",
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            true,
        );
        let profile = ProfileResource::new("Shape", spline.id, ProfileWorkplane::default());
        (spline, profile)
    }
    #[test]
    fn imported_batch_is_one_undo() {
        let mut state = AppState::default();
        let depth = state.project.undo.depth().0;
        state
            .dispatch(&DrawBatchCmd {
                update: None,
                profiles: vec![profile(), profile()],
            })
            .unwrap();
        assert_eq!(state.project.profiles.len(), 2);
        assert_eq!(state.project.undo.depth().0, depth + 1);
        state.undo();
        assert!(state.project.profiles.is_empty());
        assert!(state.project.splines.is_empty());
    }
    #[test]
    fn invalid_or_duplicate_profile_does_not_leave_partial_batch() {
        let mut state = AppState::default();
        let p = profile();
        let before = state.project.project.clone();
        assert!(
            state
                .dispatch(&DrawBatchCmd {
                    update: None,
                    profiles: vec![p.clone(), p]
                })
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(&state.project.project).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
        let mut p = profile();
        p.0.points[0].position[0] = f64::NAN;
        assert!(
            state
                .dispatch(&DrawBatchCmd {
                    update: None,
                    profiles: vec![profile(), p]
                })
                .is_err()
        );
        assert_eq!(
            serde_json::to_value(&state.project.project).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
    }
}
