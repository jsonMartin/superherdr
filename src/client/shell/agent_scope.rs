use super::*;

pub(super) fn visible_agent_workspace_ids(
    snapshot: &ClientShellSnapshot,
    focus: Option<&ClientFocusScope>,
    snooze: Option<&crate::api::schema::WorkspaceSnoozeState>,
    endpoint_id: &ClientEndpointId,
    top_level: bool,
) -> HashSet<String> {
    let mut visible = super::focus_snooze::visible_workspace_ids(
        focus,
        snooze,
        endpoint_id,
        Some(snapshot.boot_id.as_str()),
        &snapshot.workspaces,
    );
    if top_level {
        // Membership uses the complete endpoint snapshot, even when its parent is hidden.
        let parent_keys = snapshot
            .workspaces
            .iter()
            .filter_map(|workspace| {
                workspace
                    .worktree
                    .as_ref()
                    .filter(|tree| !tree.is_linked_worktree)
                    .map(|tree| tree.key.as_str())
            })
            .collect::<HashSet<_>>();
        for workspace in &snapshot.workspaces {
            if workspace.worktree.as_ref().is_some_and(|tree| {
                tree.is_linked_worktree && parent_keys.contains(tree.key.as_str())
            }) {
                visible.remove(&workspace.workspace_id);
            }
        }
    }
    visible
}

impl ClientShellState {
    pub(super) fn toggle_agent_scope(&mut self, outcome: &mut ClientShellInput) {
        self.config.top_level_agents = !self.config.top_level_agents;
        self.agent_scroll = 0;
        self.mobile_switcher_scroll = 0;
        self.hits = ShellHitMap::default();
        outcome.repaint = true;
    }
}
