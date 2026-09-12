use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::api::schema::{SnoozePersistenceInfo, SnoozeStoredRecordInfo, WorkspaceSnoozeState};
use crate::server::workspace_snooze::WorkspaceSnoozeManager;
use crate::workspace::Workspace;

const STORE_VERSION: u32 = 1;
const MAX_LABEL_BYTES: usize = 160;
const MAX_RECORDS: usize = 256;
const MAX_ANCHORS: usize = 256;
const MAX_STORE_BYTES: usize = 64 * 1024;
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum LiveSnoozeTarget {
    Workspace(String),
    Project(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum StoredTarget {
    Workspace {
        lifetime_id: String,
    },
    Project {
        worktree_key: String,
        anchor_workspace_lifetime_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoredRecord {
    record_id: String,
    created_unix_ms: i64,
    deadline_unix_ms: i64,
    label: String,
    #[serde(default)]
    project_label: Option<String>,
    target: StoredTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoreDocument {
    version: u32,
    records: Vec<StoredRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SaveOutcome {
    Durable,
    CommittedWithWarning(String),
}

#[derive(Debug, Clone)]
pub(crate) struct SnoozeStore {
    path: PathBuf,
    records: HashMap<String, StoredRecord>,
    bindings: HashMap<String, LiveSnoozeTarget>,
    write_blocked: bool,
    needs_save: bool,
}

impl SnoozeStore {
    fn empty(path: PathBuf) -> Self {
        Self {
            path,
            records: HashMap::new(),
            bindings: HashMap::new(),
            write_blocked: false,
            needs_save: false,
        }
    }

    pub(crate) fn load(
        path: PathBuf,
        workspaces: &[Workspace],
        manager: &mut WorkspaceSnoozeManager,
        now_ms: i64,
    ) -> (Self, Option<String>) {
        let mut store = Self::empty(path.clone());
        let content = match std::fs::read(&path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return (store, None),
            Err(err) => {
                store.write_blocked = true;
                return (store, Some(format!("failed to read snooze state: {err}")));
            }
        };
        if content.len() > MAX_STORE_BYTES {
            return store.quarantine("snooze state exceeds the storage limit".into());
        }
        let document = match serde_json::from_slice::<StoreDocument>(&content) {
            Ok(document) if document.version == STORE_VERSION => document,
            Ok(document) => {
                return store.quarantine(format!(
                    "unsupported snooze state version {}",
                    document.version
                ));
            }
            Err(err) => return store.quarantine(format!("invalid snooze state: {err}")),
        };

        if let Err(error) = validate_document(&document) {
            return store.quarantine(error);
        }
        let by_lifetime: HashMap<&str, &Workspace> = workspaces
            .iter()
            .map(|workspace| (workspace.lifetime_id.as_str(), workspace))
            .collect();
        let mut project_members: HashMap<&str, Vec<&Workspace>> = HashMap::new();
        for workspace in workspaces {
            if let Some(space) = workspace.worktree_space() {
                project_members
                    .entry(space.key.as_str())
                    .or_default()
                    .push(workspace);
            }
        }
        let mut pruned = false;
        for record in document.records {
            let mut record = record;
            if let StoredTarget::Project {
                anchor_workspace_lifetime_ids,
                ..
            } = &mut record.target
            {
                anchor_workspace_lifetime_ids.sort();
                anchor_workspace_lifetime_ids.dedup();
            }
            if record.record_id.is_empty() || record.deadline_unix_ms <= now_ms {
                pruned = true;
                continue;
            }
            let id = record.record_id.clone();
            if let Some(target) = resolve_target(&record.target, &by_lifetime, &project_members) {
                match &target {
                    LiveSnoozeTarget::Workspace(workspace_id) => {
                        manager.restore_workspace(workspace_id.clone(), record.deadline_unix_ms);
                    }
                    LiveSnoozeTarget::Project(project_key) => {
                        manager.restore_project(project_key.clone(), record.deadline_unix_ms);
                    }
                }
                store.bindings.insert(id.clone(), target);
            }
            store.records.insert(id, record);
        }
        store.needs_save = pruned;
        let notice = pruned.then(|| "expired snooze records were removed".into());
        (store, notice)
    }

    fn quarantine(mut self, notice: String) -> (Self, Option<String>) {
        let name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("snooze.json");
        let quarantine = self
            .path
            .with_file_name(format!("{name}.corrupt-{}", uuid::Uuid::new_v4()));
        match crate::platform::replace_file(&self.path, &quarantine) {
            Ok(()) => (
                self,
                Some(format!("{notice}; preserved unreadable snooze state")),
            ),
            Err(err) => {
                self.write_blocked = true;
                (
                    self,
                    Some(format!(
                        "{notice}; could not preserve unreadable snooze state: {err}"
                    )),
                )
            }
        }
    }

    pub(crate) fn staged(
        &self,
        live: &WorkspaceSnoozeState,
        workspaces: &[Workspace],
        now_ms: i64,
    ) -> Result<Self, String> {
        let mut next = self.clone();
        let workspace_by_id: HashMap<&str, &Workspace> = workspaces
            .iter()
            .map(|workspace| (workspace.id.as_str(), workspace))
            .collect();
        let mut project_members: HashMap<&str, Vec<&Workspace>> = HashMap::new();
        for workspace in workspaces {
            if let Some(space) = workspace.worktree_space() {
                project_members
                    .entry(space.key.as_str())
                    .or_default()
                    .push(workspace);
            }
        }
        let live_workspace_ids: HashSet<&str> = live
            .records
            .iter()
            .map(|record| record.workspace_id.as_str())
            .collect();
        let live_project_keys: HashSet<&str> = live
            .project_records
            .iter()
            .map(|record| record.project_key.as_str())
            .collect();
        let binding_by_target: HashMap<LiveSnoozeTarget, String> = self
            .bindings
            .iter()
            .map(|(record_id, target)| (target.clone(), record_id.clone()))
            .collect();

        let mut seen = HashSet::new();
        for (record_id, target) in &self.bindings {
            let present = match target {
                LiveSnoozeTarget::Workspace(id) => live_workspace_ids.contains(id.as_str()),
                LiveSnoozeTarget::Project(key) => live_project_keys.contains(key.as_str()),
            };
            if !present {
                next.bindings.remove(record_id);
                next.records.remove(record_id);
            } else {
                seen.insert(record_id.clone());
            }
        }

        for record in &live.records {
            let Some(workspace) = workspace_by_id.get(record.workspace_id.as_str()) else {
                // A closed workspace can leave an explicit record unavailable;
                // retain its UUID for exact recovery rather than guessing a replacement.
                if let Some(record_id) =
                    binding_by_target.get(&LiveSnoozeTarget::Workspace(record.workspace_id.clone()))
                {
                    next.bindings.remove(record_id);
                }
                continue;
            };
            let target = LiveSnoozeTarget::Workspace(record.workspace_id.clone());
            let existing_id = binding_by_target.get(&target).cloned();
            let same_lifetime = existing_id.as_ref().is_some_and(|id| {
                matches!(
                    self.records.get(id).map(|record| &record.target),
                    Some(StoredTarget::Workspace { lifetime_id })
                        if lifetime_id == &workspace.lifetime_id
                )
            });
            let record_id = match (same_lifetime, existing_id) {
                (true, Some(id)) => id,
                (_, Some(id)) => {
                    // Keep the old UUID unavailable when a public ID is reused.
                    next.bindings.remove(&id);
                    continue;
                }
                (_, None) => uuid::Uuid::new_v4().to_string(),
            };
            let old = self.records.get(&record_id);
            next.records.insert(
                record_id.clone(),
                StoredRecord {
                    record_id: record_id.clone(),
                    created_unix_ms: old.map_or(now_ms, |item| item.created_unix_ms),
                    deadline_unix_ms: record.deadline_unix_ms,
                    label: workspace_label(workspace),
                    project_label: workspace
                        .worktree_space()
                        .map(|space| bound_label(&space.label)),
                    target: StoredTarget::Workspace {
                        lifetime_id: workspace.lifetime_id.clone(),
                    },
                },
            );
            next.bindings.insert(record_id.clone(), target);
            seen.insert(record_id);
        }

        for record in &live.project_records {
            let members = project_members
                .get(record.project_key.as_str())
                .cloned()
                .unwrap_or_default();
            let project_label = members.first().and_then(|workspace| {
                workspace
                    .worktree_space()
                    .map(|space| bound_label(&space.label))
            });
            let target = LiveSnoozeTarget::Project(record.project_key.clone());
            let record_id = binding_by_target
                .get(&target)
                .cloned()
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            let old = self.records.get(&record_id);
            let mut anchors = if !members.is_empty() {
                members
                    .iter()
                    .map(|workspace| workspace.lifetime_id.clone())
                    .collect()
            } else {
                old.and_then(|item| match &item.target {
                    StoredTarget::Project {
                        anchor_workspace_lifetime_ids,
                        ..
                    } => Some(anchor_workspace_lifetime_ids.clone()),
                    StoredTarget::Workspace { .. } => None,
                })
                .unwrap_or_default()
            };
            anchors.sort();
            anchors.dedup();
            if anchors.len() > MAX_ANCHORS {
                return Err("snooze project anchor limit exceeded".into());
            }
            anchors.truncate(MAX_ANCHORS);
            next.records.insert(
                record_id.clone(),
                StoredRecord {
                    record_id: record_id.clone(),
                    created_unix_ms: old.map_or(now_ms, |item| item.created_unix_ms),
                    deadline_unix_ms: record.deadline_unix_ms,
                    label: old.map_or_else(
                        || bound_label(&record.project_key),
                        |item| item.label.clone(),
                    ),
                    project_label: project_label
                        .or_else(|| old.and_then(|item| item.project_label.clone())),
                    target: StoredTarget::Project {
                        worktree_key: record.project_key.clone(),
                        anchor_workspace_lifetime_ids: anchors,
                    },
                },
            );
            next.bindings.insert(record_id.clone(), target);
            seen.insert(record_id);
        }

        // Unbound records are unavailable and remain visible until wake or expiry.
        next.records
            .retain(|id, _| seen.contains(id) || !self.bindings.contains_key(id));
        next.needs_save = false;
        Ok(next)
    }

    pub(crate) fn retain_valid_live_records(
        &self,
        manager: &mut WorkspaceSnoozeManager,
        workspaces: &[Workspace],
    ) -> bool {
        let lifetime_by_id: HashMap<&str, &str> = workspaces
            .iter()
            .map(|workspace| (workspace.id.as_str(), workspace.lifetime_id.as_str()))
            .collect();
        let valid_ids: HashSet<String> = self
            .bindings
            .iter()
            .filter_map(|(record_id, target)| {
                let LiveSnoozeTarget::Workspace(workspace_id) = target else {
                    return None;
                };
                let lifetime_id = lifetime_by_id.get(workspace_id.as_str())?;
                let saved = self.records.get(record_id)?;
                let StoredTarget::Workspace {
                    lifetime_id: saved_lifetime,
                } = &saved.target
                else {
                    return None;
                };
                (saved_lifetime == lifetime_id).then(|| workspace_id.clone())
            })
            .collect();
        manager.retain_workspace_ids(&valid_ids)
    }

    pub(crate) fn save(&self) -> io::Result<SaveOutcome> {
        if self.write_blocked {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "snooze state writes are blocked",
            ));
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("snooze path has no parent"))?;
        std::fs::create_dir_all(parent)?;
        if let Ok(metadata) = std::fs::symlink_metadata(&self.path) {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "refusing to replace a non-file snooze path",
                ));
            }
        }
        let mut records: Vec<_> = self.records.values().cloned().collect();
        records.sort_by(|a, b| a.record_id.cmp(&b.record_id));
        let document = StoreDocument {
            version: STORE_VERSION,
            records,
        };
        validate_document(&document)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let bytes = serde_json::to_vec_pretty(&document)?;
        if bytes.len() > MAX_STORE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "snooze state exceeds storage limit",
            ));
        }
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("snooze.json");
        let temp = parent.join(format!(".{name}.{}.{}.tmp", std::process::id(), sequence));
        let mut file = crate::platform::create_private_state_file(&temp)?;
        if let Err(err) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(&temp);
            return Err(err);
        }
        drop(file);
        if let Err(err) = crate::platform::replace_file(&temp, &self.path) {
            let _ = std::fs::remove_file(&temp);
            return Err(err);
        }
        match crate::platform::sync_parent_directory(parent) {
            Ok(()) => Ok(SaveOutcome::Durable),
            Err(err) => Ok(SaveOutcome::CommittedWithWarning(format!(
                "snooze state committed but directory durability is uncertain: {err}"
            ))),
        }
    }

    pub(crate) fn info(
        &self,
        workspaces: &[Workspace],
        notice: Option<String>,
    ) -> SnoozePersistenceInfo {
        let by_lifetime: HashMap<&str, &Workspace> = workspaces
            .iter()
            .map(|workspace| (workspace.lifetime_id.as_str(), workspace))
            .collect();
        let mut records: Vec<_> = self
            .records
            .values()
            .map(|record| {
                let (scope, available, workspace_id, project_key) = match &record.target {
                    StoredTarget::Workspace { lifetime_id } => {
                        let workspace = by_lifetime.get(lifetime_id.as_str());
                        let available = workspace.is_some_and(|workspace| {
                            self.bindings.get(&record.record_id).is_some_and(|target| {
                                target == &LiveSnoozeTarget::Workspace(workspace.id.clone())
                            })
                        });
                        let workspace_id = available
                            .then(|| workspace.map(|item| item.id.clone()))
                            .flatten();
                        (
                            "workspace",
                            available,
                            workspace_id,
                            None,
                        )
                    }
                    StoredTarget::Project { worktree_key, .. } => {
                        let available = self.bindings.get(&record.record_id).is_some_and(
                            |target| {
                                matches!(target, LiveSnoozeTarget::Project(key) if key == worktree_key)
                            },
                        );
                        ("project", available, None, Some(worktree_key.clone()))
                    }
                };
                SnoozeStoredRecordInfo {
                    record_id: record.record_id.clone(),
                    scope: scope.into(),
                    label: record.label.clone(),
                    project_label: record.project_label.clone(),
                    created_unix_ms: record.created_unix_ms,
                    deadline_unix_ms: record.deadline_unix_ms,
                    available,
                    workspace_id,
                    project_key,
                }
            })
            .collect();
        records.sort_by(|a, b| a.record_id.cmp(&b.record_id));
        SnoozePersistenceInfo { records, notice }
    }

    pub(crate) fn live_target(&self, record_id: &str) -> Option<LiveSnoozeTarget> {
        self.bindings.get(record_id).cloned()
    }

    pub(crate) fn remove_record(&mut self, record_id: &str) -> bool {
        self.bindings.remove(record_id);
        self.records.remove(record_id).is_some()
    }

    pub(crate) fn clear(&mut self) -> bool {
        let changed = !self.records.is_empty();
        self.records.clear();
        self.bindings.clear();
        changed
    }

    pub(crate) fn next_deadline(&self) -> Option<i64> {
        self.records
            .values()
            .map(|record| record.deadline_unix_ms)
            .min()
    }

    pub(crate) fn expire_due(&mut self, now_ms: i64) -> bool {
        let expired: Vec<String> = self
            .records
            .values()
            .filter(|record| record.deadline_unix_ms <= now_ms)
            .map(|record| record.record_id.clone())
            .collect();
        expired
            .iter()
            .fold(false, |changed, id| self.remove_record(id) || changed)
    }

    pub(crate) fn has_records(&self) -> bool {
        !self.records.is_empty()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub(crate) fn same_document(&self, other: &Self) -> bool {
        self.records == other.records
    }

    pub(crate) fn needs_save(&self) -> bool {
        self.needs_save
    }

    pub(crate) fn is_write_blocked(&self) -> bool {
        self.write_blocked
    }

    pub(crate) fn reset_write_block(&mut self) {
        self.write_blocked = false;
    }
}

