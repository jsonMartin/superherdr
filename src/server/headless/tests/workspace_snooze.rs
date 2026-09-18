use super::*;

use crate::api::schema::{
    Method, Request, SuccessResponse, WorkspaceSnoozeParams, WorkspaceSnoozeState,
    WorkspaceWakeParams,
};
use crate::protocol::ServerMessage;

#[tokio::test]
async fn snooze_subscribe_and_initial_state() {
    let mut server = test_headless_server();
    let (writer, control_rx, _render_rx) = test_client_writer();
    let client_id = 1;

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
    assert!(server.snooze_subscribers.contains(&client_id));

    let ServerMessage::EndpointControl { kind, data } =
        read_server_message(control_rx.recv().expect("snooze state"))
    else {
        panic!("expected endpoint control");
    };
    assert_eq!(kind, "workspace_snooze.state");
    let state: WorkspaceSnoozeState = serde_json::from_str(&data).expect("decode state");
    assert_eq!(state.boot_id, server.client_shell_boot_id);
    assert_eq!(state.revision, 0);
    assert!(state.records.is_empty());
    shutdown_test_runtimes(&mut server);
}

#[tokio::test]
async fn two_clients_receive_authoritative_workspace_snooze_state() {
    let mut server = test_headless_server();
    let mut workspace_a = crate::workspace::Workspace::test_new("ws_1");
    let pane_a = workspace_a.focused_pane_id().expect("workspace a pane");
    let (runtime_a, _input_a) =
        crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
            80,
            24,
            0,
            b"workspace-a",
            4,
        );
    workspace_a.insert_test_runtime(pane_a, runtime_a);
    let mut workspace_b = crate::workspace::Workspace::test_new("ws_2");
    let pane_b = workspace_b.focused_pane_id().expect("workspace b pane");
    let (runtime_b, mut input_b) =
        crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
            80,
            24,
            0,
            b"\x1b[>3uworkspace-b",
            4,
        );
    workspace_b.insert_test_runtime(pane_b, runtime_b);
    server.app.state.workspaces = vec![workspace_a, workspace_b];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(1);
    server.app.state.selected = 1;
    server.app.state.mode = crate::app::Mode::Terminal;
    for workspace_index in 0..2 {
        let pane_id = server.app.state.workspaces[workspace_index]
            .focused_pane_id()
            .expect("agent pane");
        let terminal_id = server.app.state.workspaces[workspace_index]
            .terminal_id(pane_id)
            .expect("agent terminal")
            .clone();
        server
            .app
            .state
            .terminals
            .get_mut(&terminal_id)
            .expect("test terminal state")
            .set_detected_state(
                Some(crate::detect::Agent::Claude),
                crate::detect::AgentState::Working,
            );
    }
    let (writer_a, control_rx_a, _render_rx_a) = test_client_writer();
    let (writer_b, control_rx_b, _render_rx_b) = test_client_writer();

    for (client_id, writer) in [(10, writer_a), (20, writer_b)] {
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
    }
    let snapshot_a = client_shell_snapshot(recv_server_message(&control_rx_a, "snapshot a"));
    let snapshot_b = client_shell_snapshot(recv_server_message(&control_rx_b, "snapshot b"));
    assert_eq!(snapshot_a, snapshot_b);
    assert_eq!(snapshot_a.workspaces.len(), 2);
    assert_eq!(snapshot_a.agents.len(), 2);

    let mut shell_a = crate::client::ClientShellState::new(
        crate::client::ClientShellConfig::from_config(&crate::config::Config::default()),
    );
    let mut shell_b = crate::client::ClientShellState::new(
        crate::client::ClientShellConfig::from_config(&crate::config::Config::default()),
    );
    shell_a.set_snapshot(snapshot_a.clone());
    shell_b.set_snapshot(snapshot_b.clone());

    let boot_id = server.client_shell_boot_id.clone();
    let workspace_a_id = server.app.public_workspace_id(0);
    let workspace_b_id = server.app.public_workspace_id(1);
    let scope_a = crate::client::ClientFocusScope::StandaloneWorkspace {
        endpoint_id: crate::client::endpoint::ClientEndpointId::Local,
        boot_id: boot_id.clone(),
        workspace_id: workspace_a_id.clone(),
    };
    let scope_b = crate::client::ClientFocusScope::StandaloneWorkspace {
        endpoint_id: crate::client::endpoint::ClientEndpointId::Local,
        boot_id: boot_id.clone(),
        workspace_id: workspace_b_id.clone(),
    };
    let focus_actions_a = shell_a.set_focus_scope(Some(scope_a.clone()));
    dispatch_focus_actions(&mut server, 10, focus_actions_a);
    assert_eq!(
        server.clients[&10]
            .shell_location
            .as_ref()
            .and_then(|location| location.focused_workspace_id.as_deref()),
        Some(workspace_a_id.as_str())
    );
    let focus_actions_b = shell_b.set_focus_scope(Some(scope_b.clone()));
    assert!(focus_actions_b.is_empty());
    assert_eq!(
        server.clients[&20]
            .shell_location
            .as_ref()
            .and_then(|location| location.focused_workspace_id.as_deref()),
        Some(workspace_b_id.as_str())
    );
    let _ = server.handle_server_event(ServerEvent::ClientShellSnoozeSubscribe { client_id: 10 });
    apply_snooze_control(
        &mut shell_a,
        recv_server_message(&control_rx_a, "initial state a"),
    );
    let _ = server.handle_server_event(ServerEvent::ClientShellSnoozeSubscribe { client_id: 20 });
    apply_snooze_control(
        &mut shell_b,
        recv_server_message(&control_rx_b, "initial state b"),
    );

    let inventory_before = server
        .app
        .state
        .workspaces
        .iter()
        .map(|workspace| workspace.id.clone())
        .collect::<Vec<_>>();
    let deadline = crate::server::workspace_snooze::now_unix_ms() + 5_000;
    let _ = server.handle_server_event(ServerEvent::ClientShellEndpointRequest {
        client_id: 10,
        boot_id: boot_id.clone(),
        request: Box::new(Request {
            id: "snooze".into(),
            method: Method::WorkspaceSnooze(WorkspaceSnoozeParams {
                workspace_id: workspace_b_id.clone(),
                boot_id: boot_id.clone(),
                duration_seconds: None,
                deadline_unix_ms: Some(deadline),
            }),
        }),
    });
    let state_message_a = recv_snooze_control(&control_rx_a, "broadcast a");
    let state_message_b = recv_snooze_control(&control_rx_b, "broadcast b");
    let state_a = decode_snooze_state(&state_message_a);
    let state_b = decode_snooze_state(&state_message_b);
    assert_eq!(state_b, state_a);
    assert_eq!(state_a.records[0].workspace_id, workspace_b_id);
    apply_snooze_control(&mut shell_a, state_message_a);
    apply_snooze_control(&mut shell_b, state_message_b);
    let snooze_response = recv_server_message(&control_rx_a, "snooze response");
    let ServerMessage::ClientShellEndpointResponseChunk { data, .. } = snooze_response else {
        panic!("expected snooze response");
    };
    let response: crate::api::schema::SuccessResponse =
        serde_json::from_slice(&data).expect("decode snooze response");
    assert!(matches!(
        response.result,
        crate::api::schema::ResponseResult::WorkspaceSnooze { .. }
    ));
    assert!(!shell_a.is_workspace_id_visible(&workspace_b_id));
    assert!(!shell_b.is_workspace_id_visible(&workspace_b_id));
    assert!(shell_a
        .navigation_workspace_entries(&snapshot_a)
        .iter()
        .all(|entry| snapshot_a.workspaces[entry.index].workspace_id != workspace_b_id));
    assert!(shell_b.navigation_workspace_entries(&snapshot_b).is_empty());
    assert!(snapshot_a
        .agents
        .iter()
        .filter(|agent| agent.workspace_id == workspace_b_id)
        .all(|agent| !shell_a.is_agent_visible(agent) && !shell_b.is_agent_visible(agent)));
    assert!(shell_b.empty_presentation);
    assert_eq!(shell_a.focus_scope, Some(scope_a.clone()));
    assert_eq!(shell_b.focus_scope, Some(scope_b.clone()));
    assert_eq!(
        server
            .app
            .state
            .workspaces
            .iter()
            .map(|workspace| workspace.id.clone())
            .collect::<Vec<_>>(),
        inventory_before
    );

    let key = |kind| crate::protocol::ClientPaneInputEvent::Key {
        code: crate::protocol::ClientKeyCode::Char('x'),
        modifiers: 0,
        kind,
        repeat_count: 1,
        shifted_codepoint: None,
        generated_text: None,
        tracks_release: true,
        physical_key_id: Some(0x2d),
        windows_record: None,
    };
    let hidden_pane_id = server
        .app
        .public_pane_id(1, pane_b)
        .expect("hidden pane id");
    let content_seq_before = server
        .app
        .state
        .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 1, pane_b)
        .expect("hidden runtime")
        .content_seq();
    let _ = server.handle_server_event(ServerEvent::ClientShellPaneInput {
        client_id: 20,
        pane_id: hidden_pane_id.clone(),
        events: vec![key(crate::protocol::ClientKeyKind::Press)],
    });
    assert!(input_b.try_recv().is_err());
    assert!(!server.paste_client_clipboard_image_path(
        20,
        crate::protocol::ClientClipboardImageTarget::Pane(hidden_pane_id.clone()),
        "/tmp/hidden.png".into(),
    ));
    assert!(input_b.try_recv().is_err());
    let _ = server.handle_server_event(ServerEvent::ClientShellPaneInput {
        client_id: 20,
        pane_id: hidden_pane_id.clone(),
        events: vec![key(crate::protocol::ClientKeyKind::Release)],
    });
    let release = tokio::time::timeout(std::time::Duration::from_secs(1), input_b.recv())
        .await
        .expect("release bytes timeout")
        .expect("release channel closed");
    assert!(!release.is_empty());

    let stock_location = server.clients[&20]
        .shell_location
        .clone()
        .expect("stock client location");
    let mut stock = ClientConnection::new(
        (80, 24),
        crate::kitty_graphics::HostCellSize::default(),
        1,
        RenderEncoding::SemanticFrame,
        None,
    );
    stock.shell_location = Some(stock_location);
    server.clients.insert(30, stock);
    let _ = server.handle_server_event(ServerEvent::ClientShellPaneInput {
        client_id: 30,
        pane_id: hidden_pane_id,
        events: vec![key(crate::protocol::ClientKeyKind::Press)],
    });
    assert!(!input_b.try_recv().expect("stock client input").is_empty());

    let expected_revision = state_a.records[0].revision;
    let _ = server.handle_server_event(ServerEvent::ClientShellEndpointRequest {
        client_id: 10,
        boot_id: boot_id.clone(),
        request: Box::new(Request {
            id: "wake".into(),
            method: Method::WorkspaceWake(WorkspaceWakeParams {
                boot_id,
                expected_revision,
                workspace_id: Some(workspace_b_id.clone()),
                confirmed: false,
            }),
        }),
    });
    let wake_message_a = recv_snooze_control(&control_rx_a, "wake broadcast a");
    let wake_message_b = recv_snooze_control(&control_rx_b, "wake broadcast b");
    let wake_state_a = decode_snooze_state(&wake_message_a);
    let wake_state_b = decode_snooze_state(&wake_message_b);
    assert!(wake_state_a.records.is_empty());
    assert_eq!(wake_state_b, wake_state_a);
    apply_snooze_control(&mut shell_a, wake_message_a);
    apply_snooze_control(&mut shell_b, wake_message_b);
    let wake_response = recv_server_message(&control_rx_a, "wake response");
    let ServerMessage::ClientShellEndpointResponseChunk { data, .. } = wake_response else {
        panic!("expected wake response");
    };
    let response: crate::api::schema::SuccessResponse =
        serde_json::from_slice(&data).expect("decode wake response");
    assert!(matches!(
        response.result,
        crate::api::schema::ResponseResult::WorkspaceWake { .. }
    ));
    assert!(!shell_a.is_workspace_id_visible(&workspace_b_id));
    assert!(shell_b.is_workspace_id_visible(&workspace_b_id));
    assert!(shell_b.empty_presentation);
    assert_eq!(shell_a.focus_scope, Some(scope_a));
    assert_eq!(shell_b.focus_scope, Some(scope_b));
    assert_eq!(
        server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 1, pane_b)
            .expect("hidden runtime")
            .content_seq(),
        content_seq_before
    );

    let expiry_now = crate::server::workspace_snooze::now_unix_ms();
    server
        .workspace_snoozes
        .snooze(
            workspace_b_id.clone(),
            None,
            Some(expiry_now - 1),
            expiry_now - 2,
        )
        .expect("schedule expiry");
    assert!(server.handle_scheduled_tasks_headless(std::time::Instant::now(), false));
    let expiry_message_a = recv_snooze_control(&control_rx_a, "expiry broadcast a");
    let expiry_message_b = recv_snooze_control(&control_rx_b, "expiry broadcast b");
    let expiry_state_a = decode_snooze_state(&expiry_message_a);
    let expiry_state_b = decode_snooze_state(&expiry_message_b);
    assert!(expiry_state_a.records.is_empty());
    assert_eq!(expiry_state_b, expiry_state_a);
    apply_snooze_control(&mut shell_a, expiry_message_a);
    apply_snooze_control(&mut shell_b, expiry_message_b);
    assert!(!shell_a.is_workspace_id_visible(&workspace_b_id));
    assert!(shell_b.is_workspace_id_visible(&workspace_b_id));
    assert!(shell_b.empty_presentation);
    shutdown_test_runtimes(&mut server);
}

