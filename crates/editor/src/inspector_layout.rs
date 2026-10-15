//! Layout efêmero da coluna direita, separado de Document e Undo.

use petunia_core::Workspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorPane {
    Structure,
    Properties,
}

impl TryFrom<i32> for InspectorPane {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Structure),
            1 => Ok(Self::Properties),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InspectorPaneLayout {
    pub structure_ratio: f32,
    pub structure_collapsed: bool,
    pub properties_collapsed: bool,
}

impl InspectorPaneLayout {
    fn new(structure_ratio: f32) -> Self {
        Self {
            structure_ratio,
            structure_collapsed: false,
            properties_collapsed: false,
        }
    }
}

/// Slots tipados e limitados aos workspaces compilados; não é um registry de domínio.
#[derive(Debug, Clone)]
pub struct InspectorLayoutMemory {
    model: InspectorPaneLayout,
    paint: InspectorPaneLayout,
    uv: InspectorPaneLayout,
    #[cfg(feature = "animation-workspace")]
    animate: InspectorPaneLayout,
}

impl Default for InspectorLayoutMemory {
    fn default() -> Self {
        Self {
            model: InspectorPaneLayout::new(0.40),
            paint: InspectorPaneLayout::new(0.38),
            uv: InspectorPaneLayout::new(0.20),
            #[cfg(feature = "animation-workspace")]
            animate: InspectorPaneLayout::new(0.30),
        }
    }
}

impl InspectorLayoutMemory {
    fn slot(&self, workspace: Workspace) -> &InspectorPaneLayout {
        match workspace {
            Workspace::Model => &self.model,
            Workspace::Paint => &self.paint,
            Workspace::Uv => &self.uv,
            #[cfg(feature = "animation-workspace")]
            Workspace::Animate => &self.animate,
        }
    }

    fn slot_mut(&mut self, workspace: Workspace) -> &mut InspectorPaneLayout {
        match workspace {
            Workspace::Model => &mut self.model,
            Workspace::Paint => &mut self.paint,
            Workspace::Uv => &mut self.uv,
            #[cfg(feature = "animation-workspace")]
            Workspace::Animate => &mut self.animate,
        }
    }

    pub fn get(&self, workspace: Workspace) -> InspectorPaneLayout {
        *self.slot(workspace)
    }

    pub fn set_ratio(&mut self, workspace: Workspace, ratio: f32) -> bool {
        if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
            return false;
        }
        let next = ratio.clamp(0.15, 0.85);
        let slot = self.slot_mut(workspace);
        if (slot.structure_ratio - next).abs() < 0.001 {
            return false;
        }
        slot.structure_ratio = next;
        true
    }

    pub fn set_collapsed(
        &mut self,
        workspace: Workspace,
        pane: InspectorPane,
        collapsed: bool,
    ) -> bool {
        let slot = self.slot_mut(workspace);
        let value = match pane {
            InspectorPane::Structure => &mut slot.structure_collapsed,
            InspectorPane::Properties => &mut slot.properties_collapsed,
        };
        if *value == collapsed {
            return false;
        }
        *value = collapsed;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_ratios_never_change_layout_and_endpoints_keep_both_panes_reachable() {
        let mut memory = InspectorLayoutMemory::default();
        let original = memory.get(Workspace::Model);
        for value in [f32::NAN, f32::INFINITY, -0.1, 1.1] {
            assert!(!memory.set_ratio(Workspace::Model, value));
            assert_eq!(memory.get(Workspace::Model), original);
        }
        assert!(memory.set_ratio(Workspace::Model, 0.0));
        assert!((memory.get(Workspace::Model).structure_ratio - 0.15).abs() < f32::EPSILON);
        assert!(memory.set_ratio(Workspace::Model, 1.0));
        assert!((memory.get(Workspace::Model).structure_ratio - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn layout_is_restored_independently_for_every_compiled_workspace() {
        let mut memory = InspectorLayoutMemory::default();
        for (index, workspace) in Workspace::all().into_iter().enumerate() {
            assert!(memory.set_ratio(workspace, 0.5 + index as f32 * 0.05));
            let expected_changed = index % 2 == 0;
            assert_eq!(
                memory.set_collapsed(workspace, InspectorPane::Structure, expected_changed),
                expected_changed
            );
        }
        for (index, workspace) in Workspace::all().into_iter().enumerate() {
            let layout = memory.get(workspace);
            assert!((layout.structure_ratio - (0.5 + index as f32 * 0.05)).abs() < 0.001);
            assert_eq!(layout.structure_collapsed, index % 2 == 0);
            assert!(!layout.properties_collapsed);
        }
    }

    #[test]
    fn collapse_is_independent_and_idempotent() {
        let mut memory = InspectorLayoutMemory::default();
        assert!(memory.set_collapsed(Workspace::Paint, InspectorPane::Structure, true));
        assert!(!memory.set_collapsed(Workspace::Paint, InspectorPane::Structure, true));
        assert!(memory.set_collapsed(Workspace::Paint, InspectorPane::Properties, true));
        assert!(memory.set_collapsed(Workspace::Paint, InspectorPane::Structure, false));
        let layout = memory.get(Workspace::Paint);
        assert!(!layout.structure_collapsed && layout.properties_collapsed);
    }
}
