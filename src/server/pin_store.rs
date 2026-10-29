use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::server::pins::PinManager;
use crate::workspace::Workspace;

const STORE_VERSION: u32 = 1;
const MAX_WORKSPACE_PINS: usize = 256;
const MAX_AGENT_PINS: usize = 256;
const MAX_STORE_BYTES: usize = 64 * 1024;
static NEXT_TEMP_FILE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
struct StoredAgentPin {
    workspace_lifetime_id: String,
    pane_number: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StoreDocument {
    version: u32,
    pub(crate) workspaces: Vec<String>,
    pub(crate) agents: Vec<StoredAgentPin>,
}

/// Persisted mirror of the live pins in `pins.json`, beside `snooze.json`.
///
/// Entries reference stable identities — workspace `lifetime_id`s and public
/// pane numbers — so they survive restarts, while the live [`PinManager`] keeps
/// internal ids that survive pane moves.
#[derive(Debug, Clone)]
pub(crate) struct PinStore {
    path: PathBuf,
    document: StoreDocument,
    write_blocked: bool,
}

impl PinStore {
    fn empty(path: PathBuf) -> Self {
        Self {
            path,
            document: StoreDocument {
                version: STORE_VERSION,
                workspaces: Vec::new(),
                agents: Vec::new(),
            },
            write_blocked: false,
        }
    }

    /// Loads `pins.json`, restoring entries that match live workspaces by
    /// `lifetime_id` and live panes by public pane number into `manager`.
    /// Entries that match nothing are dropped. A missing file means no pins.
    pub(crate) fn load(
        path: PathBuf,
        workspaces: &[Workspace],
        manager: &mut PinManager,
    ) -> (Self, Option<String>) {
        // A missing file is an empty store. Any other read or parse problem
        // starts empty too, and leaves the original path untouched: the next
        // successful pin change overwrites it. A startup read error must not
        // stick as `write_blocked`, or that next change could never land.
        let mut store = Self::empty(path.clone());
        let content = match std::fs::read(&path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return (store, None),
            Err(err) => {
                return (store, Some(format!("failed to read pin state: {err}")));
            }
        };
        if content.len() > MAX_STORE_BYTES {
            return (
                store,
                Some("pin state exceeds the storage limit".to_string()),
            );
        }
        let document = match serde_json::from_slice::<StoreDocument>(&content) {
            Ok(document) if document.version == STORE_VERSION => document,
            Ok(document) => {
                return (
                    store,
                    Some(format!(
                        "unsupported pin state version {}",
                        document.version
                    )),
                );
            }
            Err(err) => return (store, Some(format!("invalid pin state: {err}"))),
        };
        if let Err(error) = validate_document(&document) {
            return (store, Some(error));
        }

        let by_lifetime: HashMap<&str, &Workspace> = workspaces
            .iter()
            .map(|workspace| (workspace.lifetime_id.as_str(), workspace))
            .collect();
        for lifetime_id in &document.workspaces {
            if let Some(workspace) = by_lifetime.get(lifetime_id.as_str()) {
                manager.restore_workspace(workspace.id.clone());
            }
        }
        for agent in &document.agents {
            let Some(workspace) = by_lifetime.get(agent.workspace_lifetime_id.as_str()) else {
                continue;
            };
            if let Some(pane_id) = workspace
                .public_pane_numbers
                .iter()
                .find_map(|(pane_id, number)| (*number == agent.pane_number).then_some(*pane_id))
            {
                manager.restore_pane(pane_id);
            }
        }
        // The store mirrors what is on disk so the next save only happens when
        // the live pins actually diverge from it.
        store.document = document;
        (store, None)
    }

    /// Builds the store the current pins would persist to, dropping entries
    /// whose workspace or pane no longer exists.
    pub(crate) fn staged(
        &self,
        manager: &PinManager,
        workspaces: &[Workspace],
    ) -> Result<Self, String> {
        let by_id: HashMap<&str, &Workspace> = workspaces
            .iter()
            .map(|workspace| (workspace.id.as_str(), workspace))
            .collect();
        let mut stored_workspaces = Vec::new();
        for workspace_id in manager.pinned_workspace_ids() {
            if let Some(workspace) = by_id.get(workspace_id.as_str()) {
                stored_workspaces.push(workspace.lifetime_id.clone());
            }
        }
        let mut agents = Vec::new();
        for pane_id in manager.pinned_pane_ids() {
            let Some(workspace) = workspaces
                .iter()
                .find(|workspace| workspace.pane_state(*pane_id).is_some())
            else {
                continue;
            };
            let Some(pane_number) = workspace.public_pane_number(*pane_id) else {
                continue;
            };
            agents.push(StoredAgentPin {
                workspace_lifetime_id: workspace.lifetime_id.clone(),
                pane_number,
            });
        }
        stored_workspaces.sort();
        stored_workspaces.dedup();
        agents.sort_by(|a, b| {
            a.workspace_lifetime_id
                .cmp(&b.workspace_lifetime_id)
                .then(a.pane_number.cmp(&b.pane_number))
        });
        agents.dedup();
        Ok(Self {
            path: self.path.clone(),
            document: StoreDocument {
                version: STORE_VERSION,
                workspaces: stored_workspaces,
                agents,
            },
            // A blocked flag from a failed load must not poison the next save.
            // `save` re-checks the path when the change is actually written.
            write_blocked: false,
        })
    }

