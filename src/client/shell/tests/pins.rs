use super::*;

use crate::api::schema::{
    AgentStatus, AgentViewBuiltinSortField, AgentViewSetParams, AgentViewSort, AgentViewSortField,
    AgentViewSortOrder, Method, PanePinParams, PaneTarget, PinState, ResponseResult,
    WorkspacePinParams, WorkspaceSnoozeRecord, WorkspaceSnoozeState, WorkspaceTarget,
};
use crate::client::endpoint::{
    ClientEndpointId, ClientEndpointStatus, ProfileId, SavedSshEndpoint,
};
use crate::client::shell::focus_snooze::ClientFocusScope;
use crate::config::{AgentPanelSortConfig, Config, StatusIndicatorStyle};
use crate::input::KeybindAction;
use crate::protocol::{ClientShellAgent, ClientShellWorktree};

fn pins(boot: &str, revision: u64, workspaces: &[&str], panes: &[&str]) -> PinState {
    PinState {
        boot_id: boot.into(),
        revision,
        workspace_ids: workspaces.iter().map(|id| (*id).to_owned()).collect(),
        pane_ids: panes.iter().map(|id| (*id).to_owned()).collect(),
    }
}

fn workspace(id: &str, number: usize) -> ClientShellWorkspace {
    let mut workspace = super::snapshot().workspaces.remove(0);
    workspace.workspace_id = id.into();
    workspace.label = id.into();
    workspace.number = number;
    workspace.focused = false;
    workspace.agent_status = AgentStatus::Idle;
    workspace.branch = None;
    workspace.worktree = None;
    workspace
}

fn linked(mut workspace: ClientShellWorkspace, key: &str, child: bool) -> ClientShellWorkspace {
    workspace.worktree = Some(ClientShellWorktree {
        key: key.into(),
        label: key.into(),
        is_linked_worktree: child,
    });
    if child {
        workspace.branch = Some(format!("worktree/{key}"));
    }
    workspace
}

fn snapshot_of(mut workspaces: Vec<ClientShellWorkspace>) -> ClientShellSnapshot {
    if !workspaces.iter().any(|workspace| workspace.focused) {
        if let Some(first) = workspaces.first_mut() {
            first.focused = true;
        }
    }
    let focused = workspaces
        .iter()
        .find(|workspace| workspace.focused)
        .map(|workspace| workspace.workspace_id.clone());
    let mut snapshot = super::snapshot();
    snapshot.workspaces = workspaces;
    snapshot.focused_workspace_id = focused;
    snapshot
}

fn shell(snapshot: ClientShellSnapshot) -> ClientShellState {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(snapshot));
    state.set_pane_surface(super::surface());
    state
}

fn pin_local(state: &mut ClientShellState, revision: u64, workspaces: &[&str], panes: &[&str]) {
    state.set_endpoint_pin_state(
        &ClientEndpointId::Local,
        pins("boot-1", revision, workspaces, panes),
    );
}

fn advertise_pins(state: &mut ClientShellState) {
    state.set_endpoint_methods(Some(vec![
        "pin.subscribe".into(),
        "workspace.pin".into(),
        "pane.pin".into(),
    ]));
}

fn nav_rows(state: &ClientShellState) -> Vec<(String, bool)> {
    let snapshot = state.snapshot.as_deref().expect("snapshot");
    state
        .navigation_workspace_entries(snapshot)
        .iter()
        .map(|entry| {
            (
                snapshot.workspaces[entry.index].workspace_id.clone(),
                entry.indented,
            )
        })
        .collect()
}

fn compose_hits(state: &mut ClientShellState, collapsed: bool) -> Vec<(String, bool)> {
    state.sidebar_collapsed = collapsed;
    state.sidebar_collapsed_manual = true;
    state.compose(140, 80).expect("sidebar frame for pin order");
    state
        .hits
        .workspaces
        .iter()
        .map(|hit| (hit.workspace_id.clone(), hit.indented))
        .collect()
}

fn shortcut_workspace(state: &mut ClientShellState, index: usize) -> Option<String> {
    match state.endpoint_method_for_action(KeybindAction::SwitchWorkspace(index)) {
        Some(Method::WorkspaceFocus(WorkspaceTarget { workspace_id })) => Some(workspace_id),
        _ => None,
    }
}

fn focus_workspace(state: &mut ClientShellState, action: KeybindAction) -> Option<String> {
    match state.endpoint_method_for_action(action) {
        Some(Method::WorkspaceFocus(WorkspaceTarget { workspace_id })) => Some(workspace_id),
        _ => None,
    }
}

fn focus_agent(state: &mut ClientShellState, index: usize) -> Option<String> {
    match state.endpoint_method_for_action(KeybindAction::FocusAgent(index)) {
        Some(Method::PaneFocus(PaneTarget { pane_id })) => Some(pane_id),
        _ => None,
    }
}

