use super::*;

const WORKSPACE_RECORD: &str = "00000000-0000-4000-8000-000000000001";
const PROJECT_RECORD: &str = "00000000-0000-4000-8000-000000000002";
const UNAVAILABLE_RECORD: &str = "00000000-0000-4000-8000-000000000003";

fn management_state() -> ClientShellState {
    use crate::api::schema::{SnoozePersistenceInfo, SnoozeStoredRecordInfo};
    let mut state = reset_ready_state();
    let mut shared = snooze_state(&["ws_2"], 5);
    shared.project_records = vec![ProjectSnoozeRecord {
        project_key: "repo-b".into(),
        boot_id: "boot-1".into(),
        deadline_unix_ms: 5_000_000,
        revision: 4,
    }];
    shared.persistence = Some(SnoozePersistenceInfo {
        records: vec![
            SnoozeStoredRecordInfo {
                record_id: WORKSPACE_RECORD.into(),
                scope: "workspace".into(),
                label: "Workspace Beta".into(),
                project_label: Some("Project Beta".into()),
                created_unix_ms: 0,
                deadline_unix_ms: 4_000_000,
                available: true,
                workspace_id: Some("ws_2".into()),
                project_key: None,
            },
            SnoozeStoredRecordInfo {
                record_id: PROJECT_RECORD.into(),
                scope: "project".into(),
                label: "/raw/project/key/.git".into(),
                project_label: Some("Project Beta".into()),
                created_unix_ms: 0,
                deadline_unix_ms: 5_000_000,
                available: true,
                workspace_id: None,
                project_key: Some("repo-b".into()),
            },
            SnoozeStoredRecordInfo {
                record_id: UNAVAILABLE_RECORD.into(),
                scope: "workspace".into(),
                label: "Old Space".into(),
                project_label: Some("Old Project".into()),
                created_unix_ms: 0,
                deadline_unix_ms: 6_000_000,
                available: false,
                workspace_id: None,
                project_key: None,
            },
        ],
        notice: None,
    });
    state.set_snooze_state(shared);
    state.set_endpoint_methods(Some(vec![
        "snooze.record.wake".into(),
        "workspace.wake".into(),
        "project.wake".into(),
        "snooze.reset".into(),
    ]));
    state
}

fn key(state: &mut ClientShellState, code: KeyCode) -> ClientShellInput {
    state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Key(
        crate::input::TerminalKey::new(code, KeyModifiers::empty()),
    )])
}

fn click_text(state: &mut ClientShellState, text: &str) -> ClientShellInput {
    click_visible_text(state, 100, 30, text)
}

fn visible_text_position(
    state: &mut ClientShellState,
    cols: u16,
    rows: u16,
    text: &str,
) -> (u16, u16) {
    let frame = state.compose(cols, rows).unwrap();
    frame
        .cells
        .chunks(frame.width as usize)
        .enumerate()
        .find_map(|(y, row)| {
            let line: String = row.iter().map(|cell| cell.symbol.as_str()).collect();
            line.find(text)
                .map(|x| (line[..x].chars().count() as u16, y as u16))
        })
        .unwrap_or_else(|| panic!("missing {text}: {}", composed_text(&frame)))
}

fn click_visible_text(
    state: &mut ClientShellState,
    cols: u16,
    rows: u16,
    text: &str,
) -> ClientShellInput {
    let (x, y) = visible_text_position(state, cols, rows, text);
    state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::empty(),
        },
    )])
}

fn click_rect(state: &mut ClientShellState, rect: ratatui::layout::Rect) -> ClientShellInput {
    assert!(rect.width > 0 && rect.height > 0, "missing hitbox");
    state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: rect.x + rect.width / 2,
            row: rect.y + rect.height / 2,
            modifiers: KeyModifiers::empty(),
        },
    )])
}

fn record_request(outcome: &ClientShellInput, record_id: &str, revision: u64) -> String {
    assert!(
        outcome.requests.is_empty(),
        "management must not send terminal/lifecycle input"
    );
    let [ClientShellAction::Endpoint {
        endpoint_id,
        boot_id,
        request,
    }] = outcome.actions.as_slice()
    else {
        panic!("one qualified wake request: {:?}", outcome.actions);
    };
    assert_eq!(endpoint_id, &ClientEndpointId::Local);
    assert_eq!(boot_id, "boot-1");
    let crate::api::schema::Method::SnoozeRecordWake(params) = &request.method else {
        panic!("expected exact stored record wake: {:?}", request.method);
    };
    assert_eq!(params.record_id, record_id);
    assert_eq!(params.expected_revision, revision);
    assert_eq!(params.boot_id, "boot-1");
    request.id.clone()
}

