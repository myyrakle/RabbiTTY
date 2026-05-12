//! SFTP client worker — operates over an already-established russh channel.
//!
//! The caller opens a russh `Channel<Msg>` on the active SSH session and
//! requests the `sftp` subsystem; this module wraps the resulting stream with
//! `russh_sftp::client::SftpSession` and translates GUI commands into protocol
//! calls, emitting progress and result events back over mpsc channels.
//!
//! Phase 1 only defines the protocol layer — the GUI integration lives in a
//! later phase, so the items below are unused for now.
#![allow(dead_code)]

use russh::ChannelMsg;
use russh::client::Msg;
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::FileType;
use std::path::PathBuf;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

const TRANSFER_CHUNK: usize = 32 * 1024;

#[derive(Debug, Clone)]
pub enum Command {
    List(String),
    Mkdir(String),
    Rename { from: String, to: String },
    Delete { path: String, is_dir: bool },
    Upload { local: PathBuf, remote: String },
    Download { remote: String, local: PathBuf },
    Cancel,
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub size: u64,
    pub mtime: Option<i64>,
    pub mode: Option<u32>,
    pub is_dir: bool,
    pub is_symlink: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Listed {
        path: String,
        entries: Vec<Entry>,
    },
    TransferStarted {
        path: String,
        total: u64,
    },
    TransferProgress {
        path: String,
        transferred: u64,
        total: u64,
    },
    TransferFinished {
        path: String,
    },
    Mutated {
        path: String,
    },
    Error {
        message: String,
    },
    Closed,
}

pub struct SftpHandle {
    pub tx: mpsc::UnboundedSender<Command>,
    pub rx: mpsc::UnboundedReceiver<Event>,
}

/// Run an SFTP worker over the supplied russh channel.
///
/// The channel must already have the `sftp` subsystem requested. Returns a
/// `SftpHandle` whose `tx` accepts commands and whose `rx` receives events.
pub async fn spawn_worker(channel: russh::Channel<Msg>) -> Result<SftpHandle, String> {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Command>();
    let (evt_tx, evt_rx) = mpsc::unbounded_channel::<Event>();

    let sftp = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| format!("failed to start SFTP session: {e}"))?;

    tokio::spawn(run_worker(sftp, cmd_rx, evt_tx));

    Ok(SftpHandle {
        tx: cmd_tx,
        rx: evt_rx,
    })
}

async fn run_worker(
    sftp: SftpSession,
    mut cmd_rx: mpsc::UnboundedReceiver<Command>,
    evt_tx: mpsc::UnboundedSender<Event>,
) {
    let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            Command::Shutdown => break,
            Command::Cancel => {
                cancelled.store(true, std::sync::atomic::Ordering::SeqCst);
            }
            Command::List(path) => match list_dir(&sftp, &path).await {
                Ok(entries) => {
                    let _ = evt_tx.send(Event::Listed {
                        path: path.clone(),
                        entries,
                    });
                }
                Err(e) => {
                    let _ = evt_tx.send(Event::Error {
                        message: format!("list {path}: {e}"),
                    });
                }
            },
            Command::Mkdir(path) => match sftp.create_dir(&path).await {
                Ok(()) => {
                    let _ = evt_tx.send(Event::Mutated { path });
                }
                Err(e) => {
                    let _ = evt_tx.send(Event::Error {
                        message: format!("mkdir {path}: {e}"),
                    });
                }
            },
            Command::Rename { from, to } => match sftp.rename(&from, &to).await {
                Ok(()) => {
                    let _ = evt_tx.send(Event::Mutated { path: to });
                }
                Err(e) => {
                    let _ = evt_tx.send(Event::Error {
                        message: format!("rename {from} -> {to}: {e}"),
                    });
                }
            },
            Command::Delete { path, is_dir } => {
                let result = if is_dir {
                    sftp.remove_dir(&path).await
                } else {
                    sftp.remove_file(&path).await
                };
                match result {
                    Ok(()) => {
                        let _ = evt_tx.send(Event::Mutated { path });
                    }
                    Err(e) => {
                        let _ = evt_tx.send(Event::Error {
                            message: format!("delete {path}: {e}"),
                        });
                    }
                }
            }
            Command::Upload { local, remote } => {
                cancelled.store(false, std::sync::atomic::Ordering::SeqCst);
                if let Err(e) = upload(&sftp, &local, &remote, &evt_tx, &cancelled).await {
                    let _ = evt_tx.send(Event::Error {
                        message: format!("upload {}: {e}", remote),
                    });
                }
            }
            Command::Download { remote, local } => {
                cancelled.store(false, std::sync::atomic::Ordering::SeqCst);
                if let Err(e) = download(&sftp, &remote, &local, &evt_tx, &cancelled).await {
                    let _ = evt_tx.send(Event::Error {
                        message: format!("download {remote}: {e}"),
                    });
                }
            }
        }
    }

    let _ = evt_tx.send(Event::Closed);
}

