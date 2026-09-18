use super::*;

impl ClientShellState {
    pub(super) fn record_binding(
        &mut self,
        binding: crate::input::KeybindMatch,
        outcome: &mut ClientShellInput,
    ) {
        match binding {
            crate::input::KeybindMatch::Action(crate::input::KeybindAction::Detach) => {
                outcome.detach = true;
            }
            crate::input::KeybindMatch::Action(crate::input::KeybindAction::ToggleSidebar) => {
                self.sidebar_collapsed = !self.sidebar_collapsed;
                self.sidebar_collapsed_manual = true;
                self.reveal_navigation_workspace = true;
                self.invalidate_pane_surface();
                outcome.repaint = true;
                outcome.resize = true;
                self.persist_chrome_preferences(outcome);
            }
            crate::input::KeybindMatch::Action(action) => {
                if self.workspace_preview_action_blocked()
                    && matches!(
                        action,
                        crate::input::KeybindAction::RenameWorkspace
                            | crate::input::KeybindAction::CloseWorkspace
                    )
                {
                    self.receive_endpoint_unavailable(
                        "Select an available workspace and press Enter before renaming or closing it"
                            .into(),
                    );
                    outcome.repaint = true;
                    return;
                }
                if matches!(
                    action,
                    crate::input::KeybindAction::NewWorktree
                        | crate::input::KeybindAction::OpenWorktree
                        | crate::input::KeybindAction::RemoveWorktree
                ) {
                    self.begin_worktree_action(action, outcome);
                    return;
                }
                if action == crate::input::KeybindAction::OpenNavigator {
                    self.open_navigator_overlay();
                    outcome.repaint = true;
                    return;
                }
                if action == crate::input::KeybindAction::Help {
                    self.overlay = Some(ClientShellOverlay::Help(ClientHelpOverlay {
                        query: TextEditor::default(),
                        search_focused: false,
                        scroll: 0,
                    }));
                    outcome.repaint = true;
                    return;
                }
                if action == crate::input::KeybindAction::Settings {
                    self.open_settings_overlay();
                    outcome.repaint = true;
                    return;
                }
                if action == crate::input::KeybindAction::OpenNotificationTarget {
                    self.focus_visible_notification(outcome);
                    return;
                }
                if action == crate::input::KeybindAction::ReloadConfig {
                    self.push_endpoint_method_with_kind(
                        crate::api::schema::Method::ServerReloadConfig(
                            crate::api::schema::EmptyParams::default(),
                        ),
                        PendingEndpointKind::ReloadConfig,
                        outcome,
                    );
                    self.reload_client_config();
                    self.invalidate_pane_surface();
                    outcome.repaint = true;
                    outcome.resize = true;
                    return;
                }
                if action == crate::input::KeybindAction::NewWorkspace {
                    if self.config.prompt_new_workspace_name {
                        self.open_new_workspace_overlay();
                    } else {
                        self.push_endpoint_method(
                            crate::api::schema::Method::WorkspaceCreate(
                                crate::api::schema::WorkspaceCreateParams {
                                    source_workspace_id: self.workspace_action_id(),
                                    cwd: None,
                                    focus: true,
                                    label: None,
                                    env: Default::default(),
                                },
                            ),
                            outcome,
                        );
                    }
                    outcome.repaint = true;
                    return;
                }
                if action == crate::input::KeybindAction::RenameWorkspace {
                    self.open_rename_workspace_overlay();
                    outcome.repaint = true;
                    return;
                }
                if action == crate::input::KeybindAction::CloseWorkspace {
                    if let Some(workspace_id) = self.workspace_action_id() {
                        if self.config.confirm_close {
                            self.open_confirm_close_overlay(workspace_id);
                        } else {
                            self.push_endpoint_method(
                                crate::api::schema::Method::WorkspaceClose(
                                    crate::api::schema::WorkspaceCloseParams {
                                        workspace_id,
                                        close_group: true,
                                    },
                                ),
                                outcome,
                            );
                        }
                    }
                    outcome.repaint = true;
                    return;
                }
                if action == crate::input::KeybindAction::NewTab && self.config.prompt_new_tab_name
                {
                    self.open_new_tab_overlay();
                    outcome.repaint = true;
                    return;
                }
                if action == crate::input::KeybindAction::RenameTab {
                    self.open_rename_tab_overlay();
                    outcome.repaint = true;
                    return;
                }
                if action == crate::input::KeybindAction::RenamePane {
                    self.open_rename_pane_overlay();
                    outcome.repaint = true;
                    return;
                }
                if action == crate::input::KeybindAction::WorkspacePicker {
                    self.mobile_switcher_scroll = 0;
                    self.reveal_mobile_workspace = false;
                    self.mode = ClientShellMode::Navigate;
                    self.navigate_workspace_id = self.focused_navigation_target();
                    self.reveal_navigation_workspace = true;
                    outcome.repaint = true;
                    return;
                }
                if action == crate::input::KeybindAction::EnterResizeMode {
                    self.mode = ClientShellMode::Resize;
                    outcome.repaint = true;
                    return;
                }
                if matches!(
                    action,
                    crate::input::KeybindAction::FocusProject
                        | crate::input::KeybindAction::ClearProjectFocus
                        | crate::input::KeybindAction::SnoozeWorkspace
                        | crate::input::KeybindAction::SnoozeProject
                        | crate::input::KeybindAction::ShowSnoozed
                        | crate::input::KeybindAction::ResetFocusSnooze
                ) {
                    self.activate_launcher_action(action, outcome);
                    return;
                }
                if action == crate::input::KeybindAction::CopyMode {
                    if self.enter_copy_mode(outcome) {
                        outcome.repaint = true;
                    }
                    return;
                }
                if self.handle_endpoint_navigation(action, outcome) {
                    return;
                }
                if let Some(method) = self.endpoint_method_for_action(action) {
                    self.push_endpoint_method(method, outcome);
                    return;
                }
                outcome.actions.push(ClientShellAction::Keybind(action));
            }
            crate::input::KeybindMatch::Command(command) => {
                let action = command.action.into();
                let resolved_labels = command.bindings.labels();
                let command_id = self.snapshot.as_deref().and_then(|snapshot| {
                    if let Some(candidate) = snapshot.commands.iter().find(|candidate| {
                        candidate.command_id == command.command && candidate.action == action
                    }) {
                        return Some(candidate.command_id.clone());
                    }
                    let mut candidates = snapshot.commands.iter().filter(|candidate| {
                        candidate.action == action
                            && !resolved_labels.is_empty()
                            && resolved_labels
                                .iter()
                                .all(|label| candidate.binding_labels.contains(label))
                    });
                    let candidate = candidates.next()?;
                    candidates
                        .next()
                        .is_none()
                        .then(|| candidate.command_id.clone())
                });
                let Some(command_id) = command_id else {
                    self.set_endpoint_error(
                        "custom command is not available on this endpoint; reload configuration",
                    );
                    outcome.repaint = true;
                    return;
                };
                let Some(snapshot) = self.snapshot.as_deref() else {
                    return;
                };
                let selection = (action == crate::protocol::ClientShellCommandAction::PluginAction)
                    .then(|| {
                        let selection = self.selection.as_ref()?;
                        if !selection.is_visible() {
                            return None;
                        }
                        if snapshot.focused_pane_id.as_deref() != Some(selection.pane_id.as_str()) {
                            return None;
                        }
                        let content_revision = self
                            .pane_surface
                            .as_ref()?
                            .panes
                            .iter()
                            .find(|pane| pane.pane_id == selection.pane_id)?
                            .content_revision;
                        let (anchor, cursor) = selection.ordered_cells();
                        Some(crate::api::schema::PaneSelectionReadParams {
                            pane_id: selection.pane_id.clone(),
                            anchor: crate::api::schema::PaneTextPoint {
                                row: anchor.0,
                                col: anchor.1,
                            },
                            cursor: crate::api::schema::PaneTextPoint {
                                row: cursor.0,
                                col: cursor.1,
                            },
                            content_revision: Some(content_revision),
                        })
                    })
                    .flatten();
                let params = crate::api::schema::CommandInvokeParams {
                    command_id,
                    workspace_id: snapshot.focused_workspace_id.clone(),
                    tab_id: snapshot.focused_tab_id.clone(),
                    pane_id: snapshot.focused_pane_id.clone(),
                    selection,
                };
                if action == crate::protocol::ClientShellCommandAction::Popup {
                    self.popup_pending = true;
                    self.popup_pending_deadline = None;
                    if !self.push_endpoint_method_with_kind(
                        crate::api::schema::Method::CommandInvoke(params),
                        PendingEndpointKind::PopupCommand,
                        outcome,
                    ) {
                        self.popup_pending = false;
                    }
                } else {
                    self.push_endpoint_method(
                        crate::api::schema::Method::CommandInvoke(params),
                        outcome,
                    );
                }
            }
        }
    }

