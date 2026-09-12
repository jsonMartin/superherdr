use std::collections::HashSet;

use crate::api::schema::WorkspaceSnoozeState;
use crate::protocol::{ClientShellAgent, ClientShellWorkspace};

use super::ClientEndpointId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClientFocusScope {
    Worktree {
        endpoint_id: ClientEndpointId,
        boot_id: String,
        worktree_key: String,
    },
    StandaloneWorkspace {
        endpoint_id: ClientEndpointId,
        boot_id: String,
        workspace_id: String,
    },
}

impl ClientFocusScope {
    pub(crate) fn matches_workspace(
        &self,
        endpoint_id: &ClientEndpointId,
        boot_id: Option<&str>,
        workspace: &ClientShellWorkspace,
    ) -> bool {
        match self {
            Self::Worktree {
                endpoint_id: scope_endpoint,
                boot_id: scope_boot,
                worktree_key,
            } => {
                endpoint_id == scope_endpoint
                    && boot_id == Some(scope_boot.as_str())
                    && workspace
                        .worktree
                        .as_ref()
                        .is_some_and(|w| &w.key == worktree_key)
            }
            Self::StandaloneWorkspace {
                endpoint_id: scope_endpoint,
                boot_id: scope_boot,
                workspace_id,
            } => {
                endpoint_id == scope_endpoint
                    && boot_id == Some(scope_boot.as_str())
                    && &workspace.workspace_id == workspace_id
            }
        }
    }
}

pub(crate) fn is_workspace_snoozed(
    snooze_state: Option<&WorkspaceSnoozeState>,
    boot_id: Option<&str>,
    workspace_id: &str,
) -> bool {
    let Some(state) = snooze_state else {
        return false;
    };
    let Some(boot) = boot_id else {
        return false;
    };
    if state.boot_id != boot {
        return false;
    }
    state
        .records
        .iter()
        .any(|record| record.workspace_id == workspace_id)
}

pub(crate) fn is_project_snoozed(
    snooze_state: Option<&WorkspaceSnoozeState>,
    boot_id: Option<&str>,
    project_key: &str,
) -> bool {
    let Some(state) = snooze_state else {
        return false;
    };
    boot_id == Some(state.boot_id.as_str())
        && state
            .project_records
            .iter()
            .any(|record| record.project_key == project_key)
}

fn workspace_is_project_snoozed(
    snooze_state: Option<&WorkspaceSnoozeState>,
    boot_id: Option<&str>,
    workspace: &ClientShellWorkspace,
) -> bool {
    workspace
        .worktree
        .as_ref()
        .is_some_and(|worktree| is_project_snoozed(snooze_state, boot_id, &worktree.key))
}

pub(crate) fn workspace_is_visible(
    focus_scope: Option<&ClientFocusScope>,
    snooze_state: Option<&WorkspaceSnoozeState>,
    endpoint_id: &ClientEndpointId,
    boot_id: Option<&str>,
    workspace: &ClientShellWorkspace,
) -> bool {
    if is_workspace_snoozed(snooze_state, boot_id, &workspace.workspace_id) {
        return false;
    }
    if workspace_is_project_snoozed(snooze_state, boot_id, workspace) {
        return false;
    }
    if let Some(scope) = focus_scope {
        scope.matches_workspace(endpoint_id, boot_id, workspace)
    } else {
        true
    }
}