#[test]
fn snooze_management_shows_details_and_clicking_only_selects() {
    let mut state = management_state();
    state.open_snooze_recovery(0, 0);
    assert!(matches!(
        state.overlay.as_ref(),
        Some(ClientShellOverlay::SnoozeManagement(_))
    ));
    let selected = click_text(&mut state, "Workspace Beta");
    assert!(selected.actions.is_empty() && selected.requests.is_empty());
    let frame = state.compose(100, 30).unwrap();
    let text = composed_text(&frame);
    for label in [
        "Workspace Beta",
        "Project Beta",
        "Available",
        "Outside current Focus",
    ] {
        assert!(text.contains(label), "missing {label}: {text}");
    }
    let wake = super::super::super::snooze_presets::wake_label_for_deadline(4_000_000);
    assert!(text.contains(&wake), "missing wake time {wake}: {text}");
    let selected = click_text(&mut state, "Old Space");
    assert!(selected.actions.is_empty() && selected.requests.is_empty());
    let text = composed_text(&state.compose(100, 30).unwrap());
    assert!(
        text.contains("Unavailable") && text.contains("Old Project"),
        "{text}"
    );
    let outcome = key(&mut state, KeyCode::Enter);
    record_request(&outcome, UNAVAILABLE_RECORD, 5);
}

#[test]
fn snooze_management_child_wake_preserves_parent_and_focus_then_offers_parent_wake() {
    let mut state = management_state();
    let focus = state.focus_scope.clone();
    let canonical = state.snapshot.clone();
    state.open_snooze_recovery(0, 0);
    click_text(&mut state, "Workspace Beta");
    let outcome = key(&mut state, KeyCode::Enter);
    let id = record_request(&outcome, WORKSPACE_RECORD, 5);
    assert!(!state.is_workspace_id_visible("ws_2"));
    let mut reply = state.snooze_state.clone().unwrap();
    reply.revision = 6;
    reply.records.clear();
    reply
        .persistence
        .as_mut()
        .unwrap()
        .records
        .retain(|record| record.record_id != WORKSPACE_RECORD);
    let (_, actions) = state.handle_endpoint_result(
        "boot-1",
        &id,
        Ok(crate::api::schema::ResponseResult::WorkspaceWake { state: reply }),
    );
    assert!(actions.is_empty());
    assert_eq!(state.focus_scope, focus);
    assert_eq!(state.snapshot, canonical);
    let Some(ClientShellOverlay::SnoozeManagement(view)) = &state.overlay else {
        panic!("management retained");
    };
    assert_eq!(
        view.records.len(),
        2,
        "a wake receipt must not become a duplicate snoozed row"
    );
    assert!(!view
        .records
        .iter()
        .any(|record| record.label == "Workspace Beta"));
    assert_eq!(
        state.snooze_state.as_ref().unwrap().project_records.len(),
        1
    );
    let text = composed_text(&state.compose(100, 30).unwrap());
    assert!(text.contains("Still snoozed by project"), "{text}");
    let parent = key(&mut state, KeyCode::Char('p'));
    assert!(parent.requests.is_empty());
    let [ClientShellAction::Endpoint { request, .. }] = parent.actions.as_slice() else {
        panic!("explicit parent wake");
    };
    assert!(
        matches!(&request.method, crate::api::schema::Method::ProjectWake(params)
        if params.project_key == "repo-b" && params.expected_revision == 4)
    );
}

#[test]
fn snooze_management_selection_clears_retained_parent_for_keyboard_and_mouse() {
    for mouse in [false, true] {
        let mut state = management_state();
        state.open_snooze_recovery(0, 0);
        click_text(&mut state, "Workspace Beta");
        let outcome = key(&mut state, KeyCode::Enter);
        let id = record_request(&outcome, WORKSPACE_RECORD, 5);
        let mut reply = state.snooze_state.clone().unwrap();
        reply.revision = 6;
        reply.records.clear();
        reply
            .persistence
            .as_mut()
            .unwrap()
            .records
            .retain(|record| record.record_id != WORKSPACE_RECORD);
        state.handle_endpoint_result(
            "boot-1",
            &id,
            Ok(crate::api::schema::ResponseResult::WorkspaceWake { state: reply }),
        );
        if mouse {
            click_text(&mut state, "Old Space");
        } else {
            let selected = key(&mut state, KeyCode::Down);
            assert!(selected.actions.is_empty() && selected.requests.is_empty());
        }
        let Some(ClientShellOverlay::SnoozeManagement(view)) = &state.overlay else {
            panic!("management retained");
        };
        assert!(view.parent_action.is_none());
        assert!(view.notice.is_none());
        let parent = key(&mut state, KeyCode::Char('p'));
        assert!(parent.actions.is_empty() && parent.requests.is_empty());
    }
}

