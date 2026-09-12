use super::*;

use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

use crate::api::schema::{
    AgentStatus, ProjectSnoozeRecord, WorkspaceSnoozeRecord, WorkspaceSnoozeState,
};
use crate::client::endpoint::ClientEndpointId;
use crate::client::shell::focus_snooze::ClientFocusScope;
use crate::client::shell::state::{ClientContextMenuAction, ClientShellAction};
use crate::config::Config;
use crate::input::KeybindAction;
use crate::protocol::{ClientShellAgent, ClientShellPane, ClientShellTab, ClientShellWorktree};

fn two_workspace_snapshot() -> ClientShellSnapshot {
    let mut snapshot = super::snapshot();
    snapshot.focused_workspace_id = Some("ws_1".into());
    snapshot.focused_tab_id = Some("tab_1".into());
    snapshot.focused_pane_id = Some("pane_1".into());
    snapshot.agent_order = vec!["pane_1".into(), "pane_2".into()];
    snapshot
        .workspaces
        .push(crate::protocol::ClientShellWorkspace {
            workspace_id: "ws_2".into(),
            active_tab_id: "tab_2".into(),
            new_workspace_cwd: "/repo2".into(),
            number: 2,
            label: "workspace-2".into(),
            custom_label: false,
            branch: Some("feat".into()),
            git_ahead_behind: None,
            tokens: Vec::new(),
            worktree: Some(ClientShellWorktree {
                key: "repo-b".into(),
                label: "repo-b".into(),
                is_linked_worktree: false,
            }),
            focused: false,
            agent_status: AgentStatus::Working,
        });
    snapshot.tabs.push(ClientShellTab {
        tab_id: "tab_2".into(),
        workspace_id: "ws_2".into(),
        number: 1,
        label: "1".into(),
        custom_label: false,
        zoomed: false,
        focused: false,
        agent_status: AgentStatus::Working,
    });
    snapshot.panes.push(ClientShellPane {
        pane_id: "pane_2".into(),
        workspace_id: "ws_2".into(),
        tab_id: "tab_2".into(),
        label: None,
        cwd: Some("/repo2".into()),
        foreground_cwd: Some("/repo2".into()),
        focused: false,
        right_click_passthrough: false,
    });
    snapshot.agents.insert(
        0,
        ClientShellAgent {
            pane_id: "pane_1".into(),
            workspace_id: "ws_1".into(),
            tab_id: "tab_1".into(),
            name: Some("agent-1".into()),
            display_agent: None,
            agent: Some("claude".into()),
            title: None,
            terminal_title: None,
            terminal_title_stripped: None,
            agent_status: AgentStatus::Working,
            state_change_seq: 1,
            state_labels: Vec::new(),
            tokens: Vec::new(),
            focused: true,
        },
    );
    snapshot.agents.push(ClientShellAgent {
        pane_id: "pane_2".into(),
        workspace_id: "ws_2".into(),
        tab_id: "tab_2".into(),
        name: Some("agent-2".into()),
        display_agent: None,
        agent: Some("claude".into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: AgentStatus::Working,
        state_change_seq: 2,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: false,
    });
    snapshot
}

fn snooze_state(workspace_ids: &[&str], revision: u64) -> WorkspaceSnoozeState {
    WorkspaceSnoozeState {
        boot_id: "boot-1".into(),
        revision,
        records: workspace_ids
            .iter()
            .enumerate()
            .map(|(index, workspace_id)| WorkspaceSnoozeRecord {
                workspace_id: (*workspace_id).into(),
                boot_id: "boot-1".into(),
                deadline_unix_ms: 4_000_000 + index as i64,
                revision: revision.saturating_sub(workspace_ids.len() as u64 - index as u64 - 1),
            })
            .collect(),
        project_records: Vec::new(),
        persistence: None,
    }
}

#[test]
fn snooze_filters_workspace_and_agent_rows_without_mutating_snapshot() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let snapshot = two_workspace_snapshot();
    state.set_snapshot(Box::new(snapshot.clone()));
    let baseline = state.snapshot.clone();
    state.set_snooze_state(snooze_state(&["ws_1"], 1));

    assert!(!state.is_workspace_id_visible("ws_1"));
    assert!(state.is_workspace_id_visible("ws_2"));
    assert!(!state.is_agent_visible(&snapshot.agents[0]));
    assert!(state.is_agent_visible(&snapshot.agents[1]));
    assert_eq!(state.snapshot, baseline);
}

#[test]
fn focus_scope_is_endpoint_and_boot_qualified() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let mut snapshot = two_workspace_snapshot();
    snapshot.workspaces[0].worktree = Some(ClientShellWorktree {
        key: "repo-a".into(),
        label: "repo-a".into(),
        is_linked_worktree: false,
    });
    state.set_snapshot(Box::new(snapshot));
    state.set_focus_scope(Some(ClientFocusScope::Worktree {
        endpoint_id: ClientEndpointId::Local,
        boot_id: "boot-1".into(),
        worktree_key: "repo-a".into(),
    }));

    assert!(state.is_workspace_id_visible("ws_1"));
    assert!(!state.is_workspace_id_visible("ws_2"));
    state.active_endpoint_id = ClientEndpointId::Ssh(
        crate::client::endpoint::ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
    );
    assert!(!state.is_workspace_id_visible("ws_1"));
}

#[test]
fn passive_wake_keeps_empty_presentation_latched() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_snooze_state(snooze_state(&["ws_1", "ws_2"], 2));
    assert!(state.empty_presentation);
    state.set_snooze_state(snooze_state(&["ws_2"], 3));
    assert!(state.empty_presentation);
}

#[test]
fn passive_wake_updates_empty_copy_and_removes_stale_recovery_affordance() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_snooze_state(snooze_state(&["ws_1", "ws_2"], 2));
    state.set_snooze_state(snooze_state(&[], 3));

    let frame = state.compose(80, 20).expect("empty frame");
    let text = composed_text(&frame);
    assert!(text.contains("Terminal input paused. Select a workspace."));
    assert!(!text.contains("Wake shared snoozes"));
    assert_eq!(state.hits.empty_recovery.width, 0);
    assert!(state
        .hits
        .workspaces
        .iter()
        .any(|hit| hit.workspace_id == "ws_1"));
}