    pub(super) fn request_selection_copy(&mut self, outcome: &mut ClientShellInput, live: bool) {
        let Some(selection) = self.selection.as_ref() else {
            return;
        };
        let pane_id = selection.pane_id.clone();
        let content_revision = self
            .pane_surface
            .as_ref()
            .and_then(|surface| surface.panes.iter().find(|pane| pane.pane_id == pane_id))
            .map(|pane| pane.content_revision)
            // Read an explicit selection atomically from the live terminal. Output
            // between the displayed frame and this request must not reject the copy.
            .filter(|_| !live);
        let (anchor, cursor) = selection.ordered_cells();
        self.push_endpoint_method_with_kind(
            crate::api::schema::Method::PaneSelectionRead(
                crate::api::schema::PaneSelectionReadParams {
                    pane_id,
                    anchor: crate::api::schema::PaneTextPoint {
                        row: anchor.0,
                        col: anchor.1,
                    },
                    cursor: crate::api::schema::PaneTextPoint {
                        row: cursor.0,
                        col: cursor.1,
                    },
                    content_revision,
                },
            ),
            PendingEndpointKind::SelectionCopy,
            outcome,
        );
    }

    pub(super) fn push_endpoint_method(
        &mut self,
        method: crate::api::schema::Method,
        outcome: &mut ClientShellInput,
    ) {
        self.push_endpoint_method_with_kind(method, PendingEndpointKind::Generic, outcome);
    }