#[test]
fn snooze_management_reset_is_visible_and_clickable_at_narrow_sizes() {
    for (cols, rows) in [(50, 20), (24, 8), (24, 10)] {
        let mut state = management_state();
        let focus = state.focus_scope.clone();
        state.open_snooze_recovery(0, 0);
        state.config.mouse_capture = false;
        state.compose(cols, rows).unwrap();
        assert_eq!(state.hits.snooze_management_reset.width, 0);
        state.config.mouse_capture = true;
        let frame = state.compose(cols, rows).unwrap();
        let text = composed_text(&frame);
        assert!(text.contains("Reset"), "{cols}x{rows}: {text}");
        if cols == 24 {
            assert!(text.contains("↵Wake"), "{text}");
            assert!(text.contains("Reset"), "{text}");
            assert!(text.contains("Esc"), "{text}");
            if rows == 8 {
                assert!(text.contains("Esc close"), "{text}");
            }
        }
        assert!(state.hits.snooze_management_reset.width > 0);
        let reset = state.hits.snooze_management_reset;
        let outcome = click_rect(&mut state, reset);
        assert!(outcome.actions.is_empty() && outcome.requests.is_empty());
        assert_eq!(state.focus_scope, focus);
        assert!(matches!(
            state.overlay,
            Some(ClientShellOverlay::ConfirmWakeSharedSnoozes(view))
                if view.purpose == ClientWakeConfirmationPurpose::ResetFocusSnooze
        ));
    }
}

#[test]
fn snooze_management_compact_actions_have_separate_wake_reset_and_close_hits() {
    for rows in [8, 10] {
        let mut state = management_state();
        state.open_snooze_recovery(0, 0);
        state.compose(24, rows).unwrap();
        let wake = state.hits.overlay_primary;
        let reset = state.hits.snooze_management_reset;
        let close = state.hits.overlay_cancel;
        assert!(wake.width > 0 && reset.width > 0 && close.width > 0);
        assert!(wake.right() <= reset.x);
        assert!(reset.right() <= close.x);
        let (_, footer_y) = visible_text_position(&mut state, 24, rows, "↵Wake");
        assert_eq!(footer_y, wake.y);

        let wake_outcome = click_visible_text(&mut state, 24, rows, "Wake");
        record_request(&wake_outcome, WORKSPACE_RECORD, 5);

        let mut state = management_state();
        let focus = state.focus_scope.clone();
        state.open_snooze_recovery(0, 0);
        let reset_outcome = click_visible_text(&mut state, 24, rows, "Reset");
        assert!(reset_outcome.actions.is_empty() && reset_outcome.requests.is_empty());
        assert_eq!(state.focus_scope, focus);
        assert!(matches!(
            state.overlay,
            Some(ClientShellOverlay::ConfirmWakeSharedSnoozes(view))
                if view.purpose == ClientWakeConfirmationPurpose::ResetFocusSnooze
        ));

        let mut state = management_state();
        let focus = state.focus_scope.clone();
        state.open_snooze_recovery(0, 0);
        let close_outcome = click_visible_text(&mut state, 24, rows, "Esc");
        assert!(close_outcome.actions.is_empty() && close_outcome.requests.is_empty());
        assert_eq!(state.focus_scope, focus);
        assert!(state.overlay.is_none());
    }
}