#[test]
fn explicit_focus_commit_clears_empty_latch_for_visible_workspace() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.empty_presentation = true;

    assert!(state.commit_explicit_workspace_focus("ws_1"));
    assert!(!state.empty_presentation);
}

#[test]
fn empty_latch_stays_set_without_explicit_visible_focus_commit() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_snooze_state(snooze_state(&["ws_1", "ws_2"], 1));
    assert!(state.empty_presentation);

    state.reconcile_active_workspace_visibility(false);
    assert!(state.empty_presentation);
    assert!(!state.commit_explicit_workspace_focus("ws_1"));
    assert!(state.empty_presentation);

    state.set_snooze_state(snooze_state(&[], 2));
    assert!(state.empty_presentation);
}

#[test]
fn empty_presentation_suppresses_text_and_key_press_input() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.empty_presentation = true;

    let key = crate::input::TerminalKey::new(KeyCode::Char('a'), KeyModifiers::empty());
    let key_outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(key)]);
    assert!(key_outcome.requests.is_empty());
    let text_outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Text(
        crate::input::TextCommit::new("input"),
    )]);
    assert!(text_outcome.requests.is_empty());
    let mut release = crate::input::TerminalKey::new(KeyCode::Char('a'), KeyModifiers::empty());
    release.kind = KeyEventKind::Release;
    state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(release)]);
}

#[test]
fn empty_presentation_recovery_and_sidebar_are_disjoint_at_mobile_sizes() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_snooze_state(snooze_state(&["ws_1", "ws_2"], 2));
    let profile = crate::client::endpoint::SavedSshEndpoint {
        id: crate::client::endpoint::ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        label: "Remote".into(),
        target: "remote".into(),
        session: "agents".into(),
        enabled: true,
    };
    let remote = ClientEndpointId::Ssh(profile.id.clone());
    state.set_endpoint_catalog(&[profile]);
    state.set_endpoint_status(
        &remote,
        crate::client::endpoint::ClientEndpointStatus::Online,
    );
    let mut remote_snapshot = two_workspace_snapshot();
    remote_snapshot.boot_id = "remote-boot".into();
    state.set_endpoint_snapshot(&remote, Box::new(remote_snapshot));
    assert!(state.empty_presentation);

    for (cols, rows) in [(24, 8), (25, 20)] {
        let text = composed_text(&state.compose(cols, rows).unwrap());
        assert!(
            text.contains("Wake shared snoozes"),
            "{cols}x{rows}: {text}"
        );
        assert!(state.hits.empty_recovery.width > 0);
        assert!(state.hits.empty_clear_focus.width == 0);
        if cols >= 25 {
            assert!(state
                .hits
                .workspaces
                .iter()
                .any(|workspace| workspace.endpoint_id == remote));
        }
        for workspace in &state.hits.workspaces {
            let recovery = state.hits.empty_recovery;
            let hit = workspace.rect;
            let overlaps = recovery.x < hit.right()
                && hit.x < recovery.right()
                && recovery.y < hit.bottom()
                && hit.y < recovery.bottom();
            assert!(!overlaps);
        }
    }
}

#[test]
fn empty_copy_mode_w_opens_recovery_after_shared_snooze_transition() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.mode = ClientShellMode::Copy;
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_snooze_state(snooze_state(&["ws_1", "ws_2"], 2));
    assert!(state.empty_presentation);

    let outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Char('w'), KeyModifiers::empty()),
    )]);
    assert!(outcome.requests.is_empty() && outcome.actions.is_empty());
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::SnoozeManagement(_))
    ));
    assert_eq!(state.mode, ClientShellMode::Copy);
}

#[test]
fn context_menu_snooze_targets_captured_workspace_and_boot() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.open_workspace_context_menu("ws_2".into(), 10, 5);
    let Some(ClientShellOverlay::ContextMenu(menu)) = state.overlay.as_ref() else {
        panic!("expected context menu");
    };
    let index = menu
        .items()
        .iter()
        .position(|item| item.action == ClientContextMenuAction::Snooze30Minutes)
        .expect("snooze menu item");
    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(index, &mut outcome);
    assert!(outcome.actions.is_empty());
    let preview = match state.overlay.as_ref() {
        Some(ClientShellOverlay::Snooze(snooze)) => {
            snooze.choices[snooze.selected].deadline_unix_ms
        }
        _ => panic!("expected snooze picker"),
    };
    let submitted = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Enter, KeyModifiers::empty()),
    )]);
    assert!(submitted.actions.iter().any(|action| matches!(
        action,
        ClientShellAction::Endpoint { boot_id, request, .. }
            if boot_id == "boot-1"
                && matches!(&request.method, crate::api::schema::Method::WorkspaceSnooze(params)
                    if params.workspace_id == "ws_2"
                        && params.boot_id == "boot-1"
                        && params.duration_seconds.is_none()
                        && params.deadline_unix_ms == Some(preview))
    )));

    state.open_workspace_context_menu("ws_2".into(), 10, 5);
    let Some(ClientShellOverlay::ContextMenu(menu)) = state.overlay.as_ref() else {
        panic!("expected project context menu");
    };
    let index = menu
        .items()
        .iter()
        .position(|item| item.action == ClientContextMenuAction::SnoozeProject30Minutes)
        .expect("project snooze menu item");
    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(index, &mut outcome);
    assert!(outcome.actions.is_empty());
    assert!(matches!(state.overlay, Some(ClientShellOverlay::Snooze(_))));
    let submitted = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Enter, KeyModifiers::empty()),
    )]);
    assert!(submitted.actions.iter().any(|action| matches!(
        action,
        ClientShellAction::Endpoint { boot_id, request, .. }
            if boot_id == "boot-1"
                && matches!(&request.method, crate::api::schema::Method::ProjectSnooze(params)
                    if params.workspace_id == "ws_2"
                        && params.project_key == "repo-b"
                        && params.boot_id == "boot-1"
                        && params.duration_seconds.is_none()
                        && params.deadline_unix_ms.is_some())
    )));
}