    pub(crate) fn same_document(&self, other: &Self) -> bool {
        self.document == other.document
    }

    pub(crate) fn save(&self) -> io::Result<()> {
        let document = &self.document;
        if self.write_blocked {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "pin state writes are blocked",
            ));
        }
        let parent = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("pin path has no parent"))?;
        std::fs::create_dir_all(parent)?;
        if let Ok(metadata) = std::fs::symlink_metadata(&self.path) {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "refusing to replace a non-file pin path",
                ));
            }
        }
        validate_document(document)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let bytes = serde_json::to_vec_pretty(document)?;
        if bytes.len() > MAX_STORE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "pin state exceeds storage limit",
            ));
        }
        let sequence = NEXT_TEMP_FILE.fetch_add(1, Ordering::Relaxed);
        let name = self
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("pins.json");
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
        // The file itself is durable at this point; only directory-entry
        // durability is uncertain, which is not worth failing a pin over.
        if let Err(err) = crate::platform::sync_parent_directory(parent) {
            tracing::warn!(err = %err, "pin state committed but directory durability is uncertain");
        }
        Ok(())
    }

    #[cfg(test)]
    fn document(&self) -> &StoreDocument {
        &self.document
    }

    #[cfg(test)]
    pub(crate) fn agent_pin_count(&self) -> usize {
        self.document.agents.len()
    }

    #[cfg(test)]
    pub(crate) fn is_write_blocked(&self) -> bool {
        self.write_blocked
    }
}

