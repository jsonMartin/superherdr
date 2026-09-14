use super::*;

impl ClientContextMenuOverlay {
    pub(super) fn items(&self) -> Vec<ClientContextMenuItem> {
        use ClientContextMenuAction as Action;

        let item = |label: &str, action| ClientContextMenuItem {
            label: label.to_owned(),
            action,
        };
        match &self.target {
            ClientContextMenuTarget::Workspace {
                worktree_key,
                is_active_endpoint,
                is_git,
                is_linked_worktree,
                has_worktree_children,
                collapsed,
                can_clear_focus,
                workspace_record_revision,
                project_record_revision,
                ..
            } => {
                let mut items = Vec::new();
                if *is_active_endpoint {
                    items.push(item("Rename", Action::Rename));
                    if *has_worktree_children {
                        items.push(item("Close group", Action::Close));
                    } else {
                        items.push(item("Close", Action::Close));
                    }
                    if *is_git && !*is_linked_worktree {
                        items.push(item("New worktree", Action::NewWorktree));
                        items.push(item("Open worktree...", Action::OpenWorktree));
                    } else if *is_linked_worktree {
                        items.push(item("Delete worktree checkout...", Action::RemoveWorktree));
                    }
                    if *has_worktree_children {
                        items.push(item(
                            if *collapsed { "Expand" } else { "Collapse" },
                            Action::ToggleGroup,
                        ));
                    }
                }

                items.push(item("Snooze workspace", Action::Snooze30Minutes));
                if worktree_key.is_some() && !*is_linked_worktree {
                    items.push(item("Snooze project", Action::SnoozeProject30Minutes));
                }
                if workspace_record_revision.is_some() {
                    items.push(item("Wake workspace", Action::WakeWorkspace));
                }
                if project_record_revision.is_some() {
                    items.push(item("Wake project", Action::WakeProject));
                }

                if worktree_key.is_some() {
                    items.push(item("Focus project", Action::FocusProject));
                } else {
                    items.push(item("Focus project", Action::FocusWorkspace));
                }

                if *can_clear_focus {
                    items.push(item("Clear local focus", Action::ClearFocus));
                }

                items
            }
            ClientContextMenuTarget::Tab { .. } => vec![
                item("New tab", Action::NewTab),
                item("Rename", Action::Rename),
                item("Close", Action::Close),
            ],
            ClientContextMenuTarget::Pane {
                source_pane_id,
                has_manual_label,
                right_click_passthrough,
                ..
            } => {
                let mut items = vec![item("Rename pane", Action::RenamePane)];
                if *has_manual_label {
                    items.push(item("Clear pane name", Action::ClearPaneName));
                }
                if source_pane_id.is_some() {
                    items.push(item("Swap with focused pane", Action::SwapWithFocusedPane));
                }
                items.extend([
                    item("Split right", Action::SplitRight),
                    item("Split down", Action::SplitDown),
                    item("Zoom", Action::Zoom),
                    item(
                        if *right_click_passthrough {
                            "Use Superherdr right-click menu"
                        } else {
                            "Send right-clicks to pane"
                        },
                        Action::ToggleRightClickPassthrough,
                    ),
                    item("Close pane", Action::ClosePane),
                ]);
                items
            }
        }
    }
}

