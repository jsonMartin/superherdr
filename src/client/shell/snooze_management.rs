use super::*;

fn focus_belongs_to_endpoint(focus: &ClientFocusScope, endpoint_id: &ClientEndpointId) -> bool {
    matches!(
        focus,
        ClientFocusScope::Worktree { endpoint_id: focused_endpoint, .. }
            | ClientFocusScope::StandaloneWorkspace {
                endpoint_id: focused_endpoint,
                ..
            } if focused_endpoint == endpoint_id
    )
}

impl ClientSnoozeManagementOverlay {
    /// Wake-all and reset confirmations count explicit snooze records only; inherited
    /// coverage rows are presentation and never inflate a destructive count.
    pub(super) fn explicit_record_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| {
                !matches!(
                    record.target,
                    ClientSnoozeManagementTarget::Inherited { .. }
                )
            })
            .count()
    }
}

fn snooze_management_targets_match(
    left: &ClientSnoozeManagementTarget,
    right: &ClientSnoozeManagementTarget,
) -> bool {
    match (left, right) {
        (
            ClientSnoozeManagementTarget::Workspace {
                workspace_id: left_id,
                revision: left_revision,
            },
            ClientSnoozeManagementTarget::Workspace {
                workspace_id: right_id,
                revision: right_revision,
            },
        ) => left_id == right_id && left_revision == right_revision,
        (
            ClientSnoozeManagementTarget::Project {
                project_key: left_key,
                revision: left_revision,
            },
            ClientSnoozeManagementTarget::Project {
                project_key: right_key,
                revision: right_revision,
            },
        ) => left_key == right_key && left_revision == right_revision,
        (
            ClientSnoozeManagementTarget::Persisted {
                record_id: left_id,
                deadline_unix_ms: left_deadline,
            },
            ClientSnoozeManagementTarget::Persisted {
                record_id: right_id,
                deadline_unix_ms: right_deadline,
            },
        ) => left_id == right_id && left_deadline == right_deadline,
        // Inherited identity is the covered workspace plus its project; parent deadline or
        // revision changes must not lose the selection during refresh.
        (
            ClientSnoozeManagementTarget::Inherited {
                workspace_id: left_workspace,
                project_key: left_key,
            },
            ClientSnoozeManagementTarget::Inherited {
                workspace_id: right_workspace,
                project_key: right_key,
            },
        ) => left_workspace == right_workspace && left_key == right_key,
        _ => false,
    }
}

/// Builds the management row for an explicit workspace snooze record, shared by the
/// project-tree pass (covered children) and the leftover pass (unparented records).
#[allow(clippy::too_many_arguments)]
fn workspace_management_row(
    record: &crate::api::schema::WorkspaceSnoozeRecord,
    workspace: Option<&ClientShellWorkspace>,
    stored: Option<&crate::api::schema::SnoozeStoredRecordInfo>,
    endpoint_online: bool,
    outside_focus: bool,
    project_key: Option<String>,
    covered_by_project: bool,
    project_revision: Option<u64>,
) -> ClientSnoozeManagementRecord {
    ClientSnoozeManagementRecord {
        created_unix_ms: stored.map(|stored| stored.created_unix_ms),
        target: stored.map_or_else(
            || ClientSnoozeManagementTarget::Workspace {
                workspace_id: record.workspace_id.clone(),
                revision: record.revision,
            },
            |stored| ClientSnoozeManagementTarget::Persisted {
                record_id: stored.record_id.clone(),
                deadline_unix_ms: record.deadline_unix_ms,
            },
        ),
        label: stored.map_or_else(
            || {
                workspace.map_or_else(
                    || record.workspace_id.clone(),
                    |workspace| workspace.label.clone(),
                )
            },
            |stored| stored.label.clone(),
        ),
        scope: "Workspace".to_owned(),
        project_label: stored
            .and_then(|stored| stored.project_label.clone())
            .or_else(|| {
                workspace.and_then(|workspace| {
                    workspace
                        .worktree
                        .as_ref()
                        .map(|worktree| worktree.label.clone())
                })
            }),
        deadline_unix_ms: record.deadline_unix_ms,
        available: endpoint_online
            && workspace.is_some()
            && stored.is_none_or(|stored| stored.available),
        outside_focus,
        covered_by_project,
        project_key,
        project_revision,
        workspace_id: Some(record.workspace_id.clone()),
    }
}

