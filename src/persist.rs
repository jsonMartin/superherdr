//! Session persistence — save/restore workspaces, layouts, and working directories.
//!
//! Stored at `~/.config/herdr/session.json`.
//! Optional pane screen history is stored separately at `session-history.json`.
//! Installed plugins are persisted separately at `plugins.json`.

mod io;
pub mod plugin_registry;
mod restore;
mod snapshot;
mod writer;

<<<<<<< HEAD
pub(crate) use self::io::{
    checkpoint_session_pair, clear, clear_history, load, load_history, save,
};
=======
pub use self::io::{clear_history, load, load_history};
>>>>>>> 065ef9d6a531c49fb8bee7e818ef837065b21ee9
pub use self::restore::restore;
#[cfg(unix)]
pub use self::restore::{handoff_pane_aliases, restore_handoff};
pub use self::snapshot::{
    capture, capture_history, DirectionSnapshot, LayoutSnapshot, SessionHistorySnapshot,
    SessionSnapshot, TabSnapshot, WorkspaceSnapshot,
};
pub(crate) use self::writer::SessionWriter;
