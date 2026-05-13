//! Load SSH profiles from the user's `~/.ssh/config` file.
//!
//! Wildcards (`Host *`, `Host *.example.com`, …) are treated as defaults that
//! contribute to matching literal hosts, but never produce a profile of their
//! own.

use crate::config::{SshAuthMethod, SshProfile};
use ssh2_config::{ParseRule, SshConfig};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

const KEY_FILE_PREFERENCE: &[&str] = &["id_ed25519", "id_rsa", "id_ecdsa", "id_dsa"];

pub fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".ssh/config"))
}

fn detect_default_identity_file() -> Option<String> {
    let ssh_dir = dirs::home_dir()?.join(".ssh");
    for name in KEY_FILE_PREFERENCE {
        let candidate = ssh_dir.join(name);
        if candidate.is_file() {
            return Some(candidate.to_string_lossy().to_string());
        }
    }
    None
}

pub fn load() -> Vec<SshProfile> {
    let Some(path) = config_path() else {
        return Vec::new();
    };
    let Ok(file) = File::open(&path) else {
        return Vec::new();
    };
    let mut reader = BufReader::new(file);
    let config = match SshConfig::default().parse(&mut reader, ParseRule::ALLOW_UNKNOWN_FIELDS) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Failed to parse {}: {err}", path.display());
            return Vec::new();
        }
    };

    let fallback_identity = detect_default_identity_file();
    let mut profiles = Vec::new();
    for hosts in config.get_hosts() {
        for pattern in &hosts.pattern {
            let raw = pattern.pattern.as_str();
            if raw.contains('*') || raw.contains('?') || raw.is_empty() {
                continue;
            }
            let params = config.query(raw);
            let identity_file = params
                .identity_file
                .as_ref()
                .and_then(|files| files.first())
                .map(|p| p.to_string_lossy().to_string())
                .or_else(|| fallback_identity.clone());
            profiles.push(SshProfile {
                name: raw.to_string(),
                host: params.host_name.clone().unwrap_or_else(|| raw.to_string()),
                port: params.port.unwrap_or(22),
                user: params.user.clone().unwrap_or_default(),
                auth_method: SshAuthMethod::KeyFile,
                identity_file,
                password: None,
                proxy_command: None,
            });
        }
    }
    profiles
}