#[test]
fn snooze_management_stale_record_cannot_wake_replacement_or_neighbor() {
    for replaced in [false, true] {
        let mut state = management_state();
        state.open_snooze_recovery(0, 0);
        click_text(&mut state, "Workspace Beta");
        let mut newer = state.snooze_state.clone().unwrap();
        newer.revision = 6;
        if replaced {
            newer.records[0].revision = 6;
            newer.records[0].deadline_unix_ms += 1000;
            newer.persistence.as_mut().unwrap().records[0].deadline_unix_ms += 1000;
        } else {
            newer.records.clear();
            newer.persistence.as_mut().unwrap().records.remove(0);
        }
        state.set_snooze_state(newer);
        let wake = key(&mut state, KeyCode::Enter);
        assert!(wake.actions.is_empty() && wake.requests.is_empty());
        let text = composed_text(&state.compose(100, 30).unwrap());
        assert!(
            text.contains("changed") || text.contains("no longer") || text.contains("No longer"),
            "{text}"
        );
    }
}

#[test]
fn snooze_management_wake_failure_retains_record_and_focus() {
    let mut state = management_state();
    let focus = state.focus_scope.clone();
    let before = state.snooze_state.clone();
    state.open_snooze_recovery(0, 0);
    if let Some(ClientShellOverlay::SnoozeManagement(view)) = state.overlay.as_mut() {
        view.notice = Some("Woke an earlier item.".into());
    }
    click_text(&mut state, "Workspace Beta");
    let outcome = key(&mut state, KeyCode::Enter);
    let id = record_request(&outcome, WORKSPACE_RECORD, 5);
    state.handle_endpoint_result(
        "boot-1",
        &id,
        Err(ClientShellEndpointError {
            code: Some("persistence_failed".into()),
            message: "storage unavailable".into(),
        }),
    );
    assert_eq!(state.snooze_state, before);
    assert_eq!(state.focus_scope, focus);
    assert!(matches!(
        state.overlay.as_ref(),
        Some(ClientShellOverlay::SnoozeManagement(_))
    ));
    let text = composed_text(&state.compose(100, 30).unwrap());
    assert!(text.contains("storage unavailable"), "{text}");
}

#[test]
fn snooze_management_empty_narrow_and_no_mouse_keep_input_owned() {
    for (cols, rows) in [(100, 30), (44, 12), (24, 8)] {
        let mut state = management_state();
        state.set_focus_scope(Some(ClientFocusScope::StandaloneWorkspace {
            endpoint_id: ClientEndpointId::Local,
            boot_id: "boot-1".into(),
            workspace_id: "missing".into(),
        }));
        assert!(state.empty_presentation);
        state.open_snooze_recovery(0, 0);
        let frame = state.compose(cols, rows).unwrap();
        assert!(composed_text(&frame).contains("Snoozed"));
        if cols == 24 {
            let text = composed_text(&frame);
            let wake = super::super::super::snooze_presets::wake_label_for_deadline(4_000_000);
            assert!(
                text.contains(&wake[..16]),
                "compact view must show the complete wake date and minute: {text}"
            );
            assert!(
                text.contains("Project Beta"),
                "compact view must identify the project: {text}"
            );
            assert!(
                text.contains("Esc close"),
                "compact recovery must remain discoverable: {text}"
            );
        }
        let typing = key(&mut state, KeyCode::Char('x'));
        assert!(typing.requests.is_empty() && typing.actions.is_empty());
        state.config.mouse_capture = false;
        state.compose(cols, rows).unwrap();
        assert_eq!(state.hits.overlay_primary.width, 0);
        let cancel = key(&mut state, KeyCode::Esc);
        assert!(cancel.actions.is_empty() && cancel.requests.is_empty());
        assert!(state.overlay.is_none());
        assert!(state.empty_presentation);
    }
}

#[test]
fn snooze_management_delayed_reply_does_not_replace_a_new_overlay() {
    let mut state = management_state();
    state.open_snooze_recovery(0, 0);
    click_text(&mut state, "Workspace Beta");
    let outcome = key(&mut state, KeyCode::Enter);
    let id = record_request(&outcome, WORKSPACE_RECORD, 5);
    state.toggle_global_menu();
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::GlobalMenu(_))
    ));
    let mut reply = state.snooze_state.clone().unwrap();
    reply.revision = 6;
    reply.records.clear();
    reply
        .persistence
        .as_mut()
        .unwrap()
        .records
        .retain(|record| record.record_id != WORKSPACE_RECORD);
    state.handle_endpoint_result(
        "boot-1",
        &id,
        Ok(crate::api::schema::ResponseResult::WorkspaceWake { state: reply }),
    );
    assert_eq!(state.snooze_state.as_ref().unwrap().revision, 6);
    assert!(matches!(
        state.overlay,
        Some(ClientShellOverlay::GlobalMenu(_))
    ));
}