#[test]
fn context_menu_action_rejects_captured_endpoint_after_switch() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.open_workspace_context_menu("ws_2".into(), 10, 5);
    let close_index = match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu
            .items()
            .iter()
            .position(|item| item.action == ClientContextMenuAction::Close)
            .expect("close menu item"),
        _ => panic!("expected context menu"),
    };

    let profile = crate::client::endpoint::SavedSshEndpoint {
        id: crate::client::endpoint::ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        label: "Remote".into(),
        target: "remote".into(),
        session: "agents".into(),
        enabled: true,
    };
    let remote = ClientEndpointId::Ssh(profile.id.clone());
    state.set_endpoint_catalog(&[profile]);
    state.set_endpoint_status(
        &remote,
        crate::client::endpoint::ClientEndpointStatus::Online,
    );
    let mut remote_snapshot = two_workspace_snapshot();
    remote_snapshot.boot_id = "boot-1".into();
    state.set_endpoint_snapshot(&remote, Box::new(remote_snapshot));
    assert!(state.activate_endpoint_projection(&remote));

    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(close_index, &mut outcome);
    assert!(outcome.actions.is_empty());
}

#[test]
fn inactive_context_menu_uses_cached_endpoint_for_snooze() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    let profile = crate::client::endpoint::SavedSshEndpoint {
        id: crate::client::endpoint::ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        label: "Remote".into(),
        target: "remote".into(),
        session: "agents".into(),
        enabled: true,
    };
    let remote = ClientEndpointId::Ssh(profile.id.clone());
    state.set_endpoint_catalog(&[profile]);
    state.set_endpoint_status(
        &remote,
        crate::client::endpoint::ClientEndpointStatus::Online,
    );
    let mut remote_snapshot = two_workspace_snapshot();
    remote_snapshot.boot_id = "remote-boot".into();
    state.set_endpoint_snapshot(&remote, Box::new(remote_snapshot));
    state.open_endpoint_workspace_context_menu(remote.clone(), "ws_2".into(), 10, 5);

    let Some(ClientShellOverlay::ContextMenu(menu)) = state.overlay.as_ref() else {
        panic!("expected context menu");
    };
    assert!(menu.items().iter().all(|item| !matches!(
        item.action,
        ClientContextMenuAction::Close | ClientContextMenuAction::Rename
    )));
    let index = menu
        .items()
        .iter()
        .position(|item| item.action == ClientContextMenuAction::Snooze30Minutes)
        .expect("snooze menu item");
    state.activate_context_menu_item(index, &mut ClientShellInput::default());
    let outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Enter, KeyModifiers::empty()),
    )]);
    assert!(outcome.actions.iter().any(|action| matches!(
        action,
        ClientShellAction::Endpoint { endpoint_id, boot_id, request }
            if endpoint_id == &remote
                && boot_id == "remote-boot"
                && matches!(&request.method, crate::api::schema::Method::WorkspaceSnooze(params)
                    if params.workspace_id == "ws_2" && params.boot_id == "remote-boot")
    )));
}

#[test]
fn inactive_wake_shared_confirmation_targets_captured_endpoint() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    let profile = crate::client::endpoint::SavedSshEndpoint {
        id: crate::client::endpoint::ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        label: "Remote".into(),
        target: "remote".into(),
        session: "agents".into(),
        enabled: true,
    };
    let remote = ClientEndpointId::Ssh(profile.id.clone());
    state.set_endpoint_catalog(&[profile]);
    state.set_endpoint_status(
        &remote,
        crate::client::endpoint::ClientEndpointStatus::Online,
    );
    let mut remote_snapshot = two_workspace_snapshot();
    remote_snapshot.boot_id = "remote-boot".into();
    state.set_endpoint_snapshot(&remote, Box::new(remote_snapshot));
    let mut remote_state = snooze_state(&["ws_2"], 3);
    remote_state.boot_id = "remote-boot".into();
    remote_state.records[0].boot_id = "remote-boot".into();
    remote_state.persistence = Some(crate::api::schema::SnoozePersistenceInfo {
        records: vec![
            crate::api::schema::SnoozeStoredRecordInfo {
                record_id: "stored-live".into(),
                scope: "workspace".into(),
                label: "Workspace 2".into(),
                project_label: None,
                created_unix_ms: 0,
                deadline_unix_ms: 4_000_000,
                available: true,
                workspace_id: Some("ws_2".into()),
                project_key: None,
            },
            crate::api::schema::SnoozeStoredRecordInfo {
                record_id: "stored-unavailable".into(),
                scope: "workspace".into(),
                label: "Old Space".into(),
                project_label: None,
                created_unix_ms: 0,
                deadline_unix_ms: 4_000_000,
                available: false,
                workspace_id: None,
                project_key: None,
            },
        ],
        notice: None,
    });
    state.set_endpoint_snooze_state(&remote, remote_state);
    state.open_endpoint_workspace_context_menu(remote.clone(), "ws_2".into(), 10, 5);
    let index = match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu
            .items()
            .iter()
            .position(|item| item.action == ClientContextMenuAction::WakeSharedSnoozes)
            .expect("wake shared menu item"),
        _ => panic!("expected context menu"),
    };
    state.activate_context_menu_item(index, &mut ClientShellInput::default());
    let text = composed_text(&state.compose(100, 30).unwrap());
    assert!(text.contains("2 records on Remote"), "{text}");

    let outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Enter, KeyModifiers::empty()),
    )]);
    assert!(outcome.actions.iter().any(|action| matches!(
        action,
        ClientShellAction::Endpoint {
            endpoint_id,
            boot_id,
            request,
        } if endpoint_id == &remote
            && boot_id == "remote-boot"
            && matches!(
                &request.method,
                crate::api::schema::Method::WorkspaceWake(params)
                    if params.workspace_id.is_none()
                        && params.boot_id == "remote-boot"
                        && params.expected_revision == 3
                        && params.confirmed
            )
    )));
}