fn agent(
    pane_id: &str,
    workspace_id: &str,
    name: &str,
    status: AgentStatus,
    seq: u64,
) -> ClientShellAgent {
    ClientShellAgent {
        pane_id: pane_id.into(),
        workspace_id: workspace_id.into(),
        tab_id: "tab_1".into(),
        name: Some(name.into()),
        display_agent: None,
        agent: Some("claude".into()),
        title: None,
        terminal_title: None,
        terminal_title_stripped: None,
        agent_status: status,
        state_change_seq: seq,
        state_labels: Vec::new(),
        tokens: Vec::new(),
        focused: false,
    }
}

fn attach_remote(state: &mut ClientShellState, snapshot: ClientShellSnapshot) -> ClientEndpointId {
    let profile = SavedSshEndpoint {
        id: ProfileId::parse("0123456789abcdef0123456789abcdef").expect("profile id"),
        label: "Build".into(),
        target: "dev@build.example".into(),
        session: "agents".into(),
        enabled: true,
    };
    let endpoint_id = ClientEndpointId::Ssh(profile.id.clone());
    state.set_endpoint_catalog(&[profile]);
    state.set_endpoint_status(&endpoint_id, ClientEndpointStatus::Online);
    state.set_endpoint_snapshot(&endpoint_id, Box::new(snapshot));
    endpoint_id
}

fn menu_labels(state: &ClientShellState) -> Vec<String> {
    let Some(ClientShellOverlay::ContextMenu(menu)) = &state.overlay else {
        return Vec::new();
    };
    menu.items().into_iter().map(|item| item.label).collect()
}

fn symbol_columns(frame: &FrameData, rect: Rect, symbol: &str) -> Vec<u16> {
    let mut columns = Vec::new();
    for y in rect.y..rect.bottom().min(frame.height) {
        let row = usize::from(y) * usize::from(frame.width);
        for x in rect.x..rect.right().min(frame.width) {
            if frame.cells[row + usize::from(x)].symbol == symbol {
                columns.push(x);
            }
        }
    }
    columns
}

#[test]
fn pin_glyph_is_one_display_cell() {
    use unicode_width::UnicodeWidthStr;

    assert_eq!(crate::ui::PIN_GLYPH, "⚲");
    assert_eq!(UnicodeWidthStr::width(crate::ui::PIN_GLYPH), 1);
    assert_eq!(super::super::render::display_width(crate::ui::PIN_GLYPH), 1);
}

#[test]
fn pin_state_rejects_older_revision_and_other_boot_per_endpoint() {
    let mut remote_snapshot = super::snapshot();
    remote_snapshot.boot_id = "remote-boot".into();
    let mut state = shell(super::snapshot());
    let remote = attach_remote(&mut state, remote_snapshot);

    state.set_endpoint_pin_state(&ClientEndpointId::Local, pins("boot-1", 4, &["ws_1"], &[]));
    state.set_endpoint_pin_state(&ClientEndpointId::Local, pins("boot-1", 4, &["ws_2"], &[]));
    assert_eq!(
        state
            .pin_state
            .as_ref()
            .expect("equal revision applies")
            .workspace_ids,
        ["ws_2"]
    );
    state.set_endpoint_pin_state(&ClientEndpointId::Local, pins("boot-1", 3, &["stale"], &[]));
    state.set_endpoint_pin_state(
        &ClientEndpointId::Local,
        pins("other-boot", 9, &["other"], &[]),
    );
    assert_eq!(state.pin_state.as_ref().expect("local pin").revision, 4);
    assert_eq!(
        state.pin_state.as_ref().expect("local pin").workspace_ids,
        ["ws_2"]
    );

    state.set_endpoint_pin_state(&remote, pins("remote-boot", 9, &["ws_1"], &[]));
    state.set_endpoint_pin_state(&remote, pins("remote-boot", 8, &["stale"], &[]));
    state.set_endpoint_pin_state(&remote, pins("boot-1", 20, &["wrong-boot"], &[]));
    let remote_pin = state
        .endpoints
        .iter()
        .find(|endpoint| endpoint.endpoint_id == remote)
        .and_then(|endpoint| endpoint.pin_state.as_ref())
        .expect("remote pin");
    assert_eq!(remote_pin.revision, 9);
    assert_eq!(remote_pin.workspace_ids, ["ws_1"]);
    assert_eq!(
        state.pin_state.as_ref().expect("local untouched").revision,
        4
    );
}

