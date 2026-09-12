use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::common::AgentStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceCreateParams {
    /// Workspace whose focused pane supplies the `follow` cwd policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default)]
    pub focus: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceCloseParams {
    pub workspace_id: String,
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub close_group: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceRenameParams {
    pub workspace_id: String,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceMoveParams {
    pub workspace_id: String,
    pub insert_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceMoveBlockParams {
    pub workspace_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before_workspace_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceReportMetadataParams {
    pub workspace_id: String,
    pub source: String,
    #[schemars(schema_with = "super::common::metadata_token_patch_schema")]
    pub tokens: HashMap<String, Option<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1, max = 86_400_000))]
    pub ttl_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceInfo {
    pub workspace_id: String,
    pub number: usize,
    pub label: String,
    pub focused: bool,
    pub pane_count: usize,
    pub tab_count: usize,
    pub active_tab_id: String,
    pub agent_status: AgentStatus,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[schemars(schema_with = "super::common::metadata_token_values_schema")]
    pub tokens: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree: Option<WorkspaceWorktreeInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceWorktreeInfo {
    pub repo_key: String,
    pub repo_name: String,
    pub repo_root: String,
    pub checkout_path: String,
    pub is_linked_worktree: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceSnoozeParams {
    pub workspace_id: String,
    pub boot_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceWakeParams {
    pub boot_id: String,
    pub expected_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SnoozeResetParams {
    pub boot_id: String,
    pub expected_revision: u64,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ProjectSnoozeParams {
    pub workspace_id: String,
    pub project_key: String,
    pub boot_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deadline_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ProjectWakeParams {
    pub project_key: String,
    pub boot_id: String,
    pub expected_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceSnoozeRecord {
    pub workspace_id: String,
    pub boot_id: String,
    pub deadline_unix_ms: i64,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ProjectSnoozeRecord {
    pub project_key: String,
    pub boot_id: String,
    pub deadline_unix_ms: i64,
    pub revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WorkspaceSnoozeState {
    pub boot_id: String,
    pub revision: u64,
    pub records: Vec<WorkspaceSnoozeRecord>,
    #[serde(default)]
    pub project_records: Vec<ProjectSnoozeRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persistence: Option<SnoozePersistenceInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SnoozePersistenceInfo {
    pub records: Vec<SnoozeStoredRecordInfo>,
    pub notice: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SnoozeStoredRecordInfo {
    pub record_id: String,
    pub scope: String,
    pub label: String,
    pub project_label: Option<String>,
    pub created_unix_ms: i64,
    pub deadline_unix_ms: i64,
    pub available: bool,
    pub workspace_id: Option<String>,
    pub project_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SnoozeRecordWakeParams {
    pub record_id: String,
    pub boot_id: String,
    pub expected_revision: u64,
}