fn recv_server_message(rx: &std::sync::mpsc::Receiver<Vec<u8>>, label: &str) -> ServerMessage {
    read_server_message(
        rx.recv_timeout(std::time::Duration::from_secs(1))
            .unwrap_or_else(|_| panic!("timed out waiting for {label}")),
    )
}

fn recv_snooze_control(rx: &std::sync::mpsc::Receiver<Vec<u8>>, label: &str) -> ServerMessage {
    loop {
        match recv_server_message(rx, label) {
            ServerMessage::EndpointControl { kind, data } if kind == "workspace_snooze.state" => {
                return ServerMessage::EndpointControl { kind, data };
            }
            _ => continue,
        }
    }
}

fn decode_snooze_state(message: &ServerMessage) -> WorkspaceSnoozeState {
    let ServerMessage::EndpointControl { kind, data } = message else {
        panic!("expected endpoint control");
    };
    let control = crate::client::endpoint::decode_endpoint_control(kind, data)
        .expect("decode endpoint control");
    let crate::client::endpoint::EndpointControlMessage::WorkspaceSnoozeState(state) = control
    else {
        panic!("expected workspace snooze control");
    };
    *state
}

fn apply_snooze_control(shell: &mut crate::client::ClientShellState, message: ServerMessage) {
    let ServerMessage::EndpointControl { kind, data } = message else {
        panic!("expected endpoint control");
    };
    let control = crate::client::endpoint::decode_endpoint_control(&kind, &data)
        .expect("decode endpoint control");
    let crate::client::endpoint::EndpointControlMessage::WorkspaceSnoozeState(state) = control
    else {
        panic!("expected workspace snooze control");
    };
    shell.set_snooze_state(*state);
}