#[test]
fn active_pin_cache_cannot_be_rolled_back_by_a_late_endpoint_update() {
    let mut state = shell(super::snapshot());
    state.pin_state = Some(pins("boot-1", 9, &["active"], &[]));
    state.endpoints[0].pin_state = Some(pins("boot-1", 8, &["endpoint"], &[]));
    state.set_endpoint_pin_state(&ClientEndpointId::Local, pins("boot-1", 7, &["late"], &[]));
    assert_eq!(state.pin_state.as_ref().expect("active").revision, 9);
    assert_eq!(
        state.pin_state.as_ref().expect("active").workspace_ids,
        ["active"]
    );
    assert_eq!(
        state.endpoints[0]
            .pin_state
            .as_ref()
            .expect("endpoint")
            .revision,
        8
    );
    assert_eq!(
        state.endpoints[0]
            .pin_state
            .as_ref()
            .expect("endpoint")
            .workspace_ids,
        ["endpoint"]
    );
}

#[test]
fn pin_state_follows_boot_and_clears_when_the_server_drops_the_feature() {
    let mut early = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    early.set_endpoint_pin_state(
        &ClientEndpointId::Local,
        pins("other-boot", 1, &["ws_1"], &[]),
    );
    assert!(
        early.pin_state.is_some(),
        "boot is unknown before the first snapshot"
    );
    early.set_snapshot(Box::new(super::snapshot()));
    assert!(early.pin_state.is_none());
    assert!(early.endpoints[0].pin_state.is_none());

    let mut state = shell(super::snapshot());
    pin_local(&mut state, 2, &["ws_1"], &[]);
    let mut same_boot = super::snapshot();
    same_boot.revision = 2;
    state.set_snapshot(Box::new(same_boot));
    assert_eq!(
        state
            .pin_state
            .as_ref()
            .expect("same boot keeps pins")
            .revision,
        2
    );
    let mut restarted = super::snapshot();
    restarted.boot_id = "boot-2".into();
    state.set_snapshot(Box::new(restarted));
    assert!(state.pin_state.is_none());

    let mut state = shell(super::snapshot());
    pin_local(&mut state, 2, &["ws_1"], &["pane_1"]);
    state.mark_endpoint_disconnected(&ClientEndpointId::Local);
    assert_eq!(
        state
            .pin_state
            .as_ref()
            .expect("disconnect keeps pins")
            .revision,
        2
    );
    assert_eq!(
        state.endpoints[0]
            .pin_state
            .as_ref()
            .expect("endpoint cache")
            .revision,
        2
    );

    state.retire_endpoint(&ClientEndpointId::Local);
    assert!(state.pin_state.is_none());
    assert!(state.endpoints[0].pin_state.is_none());

    let mut state = shell(super::snapshot());
    pin_local(&mut state, 2, &["ws_1"], &[]);
    state.set_endpoint_methods(Some(vec!["session.snapshot".into()]));
    assert!(state.pin_state.is_none());
    pin_local(&mut state, 3, &["ws_1"], &[]);
    state.set_endpoint_methods(None);
    assert_eq!(
        state
            .pin_state
            .as_ref()
            .expect("unknown methods keep pins")
            .revision,
        3
    );
    advertise_pins(&mut state);
    assert_eq!(
        state
            .pin_state
            .as_ref()
            .expect("advertised pins stay")
            .revision,
        3
    );
}

#[test]
fn pinned_eighth_workspace_leads_and_next_workspace_enters_the_unpinned_block() {
    let mut workspaces = (1..=10)
        .map(|number| workspace(&format!("ws_{number}"), number))
        .collect::<Vec<_>>();
    workspaces[7].focused = true;
    let mut state = shell(snapshot_of(workspaces));
    pin_local(&mut state, 1, &["ws_8"], &[]);
    let expected = [
        "ws_8", "ws_1", "ws_2", "ws_3", "ws_4", "ws_5", "ws_6", "ws_7", "ws_9", "ws_10",
    ];

    for collapsed in [false, true] {
        state.sidebar_collapsed = collapsed;
        state.sidebar_collapsed_manual = true;
        let rows = nav_rows(&state);
        assert_eq!(
            rows.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(),
            expected,
        );
        assert!(rows.iter().all(|(_, indented)| !indented));
        let rendered = compose_hits(&mut state, collapsed);
        assert_eq!(
            rendered
                .iter()
                .map(|(id, _)| id.as_str())
                .collect::<Vec<_>>(),
            expected,
        );
        let shortcuts = (0..expected.len())
            .map(|index| shortcut_workspace(&mut state, index).expect("shortcut"))
            .collect::<Vec<_>>();
        assert_eq!(shortcuts, expected);
        assert_eq!(
            focus_workspace(&mut state, KeybindAction::NextWorkspace).as_deref(),
            Some("ws_1")
        );
        assert_eq!(
            focus_workspace(&mut state, KeybindAction::PreviousWorkspace).as_deref(),
            Some("ws_10")
        );
    }
}

