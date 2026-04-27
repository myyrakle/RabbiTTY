use crate::config::SshProfile;
use crate::session::{LaunchSpec, OutputEvent, Session, SessionError};
use crate::terminal::{CellVisual, Selection, TerminalEngine, TerminalSize, TerminalTheme};
use iced::futures::channel::mpsc;
use iced::keyboard::{Key, Modifiers, key::Named};
use std::borrow::Cow;
use std::fmt::{Display, Formatter};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, Mutex};

pub struct TerminalTab {
    pub id: u64,
    pub title: String,
    #[allow(dead_code)]
    pub shell: ShellKind,
    pub session: TerminalSession,
    pub selection: Option<Selection>,
    engine: TerminalEngine,
    pending_password: Option<String>,
    password_prompt_buf: Vec<u8>,
    ssh_state: Option<SshState>,
}

enum SshState {
    Connecting,
    Authenticating,
}

pub enum TerminalSession {
    Active(Session),
    #[allow(dead_code)]
    Failed(String),
}

impl TerminalTab {
    pub fn from_shell(
        shell: ShellKind,
        columns: usize,
        lines: usize,
        theme: TerminalTheme,
        id: u64,
        output_tx: mpsc::UnboundedSender<OutputEvent>,
    ) -> Self {
        Self::launch(shell, columns, lines, theme, id, output_tx)
    }

    fn launch(
        shell: ShellKind,
        columns: usize,
        lines: usize,
        theme: TerminalTheme,
        id: u64,
        output_tx: mpsc::UnboundedSender<OutputEvent>,
    ) -> Self {
        let size = TerminalSize::new(columns, lines);
        let launch_spec = shell.launch_spec(size);
        let title = shell.title_from_program(&launch_spec.program);
        let (session, writer) = match Session::spawn(launch_spec, id, output_tx) {
            Ok(session) => {
                let writer = session.writer();
                (TerminalSession::Active(session), writer)
            }
            Err(err) => (
                TerminalSession::Failed(err.to_string()),
                Arc::new(Mutex::new(
                    Box::new(std::io::sink()) as Box<dyn Write + Send>
                )),
            ),
        };

        let (pending_password, ssh_state) = if let ShellKind::Ssh(ref profile) = shell {
            (profile.password.clone(), Some(SshState::Connecting))
        } else {
            (None, None)
        };

        let mut engine = TerminalEngine::new(size, 10_000, writer, theme);

        if let ShellKind::Ssh(ref profile) = shell {
            for line in ssh_info_lines(profile) {
                engine.feed_bytes(line.as_bytes());
            }
        }

        Self {
            id,
            title,
            shell,
            session,
            selection: None,
            engine,
            pending_password,
            password_prompt_buf: Vec::new(),
            ssh_state,
        }
    }

    pub fn feed_bytes(&mut self, bytes: &[u8]) {
        // Show "Connected!" on first real output after SSH auth
        if let Some(ref state) = self.ssh_state {
            match state {
                SshState::Connecting if self.pending_password.is_none() => {
                    let msg = format!("  {SSH_BADGE}  \x1b[1;32m✓ Connected!\x1b[0m\r\n\r\n");
                    self.engine.feed_bytes(msg.as_bytes());
                    self.ssh_state = None;
                }
                SshState::Authenticating => {
                    let msg = format!("  {SSH_BADGE}  \x1b[1;32m✓ Connected!\x1b[0m\r\n\r\n");
                    self.engine.feed_bytes(msg.as_bytes());
                    self.ssh_state = None;
                }
                _ => {}
            }
        }

        self.engine.feed_bytes(bytes);
        if let Some(new_title) = self.engine.take_title() {
            self.title = new_title;
        }
        if self.pending_password.is_some() {
            self.check_password_prompt(bytes);
        }
    }