fn dispatch_focus_actions(
    server: &mut HeadlessServer,
    client_id: u64,
    actions: Vec<crate::client::ClientShellAction>,
) {
    let [crate::client::ClientShellAction::Endpoint {
        endpoint_id: crate::client::endpoint::ClientEndpointId::Local,
        boot_id,
        request,
    }] = actions.as_slice()
    else {
        panic!("expected one local focus action");
    };
    let (respond_to, response_rx) = std::sync::mpsc::channel();
    let _ = server.handle_client_shell_api_request(
        client_id,
        crate::api::ApiRequestMessage {
            request: (**request).clone(),
            respond_to,
            response_write_complete: None,
            stream_active: None,
        },
    );
    let response = response_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("focus response");
    let response: crate::api::schema::SuccessResponse =
        serde_json::from_str(&response).expect("decode focus response");
    assert_eq!(response.id, request.id);
    assert_eq!(boot_id, &server.client_shell_boot_id);
}

#[tokio::test]
async fn workspace_snooze_rejects_noncanonical_ids_without_record() {
    let mut server = test_headless_server();
    server.app.state.workspaces = vec![crate::workspace::Workspace::test_new("ws_1")];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(0);
    let (writer, control_rx, _render_rx) = test_client_writer();
    let client_id = 41;
    let _ = server.handle_server_event(ServerEvent::ClientShellConnected {
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
    });
    let _ = recv_server_message(&control_rx, "snapshot");
    let boot_id = server.client_shell_boot_id.clone();
    for (request_id, workspace_id) in [("numeric", "1"), ("missing", "")] {
        let _ = server.handle_server_event(ServerEvent::ClientShellEndpointRequest {
            client_id,
            boot_id: boot_id.clone(),
            request: Box::new(Request {
                id: request_id.into(),
                method: Method::WorkspaceSnooze(WorkspaceSnoozeParams {
                    workspace_id: workspace_id.into(),
                    boot_id: boot_id.clone(),
                    duration_seconds: Some(1_800),
                    deadline_unix_ms: None,
                }),
            }),
        });
        let ServerMessage::ClientShellEndpointResponseChunk { data, .. } =
            recv_server_message(&control_rx, "invalid workspace response")
        else {
            panic!("expected invalid workspace response");
        };
        let response: crate::api::schema::ErrorResponse =
            serde_json::from_slice(&data).expect("decode invalid workspace response");
        assert_eq!(response.error.code, "not_found");
        assert!(server.workspace_snoozes.state().records.is_empty());
    }
    shutdown_test_runtimes(&mut server);
}