#[test]
fn snooze_management_no_mouse_clears_hits_with_and_without_a_surface() {
    for with_surface in [false, true] {
        let mut state = management_state();
        if with_surface {
            state.set_pane_surface(surface());
        }
        assert!(!state.empty_presentation);
        state.open_snooze_recovery(0, 0);
        state.compose(100, 30).unwrap();
        assert!(state.hits.overlay_primary.width > 0);
        state.config.mouse_capture = false;
        state.compose(100, 30).unwrap();
        assert_eq!(state.hits.overlay_primary.width, 0);
        assert!(state.hits.snooze_management_rows.is_empty());
        assert_eq!(state.hits.snooze_management_wake_parent.width, 0);
        assert_eq!(state.hits.snooze_management_wake_all.width, 0);
        assert_eq!(state.hits.snooze_management_reset.width, 0);
        let outcome = key(&mut state, KeyCode::Char('x'));
        assert!(outcome.actions.is_empty() && outcome.requests.is_empty());
    }
}

#[test]
fn snooze_management_all_projects_does_not_label_unavailable_as_outside_focus() {
    let mut state = management_state();
    state.clear_focus_scope();
    state.open_snooze_recovery(0, 0);
    click_text(&mut state, "Old Space");
    let text = composed_text(&state.compose(100, 30).unwrap());
    assert!(text.contains("Unavailable"));
    assert!(!text.contains("Outside current Focus"), "{text}");
}

#[test]
fn snooze_management_wake_receipt_uses_current_focus() {
    for clear_focus in [false, true] {
        let mut state = management_state();
        if !clear_focus {
            state.clear_focus_scope();
        }
        state.open_snooze_recovery(0, 0);
        click_text(&mut state, "Workspace Beta");
        let outcome = key(&mut state, KeyCode::Enter);
        let id = record_request(&outcome, WORKSPACE_RECORD, 5);
        if clear_focus {
            state.clear_focus_scope();
        } else {
            state.set_focus_scope(Some(ClientFocusScope::StandaloneWorkspace {
                endpoint_id: ClientEndpointId::Local,
                boot_id: "boot-1".into(),
                workspace_id: "ws_1".into(),
            }));
        }
        let focus = state.focus_scope.clone();
        let mut reply = state.snooze_state.clone().unwrap();
        reply.revision = 6;
        reply.records.clear();
        reply
            .persistence
            .as_mut()
            .unwrap()
            .records
            .retain(|record| record.record_id != WORKSPACE_RECORD);
        state.handle_endpoint_result(
            "boot-1",
            &id,
            Ok(crate::api::schema::ResponseResult::WorkspaceWake { state: reply }),
        );
        assert_eq!(state.focus_scope, focus);
        let Some(ClientShellOverlay::SnoozeManagement(view)) = &state.overlay else {
            panic!("management retained");
        };
        let notice = view.notice.as_deref().expect("wake receipt");
        assert_eq!(
            notice.contains("outside current Focus"),
            !clear_focus,
            "{notice}"
        );
    }
}

#[test]
fn snooze_management_changed_parent_cannot_be_woken_by_old_action() {
    let mut state = management_state();
    state.open_snooze_recovery(0, 0);
    click_text(&mut state, "Workspace Beta");
    let mut newer = state.snooze_state.clone().unwrap();
    newer.revision = 6;
    newer.project_records[0].revision = 6;
    newer.project_records[0].deadline_unix_ms += 1000;
    newer
        .persistence
        .as_mut()
        .unwrap()
        .records
        .iter_mut()
        .find(|record| record.record_id == PROJECT_RECORD)
        .unwrap()
        .deadline_unix_ms += 1000;
    state.set_snooze_state(newer);
    let wake = key(&mut state, KeyCode::Char('p'));
    assert!(wake.actions.is_empty() && wake.requests.is_empty());
    assert_eq!(
        state.snooze_state.as_ref().unwrap().project_records[0].revision,
        6
    );
}