    fn check_password_prompt(&mut self, bytes: &[u8]) {
        // Buffer recent output to detect password prompts across chunk boundaries.
        // Keep only the last 256 bytes to avoid unbounded growth.
        self.password_prompt_buf.extend_from_slice(bytes);
        if self.password_prompt_buf.len() > 256 {
            let start = self.password_prompt_buf.len() - 256;
            self.password_prompt_buf.drain(..start);
        }

        if is_password_prompt(&self.password_prompt_buf)
            && let Some(password) = self.pending_password.take()
        {
            let auth_msg = format!("  {SSH_BADGE}  \x1b[33mAuthenticating...\x1b[0m\r\n");
            self.engine.feed_bytes(auth_msg.as_bytes());
            self.ssh_state = Some(SshState::Authenticating);
            if let TerminalSession::Active(ref session) = self.session {
                let mut payload = password.into_bytes();
                payload.push(b'\n');
                let _ = session.send_bytes(&payload);
            }
            self.password_prompt_buf.clear();
        }
    }

    #[allow(dead_code)]
    pub fn status_text(&self) -> String {
        match &self.session {
            TerminalSession::Active(_) => "Session: live".into(),
            TerminalSession::Failed(err) => format!("Session error: {err}"),
        }
    }

    pub fn render_cells(&self) -> std::sync::Arc<Vec<CellVisual>> {
        self.engine.render_cells()
    }

    pub fn set_theme(&mut self, theme: TerminalTheme) {
        self.engine.set_theme(theme);
    }

    pub fn size(&self) -> TerminalSize {
        self.engine.size()
    }

    #[allow(dead_code)]
    pub fn is_alive(&self) -> bool {
        matches!(&self.session, TerminalSession::Active(_))
    }

    pub fn scroll(&mut self, delta: i32) {
        self.engine.scroll(delta);
    }

    /// Returns (display_offset, total_history_lines).
    pub fn scroll_position(&self) -> (usize, usize) {
        self.engine.scroll_position()
    }

