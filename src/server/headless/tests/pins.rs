use super::*;

use crate::api::schema::{
    ErrorResponse, Method, PanePinParams, PaneTarget, PinState, Request, ResponseResult,
    SuccessResponse, WorkspaceCloseParams, WorkspacePinParams,
};
use crate::protocol::ServerMessage;

fn pin_server() -> HeadlessServer {
    let mut server = test_headless_server();
    server.app.state.workspaces = vec![
        crate::workspace::Workspace::test_new("alpha"),
        crate::workspace::Workspace::test_new("beta"),
    ];
    server.app.state.ensure_test_terminals();
    server.app.state.active = Some(0);
    server.app.state.selected = 0;
    server
}

fn api_pin(
    server: &mut HeadlessServer,
    id: &str,
    method: Method,
) -> Result<PinState, ErrorResponse> {
    let response = api_request_raw(server, id, method);
    match serde_json::from_str::<SuccessResponse>(&response) {
        Ok(SuccessResponse {
            result: ResponseResult::Pin { state },
            ..
        }) => Ok(state),
        Ok(other) => panic!("expected pin state response, got {other:?}"),
        Err(_) => Err(serde_json::from_str(&response).expect("error response")),
    }
}

fn api_request_raw(server: &mut HeadlessServer, id: &str, method: Method) -> String {
    let (respond_to, response_rx) = std::sync::mpsc::channel();
    server.handle_api_request_with_shutdown_check(crate::api::ApiRequestMessage {
        request: Request {
            id: id.into(),
            method,
        },
        respond_to,
        response_write_complete: None,
    });
    response_rx
        .recv_timeout(std::time::Duration::from_secs(1))
        .expect("api response")
}

fn workspace_pin(workspace_id: &str, pinned: bool) -> Method {
    Method::WorkspacePin(WorkspacePinParams {
        workspace_id: workspace_id.into(),
        pinned,
    })
}

fn pane_pin(pane_id: &str, pinned: bool) -> Method {
    Method::PanePin(PanePinParams {
        pane_id: pane_id.into(),
        pinned,
    })
}

fn public_pane(server: &HeadlessServer, ws_idx: usize, pane_id: crate::layout::PaneId) -> String {
    server
        .app
        .public_pane_id(ws_idx, pane_id)
        .expect("public pane id")
}

/// Drains queued messages until a `pin.state` control message arrives.
fn recv_pin_state(rx: &std::sync::mpsc::Receiver<Vec<u8>>, label: &str) -> PinState {
    loop {
        let message = read_server_message(
            rx.recv_timeout(std::time::Duration::from_secs(1))
                .unwrap_or_else(|_| panic!("timed out waiting for {label}")),
        );
        let ServerMessage::EndpointControl { kind, data } = message else {
            continue;
        };
        if kind != "pin.state" {
            continue;
        }
        return serde_json::from_str(&data).expect("decode pin state");
    }
}

fn endpoint_pin(server: &mut HeadlessServer, client_id: u64, request_id: &str, method: Method) {
    server.handle_server_event(ServerEvent::ClientShellEndpointRequest {
        client_id,
        boot_id: server.client_shell_boot_id.clone(),
        request: Box::new(Request {
            id: request_id.into(),
            method,
        }),
    });
}

fn endpoint_pin_response(
    rx: &std::sync::mpsc::Receiver<Vec<u8>>,
    label: &str,
) -> Result<PinState, ErrorResponse> {
    loop {
        let message = read_server_message(
            rx.recv_timeout(std::time::Duration::from_secs(1))
                .unwrap_or_else(|_| panic!("timed out waiting for {label}")),
        );
        let ServerMessage::ClientShellEndpointResponseChunk { data, .. } = message else {
            continue;
        };
        if let Ok(success) = serde_json::from_slice::<SuccessResponse>(&data) {
            if let ResponseResult::Pin { state } = success.result {
                return Ok(state);
            }
            panic!("expected pin state response, got {success:?}");
        }
        return Err(serde_json::from_slice(&data).expect("decode pin error response"));
    }
}