impl ClientShellState {
    pub(super) fn snooze_management_currently_outside_focus(
        &self,
        endpoint_id: &ClientEndpointId,
        boot_id: &str,
        target: Option<&ClientSnoozeManagementFocusTarget>,
    ) -> bool {
        let Some(focus) = self.focus_scope.as_ref() else {
            return false;
        };
        let Some(snapshot) = self
            .endpoints
            .iter()
            .find(|endpoint| &endpoint.endpoint_id == endpoint_id)
            .and_then(|endpoint| endpoint.snapshot.as_deref())
            .filter(|snapshot| snapshot.boot_id == boot_id)
        else {
            return false;
        };
        let workspace = match target {
            Some(ClientSnoozeManagementFocusTarget::Workspace(workspace_id)) => snapshot
                .workspaces
                .iter()
                .find(|workspace| workspace.workspace_id == *workspace_id),
            Some(ClientSnoozeManagementFocusTarget::Project(project_key)) => {
                snapshot.workspaces.iter().find(|workspace| {
                    workspace
                        .worktree
                        .as_ref()
                        .is_some_and(|worktree| worktree.key == *project_key)
                })
            }
            None => None,
        };
        !focus_belongs_to_endpoint(focus, endpoint_id)
            || workspace.is_some_and(|workspace| {
                !focus.matches_workspace(endpoint_id, Some(boot_id), workspace)
            })
    }

    pub(super) fn open_snooze_management_for_endpoint(
        &mut self,
        endpoint_id: ClientEndpointId,
        notice: Option<String>,
    ) {
        let Some(endpoint) = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == endpoint_id)
        else {
            return;
        };
        let snapshot = endpoint.snapshot.as_deref();
        let Some(state) = endpoint.snooze_state.as_ref() else {
            return;
        };
        let boot_id = snapshot
            .map(|snapshot| snapshot.boot_id.clone())
            .unwrap_or_else(|| state.boot_id.clone());
        let workspaces = snapshot.map_or(&[][..], |snapshot| snapshot.workspaces.as_slice());
        let workspace_by_id = workspaces
            .iter()
            .map(|workspace| (workspace.workspace_id.as_str(), workspace))
            .collect::<HashMap<_, _>>();
        let worktree_by_key = workspaces
            .iter()
            .filter_map(|workspace| {
                workspace
                    .worktree
                    .as_ref()
                    .map(|worktree| (worktree.key.as_str(), workspace))
            })
            .collect::<HashMap<_, _>>();
        let persisted_by_workspace = state.persistence.as_ref().map(|persistence| {
            persistence
                .records
                .iter()
                .filter_map(|record| record.workspace_id.as_deref().map(|key| (key, record)))
                .collect::<HashMap<_, _>>()
        });
        let persisted_by_project = state.persistence.as_ref().map(|persistence| {
            persistence
                .records
                .iter()
                .filter(|record| record.scope == "project")
                .filter_map(|record| record.project_key.as_deref().map(|key| (key, record)))
                .collect::<HashMap<_, _>>()
        });
        let project_by_key = state
            .project_records
            .iter()
            .filter(|record| record.boot_id == boot_id)
            .map(|record| (record.project_key.as_str(), record))
            .collect::<HashMap<_, _>>();
        let endpoint_online = endpoint.status == ClientEndpointStatus::Online;
        let mut records = Vec::new();
        let focus = self.focus_scope.as_ref();
        // Workspace ids already placed under their project parent in the tree; the leftover
        // pass must not repeat them.
        let mut emitted_workspaces = HashSet::new();