#[test]
fn pinned_workspace_blocks_keep_children_and_existing_relative_order() {
    let mut parent_a = linked(workspace("A", 1), "repo-a", false);
    parent_a.focused = true;
    let child = linked(workspace("A1", 2), "repo-a", true);
    let standalone = workspace("B", 3);
    let parent_c = linked(workspace("C", 4), "repo-c", false);
    let mut state = shell(snapshot_of(vec![parent_a, child, standalone, parent_c]));
    pin_local(&mut state, 1, &["C", "A"], &[]);

    let expected = vec![
        ("A".to_owned(), false),
        ("A1".to_owned(), true),
        ("C".to_owned(), false),
        ("B".to_owned(), false),
    ];
    assert_eq!(nav_rows(&state), expected);
    assert_eq!(compose_hits(&mut state, false), expected);
    state.mode = ClientShellMode::Navigate;
    state.compose(44, 40).expect("mobile pin order");
    let mobile = state
        .hits
        .mobile_targets
        .iter()
        .filter_map(|(_, target)| match target {
            ClientMobileTarget::Workspace { workspace_id, .. } => Some(workspace_id.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(mobile, ["A", "A1", "C", "B"]);
}

#[test]
fn pinned_linked_worktree_escapes_its_unpinned_parent_even_when_collapsed() {
    let parent = linked(workspace("P", 1), "repo", false);
    let sibling = linked(workspace("P1", 2), "repo", true);
    let mut pinned_child = linked(workspace("P2", 3), "repo", true);
    pinned_child.focused = true;
    let mut state = shell(snapshot_of(vec![parent, sibling, pinned_child]));
    pin_local(&mut state, 1, &["P2"], &[]);
    state.collapsed_groups.insert("repo".into());

    let escaped = [("P2".to_owned(), false), ("P".to_owned(), false)];
    assert_eq!(nav_rows(&state), escaped);
    assert_eq!(compose_hits(&mut state, false), escaped);
    assert_eq!(
        focus_workspace(&mut state, KeybindAction::NextWorkspace).as_deref(),
        Some("P"),
        "next workspace from the escaped pin enters the unpinned parent"
    );

    state.sidebar_collapsed = true;
    state.sidebar_collapsed_manual = true;
    assert_eq!(
        nav_rows(&state)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        ["P2", "P", "P1"],
        "compact strip still shows the unpinned sibling, after the escaped pin"
    );
    assert_eq!(
        compose_hits(&mut state, true)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        ["P2", "P", "P1"]
    );
}

#[test]
fn unpinned_compact_sidebar_keeps_snapshot_order() {
    let mut parent = linked(workspace("ws_1", 1), "repo", false);
    parent.focused = true;
    let standalone = workspace("ws_2", 2);
    let child = linked(workspace("ws_3", 3), "repo", true);
    let mut state = shell(snapshot_of(vec![parent, standalone, child]));
    state.collapsed_groups.insert("repo".into());
    state.sidebar_collapsed = true;
    state.sidebar_collapsed_manual = true;

    assert_eq!(
        nav_rows(&state)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        ["ws_1", "ws_2", "ws_3"]
    );
    state.sidebar_collapsed = false;
    assert_eq!(
        nav_rows(&state)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        ["ws_1", "ws_2"],
        "expanded collapsed group hides the unpinned child"
    );
}

#[test]
fn pinned_snoozed_workspace_stays_hidden_until_it_wakes_pinned() {
    let mut focused = workspace("ws_1", 1);
    focused.focused = true;
    let mut pinned = workspace("ws_2", 2);
    pinned.agent_status = AgentStatus::Working;
    let mut state = shell(snapshot_of(vec![focused, pinned]));
    pin_local(&mut state, 1, &["ws_2"], &[]);
    state.set_snooze_state(WorkspaceSnoozeState {
        boot_id: "boot-1".into(),
        revision: 1,
        records: vec![WorkspaceSnoozeRecord {
            workspace_id: "ws_2".into(),
            boot_id: "boot-1".into(),
            deadline_unix_ms: i64::MAX,
            revision: 1,
        }],
        project_records: Vec::new(),
        persistence: None,
    });
    assert_eq!(
        nav_rows(&state)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        ["ws_1"]
    );
    assert_eq!(
        compose_hits(&mut state, false)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        ["ws_1"]
    );

    state.set_snooze_state(WorkspaceSnoozeState {
        boot_id: "boot-1".into(),
        revision: 2,
        records: Vec::new(),
        project_records: Vec::new(),
        persistence: None,
    });
    assert_eq!(
        nav_rows(&state)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        ["ws_2", "ws_1"]
    );
    assert_eq!(
        compose_hits(&mut state, false)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        ["ws_2", "ws_1"]
    );
}

#[test]
fn pinned_workspace_outside_focus_stays_hidden() {
    let mut focused = workspace("ws_1", 1);
    focused.focused = true;
    let other = workspace("ws_2", 2);
    let mut state = shell(snapshot_of(vec![focused, other]));
    pin_local(&mut state, 1, &["ws_2"], &[]);
    state.set_focus_scope(Some(ClientFocusScope::StandaloneWorkspace {
        endpoint_id: ClientEndpointId::Local,
        boot_id: "boot-1".into(),
        workspace_id: "ws_1".into(),
    }));
    assert_eq!(
        nav_rows(&state)
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        ["ws_1"]
    );
    assert!(!compose_hits(&mut state, false)
        .iter()
        .any(|(id, _)| id == "ws_2"));
}

#[test]
fn pinned_idle_agent_leads_priority_sort_and_numbered_shortcuts() {
    let mut snapshot = super::snapshot();
    snapshot.agents = vec![
        agent("pane_work", "ws_1", "working", AgentStatus::Working, 10),
        agent("pane_idle", "ws_1", "idle", AgentStatus::Idle, 1),
    ];
    let mut state = shell(snapshot);
    state.config.agent_panel_sort = AgentPanelSortConfig::Priority;
    pin_local(&mut state, 1, &[], &["pane_idle"]);

    for collapsed in [false, true] {
        state.sidebar_collapsed = collapsed;
        state.sidebar_collapsed_manual = true;
        state.compose(140, 80).expect("agent pin order");
        let rendered = state
            .hits
            .agents
            .iter()
            .map(|(_, pane_id)| pane_id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(rendered, ["pane_idle", "pane_work"]);
        assert_eq!(focus_agent(&mut state, 0).as_deref(), Some("pane_idle"));
        assert_eq!(focus_agent(&mut state, 1).as_deref(), Some("pane_work"));
    }
}

#[test]
fn pin_order_is_per_machine_for_spaces_and_a_leading_block_for_priority() {
    let mut local = super::snapshot();
    local.agents = vec![
        agent("local-work", "ws_1", "local-work", AgentStatus::Working, 10),
        agent("local-idle", "ws_1", "local-idle", AgentStatus::Idle, 1),
    ];
    let mut remote_snapshot = super::snapshot();
    remote_snapshot.boot_id = "remote-boot".into();
    remote_snapshot.agents = vec![
        agent(
            "remote-work",
            "ws_1",
            "remote-work",
            AgentStatus::Working,
            10,
        ),
        agent("remote-idle", "ws_1", "remote-idle", AgentStatus::Idle, 1),
    ];
    let mut state = shell(local);
    let remote = attach_remote(&mut state, remote_snapshot);
    pin_local(&mut state, 1, &[], &["local-idle"]);
    state.set_endpoint_pin_state(&remote, pins("remote-boot", 1, &[], &["remote-idle"]));
    // Recency is the priority tiebreak. The pinned block stays in machine order;
    // the unpinned tail keeps that recency order.
    assert_eq!(state.endpoints[0].endpoint_id, ClientEndpointId::Local);
    state.endpoints[0]
        .agent_recency
        .insert("local-work".into(), 20);
    state.endpoints[0]
        .agent_recency
        .insert("local-idle".into(), 1);
    let remote_endpoint = state
        .endpoints
        .iter_mut()
        .find(|endpoint| endpoint.endpoint_id == remote)
        .expect("remote");
    remote_endpoint
        .agent_recency
        .insert("remote-work".into(), 10);
    remote_endpoint
        .agent_recency
        .insert("remote-idle".into(), 1);

    let names = |sort| {
        aggregate_navigation::aggregate_agent_rows(
            &state.endpoints,
            &state.active_endpoint_id,
            sort,
            false,
        )
        .into_iter()
        .map(|row| row.agent.name.clone().expect("name"))
        .collect::<Vec<_>>()
    };
    assert_eq!(
        names(AgentPanelSortConfig::Spaces),
        ["local-idle", "local-work", "remote-idle", "remote-work"]
    );
    assert_eq!(
        names(AgentPanelSortConfig::Priority),
        ["local-idle", "remote-idle", "local-work", "remote-work"]
    );
}

#[test]
fn custom_agent_view_sort_keeps_a_machine_ordered_pinned_block() {
    let mut local = super::snapshot();
    local.agents = vec![
        agent("ws_1:p9", "ws_1", "local-p9", AgentStatus::Idle, 1),
        agent("ws_1:p2", "ws_1", "local-p2", AgentStatus::Working, 2),
    ];
    let mut remote_snapshot = super::snapshot();
    remote_snapshot.boot_id = "remote-boot".into();
    remote_snapshot.agents = vec![
        agent("ws_1:p8", "ws_1", "remote-p8", AgentStatus::Idle, 1),
        agent("ws_1:p1", "ws_1", "remote-p1", AgentStatus::Working, 2),
    ];
    let mut state = shell(local);
    let remote = attach_remote(&mut state, remote_snapshot);
    pin_local(&mut state, 1, &[], &["ws_1:p9"]);
    state.set_endpoint_pin_state(&remote, pins("remote-boot", 1, &[], &["ws_1:p8"]));
    state.set_test_endpoint_agent_view(
        &ClientEndpointId::Local,
        Some(AgentViewSetParams {
            source: "pins.test".into(),
            label: Some("pane order".into()),
            filter: None,
            sort: vec![AgentViewSort {
                field: AgentViewSortField::Builtin(AgentViewBuiltinSortField::PaneOrder),
                order: AgentViewSortOrder::Asc,
            }],
        }),
    );

    let names = aggregate_navigation::aggregate_agent_rows(
        &state.endpoints,
        &state.active_endpoint_id,
        AgentPanelSortConfig::Spaces,
        false,
    )
    .into_iter()
    .map(|row| row.agent.name.clone().expect("name"))
    .collect::<Vec<_>>();
    assert_eq!(names, ["local-p9", "remote-p8", "remote-p1", "local-p2"]);
}

#[test]
fn pinned_agent_in_a_focused_out_workspace_stays_hidden() {
    let mut focused = workspace("ws_1", 1);
    focused.focused = true;
    let other = workspace("ws_2", 2);
    let mut snapshot = snapshot_of(vec![focused, other]);
    snapshot.agents = vec![
        agent("pane_visible", "ws_1", "visible", AgentStatus::Idle, 1),
        agent("pane_hidden", "ws_2", "hidden", AgentStatus::Working, 2),
    ];
    let mut state = shell(snapshot);
    pin_local(&mut state, 1, &["ws_2"], &["pane_hidden"]);
    state.set_focus_scope(Some(ClientFocusScope::StandaloneWorkspace {
        endpoint_id: ClientEndpointId::Local,
        boot_id: "boot-1".into(),
        workspace_id: "ws_1".into(),
    }));
    let names = aggregate_navigation::aggregate_agent_rows(
        &state.endpoints,
        &state.active_endpoint_id,
        AgentPanelSortConfig::Priority,
        false,
    )
    .into_iter()
    .map(|row| row.agent.pane_id.clone())
    .collect::<Vec<_>>();
    assert_eq!(names, ["pane_visible"]);
    state.compose(140, 80).expect("focus hides pinned agent");
    assert!(state
        .hits
        .agents
        .iter()
        .all(|(_, pane_id)| pane_id == "pane_visible"));
}

#[test]
fn pin_glyph_replaces_dots_prefixes_symbols_and_follows_status_color() {
    let mut blocked = workspace("ws_1", 1);
    blocked.focused = true;
    blocked.agent_status = AgentStatus::Blocked;
    let mut state = shell(snapshot_of(vec![blocked]));
    pin_local(&mut state, 1, &["ws_1"], &[]);

    state.config.status_indicators = StatusIndicatorStyle::Dots;
    let dots = state.compose(140, 40).expect("dots pin");
    let workspace_hit = state.hits.workspaces[0].rect;
    let pin_columns = symbol_columns(&dots, workspace_hit, crate::ui::PIN_GLYPH);
    assert_eq!(
        pin_columns.len(),
        1,
        "dots draw the pin in place of the status dot"
    );
    assert!(symbol_columns(&dots, workspace_hit, "●").is_empty());
    let dots_buffer = dots.to_ratatui_buffer().expect("dots buffer");
    let pin_cell = dots_buffer
        .cell((pin_columns[0], workspace_hit.y))
        .expect("pin cell");
    assert_eq!(pin_cell.fg, state.config.palette.red);

    state.sidebar_collapsed = true;
    state.sidebar_collapsed_manual = true;
    let collapsed = state.compose(140, 40).expect("collapsed dots pin");
    let collapsed_hit = state.hits.workspaces[0].rect;
    assert_eq!(
        symbol_columns(&collapsed, collapsed_hit, crate::ui::PIN_GLYPH).len(),
        1
    );
    assert!(symbol_columns(&collapsed, collapsed_hit, "●").is_empty());

    state.sidebar_collapsed = false;
    state.config.status_indicators = StatusIndicatorStyle::Symbols;
    let symbols = state.compose(140, 40).expect("symbols pin");
    let symbol_hit = state.hits.workspaces[0].rect;
    let pin_x = symbol_columns(&symbols, symbol_hit, crate::ui::PIN_GLYPH);
    let status_x = symbol_columns(&symbols, symbol_hit, "×");
    assert_eq!(pin_x.len(), 1);
    assert_eq!(status_x.len(), 1);
    assert!(pin_x[0] < status_x[0]);
    let symbols_buffer = symbols.to_ratatui_buffer().expect("symbols buffer");
    for column in [pin_x[0], status_x[0]] {
        let cell = symbols_buffer
            .cell((column, symbol_hit.y))
            .expect("status cell");
        assert_eq!(cell.fg, state.config.palette.red);
    }

    state.mode = ClientShellMode::Navigate;
    state.config.status_indicators = StatusIndicatorStyle::Symbols;
    let mobile = state.compose(44, 30).expect("mobile symbols pin");
    let workspace_rect = state
        .hits
        .mobile_targets
        .iter()
        .find_map(|(rect, target)| match target {
            ClientMobileTarget::Workspace { workspace_id, .. } if workspace_id == "ws_1" => {
                Some(*rect)
            }
            _ => None,
        })
        .expect("mobile workspace row");
    let mobile_pin = symbol_columns(&mobile, workspace_rect, crate::ui::PIN_GLYPH);
    let mobile_status = symbol_columns(&mobile, workspace_rect, "×");
    assert_eq!(mobile_pin.len(), 1);
    assert_eq!(mobile_status.len(), 1);
    assert!(mobile_pin[0] < mobile_status[0]);
    let mobile_buffer = mobile.to_ratatui_buffer().expect("mobile buffer");
    let mobile_cell = mobile_buffer
        .cell((mobile_pin[0], workspace_rect.y))
        .expect("mobile pin cell");
    assert_eq!(mobile_cell.fg, state.config.palette.red);
}

#[test]
fn stale_pinned_row_keeps_the_dim_overlay_style() {
    let mut local = super::snapshot();
    local.workspaces[0].agent_status = AgentStatus::Idle;
    let mut remote_snapshot = super::snapshot();
    remote_snapshot.boot_id = "remote-boot".into();
    remote_snapshot.workspaces[0].agent_status = AgentStatus::Blocked;
    remote_snapshot.workspaces[0].label = "remote-workspace".into();
    let mut state = shell(local);
    let remote = attach_remote(&mut state, remote_snapshot);
    state.set_endpoint_pin_state(&remote, pins("remote-boot", 1, &["ws_1"], &[]));
    state.mark_endpoint_disconnected(&remote);
    state.config.status_indicators = StatusIndicatorStyle::Dots;

    let frame = state.compose(140, 40).expect("stale pin row");
    let hit = state
        .hits
        .workspaces
        .iter()
        .find(|hit| hit.endpoint_id == remote)
        .expect("stale workspace row");
    let columns = symbol_columns(&frame, hit.rect, crate::ui::PIN_GLYPH);
    assert_eq!(columns.len(), 1);
    assert!(symbol_columns(&frame, hit.rect, "●").is_empty());
    let buffer = frame.to_ratatui_buffer().expect("stale buffer");
    let cell = buffer
        .cell((columns[0], hit.rect.y))
        .expect("stale pin cell");
    assert_eq!(cell.fg, state.config.palette.overlay0);
    assert!(cell.modifier.contains(ratatui::style::Modifier::DIM));
}

#[test]
fn pin_context_menu_items_follow_pin_subscribe_and_send_pin_requests() {
    let mut snapshot = super::snapshot();
    snapshot.agents = vec![agent("pane_1", "ws_1", "agent", AgentStatus::Idle, 1)];
    snapshot.panes.push(crate::protocol::ClientShellPane {
        pane_id: "pane_shell".into(),
        workspace_id: "ws_1".into(),
        tab_id: "tab_1".into(),
        label: None,
        cwd: None,
        foreground_cwd: None,
        focused: false,
        right_click_passthrough: false,
    });
    let mut state = shell(snapshot);
    pin_local(&mut state, 1, &[], &[]);

    state.open_workspace_context_menu("ws_1".into(), 1, 1);
    assert!(!menu_labels(&state)
        .iter()
        .any(
            |label| label.contains("workspace") && label.contains("Pin") || label.contains("Unpin")
        ));
    state.open_pane_context_menu("pane_1".into(), 1, 1);
    assert!(!menu_labels(&state)
        .iter()
        .any(|label| label.contains("agent")));
    state.overlay = None;
    state.open_agent_context_menu(ClientEndpointId::Local, "pane_1".into(), 1, 1);
    assert!(state.overlay.is_none());

    advertise_pins(&mut state);
    state.open_workspace_context_menu("ws_1".into(), 1, 1);
    let labels = menu_labels(&state);
    let pin_index = labels
        .iter()
        .position(|label| label == "Pin workspace")
        .expect("pin workspace");
    assert!(!labels.iter().any(|label| label == "Unpin workspace"));
    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(pin_index, &mut outcome);
    assert!(matches!(
        outcome.actions.as_slice(),
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                Method::WorkspacePin(WorkspacePinParams { workspace_id, pinned: true })
                    if workspace_id == "ws_1"
            )
    ));
    assert!(state.pending_requests.values().any(|pending| {
        pending.method_name == "workspace.pin"
            && matches!(
                &pending.kind,
                PendingEndpointKind::Pin { endpoint_id } if endpoint_id == &ClientEndpointId::Local
            )
    }));

    pin_local(&mut state, 2, &["ws_1"], &["pane_1"]);
    state.open_workspace_context_menu("ws_1".into(), 1, 1);
    assert!(menu_labels(&state)
        .iter()
        .any(|label| label == "Unpin workspace"));

    state.open_pane_context_menu("pane_shell".into(), 1, 1);
    assert!(!menu_labels(&state)
        .iter()
        .any(|label| label.contains("agent")));
    state.open_pane_context_menu("pane_1".into(), 1, 1);
    let pane_labels = menu_labels(&state);
    let unpin_agent = pane_labels
        .iter()
        .position(|label| label == "Unpin agent")
        .expect("unpin agent");
    let mut outcome = ClientShellInput::default();
    state.activate_context_menu_item(unpin_agent, &mut outcome);
    assert!(matches!(
        outcome.actions.as_slice(),
        [ClientShellAction::Endpoint { request, .. }]
            if matches!(
                &request.method,
                Method::PanePin(PanePinParams { pane_id, pinned: false }) if pane_id == "pane_1"
            )
    ));

    state.overlay = None;
    state.compose(140, 40).expect("agent hit");
    let (rect, pane_id) = state
        .hits
        .agents
        .iter()
        .find(|(_, pane_id)| pane_id == "pane_1")
        .expect("agent row")
        .clone();
    state.handle_raw_events(vec![crate::raw_input::RawInputEvent::Mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right),
            column: rect.x,
            row: rect.y,
            modifiers: crossterm::event::KeyModifiers::empty(),
        },
    )]);
    assert_eq!(pane_id, "pane_1");
    assert_eq!(menu_labels(&state), ["Unpin agent"]);

    state.set_endpoint_methods(Some(vec!["workspace.focus".into()]));
    state.open_workspace_context_menu("ws_1".into(), 1, 1);
    assert!(!menu_labels(&state)
        .iter()
        .any(|label| label.contains("Pin") || label.contains("Unpin")));
    state.open_agent_context_menu(ClientEndpointId::Local, "pane_1".into(), 2, 2);
    assert!(
        !matches!(state.overlay, Some(ClientShellOverlay::ContextMenu(ref menu)) if matches!(menu.target, ClientContextMenuTarget::Agent { .. }))
    );
}