#[tokio::test]
async fn stale_boot_and_revision_wake_are_rejected() {
    let mut server = test_headless_server();
    server.app.state.workspaces = vec![crate::workspace::Workspace::test_new("ws_1")];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(0);
    let (writer, control_rx, _render_rx) = test_client_writer();
    let client_id = 31;
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
    let _ = control_rx.recv().expect("snapshot");
    let boot_id = server.client_shell_boot_id.clone();
    let workspace_id = server.app.public_workspace_id(0);
    let request = |id: &str, method| Request {
        id: id.into(),
        method,
    };

    assert!(
        !server.handle_server_event(ServerEvent::ClientShellEndpointRequest {
            client_id,
            boot_id: boot_id.clone(),
            request: Box::new(request(
                "stale-boot",
                Method::WorkspaceSnooze(WorkspaceSnoozeParams {
                    workspace_id: workspace_id.clone(),
                    boot_id: "other-boot".into(),
                    duration_seconds: Some(1800),
                    deadline_unix_ms: None,
                }),
            )),
        })
    );
    let _: crate::api::schema::ErrorResponse = serde_json::from_slice(&match read_server_message(
        control_rx.recv().expect("stale boot response"),
    ) {
        ServerMessage::ClientShellEndpointResponseChunk { data, .. } => data,
        _ => panic!("expected response"),
    })
    .expect("decode stale boot");

    assert!(
        server.handle_server_event(ServerEvent::ClientShellEndpointRequest {
            client_id,
            boot_id: boot_id.clone(),
            request: Box::new(request(
                "snooze",
                Method::WorkspaceSnooze(WorkspaceSnoozeParams {
                    workspace_id: workspace_id.clone(),
                    boot_id: boot_id.clone(),
                    duration_seconds: Some(1800),
                    deadline_unix_ms: None,
                }),
            )),
        })
    );
    let _: SuccessResponse = serde_json::from_slice(&match read_server_message(
        control_rx.recv().expect("snooze response"),
    ) {
        ServerMessage::ClientShellEndpointResponseChunk { data, .. } => data,
        _ => panic!("expected response"),
    })
    .expect("decode snooze");

    assert!(
        !server.handle_server_event(ServerEvent::ClientShellEndpointRequest {
            client_id,
            boot_id: boot_id.clone(),
            request: Box::new(request(
                "stale-wake",
                Method::WorkspaceWake(WorkspaceWakeParams {
                    boot_id: boot_id.clone(),
                    expected_revision: 99,
                    workspace_id: Some(workspace_id.clone()),
                    confirmed: false,
                }),
            )),
        })
    );
    let _: crate::api::schema::ErrorResponse = serde_json::from_slice(&match read_server_message(
        control_rx.recv().expect("stale wake response"),
    ) {
        ServerMessage::ClientShellEndpointResponseChunk { data, .. } => data,
        _ => panic!("expected response"),
    })
    .expect("decode stale wake");
    shutdown_test_runtimes(&mut server);
}