fn validate_document(document: &StoreDocument) -> Result<(), String> {
    if document.version != STORE_VERSION {
        return Err(format!(
            "unsupported snooze state version {}",
            document.version
        ));
    }
    if document.records.len() > MAX_RECORDS {
        return Err(format!(
            "snooze state has too many records (max {MAX_RECORDS})"
        ));
    }
    let mut ids = HashSet::new();
    let mut targets = HashSet::new();
    for record in &document.records {
        if uuid::Uuid::parse_str(&record.record_id).is_err()
            || !ids.insert(record.record_id.clone())
            || !targets.insert(record.target.clone())
            || record.label.len() > MAX_LABEL_BYTES
            || record
                .project_label
                .as_ref()
                .is_some_and(|label| label.len() > MAX_LABEL_BYTES)
        {
            return Err("snooze state contains duplicate or invalid record data".into());
        }
        match &record.target {
            StoredTarget::Workspace { lifetime_id } if lifetime_id.is_empty() => {
                return Err("snooze workspace target has no lifetime ID".into());
            }
            StoredTarget::Project {
                worktree_key,
                anchor_workspace_lifetime_ids,
            } if worktree_key.is_empty() || anchor_workspace_lifetime_ids.len() > MAX_ANCHORS => {
                return Err("snooze project target contains invalid provenance".into());
            }
            _ => {}
        }
    }
    Ok(())
}

