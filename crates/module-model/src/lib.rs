//! module-model — ferramentas de MODELAGEM (§8).
//!
//! Acoplar/desacoplar sem quebrar:
//! 1. crie `src/minha.rs` com `pub struct MinhaTool;` + `impl Tool`
//! 2. registre abaixo (`register::<MinhaTool>()`)
//! 3. `minha = true` em `assets/tools.toml` + textos em `assets/locales/*.toml`.

use std::collections::{HashMap, HashSet};

use petunia_config::tools::load_tools_config;
use petunia_core::AppState;

pub mod bevel;
pub mod connect;
pub mod dissolve;
pub mod draw_profile;
pub mod extrude;
pub mod inset;
pub mod merge;
pub mod mirror;
pub mod paint_tool;
pub mod primitives;
pub mod push_pull;
pub mod select;
pub mod slice;
pub mod subdivide;
pub mod symmetrize;
pub mod transform;

pub use bevel::BevelTool;
pub use connect::ConnectTool;
pub use dissolve::DissolveTool;
pub use draw_profile::{
    DrawProfileTool, GROUND_BIAS_MIN_PITCH_DEGREES, ProfileVolumeMode, build_extrude_mesh,
    build_revolve_mesh, build_sweep_mesh, extract_sweep_path_from_mesh, generate_extrude,
    generate_revolve, generate_sweep, profile_add_point, profile_align_camera_to_workplane,
    profile_begin_drag_node, profile_capture_auto, profile_capture_current, profile_capture_face,
    profile_capture_face_index, profile_capture_ground, profile_capture_view,
    profile_capture_world_plane_for_view, profile_clear_curves, profile_screen_to_plane,
    profile_set_circle, profile_set_curve_smoothness, profile_set_rectangle,
    profile_set_wall_thickness, profile_smooth_curves, profile_update_drag_handle,
    profile_workplane_label,
};
pub use extrude::ExtrudeTool;
pub use inset::InsetTool;
pub use merge::MergeTool;
pub use mirror::MirrorTool;
pub use paint_tool::PaintTool;
pub use primitives::PrimitivesTool;
pub use push_pull::PushPullTool;
pub use select::SelectTool;
pub use slice::SliceTool;
pub use subdivide::SubdivideTool;
pub use symmetrize::SymmetrizeTool;
pub use transform::TransformTool;

/// Contrato mínimo de ferramenta (pequeno de propósito).
pub trait Tool {
    fn id(&self) -> &'static str;
    fn label_key(&self) -> &'static str;
    fn hint_key(&self) -> &'static str;
    fn icon(&self) -> &'static str;
    fn shortcut(&self) -> &'static str {
        ""
    }
    /// Ativa a ferramenta; não altera geometria nem grava histórico.
    fn on_activate(&self, _state: &mut AppState) {}
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: Vec<Box<dyn Tool>>,
    ids: HashSet<String>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn register<T: Tool + Default + 'static>(&mut self, enabled: &HashMap<String, bool>) {
        let probe = T::default();
        let id = probe.id().to_string();
        let on = enabled.get(&id).copied().unwrap_or(true);
        if on && self.ids.insert(id) {
            self.tools.push(Box::new(probe));
        }
    }

    pub fn with_defaults() -> Self {
        let enabled = load_tools_config();
        let mut r = Self::new();
        r.register::<SelectTool>(&enabled);
        r.register::<TransformTool>(&enabled);
        r.register::<PrimitivesTool>(&enabled);
        r.register::<DrawProfileTool>(&enabled);
        r.register::<ExtrudeTool>(&enabled);
        r.register::<PushPullTool>(&enabled);
        r.register::<InsetTool>(&enabled);
        r.register::<BevelTool>(&enabled);
        r.register::<SubdivideTool>(&enabled);
        r.register::<SliceTool>(&enabled);
        r.register::<ConnectTool>(&enabled);
        r.register::<DissolveTool>(&enabled);
        r.register::<MergeTool>(&enabled);
        r.register::<PaintTool>(&enabled);
        r
    }

    pub fn all(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }

    pub fn get(&self, id: &str) -> Option<&dyn Tool> {
        self.tools.iter().find(|t| t.id() == id).map(|t| t.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activating_any_model_tool_preserves_geometry_and_history() {
        let tools: Vec<Box<dyn Tool>> = vec![
            Box::new(SelectTool),
            Box::new(TransformTool),
            Box::new(PrimitivesTool),
            Box::new(DrawProfileTool),
            Box::new(ExtrudeTool),
            Box::new(PushPullTool),
            Box::new(InsetTool),
            Box::new(BevelTool),
            Box::new(SubdivideTool),
            Box::new(SliceTool),
            Box::new(ConnectTool),
            Box::new(DissolveTool),
            Box::new(MirrorTool),
            Box::new(MergeTool),
            Box::new(SymmetrizeTool),
            Box::new(PaintTool),
        ];
        for tool in tools {
            let mut state = AppState::new("en");
            state.project.active_mesh_mut().unwrap().select_all();
            let before = state.project.active_mesh().unwrap().to_obj();
            let vertices = state.project.active_mesh().unwrap().verts.len();
            let faces = state.project.active_mesh().unwrap().faces.len();
            tool.on_activate(&mut state);
            let after = state.project.active_mesh().unwrap();
            assert_eq!(
                after.to_obj(),
                before,
                "{} mutates on activation",
                tool.id()
            );
            assert_eq!(after.verts.len(), vertices, "{} adds vertices", tool.id());
            assert_eq!(after.faces.len(), faces, "{} adds faces", tool.id());
            assert_eq!(
                state.project.undo.depth(),
                (0, 0),
                "{} pollutes history",
                tool.id()
            );
        }
    }
}