#[test]
fn snooze_picker_cancel_and_stale_group_do_not_mutate() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.open_workspace_context_menu("ws_2".into(), 10, 5);
    let index = match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu
            .items()
            .iter()
            .position(|item| item.action == ClientContextMenuAction::SnoozeProject30Minutes)
            .expect("project snooze menu item"),
        _ => panic!("expected context menu"),
    };
    state.activate_context_menu_item(index, &mut ClientShellInput::default());
    let baseline = state.snapshot.clone();
    let cancelled = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Esc, KeyModifiers::empty()),
    )]);
    assert!(cancelled.actions.is_empty());
    assert_eq!(state.snapshot, baseline);

    state.open_workspace_context_menu("ws_2".into(), 10, 5);
    let index = match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu
            .items()
            .iter()
            .position(|item| item.action == ClientContextMenuAction::SnoozeProject30Minutes)
            .expect("project snooze menu item"),
        _ => panic!("expected context menu"),
    };
    state.activate_context_menu_item(index, &mut ClientShellInput::default());
    let mut changed = two_workspace_snapshot();
    changed.workspaces[1].worktree.as_mut().unwrap().key = "repo-c".into();
    state.set_snapshot(Box::new(changed));
    let rejected = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Enter, KeyModifiers::empty()),
    )]);
    assert!(rejected.actions.is_empty());
    assert_eq!(
        state.endpoint_error.as_deref(),
        Some("Snooze target changed. Open the menu again.")
    );
}

fn composed_text(frame: &crate::protocol::FrameData) -> String {
    frame
        .cells
        .iter()
        .map(|cell| cell.symbol.as_str())
        .collect()
}

#[test]
fn empty_recovery_is_visible_and_requires_confirmation() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_snooze_state(snooze_state(&["ws_1", "ws_2"], 2));

    let frame = state.compose(80, 20).expect("empty frame");
    assert!(composed_text(&frame).contains("Wake shared snoozes"));
    assert!(state.hits.empty_recovery.width > 0);
    assert!(state.hits.empty_clear_focus.width == 0);

    let key = crate::input::TerminalKey::new(KeyCode::Char('w'), KeyModifiers::empty());
    let opened = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(key)]);
    assert!(opened.repaint);
    assert!(opened.actions.is_empty());
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::SnoozeManagement(_))
    ));
    let menu_frame = state.compose(80, 20).expect("recovery menu frame");
    assert!(composed_text(&menu_frame).contains("Snoozed"));

    let selected = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Char('a'), KeyModifiers::empty()),
    )]);
    assert!(selected.actions.is_empty());
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::ConfirmWakeSharedSnoozes(_))
    ));
    let confirm_frame = state.compose(80, 20).expect("confirmation frame");
    assert!(composed_text(&confirm_frame).contains("Wake shared snoozes?"));

    let cancelled = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Esc, KeyModifiers::empty()),
    )]);
    assert!(cancelled.actions.is_empty());
    assert!(state.overlay.is_none());

    let reopened = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Char('w'), KeyModifiers::empty()),
    )]);
    assert!(reopened.actions.is_empty());
    state.compose(80, 20).expect("reopened recovery menu");
    state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Char('a'), KeyModifiers::empty()),
    )]);
    state.compose(80, 20).expect("reopened confirmation");
    let primary = state.hits.overlay_primary;
    let confirmed = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: primary.x,
            row: primary.y,
            modifiers: KeyModifiers::empty(),
        },
    )]);
    assert!(confirmed.actions.iter().any(|action| matches!(
        action,
        ClientShellAction::Endpoint { boot_id, request, .. }
            if boot_id == "boot-1"
                && matches!(
                    &request.method,
                    crate::api::schema::Method::WorkspaceWake(params)
                        if params.expected_revision == 2 && params.confirmed
                )
    )));
}

#[test]
fn incidental_empty_click_does_not_wake_and_narrow_recovery_is_labeled() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_snooze_state(snooze_state(&["ws_1", "ws_2"], 2));
    let frame = state.compose(24, 8).expect("narrow empty frame");
    assert!(composed_text(&frame).contains("Wake shared"));
    let outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        },
    )]);
    assert!(outcome.actions.is_empty());
    assert!(state.overlay.is_none());
}

#[test]
fn snoozed_records_menu_is_reachable_while_another_workspace_is_visible() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let mut snapshot = two_workspace_snapshot();
    snapshot.workspaces[0].worktree = Some(ClientShellWorktree {
        key: "repo-a".into(),
        label: "repo-a".into(),
        is_linked_worktree: false,
    });
    state.set_snapshot(Box::new(snapshot));
    state.set_snooze_state(WorkspaceSnoozeState {
        boot_id: "boot-1".into(),
        revision: 8,
        records: vec![WorkspaceSnoozeRecord {
            workspace_id: "ws_1".into(),
            boot_id: "boot-1".into(),
            deadline_unix_ms: 9_000,
            revision: 3,
        }],
        project_records: vec![ProjectSnoozeRecord {
            project_key: "repo-a".into(),
            boot_id: "boot-1".into(),
            deadline_unix_ms: 10_000,
            revision: 7,
        }],
        persistence: None,
    });
    assert!(state.is_workspace_id_visible("ws_2"));
    state.set_pane_surface(surface());

    state.open_workspace_context_menu("ws_2".into(), 0, 0);
    state.compose(80, 20).expect("workspace menu frame");
    let show_index = match state.overlay.as_ref() {
        Some(ClientShellOverlay::ContextMenu(menu)) => menu
            .items()
            .iter()
            .position(|item| item.label == "Show snoozed records")
            .expect("show records action"),
        _ => panic!("expected workspace menu"),
    };
    let show_row = state.hits.context_menu_rows[show_index].0;
    let opened = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: show_row.x,
            row: show_row.y,
            modifiers: KeyModifiers::empty(),
        },
    )]);
    assert!(opened.actions.is_empty());
    let frame = state.compose(80, 20).expect("records management frame");
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::SnoozeManagement(_))
    ));
    assert!(composed_text(&frame).contains("repo-a"));
    let selected = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Char('p'), KeyModifiers::empty()),
    )]);
    assert!(selected.actions.iter().any(|action| matches!(
        action,
        ClientShellAction::Endpoint { request, .. }
            if matches!(&request.method, crate::api::schema::Method::ProjectWake(params)
                if params.project_key == "repo-a" && params.expected_revision == 7)
    )));
}