fn validate_document(document: &StoreDocument) -> Result<(), String> {
    if document.version != STORE_VERSION {
        return Err(format!(
            "unsupported pin state version {}",
            document.version
        ));
    }
    if document.workspaces.len() > MAX_WORKSPACE_PINS {
        return Err(format!(
            "pin state has too many workspace pins (max {MAX_WORKSPACE_PINS})"
        ));
    }
    if document.agents.len() > MAX_AGENT_PINS {
        return Err(format!(
            "pin state has too many agent pins (max {MAX_AGENT_PINS})"
        ));
    }
    let mut workspaces = HashSet::new();
    let mut agents = HashSet::new();
    for lifetime_id in &document.workspaces {
        if lifetime_id.is_empty() || !workspaces.insert(lifetime_id.clone()) {
            return Err("pin state contains duplicate or invalid workspace entries".into());
        }
    }
    for agent in &document.agents {
        if agent.workspace_lifetime_id.is_empty()
            || agent.pane_number == 0
            || !agents.insert(agent.clone())
        {
            return Err("pin state contains duplicate or invalid agent entries".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::PaneId;

    fn temp_path() -> PathBuf {
        std::env::temp_dir().join(format!("herdr-pins-{}.json", uuid::Uuid::new_v4()))
    }

    /// A workspace whose second pane (public number 2) comes from a real split.
    fn workspace_with_two_panes(name: &str) -> Workspace {
        let mut workspace = Workspace::test_new(name);
        workspace.test_split(ratatui::layout::Direction::Horizontal);
        workspace
    }

    fn pane_with_public_number(workspace: &Workspace, number: usize) -> PaneId {
        workspace
            .public_pane_numbers
            .iter()
            .find_map(|(pane_id, pane_number)| (*pane_number == number).then_some(*pane_id))
            .expect("pane with public number")
    }

    #[test]
    fn round_trip_restores_by_lifetime_and_public_pane_number() {
        let workspaces = vec![
            workspace_with_two_panes("alpha"),
            Workspace::test_new("beta"),
        ];
        let pinned_pane = pane_with_public_number(&workspaces[0], 2);
        let mut manager = PinManager::new();
        manager.restore_workspace(workspaces[0].id.clone());
        manager.restore_pane(pinned_pane);
        let path = temp_path();
        let store = PinStore::load(path.clone(), &workspaces, &mut PinManager::new()).0;
        let staged = store.staged(&manager, &workspaces).expect("stage document");
        assert_eq!(staged.document().workspaces.len(), 1);
        assert_eq!(staged.document().agents.len(), 1);
        staged.save().expect("save");
        let stored = staged.document().clone();

        // Simulated restart: fresh internal ids, same persisted identities.
        let mut restored = vec![
            workspace_with_two_panes("alpha-2"),
            Workspace::test_new("beta-2"),
        ];
        restored[0].lifetime_id = stored.workspaces[0].clone();
        restored[1].lifetime_id = workspaces[1].lifetime_id.clone();
        let mut restored_manager = PinManager::new();
        let (restored_store, notice) =
            PinStore::load(path.clone(), &restored, &mut restored_manager);
        assert!(notice.is_none());
        assert_eq!(restored_manager.revision(), 2);
        assert!(restored_manager.is_workspace_pinned(&restored[0].id));
        let restored_pane = pane_with_public_number(&restored[0], 2);
        assert!(restored_manager.is_pane_pinned(restored_pane));
        assert_ne!(restored_pane, pinned_pane);
        let restaged = restored_store
            .staged(&restored_manager, &restored)
            .expect("restage");
        assert_eq!(restaged.document(), &stored);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn entries_that_match_nothing_are_dropped_on_restore() {
        let workspace = Workspace::test_new("alpha");
        let mut manager = PinManager::new();
        let document = StoreDocument {
            version: STORE_VERSION,
            workspaces: vec!["gone-workspace".into()],
            agents: vec![
                StoredAgentPin {
                    workspace_lifetime_id: "gone-agent".into(),
                    pane_number: 1,
                },
                StoredAgentPin {
                    workspace_lifetime_id: workspace.lifetime_id.clone(),
                    pane_number: 9,
                },
            ],
        };
        let path = temp_path();
        std::fs::write(&path, serde_json::to_vec(&document).expect("serialize")).expect("write");
        let (store, notice) = PinStore::load(path.clone(), &[workspace], &mut manager);
        assert!(notice.is_none());
        assert_eq!(manager.revision(), 0);
        assert!(manager.pinned_workspace_ids().next().is_none());
        assert!(manager.pinned_pane_ids().next().is_none());
        // The untouched document keeps the unmatched entries until a save.
        assert!(store
            .document()
            .workspaces
            .contains(&"gone-workspace".into()));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn malformed_file_starts_empty_and_stays_until_a_change() {
        let path = temp_path();
        std::fs::write(&path, b"{not json").expect("write garbage");
        let mut manager = PinManager::new();
        let workspaces = vec![Workspace::test_new("alpha")];
        let (store, notice) = PinStore::load(path.clone(), &workspaces, &mut manager);
        let notice = notice.expect("malformed notice");
        assert!(notice.contains("invalid pin state"), "{notice}");
        assert_eq!(manager.revision(), 0);
        assert!(!store.is_write_blocked());
        // The original path keeps the unreadable bytes until a successful save.
        assert_eq!(
            std::fs::read(&path).expect("original pins.json"),
            b"{not json"
        );
        assert!(
            !std::fs::read_dir(path.parent().expect("temp dir"))
                .expect("list temp dir")
                .flatten()
                .any(|entry| { entry.file_name().to_string_lossy().contains(".corrupt-") }),
            "malformed pins.json is not renamed"
        );

        // A pin change overwrites that same path.
        let mut blocked = store.clone();
        blocked.write_blocked = true;
        let staged = blocked.staged(&manager, &workspaces).expect("stage empty");
        assert!(!staged.is_write_blocked());
        staged.save().expect("save after change");
        assert_ne!(
            std::fs::read(&path).expect("rewritten pins.json"),
            b"{not json"
        );
        let (reloaded, notice) = PinStore::load(path.clone(), &workspaces, &mut manager);
        assert!(notice.is_none());
        assert!(reloaded.document().workspaces.is_empty());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn oversized_files_stay_until_a_successful_save() {
        let path = temp_path();
        let blob = "x".repeat(MAX_STORE_BYTES + 1);
        std::fs::write(&path, blob.as_bytes()).expect("write oversized");
        let mut manager = PinManager::new();
        let workspaces = vec![Workspace::test_new("alpha")];
        let (store, notice) = PinStore::load(path.clone(), &workspaces, &mut manager);
        let notice = notice.expect("size notice");
        assert!(notice.contains("storage limit"), "{notice}");
        assert_eq!(manager.revision(), 0);
        assert!(!store.is_write_blocked());
        assert_eq!(
            std::fs::read(&path).expect("original oversized file"),
            blob.as_bytes()
        );
        manager.set_workspace(&workspaces[0].id, true);
        let staged = store.staged(&manager, &workspaces).expect("stage");
        staged.save().expect("overwrite oversized file");
        let rewritten = std::fs::read(&path).expect("rewritten pins.json");
        assert!(rewritten.len() <= MAX_STORE_BYTES);
        assert_ne!(rewritten, blob.as_bytes());
        let _ = std::fs::remove_file(path);
    }

    #[cfg(unix)]
    #[test]
    fn startup_read_error_does_not_block_a_later_save() {
        use std::os::unix::fs::PermissionsExt;

        let path = temp_path();
        std::fs::write(&path, b"{\"version\":1}").expect("write unreadable");
        let mut blocked = std::fs::metadata(&path).expect("metadata").permissions();
        blocked.set_mode(0o000);
        std::fs::set_permissions(&path, blocked).expect("deny reads");
        let mut manager = PinManager::new();
        let workspaces = vec![Workspace::test_new("alpha")];
        let (store, notice) = PinStore::load(path.clone(), &workspaces, &mut manager);
        let notice = notice.expect("read error notice");
        assert!(notice.contains("failed to read pin state"), "{notice}");
        assert!(!store.is_write_blocked());
        assert_eq!(manager.revision(), 0);

        let mut readable = std::fs::metadata(&path).expect("metadata").permissions();
        readable.set_mode(0o600);
        std::fs::set_permissions(&path, readable).expect("restore reads");
        assert!(manager.set_workspace(&workspaces[0].id, true));
        let staged = store
            .staged(&manager, &workspaces)
            .expect("stage after read error");
        assert!(!staged.is_write_blocked());
        staged.save().expect("save after the path is writable");
        let saved = std::fs::read_to_string(&path).expect("saved pins.json");
        assert!(saved.contains(&workspaces[0].lifetime_id));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn documents_beyond_entry_and_shape_bounds_are_invalid() {
        let mut document = StoreDocument {
            version: 2,
            workspaces: vec![],
            agents: vec![],
        };
        assert!(validate_document(&document).is_err());
        document.version = STORE_VERSION;
        document.workspaces = vec!["same".into(), "same".into()];
        assert!(validate_document(&document).is_err());
        document.workspaces = vec![String::new()];
        assert!(validate_document(&document).is_err());
        document.workspaces = (0..MAX_WORKSPACE_PINS + 1)
            .map(|index| format!("lifetime-{index}"))
            .collect();
        assert!(validate_document(&document).is_err());
        document.workspaces = vec![];
        document.agents = vec![StoredAgentPin {
            workspace_lifetime_id: "a".into(),
            pane_number: 1,
        }];
        assert!(validate_document(&document).is_ok());
        document.agents.push(StoredAgentPin {
            workspace_lifetime_id: "a".into(),
            pane_number: 1,
        });
        assert!(validate_document(&document).is_err());
        document.agents[1].pane_number = 0;
        assert!(validate_document(&document).is_err());
        document.agents = (0..MAX_AGENT_PINS + 1)
            .map(|index| StoredAgentPin {
                workspace_lifetime_id: format!("lifetime-{index}"),
                pane_number: 1,
            })
            .collect();
        assert!(validate_document(&document).is_err());
    }

    #[test]
    fn staged_document_tracks_a_pane_moved_to_another_workspace() {
        let mut workspaces = vec![
            workspace_with_two_panes("source"),
            workspace_with_two_panes("destination"),
        ];
        let pane_id = pane_with_public_number(&workspaces[0], 2);
        let mut manager = PinManager::new();
        manager.restore_pane(pane_id);
        let store = PinStore::empty(temp_path());
        let before = store
            .staged(&manager, &workspaces)
            .expect("stage before move");

        // Cross-workspace moves re-register the pane under the destination's
        // next public number and drop the source registration.
        let moved_tab = workspaces[0].tabs.remove(0);
        assert!(moved_tab.panes.contains_key(&pane_id));
        workspaces[0].public_pane_numbers.remove(&pane_id);
        let new_number = workspaces[1].next_public_pane_number;
        workspaces[1]
            .public_pane_numbers
            .insert(pane_id, new_number);
        workspaces[1].tabs.push(moved_tab);
        let after = store
            .staged(&manager, &workspaces)
            .expect("stage after move");

        assert_ne!(before.document(), after.document());
        assert_eq!(before.document().agents.len(), 1);
        assert_eq!(after.document().agents.len(), 1);
        assert_ne!(
            before.document().agents[0].workspace_lifetime_id,
            after.document().agents[0].workspace_lifetime_id
        );
        assert_ne!(
            before.document().agents[0].pane_number,
            after.document().agents[0].pane_number
        );
    }
}
