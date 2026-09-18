use super::*;

use crate::api::schema::{
    EmptyParams, ErrorResponse, Method, ProjectSnoozeParams, ProjectWakeParams, Request,
    ResponseResult, SuccessResponse, WorkspaceSnoozeParams, WorkspaceSnoozeState,
};
use crate::protocol::ServerMessage;
use crate::workspace::{Workspace, WorktreeSpaceMembership};

fn workspace_with_project(name: &str, project_key: &str) -> Workspace {
    let mut workspace = Workspace::test_new(name);
    workspace.worktree_space = Some(WorktreeSpaceMembership {
        key: project_key.into(),
        label: name.into(),
        repo_root: "/repo/herdr".into(),
        checkout_path: format!("/repo/{name}").into(),
        is_linked_worktree: true,
    });
    workspace
}

fn api_request(server: &mut HeadlessServer, id: &str, method: Method) -> String {
    let (respond_to, response_rx) = std::sync::mpsc::channel();
    server.handle_api_request_with_shutdown_check(crate::api::ApiRequestMessage {
        request: Request {
            id: id.into(),
            method,
        },
        respond_to,
        response_write_complete: None,
        stream_active: None,
    });
    response_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("api response")
}

fn state_from_success(response: &str) -> WorkspaceSnoozeState {
    let response: SuccessResponse = serde_json::from_str(response).expect("success response");
    match response.result {
        ResponseResult::WorkspaceSnooze { state } | ResponseResult::WorkspaceWake { state } => {
            state
        }
        other => panic!("expected snooze state response, got {other:?}"),
    }
}

fn pane_key_press() -> crate::protocol::ClientPaneInputEvent {
    crate::protocol::ClientPaneInputEvent::Key {
        code: crate::protocol::ClientKeyCode::Char('x'),
        modifiers: 0,
        kind: crate::protocol::ClientKeyKind::Press,
        repeat_count: 1,
        shifted_codepoint: None,
        generated_text: None,
        tracks_release: true,
        physical_key_id: Some(0x2d),
        windows_record: None,
    }
}

#[test]
fn project_snooze_raw_api_lists_record_and_rejects_changed_key() {
    let mut server = test_headless_server();
    let workspace = workspace_with_project("current", "project-key");
    let workspace_id = workspace.id.clone();
    server.app.state.workspaces = vec![workspace];
    server.app.state.active = Some(0);
    server.app.state.selected = 0;

    let boot_id = server.client_shell_boot_id.clone();
    let state = state_from_success(&api_request(
        &mut server,
        "project-snooze",
        Method::ProjectSnooze(ProjectSnoozeParams {
            workspace_id: workspace_id.clone(),
            project_key: "project-key".into(),
            boot_id: boot_id.clone(),
            duration_seconds: Some(5),
            deadline_unix_ms: None,
        }),
    ));
    assert_eq!(state.boot_id, boot_id);
    assert_eq!(state.revision, 1);
    assert!(state.records.is_empty());
    assert_eq!(state.project_records.len(), 1);
    assert_eq!(state.project_records[0].project_key, "project-key");
    assert_eq!(state.project_records[0].boot_id, state.boot_id);

    let listed = state_from_success(&api_request(
        &mut server,
        "snooze-list",
        Method::SnoozeList(EmptyParams {}),
    ));
    assert_eq!(listed, state);

    let stale_boot = api_request(
        &mut server,
        "stale-boot",
        Method::ProjectSnooze(ProjectSnoozeParams {
            workspace_id: workspace_id.clone(),
            project_key: "project-key".into(),
            boot_id: "old-boot".into(),
            duration_seconds: Some(5),
            deadline_unix_ms: None,
        }),
    );
    let stale_boot: ErrorResponse = serde_json::from_str(&stale_boot).expect("error response");
    assert_eq!(stale_boot.error.code, "stale_boot");
    assert_eq!(server.workspace_snoozes.state(), state);

    server.app.state.workspaces[0]
        .worktree_space
        .as_mut()
        .expect("project membership")
        .key = "changed-key".into();
    let rejected = api_request(
        &mut server,
        "changed-key",
        Method::ProjectSnooze(ProjectSnoozeParams {
            workspace_id,
            project_key: "project-key".into(),
            boot_id,
            duration_seconds: Some(5),
            deadline_unix_ms: None,
        }),
    );
    let rejected: ErrorResponse = serde_json::from_str(&rejected).expect("error response");
    assert_eq!(rejected.error.code, "stale_target");
    assert_eq!(server.workspace_snoozes.state(), state);
}

