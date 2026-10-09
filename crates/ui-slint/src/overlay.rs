//! Política de overlays temporários independente do toolkit.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Tooltip,
    Popover,
    ContextMenu,
    FloatingPanel,
    Drawer,
    Modal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverlayId {
    CommandPalette,
    Settings,
    SceneDrawer,
    AssetLibrary,
    OutlinerContextMenu,
    /// Menu da viewport (botão direito no 3D): id próprio para o Escape
    /// LIFO fechar o menu certo quando os dois modos competem.
    ContextMenu,
    MenuBar,
    PivotMenu,
    MicroInspector,
    ReferenceManager,
    AddMenu,
}

impl OverlayId {
    /// Identificador de apresentação; não pertence ao documento persistente.
    pub fn presentation_id(self) -> &'static str {
        match self {
            Self::CommandPalette => "command_palette",
            Self::Settings => "settings",
            Self::SceneDrawer => "scene_drawer",
            Self::AssetLibrary => "asset_library",
            Self::OutlinerContextMenu => "outliner_context",
            Self::ContextMenu => "context",
            Self::MenuBar => "menu",
            Self::PivotMenu => "pivot",
            Self::MicroInspector => "micro_inspector",
            Self::ReferenceManager => "references",
            Self::AddMenu => "add_menu",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlayEntry {
    pub id: OverlayId,
    pub kind: OverlayKind,
    pub pinned: bool,
    pub dismiss_on_escape: bool,
    pub dismiss_on_click_away: bool,
}

#[derive(Debug, Default)]
pub struct OverlayStack {
    entries: Vec<OverlayEntry>,
}

impl OverlayStack {
    pub fn push(&mut self, entry: OverlayEntry) {
        self.remove(entry.id);
        self.entries.push(entry);
    }

    pub fn remove(&mut self, id: OverlayId) -> Option<OverlayEntry> {
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        Some(self.entries.remove(index))
    }

    pub fn set_pinned(&mut self, id: OverlayId, pinned: bool) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) else {
            return false;
        };
        entry.pinned = pinned;
        true
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, id: OverlayId) -> bool {
        self.entries.iter().any(|entry| entry.id == id)
    }

    pub fn esc(&mut self) -> Option<OverlayEntry> {
        let entry = self.entries.last()?;
        if !entry.dismiss_on_escape {
            return None;
        }
        self.entries.pop()
    }

    pub fn click_away(&mut self) -> Option<OverlayEntry> {
        let entry = self.entries.last()?;
        if entry.pinned || !entry.dismiss_on_click_away {
            return None;
        }
        self.entries.pop()
    }

    pub fn top(&self) -> Option<OverlayEntry> {
        self.entries.last().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: OverlayId, kind: OverlayKind) -> OverlayEntry {
        OverlayEntry {
            id,
            kind,
            pinned: false,
            dismiss_on_escape: true,
            dismiss_on_click_away: true,
        }
    }

    #[test]
    fn escape_closes_the_most_recent_layer_first() {
        let mut stack = OverlayStack::default();
        stack.push(entry(OverlayId::SceneDrawer, OverlayKind::Drawer));
        stack.push(entry(OverlayId::CommandPalette, OverlayKind::Popover));
        assert_eq!(
            stack.esc().map(|item| item.kind),
            Some(OverlayKind::Popover)
        );
        assert_eq!(stack.esc().map(|item| item.kind), Some(OverlayKind::Drawer));
        assert!(stack.is_empty());
    }

    #[test]
    fn outside_click_skips_pinned_layers() {
        let mut stack = OverlayStack::default();
        stack.push(entry(OverlayId::SceneDrawer, OverlayKind::Drawer));
        stack.push(OverlayEntry {
            id: OverlayId::Settings,
            kind: OverlayKind::FloatingPanel,
            pinned: true,
            dismiss_on_escape: true,
            dismiss_on_click_away: true,
        });
        assert!(stack.click_away().is_none());
        assert_eq!(
            stack.top().map(|item| item.kind),
            Some(OverlayKind::FloatingPanel)
        );
    }

    #[test]
    fn outside_click_does_not_close_non_dismissible_layers() {
        let mut stack = OverlayStack::default();
        stack.push(OverlayEntry {
            id: OverlayId::CommandPalette,
            kind: OverlayKind::Modal,
            pinned: false,
            dismiss_on_escape: true,
            dismiss_on_click_away: false,
        });
        assert!(stack.click_away().is_none());
        assert_eq!(stack.len(), 1);
    }

    #[test]
    fn reopening_existing_surface_moves_it_to_top_without_duplicates() {
        let mut stack = OverlayStack::default();
        stack.push(entry(OverlayId::SceneDrawer, OverlayKind::Drawer));
        stack.push(entry(OverlayId::Settings, OverlayKind::Modal));
        stack.push(entry(OverlayId::SceneDrawer, OverlayKind::Drawer));

        assert_eq!(stack.len(), 2);
        assert_eq!(
            stack.top().map(|entry| entry.id),
            Some(OverlayId::SceneDrawer)
        );
    }

    #[test]
    fn explicit_close_removes_only_the_matching_surface() {
        let mut stack = OverlayStack::default();
        stack.push(entry(OverlayId::SceneDrawer, OverlayKind::Drawer));
        stack.push(entry(OverlayId::AssetLibrary, OverlayKind::Drawer));

        assert_eq!(
            stack.remove(OverlayId::SceneDrawer).map(|entry| entry.id),
            Some(OverlayId::SceneDrawer)
        );
        assert_eq!(
            stack.top().map(|entry| entry.id),
            Some(OverlayId::AssetLibrary)
        );
    }
}
