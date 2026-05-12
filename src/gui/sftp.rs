//! GUI state for the SFTP drawer attached to an SSH terminal tab.
//!
//! Phase 2a defines the persistent state held alongside each `TerminalTab`.
//! The drawer rendering, lifecycle, and protocol wiring live in subsequent
//! phases — items below are unused until then.
#![allow(dead_code)]

use crate::ssh::sftp;
use tokio::sync::mpsc;

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

    /// Returns true when the worker channel is open and ready to accept
    /// further commands.
    pub fn is_connected(&self) -> bool {
        self.command_tx.is_some()
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
