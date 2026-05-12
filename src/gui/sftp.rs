//! GUI state for the SFTP drawer attached to an SSH terminal tab.

use crate::ssh::sftp;
use iced::futures::channel::mpsc;

/// Default initial path used when the drawer first opens.
pub const DEFAULT_PATH: &str = ".";

/// Default fraction of tab height the drawer occupies when first opened.
pub const DEFAULT_HEIGHT_RATIO: f32 = 0.45;

#[derive(Debug, Clone)]
pub struct TransferRow {
    pub path: String,
    pub transferred: u64,
    pub total: u64,
    pub finished: bool,
}

#[derive(Default, Debug)]
pub struct SftpDrawerState {
    /// True once the user has toggled the drawer open. Renders the panel.
    pub open: bool,
    /// True while the initial `open_sftp` future is in flight.
    pub opening: bool,
    /// True while a `List` command is awaiting a response.
    pub loading: bool,
    /// Last error message from the worker (rendered in the drawer when set).
    pub error: Option<String>,
    /// Remote path currently being shown.
    pub current_path: String,
    /// Entries returned by the most recent `List`.
    pub entries: Vec<sftp::Entry>,
    /// In-flight or completed transfers, newest first.
    pub transfers: Vec<TransferRow>,
    /// Command sender into the worker. `None` until the channel opens.
    pub command_tx: Option<mpsc::UnboundedSender<sftp::Command>>,
    /// Fraction of available height the drawer should occupy (0.0..=1.0).
    pub height_ratio: f32,
}

/// Best-effort parent path for the SFTP browser. Returns `None` only when
/// the current path is already the filesystem root.
pub fn parent_path(path: &str) -> Option<String> {
    match path {
        "/" => None,
        "" | "." => Some("..".to_string()),
        _ if path == ".." || path.ends_with("/..") => Some(format!("{path}/..")),
        _ => {
            let trimmed = path.trim_end_matches('/');
            match trimmed.rfind('/') {
                None => Some(".".to_string()),
                Some(0) => Some("/".to_string()),
                Some(i) => Some(trimmed[..i].to_string()),
            }
        }
    }
}

/// Join a directory entry name onto the current remote path.
pub fn join_path(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_string()
    } else if base == "/" {
        format!("/{name}")
    } else {
        format!("{}/{name}", base.trim_end_matches('/'))
    }
}

impl SftpDrawerState {
    pub fn new() -> Self {
        Self {
            open: false,
            opening: false,
            loading: false,
            error: None,
            current_path: DEFAULT_PATH.to_string(),
            entries: Vec::new(),
            transfers: Vec::new(),
            command_tx: None,
            height_ratio: DEFAULT_HEIGHT_RATIO,
        }
    }

    /// Reset the drawer's volatile state when the underlying SSH session
    /// drops or the tab is closed.
    pub fn reset(&mut self) {
        self.open = false;
        self.opening = false;
        self.loading = false;
        self.error = None;
        self.entries.clear();
        self.transfers.clear();
        self.command_tx = None;
    }
}

#[cfg(test)]
mod tests {
    use super::{join_path, parent_path};

    #[test]
    fn parent_handles_common_cases() {
        assert_eq!(parent_path("/"), None);
        assert_eq!(parent_path("."), Some("..".to_string()));
        assert_eq!(parent_path(".."), Some("../..".to_string()));
        assert_eq!(parent_path("../.."), Some("../../..".to_string()));
        assert_eq!(parent_path("/home/me"), Some("/home".to_string()));
        assert_eq!(parent_path("/home"), Some("/".to_string()));
        assert_eq!(parent_path("/home/me/docs/"), Some("/home/me".to_string()));
        assert_eq!(parent_path("foo"), Some(".".to_string()));
        assert_eq!(parent_path("./foo"), Some(".".to_string()));
        assert_eq!(parent_path("../foo"), Some("..".to_string()));
    }

    #[test]
    fn join_handles_common_cases() {
        assert_eq!(join_path("/", "foo"), "/foo");
        assert_eq!(join_path("/home", "me"), "/home/me");
        assert_eq!(join_path("/home/", "me"), "/home/me");
        assert_eq!(join_path("", "foo"), "foo");
    }
}