#[test]
fn filtered_worktree_parent_emits_children_as_standalone_rows_when_collapsed() {
    let mut snapshot = two_workspace_snapshot();
    snapshot.workspaces[0].worktree = Some(ClientShellWorktree {
        key: "repo-a".into(),
        label: "repo-a".into(),
        is_linked_worktree: false,
    });
    snapshot.workspaces[1].worktree.as_mut().unwrap().key = "repo-a".into();
    let mut unrelated = snapshot.workspaces[1].clone();
    unrelated.workspace_id = "ws_other".into();
    unrelated.worktree = Some(ClientShellWorktree {
        key: "repo-b".into(),
        label: "repo-b".into(),
        is_linked_worktree: false,
    });
    snapshot.workspaces.push(unrelated);
    let mut collapsed = std::collections::HashSet::new();
    collapsed.insert("repo-a".to_owned());
    let entries = crate::client::shell::sidebar::workspace_entries_with_filter(
        &snapshot,
        &collapsed,
        |workspace| workspace.workspace_id != "ws_1",
    );
    let entry_ids = entries
        .iter()
        .map(|entry| {
            (
                snapshot.workspaces[entry.index].workspace_id.clone(),
                entry.indented,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        entry_ids,
        vec![("ws_2".into(), false), ("ws_other".into(), false)]
    );
}

#[test]
fn local_agent_shortcuts_skip_hidden_workspaces() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let snapshot = two_workspace_snapshot();
    state.set_snapshot(Box::new(snapshot));
    state.set_snooze_state(snooze_state(&["ws_1"], 1));

    let mut pane_for = |action| {
        let crate::api::schema::Method::PaneFocus(target) = state
            .endpoint_method_for_action(action)
            .expect("visible agent action")
        else {
            panic!("expected pane focus");
        };
        target.pane_id
    };
    assert_eq!(pane_for(KeybindAction::FocusAgent(0)), "pane_2");
    assert_eq!(pane_for(KeybindAction::NextAgent), "pane_2");
    assert_eq!(pane_for(KeybindAction::PreviousAgent), "pane_2");
}

#[test]
fn focus_snooze_recovery_bar_reserves_surface_and_keeps_both_actions_reachable() {
    for (cols, rows) in [(100, 30), (44, 20), (24, 8)] {
        let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
        state.set_snapshot(Box::new(two_workspace_snapshot()));
        let baseline = state.surface_size(cols, rows);
        state.set_focus_scope(Some(ClientFocusScope::StandaloneWorkspace {
            endpoint_id: ClientEndpointId::Local,
            boot_id: "boot-1".into(),
            workspace_id: "ws_1".into(),
        }));
        state.set_snooze_state(snooze_state(&["ws_2"], 1));
        state.set_pane_surface(surface());
        let frame = state.compose(cols, rows).expect("focused frame");
        let focused_size = state.surface_size(cols, rows);
        assert_eq!(focused_size.rows + 1, baseline.rows);
        let clear = state.hits.feature_clear_focus;
        let show = state.hits.feature_show_snoozed;
        assert!(clear.width > 0 && show.width > 0);
        assert_eq!(clear.y, rows - 1);
        assert_eq!(show.y, rows - 1);
        assert!(state.layout(cols, rows).pane_surface.bottom() <= clear.y);
        let footer: String = frame
            .cells
            .iter()
            .skip(usize::from(cols) * usize::from(rows - 1))
            .map(|cell| cell.symbol.as_str())
            .collect();
        assert!(footer.contains("Clear"), "{footer}");
        assert!(footer.contains("Show"), "{footer}");
        let show_outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(
            crossterm::event::MouseEvent {
                kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
                column: show.x,
                row: show.y,
                modifiers: KeyModifiers::empty(),
            },
        )]);
        assert!(show_outcome.actions.is_empty());
        assert!(show_outcome.requests.is_empty());
        assert!(matches!(
            state.overlay,
            Some(ClientShellOverlay::SnoozeManagement(_))
        ));
        state.overlay = None;
        state.compose(cols, rows).unwrap();
        let clear_outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(
            crossterm::event::MouseEvent {
                kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
                column: clear.x,
                row: clear.y,
                modifiers: KeyModifiers::empty(),
            },
        )]);
        assert!(state.focus_scope.is_none());
        assert_eq!(state.snooze_state.as_ref().unwrap().records.len(), 1);
        assert!(
            !clear_outcome.resize,
            "Snooze keeps the existing reserved row"
        );
        assert!(clear_outcome.requests.is_empty());
    }
}

#[test]
fn focus_snooze_recovery_bar_clear_last_filter_requests_resize() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    let baseline = state.surface_size(100, 30);
    state.set_focus_scope(Some(ClientFocusScope::StandaloneWorkspace {
        endpoint_id: ClientEndpointId::Local,
        boot_id: "boot-1".into(),
        workspace_id: "ws_1".into(),
    }));
    state.set_pane_surface(surface());
    state.compose(100, 30).unwrap();
    let clear = state.hits.feature_clear_focus;
    let outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: clear.x,
            row: clear.y,
            modifiers: KeyModifiers::empty(),
        },
    )]);
    assert!(outcome.resize);
    assert_eq!(state.surface_size(100, 30), baseline);
    assert!(
        state.pane_surface.is_none(),
        "old surface must be invalidated before reuse"
    );
}

#[test]
fn focus_snooze_recovery_bar_counts_explicit_persisted_records_once() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    let mut shared = snooze_state(&["ws_1", "ws_2"], 2);
    shared.persistence = Some(crate::api::schema::SnoozePersistenceInfo {
        records: vec![crate::api::schema::SnoozeStoredRecordInfo {
            record_id: "00000000-0000-4000-8000-000000000001".into(),
            scope: "workspace".into(),
            label: "Unavailable".into(),
            project_label: None,
            created_unix_ms: 0,
            deadline_unix_ms: 4_000_000,
            available: false,
            workspace_id: None,
            project_key: None,
        }],
        notice: None,
    });
    state.set_snooze_state(shared);
    let frame = state.compose(100, 30).unwrap();
    let text = composed_text(&frame);
    assert!(text.contains("Snoozed: 1"), "{text}");
    assert!(state.hits.feature_show_snoozed.width > 0);
}

