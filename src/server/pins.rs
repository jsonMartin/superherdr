use std::collections::HashSet;

use crate::layout::PaneId;
use crate::workspace::Workspace;

/// Server-owned live pin state for the current boot.
///
/// Pins are keyed by internal workspace and pane ids, which survive pane moves
/// between workspaces; public ids are derived when building a `PinState`.
#[derive(Debug, Clone)]
pub(crate) struct PinManager {
    revision: u64,
    workspace_ids: HashSet<String>,
    pane_ids: HashSet<PaneId>,
}

impl PinManager {
    pub(crate) fn new() -> Self {
        Self {
            revision: 0,
            workspace_ids: HashSet::new(),
            pane_ids: HashSet::new(),
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) fn is_workspace_pinned(&self, workspace_id: &str) -> bool {
        self.workspace_ids.contains(workspace_id)
    }

    pub(crate) fn is_pane_pinned(&self, pane_id: PaneId) -> bool {
        self.pane_ids.contains(&pane_id)
    }

    pub(crate) fn pinned_workspace_ids(&self) -> impl Iterator<Item = &String> {
        self.workspace_ids.iter()
    }

    pub(crate) fn pinned_pane_ids(&self) -> impl Iterator<Item = &PaneId> {
        self.pane_ids.iter()
    }

    /// Restores a validated persisted workspace pin. The caller has already
    /// resolved the entry to a live workspace.
    pub(crate) fn restore_workspace(&mut self, workspace_id: String) {
        self.revision = self.revision.saturating_add(1);
        self.workspace_ids.insert(workspace_id);
    }

    /// Restores a validated persisted agent pin. The caller has already
    /// resolved the entry to a live pane.
    pub(crate) fn restore_pane(&mut self, pane_id: PaneId) {
        self.revision = self.revision.saturating_add(1);
        self.pane_ids.insert(pane_id);
    }

    /// Sets a workspace pin; returns whether anything changed.
    pub(crate) fn set_workspace(&mut self, workspace_id: &str, pinned: bool) -> bool {
        let changed = if pinned {
            self.workspace_ids.insert(workspace_id.to_owned())
        } else {
            self.workspace_ids.remove(workspace_id)
        };
        if changed {
            self.revision = self.revision.saturating_add(1);
        }
        changed
    }

    /// Sets a pane pin; returns whether anything changed.
    pub(crate) fn set_pane(&mut self, pane_id: PaneId, pinned: bool) -> bool {
        let changed = if pinned {
            self.pane_ids.insert(pane_id)
        } else {
            self.pane_ids.remove(&pane_id)
        };
        if changed {
            self.revision = self.revision.saturating_add(1);
        }
        changed
    }

    /// Drops pins whose workspace or pane no longer exists; returns whether
    /// anything changed.
    pub(crate) fn retain_live(&mut self, workspaces: &[Workspace]) -> bool {
        let live_workspace_ids: HashSet<&str> = workspaces
            .iter()
            .map(|workspace| workspace.id.as_str())
            .collect();
        let live_pane_ids: HashSet<PaneId> = workspaces
            .iter()
            .flat_map(|workspace| workspace.tabs.iter())
            .flat_map(|tab| tab.panes.keys().copied())
            .collect();
        let before = self.workspace_ids.len() + self.pane_ids.len();
        self.workspace_ids
            .retain(|workspace_id| live_workspace_ids.contains(workspace_id.as_str()));
        self.pane_ids
            .retain(|pane_id| live_pane_ids.contains(pane_id));
        let changed = self.workspace_ids.len() + self.pane_ids.len() != before;
        if changed {
            self.revision = self.revision.saturating_add(1);
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pin_changes_bump_revision_only_on_real_changes() {
        let mut manager = PinManager::new();
        assert!(manager.set_workspace("ws_1", true));
        assert_eq!(manager.revision(), 1);
        assert!(manager.is_workspace_pinned("ws_1"));
        assert!(!manager.set_workspace("ws_1", true));
        assert_eq!(manager.revision(), 1);
        assert!(manager.set_workspace("ws_1", false));
        assert_eq!(manager.revision(), 2);
        assert!(!manager.is_workspace_pinned("ws_1"));
        assert!(!manager.set_workspace("ws_1", false));
        assert_eq!(manager.revision(), 2);

        let pane_id = PaneId::from_raw(7);
        assert!(manager.set_pane(pane_id, true));
        assert_eq!(manager.revision(), 3);
        assert!(manager.is_pane_pinned(pane_id));
        assert!(!manager.set_pane(pane_id, true));
        assert_eq!(manager.revision(), 3);
        assert!(manager.set_pane(pane_id, false));
        assert_eq!(manager.revision(), 4);
    }

    #[test]
    fn retain_live_drops_closed_workspaces_and_panes() {
        let workspace = Workspace::test_new("alpha");
        let pane_id = workspace.focused_pane_id().expect("pane");
        let mut manager = PinManager::new();
        manager.restore_workspace(workspace.id.clone());
        manager.restore_pane(pane_id);
        let revision_before = manager.revision();

        // Live items keep their pins without a revision change.
        assert!(!manager.retain_live(std::slice::from_ref(&workspace)));
        assert_eq!(manager.revision(), revision_before);
        assert!(manager.is_workspace_pinned(&workspace.id));
        assert!(manager.is_pane_pinned(pane_id));

        // Closing everything prunes both pins once.
        let mut closed = manager.clone();
        assert!(closed.retain_live(&[]));
        assert_eq!(closed.revision(), revision_before + 1);
        assert!(!closed.is_workspace_pinned(&workspace.id));
        assert!(!closed.is_pane_pinned(pane_id));
        assert!(!closed.retain_live(&[]));
        assert_eq!(closed.revision(), revision_before + 1);
    }
}