async fn list_dir(sftp: &SftpSession, path: &str) -> Result<Vec<Entry>, String> {
    let dir = sftp.read_dir(path).await.map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for entry in dir {
        let metadata = entry.metadata();
        let is_dir = matches!(metadata.file_type(), FileType::Dir);
        let is_symlink = matches!(metadata.file_type(), FileType::Symlink);
        out.push(Entry {
            name: entry.file_name(),
            size: metadata.size.unwrap_or(0),
            mtime: metadata.mtime.map(|v| v as i64),
            mode: metadata.permissions,
            is_dir,
            is_symlink,
        });
    }
    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a
            .name
            .to_ascii_lowercase()
            .cmp(&b.name.to_ascii_lowercase()),
    });
    Ok(out)
}

async fn upload(
    sftp: &SftpSession,
    local: &std::path::Path,
    remote: &str,
    evt_tx: &mpsc::UnboundedSender<Event>,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
    let mut local_file = tokio::fs::File::open(local)
        .await
        .map_err(|e| format!("open local: {e}"))?;
    let total = local_file.metadata().await.map(|m| m.len()).unwrap_or(0);

    let mut remote_file = sftp
        .create(remote)
        .await
        .map_err(|e| format!("create remote: {e}"))?;

    let _ = evt_tx.send(Event::TransferStarted {
        path: remote.to_string(),
        total,
    });

    let mut buf = vec![0u8; TRANSFER_CHUNK];
    let mut transferred = 0u64;
    loop {
        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("cancelled".into());
        }
        let n = local_file
            .read(&mut buf)
            .await
            .map_err(|e| format!("read local: {e}"))?;
        if n == 0 {
            break;
        }
        remote_file
            .write_all(&buf[..n])
            .await
            .map_err(|e| format!("write remote: {e}"))?;
        transferred += n as u64;
        let _ = evt_tx.send(Event::TransferProgress {
            path: remote.to_string(),
            transferred,
            total,
        });
    }
    remote_file
        .shutdown()
        .await
        .map_err(|e| format!("close remote: {e}"))?;

    let _ = evt_tx.send(Event::TransferFinished {
        path: remote.to_string(),
    });
    Ok(())
}

async fn download(
    sftp: &SftpSession,
    remote: &str,
    local: &std::path::Path,
    evt_tx: &mpsc::UnboundedSender<Event>,
    cancelled: &std::sync::atomic::AtomicBool,
) -> Result<(), String> {
    let metadata = sftp
        .metadata(remote)
        .await
        .map_err(|e| format!("stat remote: {e}"))?;
    let total = metadata.size.unwrap_or(0);

    let mut remote_file = sftp
        .open(remote)
        .await
        .map_err(|e| format!("open remote: {e}"))?;
    let mut local_file = tokio::fs::File::create(local)
        .await
        .map_err(|e| format!("create local: {e}"))?;

    let _ = evt_tx.send(Event::TransferStarted {
        path: remote.to_string(),
        total,
    });

    let mut buf = vec![0u8; TRANSFER_CHUNK];
    let mut transferred = 0u64;
    loop {
        if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("cancelled".into());
        }
        let n = remote_file
            .read(&mut buf)
            .await
            .map_err(|e| format!("read remote: {e}"))?;
        if n == 0 {
            break;
        }
        local_file
            .write_all(&buf[..n])
            .await
            .map_err(|e| format!("write local: {e}"))?;
        transferred += n as u64;
        let _ = evt_tx.send(Event::TransferProgress {
            path: remote.to_string(),
            transferred,
            total,
        });
    }
    local_file
        .flush()
        .await
        .map_err(|e| format!("flush local: {e}"))?;

    let _ = evt_tx.send(Event::TransferFinished {
        path: remote.to_string(),
    });
    Ok(())
}

/// Open the SFTP subsystem on a russh channel.
///
/// Caller obtains the channel via `session.channel_open_session().await?`,
/// then passes it here. The returned channel is ready to be wrapped by
/// `spawn_worker`.
pub async fn request_sftp(channel: &mut russh::Channel<Msg>) -> Result<(), String> {
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| format!("request sftp subsystem: {e}"))?;
    // Drain any reply messages until the subsystem ack settles. russh's
    // `request_subsystem(want_reply=true)` already waits for SUCCESS/FAILURE,
    // but some servers send no banner; we just return now.
    Ok(())
}

/// Silence unused warning for `ChannelMsg` while integration is pending.
#[allow(dead_code)]
fn _bind_channel_msg(_msg: ChannelMsg) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_sort_dirs_first_then_alpha() {
        fn entry(name: &str, is_dir: bool) -> Entry {
            Entry {
                name: name.into(),
                size: 0,
                mtime: None,
                mode: None,
                is_dir,
                is_symlink: false,
            }
        }
        let mut entries = [
            entry("zfile", false),
            entry("Bdir", true),
            entry("afile", false),
            entry("adir", true),
        ];
        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a
                .name
                .to_ascii_lowercase()
                .cmp(&b.name.to_ascii_lowercase()),
        });
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["adir", "Bdir", "afile", "zfile"]);
    }
}