#[test]
fn focus_snooze_recovery_bar_survives_unavailable_endpoint_without_mouse_leaks() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_focus_scope(Some(ClientFocusScope::StandaloneWorkspace {
        endpoint_id: ClientEndpointId::Local,
        boot_id: "boot-1".into(),
        workspace_id: "ws_1".into(),
    }));
    let frame = state.compose(44, 20).unwrap();
    assert!(composed_text(&frame).contains("Clear"));
    assert!(state.hits.feature_clear_focus.width > 0);
    state.config.mouse_capture = false;
    let frame = state.compose(44, 20).unwrap();
    assert!(composed_text(&frame).contains("Clear"));
    assert_eq!(state.hits.feature_clear_focus.width, 0);
    assert_eq!(state.hits.feature_show_snoozed.width, 0);
}

#[test]
fn focus_snooze_recovery_bar_does_not_label_remote_focus_from_local_id_collision() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    let mut snapshot = two_workspace_snapshot();
    snapshot.workspaces[0].label = "WRONG_LOCAL_LABEL".into();
    state.set_snapshot(Box::new(snapshot));
    state.set_focus_scope(Some(ClientFocusScope::StandaloneWorkspace {
        endpoint_id: ClientEndpointId::Ssh(
            crate::client::endpoint::ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        ),
        boot_id: "boot-1".into(),
        workspace_id: "ws_1".into(),
    }));
    let frame = state.compose(100, 30).unwrap();
    let footer: String = frame
        .cells
        .iter()
        .skip(100 * 29)
        .map(|cell| cell.symbol.as_str())
        .collect();
    assert!(!footer.contains("WRONG_LOCAL_LABEL"), "{footer}");
    assert!(footer.contains("Clear"), "{footer}");
}

#[test]
fn focus_snooze_recovery_bar_reserves_bottom_tab_and_sidebar_geometry() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.config.tab_bar_position = crate::config::TabBarPositionConfig::Bottom;
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_snooze_state(snooze_state(&["ws_2"], 1));
    let layout = state.layout(100, 30);
    assert!(layout.pane_surface.bottom() <= 29);
    assert!(layout.sidebar.bottom() <= 29);
    assert!(
        layout.tab_bar.bottom() <= 29,
        "footer cannot cover the bottom tabs"
    );
}

#[test]
fn focus_snooze_bindings_default_unbound_and_configured_actions_preserve_scope() {
    let defaults = Config::default();
    assert!(!defaults.keys.focus_project.has_values());
    assert!(!defaults.keys.clear_project_focus.has_values());
    assert!(!defaults.keys.snooze_workspace.has_values());
    assert!(!defaults.keys.snooze_project.has_values());
    assert!(!defaults.keys.show_snoozed.has_values());
    let config: Config = toml::from_str(
        r#"[keys]
focus_project = "ctrl+alt+f"
clear_project_focus = "ctrl+alt+l"
snooze_workspace = "ctrl+alt+s"
snooze_project = "ctrl+alt+p"
show_snoozed = "ctrl+alt+w"
"#,
    )
    .unwrap();
    let serialized = format!("[keys]\n{}", toml::to_string(&config.keys).unwrap());
    let restored: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(config.keys.focus_project, restored.keys.focus_project);
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&restored));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_pane_surface(surface());
    let key = |c| {
        crate::raw_input::RawInputEvent::Key(crate::input::TerminalKey::new(
            KeyCode::Char(c),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ))
    };
    let focus = state.handle_raw_events(vec![key('f')]);
    assert!(focus.resize);
    assert!(
        matches!(&state.focus_scope, Some(ClientFocusScope::StandaloneWorkspace { workspace_id, .. }) if workspace_id == "ws_1")
    );
    state.set_snooze_state(snooze_state(&["ws_2"], 1));
    state.handle_raw_events(vec![key('l')]);
    assert!(state.focus_scope.is_none());
    assert_eq!(state.snooze_state.as_ref().unwrap().records.len(), 1);
    state.handle_raw_events(vec![key('s')]);
    assert!(
        matches!(&state.overlay, Some(ClientShellOverlay::Snooze(picker)) if picker.workspace_id == "ws_1")
    );
    let cancel = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Esc, KeyModifiers::empty()),
    )]);
    assert!(cancel.actions.is_empty());
    assert!(state.overlay.is_none());
    assert_eq!(state.snooze_state.as_ref().unwrap().records.len(), 1);
    let show = state.handle_raw_events(vec![key('w')]);
    assert!(show.actions.is_empty());
    assert!(matches!(
        &state.overlay,
        Some(ClientShellOverlay::SnoozeManagement(_))
    ));
}

#[test]
fn focus_snooze_launcher_targets_navigate_selection_and_is_reachable_when_empty() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.mode = ClientShellMode::Navigate;
    state.navigate_workspace_id = Some("ws_2".into());
    state.record_binding(
        crate::input::KeybindMatch::Action(KeybindAction::FocusProject),
        &mut ClientShellInput::default(),
    );
    assert!(
        matches!(&state.focus_scope, Some(ClientFocusScope::Worktree { worktree_key, .. }) if worktree_key == "repo-b")
    );
    state.set_snooze_state(snooze_state(&["ws_1", "ws_2"], 2));
    assert!(state.empty_presentation);
    let mut hidden = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(KeybindAction::SnoozeWorkspace),
        &mut hidden,
    );
    assert!(
        state.overlay.is_none(),
        "empty-view launcher must not target the hidden active workspace"
    );
    assert!(hidden.actions.is_empty());
    state.toggle_global_menu();
    let frame = state.compose(100, 30).unwrap();
    assert!(composed_text(&frame).contains("Show snoozed items"));
    let index = super::super::global_menu::global_menu_items(state.snapshot.as_deref().unwrap())
        .iter()
        .position(|(_, action)| {
            *action
                == super::super::global_menu::ClientGlobalMenuAction::Binding(
                    KeybindAction::ShowSnoozed,
                )
        })
        .unwrap();
    let mut outcome = ClientShellInput::default();
    state.move_global_menu_selection(index as isize);
    let narrow = state.compose(24, 8).unwrap();
    assert!(composed_text(&narrow).contains("Show snoozed items"));
    state.activate_global_menu_item(index, &mut outcome);
    assert!(outcome.actions.is_empty());
    assert!(matches!(
        &state.overlay,
        Some(ClientShellOverlay::SnoozeManagement(_))
    ));
    assert_eq!(state.snooze_state.as_ref().unwrap().records.len(), 2);
}