    pub fn selected_text(&self) -> Option<String> {
        let sel = self.selection.as_ref().filter(|s| !s.is_empty())?;
        let cells = self.engine.render_cells();
        let size = self.engine.size();
        let (start, end) = sel.ordered();
        let mut result = String::new();
        for row in start.row..=end.row {
            let col_start = if row == start.row { start.col } else { 0 };
            let col_end = if row == end.row {
                end.col
            } else {
                size.columns.saturating_sub(1)
            };
            for col in col_start..=col_end {
                let idx = row * size.columns + col;
                if let Some(cell) = cells.get(idx) {
                    result.push(cell.ch);
                }
            }
            let trimmed_len = result.trim_end_matches(' ').len();
            result.truncate(trimmed_len);
            if row != end.row {
                result.push('\n');
            }
        }
        let trimmed_len = result.trim_end_matches(' ').len();
        result.truncate(trimmed_len);
        if result.is_empty() {
            None
        } else {
            Some(result)
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    pub fn scroll_to_relative(&mut self, rel: f32) {
        self.engine.scroll_to_relative(rel);
    }

    /// Returns true when the terminal program has enabled mouse reporting.
    pub fn mouse_mode(&self) -> bool {
        self.engine.mouse_mode()
    }

    /// Returns true when the terminal is in the alternate screen buffer.
    pub fn alt_screen(&self) -> bool {
        self.engine.alt_screen()
    }

    /// Send scroll as arrow key sequences (for alt screen without mouse mode).
    pub fn send_scroll_as_arrows(&self, lines: i32) {
        let TerminalSession::Active(session) = &self.session else {
            return;
        };
        let arrow = if lines > 0 { b'A' } else { b'B' }; // Up / Down
        let seq = [b'\x1b', b'[', arrow];
        for _ in 0..lines.unsigned_abs() {
            let _ = session.send_bytes(&seq);
        }
    }

    /// Send a mouse event to the PTY using SGR or legacy encoding.
    pub fn send_mouse_event(&self, button: u8, col: usize, row: usize, pressed: bool) {
        let TerminalSession::Active(session) = &self.session else {
            return;
        };
        // SGR encoding: \x1b[<btn;col;row;M/m  (M=press, m=release)
        // Columns and rows are 1-based in the protocol.
        if self.engine.sgr_mouse() {
            let suffix = if pressed { 'M' } else { 'm' };
            let seq = format!("\x1b[<{};{};{}{}", button, col + 1, row + 1, suffix);
            let _ = session.send_bytes(seq.as_bytes());
        } else {
            // Legacy X10/normal encoding: only sends press, limited to 223 cols/rows
            if pressed {
                let cb = 32 + button;
                let cx = 32 + (col as u8 + 1);
                let cy = 32 + (row as u8 + 1);
                let seq = [b'\x1b', b'[', b'M', cb, cx, cy];
                let _ = session.send_bytes(&seq);
            }
        }
    }

    pub fn resize(&mut self, columns: usize, lines: usize) {
        let new_size = TerminalSize::new(columns, lines);
        self.engine.resize(new_size);

        if let TerminalSession::Active(session) = &mut self.session {
            let _ = session.resize(lines as u16, columns as u16);
        }
    }

    pub fn handle_key(&mut self, key: &Key, modifiers: Modifiers, text: Option<&str>) {
        if let TerminalSession::Active(session) = &self.session
            && let Some(bytes) = self.key_to_bytes(key, modifiers, text)
            && let Err(err) = session.send_bytes(&bytes)
        {
            eprintln!("Failed to send key to session: {err}")
        }
    }

    fn key_to_bytes<'a>(
        &self,
        key: &Key,
        modifiers: Modifiers,
        text: Option<&'a str>,
    ) -> Option<Cow<'a, [u8]>> {
        match key {
            Key::Named(named) => match named {
                Named::Enter => Some(Cow::Borrowed(b"\r")),
                Named::Backspace => Some(Cow::Borrowed(b"\x7f")),
                Named::Tab => {
                    if modifiers.shift() {
                        Some(Cow::Borrowed(b"\x1b[Z"))
                    } else {
                        Some(Cow::Borrowed(b"\t"))
                    }
                }
                Named::Escape => Some(Cow::Borrowed(b"\x1b")),
                Named::ArrowUp => Some(Cow::Borrowed(b"\x1b[A")),
                Named::ArrowDown => Some(Cow::Borrowed(b"\x1b[B")),
                Named::ArrowRight => Some(Cow::Borrowed(b"\x1b[C")),
                Named::ArrowLeft => Some(Cow::Borrowed(b"\x1b[D")),
                Named::Home => Some(Cow::Borrowed(b"\x1b[H")),
                Named::End => Some(Cow::Borrowed(b"\x1b[F")),
                Named::Delete => Some(Cow::Borrowed(b"\x1b[3~")),
                Named::PageUp => Some(Cow::Borrowed(b"\x1b[5~")),
                Named::PageDown => Some(Cow::Borrowed(b"\x1b[6~")),
                Named::Insert => Some(Cow::Borrowed(b"\x1b[2~")),
                Named::F1 => Some(Cow::Borrowed(b"\x1bOP")),
                Named::F2 => Some(Cow::Borrowed(b"\x1bOQ")),
                Named::F3 => Some(Cow::Borrowed(b"\x1bOR")),
                Named::F4 => Some(Cow::Borrowed(b"\x1bOS")),
                Named::F5 => Some(Cow::Borrowed(b"\x1b[15~")),
                Named::F6 => Some(Cow::Borrowed(b"\x1b[17~")),
                Named::F7 => Some(Cow::Borrowed(b"\x1b[18~")),
                Named::F8 => Some(Cow::Borrowed(b"\x1b[19~")),
                Named::F9 => Some(Cow::Borrowed(b"\x1b[20~")),
                Named::F10 => Some(Cow::Borrowed(b"\x1b[21~")),
                Named::F11 => Some(Cow::Borrowed(b"\x1b[23~")),
                Named::F12 => Some(Cow::Borrowed(b"\x1b[24~")),
                Named::Space => {
                    if modifiers.control() {
                        Some(Cow::Borrowed(b"\0"))
                    } else {
                        Some(Cow::Borrowed(b" "))
                    }
                }
                _ => None,
            },

            Key::Character(c) if modifiers.control() => c.chars().next().and_then(|ch| {
                let upper = ch.to_ascii_uppercase();
                if upper.is_ascii_alphabetic() {
                    Some(Cow::Owned(vec![(upper as u8) - b'A' + 1]))
                } else {
                    None
                }
            }),

            Key::Character(_) => text.map(|t| Cow::Borrowed(t.as_bytes())),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ShellKind {
    Default,
    Shell { name: String, path: String },
    Ssh(SshProfile),
}

impl ShellKind {
    fn launch_spec(&self, size: TerminalSize) -> LaunchSpec {
        let (program, args) = match self {
            ShellKind::Ssh(profile) => return profile.launch_spec(size),
            ShellKind::Default => resolve_default_shell(),
            ShellKind::Shell { path, .. } => (path.clone(), vec!["-l".to_string()]),
        };

        let env = title_env_for_shell(&program);

        LaunchSpec {
            program,
            args,
            env,
            rows: size.lines as u16,
            cols: size.columns as u16,
        }
    }

    fn title_from_program(&self, program: &str) -> String {
        if let ShellKind::Ssh(profile) = self {
            return profile.tab_title();
        }

        Path::new(program)
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("shell")
            .to_string()
    }

    pub fn display_name(&self) -> String {
        match self {
            ShellKind::Default => default_shell_display_name(),
            ShellKind::Shell { name, .. } => name.clone(),
            ShellKind::Ssh(profile) => format!("SSH: {}", profile.tab_title()),
        }
    }
}

impl SshProfile {
    fn launch_spec(&self, size: TerminalSize) -> LaunchSpec {
        let mut args = Vec::new();

        if self.port != 22 {
            args.push("-p".to_string());
            args.push(self.port.to_string());
        }

        if let Some(ref identity) = self.identity_file {
            let expanded = if identity.starts_with("~/") {
                dirs::home_dir()
                    .map(|h| h.join(&identity[2..]).to_string_lossy().to_string())
                    .unwrap_or_else(|| identity.clone())
            } else {
                identity.clone()
            };
            args.push("-i".to_string());
            args.push(expanded);
        }

        let destination = if self.user.is_empty() {
            self.host.clone()
        } else {
            format!("{}@{}", self.user, self.host)
        };
        args.push(destination);

        LaunchSpec {
            program: "ssh".to_string(),
            args,
            env: vec![("TERM".to_string(), "xterm-256color".to_string())],
            rows: size.lines as u16,
            cols: size.columns as u16,
        }
    }

    fn tab_title(&self) -> String {
        if self.name.is_empty() {
            if self.user.is_empty() {
                self.host.clone()
            } else {
                format!("{}@{}", self.user, self.host)
            }
        } else {
            self.name.clone()
        }
    }
}

fn default_shell_display_name() -> String {
    use std::sync::OnceLock;
    static CACHED: OnceLock<String> = OnceLock::new();
    CACHED
        .get_or_init(|| {
            let (program, _) = resolve_default_shell();
            let name = Path::new(&program)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("shell");
            format!("{name} (Default)")
        })
        .clone()
}

fn title_env_for_shell(program: &str) -> Vec<(String, String)> {
    let name = Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");

    match name {
        "bash" => vec![(
            "PROMPT_COMMAND".to_string(),
            r#"printf "\033]0;%s\007" "${PWD/#$HOME/~}""#.to_string(),
        )],
        _ => vec![],
    }
}

fn resolve_default_shell() -> (String, Vec<String>) {
    #[cfg(target_family = "unix")]
    {
        if let Ok(shell) = std::env::var("SHELL") {
            let shell = shell.trim();
            if !shell.is_empty() {
                return (shell.to_string(), vec!["-l".to_string()]);
            }
        }

        const FALLBACKS: &[&str] = &["zsh", "bash", "fish", "sh"];
        if let Some(candidate) = FALLBACKS.iter().find(|c| command_exists(c)) {
            return ((*candidate).to_string(), vec!["-l".to_string()]);
        }

        ("sh".to_string(), vec!["-l".to_string()])
    }

    #[cfg(target_family = "windows")]
    {
        (
            "powershell".to_string(),
            vec![
                "-NoLogo".to_string(),
                "-ExecutionPolicy".to_string(),
                "Bypass".to_string(),
            ],
        )
    }
}

#[cfg(target_family = "unix")]
fn command_exists(program: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {program} >/dev/null 2>&1"))
        .status()
        .is_ok_and(|status| status.success())
}

/// Discover available shells from `/etc/shells` (Unix) or known Windows shells.
pub fn discover_available_shells() -> Vec<ShellKind> {
    let mut shells = vec![ShellKind::Default];

    #[cfg(target_family = "unix")]
    {
        let default_path = std::env::var("SHELL").unwrap_or_default();
        let default_path = default_path.trim();

        let etc_shells = std::fs::read_to_string("/etc/shells")
            .or_else(|_| std::fs::read_to_string("/usr/share/defaults/etc/shells"))
            .unwrap_or_default();

        for line in etc_shells.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // Skip if same as default shell (already listed)
            if line == default_path {
                continue;
            }
            // Skip non-interactive shells
            let name = Path::new(line)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            if name.is_empty() || matches!(name, "nologin" | "false") {
                continue;
            }
            shells.push(ShellKind::Shell {
                name: name.to_string(),
                path: line.to_string(),
            });
        }
    }

    #[cfg(target_family = "windows")]
    {
        shells.push(ShellKind::Shell {
            name: "cmd".to_string(),
            path: "cmd".to_string(),
        });
    }

    shells
}

impl Display for ShellKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ShellKind::Default => write!(f, "shell"),
            ShellKind::Shell { name, .. } => write!(f, "{name}"),
            ShellKind::Ssh(profile) => write!(f, "ssh: {}", profile.tab_title()),
        }
    }
}

