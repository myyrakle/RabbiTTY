use crate::config::ShortcutsConfig;
use iced::keyboard::{Key, Modifiers, key::Named};

#[derive(Debug, Clone, Copy)]
pub(super) enum ShortcutAction {
    NewTab,
    CloseTab,
    OpenSettings,
    NextTab,
    PrevTab,
    Quit,
}

impl ShortcutAction {
    pub(super) fn resolve(
        key: &Key,
        modifiers: Modifiers,
        shortcuts: &ShortcutsConfig,
    ) -> Option<Self> {
        if shortcut_matches(&shortcuts.new_tab, key, modifiers) {
            return Some(Self::NewTab);
        }
        if shortcut_matches(&shortcuts.close_tab, key, modifiers) {
            return Some(Self::CloseTab);
        }
        if shortcut_matches(&shortcuts.open_settings, key, modifiers) {
            return Some(Self::OpenSettings);
        }
        if shortcut_matches(&shortcuts.next_tab, key, modifiers) {
            return Some(Self::NextTab);
        }
        if shortcut_matches(&shortcuts.prev_tab, key, modifiers) {
            return Some(Self::PrevTab);
        }
        if shortcut_matches(&shortcuts.quit, key, modifiers) {
            return Some(Self::Quit);
        }
        None
    }
}

#[derive(Debug, Clone)]
struct ParsedShortcut {
    modifiers: Modifiers,
    key: String,
}

pub(super) fn shortcut_matches(binding: &str, key: &Key, modifiers: Modifiers) -> bool {
    let Some(parsed) = parse_shortcut(binding) else {
        return false;
    };
    let Some(event_key) = event_key_token(key) else {
        return false;
    };

    let tracked = Modifiers::SHIFT | Modifiers::CTRL | Modifiers::ALT | Modifiers::LOGO;
    let pressed = modifiers & tracked;

    parsed.key == event_key && parsed.modifiers == pressed
}

fn parse_shortcut(value: &str) -> Option<ParsedShortcut> {
    let mut modifiers = Modifiers::default();
    let mut key: Option<String> = None;

    for token in value.split('+') {
        let token = token.trim();
        if token.is_empty() {
            return None;
        }

        match token.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => modifiers.insert(Modifiers::CTRL),
            "alt" | "option" => modifiers.insert(Modifiers::ALT),
            "shift" => modifiers.insert(Modifiers::SHIFT),
            "cmd" | "command" | "meta" | "super" => modifiers.insert(Modifiers::COMMAND),
            _ => {
                if key.is_some() {
                    return None;
                }
                key = normalize_shortcut_key_token(token);
                key.as_ref()?;
            }
        }
    }

    Some(ParsedShortcut {
        modifiers,
        key: key?,
    })
}

fn event_key_token(key: &Key) -> Option<String> {
    match key {
        Key::Named(named) => match named {
            Named::Enter => Some("Enter".to_string()),
            Named::Tab => Some("Tab".to_string()),
            Named::Space => Some("Space".to_string()),
            Named::Escape => Some("Escape".to_string()),
            Named::ArrowUp => Some("ArrowUp".to_string()),
            Named::ArrowDown => Some("ArrowDown".to_string()),
            Named::ArrowLeft => Some("ArrowLeft".to_string()),
            Named::ArrowRight => Some("ArrowRight".to_string()),
            Named::Home => Some("Home".to_string()),
            Named::End => Some("End".to_string()),
            Named::Delete => Some("Delete".to_string()),
            Named::Backspace => Some("Backspace".to_string()),
            Named::Insert => Some("Insert".to_string()),
            Named::PageUp => Some("PageUp".to_string()),
            Named::PageDown => Some("PageDown".to_string()),
            Named::F1 => Some("F1".to_string()),
            Named::F2 => Some("F2".to_string()),
            Named::F3 => Some("F3".to_string()),
            Named::F4 => Some("F4".to_string()),
            Named::F5 => Some("F5".to_string()),
            Named::F6 => Some("F6".to_string()),
            Named::F7 => Some("F7".to_string()),
            Named::F8 => Some("F8".to_string()),
            Named::F9 => Some("F9".to_string()),
            Named::F10 => Some("F10".to_string()),
            Named::F11 => Some("F11".to_string()),
            Named::F12 => Some("F12".to_string()),
            _ => None,
        },
        Key::Character(c) => {
            let mut chars = c.chars();
            let ch = chars.next()?;
            if chars.next().is_some() {
                return None;
            }

            if ch.is_ascii_alphabetic() {
                return Some(ch.to_ascii_uppercase().to_string());
            }

            match ch {
                ',' => Some("Comma".to_string()),
                '.' => Some("Period".to_string()),
                _ if ch.is_ascii_digit()
                    || matches!(ch, '[' | ']' | '/' | ';' | '\'' | '-' | '=' | '`') =>
                {
                    Some(ch.to_string())
                }
                _ => None,
            }
        }
        Key::Unidentified => None,
    }
}

fn normalize_shortcut_key_token(value: &str) -> Option<String> {
    let lower = value.trim().to_ascii_lowercase();

    let normalized = match lower.as_str() {
        "esc" | "escape" => "Escape".to_string(),
        "enter" | "return" => "Enter".to_string(),
        "tab" => "Tab".to_string(),
        "space" | "spacebar" => "Space".to_string(),
        "home" => "Home".to_string(),
        "end" => "End".to_string(),
        "delete" | "del" => "Delete".to_string(),
        "backspace" => "Backspace".to_string(),
        "insert" | "ins" => "Insert".to_string(),
        "pageup" | "page-up" | "pgup" => "PageUp".to_string(),
        "pagedown" | "page-down" | "pgdown" => "PageDown".to_string(),
        "up" | "arrowup" => "ArrowUp".to_string(),
        "down" | "arrowdown" => "ArrowDown".to_string(),
        "left" | "arrowleft" => "ArrowLeft".to_string(),
        "right" | "arrowright" => "ArrowRight".to_string(),
        "comma" => "Comma".to_string(),
        "period" | "dot" => "Period".to_string(),
        "f1" => "F1".to_string(),
        "f2" => "F2".to_string(),
        "f3" => "F3".to_string(),
        "f4" => "F4".to_string(),
        "f5" => "F5".to_string(),
        "f6" => "F6".to_string(),
        "f7" => "F7".to_string(),
        "f8" => "F8".to_string(),
        "f9" => "F9".to_string(),
        "f10" => "F10".to_string(),
        "f11" => "F11".to_string(),
        "f12" => "F12".to_string(),
        _ => {
            if lower.chars().count() == 1 {
                let ch = lower.chars().next()?;
                if ch.is_ascii_alphanumeric() {
                    ch.to_ascii_uppercase().to_string()
                } else if matches!(
                    ch,
                    ',' | '.' | '[' | ']' | '/' | ';' | '\'' | '-' | '=' | '`'
                ) {
                    ch.to_string()
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }
    };

    Some(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_matches_normalized_named_key() {
        let matches = shortcut_matches(
            "ctrl + shift + pgdown",
            &Key::Named(Named::PageDown),
            Modifiers::CTRL | Modifiers::SHIFT,
        );

        assert!(matches);
    }

    #[test]
    fn shortcut_rejects_invalid_binding() {
        let matches =
            shortcut_matches("Ctrl+Unknown", &Key::Character("x".into()), Modifiers::CTRL);

        assert!(!matches);
    }
}