        // Explicit project snoozes become tree parents; their current member workspaces are
        // listed underneath in canonical snapshot order. Children with their own explicit
        // workspace record keep that real record (own deadline, wake keeps the parent
        // restriction); everyone else gets a display-only inherited row carrying the
        // parent's deadline and identity.
        for record in state
            .project_records
            .iter()
            .filter(|record| record.boot_id == boot_id)
        {
            let workspace = worktree_by_key.get(record.project_key.as_str()).copied();
            let stored = persisted_by_project
                .as_ref()
                .and_then(|records| records.get(record.project_key.as_str()).copied());
            let outside_focus = focus.is_some_and(|focus| {
                !focus_belongs_to_endpoint(focus, &endpoint_id)
                    || workspace.is_some_and(|workspace| {
                        !focus.matches_workspace(&endpoint_id, Some(&boot_id), workspace)
                    })
            });
            let project_label = stored.map_or_else(
                || {
                    workspace
                        .and_then(|workspace| workspace.worktree.as_ref())
                        .map_or_else(
                            || record.project_key.clone(),
                            |worktree| worktree.label.clone(),
                        )
                },
                |stored| {
                    stored
                        .project_label
                        .clone()
                        .unwrap_or_else(|| stored.label.clone())
                },
            );
            records.push(ClientSnoozeManagementRecord {
                created_unix_ms: stored.map(|stored| stored.created_unix_ms),
                target: stored.map_or_else(
                    || ClientSnoozeManagementTarget::Project {
                        project_key: record.project_key.clone(),
                        revision: record.revision,
                    },
                    |stored| ClientSnoozeManagementTarget::Persisted {
                        record_id: stored.record_id.clone(),
                        deadline_unix_ms: record.deadline_unix_ms,
                    },
                ),
                label: project_label.clone(),
                scope: "Project".to_owned(),
                project_label: None,
                deadline_unix_ms: record.deadline_unix_ms,
                available: endpoint_online
                    && workspace.is_some()
                    && stored.is_none_or(|stored| stored.available),
                outside_focus,
                covered_by_project: false,
                project_key: Some(record.project_key.clone()),
                project_revision: Some(record.revision),
                workspace_id: None,
            });
            for member in workspaces.iter().filter(|workspace| {
                workspace
                    .worktree
                    .as_ref()
                    .is_some_and(|worktree| worktree.key == record.project_key)
            }) {
                if !emitted_workspaces.insert(member.workspace_id.clone()) {
                    continue;
                }
                let own = state.records.iter().find(|candidate| {
                    candidate.boot_id == boot_id && candidate.workspace_id == member.workspace_id
                });
                let member_outside_focus = focus.is_some_and(|focus| {
                    !focus_belongs_to_endpoint(focus, &endpoint_id)
                        || !focus.matches_workspace(&endpoint_id, Some(&boot_id), member)
                });
                if let Some(own) = own {
                    let member_stored = persisted_by_workspace
                        .as_ref()
                        .and_then(|records| records.get(member.workspace_id.as_str()).copied());
                    records.push(workspace_management_row(
                        own,
                        Some(member),
                        member_stored,
                        endpoint_online,
                        member_outside_focus,
                        Some(record.project_key.clone()),
                        true,
                        Some(record.revision),
                    ));
                } else {
                    records.push(ClientSnoozeManagementRecord {
                        created_unix_ms: stored.map(|stored| stored.created_unix_ms),
                        target: ClientSnoozeManagementTarget::Inherited {
                            workspace_id: member.workspace_id.clone(),
                            project_key: record.project_key.clone(),
                        },
                        label: member.label.clone(),
                        scope: "Inherited".to_owned(),
                        project_label: Some(project_label.clone()),
                        deadline_unix_ms: record.deadline_unix_ms,
                        available: endpoint_online,
                        outside_focus: member_outside_focus,
                        covered_by_project: true,
                        project_key: Some(record.project_key.clone()),
                        project_revision: Some(record.revision),
                        workspace_id: Some(member.workspace_id.clone()),
                    });
                }
            }
        }
        // Workspace records without a same-boot project parent stay standalone rows.
        for record in state
            .records
            .iter()
            .filter(|record| record.boot_id == boot_id)
        {
            if emitted_workspaces.contains(&record.workspace_id) {
                continue;
            }
            let workspace = workspace_by_id.get(record.workspace_id.as_str()).copied();
            let stored = persisted_by_workspace
                .as_ref()
                .and_then(|records| records.get(record.workspace_id.as_str()).copied());
            let project_key = workspace
                .and_then(|workspace| workspace.worktree.as_ref())
                .map(|worktree| worktree.key.clone());
            let covered_by_project = project_key
                .as_deref()
                .is_some_and(|key| project_by_key.contains_key(key));
            let project_revision = project_key
                .as_deref()
                .and_then(|key| project_by_key.get(key))
                .map(|project| project.revision);
            let outside_focus = focus.is_some_and(|focus| {
                !focus_belongs_to_endpoint(focus, &endpoint_id)
                    || workspace.is_some_and(|workspace| {
                        !focus.matches_workspace(&endpoint_id, Some(&boot_id), workspace)
                    })
            });
            records.push(workspace_management_row(
                record,
                workspace,
                stored,
                endpoint_online,
                outside_focus,
                project_key,
                covered_by_project,
                project_revision,
            ));
        }
        if let Some(persistence) = state.persistence.as_ref() {
            for record in persistence
                .records
                .iter()
                .filter(|record| !record.available)
            {
                let duplicate = record.workspace_id.as_deref().is_some_and(|workspace_id| {
                    records.iter().any(|row| {
                        matches!(&row.target, ClientSnoozeManagementTarget::Workspace { workspace_id: id, .. } if id == workspace_id)
                    })
                }) || record.project_key.as_deref().is_some_and(|project_key| {
                    records.iter().any(|row| {
                        matches!(&row.target, ClientSnoozeManagementTarget::Project { project_key: key, .. } if key == project_key)
                    })
                }) || records.iter().any(|row| {
                        matches!(&row.target, ClientSnoozeManagementTarget::Persisted { record_id, .. } if record_id == &record.record_id)
                });
                if duplicate {
                    continue;
                }
                records.push(ClientSnoozeManagementRecord {
                    created_unix_ms: Some(record.created_unix_ms),
                    target: ClientSnoozeManagementTarget::Persisted {
                        record_id: record.record_id.clone(),
                        deadline_unix_ms: record.deadline_unix_ms,
                    },
                    label: record.label.clone(),
                    scope: record.scope.clone(),
                    project_label: record.project_label.clone(),
                    deadline_unix_ms: record.deadline_unix_ms,
                    available: false,
                    outside_focus: false,
                    covered_by_project: false,
                    project_key: record.project_key.clone(),
                    project_revision: None,
                    workspace_id: record.workspace_id.clone(),
                });
            }
        }
        let notice = notice.or_else(|| {
            state
                .persistence
                .as_ref()
                .and_then(|persistence| persistence.notice.clone())
        });
        let notice = notice.or_else(|| records.is_empty().then(|| "Nothing snoozed.".to_owned()));
        let endpoint_label = self.endpoint_label(&endpoint_id).to_owned();
        self.overlay = Some(ClientShellOverlay::SnoozeManagement(
            ClientSnoozeManagementOverlay {
                now_unix_ms: (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000)
                    as i64,
                host_count: self
                    .endpoints
                    .iter()
                    .filter(|endpoint| endpoint.snooze_state.is_some())
                    .count(),
                endpoint_id,
                boot_id,
                endpoint_label,
                expected_revision: state.revision,
                records,
                selected: 0,
                scroll: 0,
                notice,
                restriction: None,
                parent_action: None,
            },
        ));
    }

    pub(super) fn refresh_snooze_management_endpoint_status(
        &mut self,
        endpoint_id: &ClientEndpointId,
    ) {
        let Some(ClientShellOverlay::SnoozeManagement(management)) = self.overlay.as_ref() else {
            return;
        };
        if &management.endpoint_id != endpoint_id {
            return;
        }
        let selected = management.selected;
        let selected_target = management
            .records
            .get(selected)
            .map(|record| record.target.clone());
        let boot_id = management.boot_id.clone();
        let scroll = management.scroll;
        let notice = management.notice.clone();
        let restriction = management.restriction.clone();
        let parent_action = management.parent_action.clone();

        self.open_snooze_management_for_endpoint(endpoint_id.clone(), notice);
        let Some(ClientShellOverlay::SnoozeManagement(management)) = self.overlay.as_mut() else {
            return;
        };
        let same_boot = management.boot_id == boot_id;
        management.selected = selected_target
            .as_ref()
            .and_then(|target| {
                management
                    .records
                    .iter()
                    .position(|record| snooze_management_targets_match(&record.target, target))
            })
            .unwrap_or(selected)
            .min(management.records.len().saturating_sub(1));
        management.scroll = scroll;
        if same_boot {
            management.restriction = restriction;
            management.parent_action = parent_action;
        }
    }

    pub(super) fn move_snooze_management_selection(&mut self, delta: isize) {
        let visible_height = self.hits.snooze_management_popup.height;
        let fallback_height = self.last_composed_size.map_or(0, |(width, height)| {
            if width <= 28 || height <= 8 {
                height
            } else {
                height.saturating_sub(2).min(22)
            }
        });
        let Some(ClientShellOverlay::SnoozeManagement(management)) = self.overlay.as_ref() else {
            return;
        };
        let Some(last) = management.records.len().checked_sub(1) else {
            return;
        };
        let selected = (management.selected as isize + delta).clamp(0, last as isize) as usize;
        let visible = usize::from(if visible_height == 0 {
            fallback_height
        } else {
            visible_height
        })
        .saturating_sub(13)
        .max(1);
        self.select_snooze_management_record(selected);
        if let Some(ClientShellOverlay::SnoozeManagement(management)) = self.overlay.as_mut() {
            management.scroll = selected.saturating_sub(visible.saturating_sub(1));
        }
    }

    pub(super) fn cycle_snooze_management_endpoint(&mut self, delta: isize) {
        let Some(current) = self.overlay.as_ref().and_then(|overlay| match overlay {
            ClientShellOverlay::SnoozeManagement(management) => {
                Some(management.endpoint_id.clone())
            }
            _ => None,
        }) else {
            return;
        };
        let endpoints = self
            .endpoints
            .iter()
            .filter(|endpoint| endpoint.snooze_state.is_some())
            .map(|endpoint| endpoint.endpoint_id.clone())
            .collect::<Vec<_>>();
        let Some(index) = endpoints.iter().position(|endpoint| endpoint == &current) else {
            return;
        };
        let Some(next) =
            endpoints.get((index as isize + delta).rem_euclid(endpoints.len() as isize) as usize)
        else {
            return;
        };
        if next == &current {
            return;
        }
        self.open_snooze_management_for_endpoint(next.clone(), None);
    }

    pub(super) fn select_snooze_management_record(&mut self, selected: usize) {
        let Some(ClientShellOverlay::SnoozeManagement(management)) = self.overlay.as_mut() else {
            return;
        };
        if management.selected != selected {
            management.selected = selected;
            management.notice = None;
            management.parent_action = None;
        }
    }

    pub(super) fn activate_snooze_management_wake(&mut self, outcome: &mut ClientShellInput) {
        let Some(ClientShellOverlay::SnoozeManagement(management)) = self.overlay.as_ref() else {
            return;
        };
        let Some(record) = management.records.get(management.selected).cloned() else {
            return;
        };
        let endpoint_id = management.endpoint_id.clone();
        let boot_id = management.boot_id.clone();
        let expected_revision = management.expected_revision;
        let valid = self.endpoint_is_online(&endpoint_id)
            && self.endpoint_boot_id(&endpoint_id) == Some(boot_id.as_str())
            && self.snooze_record_is_current(&endpoint_id, &record, &boot_id);
        if !valid {
            self.set_snooze_management_restriction(
                "Snooze record changed or is no longer available. Close and reopen to refresh.",
            );
            return;
        }
        let label = record.label.clone();
        let project_key = record.project_key.clone();
        let covered_by_project = record.covered_by_project;
        let method = match record.target {
            // Inherited rows are display-only coverage; waking one must never mutate the
            // endpoint. The explicit covering action stays on Wake project (P).
            ClientSnoozeManagementTarget::Inherited { project_key, .. } => {
                self.set_snooze_management_restriction(&format!(
                    "{} is snoozed by project {}. Use Wake project (P) to wake it.",
                    label,
                    record
                        .project_label
                        .as_deref()
                        .unwrap_or(project_key.as_str())
                ));
                return;
            }
            ClientSnoozeManagementTarget::Workspace {
                workspace_id,
                revision,
            } => {
                crate::api::schema::Method::WorkspaceWake(crate::api::schema::WorkspaceWakeParams {
                    workspace_id: Some(workspace_id),
                    boot_id: boot_id.clone(),
                    expected_revision: revision,
                    confirmed: false,
                })
            }
            ClientSnoozeManagementTarget::Project {
                project_key,
                revision,
            } => crate::api::schema::Method::ProjectWake(crate::api::schema::ProjectWakeParams {
                project_key,
                boot_id: boot_id.clone(),
                expected_revision: revision,
            }),
            ClientSnoozeManagementTarget::Persisted { record_id, .. } => {
                crate::api::schema::Method::SnoozeRecordWake(
                    crate::api::schema::SnoozeRecordWakeParams {
                        record_id,
                        boot_id: boot_id.clone(),
                        expected_revision,
                    },
                )
            }
        };
        self.push_endpoint_method_for(
            &endpoint_id,
            boot_id,
            method,
            PendingEndpointKind::SnoozeManagementWake {
                endpoint_id: endpoint_id.clone(),
                label,
                project_key,
                covered_by_project,
                focus_target: record
                    .workspace_id
                    .clone()
                    .map(ClientSnoozeManagementFocusTarget::Workspace)
                    .or_else(|| {
                        record
                            .project_key
                            .clone()
                            .map(ClientSnoozeManagementFocusTarget::Project)
                    }),
            },
            outcome,
        );
    }

    fn snooze_record_is_current(
        &self,
        endpoint_id: &ClientEndpointId,
        record: &ClientSnoozeManagementRecord,
        boot_id: &str,
    ) -> bool {
        let Some(state) = self
            .endpoints
            .iter()
            .find(|endpoint| &endpoint.endpoint_id == endpoint_id)
            .and_then(|endpoint| endpoint.snooze_state.as_ref())
            .filter(|state| state.boot_id == boot_id)
        else {
            return false;
        };
        match &record.target {
            ClientSnoozeManagementTarget::Workspace {
                workspace_id,
                revision,
            } => state.records.iter().any(|current| {
                current.workspace_id == *workspace_id && current.revision == *revision
            }),
            ClientSnoozeManagementTarget::Project {
                project_key,
                revision,
            } => state.project_records.iter().any(|current| {
                current.project_key == *project_key && current.revision == *revision
            }),
            ClientSnoozeManagementTarget::Persisted {
                record_id,
                deadline_unix_ms,
            } => state.persistence.as_ref().is_some_and(|persistence| {
                persistence.records.iter().any(|current| {
                    current.record_id == *record_id && current.deadline_unix_ms == *deadline_unix_ms
                })
            }),
            // Display-only coverage exists while its captured parent project record does.
            ClientSnoozeManagementTarget::Inherited { project_key, .. } => {
                record.project_revision.is_some_and(|revision| {
                    state.project_records.iter().any(|current| {
                        current.project_key == *project_key && current.revision == revision
                    })
                })
            }
        }
    }

    fn set_snooze_management_restriction(&mut self, message: &str) {
        if let Some(ClientShellOverlay::SnoozeManagement(management)) = self.overlay.as_mut() {
            management.restriction = Some(message.to_owned());
        }
    }

    pub(super) fn activate_snooze_management_parent(&mut self, outcome: &mut ClientShellInput) {
        let Some(ClientShellOverlay::SnoozeManagement(management)) = self.overlay.as_ref() else {
            return;
        };
        if !self.endpoint_is_online(&management.endpoint_id)
            || self.endpoint_boot_id(&management.endpoint_id) != Some(management.boot_id.as_str())
        {
            self.set_snooze_management_restriction(
                "Server session changed. Close and reopen to refresh.",
            );
            return;
        }
        let Some(record) = management.records.get(management.selected) else {
            return;
        };
        let Some(parent) = (if record.covered_by_project {
            Some(record)
        } else {
            management.parent_action.as_ref()
        }) else {
            return;
        };
        let Some(project_key) = parent.project_key.clone() else {
            return;
        };
        let Some(project) = management.records.iter().find(|row| {
            row.scope == "Project" && row.project_key.as_deref() == Some(project_key.as_str())
        }) else {
            self.set_snooze_management_restriction(
                "Parent project changed. Close and reopen to refresh.",
            );
            return;
        };
        let boot_id = management.boot_id.clone();
        let endpoint_id = management.endpoint_id.clone();
        let Some(captured_revision) = project.project_revision else {
            self.set_snooze_management_restriction(
                "Parent project changed. Close and reopen to refresh.",
            );
            return;
        };
        let Some(revision) = self
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_id == endpoint_id)
            .and_then(|endpoint| endpoint.snooze_state.as_ref())
            .filter(|state| state.boot_id == boot_id)
            .and_then(|state| {
                state
                    .project_records
                    .iter()
                    .find(|record| record.project_key == project_key)
            })
            .map(|record| record.revision)
        else {
            self.set_snooze_management_restriction(
                "Parent project changed. Close and reopen to refresh.",
            );
            return;
        };
        if revision != captured_revision {
            self.set_snooze_management_restriction(
                "Parent project changed. Close and reopen to refresh.",
            );
            return;
        }
        let project_label = project.label.clone();
        if !self.snooze_record_is_current(&endpoint_id, project, &boot_id) {
            self.set_snooze_management_restriction(
                "Parent project changed. Close and reopen to refresh.",
            );
            return;
        }
        self.push_endpoint_method_for(
            &endpoint_id,
            boot_id.clone(),
            crate::api::schema::Method::ProjectWake(crate::api::schema::ProjectWakeParams {
                project_key: project_key.clone(),
                boot_id,
                expected_revision: revision,
            }),
            PendingEndpointKind::SnoozeManagementWake {
                endpoint_id: endpoint_id.clone(),
                label: project_label,
                project_key: None,
                covered_by_project: false,
                focus_target: Some(ClientSnoozeManagementFocusTarget::Project(project_key)),
            },
            outcome,
        );
    }

    pub(super) fn activate_snooze_management_wake_all(&mut self) {
        let Some(ClientShellOverlay::SnoozeManagement(management)) = self.overlay.as_ref() else {
            return;
        };
        let endpoint_id = management.endpoint_id.clone();
        let boot_id = management.boot_id.clone();
        let expected_revision = management.expected_revision;
        let count = management.explicit_record_count();
        let endpoint_label = management.endpoint_label.clone();
        self.overlay = Some(ClientShellOverlay::ConfirmWakeSharedSnoozes(
            ClientConfirmWakeSharedSnoozesOverlay {
                endpoint_id,
                boot_id,
                expected_revision,
                count,
                purpose: ClientWakeConfirmationPurpose::WakeSharedSnoozes,
                endpoint_label,
            },
        ));
    }
}

impl ClientShellState {
    pub(crate) fn tick_snooze_management_clock(&mut self) -> bool {
        let Some(ClientShellOverlay::SnoozeManagement(view)) = self.overlay.as_mut() else {
            return false;
        };
        let now = (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64;
        if now.div_euclid(1_000) == view.now_unix_ms.div_euclid(1_000) {
            return false;
        }
        view.now_unix_ms = now;
        true
    }
}
