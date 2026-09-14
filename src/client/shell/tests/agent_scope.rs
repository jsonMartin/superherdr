use super::*;
use crate::api::schema::{Method, WorkspaceSnoozeRecord, WorkspaceSnoozeState};
use crate::client::endpoint::{
    ClientEndpointId, ClientEndpointStatus, ProfileId, SavedSshEndpoint,
};
use crate::client::shell::global_menu::{global_menu_items, ClientGlobalMenuAction};
use crate::input::{KeybindAction, KeybindMatch};

fn grouped_snapshot() -> ClientShellSnapshot {
    let mut value = snapshot();
    let base = value.workspaces[0].clone();
    value.workspaces.clear();
    for (id, linked, key) in [
        ("ws_1", false, Some("repo")),
        ("child", true, Some("repo")),
        ("standalone", false, None),
        ("orphan", true, Some("other")),
    ] {
        let mut workspace = base.clone();
        workspace.workspace_id = id.into();
        workspace.label = id.into();
        workspace.worktree = key.map(|key| ClientShellWorktree {
            key: key.into(),
            label: key.into(),
            is_linked_worktree: linked,
        });
        value.workspaces.push(workspace);
        value.agents.push(ClientShellAgent {
            pane_id: id.into(),
            workspace_id: id.into(),
            tab_id: "tab_1".into(),
            name: Some(id.into()),
            display_agent: None,
            agent: Some("codex".into()),
            title: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_status: AgentStatus::Blocked,
            state_change_seq: 1,
            state_labels: Vec::new(),
            tokens: Vec::new(),
            focused: false,
        });
    }
    value
}

fn state() -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(grouped_snapshot()));
    state.set_pane_surface(surface());
    state
}

fn targets(state: &ClientShellState) -> Vec<(ClientEndpointId, String)> {
    super::super::aggregate_navigation::online_agent_targets(
        &state.endpoints,
        state.config.agent_panel_sort,
        state.config.top_level_agents,
    )
    .into_iter()
    .map(|target| (target.endpoint_id, target.pane_id))
    .collect()
}

#[test]
fn top_level_toggle_filters_agents_and_navigation_without_runtime_actions() {
    let mut state = state();
    let before = state.snapshot.clone();
    assert_eq!(targets(&state).len(), 4);
    state.compose(106, 40).unwrap();
    let toggle = state.hits.agent_scope_toggle;
    assert!(!toggle.is_empty());
    let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: toggle.x,
        row: toggle.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(state.config.top_level_agents);
    assert!(outcome.actions.is_empty());
    assert_eq!(state.snapshot, before);
    assert_eq!(
        targets(&state)
            .iter()
            .map(|(_, pane)| pane.as_str())
            .collect::<Vec<_>>(),
        ["ws_1", "standalone", "orphan"]
    );
    state.compose(106, 40).unwrap();
    assert!(!state.hits.agents.iter().any(|(_, pane)| pane == "child"));
    assert!(!state
        .hits
        .endpoint_agents
        .iter()
        .any(|(_, _, pane)| pane == "child"));
    assert_eq!(
        state.endpoint_method_for_action(KeybindAction::FocusAgent(1)),
        Some(Method::PaneFocus(crate::api::schema::PaneTarget {
            pane_id: "standalone".into()
        }))
    );
    let mut next = ClientShellInput::default();
    state.record_binding(
        KeybindMatch::Action(KeybindAction::FocusAgent(1)),
        &mut next,
    );
    assert!(
        matches!(&next.actions[..], [ClientShellAction::Endpoint { request, .. }]
        if matches!(&request.method, Method::PaneFocus(pane) if pane.pane_id == "standalone"))
    );
    for (current, action, expected) in [
        ("ws_1", KeybindAction::NextAgent, "standalone"),
        ("orphan", KeybindAction::NextAgent, "ws_1"),
        ("ws_1", KeybindAction::PreviousAgent, "orphan"),
        ("standalone", KeybindAction::PreviousAgent, "ws_1"),
        ("child", KeybindAction::NextAgent, "ws_1"),
    ] {
        let mut snapshot = grouped_snapshot();
        snapshot.focused_pane_id = Some(current.into());
        state.set_snapshot(Box::new(snapshot));
        assert_eq!(
            state.endpoint_method_for_action(action),
            Some(Method::PaneFocus(crate::api::schema::PaneTarget {
                pane_id: expected.into(),
            }))
        );
    }
    assert!(state.is_workspace_id_visible("child"));
    let mut show_all = ClientShellInput::default();
    state.toggle_agent_scope(&mut show_all);
    assert!(show_all.actions.is_empty());
    assert_eq!(targets(&state).len(), 4);
}