#[test]
fn project_and_workspace_snooze_overlap_and_stale_project_wake_preserves_records() {
    let mut server = test_headless_server();
    let workspace = workspace_with_project("current", "project-key");
    let workspace_id = workspace.id.clone();
    server.app.state.workspaces = vec![workspace];
    server.app.state.active = Some(0);
    server.app.state.selected = 0;
    let boot_id = server.client_shell_boot_id.clone();

    let first_project = state_from_success(&api_request(
        &mut server,
        "project-first",
        Method::ProjectSnooze(ProjectSnoozeParams {
            workspace_id: workspace_id.clone(),
            project_key: "project-key".into(),
            boot_id: boot_id.clone(),
            duration_seconds: Some(30),
            deadline_unix_ms: None,
        }),
    ));
    let first_project_revision = first_project.project_records[0].revision;
    let overlapping = state_from_success(&api_request(
        &mut server,
        "workspace-overlap",
        Method::WorkspaceSnooze(WorkspaceSnoozeParams {
            workspace_id: workspace_id.clone(),
            boot_id: boot_id.clone(),
            duration_seconds: Some(30),
            deadline_unix_ms: None,
        }),
    ));
    assert_eq!(overlapping.records.len(), 1);
    assert_eq!(overlapping.project_records.len(), 1);

    let newer_project = state_from_success(&api_request(
        &mut server,
        "project-newer",
        Method::ProjectSnooze(ProjectSnoozeParams {
            workspace_id: workspace_id.clone(),
            project_key: "project-key".into(),
            boot_id: boot_id.clone(),
            duration_seconds: Some(30),
            deadline_unix_ms: None,
        }),
    ));
    assert!(newer_project.project_records[0].revision > first_project_revision);
    let stale = api_request(
        &mut server,
        "project-stale-wake",
        Method::ProjectWake(ProjectWakeParams {
            project_key: "project-key".into(),
            boot_id: boot_id.clone(),
            expected_revision: first_project_revision,
        }),
    );
    let stale: ErrorResponse = serde_json::from_str(&stale).expect("stale wake response");
    assert_eq!(stale.error.code, "stale_revision");
    assert!(server.workspace_snoozes.is_project_snoozed("project-key"));

    let current = server
        .workspace_snoozes
        .project_record_revision("project-key")
        .expect("project record");
    let woken = state_from_success(&api_request(
        &mut server,
        "project-wake",
        Method::ProjectWake(ProjectWakeParams {
            project_key: "project-key".into(),
            boot_id,
            expected_revision: current,
        }),
    ));
    assert!(woken.project_records.is_empty());
    assert_eq!(woken.records.len(), 1);
    assert_eq!(woken.records[0].workspace_id, workspace_id);
}

#[test]
fn project_wake_can_remove_orphan_record_without_live_workspace() {
    let mut server = test_headless_server();
    let workspace = workspace_with_project("orphan", "project-key");
    let workspace_id = workspace.id.clone();
    server.app.state.workspaces = vec![workspace];
    server.app.state.active = Some(0);
    let boot_id = server.client_shell_boot_id.clone();
    let snoozed = state_from_success(&api_request(
        &mut server,
        "orphan-snooze",
        Method::ProjectSnooze(ProjectSnoozeParams {
            workspace_id,
            project_key: "project-key".into(),
            boot_id: boot_id.clone(),
            duration_seconds: Some(30),
            deadline_unix_ms: None,
        }),
    ));
    let revision = snoozed.project_records[0].revision;
    server.app.state.workspaces.clear();

    let woken = state_from_success(&api_request(
        &mut server,
        "orphan-wake",
        Method::ProjectWake(ProjectWakeParams {
            project_key: "project-key".into(),
            boot_id,
            expected_revision: revision,
        }),
    ));
    assert!(woken.records.is_empty());
    assert!(woken.project_records.is_empty());
}