fn resolve_target(
    target: &StoredTarget,
    by_lifetime: &HashMap<&str, &Workspace>,
    project_members: &HashMap<&str, Vec<&Workspace>>,
) -> Option<LiveSnoozeTarget> {
    match target {
        StoredTarget::Workspace { lifetime_id } => by_lifetime
            .get(lifetime_id.as_str())
            .map(|workspace| LiveSnoozeTarget::Workspace(workspace.id.clone())),
        StoredTarget::Project {
            worktree_key,
            anchor_workspace_lifetime_ids,
        } => project_members
            .get(worktree_key.as_str())
            .is_some_and(|members| {
                members.iter().any(|workspace| {
                    anchor_workspace_lifetime_ids
                        .iter()
                        .any(|id| id == &workspace.lifetime_id)
                })
            })
            .then(|| LiveSnoozeTarget::Project(worktree_key.clone())),
    }
}

fn bound_label(value: &str) -> String {
    let mut end = value.len().min(MAX_LABEL_BYTES);
    while end > 0 && !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn workspace_label(workspace: &Workspace) -> String {
    workspace
        .custom_name
        .as_deref()
        .or_else(|| workspace.worktree_space().map(|space| space.label.as_str()))
        .or_else(|| {
            workspace
                .identity_cwd
                .file_name()
                .and_then(|name| name.to_str())
        })
        .map(bound_label)
        .unwrap_or_else(|| "workspace".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::WorkspaceSnoozeState;

    fn temp_path() -> PathBuf {
        std::env::temp_dir().join(format!("herdr-snooze-{}.json", uuid::Uuid::new_v4()))
    }

    fn live_state(manager: &WorkspaceSnoozeManager) -> WorkspaceSnoozeState {
        manager.state()
    }

    #[test]
    fn sidecar_round_trip_restores_workspace_by_lifetime() {
        let mut workspace = Workspace::test_new("alpha");
        workspace.id = "ws_same".into();
        let mut manager = WorkspaceSnoozeManager::new("boot-a".into());
        manager
            .snooze(workspace.id.clone(), None, Some(5_000), 1_000)
            .expect("valid deadline");
        let path = temp_path();
        let store = SnoozeStore::empty(path.clone())
            .staged(
                &live_state(&manager),
                std::slice::from_ref(&workspace),
                1_000,
            )
            .expect("stage");
        assert!(matches!(store.save().expect("save"), SaveOutcome::Durable));

        let mut restored_manager = WorkspaceSnoozeManager::new("boot-b".into());
        let (restored, notice) = SnoozeStore::load(
            path.clone(),
            std::slice::from_ref(&workspace),
            &mut restored_manager,
            1_000,
        );
        assert!(notice.is_none());
        assert!(restored_manager.is_snoozed("ws_same"));
        assert_eq!(
            restored
                .info(std::slice::from_ref(&workspace), None)
                .records
                .len(),
            1
        );
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn reused_public_workspace_id_stays_unavailable_and_expires() {
        let mut original = Workspace::test_new("original");
        original.id = "ws_same".into();
        let mut manager = WorkspaceSnoozeManager::new("boot-a".into());
        manager
            .snooze(original.id.clone(), None, Some(5_000), 1_000)
            .expect("valid deadline");
        let path = temp_path();
        let store = SnoozeStore::empty(path.clone())
            .staged(
                &live_state(&manager),
                std::slice::from_ref(&original),
                1_000,
            )
            .expect("stage");
        store.save().expect("save");

        let mut reused = Workspace::test_new("reused");
        reused.id = "ws_same".into();
        let mut restored_manager = WorkspaceSnoozeManager::new("boot-b".into());
        let (restored, _) = SnoozeStore::load(
            path.clone(),
            std::slice::from_ref(&reused),
            &mut restored_manager,
            1_000,
        );
        let info = restored.info(std::slice::from_ref(&reused), None);
        assert_eq!(info.records.len(), 1);
        assert!(!info.records[0].available);
        assert!(info.records[0].workspace_id.is_none());
        let mut expired = restored;
        assert!(expired.expire_due(5_000));
        assert!(expired.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn project_restore_requires_an_anchor() {
        let mut workspace = Workspace::test_new("repo");
        workspace.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo".into(),
            label: "Repo".into(),
            repo_root: "/repo".into(),
            checkout_path: "/repo".into(),
            is_linked_worktree: false,
        });
        let record_id = uuid::Uuid::new_v4().to_string();
        let document = StoreDocument {
            version: STORE_VERSION,
            records: vec![StoredRecord {
                record_id,
                created_unix_ms: 1,
                deadline_unix_ms: 5_000,
                label: "Repo".into(),
                project_label: Some("Repo".into()),
                target: StoredTarget::Project {
                    worktree_key: "repo".into(),
                    anchor_workspace_lifetime_ids: vec![workspace.lifetime_id.clone()],
                },
            }],
        };
        let path = temp_path();
        std::fs::write(&path, serde_json::to_vec(&document).expect("serialize")).expect("write");
        let mut manager = WorkspaceSnoozeManager::new("boot".into());
        let (store, _) = SnoozeStore::load(
            path.clone(),
            std::slice::from_ref(&workspace),
            &mut manager,
            1_000,
        );
        assert!(manager.is_project_snoozed("repo"));
        assert!(store.info(&[], None).records[0].available);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn project_restore_without_matching_anchor_remains_unavailable() {
        let mut workspace = Workspace::test_new("repo");
        workspace.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo".into(),
            label: "Repo".into(),
            repo_root: "/repo".into(),
            checkout_path: "/repo".into(),
            is_linked_worktree: false,
        });
        let document = StoreDocument {
            version: STORE_VERSION,
            records: vec![StoredRecord {
                record_id: uuid::Uuid::new_v4().to_string(),
                created_unix_ms: 1,
                deadline_unix_ms: 5_000,
                label: "Repo".into(),
                project_label: Some("Repo".into()),
                target: StoredTarget::Project {
                    worktree_key: "repo".into(),
                    anchor_workspace_lifetime_ids: vec!["different-boot-anchor".into()],
                },
            }],
        };
        let path = temp_path();
        std::fs::write(&path, serde_json::to_vec(&document).expect("serialize")).expect("write");
        let mut manager = WorkspaceSnoozeManager::new("boot".into());
        let (store, _) = SnoozeStore::load(
            path.clone(),
            std::slice::from_ref(&workspace),
            &mut manager,
            1_000,
        );
        assert!(!manager.is_project_snoozed("repo"));
        assert!(!store.info(&[], None).records[0].available);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn corrupt_store_is_quarantined_before_any_restore() {
        let workspace = Workspace::test_new("corrupt");
        let record_id = uuid::Uuid::new_v4().to_string();
        let document = StoreDocument {
            version: STORE_VERSION,
            records: vec![
                StoredRecord {
                    record_id: record_id.clone(),
                    created_unix_ms: 1,
                    deadline_unix_ms: 5_000,
                    label: "one".into(),
                    project_label: None,
                    target: StoredTarget::Workspace {
                        lifetime_id: workspace.lifetime_id.clone(),
                    },
                },
                StoredRecord {
                    record_id,
                    created_unix_ms: 2,
                    deadline_unix_ms: 6_000,
                    label: "duplicate".into(),
                    project_label: None,
                    target: StoredTarget::Project {
                        worktree_key: "other".into(),
                        anchor_workspace_lifetime_ids: vec![],
                    },
                },
            ],
        };
        let path = temp_path();
        std::fs::write(&path, serde_json::to_vec(&document).expect("serialize")).expect("write");
        let mut manager = WorkspaceSnoozeManager::new("boot".into());
        let (_, notice) = SnoozeStore::load(
            path.clone(),
            std::slice::from_ref(&workspace),
            &mut manager,
            1_000,
        );
        assert!(notice.is_some());
        assert!(manager.state().records.is_empty());
        if let Some(parent) = path.parent() {
            if let Ok(entries) = std::fs::read_dir(parent) {
                for entry in entries.flatten() {
                    if entry.file_name().to_string_lossy().starts_with(
                        path.file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or(""),
                    ) {
                        let _ = std::fs::remove_file(entry.path());
                    }
                }
            }
        }
    }

    #[test]
    fn precommit_failure_keeps_previous_path_untouched() {
        let blocker = temp_path();
        std::fs::write(&blocker, b"existing").expect("write blocker");
        let path = blocker.join("snooze.json");
        let store = SnoozeStore::empty(path);
        assert!(store.save().is_err());
        assert_eq!(std::fs::read(&blocker).expect("read blocker"), b"existing");
        let _ = std::fs::remove_file(blocker);
    }

    #[test]
    fn labels_are_bounded_on_utf8_character_boundaries() {
        let label = bound_label(&"é".repeat(200));
        assert!(label.len() <= MAX_LABEL_BYTES);
        assert!(std::str::from_utf8(label.as_bytes()).is_ok());
    }

    #[test]
    fn nested_records_survive_workspace_reuse_and_project_member_changes() {
        let mut original = Workspace::test_new("original");
        original.id = "same".into();
        original.worktree_space = Some(crate::workspace::WorktreeSpaceMembership {
            key: "repo".into(),
            label: "Repo".into(),
            repo_root: "/repo".into(),
            checkout_path: "/repo".into(),
            is_linked_worktree: false,
        });
        let mut manager = WorkspaceSnoozeManager::new("boot".into());
        manager
            .snooze(original.id.clone(), None, Some(10_000), 1_000)
            .expect("valid deadline");
        manager
            .project_snooze("repo".into(), None, Some(10_000), 1_000)
            .expect("valid deadline");
        let path = temp_path();
        let store = SnoozeStore::empty(path.clone())
            .staged(&manager.state(), std::slice::from_ref(&original), 1_000)
            .expect("stage");
        store.save().expect("save");

        let mut loaded_manager = WorkspaceSnoozeManager::new("boot-2".into());
        let (loaded, _) = SnoozeStore::load(
            path.clone(),
            std::slice::from_ref(&original),
            &mut loaded_manager,
            1_000,
        );
        assert_eq!(loaded.records.len(), 2);

        let mut replacement = Workspace::test_new("replacement");
        replacement.id = "same".into();
        replacement.worktree_space = original.worktree_space.clone();
        let staged_replacement = loaded
            .staged(
                &loaded_manager.state(),
                std::slice::from_ref(&replacement),
                1_000,
            )
            .expect("stage replacement");
        assert_eq!(staged_replacement.records.len(), 2);
        assert_eq!(staged_replacement.bindings.len(), 1);
        let mut candidate_manager = loaded_manager.clone();
        assert!(
            staged_replacement.retain_valid_live_records(
                &mut candidate_manager,
                std::slice::from_ref(&replacement),
            )
        );
        assert!(!candidate_manager.is_snoozed("same"));
        let restaged = staged_replacement
            .staged(
                &candidate_manager.state(),
                std::slice::from_ref(&replacement),
                1_000,
            )
            .expect("stage after pruning");
        assert_eq!(restaged.records.len(), 2);
        assert_eq!(restaged.bindings.len(), 1);

        let mut project_only = loaded_manager.state();
        project_only.records.clear();
        let zero_members = loaded
            .staged(&project_only, &[], 1_000)
            .expect("stage zero members");
        assert_eq!(zero_members.records.len(), 1);

        let mut future_manager = WorkspaceSnoozeManager::new("boot-2".into());
        future_manager.restore_project("repo".into(), 10_000);
        let future = zero_members
            .staged(
                &future_manager.state(),
                std::slice::from_ref(&replacement),
                1_000,
            )
            .expect("stage future member");
        assert_eq!(future.records.len(), 1);
        let _ = std::fs::remove_file(path);
    }
}
