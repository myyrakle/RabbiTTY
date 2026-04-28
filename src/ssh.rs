use crate::config::SshProfile;
use crate::session::OutputEvent;
use async_trait::async_trait;
use iced::futures::channel::mpsc as futures_mpsc;
use russh::keys::*;
use russh::*;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc as tokio_mpsc;

const SSH_BADGE: &str = "\x1b[1;97;46m SSH \x1b[0m";

struct SshHandler {
    fingerprint_tx: Option<tokio::sync::oneshot::Sender<String>>,
}

#[async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        if let Some(tx) = self.fingerprint_tx.take() {
            let fp = server_public_key
                .fingerprint(ssh_key::HashAlg::Sha256)
                .to_string();
            let _ = tx.send(fp);
        }
        // TODO: proper host key verification against known_hosts
        Ok(true)
    }
}

/// Sync Write → async tokio channel bridge
struct SshWriter {
    tx: tokio_mpsc::UnboundedSender<Vec<u8>>,
}

impl Write for SshWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.tx.send(buf.to_vec()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "SSH channel closed")
        })?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct SshSessionHandle {
    pub writer: Arc<Mutex<Box<dyn Write + Send>>>,
    pub resize_tx: tokio_mpsc::UnboundedSender<(u16, u16)>,
}

pub fn spawn_ssh_session(
    profile: SshProfile,
    tab_id: u64,
    rows: u16,
    cols: u16,
    output_tx: futures_mpsc::UnboundedSender<OutputEvent>,
) -> SshSessionHandle {
    let (write_tx, write_rx) = tokio_mpsc::unbounded_channel::<Vec<u8>>();
    let (resize_tx, resize_rx) = tokio_mpsc::unbounded_channel::<(u16, u16)>();

    let writer: Arc<Mutex<Box<dyn Write + Send>>> =
        Arc::new(Mutex::new(Box::new(SshWriter { tx: write_tx })));

    tokio::spawn(async move {
        let mut otx = output_tx;
        if let Err(e) = ssh_task(profile, tab_id, rows, cols, write_rx, resize_rx, &mut otx).await {
            let msg = format!("\r\n  {SSH_BADGE}  \x1b[1;31m{e}\x1b[0m\r\n");
            let _ = otx.unbounded_send(OutputEvent::Data {
                tab_id,
                bytes: msg.into_bytes(),
            });
        }
        let _ = otx.unbounded_send(OutputEvent::Closed { tab_id });
    });

    SshSessionHandle { writer, resize_tx }
}

fn send_status(output_tx: &mut futures_mpsc::UnboundedSender<OutputEvent>, tab_id: u64, msg: &str) {
    let _ = output_tx.unbounded_send(OutputEvent::Data {
        tab_id,
        bytes: msg.as_bytes().to_vec(),
    });
}