#[test]
fn focus_snooze_launcher_cannot_target_stale_hidden_navigate_workspace() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_snooze_state(snooze_state(&["ws_2"], 1));
    state.mode = ClientShellMode::Navigate;
    state.navigate_workspace_id = Some("ws_2".into());
    let mut outcome = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(KeybindAction::SnoozeWorkspace),
        &mut outcome,
    );
    assert!(
        !matches!(&state.overlay, Some(ClientShellOverlay::Snooze(picker)) if picker.workspace_id == "ws_2")
    );
    assert!(outcome.actions.is_empty());
    assert_eq!(state.snooze_state.as_ref().unwrap().records.len(), 1);
}

fn reset_ready_state() -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(two_workspace_snapshot()));
    state.set_endpoint_methods(Some(vec!["snooze.reset".into(), "workspace.wake".into()]));
    state.set_snooze_state(snooze_state(&["ws_2"], 4));
    state.set_focus_scope(Some(ClientFocusScope::StandaloneWorkspace {
        endpoint_id: ClientEndpointId::Local,
        boot_id: "boot-1".into(),
        workspace_id: "ws_1".into(),
    }));
    state
}

fn open_reset(state: &mut ClientShellState) {
    let mut outcome = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(KeybindAction::ResetFocusSnooze),
        &mut outcome,
    );
    assert!(outcome.actions.is_empty() && outcome.requests.is_empty());
    assert!(state.overlay.is_some());
}

fn submit_reset(state: &mut ClientShellState) -> String {
    open_reset(state);
    let outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Enter, KeyModifiers::empty()),
    )]);
    assert!(
        outcome.requests.is_empty(),
        "reset must not emit terminal/lifecycle messages"
    );
    match &outcome.actions[..] {
        [ClientShellAction::Endpoint {
            endpoint_id,
            request,
            ..
        }] => {
            assert_eq!(endpoint_id, &ClientEndpointId::Local);
            match &request.method {
                crate::api::schema::Method::SnoozeReset(params) => {
                    assert_eq!(params.boot_id, "boot-1");
                    assert_eq!(params.expected_revision, 4);
                    assert!(params.confirmed);
                }
                other => panic!("unexpected reset method: {other:?}"),
            }
            request.id.clone()
        }
        other => panic!("expected only exact endpoint reset, got {other:?}"),
    }
}

#[test]
fn snooze_reset_client_cancel_and_rejection_preserve_focus_and_shared_state() {
    let mut state = reset_ready_state();
    let focus = state.focus_scope.clone();
    let shared = state.snooze_state.clone();
    open_reset(&mut state);
    let frame = state.compose(100, 30).unwrap();
    let text = composed_text(&frame);
    assert!(text.contains("Reset Focus"), "{text}");
    assert!(state.hits.overlay_primary.width > 0);
    state.config.mouse_capture = false;
    let frame = state.compose(100, 30).unwrap();
    assert!(composed_text(&frame).contains("Reset Focus"));
    assert_eq!(state.hits.overlay_primary.width, 0);
    assert_eq!(state.hits.overlay_cancel.width, 0);
    state.config.mouse_capture = true;
    let cancelled = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Esc, KeyModifiers::empty()),
    )]);
    assert!(cancelled.actions.is_empty() && cancelled.requests.is_empty());
    assert!(state.overlay.is_none());
    assert_eq!(state.focus_scope, focus);
    assert_eq!(state.snooze_state, shared);
    let id = submit_reset(&mut state);
    assert_eq!(
        state.focus_scope, focus,
        "do not clear Focus optimistically"
    );
    let (repaint, actions) = state.handle_endpoint_result(
        "boot-1",
        &id,
        Err(ClientShellEndpointError {
            code: Some("persistence_failed".into()),
            message: "disk unavailable".into(),
        }),
    );
    assert!(repaint && actions.is_empty());
    assert_eq!(state.focus_scope, focus);
    assert_eq!(state.snooze_state, shared);
    assert!(state.visible_endpoint_notice.is_some());
}

#[test]
fn snooze_reset_client_success_clears_only_captured_focus_and_updates_shared_state() {
    let mut state = reset_ready_state();
    let canonical = state.snapshot.clone();
    let id = submit_reset(&mut state);
    let (repaint, actions) = state.handle_endpoint_result(
        "boot-1",
        &id,
        Ok(crate::api::schema::ResponseResult::WorkspaceWake {
            state: snooze_state(&[], 5),
        }),
    );
    assert!(repaint);
    assert!(
        actions.is_empty(),
        "visible active workspace needs no selection or lifecycle action"
    );
    assert!(state.focus_scope.is_none());
    assert!(state.snooze_state.as_ref().unwrap().records.is_empty());
    assert!(state.is_workspace_id_visible("ws_1") && state.is_workspace_id_visible("ws_2"));
    assert_eq!(state.snapshot, canonical);
}

#[test]
fn snooze_reset_client_response_preserves_newer_focus_and_newer_snooze_broadcast() {
    for changed_focus in [false, true] {
        let mut state = reset_ready_state();
        let id = submit_reset(&mut state);
        let newer_focus = Some(ClientFocusScope::Worktree {
            endpoint_id: ClientEndpointId::Local,
            boot_id: "boot-1".into(),
            worktree_key: "repo-b".into(),
        });
        if changed_focus {
            state.set_focus_scope(newer_focus.clone());
        }
        state.set_snooze_state(snooze_state(&["ws_1"], 6));
        state.handle_endpoint_result(
            "boot-1",
            &id,
            Ok(crate::api::schema::ResponseResult::WorkspaceWake {
                state: snooze_state(&[], 5),
            }),
        );
        assert_eq!(
            state.focus_scope,
            if changed_focus { newer_focus } else { None }
        );
        assert_eq!(state.snooze_state.as_ref().unwrap().revision, 6);
        assert!(!state.is_workspace_id_visible("ws_1"));
    }
}