pub(crate) fn visible_workspace_ids(
    focus_scope: Option<&ClientFocusScope>,
    snooze_state: Option<&WorkspaceSnoozeState>,
    endpoint_id: &ClientEndpointId,
    boot_id: Option<&str>,
    workspaces: &[ClientShellWorkspace],
) -> HashSet<String> {
    let snoozed = snooze_state
        .filter(|state| boot_id == Some(state.boot_id.as_str()))
        .map(|state| {
            state
                .records
                .iter()
                .map(|record| record.workspace_id.as_str())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let project_snoozed = snooze_state
        .filter(|state| boot_id == Some(state.boot_id.as_str()))
        .map(|state| {
            state
                .project_records
                .iter()
                .map(|record| record.project_key.as_str())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    workspaces
        .iter()
        .filter(|workspace| {
            !snoozed.contains(workspace.workspace_id.as_str())
                && !workspace.worktree.as_ref().is_some_and(|worktree| {
                    project_snoozed.contains(worktree.key.as_str())
                })
                && focus_scope.is_none_or(|scope| {
                    scope.matches_workspace(endpoint_id, boot_id, workspace)
                })
        })
        .map(|workspace| workspace.workspace_id.clone())
        .collect()
}

pub(crate) fn agent_is_visible(
    focus_scope: Option<&ClientFocusScope>,
    snooze_state: Option<&WorkspaceSnoozeState>,
    endpoint_id: &ClientEndpointId,
    boot_id: Option<&str>,
    agent: &ClientShellAgent,
    workspaces: &[ClientShellWorkspace],
) -> bool {
    let Some(workspace) = workspaces
        .iter()
        .find(|ws| ws.workspace_id == agent.workspace_id)
    else {
        return false;
    };
    workspace_is_visible(
        focus_scope,
        snooze_state,
        endpoint_id,
        boot_id,
        workspace,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::{ProjectSnoozeRecord, WorkspaceSnoozeRecord};
    use crate::protocol::ClientShellWorktree;

    fn test_workspace(id: &str, worktree_key: Option<&str>) -> ClientShellWorkspace {
        ClientShellWorkspace {
            workspace_id: id.to_string(),
            active_tab_id: "tab_1".to_string(),
            new_workspace_cwd: "/test".to_string(),
            number: 1,
            label: id.to_string(),
            custom_label: false,
            branch: None,
            git_ahead_behind: None,
            tokens: Vec::new(),
            worktree: worktree_key.map(|k| ClientShellWorktree {
                key: k.to_string(),
                label: k.to_string(),
                is_linked_worktree: false,
            }),
            focused: false,
            agent_status: crate::api::schema::AgentStatus::Idle,
        }
    }

    #[test]
    fn test_snooze_hides_workspace_and_agents() {
        let ws1 = test_workspace("ws_1", Some("repo_a"));
        let ws2 = test_workspace("ws_2", Some("repo_a"));
        let endpoint = ClientEndpointId::Local;
        let boot = "boot_1";

        let snooze_state = WorkspaceSnoozeState {
            boot_id: "boot_1".to_string(),
            revision: 1,
            records: vec![WorkspaceSnoozeRecord {
                workspace_id: "ws_1".to_string(),
                boot_id: "boot_1".to_string(),
                deadline_unix_ms: 10_000,
                revision: 1,
            }],
            project_records: Vec::new(),
            persistence: None,
        };

        // Snoozed workspace is hidden
        assert!(!workspace_is_visible(
            None,
            Some(&snooze_state),
            &endpoint,
            Some(boot),
            &ws1,
        ));
        // Non-snoozed workspace is visible
        assert!(workspace_is_visible(
            None,
            Some(&snooze_state),
            &endpoint,
            Some(boot),
            &ws2,
        ));
        // Missing boot fails visible
        assert!(workspace_is_visible(
            None,
            Some(&snooze_state),
            &endpoint,
            None,
            &ws1,
        ));
        // Retired / cleared state makes workspace visible
        let empty_snooze = WorkspaceSnoozeState {
            boot_id: "boot_1".to_string(),
            revision: 2,
            records: Vec::new(),
            project_records: Vec::new(),
            persistence: None,
        };
        assert!(workspace_is_visible(
            None,
            Some(&empty_snooze),
            &endpoint,
            Some(boot),
            &ws1,
        ));
    }

    #[test]
    fn test_local_focus_scope_matches_only_in_scope() {
        let ws1 = test_workspace("ws_1", Some("repo_a"));
        let ws2 = test_workspace("ws_2", Some("repo_b"));
        let ws3 = test_workspace("ws_3", None);
        let endpoint = ClientEndpointId::Local;
        let boot = "boot_1";

        let focus_scope = ClientFocusScope::Worktree {
            endpoint_id: endpoint.clone(),
            boot_id: boot.to_string(),
            worktree_key: "repo_a".to_string(),
        };

        assert!(workspace_is_visible(
            Some(&focus_scope),
            None,
            &endpoint,
            Some(boot),
            &ws1,
        ));
        assert!(!workspace_is_visible(
            Some(&focus_scope),
            None,
            &endpoint,
            Some(boot),
            &ws2,
        ));
        assert!(!workspace_is_visible(
            Some(&focus_scope),
            None,
            &endpoint,
            Some(boot),
            &ws3,
        ));
    }

    #[test]
    fn project_snooze_hides_members_with_matching_key_only() {
        let members = [
            test_workspace("ws_primary", Some("repo_a")),
            test_workspace("ws_child", Some("repo_a")),
            test_workspace("ws_other", Some("repo_b")),
        ];
        let state = WorkspaceSnoozeState {
            boot_id: "boot_1".into(),
            revision: 1,
            records: Vec::new(),
            project_records: vec![ProjectSnoozeRecord {
                project_key: "repo_a".into(),
                boot_id: "boot_1".into(),
                deadline_unix_ms: 10_000,
                revision: 1,
            }],
            persistence: None,
        };
        let visible = visible_workspace_ids(
            None,
            Some(&state),
            &ClientEndpointId::Local,
            Some("boot_1"),
            &members,
        );
        assert!(!visible.contains("ws_primary"));
        assert!(!visible.contains("ws_child"));
        assert!(visible.contains("ws_other"));
    }
}
