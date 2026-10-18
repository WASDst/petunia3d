//! Editor-wide construction frame, independent of any drawing tool or viewport camera.
use petunia_project::{ProfileError, Workplane};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkplaneMode {
    #[default]
    Auto,
    Ground,
    View,
    Face,
    Selection,
    Custom,
    Reference,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkplaneSession {
    pub mode: WorkplaneMode,
    pub plane: Workplane,
    pub locked: bool,
}

impl Default for WorkplaneSession {
    fn default() -> Self {
        Self {
            mode: WorkplaneMode::Auto,
            plane: Workplane {
                origin: [0.0; 3],
                right: [1.0, 0.0, 0.0],
                up: [0.0, 0.0, -1.0],
                normal: [0.0, 1.0, 0.0],
            },
            locked: false,
        }
    }
}

impl WorkplaneSession {
    pub fn set(&mut self, mode: WorkplaneMode, plane: Workplane) -> Result<(), ProfileError> {
        let plane = plane.try_normalized()?;
        *self = Self {
            mode,
            plane,
            locked: mode != WorkplaneMode::Auto,
        };
        Ok(())
    }
}