#[test]
fn project_expiry_preserves_later_workspace_and_tracks_earliest_deadline() {
    let mut manager = crate::server::workspace_snooze::WorkspaceSnoozeManager::new("boot".into());
    manager
        .project_snooze("project-key".into(), None, Some(2_000), 1_000)
        .expect("project snooze");
    manager
        .snooze("workspace-id".into(), None, Some(3_000), 1_000)
        .expect("workspace snooze");

    assert_eq!(manager.next_deadline(), Some(2_000));
    assert!(!manager.expire_due(1_999));
    assert!(manager.is_project_snoozed("project-key"));
    assert!(manager.is_snoozed("workspace-id"));

    assert!(manager.expire_due(2_000));
    assert!(!manager.is_project_snoozed("project-key"));
    assert!(manager.is_snoozed("workspace-id"));
    assert_eq!(manager.next_deadline(), Some(3_000));

    assert!(!manager.expire_due(1_000));
    assert!(!manager.is_project_snoozed("project-key"));
    assert!(manager.is_snoozed("workspace-id"));
}

#[tokio::test]
async fn project_snooze_endpoint_hides_future_matching_workspace_and_preserves_unrelated_input() {
    let mut server = test_headless_server();
    let current = workspace_with_project("current", "project-key");
    let mut unrelated = Workspace::test_new("unrelated");
    let unrelated_pane = unrelated.focused_pane_id().expect("unrelated pane");
    let (unrelated_runtime, _unrelated_input) =
        crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
            80,
            24,
            0,
            b"unrelated",
            4,
        );
    unrelated.insert_test_runtime(unrelated_pane, unrelated_runtime);
    server.app.state.workspaces = vec![current, unrelated];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(0);
    server.app.state.selected = 0;

    let (writer, control_rx, _render_rx) = test_client_writer();
    let client_id = 7;
    assert!(
        server.handle_server_event(ServerEvent::ClientShellConnected {
            surface_reuse: false,
            surface_delta: false,
            client_id,
            surface_cols: 80,
            surface_rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            pixel_mouse: false,
            direct_graphics: false,
            endpoint_keybindings: false,
            mouse_capture: false,
            surface_active: true,
            writer,
        })
    );
    let _ = control_rx.recv().expect("initial snapshot");
    assert!(!server.handle_server_event(ServerEvent::ClientShellSnoozeSubscribe { client_id }));
    let _ = control_rx.recv().expect("initial snooze state");

    let boot_id = server.client_shell_boot_id.clone();
    let current_id = server.app.public_workspace_id(0);
    assert!(
        server.handle_server_event(ServerEvent::ClientShellEndpointRequest {
            client_id,
            boot_id: boot_id.clone(),
            request: Box::new(Request {
                id: "endpoint-project-snooze".into(),
                method: Method::ProjectSnooze(ProjectSnoozeParams {
                    workspace_id: current_id,
                    project_key: "project-key".into(),
                    boot_id: boot_id.clone(),
                    duration_seconds: Some(30),
                    deadline_unix_ms: None,
                }),
            }),
        })
    );
    let ServerMessage::EndpointControl { kind, data } =
        read_server_message(control_rx.recv().expect("project snooze broadcast"))
    else {
        panic!("expected project snooze broadcast");
    };
    assert_eq!(kind, "workspace_snooze.state");
    let state: WorkspaceSnoozeState = serde_json::from_str(&data).expect("snooze state");
    assert_eq!(state.project_records.len(), 1);
    let _ = control_rx.recv().expect("project snooze response");

    let mut future = workspace_with_project("future", "project-key");
    let future_pane = future.focused_pane_id().expect("future pane");
    let (future_runtime, mut future_input) =
        crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
            80, 24, 0, b"future", 4,
        );
    future.insert_test_runtime(future_pane, future_runtime);
    server.app.state.workspaces.push(future);
    server.app.state.ensure_test_terminals();
    let future_id = server.app.public_workspace_id(2);
    let future_tab = server.app.public_tab_id(2, 0).expect("future tab");
    server
        .clients
        .get_mut(&client_id)
        .expect("shell client")
        .shell_location
        .as_mut()
        .expect("shell location")
        .focus_tab(future_id, future_tab);
    assert!(!server.shell_client_views_pane(client_id, 2, future_pane));

    let unrelated_id = server.app.public_workspace_id(1);
    let unrelated_tab = server.app.public_tab_id(1, 0).expect("unrelated tab");
    server
        .clients
        .get_mut(&client_id)
        .expect("shell client")
        .shell_location
        .as_mut()
        .expect("shell location")
        .focus_tab(unrelated_id, unrelated_tab);
    assert!(server.shell_client_views_pane(client_id, 1, unrelated_pane));

    let future_id = server.app.public_workspace_id(2);
    let future_tab = server.app.public_tab_id(2, 0).expect("future tab");
    server
        .clients
        .get_mut(&client_id)
        .expect("shell client")
        .shell_location
        .as_mut()
        .expect("shell location")
        .focus_tab(future_id, future_tab);
    let future_id = server
        .app
        .public_pane_id(2, future_pane)
        .expect("future pane id");
    assert!(
        !server.handle_server_event(ServerEvent::ClientShellPaneInput {
            client_id,
            pane_id: future_id,
            events: vec![pane_key_press()],
        })
    );
    assert!(future_input.try_recv().is_err());

    shutdown_test_runtimes(&mut server);
}