impl ClientShellState {
    pub(super) fn activate_launcher_action(
        &mut self,
        action: crate::input::KeybindAction,
        outcome: &mut ClientShellInput,
    ) -> bool {
        use crate::input::KeybindAction;

        if action == KeybindAction::ResetFocusSnooze {
            let Some(boot_id) = self
                .snapshot
                .as_ref()
                .map(|snapshot| snapshot.boot_id.clone())
            else {
                self.endpoint_error = Some("No server endpoint is available.".to_owned());
                outcome.repaint = true;
                return true;
            };
            let Some(expected_revision) = self
                .snooze_state
                .as_ref()
                .filter(|state| state.boot_id == boot_id)
                .map(|state| state.revision)
            else {
                self.endpoint_error = Some(
                    "Current server snooze state is unavailable; refresh and try again.".to_owned(),
                );
                outcome.repaint = true;
                return true;
            };
            let count = super::recovery_bar::snooze_count(self);
            self.open_reset_confirmation(
                self.active_endpoint_id.clone(),
                boot_id,
                expected_revision,
                count,
            );
            outcome.repaint = true;
            return true;
        }
        if action == KeybindAction::ClearProjectFocus {
            outcome.actions.extend(self.clear_focus_scope());
            outcome.repaint = true;
            return true;
        }
        if action == KeybindAction::ShowSnoozed {
            self.open_snooze_recovery(0, 0);
            if self.overlay.is_none() {
                self.endpoint_error = Some("No snoozed items are available.".to_owned());
            }
            outcome.repaint = true;
            return true;
        }
        let target = self.launcher_workspace_target();
        let Some((endpoint_id, boot_id, workspace_id, worktree_key)) = target else {
            self.endpoint_error = Some("No eligible workspace is selected.".to_owned());
            outcome.repaint = true;
            return true;
        };
        match action {
            KeybindAction::FocusProject => {
                let scope = match worktree_key {
                    Some(worktree_key) => ClientFocusScope::Worktree {
                        endpoint_id: endpoint_id.clone(),
                        boot_id,
                        worktree_key,
                    },
                    None => ClientFocusScope::StandaloneWorkspace {
                        endpoint_id: endpoint_id.clone(),
                        boot_id,
                        workspace_id: workspace_id.clone(),
                    },
                };
                outcome
                    .actions
                    .extend(self.set_focus_scope_with_preferred_target(
                        Some(scope),
                        Some((endpoint_id, workspace_id)),
                    ));
            }
            KeybindAction::SnoozeWorkspace => {
                self.open_snooze_overlay(endpoint_id, boot_id, workspace_id, None);
            }
            KeybindAction::SnoozeProject => {
                let Some(worktree_key) = worktree_key else {
                    self.endpoint_error = Some("Selected workspace has no project.".to_owned());
                    outcome.repaint = true;
                    return true;
                };
                self.open_snooze_overlay(endpoint_id, boot_id, workspace_id, Some(worktree_key));
            }
            KeybindAction::ClearProjectFocus => unreachable!(),
            _ => return false,
        }
        outcome.repaint = true;
        true
    }

    fn launcher_workspace_target(
        &self,
    ) -> Option<(ClientEndpointId, String, String, Option<String>)> {
        let snapshot = self.snapshot.as_deref()?;
        let workspace_id = if self.mode == ClientShellMode::Navigate {
            self.navigate_workspace_id
                .as_deref()
                .filter(|selected| {
                    self.navigation_workspace_entries(snapshot)
                        .iter()
                        .any(|entry| {
                            snapshot.workspaces[entry.index].workspace_id.as_str() == *selected
                        })
                })
                .map(str::to_owned)
                .or_else(|| snapshot.focused_workspace_id.clone())?
        } else {
            snapshot.focused_workspace_id.clone()?
        };
        let workspace = snapshot.workspaces.iter().find(|workspace| {
            workspace.workspace_id == workspace_id && self.is_workspace_visible(workspace)
        })?;
        Some((
            self.active_endpoint_id.clone(),
            snapshot.boot_id.clone(),
            workspace.workspace_id.clone(),
            workspace
                .worktree
                .as_ref()
                .map(|worktree| worktree.key.clone()),
        ))
    }

    pub(super) fn submit_wake_shared_snoozes(&mut self, outcome: &mut ClientShellInput) {
        let Some(ClientShellOverlay::ConfirmWakeSharedSnoozes(confirm)) = self.overlay.take()
        else {
            return;
        };
        match confirm.purpose {
            ClientWakeConfirmationPurpose::WakeSharedSnoozes => {
                if !self.endpoint_is_online(&confirm.endpoint_id)
                    || self.endpoint_boot_id(&confirm.endpoint_id) != Some(confirm.boot_id.as_str())
                {
                    return;
                }
                let endpoint_id = confirm.endpoint_id.clone();
                self.push_endpoint_method_for(
                    &endpoint_id,
                    confirm.boot_id.clone(),
                    crate::api::schema::Method::WorkspaceWake(
                        crate::api::schema::WorkspaceWakeParams {
                            workspace_id: None,
                            boot_id: confirm.boot_id,
                            expected_revision: confirm.expected_revision,
                            confirmed: true,
                        },
                    ),
                    PendingEndpointKind::Generic,
                    outcome,
                );
            }
            ClientWakeConfirmationPurpose::ResetFocusSnooze => {
                let endpoint_id = confirm.endpoint_id.clone();
                if !self.endpoint_is_online(&endpoint_id)
                    || self.endpoint_boot_id(&endpoint_id) != Some(confirm.boot_id.as_str())
                {
                    return;
                }
                self.push_endpoint_method_for(
                    &endpoint_id,
                    confirm.boot_id.clone(),
                    crate::api::schema::Method::SnoozeReset(
                        crate::api::schema::SnoozeResetParams {
                            boot_id: confirm.boot_id,
                            expected_revision: confirm.expected_revision,
                            confirmed: true,
                        },
                    ),
                    PendingEndpointKind::SnoozeReset {
                        endpoint_id: endpoint_id.clone(),
                        captured_focus: self.focus_scope.clone(),
                    },
                    outcome,
                );
            }
        }
    }

