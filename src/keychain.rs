use keyring::Entry;

const SERVICE: &str = "rabbitty-ssh";

fn entry_for(host: &str, user: &str) -> Option<Entry> {
    let key = if user.is_empty() {
        host.to_string()
    } else {
        format!("{user}@{host}")
    };
    Entry::new(SERVICE, &key).ok()
}

pub fn get_password(host: &str, user: &str) -> Option<String> {
    entry_for(host, user)?.get_password().ok()
}

pub fn set_password(host: &str, user: &str, password: &str) {
    if let Some(entry) = entry_for(host, user) {
        let _ = entry.set_password(password);
    }
}

pub fn delete_password(host: &str, user: &str) {
    if let Some(entry) = entry_for(host, user) {
        let _ = entry.delete_credential();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_key_format() {
        // With user
        let entry = entry_for("host.com", "admin").unwrap();
        // Just verify it doesn't panic — the key is "admin@host.com"
        drop(entry);

        // Without user
        let entry = entry_for("bare.host", "").unwrap();
        drop(entry);
    }

    #[test]
    fn roundtrip_set_get_delete() {
        let host = "rabbitty-test-host.local";
        let user = "testuser";

        // Set
        set_password(host, user, "test_pw_12345");

        // Get
        let pw = get_password(host, user);
        assert_eq!(pw.as_deref(), Some("test_pw_12345"));

        // Delete
        delete_password(host, user);
        let pw = get_password(host, user);
        assert!(pw.is_none());
    }

    #[test]
    fn get_nonexistent_returns_none() {
        let pw = get_password("nonexistent-rabbitty-host.test", "nobody");
        assert!(pw.is_none());
    }
}