    pub(super) fn push_endpoint_notice(
        &mut self,
        kind: ClientEndpointNoticeKind,
        code: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> bool {
        let boot_id = self
            .snapshot
            .as_deref()
            .map(|snapshot| snapshot.boot_id.clone())
            .unwrap_or_else(|| "disconnected".to_owned());
        self.push_endpoint_notice_with_boot(&boot_id, kind, code, title, body)
    }

    fn push_endpoint_notice_for(
        &mut self,
        endpoint_id: &ClientEndpointId,
        boot_id: &str,
        kind: ClientEndpointNoticeKind,
        code: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> bool {
        let boot_id = format!("{}:{boot_id}", endpoint_id.storage_key());
        self.push_endpoint_notice_with_boot(&boot_id, kind, code, title, body)
    }

    fn push_endpoint_notice_with_boot(
        &mut self,
        boot_id: &str,
        kind: ClientEndpointNoticeKind,
        code: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> bool {
        let key = ClientEndpointNoticeKey {
            boot_id: boot_id.to_owned(),
            kind,
            code: code.into(),
        };
        let body = body.into();
        if kind == ClientEndpointNoticeKind::Rejected {
            if self
                .visible_endpoint_notice
                .as_ref()
                .is_some_and(|notice| notice.key == key && notice.body == body)
            {
                return false;
            }
        } else if !self.endpoint_notice_seen.insert(key.clone()) {
            return false;
        }
        let duration_seconds = if kind == ClientEndpointNoticeKind::Rejected {
            3
        } else {
            8
        };
        self.visible_endpoint_notice = Some(ClientVisibleEndpointNotice {
            key,
            title: title.into(),
            body,
            deadline: std::time::Instant::now() + std::time::Duration::from_secs(duration_seconds),
        });
        true
    }

    pub(super) fn push_endpoint_method_with_kind(
        &mut self,
        method: crate::api::schema::Method,
        kind: PendingEndpointKind,
        outcome: &mut ClientShellInput,
    ) -> bool {
        let endpoint_id = self.active_endpoint_id.clone();
        let Some(boot_id) = self.endpoint_boot_id(&endpoint_id).map(str::to_owned) else {
            let label = self.active_endpoint_label().to_owned();
            outcome.repaint |= self.receive_endpoint_unavailable(format!("{label} is not ready"));
            return false;
        };
        self.push_endpoint_method_for(&endpoint_id, boot_id, method, kind, outcome)
    }

    pub(super) fn push_endpoint_method_for(
        &mut self,
        endpoint_id: &ClientEndpointId,
        boot_id: String,
        method: crate::api::schema::Method,
        kind: PendingEndpointKind,
        outcome: &mut ClientShellInput,
    ) -> bool {
        let inactive_shared = endpoint_id != &self.active_endpoint_id
            && crate::client::endpoint_commands::is_shared_snooze_method(&method);
        if endpoint_id != &self.active_endpoint_id && !inactive_shared {
            return false;
        }
        if !self.endpoint_is_online(endpoint_id) {
            let label = self.endpoint_label(endpoint_id).to_owned();
            let body = format!("{label} is not ready");
            outcome.repaint |= if crate::client::endpoint_commands::is_shared_snooze_method(&method)
            {
                self.push_endpoint_notice_for(
                    endpoint_id,
                    &boot_id,
                    ClientEndpointNoticeKind::Unavailable,
                    body.clone(),
                    "Endpoint unavailable",
                    body,
                )
            } else {
                self.receive_endpoint_unavailable(body)
            };
            return false;
        }
        let method_name = crate::api::api_method_name(&method).to_owned();
        if !self.supports_endpoint_method_for(endpoint_id, &method) {
            let body = format!(
                "This server does not support {method_name} yet. Update and restart it to enable this action."
            );
            outcome.repaint |= if crate::client::endpoint_commands::is_shared_snooze_method(&method)
            {
                self.push_endpoint_notice_for(
                    endpoint_id,
                    &boot_id,
                    ClientEndpointNoticeKind::Unsupported,
                    method_name.clone(),
                    "Action unavailable",
                    body,
                )
            } else {
                self.push_endpoint_notice(
                    ClientEndpointNoticeKind::Unsupported,
                    method_name.clone(),
                    "Action unavailable",
                    body,
                )
            };
            return false;
        }
        let Some(snapshot) = self
            .endpoints
            .iter()
            .find(|endpoint| &endpoint.endpoint_id == endpoint_id)
            .and_then(|endpoint| endpoint.snapshot.as_deref())
        else {
            return false;
        };
        if snapshot.boot_id != boot_id {
            return false;
        }
        let confirmation_workspace_id = match &method {
            crate::api::schema::Method::TabClose(target) => snapshot
                .tabs
                .iter()
                .find(|tab| tab.tab_id == target.tab_id)
                .map(|tab| tab.workspace_id.clone()),
            crate::api::schema::Method::PaneClose(target) => snapshot
                .panes
                .iter()
                .find(|pane| pane.pane_id == target.pane_id)
                .map(|pane| pane.workspace_id.clone()),
            _ => None,
        };
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.saturating_add(1);
        let request_id = format!("client-shell:{request_id}");
        self.pending_requests.insert(
            request_id.clone(),
            PendingEndpointRequest {
                boot_id: snapshot.boot_id.clone(),
                method_name,
                confirmation_workspace_id,
                kind: if inactive_shared && matches!(&kind, PendingEndpointKind::Generic) {
                    PendingEndpointKind::SharedSnooze {
                        endpoint_id: endpoint_id.clone(),
                    }
                } else {
                    kind
                },
            },
        );
        outcome.actions.push(ClientShellAction::Endpoint {
            endpoint_id: endpoint_id.clone(),
            boot_id: snapshot.boot_id.clone(),
            request: Box::new(crate::api::schema::Request {
                id: request_id,
                method,
            }),
        });
        true
    }

    pub(crate) fn receive_endpoint_error(&mut self, message: String) -> bool {
        self.push_endpoint_notice(
            ClientEndpointNoticeKind::Rejected,
            "paste_rejected",
            "Paste rejected",
            message,
        )
    }

    pub(crate) fn receive_endpoint_unavailable(&mut self, message: String) -> bool {
        self.push_endpoint_notice(
            ClientEndpointNoticeKind::Unavailable,
            message.clone(),
            "Endpoint unavailable",
            message,
        )
    }

    pub(crate) fn focus_endpoint_target(
        &mut self,
        target: ClientEndpointFocusTarget,
    ) -> Vec<ClientShellAction> {
        let method = match target {
            ClientEndpointFocusTarget::Workspace(workspace_id) => {
                crate::api::schema::Method::WorkspaceFocus(crate::api::schema::WorkspaceTarget {
                    workspace_id,
                })
            }
            ClientEndpointFocusTarget::Tab(tab_id) => {
                crate::api::schema::Method::TabFocus(crate::api::schema::TabTarget { tab_id })
            }
            ClientEndpointFocusTarget::Pane(pane_id) => {
                crate::api::schema::Method::PaneFocus(crate::api::schema::PaneTarget { pane_id })
            }
        };
        let mut outcome = ClientShellInput::default();
        self.push_endpoint_method(method, &mut outcome);
        outcome.actions
    }

    pub(crate) fn cancel_endpoint_request(&mut self, request_id: &str) -> bool {
        let Some(pending) = self.pending_requests.get(request_id) else {
            return false;
        };
        let boot_id = pending.boot_id.clone();
        let (repaint, actions) = self.handle_endpoint_result(
            &boot_id,
            request_id,
            Err(ClientShellEndpointError {
                code: Some("endpoint_cancelled".into()),
                message: "This server action was interrupted. Check its state before retrying."
                    .into(),
            }),
        );
        debug_assert!(
            actions.is_empty(),
            "cancellation must not start another action"
        );
        repaint
    }

    pub(crate) fn endpoint_request_can_complete(
        &self,
        endpoint_id: &ClientEndpointId,
        request_id: &str,
    ) -> bool {
        self.endpoint_is_active(endpoint_id)
            || self
                .pending_requests
                .get(request_id)
                .is_some_and(|pending| {
                    matches!(
                        &pending.kind,
                        PendingEndpointKind::SharedSnooze {
                            endpoint_id: pending_endpoint_id,
                        }
                            | PendingEndpointKind::SnoozeReset {
                                endpoint_id: pending_endpoint_id,
                                ..
                            }
                            | PendingEndpointKind::SnoozeManagementWake {
                                endpoint_id: pending_endpoint_id,
                                ..
                            } if pending_endpoint_id == endpoint_id
                    )
                })
    }

    pub(crate) fn handle_endpoint_result(
        &mut self,
        boot_id: &str,
        request_id: &str,
        result: Result<crate::api::schema::ResponseResult, ClientShellEndpointError>,
    ) -> (bool, Vec<ClientShellAction>) {
        let Some(pending) = self.pending_requests.remove(request_id) else {
            return (false, Vec::new());
        };
        let shared_snooze_endpoint = match &pending.kind {
            PendingEndpointKind::SharedSnooze { endpoint_id }
            | PendingEndpointKind::SnoozeReset { endpoint_id, .. }
            | PendingEndpointKind::SnoozeManagementWake { endpoint_id, .. } => Some(endpoint_id),
            _ => None,
        };
        let target_snapshot = shared_snooze_endpoint
            .map(|endpoint_id| {
                self.endpoints
                    .iter()
                    .find(|endpoint| &endpoint.endpoint_id == endpoint_id)
                    .and_then(|endpoint| endpoint.snapshot.as_deref())
            })
            .unwrap_or_else(|| self.snapshot.as_deref());
        if pending.boot_id != boot_id
            || target_snapshot.is_none_or(|snapshot| snapshot.boot_id != boot_id)
        {
            return (false, Vec::new());
        }
        if let PendingEndpointKind::PaneLinkResolve { target } = pending.kind {
            return self.complete_link_hover(target, result);
        }
        if result.is_ok() {
            let timeout_key = shared_snooze_endpoint.map_or_else(
                || ClientEndpointNoticeKey {
                    boot_id: boot_id.to_owned(),
                    kind: ClientEndpointNoticeKind::Timeout,
                    code: pending.method_name.clone(),
                },
                |endpoint_id| ClientEndpointNoticeKey {
                    boot_id: format!("{}:{boot_id}", endpoint_id.storage_key()),
                    kind: ClientEndpointNoticeKind::Timeout,
                    code: pending.method_name.clone(),
                },
            );
            self.endpoint_notice_seen.remove(&timeout_key);
        }
        if let Err(error) = &result {
            let code = error.code.as_deref().unwrap_or("invalid_response");
            if !matches!(
                code,
                "confirmation_required" | "stale_content" | "stale_target"
            ) {
                let (kind, notice_code, title, body) = match code {
                    "endpoint_timeout" => (
                        ClientEndpointNoticeKind::Timeout,
                        pending.method_name.clone(),
                        "Server timed out",
                        format!("This server did not respond to {}.", pending.method_name),
                    ),
                    "endpoint_cancelled" => (
                        ClientEndpointNoticeKind::Unavailable,
                        "cancelled".to_owned(),
                        "Action interrupted",
                        error.message.clone(),
                    ),
                    "server_unavailable" => (
                        ClientEndpointNoticeKind::Unavailable,
                        "server".to_owned(),
                        "Server unavailable",
                        error.message.clone(),
                    ),
                    _ => (
                        ClientEndpointNoticeKind::Rejected,
                        format!("{}:{code}", pending.method_name),
                        "Action rejected",
                        error.message.clone(),
                    ),
                };
                let body = shared_snooze_endpoint
                    .filter(|endpoint_id| *endpoint_id != &self.active_endpoint_id)
                    .map_or(body.clone(), |endpoint_id| {
                        format!("{}: {body}", self.endpoint_label(endpoint_id))
                    });
                if let Some(endpoint_id) = shared_snooze_endpoint {
                    self.push_endpoint_notice_for(
                        endpoint_id,
                        boot_id,
                        kind,
                        notice_code,
                        title,
                        body,
                    );
                } else {
                    self.push_endpoint_notice(kind, notice_code, title, body);
                }
            }
        }
        match pending.kind {
            PendingEndpointKind::Generic => {}
            PendingEndpointKind::SharedSnooze { endpoint_id } => {
                let state = match result {
                    Ok(crate::api::schema::ResponseResult::WorkspaceSnooze { state })
                    | Ok(crate::api::schema::ResponseResult::WorkspaceWake { state }) => state,
                    _ => return (true, Vec::new()),
                };
                if state.boot_id != boot_id {
                    return (false, Vec::new());
                }
                let current_revision = self
                    .endpoints
                    .iter()
                    .find(|endpoint| endpoint.endpoint_id == endpoint_id)
                    .and_then(|endpoint| endpoint.snooze_state.as_ref())
                    .filter(|current| current.boot_id == boot_id)
                    .map_or(0, |current| current.revision);
                let mut outcome = ClientShellInput::default();
                if state.revision >= current_revision {
                    outcome
                        .actions
                        .extend(self.set_endpoint_snooze_state(&endpoint_id, state));
                }
                return (true, outcome.actions);
            }
            PendingEndpointKind::SnoozeManagementWake {
                endpoint_id,
                label,
                project_key,
                covered_by_project,
                focus_target,
            } => {
                let management_active = matches!(
                    self.overlay.as_ref(),
                    Some(ClientShellOverlay::SnoozeManagement(management))
                        if management.endpoint_id == endpoint_id
                            && management.boot_id == boot_id
                );
                match result {
                    Ok(crate::api::schema::ResponseResult::WorkspaceWake { state }) => {
                        if state.boot_id != boot_id {
                            return (false, Vec::new());
                        }
                        let current_revision = self
                            .endpoints
                            .iter()
                            .find(|endpoint| endpoint.endpoint_id == endpoint_id)
                            .and_then(|endpoint| endpoint.snooze_state.as_ref())
                            .filter(|current| current.boot_id == boot_id)
                            .map_or(0, |current| current.revision);
                        let mut outcome = ClientShellInput::default();
                        if state.revision >= current_revision {
                            outcome
                                .actions
                                .extend(self.set_endpoint_snooze_state(&endpoint_id, state));
                        }
                        let endpoint_snapshot = self
                            .endpoints
                            .iter()
                            .find(|endpoint| endpoint.endpoint_id == endpoint_id)
                            .and_then(|endpoint| endpoint.snapshot.as_deref());
                        let retained = management_active
                            .then_some(covered_by_project)
                            .and_then(|covered| covered.then_some(project_key))
                            .flatten()
                            .and_then(|project_key| {
                                let project = self
                                    .endpoints
                                    .iter()
                                    .find(|endpoint| endpoint.endpoint_id == endpoint_id)
                                    .and_then(|endpoint| endpoint.snooze_state.as_ref())?
                                    .project_records
                                    .iter()
                                    .find(|project| {
                                        project.boot_id == boot_id
                                            && project.project_key == project_key
                                    })?;
                                let project_label = endpoint_snapshot
                                    .and_then(|snapshot| {
                                        snapshot.workspaces.iter().find(|workspace| {
                                            workspace
                                                .worktree
                                                .as_ref()
                                                .is_some_and(|worktree| worktree.key == project_key)
                                        })
                                    })
                                    .and_then(|workspace| workspace.worktree.as_ref())
                                    .map(|worktree| worktree.label.clone())
                                    .unwrap_or_else(|| project_key.clone());
                                Some(ClientSnoozeManagementRecord {
                                    created_unix_ms: None,
                                    target: ClientSnoozeManagementTarget::Project {
                                        project_key: project_key.clone(),
                                        revision: project.revision,
                                    },
                                    label: label.clone(),
                                    scope: "Workspace".to_owned(),
                                    project_label: Some(project_label),
                                    deadline_unix_ms: project.deadline_unix_ms,
                                    available: false,
                                    outside_focus: false,
                                    covered_by_project: true,
                                    project_key: Some(project_key),
                                    project_revision: Some(project.revision),
                                    workspace_id: None,
                                })
                            });
                        let outside_focus = self.snooze_management_currently_outside_focus(
                            &endpoint_id,
                            boot_id,
                            focus_target.as_ref(),
                        );
                        let message = retained.as_ref().map_or_else(
                            || if outside_focus {
                                format!("Woke {label}. It remains outside current Focus.")
                            } else {
                                format!("Woke {label}.")
                            },
                            |record| if outside_focus {
                                format!("Woke {label}. Still snoozed by project {}; it remains outside current Focus.", record.project_label.as_deref().unwrap_or("parent"))
                            } else {
                                format!("Woke {label}. Still snoozed by project {}.", record.project_label.as_deref().unwrap_or("parent"))
                            },
                        );
                        if management_active {
                            self.open_snooze_management_for_endpoint(
                                endpoint_id.clone(),
                                Some(message),
                            );
                            if let Some(retained) = retained {
                                if let Some(ClientShellOverlay::SnoozeManagement(management)) =
                                    self.overlay.as_mut()
                                {
                                    management.parent_action = Some(retained);
                                }
                            }
                        }
                        return (true, outcome.actions);
                    }
                    Err(error) => {
                        if let Some(ClientShellOverlay::SnoozeManagement(management)) =
                            self.overlay.as_mut().filter(|overlay| {
                                matches!(
                                    overlay,
                                    ClientShellOverlay::SnoozeManagement(management)
                                        if management.endpoint_id == endpoint_id
                                            && management.boot_id == boot_id
                                )
                            })
                        {
                            management.restriction = Some(error.message);
                        }
                        return (true, Vec::new());
                    }
                    _ => return (true, Vec::new()),
                }
            }
            PendingEndpointKind::SnoozeReset {
                endpoint_id,
                captured_focus,
            } => {
                if !self.endpoint_is_online(&endpoint_id)
                    || self.endpoint_boot_id(&endpoint_id) != Some(boot_id)
                {
                    return (false, Vec::new());
                }
                let state = match result {
                    Ok(crate::api::schema::ResponseResult::WorkspaceSnooze { state })
                    | Ok(crate::api::schema::ResponseResult::WorkspaceWake { state }) => state,
                    _ => return (true, Vec::new()),
                };
                if state.boot_id != boot_id {
                    return (true, Vec::new());
                }
                let current_revision = self
                    .endpoints
                    .iter()
                    .find(|endpoint| endpoint.endpoint_id == endpoint_id)
                    .and_then(|endpoint| endpoint.snooze_state.as_ref())
                    .filter(|current| current.boot_id == boot_id)
                    .map_or(0, |current| current.revision);
                let mut outcome = ClientShellInput::default();
                if state.revision >= current_revision {
                    outcome
                        .actions
                        .extend(self.set_endpoint_snooze_state(&endpoint_id, state));
                }
                if self.focus_scope == captured_focus {
                    outcome.actions.extend(self.clear_focus_scope());
                }
                return (true, outcome.actions);
            }
            PendingEndpointKind::PaneLinkResolve { .. } => unreachable!("handled above"),
            PendingEndpointKind::ProductAnnouncementDismiss { version, id } => {
                return match result {
                    Ok(_) => (false, Vec::new()),
                    Err(_) => {
                        let key = (version.clone(), id.clone());
                        if self.dismissed_product_announcement.as_ref() == Some(&key) {
                            self.dismissed_product_announcement = None;
                        }
                        if self.overlay.is_none() {
                            if let Some(announcement) = self
                                .snapshot
                                .as_deref()
                                .and_then(|snapshot| snapshot.product_announcement.as_ref())
                                .filter(|announcement| {
                                    announcement.version == version && announcement.id == id
                                })
                            {
                                self.overlay = Some(ClientShellOverlay::ProductAnnouncement(
                                    product_announcement_state(announcement),
                                ));
                            }
                        }
                        (true, Vec::new())
                    }
                };
            }
            PendingEndpointKind::ReleaseNotesDismiss => {
                return match result {
                    Ok(_) => (false, Vec::new()),
                    Err(_) => {
                        if self.overlay.is_none() {
                            if let Some(notes) = self
                                .snapshot
                                .as_deref()
                                .and_then(|snapshot| snapshot.release_notes.as_ref())
                            {
                                self.overlay = Some(ClientShellOverlay::ReleaseNotes(
                                    release_notes_state(notes),
                                ));
                            }
                        }
                        (true, Vec::new())
                    }
                };
            }
            PendingEndpointKind::PopupCommand => {
                return match result {
                    Ok(_) => {
                        self.popup_pending_deadline =
                            Some(std::time::Instant::now() + std::time::Duration::from_secs(1));
                        (false, Vec::new())
                    }
                    Err(_) => {
                        self.popup_pending = false;
                        self.popup_pending_deadline = None;
                        (true, Vec::new())
                    }
                };
            }
            PendingEndpointKind::PaneScroll { pane_id, serial } => {
                let mut outcome = ClientShellInput::default();
                let repaint = self.complete_pane_scroll(pane_id, serial, result, &mut outcome);
                return (repaint, outcome.actions);
            }
            PendingEndpointKind::SelectionCopy => {
                return match result {
                    Ok(crate::api::schema::ResponseResult::PaneSelection { text, .. })
                        if !text.is_empty() =>
                    {
                        let repaint = self.show_copy_feedback(std::time::Instant::now());
                        (
                            repaint,
                            vec![ClientShellAction::ClipboardWrite(text.into_bytes())],
                        )
                    }
                    Ok(crate::api::schema::ResponseResult::PaneSelection { .. }) => {
                        (false, Vec::new())
                    }
                    Ok(_) => {
                        self.set_endpoint_error("endpoint returned an unexpected selection result");
                        (true, Vec::new())
                    }
                    Err(_) => (true, Vec::new()),
                };
            }
            PendingEndpointKind::WordSelection {
                pane_id,
                absolute_row,
                generation,
            } => {
                return self.complete_word_selection_row(pane_id, absolute_row, generation, result);
            }
            PendingEndpointKind::PaneLinkActivate {
                pane_id,
                inner_rect,
                fallback_events,
            } => {
                let completed_before_release = !fallback_events.iter().any(|event| {
                    event.kind
                        == crossterm::event::MouseEventKind::Up(crossterm::event::MouseButton::Left)
                });
                let replay = (self.mode == ClientShellMode::Terminal
                    && self.overlay.is_none()
                    && self
                        .hits
                        .panes
                        .iter()
                        .any(|hit| hit.pane_id == pane_id && hit.inner_rect == inner_rect))
                .then_some(fallback_events);
                if replay.is_none() {
                    self.url_click_consumes_until_up = completed_before_release;
                }
                let replay_action = |events: Option<Vec<crossterm::event::MouseEvent>>| {
                    events
                        .map(ClientShellAction::ReplayMouse)
                        .into_iter()
                        .collect()
                };
                return match result {
                    Ok(crate::api::schema::ResponseResult::PaneLinkActivated {
                        handled: true,
                        ..
                    }) => {
                        self.url_click_consumes_until_up = completed_before_release;
                        (false, Vec::new())
                    }
                    Ok(crate::api::schema::ResponseResult::PaneLinkActivated {
                        url: Some(url),
                        handled: false,
                    }) if crate::app::actions::safe_web_url(&url).is_some() => {
                        self.url_click_consumes_until_up = completed_before_release;
                        (false, vec![ClientShellAction::OpenSafeWebUrl(url)])
                    }
                    Ok(crate::api::schema::ResponseResult::PaneLinkActivated { .. }) => {
                        (false, replay_action(replay))
                    }
                    Ok(_) => {
                        self.set_endpoint_error("endpoint returned an unexpected link result");
                        (true, replay_action(replay))
                    }
                    Err(error)
                        if matches!(
                            error.code.as_deref(),
                            Some("stale_content" | "stale_target" | "endpoint_cancelled")
                        ) =>
                    {
                        self.url_click_consumes_until_up = completed_before_release;
                        (false, Vec::new())
                    }
                    Err(_) => (true, replay_action(replay)),
                };
            }
            PendingEndpointKind::CopyMotion {
                pane_id,
                origin,
                session_generation,
            } => {
                let mut outcome = ClientShellInput::default();
                let (repaint, continue_queue) = match result {
                    Ok(crate::api::schema::ResponseResult::PaneCopyMotion {
                        pane_id: returned_pane_id,
                        cursor,
                        content_revision,
                    }) if returned_pane_id == pane_id => (
                        self.apply_copy_motion_target(
                            &pane_id,
                            origin,
                            cursor,
                            content_revision,
                            &mut outcome,
                        ),
                        true,
                    ),
                    Ok(crate::api::schema::ResponseResult::PaneCopyMotion { .. }) => (false, false),
                    Ok(_) => {
                        self.set_endpoint_error(
                            "endpoint returned an unexpected copy-motion result",
                        );
                        (true, false)
                    }
                    Err(_) => (true, false),
                };
                self.complete_copy_operation(session_generation, continue_queue, &mut outcome);
                return (repaint || outcome.repaint, outcome.actions);
            }
            PendingEndpointKind::CopySearch {
                pane_id,
                origin,
                query,
                direction,
                repeat,
                generation,
                session_generation,
            } => {
                let mut outcome = ClientShellInput::default();
                let (repaint, continue_queue) = match result {
                    Ok(crate::api::schema::ResponseResult::PaneCopySearch {
                        pane_id: returned_pane_id,
                        content_revision,
                        matches,
                        total,
                        current,
                        current_global,
                    }) if returned_pane_id == pane_id => {
                        let repaint = self.apply_copy_search_result(
                            &pane_id,
                            origin,
                            query,
                            direction,
                            repeat,
                            generation,
                            ClientCopySearchResult {
                                content_revision,
                                matches,
                                total,
                                current: current.and_then(|index| usize::try_from(index).ok()),
                                current_global,
                            },
                            &mut outcome,
                        );
                        if !repaint {
                            self.cancel_deferred_copy_after_search(generation);
                        }
                        (repaint, repaint)
                    }
                    Ok(crate::api::schema::ResponseResult::PaneCopySearch { .. }) => {
                        self.cancel_deferred_copy_after_search(generation);
                        (false, false)
                    }
                    Ok(_) => {
                        self.cancel_deferred_copy_after_search(generation);
                        self.set_endpoint_error(
                            "endpoint returned an unexpected copy-search result",
                        );
                        (true, false)
                    }
                    Err(_) => {
                        self.cancel_deferred_copy_after_search(generation);
                        (true, false)
                    }
                };
                self.complete_copy_operation(session_generation, continue_queue, &mut outcome);
                return (repaint || outcome.repaint, outcome.actions);
            }
            PendingEndpointKind::ReloadConfig => {
                let repaint = match result {
                    Ok(crate::api::schema::ResponseResult::ConfigReload { .. }) => false,
                    Ok(_) => {
                        self.set_endpoint_error(
                            "endpoint returned an unexpected config reload result",
                        );
                        true
                    }
                    Err(_) => true,
                };
                return (repaint, Vec::new());
            }
            kind @ (PendingEndpointKind::IntegrationList
            | PendingEndpointKind::IntegrationInstall) => {
                return self.handle_settings_endpoint_result(kind, result);
            }
            kind => {
                let mut outcome = ClientShellInput::default();
                let repaint = self.handle_worktree_endpoint_result(kind, result, &mut outcome);
                return (repaint || outcome.repaint, outcome.actions);
            }
        }
        let repaint = match result {
            Ok(_) => false,
            Err(error)
                if self.config.confirm_close
                    && error.code.as_deref() == Some("confirmation_required")
                    && pending.confirmation_workspace_id.is_some() =>
            {
                if let Some(workspace_id) = pending.confirmation_workspace_id {
                    self.open_confirm_close_overlay(workspace_id);
                }
                true
            }
            Err(_) => true,
        };
        (repaint, Vec::new())
    }

    pub(super) fn endpoint_method_for_action(
        &mut self,
        action: crate::input::KeybindAction,
    ) -> Option<crate::api::schema::Method> {
        use crate::api::schema::{
            Method, PaneDirection, PaneFocusDirectionParams, PaneResizeParams, PaneSplitParams,
            PaneSwapParams, PaneTarget, PaneZoomMode, PaneZoomParams, SplitDirection,
            TabCreateParams, TabMoveParams, TabTarget, WorkspaceTarget,
        };
        use crate::input::KeybindAction;

        let snapshot = self.snapshot.as_deref()?;
        let focused_workspace = snapshot.focused_workspace_id.clone()?;
        let focused_tab = snapshot.focused_tab_id.clone();
        let focused_pane = snapshot.focused_pane_id.clone();
        let agent_workspaces = super::agent_scope::visible_agent_workspace_ids(
            snapshot,
            self.focus_scope.as_ref(),
            self.snooze_state.as_ref(),
            &self.active_endpoint_id,
            self.config.top_level_agents,
        );
        let direction = |action| match action {
            KeybindAction::FocusPaneLeft
            | KeybindAction::SwapPaneLeft
            | KeybindAction::ResizePaneLeft => Some(PaneDirection::Left),
            KeybindAction::FocusPaneDown
            | KeybindAction::SwapPaneDown
            | KeybindAction::ResizePaneDown => Some(PaneDirection::Down),
            KeybindAction::FocusPaneUp
            | KeybindAction::SwapPaneUp
            | KeybindAction::ResizePaneUp => Some(PaneDirection::Up),
            KeybindAction::FocusPaneRight
            | KeybindAction::SwapPaneRight
            | KeybindAction::ResizePaneRight => Some(PaneDirection::Right),
            _ => None,
        };

        match action {
            KeybindAction::FocusAgent(index) => {
                let agents = super::agent_sidebar::ordered_agent_pane_ids_with_filter(
                    snapshot,
                    self.config.agent_panel_sort,
                    |agent| agent_workspaces.contains(&agent.workspace_id),
                );
                Some(Method::PaneFocus(PaneTarget {
                    pane_id: agents.get(index)?.clone(),
                }))
            }
            KeybindAction::PreviousAgent | KeybindAction::NextAgent => {
                let agents = super::agent_sidebar::ordered_agent_pane_ids_with_filter(
                    snapshot,
                    self.config.agent_panel_sort,
                    |agent| agent_workspaces.contains(&agent.workspace_id),
                );
                if agents.is_empty() {
                    return None;
                }
                let current = agents.iter().position(|pane_id| {
                    Some(pane_id.as_str()) == snapshot.focused_pane_id.as_deref()
                });
                let next = match (current, action) {
                    (Some(current), KeybindAction::PreviousAgent) => {
                        (current + agents.len() - 1) % agents.len()
                    }
                    (Some(current), KeybindAction::NextAgent) => (current + 1) % agents.len(),
                    (None, KeybindAction::PreviousAgent) => agents.len() - 1,
                    (None, KeybindAction::NextAgent) => 0,
                    _ => unreachable!("relative agent action"),
                };
                let pane_id = agents[next].clone();
                if !self
                    .hits
                    .agents
                    .iter()
                    .any(|(_, visible_pane_id)| visible_pane_id == &pane_id)
                {
                    self.agent_scroll = next.min(self.hits.agent_max_scroll);
                }
                Some(Method::PaneFocus(PaneTarget { pane_id }))
            }
            KeybindAction::SwitchWorkspace(index) => {
                let entries = self.navigation_workspace_entries(snapshot);
                let workspace_id = snapshot
                    .workspaces
                    .get(entries.get(index)?.index)?
                    .workspace_id
                    .clone();
                self.empty_presentation = false;
                self.reveal_workspace(&workspace_id);
                Some(Method::WorkspaceFocus(WorkspaceTarget { workspace_id }))
            }
            KeybindAction::PreviousWorkspace | KeybindAction::NextWorkspace => {
                let entries = self.navigation_workspace_entries(snapshot);
                if entries.is_empty() {
                    return None;
                }
                let current = entries
                    .iter()
                    .position(|entry| {
                        snapshot.workspaces[entry.index].workspace_id == focused_workspace
                    })
                    .unwrap_or(0);
                let delta = if action == KeybindAction::PreviousWorkspace {
                    -1
                } else {
                    1
                };
                let next = (current as isize + delta).rem_euclid(entries.len() as isize) as usize;
                let workspace_id = snapshot.workspaces[entries[next].index]
                    .workspace_id
                    .clone();
                self.empty_presentation = false;
                self.reveal_workspace(&workspace_id);
                Some(Method::WorkspaceFocus(WorkspaceTarget { workspace_id }))
            }
            KeybindAction::SwitchTab(index) => {
                let tabs = snapshot
                    .tabs
                    .iter()
                    .filter(|tab| tab.workspace_id == focused_workspace)
                    .collect::<Vec<_>>();
                Some(Method::TabFocus(TabTarget {
                    tab_id: tabs.get(index)?.tab_id.clone(),
                }))
            }
            KeybindAction::PreviousTab | KeybindAction::NextTab => {
                let tabs = snapshot
                    .tabs
                    .iter()
                    .filter(|tab| tab.workspace_id == focused_workspace)
                    .collect::<Vec<_>>();
                let focused_tab = focused_tab?;
                let current = tabs.iter().position(|tab| tab.tab_id == focused_tab)?;
                let delta = if action == KeybindAction::PreviousTab {
                    -1
                } else {
                    1
                };
                let next = (current as isize + delta).rem_euclid(tabs.len() as isize) as usize;
                Some(Method::TabFocus(TabTarget {
                    tab_id: tabs[next].tab_id.clone(),
                }))
            }
            KeybindAction::MoveTabPrevious | KeybindAction::MoveTabNext => {
                let tabs = snapshot
                    .tabs
                    .iter()
                    .filter(|tab| tab.workspace_id == focused_workspace)
                    .collect::<Vec<_>>();
                if tabs.len() <= 1 {
                    return None;
                }
                let focused_tab = focused_tab?;
                let source = tabs.iter().position(|tab| tab.tab_id == focused_tab)?;
                let insert_index = if action == KeybindAction::MoveTabNext {
                    if source + 1 >= tabs.len() {
                        0
                    } else {
                        source + 2
                    }
                } else if source == 0 {
                    tabs.len()
                } else {
                    source - 1
                };
                Some(Method::TabMove(TabMoveParams {
                    tab_id: focused_tab,
                    insert_index,
                }))
            }
            KeybindAction::NewTab if !self.config.prompt_new_tab_name => {
                Some(Method::TabCreate(TabCreateParams {
                    workspace_id: Some(focused_workspace),
                    cwd: None,
                    focus: true,
                    label: None,
                    env: Default::default(),
                }))
            }
            KeybindAction::FocusPaneLeft
            | KeybindAction::FocusPaneDown
            | KeybindAction::FocusPaneUp
            | KeybindAction::FocusPaneRight => {
                Some(Method::PaneFocusDirection(PaneFocusDirectionParams {
                    pane_id: focused_pane,
                    direction: direction(action)?,
                }))
            }
            KeybindAction::SwapPaneLeft
            | KeybindAction::SwapPaneDown
            | KeybindAction::SwapPaneUp
            | KeybindAction::SwapPaneRight => Some(Method::PaneSwap(PaneSwapParams {
                pane_id: focused_pane,
                direction: Some(direction(action)?),
                source_pane_id: None,
                target_pane_id: None,
            })),
            KeybindAction::SplitVertical | KeybindAction::SplitHorizontal => {
                Some(Method::PaneSplit(PaneSplitParams {
                    workspace_id: Some(focused_workspace),
                    target_pane_id: focused_pane,
                    direction: if action == KeybindAction::SplitVertical {
                        SplitDirection::Right
                    } else {
                        SplitDirection::Down
                    },
                    ratio: None,
                    cwd: None,
                    focus: true,
                    right_click: Default::default(),
                    env: Default::default(),
                }))
            }
            KeybindAction::CloseTab => Some(Method::TabClose(TabTarget {
                tab_id: focused_tab?,
            })),
            KeybindAction::ClosePane => Some(Method::PaneClose(PaneTarget {
                pane_id: focused_pane.clone()?,
            })),
            KeybindAction::CyclePaneNext | KeybindAction::CyclePanePrevious => {
                let focused_tab = focused_tab?;
                let panes = snapshot
                    .panes
                    .iter()
                    .filter(|pane| pane.tab_id == focused_tab)
                    .collect::<Vec<_>>();
                if panes.is_empty() {
                    return None;
                }
                let focused_pane = focused_pane?;
                let current = panes
                    .iter()
                    .position(|pane| pane.pane_id == focused_pane)
                    .unwrap_or(0);
                let next = if action == KeybindAction::CyclePanePrevious {
                    (current + panes.len() - 1) % panes.len()
                } else {
                    (current + 1) % panes.len()
                };
                Some(Method::PaneFocus(PaneTarget {
                    pane_id: panes[next].pane_id.clone(),
                }))
            }
            KeybindAction::LastPane => {
                let pane_id = self.previous_pane_id.as_ref()?;
                if Some(pane_id.as_str()) == focused_pane.as_deref()
                    || !snapshot.panes.iter().any(|pane| &pane.pane_id == pane_id)
                {
                    return None;
                }
                Some(Method::PaneFocus(PaneTarget {
                    pane_id: pane_id.clone(),
                }))
            }
            KeybindAction::Zoom => Some(Method::PaneZoom(PaneZoomParams {
                pane_id: focused_pane,
                mode: PaneZoomMode::Toggle,
            })),
            KeybindAction::EditScrollback => Some(Method::PaneEditScrollback(PaneTarget {
                pane_id: focused_pane?,
            })),
            KeybindAction::ResizePaneLeft
            | KeybindAction::ResizePaneDown
            | KeybindAction::ResizePaneUp
            | KeybindAction::ResizePaneRight => Some(Method::PaneResize(PaneResizeParams {
                pane_id: focused_pane,
                direction: direction(action)?,
                amount: None,
            })),
            _ => None,
        }
    }
}