#[test]
fn pin_unpin_idempotent_pin_and_not_found_on_the_endpoint_lane() {
    let mut server = pin_server();
    let (writer_a, control_rx_a, _render_a) = test_client_writer();
    let (writer_b, control_rx_b, _render_b) = test_client_writer();
    for (client_id, writer) in [(10, writer_a), (20, writer_b)] {
        assert!(
            server.handle_server_event(ServerEvent::ClientShellConnected {
                surface_reuse: false,
                surface_delta: false,
                surface_scroll: false,
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
    let _ = client_shell_snapshot(&control_rx_a);
    let _ = client_shell_snapshot(&control_rx_b);

    // Subscribe both clients; each receives the empty initial state.
    assert!(!server.handle_server_event(ServerEvent::ClientShellPinSubscribe { client_id: 10 }));
    assert!(!server.handle_server_event(ServerEvent::ClientShellPinSubscribe { client_id: 20 }));
    let initial_a = recv_pin_state(&control_rx_a, "initial state a");
    let initial_b = recv_pin_state(&control_rx_b, "initial state b");
    assert_eq!(initial_a.boot_id, server.client_shell_boot_id);
    assert_eq!(initial_a.revision, 0);
    assert!(initial_a.workspace_ids.is_empty() && initial_a.pane_ids.is_empty());
    assert_eq!(initial_b, initial_a);

    let workspace_b = server.app.public_workspace_id(1);
    let pane_b = server.app.state.workspaces[1]
        .focused_pane_id()
        .expect("beta pane");
    let pane_b_id = public_pane(&server, 1, pane_b);

    endpoint_pin(&mut server, 10, "pin-ws", workspace_pin(&workspace_b, true));
    // The broadcast precedes the response on the requester's queue.
    let broadcast_a = recv_pin_state(&control_rx_a, "workspace broadcast a");
    let broadcast_b = recv_pin_state(&control_rx_b, "workspace broadcast b");
    let state = endpoint_pin_response(&control_rx_a, "pin response").expect("workspace pin");
    assert_eq!(state.revision, 1);
    assert_eq!(state.workspace_ids, vec![workspace_b.clone()]);
    assert_eq!(broadcast_a, state);
    assert_eq!(broadcast_b, state);

    // Pinning the same workspace again succeeds without a new revision or a
    // broadcast.
    endpoint_pin(
        &mut server,
        10,
        "pin-ws-again",
        workspace_pin(&workspace_b, true),
    );
    let state_again =
        endpoint_pin_response(&control_rx_a, "idempotent pin response").expect("idempotent pin");
    assert_eq!(state_again.revision, 1);
    assert!(
        control_rx_b
            .recv_timeout(std::time::Duration::from_millis(250))
            .is_err(),
        "idempotent pin does not broadcast"
    );

    endpoint_pin(&mut server, 20, "pin-pane", pane_pin(&pane_b_id, true));
    let broadcast_a = recv_pin_state(&control_rx_a, "pane broadcast a");
    let broadcast_b = recv_pin_state(&control_rx_b, "pane broadcast b");
    let state = endpoint_pin_response(&control_rx_b, "pane pin response").expect("pane pin");
    assert_eq!(state.revision, 2);
    assert_eq!(state.pane_ids, vec![pane_b_id.clone()]);
    assert_eq!(broadcast_a, state);
    assert_eq!(broadcast_b, state);

    // Unknown ids fail with not_found and change nothing.
    endpoint_pin(
        &mut server,
        10,
        "pin-missing-ws",
        workspace_pin("w_999", true),
    );
    let error = endpoint_pin_response(&control_rx_a, "missing workspace response")
        .expect_err("missing workspace must fail");
    assert_eq!(error.error.code, "not_found");
    endpoint_pin(
        &mut server,
        10,
        "pin-missing-pane",
        pane_pin("w_999:p9", true),
    );
    let error = endpoint_pin_response(&control_rx_a, "missing pane response")
        .expect_err("missing pane must fail");
    assert_eq!(error.error.code, "not_found");
    assert_eq!(server.pin_state().revision, 2);

    // Unpinning bumps the revision and empties the lists.
    endpoint_pin(&mut server, 20, "unpin-pane", pane_pin(&pane_b_id, false));
    let broadcast_a = recv_pin_state(&control_rx_a, "unpin pane broadcast a");
    let broadcast_b = recv_pin_state(&control_rx_b, "unpin pane broadcast b");
    let state = endpoint_pin_response(&control_rx_b, "unpin pane response").expect("unpin pane");
    assert_eq!(state.revision, 3);
    assert!(state.pane_ids.is_empty());
    assert_eq!(broadcast_a, state);
    assert_eq!(broadcast_b, state);
    endpoint_pin(
        &mut server,
        10,
        "unpin-ws",
        workspace_pin(&workspace_b, false),
    );
    let broadcast_a = recv_pin_state(&control_rx_a, "unpin workspace broadcast a");
    let broadcast_b = recv_pin_state(&control_rx_b, "unpin workspace broadcast b");
    let state =
        endpoint_pin_response(&control_rx_a, "unpin workspace response").expect("unpin workspace");
    assert_eq!(state.revision, 4);
    assert!(state.workspace_ids.is_empty());
    assert_eq!(broadcast_a, state);
    assert_eq!(broadcast_b, state);
    shutdown_test_runtimes(&mut server);
}

#[test]
fn closing_a_workspace_or_pane_prunes_its_pins() {
    let mut server = pin_server();
    // A second pane in alpha so its workspace survives the pane close below.
    let extra_pane =
        server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
    server.app.state.ensure_test_terminals();
    let workspace_a = server.app.public_workspace_id(0);
    let state =
        api_pin(&mut server, "pin-ws", workspace_pin(&workspace_a, true)).expect("workspace pin");
    assert_eq!(state.revision, 1);
    let extra_pane_id = public_pane(&server, 0, extra_pane);
    let state = api_pin(&mut server, "pin-pane", pane_pin(&extra_pane_id, true)).expect("pane pin");
    assert_eq!(state.revision, 2);
    let workspace_b = server.app.public_workspace_id(1);
    let state =
        api_pin(&mut server, "pin-beta", workspace_pin(&workspace_b, true)).expect("beta pin");
    assert_eq!(state.revision, 3);

    // Closing the pinned pane removes its pin but keeps the workspace pins.
    api_request_raw(
        &mut server,
        "close-pane",
        Method::PaneClose(PaneTarget {
            pane_id: extra_pane_id,
        }),
    );
    let state = server.pin_state();
    assert_eq!(state.revision, 4);
    assert!(state.pane_ids.is_empty());
    assert_eq!(state.workspace_ids.len(), 2);

    // Closing a pinned workspace removes its pin and leaves nothing behind.
    api_request_raw(
        &mut server,
        "close-workspace",
        Method::WorkspaceClose(WorkspaceCloseParams {
            workspace_id: workspace_a,
            close_group: false,
        }),
    );
    let state = server.pin_state();
    assert_eq!(state.revision, 5);
    assert_eq!(state.workspace_ids.len(), 1);
    assert!(state.workspace_ids[0] == server.app.public_workspace_id(0));
    shutdown_test_runtimes(&mut server);
}

/// Keeps the session checkpoint of these tests inside a throwaway directory.
/// Persist tests share a lock because the config-home env var is process-wide
/// and the session checkpoint resolves it at save time.
static PERSIST_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

struct TempConfigHome {
    previous: Option<std::ffi::OsString>,
    previous_session: Option<std::ffi::OsString>,
    path: std::path::PathBuf,
}

impl TempConfigHome {
    fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "herdr-pin-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&path).expect("create temp config home");
        let previous = std::env::var_os("XDG_CONFIG_HOME");
        std::env::set_var("XDG_CONFIG_HOME", &path);
        let previous_session = std::env::var_os(crate::session::SESSION_ENV_VAR);
        std::env::remove_var(crate::session::SESSION_ENV_VAR);
        Self {
            previous,
            previous_session,
            path,
        }
    }

    fn data_dir(&self) -> std::path::PathBuf {
        crate::session::data_dir()
    }
}

impl Drop for TempConfigHome {
    fn drop(&mut self) {
        match &self.previous {
            Some(previous) => std::env::set_var("XDG_CONFIG_HOME", previous),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match &self.previous_session {
            Some(previous) => std::env::set_var(crate::session::SESSION_ENV_VAR, previous),
            None => std::env::remove_var(crate::session::SESSION_ENV_VAR),
        }
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn enable_pin_persistence(server: &mut HeadlessServer, pin_path: std::path::PathBuf) {
    server.app.policy.persist_session = true;
    let mut pins = crate::server::pins::PinManager::new();
    let (store, notice) =
        crate::server::pin_store::PinStore::load(pin_path, &server.app.state.workspaces, &mut pins);
    assert!(notice.is_none());
    server.pins = pins;
    server.pin_store = Some(store);
    server.remember_published_pins();
}

fn read_pin_document(pin_path: &std::path::Path) -> serde_json::Value {
    let content = std::fs::read(pin_path).expect("pins.json must already be saved");
    serde_json::from_slice(&content).expect("decode pins.json")
}

#[test]
fn pinned_pane_moves_update_persistence_and_survive_restore() {
    let _persist_lock = PERSIST_TEST_LOCK.lock().expect("persist test lock");
    let temp_home = TempConfigHome::new("move");
    let pin_path = temp_home.data_dir().join("pins.json");

    for source_removed in [false, true] {
        let mut server = pin_server();
        let split_pane = (!source_removed).then(|| {
            let pane =
                server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
            server.app.state.ensure_test_terminals();
            pane
        });
        enable_pin_persistence(&mut server, pin_path.clone());

        let source_ws_id = server.app.public_workspace_id(0);
        let destination_ws_id = server.app.public_workspace_id(1);
        let pane_id = split_pane.unwrap_or_else(|| {
            server.app.state.workspaces[0]
                .focused_pane_id()
                .expect("pane")
        });
        let pane_public_id = public_pane(&server, 0, pane_id);
        let source_lifetime = server.app.state.workspaces[0].lifetime_id.clone();
        let destination_lifetime = server.app.state.workspaces[1].lifetime_id.clone();

        api_pin(&mut server, "pin-ws", workspace_pin(&source_ws_id, true)).expect("workspace pin");
        api_pin(&mut server, "pin-pane", pane_pin(&pane_public_id, true)).expect("pane pin");

        // Move the pinned pane into the destination workspace as a new tab.
        let moved = api_request_raw(
            &mut server,
            "move-pane",
            Method::PaneMove(crate::api::schema::PaneMoveParams {
                pane_id: pane_public_id,
                destination: crate::api::schema::PaneMoveDestination::NewTab {
                    workspace_id: Some(destination_ws_id.clone()),
                    label: None,
                },
                focus: false,
            }),
        );
        assert!(!moved.contains("\"error\""), "{moved}");

        // The pin follows the pane under its new public id; a removed source
        // workspace loses its own pin in the same change.
        let destination_ws_idx = server
            .app
            .state
            .workspaces
            .iter()
            .position(|workspace| workspace.id == destination_ws_id)
            .expect("destination workspace");
        let state = server.pin_state();
        let moved_pane_id = public_pane(&server, destination_ws_idx, pane_id);
        assert_eq!(state.pane_ids, vec![moved_pane_id]);
        // Two pins then one reconcile: membership loss must not bump again
        // when the public id also changed.
        assert_eq!(state.revision, 3);
        if source_removed {
            assert!(state.workspace_ids.is_empty());
        } else {
            assert_eq!(state.workspace_ids, vec![source_ws_id.clone()]);
        }

        // The store was re-persisted synchronously: pins.json now points at
        // the destination workspace identity without any debounce wait.
        let document = read_pin_document(&pin_path);
        let stored_workspace_ids = document["workspaces"]
            .as_array()
            .expect("workspaces array")
            .iter()
            .map(|value| value.as_str().expect("lifetime id").to_owned())
            .collect::<Vec<_>>();
        let stored_agents = document["agents"].as_array().expect("agents array");
        assert_eq!(stored_agents.len(), 1);
        assert_eq!(
            stored_agents[0]["workspace_lifetime_id"].as_str(),
            Some(destination_lifetime.as_str())
        );
        let stored_pane_number = stored_agents[0]["pane_number"]
            .as_u64()
            .expect("pane number");
        if source_removed {
            assert!(stored_workspace_ids.is_empty());
        } else {
            assert_eq!(stored_workspace_ids, vec![source_lifetime.clone()]);
        }

        // Simulated restart: fresh internal ids, same persisted identities.
        let lifetime_ids: Vec<String> = server
            .app
            .state
            .workspaces
            .iter()
            .map(|workspace| workspace.lifetime_id.clone())
            .collect();
        let pane_counts: Vec<usize> = server
            .app
            .state
            .workspaces
            .iter()
            .map(|workspace| workspace.public_pane_numbers.len())
            .collect();
        let restored: Vec<crate::workspace::Workspace> = lifetime_ids
            .iter()
            .zip(pane_counts)
            .map(|(lifetime_id, count)| {
                let mut workspace = crate::workspace::Workspace::test_new("restored");
                workspace.lifetime_id = lifetime_id.clone();
                for _ in 1..count {
                    workspace.test_split(ratatui::layout::Direction::Horizontal);
                }
                workspace
            })
            .collect();
        let mut restored_manager = crate::server::pins::PinManager::new();
        let (restored_store, notice) = crate::server::pin_store::PinStore::load(
            pin_path.clone(),
            &restored,
            &mut restored_manager,
        );
        assert!(notice.is_none());
        if source_removed {
            assert!(restored_manager.pinned_workspace_ids().next().is_none());
        } else {
            let restored_source = restored
                .iter()
                .find(|workspace| workspace.lifetime_id == source_lifetime)
                .expect("restored source workspace");
            assert!(restored_manager.is_workspace_pinned(&restored_source.id));
        }
        let restored_destination = restored
            .iter()
            .find(|workspace| workspace.lifetime_id == destination_lifetime)
            .expect("restored destination workspace");
        let restored_pane = restored_destination
            .public_pane_numbers
            .iter()
            .find_map(|(pane_id, number)| {
                (*number as u64 == stored_pane_number).then_some(*pane_id)
            })
            .expect("restored pane");
        assert!(restored_manager.is_pane_pinned(restored_pane));
        assert_eq!(restored_store.agent_pin_count(), 1);
        shutdown_test_runtimes(&mut server);
    }
}

#[test]
fn split_pin_restores_from_the_checkpointed_session() {
    let _persist_lock = PERSIST_TEST_LOCK.lock().expect("persist test lock");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("restore runtime");
    let _runtime_guard = runtime.enter();
    let temp_home = TempConfigHome::new("split");
    let pin_path = temp_home.data_dir().join("pins.json");
    let mut server = pin_server();
    // Split without the API so the server under test does not spawn a PTY.
    // The checkpoint inside the pin response is what makes the split durable.
    let split_pane =
        server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
    server.app.state.ensure_test_terminals();
    enable_pin_persistence(&mut server, pin_path.clone());

    let lifetime_id = server.app.state.workspaces[0].lifetime_id.clone();
    let split_pane_id = public_pane(&server, 0, split_pane);
    let state = api_pin(&mut server, "pin-pane", pane_pin(&split_pane_id, true)).expect("pane pin");
    assert_eq!(state.revision, 1);

    let document = read_pin_document(&pin_path);
    let agents = document["agents"].as_array().expect("agents array");
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["pane_number"], 2);
    assert_eq!(
        agents[0]["workspace_lifetime_id"].as_str(),
        Some(lifetime_id.as_str())
    );

    let snapshot = crate::persist::load().expect("checkpointed session.json");
    let (events_tx, _events_rx) = tokio::sync::mpsc::channel(8);
    #[cfg(windows)]
    let shell = r"C:\Windows\System32\whoami.exe";
    #[cfg(not(windows))]
    let shell = "/bin/sh";
    let (workspaces, _terminals, runtimes) = crate::persist::restore(
        &snapshot,
        None,
        24,
        80,
        1_048_576,
        shell,
        crate::config::ShellModeConfig::NonLogin,
        false,
        events_tx,
        std::sync::Arc::new(tokio::sync::Notify::new()),
        std::sync::Arc::new(crate::render_signal::RenderSignal::new()),
    );
    let restored_workspace = workspaces
        .iter()
        .find(|workspace| workspace.lifetime_id == lifetime_id)
        .expect("restored workspace");
    let restored_pane = restored_workspace
        .public_pane_numbers
        .iter()
        .find_map(|(pane_id, number)| (*number == 2).then_some(*pane_id))
        .expect("restored pane number 2");
    let mut restored_manager = crate::server::pins::PinManager::new();
    let (_, notice) =
        crate::server::pin_store::PinStore::load(pin_path, &workspaces, &mut restored_manager);
    assert!(notice.is_none());
    assert!(restored_manager.is_pane_pinned(restored_pane));
    for (_, runtime) in runtimes {
        runtime.shutdown();
    }
    shutdown_test_runtimes(&mut server);
}

fn subscribe_pin_client(
    server: &mut HeadlessServer,
    client_id: u64,
) -> std::sync::mpsc::Receiver<Vec<u8>> {
    let (writer, control_rx, _render_rx) = test_client_writer();
    assert!(
        server.handle_server_event(ServerEvent::ClientShellConnected {
            surface_reuse: false,
            surface_delta: false,
            surface_scroll: false,
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
    let _ = client_shell_snapshot(&control_rx);
    assert!(!server.handle_server_event(ServerEvent::ClientShellPinSubscribe { client_id }));
    let _ = recv_pin_state(&control_rx, "subscribe");
    control_rx
}

#[test]
fn failed_pin_save_keeps_the_change_and_still_broadcasts() {
    let _persist_lock = PERSIST_TEST_LOCK.lock().expect("persist test lock");
    let temp_home = TempConfigHome::new("save-fail");
    let pin_path = temp_home.data_dir().join("pins.json");
    let mut server = pin_server();
    enable_pin_persistence(&mut server, pin_path.clone());
    let control_rx = subscribe_pin_client(&mut server, 10);
    let workspace_id = server.app.public_workspace_id(0);

    endpoint_pin(
        &mut server,
        10,
        "missing",
        workspace_pin("missing-workspace", true),
    );
    let error = endpoint_pin_response(&control_rx, "missing workspace").expect_err("not found");
    assert_eq!(error.error.code, "not_found");
    assert_eq!(server.pin_state().revision, 0);
    assert!(
        control_rx
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err(),
        "not found does not broadcast"
    );

    // A directory at the pin path makes the write fail after the change is kept.
    std::fs::create_dir_all(&pin_path).expect("pins.json directory");
    endpoint_pin(
        &mut server,
        10,
        "pin-ws",
        workspace_pin(&workspace_id, true),
    );
    let broadcast = recv_pin_state(&control_rx, "failed-save broadcast");
    let state = endpoint_pin_response(&control_rx, "failed-save response").expect("kept pin");
    assert_eq!(state.revision, 1);
    assert_eq!(state.workspace_ids, vec![workspace_id.clone()]);
    assert_eq!(broadcast, state);
    assert_eq!(server.pin_state(), state);
    assert!(
        std::fs::metadata(&pin_path).expect("pin path").is_dir(),
        "failed save leaves the previous path untouched"
    );

    endpoint_pin(
        &mut server,
        10,
        "pin-ws-again",
        workspace_pin(&workspace_id, true),
    );
    let again = endpoint_pin_response(&control_rx, "idempotent").expect("idempotent");
    assert_eq!(again.revision, 1);
    assert!(
        control_rx
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err(),
        "idempotent pin does not broadcast"
    );
    shutdown_test_runtimes(&mut server);
}

#[test]
fn failed_pin_checkpoint_keeps_the_change() {
    let _persist_lock = PERSIST_TEST_LOCK.lock().expect("persist test lock");
    let temp_home = TempConfigHome::new("checkpoint-fail");
    let pin_path = temp_home.data_dir().join("pins.json");
    let session_path = temp_home.data_dir().join("session.json");
    std::fs::create_dir_all(&session_path).expect("session.json directory");
    let mut server = pin_server();
    enable_pin_persistence(&mut server, pin_path.clone());
    let control_rx = subscribe_pin_client(&mut server, 11);
    let workspace_id = server.app.public_workspace_id(0);

    endpoint_pin(
        &mut server,
        11,
        "pin-ws",
        workspace_pin(&workspace_id, true),
    );
    let broadcast = recv_pin_state(&control_rx, "checkpoint-fail broadcast");
    let state = endpoint_pin_response(&control_rx, "checkpoint-fail response").expect("kept pin");
    assert_eq!(state.revision, 1);
    assert_eq!(state.workspace_ids, vec![workspace_id]);
    assert_eq!(broadcast, state);
    assert!(
        session_path.is_dir(),
        "checkpoint did not replace the directory"
    );
    assert!(
        std::fs::symlink_metadata(&pin_path).is_err(),
        "pin file is not written when the checkpoint fails"
    );
    shutdown_test_runtimes(&mut server);
}

#[test]
fn unpinned_pane_move_does_not_bump_or_broadcast() {
    let mut server = pin_server();
    let extra = server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
    server.app.state.ensure_test_terminals();
    let control_rx = subscribe_pin_client(&mut server, 12);
    let workspace_id = server.app.public_workspace_id(0);
    api_pin(&mut server, "pin-ws", workspace_pin(&workspace_id, true)).expect("workspace pin");
    let broadcast = recv_pin_state(&control_rx, "workspace pin broadcast");
    assert_eq!(broadcast.revision, 1);

    let pane_public_id = public_pane(&server, 0, extra);
    let destination = server.app.public_workspace_id(1);
    let moved = api_request_raw(
        &mut server,
        "move-unpinned",
        Method::PaneMove(crate::api::schema::PaneMoveParams {
            pane_id: pane_public_id,
            destination: crate::api::schema::PaneMoveDestination::NewTab {
                workspace_id: Some(destination),
                label: None,
            },
            focus: false,
        }),
    );
    assert!(!moved.contains("\"error\""), "{moved}");
    let state = server.pin_state();
    assert_eq!(state.revision, 1);
    assert_eq!(state.workspace_ids, vec![workspace_id]);
    assert!(state.pane_ids.is_empty());
    assert!(
        control_rx
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err(),
        "unpinned pane move does not broadcast"
    );
    shutdown_test_runtimes(&mut server);
}

#[test]
fn pinned_pane_move_bumps_revision_without_persistence() {
    let mut server = pin_server();
    let extra = server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
    server.app.state.ensure_test_terminals();
    let control_rx = subscribe_pin_client(&mut server, 13);
    let pane_public_id = public_pane(&server, 0, extra);
    let destination = server.app.public_workspace_id(1);
    api_pin(&mut server, "pin-pane", pane_pin(&pane_public_id, true)).expect("pane pin");
    let _ = recv_pin_state(&control_rx, "pin broadcast");
    assert_eq!(server.pin_state().revision, 1);

    let moved = api_request_raw(
        &mut server,
        "move-pinned",
        Method::PaneMove(crate::api::schema::PaneMoveParams {
            pane_id: pane_public_id,
            destination: crate::api::schema::PaneMoveDestination::NewTab {
                workspace_id: Some(destination.clone()),
                label: None,
            },
            focus: false,
        }),
    );
    assert!(!moved.contains("\"error\""), "{moved}");
    let destination_idx = server
        .app
        .state
        .workspaces
        .iter()
        .position(|workspace| workspace.id == destination)
        .expect("destination");
    let state = server.pin_state();
    assert_eq!(state.revision, 2);
    assert_eq!(
        state.pane_ids,
        vec![public_pane(&server, destination_idx, extra)]
    );
    let broadcast = recv_pin_state(&control_rx, "move broadcast");
    assert_eq!(broadcast, state);
    shutdown_test_runtimes(&mut server);
}

#[test]
fn pane_death_prunes_pins_but_agent_release_keeps_them() {
    let mut server = pin_server();
    let extra = server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
    server.app.state.ensure_test_terminals();
    let control_rx = subscribe_pin_client(&mut server, 14);
    let pane_public_id = public_pane(&server, 0, extra);
    api_pin(&mut server, "pin-pane", pane_pin(&pane_public_id, true)).expect("pane pin");
    let _ = recv_pin_state(&control_rx, "pin broadcast");
    assert_eq!(server.pin_state().revision, 1);

    assert!(
        server.handle_internal_event_with_forwarding(AppEvent::HookAgentReleased {
            pane_id: extra,
            source: "test".into(),
            agent_label: "claude".into(),
            known_agent: None,
            seq: None,
        })
    );
    assert!(server.app.find_pane(extra).is_some());
    assert_eq!(server.pin_state().revision, 1);
    assert_eq!(server.pin_state().pane_ids, vec![pane_public_id]);
    assert!(
        control_rx
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err(),
        "agent release keeps the pin and does not broadcast"
    );

    assert!(
        server.handle_internal_event_with_forwarding(AppEvent::PaneDied {
            pane_id: extra,
            exit_reason: crate::platform::ChildExitReason::Exited,
        })
    );
    assert!(server.app.find_pane(extra).is_none());
    let state = server.pin_state();
    assert_eq!(state.revision, 2);
    assert!(state.pane_ids.is_empty());
    let broadcast = recv_pin_state(&control_rx, "pane died broadcast");
    assert_eq!(broadcast, state);
    shutdown_test_runtimes(&mut server);
}

#[test]
fn worktree_remove_and_plugin_pane_close_prune_pins() {
    let mut server = pin_server();
    let checkout = std::path::PathBuf::from("/repo/herdr-issue");
    server.app.state.workspaces[1].worktree_space =
        Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo-key".into(),
            label: "herdr".into(),
            repo_root: "/repo/herdr".into(),
            checkout_path: checkout.clone(),
            is_linked_worktree: true,
        });
    let extra = server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
    server.app.state.ensure_test_terminals();
    let linked_id = server.app.public_workspace_id(1);
    let linked_pane = server.app.state.workspaces[1]
        .focused_pane_id()
        .expect("linked pane");
    api_pin(&mut server, "pin-linked", workspace_pin(&linked_id, true)).expect("linked pin");
    let extra_public = public_pane(&server, 0, extra);
    api_pin(&mut server, "pin-plugin", pane_pin(&extra_public, true)).expect("plugin pin");
    assert_eq!(server.pin_state().revision, 2);

    let checkout_key = crate::worktree::canonical_or_original(&checkout);
    server
        .app
        .pending_api_worktree_removes
        .insert(linked_id.clone(), 7);
    server
        .app
        .pending_api_worktree_remove_paths
        .insert(checkout_key.clone(), 7);
    let (respond_to, _response_rx) = std::sync::mpsc::channel();
    assert!(
        server.handle_internal_event_with_forwarding(AppEvent::WorktreeRemoveFinished(Box::new(
            crate::events::WorktreeRemoveResult {
                workspace_id: linked_id.clone(),
                path: checkout,
                workspace: None,
                worktree: None,
                forced: true,
                api_request: Some(crate::events::ApiWorktreeRemoveRequest {
                    id: "req".into(),
                    operation_id: 7,
                    checkout_key,
                    shutdown_panes: vec![linked_pane],
                    respond_to,
                }),
                result: Ok(()),
            }
        )))
    );
    assert!(server
        .app
        .state
        .workspaces
        .iter()
        .all(|workspace| workspace.id != linked_id));
    let state = server.pin_state();
    assert_eq!(state.revision, 3);
    assert!(state.workspace_ids.is_empty());
    assert_eq!(state.pane_ids, vec![extra_public.clone()]);

    server.app.state.plugin_panes.insert(
        extra,
        crate::app::state::PluginPaneRecord {
            plugin_id: "pins".into(),
            entrypoint: "main".into(),
        },
    );
    let closed = api_request_raw(
        &mut server,
        "close-plugin",
        Method::PluginPaneClose(crate::api::schema::PluginPaneCloseParams {
            pane_id: extra_public,
        }),
    );
    assert!(!closed.contains("\"error\""), "{closed}");
    let state = server.pin_state();
    assert_eq!(state.revision, 4);
    assert!(state.pane_ids.is_empty());
    assert!(server.app.find_pane(extra).is_none());
    shutdown_test_runtimes(&mut server);
}
