use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::schema::{ProjectSnoozeRecord, WorkspaceSnoozeRecord, WorkspaceSnoozeState};

pub(crate) fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceSnoozeError {
    InvalidDuration,
    InvalidDeadline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceWakeError {
    NotFound,
    CoveredByProject,
    StaleRevision {
        expected: u64,
        current: u64,
    },
    // Nothing constructs this yet, but the server maps it to the `stale_boot` API error.
    #[allow(dead_code)]
    StaleBoot {
        expected: String,
        current: String,
    },
    UnconfirmedWakeAll,
}

#[derive(Debug, Clone)]
pub(crate) struct WorkspaceSnoozeManager {
    boot_id: String,
    revision: u64,
    records: HashMap<String, WorkspaceSnoozeRecord>,
    project_records: HashMap<String, ProjectSnoozeRecord>,
}

impl WorkspaceSnoozeManager {
    pub(crate) fn new(boot_id: String) -> Self {
        Self {
            boot_id,
            revision: 0,
            records: HashMap::new(),
            project_records: HashMap::new(),
        }
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// Advances the global revision for a persisted-state change that has no
    /// corresponding live workspace or project record.
    pub(crate) fn bump_revision(&mut self) -> u64 {
        self.revision = self.revision.saturating_add(1);
        self.revision
    }

    /// Restores a validated persisted deadline without applying new-action
    /// duration limits. The caller has already proven the target identity.
    pub(crate) fn restore_workspace(
        &mut self,
        workspace_id: String,
        deadline_unix_ms: i64,
    ) -> WorkspaceSnoozeRecord {
        self.bump_revision();
        let record = WorkspaceSnoozeRecord {
            workspace_id: workspace_id.clone(),
            boot_id: self.boot_id.clone(),
            deadline_unix_ms,
            revision: self.revision,
        };
        self.records.insert(workspace_id, record.clone());
        record
    }

    /// Restores a validated persisted project deadline without applying
    /// new-action duration limits. The caller has proven an anchor identity.
    pub(crate) fn restore_project(
        &mut self,
        project_key: String,
        deadline_unix_ms: i64,
    ) -> ProjectSnoozeRecord {
        self.bump_revision();
        let record = ProjectSnoozeRecord {
            project_key: project_key.clone(),
            boot_id: self.boot_id.clone(),
            deadline_unix_ms,
            revision: self.revision,
        };
        self.project_records.insert(project_key, record.clone());
        record
    }

    pub(crate) fn retain_workspace_ids(&mut self, valid_ids: &HashSet<String>) -> bool {
        let before = self.records.len();
        self.records
            .retain(|workspace_id, _| valid_ids.contains(workspace_id));
        if self.records.len() != before {
            self.bump_revision();
            true
        } else {
            false
        }
    }

    pub(crate) fn snooze(
        &mut self,
        workspace_id: String,
        duration_seconds: Option<u64>,
        deadline_unix_ms: Option<i64>,
        now_ms: i64,
    ) -> Result<WorkspaceSnoozeRecord, WorkspaceSnoozeError> {
        let dl_ms = validate_deadline(duration_seconds, deadline_unix_ms, now_ms)?;
        self.revision = self.revision.saturating_add(1);
        let record = WorkspaceSnoozeRecord {
            workspace_id: workspace_id.clone(),
            boot_id: self.boot_id.clone(),
            deadline_unix_ms: dl_ms,
            revision: self.revision,
        };

        self.records.insert(workspace_id, record.clone());
        Ok(record)
    }

    pub(crate) fn project_snooze(
        &mut self,
        project_key: String,
        duration_seconds: Option<u64>,
        deadline_unix_ms: Option<i64>,
        now_ms: i64,
    ) -> Result<ProjectSnoozeRecord, WorkspaceSnoozeError> {
        let dl_ms = validate_deadline(duration_seconds, deadline_unix_ms, now_ms)?;
        self.revision = self.revision.saturating_add(1);
        let record = ProjectSnoozeRecord {
            project_key: project_key.clone(),
            boot_id: self.boot_id.clone(),
            deadline_unix_ms: dl_ms,
            revision: self.revision,
        };
        self.project_records.insert(project_key, record.clone());
        Ok(record)
    }

    #[cfg(test)]
    pub(crate) fn project_record_revision(&self, project_key: &str) -> Option<u64> {
        self.project_records
            .get(project_key)
            .map(|record| record.revision)
    }

    pub(crate) fn is_project_snoozed(&self, project_key: &str) -> bool {
        self.project_records.contains_key(project_key)
    }

    pub(crate) fn project_wake(
        &mut self,
        project_key: &str,
        expected_revision: u64,
    ) -> Result<(), WorkspaceWakeError> {
        let Some(record) = self.project_records.get(project_key) else {
            return Err(WorkspaceWakeError::NotFound);
        };
        if record.revision != expected_revision {
            return Err(WorkspaceWakeError::StaleRevision {
                expected: expected_revision,
                current: record.revision,
            });
        }
        self.project_records.remove(project_key);
        self.revision = self.revision.saturating_add(1);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn wake(
        &mut self,
        workspace_id: Option<&str>,
        expected_revision: u64,
        confirmed: bool,
    ) -> Result<Vec<String>, WorkspaceWakeError> {
        self.wake_workspace(workspace_id, expected_revision, confirmed, None)
    }

    pub(crate) fn wake_workspace(
        &mut self,
        workspace_id: Option<&str>,
        expected_revision: u64,
        confirmed: bool,
        project_key: Option<&str>,
    ) -> Result<Vec<String>, WorkspaceWakeError> {
        if let Some(id) = workspace_id {
            let Some(record) = self.records.get(id) else {
                if project_key.is_some_and(|key| self.is_project_snoozed(key)) {
                    return Err(WorkspaceWakeError::CoveredByProject);
                }
                return Err(WorkspaceWakeError::NotFound);
            };
            if record.revision != expected_revision {
                return Err(WorkspaceWakeError::StaleRevision {
                    expected: expected_revision,
                    current: record.revision,
                });
            }
            self.records.remove(id);
            self.revision = self.revision.saturating_add(1);
            Ok(vec![id.to_string()])
        } else {
            if !confirmed {
                return Err(WorkspaceWakeError::UnconfirmedWakeAll);
            }
            if self.revision != expected_revision {
                return Err(WorkspaceWakeError::StaleRevision {
                    expected: expected_revision,
                    current: self.revision,
                });
            }
            let mut removed: Vec<String> = self.records.keys().cloned().collect();
            removed.extend(
                self.project_records
                    .keys()
                    .map(|key| format!("project:{key}")),
            );
            if !removed.is_empty() {
                self.records.clear();
                self.project_records.clear();
                self.revision = self.revision.saturating_add(1);
            }
            Ok(removed)
        }
    }

    pub(crate) fn expire_due(&mut self, now_ms: i64) -> bool {
        let before_len = self.records.len() + self.project_records.len();
        self.records
            .retain(|_, record| record.deadline_unix_ms > now_ms);
        self.project_records
            .retain(|_, record| record.deadline_unix_ms > now_ms);
        let after_len = self.records.len() + self.project_records.len();
        if after_len != before_len {
            self.revision = self.revision.saturating_add(1);
            true
        } else {
            false
        }
    }

    pub(crate) fn next_deadline(&self) -> Option<i64> {
        self.records
            .values()
            .map(|r| r.deadline_unix_ms)
            .chain(self.project_records.values().map(|r| r.deadline_unix_ms))
            .min()
    }

    pub(crate) fn is_snoozed(&self, workspace_id: &str) -> bool {
        self.records.contains_key(workspace_id)
    }

    pub(crate) fn state(&self) -> WorkspaceSnoozeState {
        let mut records: Vec<WorkspaceSnoozeRecord> = self.records.values().cloned().collect();
        records.sort_by(|a, b| a.workspace_id.cmp(&b.workspace_id));
        let mut project_records: Vec<ProjectSnoozeRecord> =
            self.project_records.values().cloned().collect();
        project_records.sort_by(|a, b| a.project_key.cmp(&b.project_key));
        WorkspaceSnoozeState {
            boot_id: self.boot_id.clone(),
            revision: self.revision,
            records,
            project_records,
            persistence: None,
        }
    }
}

fn validate_deadline(
    duration_seconds: Option<u64>,
    deadline_unix_ms: Option<i64>,
    now_ms: i64,
) -> Result<i64, WorkspaceSnoozeError> {
    if let Some(ms) = deadline_unix_ms {
        if ms <= now_ms || ms > now_ms.saturating_add(30 * 86_400 * 1000) {
            return Err(WorkspaceSnoozeError::InvalidDeadline);
        }
        Ok(ms)
    } else if let Some(secs) = duration_seconds {
        if secs == 0 || secs > 30 * 86_400 {
            return Err(WorkspaceSnoozeError::InvalidDuration);
        }
        let duration_ms = (secs as i64).saturating_mul(1000);
        Ok(now_ms.saturating_add(duration_ms))
    } else {
        Ok(now_ms.saturating_add(1800 * 1000))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snooze_and_wake_lifecycle() {
        let mut manager = WorkspaceSnoozeManager::new("boot-test".into());
        assert_eq!(manager.revision(), 0);

        let rec1 = manager
            .snooze("ws_1".into(), Some(1800), None, 1000)
            .unwrap();
        assert_eq!(rec1.revision, 1);
        assert_eq!(manager.revision(), 1);
        assert!(manager.is_snoozed("ws_1"));

        // Re-snooze replaces and bumps record revision to global revision
        let rec2 = manager.snooze("ws_1".into(), Some(60), None, 2000).unwrap();
        assert_eq!(rec2.revision, 2);
        assert_eq!(manager.revision(), 2);

        // Stale wake rejected
        let stale_err = manager.wake(Some("ws_1"), 1, false);
        assert_eq!(
            stale_err,
            Err(WorkspaceWakeError::StaleRevision {
                expected: 1,
                current: 2
            })
        );
        assert!(manager.is_snoozed("ws_1"));

        // Matching wake succeeds
        assert!(manager.wake(Some("ws_1"), 2, false).is_ok());
        assert!(!manager.is_snoozed("ws_1"));
        assert_eq!(manager.revision(), 3);
    }

    #[test]
    fn record_incarnation_revision_is_monotonic_and_rejects_delayed_wake() {
        let mut manager = WorkspaceSnoozeManager::new("boot-test".into());
        let rec1 = manager
            .snooze("ws_a".into(), Some(1800), None, 1000)
            .unwrap();
        assert_eq!(rec1.revision, 1);
        assert_eq!(manager.revision(), 1);

        assert!(manager.wake(Some("ws_a"), 1, false).is_ok());
        assert_eq!(manager.revision(), 2);
        assert!(!manager.is_snoozed("ws_a"));

        let rec2 = manager
            .snooze("ws_a".into(), Some(1800), None, 2000)
            .unwrap();
        assert_eq!(rec2.revision, 3);
        assert_eq!(manager.revision(), 3);
        assert!(manager.is_snoozed("ws_a"));

        let stale_err = manager.wake(Some("ws_a"), 1, false);
        assert_eq!(
            stale_err,
            Err(WorkspaceWakeError::StaleRevision {
                expected: 1,
                current: 3
            })
        );
        assert!(manager.is_snoozed("ws_a"));
        assert_eq!(manager.state().records.len(), 1);
    }

    #[test]
    fn wake_all_requires_confirmation_and_matching_revision() {
        let mut manager = WorkspaceSnoozeManager::new("boot-test".into());
        manager
            .snooze("ws_1".into(), Some(100), None, 1000)
            .unwrap();
        manager
            .snooze("ws_2".into(), Some(100), None, 1000)
            .unwrap();
        assert_eq!(manager.state().records.len(), 2);
        let rev = manager.revision();
        assert_eq!(rev, 2);

        // Unconfirmed rejected
        let unconfirmed = manager.wake(None, rev, false);
        assert_eq!(unconfirmed, Err(WorkspaceWakeError::UnconfirmedWakeAll));
        assert_eq!(manager.state().records.len(), 2);

        // Stale revision rejected
        let stale = manager.wake(None, 1, true);
        assert_eq!(
            stale,
            Err(WorkspaceWakeError::StaleRevision {
                expected: 1,
                current: 2
            })
        );
        assert_eq!(manager.state().records.len(), 2);

        // Confirmed with matching revision succeeds
        let woken = manager.wake(None, rev, true).unwrap();
        assert_eq!(woken.len(), 2);
        assert_eq!(manager.state().records.len(), 0);
    }

    #[test]
    fn expiry_removes_elapsed_deadlines() {
        let mut manager = WorkspaceSnoozeManager::new("boot-test".into());
        manager
            .snooze("ws_1".into(), None, Some(5000), 1000)
            .unwrap();
        manager
            .snooze("ws_2".into(), None, Some(10000), 1000)
            .unwrap();

        assert!(!manager.expire_due(4000));
        assert_eq!(manager.state().records.len(), 2);

        assert!(manager.expire_due(6000));
        assert_eq!(manager.state().records.len(), 1);
        assert!(!manager.is_snoozed("ws_1"));
        assert!(manager.is_snoozed("ws_2"));
    }
}