fn inactive_management_state() -> (ClientShellState, ClientEndpointId) {
    use crate::client::endpoint::{ClientEndpointStatus, ProfileId, SavedSshEndpoint};

    let mut state = reset_ready_state();
    let profile = SavedSshEndpoint {
        id: ProfileId::parse("0123456789abcdef0123456789abcdef").unwrap(),
        label: "Remote build".into(),
        target: "dev@build.example".into(),
        session: "agents".into(),
        enabled: true,
    };
    let endpoint_id = ClientEndpointId::Ssh(profile.id.clone());
    state.set_endpoint_catalog(&[profile]);
    state.set_endpoint_status(&endpoint_id, ClientEndpointStatus::Online);
    let mut remote_snapshot = two_workspace_snapshot();
    remote_snapshot.boot_id = "remote-boot".into();
    state.set_endpoint_snapshot(&endpoint_id, Box::new(remote_snapshot));
    let mut remote_state = snooze_state(&["ws_2"], 7);
    remote_state.boot_id = "remote-boot".into();
    for record in &mut remote_state.records {
        record.boot_id = "remote-boot".into();
    }
    state.set_endpoint_snooze_state(&endpoint_id, remote_state);
    state.set_snooze_state(snooze_state(&[], 8));
    (state, endpoint_id)
}

#[test]
fn snooze_management_refreshes_availability_on_endpoint_status_change() {
    let (mut state, remote) = inactive_management_state();
    let active_before = state.active_endpoint_id.clone();
    let remote_state = state
        .endpoints
        .iter_mut()
        .find(|endpoint| endpoint.endpoint_id == remote)
        .and_then(|endpoint| endpoint.snooze_state.as_mut())
        .expect("remote snooze state");
    remote_state
        .records
        .push(crate::api::schema::WorkspaceSnoozeRecord {
            workspace_id: "ws_1".into(),
            boot_id: "remote-boot".into(),
            deadline_unix_ms: 4_000_001,
            revision: 6,
        });

    state.open_snooze_recovery(0, 0);
    if let Some(ClientShellOverlay::SnoozeManagement(management)) = state.overlay.as_mut() {
        management.selected = 1;
        management.scroll = 3;
        management.restriction = Some("preserve restriction".into());
        management.parent_action = management.records.first().cloned();
    } else {
        panic!("expected management overlay");
    }

    state.set_endpoint_status(
        &remote,
        crate::client::endpoint::ClientEndpointStatus::Reconnecting,
    );
    let Some(ClientShellOverlay::SnoozeManagement(management)) = state.overlay.as_ref() else {
        panic!("expected refreshed management overlay");
    };
    assert_eq!(management.endpoint_id, remote);
    assert_eq!(state.active_endpoint_id, active_before);
    assert_eq!(management.selected, 1);
    assert_eq!(management.scroll, 3);
    assert_eq!(management.restriction.as_deref(), Some("preserve restriction"));
    assert!(management.parent_action.is_some());
    assert!(!management.records[management.selected].available);

    state.set_endpoint_status(
        &remote,
        crate::client::endpoint::ClientEndpointStatus::Online,
    );
    let Some(ClientShellOverlay::SnoozeManagement(management)) = state.overlay.as_ref() else {
        panic!("expected refreshed management overlay");
    };
    assert_eq!(management.endpoint_id, remote);
    assert_eq!(state.active_endpoint_id, active_before);
    assert_eq!(management.selected, 1);
    assert_eq!(management.restriction.as_deref(), Some("preserve restriction"));
    assert!(management.parent_action.is_some());
    assert!(management.records[management.selected].available);
}

#[test]
fn inactive_only_recovery_is_aggregate_and_navigable_at_both_sizes() {
    let (mut state, remote) = inactive_management_state();
    assert_eq!(
        super::super::super::recovery_bar::snooze_count_all(&state),
        1
    );
    state.open_snooze_recovery(0, 0);
    let Some(ClientShellOverlay::SnoozeManagement(view)) = state.overlay.as_ref() else {
        panic!("expected inactive management view");
    };
    assert_eq!(view.endpoint_id, remote);
    assert_eq!(view.endpoint_label, "Remote build");
    let wide = composed_text(&state.compose(100, 30).unwrap());
    assert!(wide.contains("Remote build"), "{wide}");
    assert!(wide.contains("next ›"), "{wide}");
    assert!(state.hits.snooze_management_next.width > 0);
    let active_before = state.active_endpoint_id.clone();
    let _ = click_visible_text(&mut state, 100, 30, "next ›");
    assert_eq!(state.active_endpoint_id, active_before);
    assert!(matches!(
        state.overlay.as_ref(),
        Some(ClientShellOverlay::SnoozeManagement(view))
            if view.endpoint_id == ClientEndpointId::Local
    ));
    let _ = click_visible_text(&mut state, 100, 30, "‹ prev");
    assert!(matches!(
        state.overlay.as_ref(),
        Some(ClientShellOverlay::SnoozeManagement(view))
            if view.endpoint_id == remote
    ));
    let narrow = composed_text(&state.compose(24, 8).unwrap());
    assert!(narrow.contains("Snoozed"), "{narrow}");
    assert!(narrow.contains("›"), "{narrow}");
    assert!(state.hits.snooze_management_next.width > 0);
    let _ = click_visible_text(&mut state, 24, 8, "›");
    assert_eq!(state.active_endpoint_id, active_before);
    let Some(ClientShellOverlay::SnoozeManagement(view)) = state.overlay.as_ref() else {
        panic!("expected cycled management view");
    };
    assert_eq!(view.endpoint_id, ClientEndpointId::Local);
    assert!(view.records.is_empty());
    assert_eq!(
        view.notice.as_deref(),
        Some("No snoozed records on this server.")
    );
}

