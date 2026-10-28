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
fn split_pin_and_restore_do_not_wait_for_the_session_save_debounce() {
    let _persist_lock = PERSIST_TEST_LOCK.lock().expect("persist test lock");
    let temp_home = TempConfigHome::new("split");
    let pin_path = temp_home.data_dir().join("pins.json");
    let mut server = pin_server();
    // Split without the API so no PTY is spawned; the pin path under test is
    // durability timing, not splitting.
    let split_pane =
        server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
    server.app.state.ensure_test_terminals();
    enable_pin_persistence(&mut server, pin_path.clone());

    let split_pane_id = public_pane(&server, 0, split_pane);
    let state = api_pin(&mut server, "pin-pane", pane_pin(&split_pane_id, true)).expect("pane pin");
    assert_eq!(state.revision, 1);

    // The pin save is synchronous: pins.json already holds the split pane's
    // public number, even though the session save stays debounced.
    let document = read_pin_document(&pin_path);
    let agents = document["agents"].as_array().expect("agents array");
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0]["pane_number"], 2);
    assert_eq!(
        agents[0]["workspace_lifetime_id"].as_str(),
        Some(server.app.state.workspaces[0].lifetime_id.as_str())
    );

    // Restore into a fresh manager with remapped pane ids, without waiting
    // for the debounced session save.
    let lifetime_id = server.app.state.workspaces[0].lifetime_id.clone();
    let mut restored = crate::workspace::Workspace::test_new("restored");
    restored.lifetime_id = lifetime_id;
    restored.test_split(ratatui::layout::Direction::Horizontal);
    let mut restored_manager = crate::server::pins::PinManager::new();
    let (_, notice) = crate::server::pin_store::PinStore::load(
        pin_path,
        std::slice::from_ref(&restored),
        &mut restored_manager,
    );
    assert!(notice.is_none());
    let restored_pane = restored
        .public_pane_numbers
        .iter()
        .find_map(|(pane_id, number)| (*number == 2).then_some(*pane_id))
        .expect("restored split pane");
    assert!(restored_manager.is_pane_pinned(restored_pane));
    shutdown_test_runtimes(&mut server);
}
