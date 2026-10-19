//! Como nomes e valores aparecem no terminal e no clipboard.

use unicode_width::UnicodeWidthChar;

const SENSITIVE_PARTS: [&str; 4] = ["TOKEN", "KEY", "SECRET", "PASS"];
const VISIBLE_CHARS: usize = 5;

pub fn is_sensitive(name: &str) -> bool {
    let upper = name.to_uppercase();
    SENSITIVE_PARTS.iter().any(|part| upper.contains(part))
}

/// Valor como aparece no terminal: mascarado se o nome é sensível e com
/// todo caractere de controle escapado (`\n`, `\r`, `\t`, `\x1b`…).
pub fn display_value(name: &str, value: &str) -> String {
    let shown = if !is_sensitive(name) {
        value.to_string()
    } else if value.chars().count() <= VISIBLE_CHARS {
        "•••".to_string()
    } else {
        let head: String = value.chars().take(VISIBLE_CHARS).collect();
        format!("{head}…")
    };
    escape(&shown)
}

/// Texto seguro para o terminal: todo caractere de controle sai escapado.
/// Vale para valores e também para nomes, que podem vir de um `-f`.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\x{:02x}", u32::from(c))),
            c => out.push(c),
        }
    }
    out
}

/// Colunas que o texto ocupa no terminal (emoji e CJK ocupam 2).
pub fn width(text: &str) -> usize {
    text.chars().map(|c| c.width().unwrap_or(0)).sum()
}

/// Corta `text` em `width` colunas, terminando em `…` quando não cabe.
pub fn fit(text: &str, width: usize) -> String {
    if self::width(text) <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut used = 0;
    let mut head = String::new();
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > width - 1 {
            break;
        }
        used += w;
        head.push(c);
    }
    format!("{head}…")
}

/// Quebra `text` em pedaços de até `width` colunas, sem partir caractere.
pub fn wrap(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![text.to_string()];
    }
    let mut lines = vec![String::new()];
    let mut used = 0;
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > width && used > 0 {
            lines.push(String::new());
            used = 0;
        }
        lines.last_mut().expect("nunca vazio").push(c);
        used += w;
    }
    lines
}

pub fn masked_line(name: &str, value: &str) -> String {
    format!("{}={}", escape(name), display_value(name, value))
}

/// `NOME=valor` para o clipboard: valor real, com aspas simples só quando
/// o shell precisaria delas.
pub fn shell_line(name: &str, value: &str) -> String {
    let safe = !value.is_empty()
        && value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_@%+:,./-".contains(c));
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

    /// Escape de terminal num valor não pode mudar cor, título da janela
    /// nem o prompt: todo caractere de controle sai escapado na tela.
    #[test]
    fn escapes_control_chars() {
        assert_eq!(
            display_value("MSG", "a\u{1b}]0;X\u{7}\tb\u{9b}c\u{7f}"),
            "a\\x1b]0;X\\x07\\tb\\x9bc\\x7f"
        );
    }

    /// O nome também vai para o terminal: um `\x1b` nele não pode pintar a
    /// tela nem mudar o título.
    #[test]
    fn masked_line_escapes_control_chars_in_name() {
        assert_eq!(masked_line("A\u{1b}[2J\u{7}B", "1"), "A\\x1b[2J\\x07B=1");
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
    fn shell_line_quotes_zsh_equals_expansion() {
        assert_eq!(shell_line("A", "=ls"), "A='=ls'");
        assert_eq!(shell_line("A", "x:=ls"), "A='x:=ls'");
    }

    #[test]
    fn shell_line_never_masks() {
        assert_eq!(shell_line("FAKE_TOKEN", "abcdefgh"), "FAKE_TOKEN=abcdefgh");
    }

    #[test]
    fn width_counts_wide_chars() {
        assert_eq!(width("abc"), 3);
        assert_eq!(width("🔒 x"), 4);
        assert_eq!(width("✅"), 2);
    }

    #[test]
    fn fit_by_display_width() {
        assert_eq!(fit("abcdef", 4), "abc…");
        assert_eq!(fit("abc", 4), "abc");
        assert_eq!(fit("abc", 0), "");
        assert_eq!(fit("ab🔒cd", 4), "ab…");
        assert_eq!(fit("日本語テキスト", 6), "日本…");
    }

    #[test]
    fn wrap_never_splits_wide_char() {
        assert_eq!(wrap("abcdefg", 3), vec!["abc", "def", "g"]);
        assert_eq!(wrap("a日本", 2), vec!["a", "日", "本"]);
        assert_eq!(wrap("", 5), vec![""]);
    }
}