#[test]
fn snooze_reset_client_stale_boot_and_wrong_response_do_not_clear_focus() {
    for wrong_response in [false, true] {
        let mut state = reset_ready_state();
        let focus = state.focus_scope.clone();
        let id = submit_reset(&mut state);
        let mut response = snooze_state(&[], 5);
        if wrong_response {
            response.boot_id = "other-boot".into();
        }
        state.handle_endpoint_result(
            if wrong_response {
                "boot-1"
            } else {
                "other-boot"
            },
            &id,
            Ok(crate::api::schema::ResponseResult::WorkspaceWake { state: response }),
        );
        assert_eq!(state.focus_scope, focus);
        assert_eq!(state.snooze_state.as_ref().unwrap().revision, 4);
    }
}

#[test]
fn snooze_reset_client_empty_view_binding_and_stale_confirmation_are_safe() {
    assert!(!Config::default().keys.reset_focus_snooze.has_values());
    let config: Config = toml::from_str("[keys]\nreset_focus_snooze = \"ctrl+alt+r\"\n").unwrap();
    let serialized = format!("[keys]\n{}", toml::to_string(&config.keys).unwrap());
    let restored: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(
        config.keys.reset_focus_snooze,
        restored.keys.reset_focus_snooze
    );
    let mut state = reset_ready_state();
    state.config = ClientShellConfig::from_config(&restored);
    state.set_snooze_state(snooze_state(&["ws_1", "ws_2"], 4));
    assert!(state.empty_presentation);
    let outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(
            KeyCode::Char('r'),
            KeyModifiers::CONTROL | KeyModifiers::ALT,
        ),
    )]);
    assert!(state.overlay.is_some());
    assert!(outcome.requests.is_empty() && outcome.actions.is_empty());
    let mut newer = two_workspace_snapshot();
    newer.boot_id = "replacement-boot".into();
    state.set_snapshot(Box::new(newer));
    let outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Enter, KeyModifiers::empty()),
    )]);
    assert!(outcome.actions.is_empty() && outcome.requests.is_empty());
}

#[test]
fn snooze_reset_client_focus_only_and_unsupported_server_remain_recoverable() {
    let mut state = reset_ready_state();
    state.set_snooze_state(snooze_state(&[], 4));
    let focus = state.focus_scope.clone();
    let id = submit_reset(&mut state);
    state.handle_endpoint_result(
        "boot-1",
        &id,
        Ok(crate::api::schema::ResponseResult::WorkspaceWake {
            state: snooze_state(&[], 4),
        }),
    );
    assert!(
        state.focus_scope.is_none(),
        "successful empty server reset still clears local Focus"
    );
    state.set_focus_scope(focus.clone());
    state.set_endpoint_methods(Some(vec!["workspace.wake".into()]));
    open_reset(&mut state);
    let outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(KeyCode::Enter, KeyModifiers::empty()),
    )]);
    assert!(outcome.actions.is_empty() && outcome.requests.is_empty());
    assert_eq!(state.focus_scope, focus);
    assert!(state.visible_endpoint_notice.is_some());
}

#[test]
fn snooze_reset_client_recovery_menu_keeps_wake_and_reset_distinct() {
    for reset in [false, true] {
        let mut state = reset_ready_state();
        let focus = state.focus_scope.clone();
        state.open_snooze_recovery(0, 0);
        assert!(matches!(
            state.overlay,
            Some(ClientShellOverlay::SnoozeManagement(_))
        ));
        state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
            crate::input::TerminalKey::new(
                KeyCode::Char(if reset { 'r' } else { 'a' }),
                KeyModifiers::empty(),
            ),
        )]);
        let frame = state.compose(100, 30).unwrap();
        if reset {
            let text = composed_text(&frame);
            assert!(
                text.contains("Local"),
                "reset confirmation must identify shared server: {text}"
            );
        }
        let outcome = state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
            crate::input::TerminalKey::new(KeyCode::Enter, KeyModifiers::empty()),
        )]);
        let [ClientShellAction::Endpoint { request, .. }] = outcome.actions.as_slice() else {
            panic!("one shared request");
        };
        assert!(if reset {
            matches!(request.method, crate::api::schema::Method::SnoozeReset(_))
        } else {
            matches!(request.method, crate::api::schema::Method::WorkspaceWake(_))
        });
        assert_eq!(state.focus_scope, focus);
        assert!(outcome.requests.is_empty());
    }
}

#[test]
fn snooze_reset_client_confirmation_counts_unavailable_records_and_requires_metadata() {
    let mut state = reset_ready_state();
    let mut shared = snooze_state(&[], 4);
    shared.persistence = Some(crate::api::schema::SnoozePersistenceInfo {
        records: vec![crate::api::schema::SnoozeStoredRecordInfo {
            record_id: "00000000-0000-4000-8000-000000000001".into(),
            scope: "workspace".into(),
            label: "unavailable-space".into(),
            project_label: None,
            created_unix_ms: 0,
            deadline_unix_ms: 4_000_000,
            available: false,
            workspace_id: None,
            project_key: None,
        }],
        notice: None,
    });
    state.set_snooze_state(shared);
    open_reset(&mut state);
    let Some(ClientShellOverlay::ConfirmWakeSharedSnoozes(confirm)) = &state.overlay else {
        panic!("confirmation");
    };
    assert_eq!(confirm.count, 1);
    state.overlay = None;
    state.snooze_state = None;
    let mut outcome = ClientShellInput::default();
    state.record_binding(
        crate::input::KeybindMatch::Action(KeybindAction::ResetFocusSnooze),
        &mut outcome,
    );
    assert!(outcome.actions.is_empty() && outcome.requests.is_empty());
    assert!(
        state.overlay.is_none(),
        "cannot invent a reset revision before server metadata arrives"
    );
    assert!(state.endpoint_error.is_some() || state.visible_endpoint_notice.is_some());
}

#[path = "snooze_management.rs"]
mod snooze_management_tests;