#[test]
fn inactive_management_wake_stays_qualified_after_cycling() {
    let (mut state, remote) = inactive_management_state();
    state.open_snooze_recovery(0, 0);
    let outcome = key(&mut state, KeyCode::Enter);
    let [ClientShellAction::Endpoint {
        endpoint_id,
        boot_id,
        request,
    }] = outcome.actions.as_slice()
    else {
        panic!("expected qualified remote wake: {:?}", outcome.actions);
    };
    assert_eq!(endpoint_id, &remote);
    assert_eq!(boot_id, "remote-boot");
    let request_id = request.id.clone();
    let _ = key(&mut state, KeyCode::Right);
    assert!(matches!(
        state.overlay.as_ref(),
        Some(ClientShellOverlay::SnoozeManagement(view))
            if view.endpoint_id == ClientEndpointId::Local
    ));
    let mut reply = snooze_state(&[], 8);
    reply.boot_id = "remote-boot".into();
    let _ = state.handle_endpoint_result(
        "remote-boot",
        &request_id,
        Ok(crate::api::schema::ResponseResult::WorkspaceWake { state: reply }),
    );
    assert!(matches!(
        state.overlay.as_ref(),
        Some(ClientShellOverlay::SnoozeManagement(view))
            if view.endpoint_id == ClientEndpointId::Local
    ));
    assert!(state
        .endpoints
        .iter()
        .find(|endpoint| endpoint.endpoint_id == remote)
        .and_then(|endpoint| endpoint.snooze_state.as_ref())
        .is_some_and(|state| state.records.is_empty()));
}

#[test]
fn inactive_management_wake_all_confirmation_targets_viewed_endpoint() {
    let (mut state, remote) = inactive_management_state();
    state.open_snooze_recovery(0, 0);
    let _ = key(&mut state, KeyCode::Char('a'));
    let Some(ClientShellOverlay::ConfirmWakeSharedSnoozes(confirm)) = state.overlay.as_ref() else {
        panic!("expected wake-all confirmation");
    };
    assert_eq!(confirm.endpoint_id, remote);
    assert_eq!(confirm.endpoint_label, "Remote build");
    let outcome = key(&mut state, KeyCode::Enter);
    let [ClientShellAction::Endpoint {
        endpoint_id,
        boot_id,
        request,
    }] = outcome.actions.as_slice()
    else {
        panic!("expected qualified wake-all request: {:?}", outcome.actions);
    };
    assert_eq!(endpoint_id, &remote);
    assert_eq!(boot_id, "remote-boot");
    assert!(matches!(
        &request.method,
        crate::api::schema::Method::WorkspaceWake(params)
            if params.workspace_id.is_none() && params.confirmed
    ));
}

#[test]
fn remote_reset_clears_unchanged_local_focus_after_success() {
    let (mut state, remote) = inactive_management_state();
    state.open_reset_confirmation(remote, "remote-boot".into(), 7, 1);
    let outcome = key(&mut state, KeyCode::Enter);
    let [ClientShellAction::Endpoint { request, .. }] = outcome.actions.as_slice() else {
        panic!("expected qualified reset request: {:?}", outcome.actions);
    };
    let request_id = request.id.clone();
    let mut reply = snooze_state(&[], 8);
    reply.boot_id = "remote-boot".into();
    let _ = state.handle_endpoint_result(
        "remote-boot",
        &request_id,
        Ok(crate::api::schema::ResponseResult::WorkspaceWake { state: reply }),
    );
    assert!(state.focus_scope.is_none());
}