#[test]
fn parent_snooze_and_collapse_do_not_promote_children_but_absence_does() {
    let mut state = state();
    state.config.top_level_agents = true;
    state.collapsed_groups.insert("repo".into());
    state.set_snooze_state(WorkspaceSnoozeState {
        boot_id: "boot-1".into(),
        revision: 1,
        persistence: None,
        project_records: vec![],
        records: vec![WorkspaceSnoozeRecord {
            workspace_id: "ws_1".into(),
            boot_id: "boot-1".into(),
            deadline_unix_ms: i64::MAX,
            revision: 1,
        }],
    });
    assert_eq!(
        targets(&state)
            .iter()
            .map(|(_, pane)| pane.as_str())
            .collect::<Vec<_>>(),
        ["standalone", "orphan"]
    );
    let mut changed = grouped_snapshot();
    changed.revision += 1;
    changed
        .workspaces
        .retain(|workspace| workspace.workspace_id != "ws_1");
    state.set_snapshot(Box::new(changed));
    assert!(targets(&state).iter().any(|(_, pane)| pane == "child"));
}

#[test]
fn remote_group_membership_is_endpoint_local_and_preserves_external_order() {
    let mut state = state();
    let profile = SavedSshEndpoint {
        id: ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        label: "Remote".into(),
        target: "example.invalid".into(),
        session: "test".into(),
        enabled: true,
    };
    let remote_id = ClientEndpointId::Ssh(profile.id.clone());
    state.set_endpoint_catalog(&[profile]);
    state.set_endpoint_status(&remote_id, ClientEndpointStatus::Online);
    let mut remote = grouped_snapshot();
    remote.boot_id = "remote-boot".into();
    remote
        .workspaces
        .retain(|workspace| workspace.workspace_id != "ws_1");
    remote.agent_view_label = Some("external".into());
    remote.agent_order = vec!["orphan".into(), "child".into()];
    state.set_endpoint_snapshot(&remote_id, Box::new(remote));
    state.config.top_level_agents = true;
    let selected = targets(&state);
    assert!(!selected.contains(&(ClientEndpointId::Local, "child".into())));
    assert_eq!(
        selected
            .iter()
            .filter(|(id, _)| id == &remote_id)
            .map(|(_, pane)| pane.as_str())
            .collect::<Vec<_>>(),
        ["orphan", "child"]
    );
}

#[test]
fn empty_and_compact_views_keep_recovery_available() {
    let mut state = state();
    let mut projected = grouped_snapshot();
    projected
        .agents
        .retain(|agent| agent.workspace_id == "child");
    state.set_snapshot(Box::new(projected));
    state.config.top_level_agents = true;
    state.sidebar_width = 18;
    state.compose(106, 40).unwrap();
    assert!(targets(&state).is_empty());
    assert!(!state.hits.agent_scope_toggle.is_empty());
    assert!(!super::super::contains(
        state.hits.agent_sort_toggle,
        (
            state.hits.agent_scope_toggle.x,
            state.hits.agent_scope_toggle.y
        )
    ));
    let index = global_menu_items(state.snapshot.as_deref().unwrap())
        .iter()
        .position(|(_, action)| *action == ClientGlobalMenuAction::ToggleAgentScope)
        .unwrap();
    let mut outcome = ClientShellInput::default();
    state.activate_global_menu_item(index, &mut outcome);
    assert!(outcome.actions.is_empty());
    assert_eq!(targets(&state).len(), 1);
}

#[test]
fn multiple_parent_agents_remain_visible_and_scope_survives_config_reload() {
    let mut state = state();
    let mut projected = grouped_snapshot();
    let mut second_parent = projected.agents[0].clone();
    second_parent.pane_id = "second-parent".into();
    projected.agents.push(second_parent);
    state.set_snapshot(Box::new(projected));
    state.config.top_level_agents = true;
    state.config.apply_live_config(&Config::default(), &[], &[]);
    assert!(state.config.top_level_agents);
    assert_eq!(targets(&state).len(), 4);
    assert!(!ClientShellConfig::from_config(&Config::default()).top_level_agents);
}

#[test]
fn mobile_scope_recovery_is_local_and_leaves_child_workspace_access() {
    let mut state = state();
    state.config.top_level_agents = true;
    state.mode = ClientShellMode::Navigate;
    state.compose(56, 40).unwrap();
    let rect = state
        .hits
        .mobile_targets
        .iter()
        .find(|(_, target)| matches!(target, ClientMobileTarget::ToggleAgentScope))
        .unwrap()
        .0;
    assert!(state.hits.mobile_targets.iter().any(|(_, target)|
        matches!(target, ClientMobileTarget::Workspace { workspace_id, .. } if workspace_id == "child")));
    let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(outcome.actions.is_empty());
    assert!(!state.config.top_level_agents);
}
