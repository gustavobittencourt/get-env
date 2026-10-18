//! Como nomes e valores aparecem no terminal e no clipboard.

const SENSITIVE_PARTS: [&str; 4] = ["TOKEN", "KEY", "SECRET", "PASS"];
const VISIBLE_CHARS: usize = 5;

pub fn is_sensitive(name: &str) -> bool {
    let upper = name.to_uppercase();
    SENSITIVE_PARTS.iter().any(|part| upper.contains(part))
}

/// Valor como aparece no terminal: mascarado se o nome é sensível e com
/// quebras de linha escapadas.
pub fn display_value(name: &str, value: &str) -> String {
    let shown = if !is_sensitive(name) {
        value.to_string()
    } else if value.chars().count() <= VISIBLE_CHARS {
        "•••".to_string()
    } else {
        let head: String = value.chars().take(VISIBLE_CHARS).collect();
        format!("{head}…")
    };
    shown.replace('\n', "\\n").replace('\r', "\\r")
}

pub fn masked_line(name: &str, value: &str) -> String {
    format!("{name}={}", display_value(name, value))
}

/// `NOME=valor` para o clipboard: valor real, com aspas simples só quando
/// o shell precisaria delas.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "a tabela da Task 5 passa a usar")
)]
pub fn shell_line(name: &str, value: &str) -> String {
    let safe = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_@%+=:,./-".contains(c));
    if safe {
        format!("{name}={value}")
    } else {
        format!("{name}='{}'", value.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensitive_names_ignore_case() {
        for n in ["OPENAI_API_KEY", "gh_token", "Db_Password", "MY_SECRET"] {
            assert!(is_sensitive(n), "{n}");
        }
        for n in ["PATH", "HOME", "PWD"] {
            assert!(!is_sensitive(n), "{n}");
        }
    }

    #[test]
    fn masks_long_sensitive_value() {
        assert_eq!(display_value("FAKE_TOKEN", "abcdefgh"), "abcde…");
    }

    #[test]
    fn masks_short_sensitive_value_fully() {
        assert_eq!(display_value("API_KEY", "abcde"), "•••");
        assert_eq!(display_value("API_KEY", ""), "•••");
    }

    #[test]
    fn masks_by_char_not_byte() {
        assert_eq!(display_value("API_KEY", "çãõéíú"), "çãõéí…");
    }

    #[test]
    fn plain_value_untouched() {
        assert_eq!(display_value("HOME", "/Users/x"), "/Users/x");
    }

    #[test]
    fn escapes_newlines() {
        assert_eq!(display_value("MSG", "a\nb\r"), "a\\nb\\r");
    }

    #[test]
    fn masked_line_format() {
        assert_eq!(masked_line("FAKE_TOKEN", "abcdefgh"), "FAKE_TOKEN=abcde…");
    }

    #[test]
    fn shell_line_raw_when_safe() {
        assert_eq!(shell_line("PATH", "/usr/bin:/bin"), "PATH=/usr/bin:/bin");
    }

    #[test]
    fn shell_line_quotes_when_needed() {
        assert_eq!(shell_line("MSG", "olá mundo"), "MSG='olá mundo'");
        assert_eq!(shell_line("A", "x$y"), "A='x$y'");
        assert_eq!(shell_line("A", "it's"), "A='it'\\''s'");
        assert_eq!(shell_line("A", "l1\nl2"), "A='l1\nl2'");
        assert_eq!(shell_line("A", ""), "A=''");
    }

    #[test]
    fn shell_line_never_masks() {
        assert_eq!(shell_line("FAKE_TOKEN", "abcdefgh"), "FAKE_TOKEN=abcdefgh");
    }
}