#[test]
fn manager_expiry_and_wake_incarnations_are_safe() {
    let mut manager = crate::server::workspace_snooze::WorkspaceSnoozeManager::new("boot".into());
    let first = manager
        .snooze("ws".into(), None, Some(2_000), 1_000)
        .unwrap();
    assert!(!manager.expire_due(1_999));
    assert!(manager.is_snoozed("ws"));
    assert!(manager.expire_due(2_000));
    assert!(!manager.is_snoozed("ws"));
    let second = manager
        .snooze("ws".into(), None, Some(4_000), 2_000)
        .unwrap();
    assert!(second.revision > first.revision);
    assert_eq!(
        manager.wake(Some("ws"), first.revision, false),
        Err(
            crate::server::workspace_snooze::WorkspaceWakeError::StaleRevision {
                expected: first.revision,
                current: second.revision,
            }
        )
    );
    assert!(manager.wake(Some("ws"), second.revision, false).is_ok());
}

#[test]
fn scheduled_expiry_publishes_retired_state() {
    let mut manager = crate::server::workspace_snooze::WorkspaceSnoozeManager::new("boot".into());
    manager
        .snooze("ws".into(), None, Some(2_000), 1_000)
        .unwrap();
    assert!(manager.expire_due(2_000));
    assert!(manager.state().records.is_empty());
}