#[test]
fn failed_pin_response_shows_the_existing_notice_and_keeps_pins() {
    let mut state = shell(super::snapshot());
    pin_local(&mut state, 4, &["ws_1"], &[]);
    state.pending_requests.insert(
        "client-shell:pin".into(),
        PendingEndpointRequest {
            boot_id: "boot-1".into(),
            method_name: "workspace.pin".into(),
            confirmation_workspace_id: None,
            kind: PendingEndpointKind::Pin {
                endpoint_id: ClientEndpointId::Local,
            },
        },
    );
    state.handle_endpoint_result(
        "boot-1",
        "client-shell:pin",
        Err(ClientShellEndpointError {
            code: Some("rejected".into()),
            message: "workspace is gone".into(),
        }),
    );
    assert_eq!(state.pin_state.as_ref().expect("unchanged").revision, 4);
    assert_eq!(
        state.pin_state.as_ref().expect("unchanged").workspace_ids,
        ["ws_1"]
    );
    assert_eq!(
        state
            .visible_endpoint_notice
            .as_ref()
            .expect("notice")
            .title,
        "Action rejected"
    );

    state.pending_requests.insert(
        "client-shell:pin-ok".into(),
        PendingEndpointRequest {
            boot_id: "boot-1".into(),
            method_name: "workspace.pin".into(),
            confirmation_workspace_id: None,
            kind: PendingEndpointKind::Pin {
                endpoint_id: ClientEndpointId::Local,
            },
        },
    );
    state.handle_endpoint_result(
        "boot-1",
        "client-shell:pin-ok",
        Ok(ResponseResult::Pin {
            state: pins("other-boot", 9, &["nope"], &[]),
        }),
    );
    assert_eq!(
        state
            .pin_state
            .as_ref()
            .expect("boot mismatch ignored")
            .revision,
        4
    );

    state.pending_requests.insert(
        "client-shell:pin-new".into(),
        PendingEndpointRequest {
            boot_id: "boot-1".into(),
            method_name: "workspace.pin".into(),
            confirmation_workspace_id: None,
            kind: PendingEndpointKind::Pin {
                endpoint_id: ClientEndpointId::Local,
            },
        },
    );
    state.handle_endpoint_result(
        "boot-1",
        "client-shell:pin-new",
        Ok(ResponseResult::Pin {
            state: pins("boot-1", 5, &["ws_9"], &["pane_9"]),
        }),
    );
    let applied = state.pin_state.as_ref().expect("applied");
    assert_eq!(applied.revision, 5);
    assert_eq!(applied.workspace_ids, ["ws_9"]);
    assert_eq!(applied.pane_ids, ["pane_9"]);
}