impl Display for SessionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionError::Spawn(err) => write!(f, "{err}"),
            SessionError::Io(err) => write!(f, "{err}"),
        }
    }
}

/// ANSI badge: white-on-teal " SSH " label.
const SSH_BADGE: &str = "\x1b[1;97;46m SSH \x1b[0m";

/// Build the informational lines shown when an SSH tab is opened.
fn ssh_info_lines(profile: &SshProfile) -> Vec<String> {
    let mut lines = Vec::new();

    // Connection target
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
    lines.push(format!(
        "\r\n  {SSH_BADGE}  \x1b[1mConnecting to {dest}{port_info}\x1b[0m\r\n"
    ));

    // Auth method
    if let Some(ref identity) = profile.identity_file {
        lines.push(format!(
            "         \x1b[36mUsing private key from  \x1b[1;4m{identity}\x1b[0m\r\n"
        ));
    } else if profile.password.is_some() {
        lines.push("         \x1b[36mUsing saved password\x1b[0m\r\n".to_string());
    }

    lines.push("\r\n".to_string());
    lines
}

fn is_password_prompt(buf: &[u8]) -> bool {
    let haystack = String::from_utf8_lossy(buf).to_ascii_lowercase();
    haystack.contains("password:")
        || haystack.contains("password for")
        || haystack.contains("'s password:")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_standard_password_prompts() {
        assert!(is_password_prompt(b"Password:"));
        assert!(is_password_prompt(b"password: "));
        assert!(is_password_prompt(b"admin@host's password:"));
        assert!(is_password_prompt(b"Password for admin:"));
        assert!(is_password_prompt(b"PASSWORD:"));
    }

    #[test]
    fn rejects_non_password_output() {
        assert!(!is_password_prompt(b"Last login: Mon Jan 1"));
        assert!(!is_password_prompt(b"$ echo hello"));
        assert!(!is_password_prompt(b""));
        assert!(!is_password_prompt(b"Welcome to Ubuntu"));
    }

    #[test]
    fn ssh_profile_launch_spec_includes_port_and_user() {
        let profile = SshProfile {
            name: "test".into(),
            host: "example.com".into(),
            port: 2222,
            user: "admin".into(),
            identity_file: None,
            password: Some("secret".into()),
        };
        let size = TerminalSize::new(80, 24);
        let spec = profile.launch_spec(size);
        assert_eq!(spec.program, "ssh");
        assert!(spec.args.contains(&"-p".to_string()));
        assert!(spec.args.contains(&"2222".to_string()));
        assert!(spec.args.contains(&"admin@example.com".to_string()));
    }

    #[test]
    fn ssh_profile_tab_title() {
        let with_name = SshProfile {
            name: "Production".into(),
            host: "prod.example.com".into(),
            port: 22,
            user: "deploy".into(),
            identity_file: None,
            password: None,
        };
        assert_eq!(with_name.tab_title(), "Production");

        let no_name = SshProfile {
            name: String::new(),
            host: "dev.example.com".into(),
            port: 22,
            user: "user".into(),
            identity_file: None,
            password: None,
        };
        assert_eq!(no_name.tab_title(), "user@dev.example.com");

        let no_name_no_user = SshProfile {
            name: String::new(),
            host: "bare.host".into(),
            port: 22,
            user: String::new(),
            identity_file: None,
            password: None,
        };
        assert_eq!(no_name_no_user.tab_title(), "bare.host");
    }
}
