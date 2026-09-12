use super::*;

impl HeadlessServer {
    pub(super) fn handle_client_shell_endpoint_request(
        &mut self,
        client_id: u64,
        boot_id: String,
        mut request: Box<api::schema::Request>,
    ) -> bool {
        let Some(client) = self.clients.get(&client_id) else {
            return false;
        };
        if !matches!(client.mode, ClientConnectionMode::ClientShell) {
            self.remove_client_and_resize_if_needed(client_id);
            return true;
        }
        let request_id = request.id.clone();
        if !crate::server::client_commands::supports_client_shell_method(&request.method) {
            let message = crate::server::client_commands::error_message(
                boot_id,
                request_id,
                "unsupported_endpoint_command",
                "this method is not available through the client shell command lane",
            );
            self.send_to_client(client_id, message);
            return false;
        }
        if boot_id != self.client_shell_boot_id {
            let message = crate::server::client_commands::error_message(
                boot_id,
                request_id,
                "stale_boot",
                "endpoint command targeted an earlier server boot",
            );
            self.send_to_client(client_id, message);
            return false;
        }
        let surface_active = client.shell_surface_active;
        if let api::schema::Method::ClientShellSurfaceSet(params) = &request.method {
            let Some((changed, projection_revision)) =
                self.set_client_shell_surface_active(client_id, params.active)
            else {
                return false;
            };
            self.send_to_client(
                client_id,
                crate::server::client_commands::success_message_with_result(
                    boot_id,
                    request_id,
                    api::schema::ResponseResult::ClientShellSurfaceSet {
                        active: params.active,
                        projection_revision,
                    },
                ),
            );
            return changed;
        }
        if let api::schema::Method::WorkspaceSnoozeSubscribe(_) = &request.method {
            self.snooze_subscribers.insert(client_id);
            let state = self.snooze_state();
            self.send_to_client(
                client_id,
                crate::server::client_commands::success_message_with_result(
                    boot_id,
                    request_id,
                    api::schema::ResponseResult::WorkspaceSnooze { state },
                ),
            );
            return false;
        }
        if let api::schema::Method::SnoozeList(_) = &request.method {
            let state = self.snooze_state();
            self.send_to_client(
                client_id,
                crate::server::client_commands::success_message_with_result(
                    boot_id,
                    request_id,
                    api::schema::ResponseResult::WorkspaceSnooze { state },
                ),
            );
            return false;
        }
        if let api::schema::Method::SnoozeReset(params) = &request.method {
            if params.boot_id != self.client_shell_boot_id {
                self.send_to_client(
                    client_id,
                    crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "stale_boot",
                        "snooze reset targeted an earlier server boot",
                    ),
                );
                return false;
            }
            match self.commit_snooze_reset(params.expected_revision, params.confirmed) {
                Ok(_) => {
                    let state = self.snooze_state();
                    self.broadcast_snooze_state();
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::success_message_with_result(
                            boot_id,
                            request_id,
                            api::schema::ResponseResult::WorkspaceWake { state },
                        ),
                    );
                    return true;
                }
                Err(error) => {
                    let (code, message) = self.snooze_reset_error(error);
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::error_message(
                            boot_id, request_id, &code, message,
                        ),
                    );
                    return false;
                }
            }
        }
        if let api::schema::Method::SnoozeRecordWake(params) = &request.method {
            if params.boot_id != self.client_shell_boot_id {
                self.send_to_client(
                    client_id,
                    crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "stale_boot",
                        "snooze record wake targeted an earlier server boot",
                    ),
                );
                return false;
            }
            match self.commit_snooze_record_wake(&params.record_id, params.expected_revision) {
                Ok(_) => {
                    let state = self.snooze_state();
                    self.broadcast_snooze_state();
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::success_message_with_result(
                            boot_id,
                            request_id,
                            api::schema::ResponseResult::WorkspaceWake { state },
                        ),
                    );
                    return true;
                }
                Err(error) => {
                    let (code, message) = self.snooze_record_wake_error(error);
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::error_message(
                            boot_id, request_id, &code, message,
                        ),
                    );
                    return false;
                }
            }
        }
        if let api::schema::Method::ProjectSnooze(params) = &request.method {
            if params.boot_id != self.client_shell_boot_id {
                self.send_to_client(
                    client_id,
                    crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "stale_boot",
                        "project snooze targeted an earlier server boot",
                    ),
                );
                return false;
            }
            let Some(canonical_id) = self.app.canonical_workspace_id(&params.workspace_id) else {
                self.send_to_client(
                    client_id,
                    crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "not_found",
                        "workspace not found",
                    ),
                );
                return false;
            };
            let valid_key = self
                .app
                .state
                .workspaces
                .iter()
                .find(|workspace| workspace.id == canonical_id)
                .and_then(|workspace| workspace.worktree_space())
                .is_some_and(|space| space.key == params.project_key);
            if !valid_key {
                self.send_to_client(
                    client_id,
                    crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "stale_target",
                        "workspace no longer belongs to this project",
                    ),
                );
                return false;
            }
            match self.commit_snooze_change(|manager| {
                manager
                    .project_snooze(
                        params.project_key.clone(),
                        params.duration_seconds,
                        params.deadline_unix_ms,
                        crate::server::workspace_snooze::now_unix_ms(),
                    )
                    .map(|_| ())
            }) {
                Ok(_) => {
                    let state = self.snooze_state();
                    self.broadcast_snooze_state();
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::success_message_with_result(
                            boot_id,
                            request_id,
                            api::schema::ResponseResult::WorkspaceSnooze { state },
                        ),
                    );
                    return true;
                }
                Err(error) => {
                    let (code, message) = self.snooze_action_error(error);
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::error_message(boot_id, request_id, &code, message),
                    );
                    return false;
                }
            }
        }
        if let api::schema::Method::ProjectWake(params) = &request.method {
            if params.boot_id != self.client_shell_boot_id {
                self.send_to_client(
                    client_id,
                    crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "stale_boot",
                        "project wake targeted an earlier server boot",
                    ),
                );
                return false;
            }
            match self.commit_snooze_wake(|manager| {
                manager.project_wake(&params.project_key, params.expected_revision)
            }) {
                Ok(_) => {
                    let state = self.snooze_state();
                    self.broadcast_snooze_state();
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::success_message_with_result(
                            boot_id,
                            request_id,
                            api::schema::ResponseResult::WorkspaceWake { state },
                        ),
                    );
                    return true;
                }
                Err(SnoozeWakeCommitError::Action(
                    crate::server::workspace_snooze::WorkspaceWakeError::NotFound,
                )) => {
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::error_message(
                            boot_id,
                            request_id,
                            "not_found",
                            "project snooze record not found",
                        ),
                    );
                }
                Err(SnoozeWakeCommitError::Action(
                    crate::server::workspace_snooze::WorkspaceWakeError::StaleRevision {
                    expected,
                    current,
                    },
                )) => {
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::error_message(
                            boot_id,
                            request_id,
                            "stale_revision",
                            format!("project snooze record revision mismatch (expected {expected}, current {current})"),
                        ),
                    );
                }
                Err(SnoozeWakeCommitError::Persistence(message)) => {
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::error_message(
                            boot_id,
                            request_id,
                            "persistence_failed",
                            message,
                        ),
                    );
                }
                Err(SnoozeWakeCommitError::Action(_)) => {}
            }
            return false;
        }
        if let api::schema::Method::WorkspaceSnooze(params) = &request.method {
            if params.boot_id != self.client_shell_boot_id {
                let message = crate::server::client_commands::error_message(
                    boot_id,
                    request_id,
                    "stale_boot",
                    "workspace snooze targeted an earlier server boot",
                );
                self.send_to_client(client_id, message);
                return false;
            }
            let Some(canonical_id) = self.app.canonical_workspace_id(&params.workspace_id) else {
                let message = crate::server::client_commands::error_message(
                    boot_id,
                    request_id,
                    "not_found",
                    "workspace not found",
                );
                self.send_to_client(client_id, message);
                return false;
            };
            match self.commit_snooze_change(|manager| {
                manager
                    .snooze(
                        canonical_id,
                        params.duration_seconds,
                        params.deadline_unix_ms,
                        crate::server::workspace_snooze::now_unix_ms(),
                    )
                    .map(|_| ())
            }) {
                Ok(_) => {
                    let state = self.snooze_state();
                    self.broadcast_snooze_state();
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::success_message_with_result(
                            boot_id,
                            request_id,
                            api::schema::ResponseResult::WorkspaceSnooze { state },
                        ),
                    );
                    return true;
                }
                Err(SnoozeCommitError::Action(
                    crate::server::workspace_snooze::WorkspaceSnoozeError::InvalidDuration,
                )) => {
                    let message = crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "invalid_duration",
                        "invalid snooze duration (must be between 1 second and 30 days)",
                    );
                    self.send_to_client(client_id, message);
                    return false;
                }
                Err(SnoozeCommitError::Action(
                    crate::server::workspace_snooze::WorkspaceSnoozeError::InvalidDeadline,
                )) => {
                    let message = crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "invalid_deadline",
                        "invalid snooze deadline (must be in the future, up to 30 days)",
                    );
                    self.send_to_client(client_id, message);
                    return false;
                }
                Err(SnoozeCommitError::Persistence(message)) => {
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::error_message(
                            boot_id,
                            request_id,
                            "persistence_failed",
                            message,
                        ),
                    );
                    return false;
                }
            }
        }
        if let api::schema::Method::WorkspaceWake(params) = &request.method {
            if params.boot_id != self.client_shell_boot_id {
                let message = crate::server::client_commands::error_message(
                    boot_id,
                    request_id,
                    "stale_boot",
                    "workspace wake targeted an earlier server boot",
                );
                self.send_to_client(client_id, message);
                return false;
            }
            let canonical_target = match params.workspace_id.as_deref() {
                Some(id) => match self.app.canonical_workspace_id(id) {
                    Some(id) => Some(id),
                    None if self.workspace_snoozes.is_snoozed(id) => Some(id.to_owned()),
                    None => {
                        self.send_to_client(
                            client_id,
                            crate::server::client_commands::error_message(
                                boot_id,
                                request_id,
                                "not_found",
                                "workspace not found",
                            ),
                        );
                        return false;
                    }
                },
                None => None,
            };
            let project_key = canonical_target.as_deref().and_then(|id| {
                self.app
                    .state
                    .workspaces
                    .iter()
                    .find(|workspace| workspace.id == id)
                    .and_then(|workspace| workspace.worktree_space())
                    .map(|space| space.key.clone())
            });
            let wake_result = if params.workspace_id.is_none() {
                self.commit_snooze_wake_all(params.expected_revision, params.confirmed)
            } else {
                self.commit_snooze_wake(|manager| {
                    manager.wake_workspace(
                        canonical_target.as_deref(),
                        params.expected_revision,
                        params.confirmed,
                        project_key.as_deref(),
                    )
                    .map(|_| ())
                })
            };
            match wake_result {
                Ok(_) => {
                    let state = self.snooze_state();
                    self.broadcast_snooze_state();
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::success_message_with_result(
                            boot_id,
                            request_id,
                            api::schema::ResponseResult::WorkspaceWake { state },
                        ),
                    );
                    return true;
                }
                Err(SnoozeWakeCommitError::Action(
                    crate::server::workspace_snooze::WorkspaceWakeError::NotFound,
                )) => {
                    let message = crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "not_found",
                        "workspace snooze record not found",
                    );
                    self.send_to_client(client_id, message);
                    return false;
                }
                Err(SnoozeWakeCommitError::Action(
                    crate::server::workspace_snooze::WorkspaceWakeError::CoveredByProject,
                )) => {
                    let message = crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "covered_by_project",
                        "still snoozed by project",
                    );
                    self.send_to_client(client_id, message);
                    return false;
                }
                Err(SnoozeWakeCommitError::Action(
                    crate::server::workspace_snooze::WorkspaceWakeError::StaleRevision {
                    expected,
                    current,
                    },
                )) => {
                    let message = crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "stale_revision",
                        format!(
                            "workspace snooze record revision mismatch (expected {expected}, current {current})"
                        ),
                    );
                    self.send_to_client(client_id, message);
                    return false;
                }
                Err(SnoozeWakeCommitError::Action(
                    crate::server::workspace_snooze::WorkspaceWakeError::UnconfirmedWakeAll,
                )) => {
                    let message = crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "unconfirmed",
                        "wake all requires confirmed: true",
                    );
                    self.send_to_client(client_id, message);
                    return false;
                }
                Err(SnoozeWakeCommitError::Action(
                    crate::server::workspace_snooze::WorkspaceWakeError::StaleBoot {
                    expected,
                    current,
                    },
                )) => {
                    let message = crate::server::client_commands::error_message(
                        boot_id,
                        request_id,
                        "stale_boot",
                        format!(
                            "workspace snooze boot mismatch (expected {expected}, current {current})"
                        ),
                    );
                    self.send_to_client(client_id, message);
                    return false;
                }
                Err(SnoozeWakeCommitError::Persistence(message)) => {
                    self.send_to_client(
                        client_id,
                        crate::server::client_commands::error_message(
                            boot_id,
                            request_id,
                            "persistence_failed",
                            message,
                        ),
                    );
                    return false;
                }
            }
        }
        if client.shell_endpoint_command_in_flight {
            let message = crate::server::client_commands::error_message(
                boot_id,
                request_id,
                "endpoint_busy",
                "this endpoint is still processing another command",
            );
            self.send_to_client(client_id, message);
            return false;
        }
        if !surface_active {
            let message = crate::server::client_commands::error_message(
                boot_id,
                request_id,
                "surface_inactive",
                "this method requires an active client shell surface",
            );
            self.send_to_client(client_id, message);
            return false;
        }

        let api_request_id = format!(
            "endpoint:{}:{client_id}:{request_id}",
            self.client_shell_boot_id
        );
        request.id = api_request_id.clone();
        let (respond_to, response_rx) = std::sync::mpsc::channel();
        if let Err(err) = crate::server::client_commands::spawn_response_waiter(
            client_id,
            boot_id.clone(),
            request_id.clone(),
            response_rx,
            self.server_event_tx.clone(),
        ) {
            let message = crate::server::client_commands::error_message(
                boot_id,
                request_id,
                "server_unavailable",
                format!("failed to start endpoint response bridge: {err}"),
            );
            self.send_to_client(client_id, message);
            return false;
        }
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.shell_endpoint_command_in_flight = true;
            // A later source restore has a new projection revision. Keep this request's lease
            // so a delayed worktree response cannot focus a pane after endpoint switching.
            client.shell_endpoint_command_surface_revision = Some(client.shell_projection_revision);
            let deferred_worktree = matches!(
                &request.method,
                api::schema::Method::WorktreeCreate(_) | api::schema::Method::WorktreeRemove(_)
            );
            let deferred_navigation = matches!(
                &request.method,
                api::schema::Method::WorktreeCreate(params) if params.focus
            );
            client.shell_deferred_navigation_request_id =
                deferred_worktree.then(|| api_request_id.clone());
            client.shell_deferred_navigation_response = deferred_navigation.then(Vec::new);
        }
        let foreground_changed = self.promote_client_to_foreground(client_id);
        foreground_changed
            | self.handle_client_shell_api_request(
                client_id,
                api::ApiRequestMessage {
                    request: *request,
                    respond_to,
                    response_write_complete: None,
                    stream_active: None,
                },
            )
    }
}