    pub(super) fn open_reset_confirmation(
        &mut self,
        endpoint_id: ClientEndpointId,
        boot_id: String,
        expected_revision: u64,
        count: usize,
    ) {
        let endpoint_label = self.endpoint_label(&endpoint_id).to_owned();
        self.overlay = Some(ClientShellOverlay::ConfirmWakeSharedSnoozes(
            ClientConfirmWakeSharedSnoozesOverlay {
                endpoint_id,
                boot_id,
                expected_revision,
                count,
                purpose: ClientWakeConfirmationPurpose::ResetFocusSnooze,
                endpoint_label,
            },
        ));
    }

    pub(super) fn open_snooze_recovery(&mut self, x: u16, y: u16) {
        let _ = (x, y);
        let visible = |endpoint: &&ClientShellEndpoint| {
            endpoint.snooze_state.as_ref().is_some_and(|state| {
                !state.records.is_empty()
                    || !state.project_records.is_empty()
                    || state.persistence.as_ref().is_some_and(|persistence| {
                        !persistence.records.is_empty() || persistence.notice.is_some()
                    })
            })
        };
        let endpoint_id = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == self.active_endpoint_id && visible(endpoint))
            .or_else(|| self.endpoints.iter().find(visible))
            .map(|endpoint| endpoint.endpoint_id.clone());
        if let Some(endpoint_id) = endpoint_id {
            self.open_snooze_management_for_endpoint(endpoint_id, None);
        }
    }

    // Inherited from Herdr; Superherdr routes callers elsewhere. Kept to simplify upstream merges.
    #[allow(dead_code)]
    pub(super) fn open_workspace_context_menu(&mut self, workspace_id: String, x: u16, y: u16) {
        self.open_endpoint_workspace_context_menu(
            self.active_endpoint_id.clone(),
            workspace_id,
            x,
            y,
        );
    }

    pub(super) fn open_endpoint_workspace_context_menu(
        &mut self,
        endpoint_id: ClientEndpointId,
        workspace_id: String,
        x: u16,
        y: u16,
    ) {
        let is_active_endpoint = endpoint_id == self.active_endpoint_id;
        let Some(snapshot) = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == endpoint_id)
            .and_then(|endpoint| endpoint.snapshot.as_deref())
        else {
            return;
        };
        let Some(workspace) = snapshot
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_id == workspace_id)
        else {
            return;
        };
        let worktree = workspace.worktree.as_ref();
        let has_worktree_children = worktree.is_some_and(|worktree| {
            !worktree.is_linked_worktree
                && snapshot
                    .workspaces
                    .iter()
                    .filter(|candidate| {
                        candidate
                            .worktree
                            .as_ref()
                            .is_some_and(|candidate| candidate.key == worktree.key)
                    })
                    .count()
                    >= 2
        });
        let collapsed =
            worktree.is_some_and(|worktree| self.collapsed_groups.contains(&worktree.key));
        let can_clear_focus = self.focus_scope.is_some();
        let snooze_state = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == endpoint_id)
            .and_then(|endpoint| endpoint.snooze_state.as_ref());
        let workspace_record_revision = snooze_state
            .and_then(|state| {
                (state.boot_id == snapshot.boot_id).then(|| {
                    state
                        .records
                        .iter()
                        .find(|record| record.workspace_id == workspace_id)
                        .map(|record| record.revision)
                })
            })
            .flatten();
        let project_record_revision = worktree.and_then(|worktree| {
            snooze_state
                .and_then(|state| {
                    (state.boot_id == snapshot.boot_id).then(|| {
                        state
                            .project_records
                            .iter()
                            .find(|record| record.project_key == worktree.key)
                            .map(|record| record.revision)
                    })
                })
                .flatten()
        });
        self.overlay = Some(ClientShellOverlay::ContextMenu(ClientContextMenuOverlay {
            target: ClientContextMenuTarget::Workspace {
                endpoint_id,
                boot_id: snapshot.boot_id.clone(),
                workspace_id,
                is_active_endpoint,
                worktree_key: worktree.map(|w| w.key.clone()),
                expected_revision: snooze_state.map_or(0, |state| state.revision),
                workspace_record_revision,
                project_record_revision,
                is_git: worktree.is_some() || workspace.branch.is_some(),
                is_linked_worktree: worktree.is_some_and(|worktree| worktree.is_linked_worktree),
                has_worktree_children,
                collapsed,
                can_clear_focus,
            },
            x,
            y,
            highlighted: 0,
        }));
    }

    pub(super) fn open_tab_context_menu(&mut self, tab_id: String, x: u16, y: u16) {
        let Some(tab) = self
            .snapshot
            .as_deref()
            .and_then(|snapshot| snapshot.tabs.iter().find(|tab| tab.tab_id == tab_id))
        else {
            return;
        };
        self.overlay = Some(ClientShellOverlay::ContextMenu(ClientContextMenuOverlay {
            target: ClientContextMenuTarget::Tab {
                tab_id,
                workspace_id: tab.workspace_id.clone(),
            },
            x,
            y,
            highlighted: 0,
        }));
    }

    pub(super) fn open_pane_context_menu(&mut self, pane_id: String, x: u16, y: u16) {
        let Some(snapshot) = self.snapshot.as_deref() else {
            return;
        };
        let Some(pane) = snapshot.panes.iter().find(|pane| pane.pane_id == pane_id) else {
            return;
        };
        let source_pane_id = snapshot
            .focused_pane_id
            .clone()
            .filter(|focused| focused != &pane_id);
        self.overlay = Some(ClientShellOverlay::ContextMenu(ClientContextMenuOverlay {
            target: ClientContextMenuTarget::Pane {
                pane_id,
                workspace_id: pane.workspace_id.clone(),
                source_pane_id,
                has_manual_label: pane.label.is_some(),
                right_click_passthrough: pane.right_click_passthrough,
            },
            x,
            y,
            highlighted: 0,
        }));
    }

    pub(super) fn move_context_menu_selection(&mut self, delta: isize) {
        let Some(ClientShellOverlay::ContextMenu(menu)) = self.overlay.as_mut() else {
            return;
        };
        let item_count = menu.items().len();
        if item_count == 0 {
            return;
        }
        menu.highlighted = (menu.highlighted as isize + delta)
            .clamp(0, item_count.saturating_sub(1) as isize) as usize;
    }

    pub(super) fn activate_context_menu_item(
        &mut self,
        index: usize,
        outcome: &mut ClientShellInput,
    ) {
        let Some(ClientShellOverlay::ContextMenu(menu)) = self.overlay.take() else {
            return;
        };
        let Some(action) = menu.items().get(index).map(|item| item.action) else {
            outcome.repaint = true;
            return;
        };
        match menu.target {
            ClientContextMenuTarget::Workspace {
                endpoint_id,
                boot_id,
                workspace_id,
                worktree_key,
                expected_revision,
                workspace_record_revision,
                project_record_revision,
                ..
            } => self.activate_workspace_context_action(
                endpoint_id,
                boot_id,
                workspace_id,
                worktree_key,
                expected_revision,
                workspace_record_revision,
                project_record_revision,
                action,
                outcome,
            ),
            ClientContextMenuTarget::Tab {
                tab_id,
                workspace_id,
            } => self.activate_tab_context_action(tab_id, workspace_id, action, outcome),
            ClientContextMenuTarget::Pane {
                pane_id,
                workspace_id,
                source_pane_id,
                right_click_passthrough,
                ..
            } => self.activate_pane_context_action(
                pane_id,
                workspace_id,
                source_pane_id,
                right_click_passthrough,
                action,
                outcome,
            ),
        }
        outcome.repaint = true;
    }

    fn activate_workspace_context_action(
        &mut self,
        endpoint_id: ClientEndpointId,
        boot_id: String,
        workspace_id: String,
        worktree_key: Option<String>,
        expected_revision: u64,
        workspace_record_revision: Option<u64>,
        project_record_revision: Option<u64>,
        action: ClientContextMenuAction,
        outcome: &mut ClientShellInput,
    ) {
        use crate::input::KeybindAction;

        let is_shared = matches!(
            action,
            ClientContextMenuAction::Snooze30Minutes
                | ClientContextMenuAction::SnoozeProject30Minutes
                | ClientContextMenuAction::WakeWorkspace
                | ClientContextMenuAction::WakeProject
        );
        let is_focus = matches!(
            action,
            ClientContextMenuAction::FocusProject | ClientContextMenuAction::FocusWorkspace
        );
        let is_recovery = action == ClientContextMenuAction::ShowSnoozedRecords;
        let is_valid = if action == ClientContextMenuAction::ClearFocus {
            true
        } else if action == ClientContextMenuAction::WakeSharedSnoozes {
            self.endpoint_is_online(&endpoint_id)
                && self.endpoint_boot_id(&endpoint_id) == Some(boot_id.as_str())
        } else {
            self.endpoint_is_online(&endpoint_id)
                && self
                    .endpoints
                    .iter()
                    .find(|endpoint| endpoint.endpoint_id == endpoint_id)
                    .and_then(|endpoint| endpoint.snapshot.as_deref())
                    .is_some_and(|snapshot| {
                        let Some(workspace) = snapshot
                            .workspaces
                            .iter()
                            .find(|workspace| workspace.workspace_id == workspace_id)
                        else {
                            return false;
                        };
                        snapshot.boot_id == boot_id
                            && (is_shared
                                || is_focus
                                || is_recovery
                                || endpoint_id == self.active_endpoint_id)
                            && (!matches!(
                                action,
                                ClientContextMenuAction::FocusProject
                                    | ClientContextMenuAction::SnoozeProject30Minutes
                                    | ClientContextMenuAction::WakeProject
                            ) || worktree_key.as_deref().is_some_and(|key| {
                                workspace
                                    .worktree
                                    .as_ref()
                                    .is_some_and(|worktree| worktree.key == key)
                            }))
                    })
        };
        if !is_valid {
            return;
        }

        match action {
            ClientContextMenuAction::Snooze30Minutes => {
                self.open_snooze_overlay(endpoint_id, boot_id, workspace_id, None);
            }
            ClientContextMenuAction::SnoozeProject30Minutes => {
                let Some(project_key) = worktree_key else {
                    return;
                };
                self.open_snooze_overlay(endpoint_id, boot_id, workspace_id, Some(project_key));
            }
            ClientContextMenuAction::ShowSnoozedRecords => {
                self.open_snooze_management_for_endpoint(endpoint_id, None);
            }
            ClientContextMenuAction::WakeWorkspace => {
                let Some(record_revision) = workspace_record_revision else {
                    return;
                };
                self.push_endpoint_method_for(
                    &endpoint_id,
                    boot_id.clone(),
                    crate::api::schema::Method::WorkspaceWake(
                        crate::api::schema::WorkspaceWakeParams {
                            workspace_id: Some(workspace_id),
                            boot_id: boot_id.clone(),
                            expected_revision: record_revision,
                            confirmed: false,
                        },
                    ),
                    PendingEndpointKind::Generic,
                    outcome,
                );
            }
            ClientContextMenuAction::WakeProject => {
                let (Some(project_key), Some(record_revision)) =
                    (worktree_key, project_record_revision)
                else {
                    return;
                };
                self.push_endpoint_method_for(
                    &endpoint_id,
                    boot_id.clone(),
                    crate::api::schema::Method::ProjectWake(
                        crate::api::schema::ProjectWakeParams {
                            project_key,
                            boot_id: boot_id.clone(),
                            expected_revision: record_revision,
                        },
                    ),
                    PendingEndpointKind::Generic,
                    outcome,
                );
            }
            ClientContextMenuAction::WakeSharedSnoozes => {
                let count = super::recovery_bar::snooze_count_for(self, &endpoint_id);
                self.overlay = Some(ClientShellOverlay::ConfirmWakeSharedSnoozes(
                    ClientConfirmWakeSharedSnoozesOverlay {
                        endpoint_id: endpoint_id.clone(),
                        boot_id: boot_id.clone(),
                        expected_revision,
                        purpose: ClientWakeConfirmationPurpose::WakeSharedSnoozes,
                        endpoint_label: self.endpoint_label(&endpoint_id).to_owned(),
                        count,
                    },
                ));
                outcome.repaint = true;
            }
            ClientContextMenuAction::FocusProject => {
                if let Some(key) = worktree_key {
                    // The shared transition applies the scope with the clicked workspace as the
                    // preferred target, emitting at most one selection action instead of a
                    // reconcile fallback followed by the explicit focus (the double-selection
                    // flicker). Snoozed targets fall back to an eligible member or the empty
                    // view so hidden work is never revealed.
                    let scope = ClientFocusScope::Worktree {
                        endpoint_id: endpoint_id.clone(),
                        boot_id,
                        worktree_key: key,
                    };
                    outcome
                        .actions
                        .extend(self.set_focus_scope_with_preferred_target(
                            Some(scope),
                            Some((endpoint_id, workspace_id)),
                        ));
                }
            }
            ClientContextMenuAction::FocusWorkspace => {
                let scope = ClientFocusScope::StandaloneWorkspace {
                    endpoint_id: endpoint_id.clone(),
                    boot_id,
                    workspace_id: workspace_id.clone(),
                };
                outcome
                    .actions
                    .extend(self.set_focus_scope_with_preferred_target(
                        Some(scope),
                        Some((endpoint_id, workspace_id)),
                    ));
            }
            ClientContextMenuAction::ClearFocus => {
                let actions = self.clear_focus_scope();
                outcome.actions.extend(actions);
            }
            ClientContextMenuAction::Rename => {
                let label = self
                    .snapshot
                    .as_deref()
                    .and_then(|snapshot| {
                        snapshot
                            .workspaces
                            .iter()
                            .find(|workspace| workspace.workspace_id == workspace_id)
                    })
                    .map(|workspace| workspace.label.clone());
                if let Some(label) = label {
                    self.overlay = Some(ClientShellOverlay::Rename(ClientRenameOverlay {
                        title: "rename workspace",
                        input: label,
                        replace_on_type: false,
                        target: ClientRenameTarget::Workspace { workspace_id },
                    }));
                }
            }
            ClientContextMenuAction::Close => {
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
            ClientContextMenuAction::NewWorktree => {
                self.begin_worktree_action_for(KeybindAction::NewWorktree, workspace_id, outcome)
            }
            ClientContextMenuAction::OpenWorktree => {
                self.begin_worktree_action_for(KeybindAction::OpenWorktree, workspace_id, outcome)
            }
            ClientContextMenuAction::RemoveWorktree => {
                self.begin_worktree_action_for(KeybindAction::RemoveWorktree, workspace_id, outcome)
            }
            ClientContextMenuAction::ToggleGroup => {
                let key = self.snapshot.as_deref().and_then(|snapshot| {
                    snapshot
                        .workspaces
                        .iter()
                        .find(|workspace| workspace.workspace_id == workspace_id)
                        .and_then(|workspace| workspace.worktree.as_ref())
                        .map(|worktree| worktree.key.clone())
                });
                if let Some(key) = key {
                    if !self.collapsed_groups.remove(&key) {
                        self.collapsed_groups.insert(key);
                    }
                    self.persist_chrome_preferences(outcome);
                }
            }
            _ => {}
        }
    }

    pub(super) fn open_snooze_overlay(
        &mut self,
        endpoint_id: ClientEndpointId,
        boot_id: String,
        workspace_id: String,
        project_key: Option<String>,
    ) {
        if !self.endpoint_is_online(&endpoint_id) {
            return;
        }
        let Some(snapshot) = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == endpoint_id)
            .and_then(|endpoint| endpoint.snapshot.as_deref())
        else {
            return;
        };
        let Some(workspace) = snapshot
            .workspaces
            .iter()
            .find(|workspace| workspace.workspace_id == workspace_id)
        else {
            return;
        };
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok());
        let Some(now_ms) = now_ms else {
            return;
        };
        let choices = match super::snooze_presets::snooze_choices(now_ms) {
            Ok(choices) => choices,
            Err(error) => {
                self.endpoint_error = Some(format!("Snooze unavailable: {error}"));
                return;
            }
        };
        if choices.is_empty() {
            return;
        }
        self.overlay = Some(ClientShellOverlay::Snooze(ClientSnoozeOverlay {
            endpoint_id,
            boot_id,
            workspace_id,
            target_label: workspace.label.clone(),
            project_key,
            project_label: workspace
                .worktree
                .as_ref()
                .map(|worktree| worktree.label.clone()),
            choices,
            selected: 0,
        }));
    }

    pub(super) fn move_snooze_selection(&mut self, delta: isize) {
        let Some(ClientShellOverlay::Snooze(snooze)) = self.overlay.as_mut() else {
            return;
        };
        let Some(last) = snooze.choices.len().checked_sub(1) else {
            return;
        };
        snooze.selected = (snooze.selected as isize + delta).clamp(0, last as isize) as usize;
    }

    fn snooze_target_is_current(&self, snooze: &ClientSnoozeOverlay) -> bool {
        self.endpoint_is_online(&snooze.endpoint_id)
            && self
                .endpoints
                .iter()
                .find(|endpoint| endpoint.endpoint_id == snooze.endpoint_id)
                .and_then(|endpoint| endpoint.snapshot.as_deref())
                .is_some_and(|snapshot| {
                    snapshot.boot_id == snooze.boot_id
                        && snapshot.workspaces.iter().any(|workspace| {
                            workspace.workspace_id == snooze.workspace_id
                                && snooze.project_key.as_deref().is_none_or(|project_key| {
                                    workspace
                                        .worktree
                                        .as_ref()
                                        .is_some_and(|worktree| worktree.key == project_key)
                                })
                        })
                })
    }

    pub(super) fn submit_snooze_overlay(&mut self, outcome: &mut ClientShellInput) {
        let Some(ClientShellOverlay::Snooze(snooze)) = self.overlay.take() else {
            return;
        };
        let endpoint_id = snooze.endpoint_id.clone();
        let boot_id = snooze.boot_id.clone();
        let workspace_id = snooze.workspace_id.clone();
        let project_key = snooze.project_key.clone();
        let Some(choice) = snooze.choices.get(snooze.selected) else {
            return;
        };
        if !self.snooze_target_is_current(&snooze) {
            self.endpoint_error = Some("Snooze target changed. Open the menu again.".to_owned());
            return;
        }
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| i64::try_from(duration.as_millis()).ok());
        if now_ms.is_none_or(|now| choice.deadline_unix_ms <= now) {
            self.endpoint_error = Some("Snooze preview expired; choose again".to_owned());
            self.open_snooze_overlay(endpoint_id, boot_id, workspace_id, project_key);
            outcome.repaint = true;
            return;
        }
        let method = match project_key {
            Some(project_key) => {
                crate::api::schema::Method::ProjectSnooze(crate::api::schema::ProjectSnoozeParams {
                    workspace_id,
                    project_key,
                    boot_id: boot_id.clone(),
                    duration_seconds: None,
                    deadline_unix_ms: Some(choice.deadline_unix_ms),
                })
            }
            None => crate::api::schema::Method::WorkspaceSnooze(
                crate::api::schema::WorkspaceSnoozeParams {
                    workspace_id,
                    boot_id: boot_id.clone(),
                    duration_seconds: None,
                    deadline_unix_ms: Some(choice.deadline_unix_ms),
                },
            ),
        };
        self.push_endpoint_method_for(
            &endpoint_id,
            boot_id,
            method,
            PendingEndpointKind::Generic,
            outcome,
        );
    }

    fn activate_tab_context_action(
        &mut self,
        tab_id: String,
        workspace_id: String,
        action: ClientContextMenuAction,
        outcome: &mut ClientShellInput,
    ) {
        use crate::api::schema::{Method, TabTarget};

        self.push_endpoint_method(
            Method::TabFocus(TabTarget {
                tab_id: tab_id.clone(),
            }),
            outcome,
        );
        match action {
            ClientContextMenuAction::NewTab => {
                if self.config.prompt_new_tab_name {
                    let default_name = (self
                        .snapshot
                        .as_deref()
                        .map(|snapshot| {
                            snapshot
                                .tabs
                                .iter()
                                .filter(|tab| tab.workspace_id == workspace_id)
                                .count()
                        })
                        .unwrap_or(0)
                        + 1)
                    .to_string();
                    self.overlay = Some(ClientShellOverlay::Rename(ClientRenameOverlay {
                        title: "new tab",
                        input: default_name.clone(),
                        replace_on_type: true,
                        target: ClientRenameTarget::NewTab {
                            workspace_id,
                            default_name,
                        },
                    }));
                } else {
                    self.push_endpoint_method(
                        Method::TabCreate(crate::api::schema::TabCreateParams {
                            workspace_id: Some(workspace_id),
                            cwd: None,
                            focus: true,
                            label: None,
                            env: Default::default(),
                        }),
                        outcome,
                    );
                }
            }
            ClientContextMenuAction::Rename => {
                let tab = self
                    .snapshot
                    .as_deref()
                    .and_then(|snapshot| snapshot.tabs.iter().find(|tab| tab.tab_id == tab_id));
                if let Some(tab) = tab {
                    self.overlay = Some(ClientShellOverlay::Rename(ClientRenameOverlay {
                        title: "rename tab",
                        input: tab.label.clone(),
                        replace_on_type: false,
                        target: ClientRenameTarget::Tab {
                            tab_id,
                            auto_name: !tab.custom_label,
                            original_name: tab.label.clone(),
                        },
                    }));
                }
            }
            ClientContextMenuAction::Close => {
                self.push_endpoint_method(Method::TabClose(TabTarget { tab_id }), outcome);
            }
            _ => {}
        }
    }

    fn activate_pane_context_action(
        &mut self,
        pane_id: String,
        workspace_id: String,
        source_pane_id: Option<String>,
        right_click_passthrough: bool,
        action: ClientContextMenuAction,
        outcome: &mut ClientShellInput,
    ) {
        use crate::api::schema::{
            Method, PaneInputSetParams, PaneRenameParams, PaneRightClickTarget, PaneSplitParams,
            PaneSwapParams, PaneTarget, PaneZoomMode, PaneZoomParams, SplitDirection,
        };

        match action {
            ClientContextMenuAction::RenamePane => {
                let label = self.snapshot.as_deref().and_then(|snapshot| {
                    snapshot
                        .panes
                        .iter()
                        .find(|pane| pane.pane_id == pane_id)
                        .and_then(|pane| pane.label.clone())
                });
                self.overlay = Some(ClientShellOverlay::Rename(ClientRenameOverlay {
                    title: "rename pane",
                    input: label.clone().unwrap_or_default(),
                    replace_on_type: label.is_none(),
                    target: ClientRenameTarget::Pane { pane_id },
                }));
            }
            ClientContextMenuAction::ClearPaneName => self.push_endpoint_method(
                Method::PaneRename(PaneRenameParams {
                    pane_id,
                    label: None,
                }),
                outcome,
            ),
            ClientContextMenuAction::SwapWithFocusedPane => {
                if let Some(source_pane_id) = source_pane_id {
                    self.push_endpoint_method(
                        Method::PaneSwap(PaneSwapParams {
                            pane_id: None,
                            direction: None,
                            source_pane_id: Some(source_pane_id.clone()),
                            target_pane_id: Some(pane_id),
                        }),
                        outcome,
                    );
                    self.push_endpoint_method(
                        Method::PaneFocus(PaneTarget {
                            pane_id: source_pane_id,
                        }),
                        outcome,
                    );
                }
            }
            ClientContextMenuAction::SplitRight | ClientContextMenuAction::SplitDown => {
                self.push_endpoint_method(
                    Method::PaneSplit(PaneSplitParams {
                        workspace_id: Some(workspace_id),
                        target_pane_id: Some(pane_id),
                        direction: if action == ClientContextMenuAction::SplitRight {
                            SplitDirection::Right
                        } else {
                            SplitDirection::Down
                        },
                        ratio: None,
                        cwd: None,
                        focus: true,
                        right_click: Default::default(),
                        env: Default::default(),
                    }),
                    outcome,
                );
            }
            ClientContextMenuAction::Zoom => self.push_endpoint_method(
                Method::PaneZoom(PaneZoomParams {
                    pane_id: Some(pane_id),
                    mode: PaneZoomMode::Toggle,
                }),
                outcome,
            ),
            ClientContextMenuAction::ToggleRightClickPassthrough => self.push_endpoint_method(
                Method::PaneInputSet(PaneInputSetParams {
                    pane_id,
                    right_click: if right_click_passthrough {
                        PaneRightClickTarget::Herdr
                    } else {
                        PaneRightClickTarget::Pane
                    },
                }),
                outcome,
            ),
            ClientContextMenuAction::ClosePane => {
                self.push_endpoint_method(Method::PaneClose(PaneTarget { pane_id }), outcome)
            }
            _ => {}
        }
    }
}
