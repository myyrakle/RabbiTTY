#[macro_export]
macro_rules! t {
    ($key:expr) => {
        $crate::i18n::t($key)
    };
}

include!(concat!(env!("OUT_DIR"), "/translations.rs"));

fn detect_locale() -> &'static str {
    let lang = std::env::var("LANG")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .unwrap_or_default()
        .to_lowercase();

    if lang.starts_with("ko") { "ko" } else { "en" }
}

pub fn t(key: &'static str) -> &'static str {
    static LOCALE: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();
    let locale = LOCALE.get_or_init(detect_locale);
    get_translation(locale, key).unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn en_translation_works() {
        assert_eq!(
            get_translation("en", "shell_picker.title"),
            Some("Start New Session")
        );
    }

    #[test]
    fn ko_translation_works() {
        assert_eq!(
            get_translation("ko", "shell_picker.title"),
            Some("새 세션 시작")
        );
    }

    #[test]
    fn unknown_key_returns_none() {
        assert_eq!(get_translation("en", "nonexistent.key"), None);
    }
}