#[test]
fn project_snooze_persistent_handoff_refreshes_anchors() {
    use crate::server::{snooze_store::SnoozeStore, workspace_snooze::WorkspaceSnoozeManager};
    let mut server = test_headless_server();
    let path = std::env::temp_dir()
        .join(format!("herdr-snooze-handoff-{}", uuid::Uuid::new_v4()))
        .join("snooze.json");
    let now = crate::server::workspace_snooze::now_unix_ms();
    server.app.state.workspaces = vec![workspace_with_project("old", "group")];
    let (store, notice) = SnoozeStore::load(
        path.clone(),
        &server.app.state.workspaces,
        &mut server.workspace_snoozes,
        now,
    );
    assert!(notice.is_none());
    server.snooze_store = Some(store);
    assert!(server
        .commit_snooze_change(|manager| manager
            .project_snooze("group".into(), Some(3600), None, now)
            .map(|_| ()))
        .is_ok());
    server.app.state.workspaces = vec![workspace_with_project("new", "group")];
    server.app.state.session_dirty = false;
    server.flush_snooze_store_for_handoff().unwrap();
    let mut restored = WorkspaceSnoozeManager::new("new-boot".into());
    let (store, notice) = SnoozeStore::load(
        path.clone(),
        &server.app.state.workspaces,
        &mut restored,
        now,
    );
    assert!(notice.is_none());
    assert!(restored.is_project_snoozed("group"));
    assert!(store.info(&server.app.state.workspaces, None).records[0].available);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn project_snooze_repeated_deadline_cannot_acknowledge_failed_membership_save() {
    use crate::server::snooze_store::SnoozeStore;
    let mut server = test_headless_server();
    let path = std::env::temp_dir()
        .join(format!("herdr-snooze-retry-{}", uuid::Uuid::new_v4()))
        .join("snooze.json");
    let now = crate::server::workspace_snooze::now_unix_ms();
    let deadline = now + 3_600_000;
    server.app.state.workspaces = vec![workspace_with_project("old", "group")];
    let (store, _) = SnoozeStore::load(
        path.clone(),
        &server.app.state.workspaces,
        &mut server.workspace_snoozes,
        now,
    );
    server.snooze_store = Some(store);
    assert!(server
        .commit_snooze_change(|manager| manager
            .project_snooze("group".into(), None, Some(deadline), now)
            .map(|_| ()))
        .is_ok());
    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();
    server.app.state.workspaces = vec![workspace_with_project("new", "group")];
    server.app.state.session_dirty = true;
    server.reconcile_snooze_membership_if_dirty();
    assert!(server.snooze_notice.is_some());
    assert!(server
        .commit_snooze_change(|manager| manager
            .project_snooze("group".into(), None, Some(deadline), now)
            .map(|_| ()))
        .is_err());
    assert!(server.flush_snooze_store_for_handoff().is_err());
    std::fs::remove_dir(&path).unwrap();
    assert!(server
        .commit_snooze_change(|manager| manager
            .project_snooze("group".into(), None, Some(deadline), now)
            .map(|_| ()))
        .is_ok());
    assert!(server.snooze_notice.is_none());
    assert!(path.is_file());
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

fn snooze_reset_request(
    server: &mut HeadlessServer,
    boot_id: String,
    expected_revision: u64,
    confirmed: bool,
) -> serde_json::Value {
    serde_json::from_str(&api_request(
        server,
        "reset",
        Method::SnoozeReset(crate::api::schema::SnoozeResetParams {
            boot_id,
            expected_revision,
            confirmed,
        }),
    ))
    .unwrap()
}

#[test]
fn snooze_reset_requires_confirmation_current_boot_and_revision() {
    let mut server = test_headless_server();
    server.app.state.workspaces = vec![workspace_with_project("workspace", "group")];
    let now = crate::server::workspace_snooze::now_unix_ms();
    server
        .workspace_snoozes
        .project_snooze("group".into(), Some(3600), None, now)
        .unwrap();
    let boot = server.client_shell_boot_id.clone();
    let revision = server.workspace_snoozes.revision();
    for (target_boot, target_revision, confirmed) in [
        (boot.clone(), revision, false),
        ("earlier-boot".into(), revision, true),
        (boot.clone(), revision + 1, true),
    ] {
        let result = snooze_reset_request(&mut server, target_boot, target_revision, confirmed);
        assert!(result.get("error").is_some(), "{result}");
        assert!(server.workspace_snoozes.is_project_snoozed("group"));
    }
    let lifetime = server.app.state.workspaces[0].lifetime_id.clone();
    let result = snooze_reset_request(&mut server, boot, revision, true);
    assert!(result.get("error").is_none(), "{result}");
    assert!(!server.workspace_snoozes.is_project_snoozed("group"));
    assert_eq!(server.app.state.workspaces[0].lifetime_id, lifetime);
}

#[test]
fn snooze_reset_recovers_only_feature_file_after_unknown_schema() {
    use crate::server::snooze_store::SnoozeStore;
    let mut server = test_headless_server();
    let root = std::env::temp_dir().join(format!("herdr-snooze-reset-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let path = root.join("snooze.json");
    let sibling = root.join("session-sentinel.json");
    std::fs::write(&sibling, b"KEEP GENERAL STATE").unwrap();
    std::fs::write(&path, br#"{"version":999,"records":[]}"#).unwrap();
    let now = crate::server::workspace_snooze::now_unix_ms();
    let (store, notice) = SnoozeStore::load(
        path.clone(),
        &server.app.state.workspaces,
        &mut server.workspace_snoozes,
        now,
    );
    assert!(notice.is_some());
    server.snooze_store = Some(store);
    server.snooze_notice = notice;
    let revision = server.workspace_snoozes.revision();
    let quarantined = std::fs::read_dir(&root)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("snooze.json.corrupt-")
        })
        .expect("unknown schema is preserved before recovery");
    let original = std::fs::read(&quarantined).unwrap();
    assert!(!path.exists());
    let boot = server.client_shell_boot_id.clone();
    let result = snooze_reset_request(&mut server, boot, revision, true);
    assert!(result.get("error").is_none(), "{result}");
    assert!(server.snooze_notice.is_none());
    let (_, notice) = SnoozeStore::load(
        path,
        &server.app.state.workspaces,
        &mut server.workspace_snoozes,
        now,
    );
    assert!(notice.is_none());
    assert_eq!(std::fs::read(&sibling).unwrap(), b"KEEP GENERAL STATE");
    assert_eq!(std::fs::read(quarantined).unwrap(), original);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn snooze_reset_clears_live_and_unavailable_records_only_after_successful_save() {
    use crate::server::snooze_store::SnoozeStore;
    let mut server = test_headless_server();
    let path = std::env::temp_dir()
        .join(format!(
            "herdr-snooze-reset-failure-{}",
            uuid::Uuid::new_v4()
        ))
        .join("snooze.json");
    let now = crate::server::workspace_snooze::now_unix_ms();
    server.app.state.workspaces = vec![workspace_with_project("old", "group")];
    let old_id = server.app.state.workspaces[0].id.clone();
    let (store, _) = SnoozeStore::load(
        path.clone(),
        &server.app.state.workspaces,
        &mut server.workspace_snoozes,
        now,
    );
    server.snooze_store = Some(store);
    assert!(server
        .commit_snooze_change(|manager| {
            manager.project_snooze("group".into(), Some(3600), None, now)?;
            manager.snooze(old_id.clone(), Some(3600), None, now)?;
            Ok(())
        })
        .is_ok());
    server.app.state.workspaces = vec![workspace_with_project("new", "group")];
    let new_id = server.app.state.workspaces[0].id.clone();
    server.app.state.session_dirty = true;
    server.reconcile_snooze_membership_if_dirty();
    let records = server.snooze_state().persistence.unwrap().records;
    assert_eq!(records.len(), 2);
    assert!(records.iter().any(|record| !record.available));
    let backup = path.with_extension("backup");
    std::fs::rename(&path, &backup).unwrap();
    std::fs::create_dir(&path).unwrap();
    let boot = server.client_shell_boot_id.clone();
    let revision = server.workspace_snoozes.revision();
    let failed = snooze_reset_request(&mut server, boot.clone(), revision, true);
    assert!(failed.get("error").is_some(), "{failed}");
    assert_eq!(server.snooze_state().persistence.unwrap().records, records);
    assert!(server.workspace_snoozes.is_project_snoozed("group"));
    std::fs::remove_dir(&path).unwrap();
    std::fs::rename(&backup, &path).unwrap();
    let success = snooze_reset_request(&mut server, boot, revision, true);
    assert!(success.get("error").is_none(), "{success}");
    let state = server.snooze_state();
    assert!(state.records.is_empty() && state.project_records.is_empty());
    assert!(state.persistence.unwrap().records.is_empty());
    assert_eq!(server.app.state.workspaces.len(), 1);
    assert_eq!(server.app.state.workspaces[0].id, new_id);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn snooze_reset_recovers_write_block_only_after_path_is_usable() {
    use crate::server::snooze_store::SnoozeStore;
    let mut server = test_headless_server();
    let root = std::env::temp_dir().join(format!("herdr-snooze-blocked-{}", uuid::Uuid::new_v4()));
    let path = root.join("snooze.json");
    std::fs::create_dir_all(&path).unwrap();
    let now = crate::server::workspace_snooze::now_unix_ms();
    let (store, notice) = SnoozeStore::load(
        path.clone(),
        &server.app.state.workspaces,
        &mut server.workspace_snoozes,
        now,
    );
    assert!(store.is_write_blocked());
    server.snooze_store = Some(store);
    server.snooze_notice = notice;
    let revision = server.workspace_snoozes.revision();
    assert!(server.commit_snooze_wake_all(revision, true).is_err());
    assert!(server.commit_snooze_reset(revision, true).is_err());
    assert!(server.snooze_store.as_ref().unwrap().is_write_blocked());
    assert!(path.is_dir());
    std::fs::remove_dir(&path).unwrap();
    assert!(server.commit_snooze_wake_all(revision, true).is_err());
    assert!(server.commit_snooze_reset(revision, true).is_ok());
    assert!(!server.snooze_store.as_ref().unwrap().is_write_blocked());
    assert!(server.snooze_notice.is_none());
    assert!(path.is_file());
    std::fs::remove_dir_all(root).unwrap();
}