async fn ssh_task(
    profile: SshProfile,
    tab_id: u64,
    rows: u16,
    cols: u16,
    mut write_rx: tokio_mpsc::UnboundedReceiver<Vec<u8>>,
    mut resize_rx: tokio_mpsc::UnboundedReceiver<(u16, u16)>,
    output_tx: &mut futures_mpsc::UnboundedSender<OutputEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // --- Status: Connecting ---
    let dest = if profile.user.is_empty() {
        profile.host.to_string()
    } else {
        format!("{}@{}", profile.user, profile.host)
    };
    let port_info = if profile.port != 22 {
        format!(":{}", profile.port)
    } else {
        String::new()
    };
    send_status(
        output_tx,
        tab_id,
        &format!("\r\n  {SSH_BADGE}  \x1b[1mConnecting to {dest}{port_info}\x1b[0m\r\n"),
    );

    // Auth method hint
    if let Some(ref identity) = profile.identity_file {
        send_status(
            output_tx,
            tab_id,
            &format!("         \x1b[36mUsing private key from  \x1b[1;4m{identity}\x1b[0m\r\n"),
        );
    } else if profile.password.is_some() {
        send_status(
            output_tx,
            tab_id,
            "         \x1b[36mUsing saved password\x1b[0m\r\n",
        );
    }

    // --- TCP + SSH handshake ---
    let config = Arc::new(client::Config {
        inactivity_timeout: Some(std::time::Duration::from_secs(30)),
        ..<_>::default()
    });

    let (fp_tx, fp_rx) = tokio::sync::oneshot::channel();
    let handler = SshHandler {
        fingerprint_tx: Some(fp_tx),
    };

    let addr = format!("{}:{}", profile.host, profile.port);
    let mut session = client::connect(config, &*addr, handler).await?;

    // Display host key fingerprint
    if let Ok(fp) = fp_rx.await {
        send_status(
            output_tx,
            tab_id,
            "         \x1b[36mHost key fingerprint:\x1b[0m\r\n",
        );
        send_status(
            output_tx,
            tab_id,
            &format!("         \x1b[1;46;97m {fp} \x1b[0m\r\n"),
        );
    }

    // --- Authenticate ---
    send_status(
        output_tx,
        tab_id,
        &format!("  {SSH_BADGE}  \x1b[33mAuthenticating...\x1b[0m\r\n"),
    );

    let user = if profile.user.is_empty() {
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "root".into())
    } else {
        profile.user.clone()
    };

    let authenticated = if let Some(ref identity_path) = profile.identity_file {
        let expanded = if identity_path.starts_with("~/") {
            dirs::home_dir()
                .map(|h| h.join(&identity_path[2..]).to_string_lossy().to_string())
                .unwrap_or_else(|| identity_path.clone())
        } else {
            identity_path.clone()
        };
        let key_pair = load_secret_key(&expanded, None)?;
        session
            .authenticate_publickey(&user, Arc::new(key_pair))
            .await?
    } else if let Some(ref password) = profile.password {
        session.authenticate_password(&user, password).await?
    } else {
        // Try default keys
        try_default_keys(&mut session, &user).await
    };

    if !authenticated {
        return Err("Authentication failed".into());
    }

    // --- Connected ---
    send_status(
        output_tx,
        tab_id,
        &format!("  {SSH_BADGE}  \x1b[1;32m\u{2713} Connected!\x1b[0m\r\n\r\n"),
    );

    // --- Open channel with PTY + shell ---
    let mut channel = session.channel_open_session().await?;
    channel
        .request_pty(false, "xterm-256color", cols as u32, rows as u32, 0, 0, &[])
        .await?;
    channel.request_shell(false).await?;

    // --- I/O bridge ---
    loop {
        tokio::select! {
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { data }) => {
                        let _ = output_tx.unbounded_send(OutputEvent::Data {
                            tab_id,
                            bytes: data.to_vec(),
                        });
                    }
                    Some(ChannelMsg::Eof)
                    | Some(ChannelMsg::Close)
                    | Some(ChannelMsg::ExitStatus { .. })
                    | None => break,
                    _ => {}
                }
            }
            bytes = write_rx.recv() => {
                match bytes {
                    Some(bytes) => channel.data(&bytes[..]).await?,
                    None => break,
                }
            }
            resize = resize_rx.recv() => {
                match resize {
                    Some((r, c)) => channel.window_change(c as u32, r as u32, 0, 0).await?,
                    None => break,
                }
            }
        }
    }

    Ok(())
}

async fn try_default_keys(session: &mut client::Handle<SshHandler>, user: &str) -> bool {
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return false,
    };
    let candidates = [
        home.join(".ssh/id_ed25519"),
        home.join(".ssh/id_rsa"),
        home.join(".ssh/id_ecdsa"),
    ];
    for path in &candidates {
        if !path.exists() {
            continue;
        }
        let key_pair = match load_secret_key(path, None) {
            Ok(k) => k,
            Err(_) => continue,
        };
        match session
            .authenticate_publickey(user, Arc::new(key_pair))
            .await
        {
            Ok(true) => return true,
            _ => continue,
        }
    }
    false
}
