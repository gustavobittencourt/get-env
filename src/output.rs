//! O que a CLI escreve no terminal: o cartão do resultado único e as
//! mensagens de erro.

use ratatui::style::{Modifier, Style};

use crate::format::{display_value, fit, is_sensitive, width, wrap};
use crate::i18n::lang;
use crate::theme::{Palette, paint};

/// Rodapé do cartão: o resultado da cópia.
pub enum Footer {
    Copied,
    Failed(String),
}

const MIN_INNER: usize = 28;

/// Cartão de borda arredondada com o nome no título, o valor (mascarado
/// se sensível) no corpo e o resultado da cópia no rodapé. `inner` é o
/// número de colunas entre as bordas laterais.
pub fn card(
    name: &str,
    value: &str,
    footer: &Footer,
    palette: &Palette,
    max_width: usize,
) -> String {
    let shown = display_value(name, value);
    let masked_note = format!("  {}", lang().masked());
    let sensitive = is_sensitive(name);
    let body_width = width(&shown) + if sensitive { width(&masked_note) } else { 0 };
    let (foot, foot_style) = match footer {
        Footer::Copied => (lang().value_copied().to_string(), palette.ok),
        Footer::Failed(e) => (format!("❌ {}", lang().copy_failed(e)), palette.err),
    };
    let inner = (body_width + 2)
        .max(width(name) + 3)
        .max(width(&foot) + 6)
        .max(MIN_INNER)
        .min(max_width.max(10) - 2);
    let border = |text: &str| paint(text, palette.border);
    let value_style = if sensitive {
        palette.masked
    } else {
        palette.value
    };

    let title = fit(name, inner - 3);
    let mut out = format!(
        "{}{}{}\n",
        border("╭─ "),
        paint(&title, palette.title),
        border(&format!(
            " {}╮",
            "─".repeat(inner.saturating_sub(3 + width(&title)))
        ))
    );

    let body_line = |painted: String, used: usize| {
        format!(
            "{}{painted}{}{}\n",
            border("│ "),
            " ".repeat(inner.saturating_sub(2 + used)),
            border(" │")
        )
    };
    if body_width <= inner - 2 {
        let mut painted = paint(&shown, value_style);
        if sensitive {
            painted += &paint(&masked_note, palette.secondary);
        }
        out += &body_line(painted, body_width);
    } else {
        let body = if sensitive {
            format!("{shown}{masked_note}")
        } else {
            shown
        };
        for chunk in wrap(&body, inner - 2) {
            out += &body_line(paint(&chunk, value_style), width(&chunk));
        }
    }

    let foot = fit(&foot, inner - 6);
    out += &format!(
        "{}  {}  {}\n",
        border(&format!(
            "╰{}",
            "─".repeat(inner.saturating_sub(5 + width(&foot)))
        )),
        paint(&foot, foot_style),
        border("─╯")
    );
    out
}

/// `❌ <text><emphasis>`; com paleta, em cor de erro e `emphasis` em negrito.
pub fn error_line(text: &str, emphasis: &str, palette: Option<&Palette>) -> String {
    let Some(p) = palette else {
        return format!("❌ {text}{emphasis}");
    };
    let mut line = format!("❌ {}", paint(text, p.err));
    if !emphasis.is_empty() {
        line += &paint(emphasis, p.err.add_modifier(Modifier::BOLD));
    }
    line
}

/// Linha de uso; com paleta, o nome do comando em negrito.
pub fn usage_line(usage: &str, palette: Option<&Palette>) -> String {
    match palette {
        Some(_) => usage.replacen("get-env", &paint("get-env", Style::new().bold()), 1),
        None => usage.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::width;
    use crate::theme::{Palette, Theme};

    /// Tira as sequências `\x1b[…m`.
    fn strip(s: &str) -> String {
        let mut out = String::new();
        let mut chars = s.chars();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                for c in chars.by_ref() {
                    if c == 'm' {
                        break;
                    }
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    #[test]
    fn card_layout_matches_spec() {
        let p = Palette::of(Theme::Mono);
        assert_eq!(
            strip(&card("FAKE_TOKEN", "abcdefgh", &Footer::Copied, &p, 80)),
            concat!(
                "╭─ FAKE_TOKEN ───────────────╮\n",
                "│ abcde…  🔒 mascarado       │\n",
                "╰───────  ✅ valor copiado  ─╯\n",
            )
        );
    }

    #[test]
    fn card_wraps_long_value_inside_frame() {
        let p = Palette::of(Theme::Dark);
        let value = "x".repeat(100);
        let out = strip(&card("LONG", &value, &Footer::Copied, &p, 40));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 5);
        assert!(lines.iter().all(|l| width(l) == 40), "{out}");
    }

    #[test]
    fn card_failure_footer() {
        let p = Palette::of(Theme::Mono);
        let out = strip(&card(
            "A",
            "1",
            &Footer::Failed("comando de clipboard saiu com 3".into()),
            &p,
            80,
        ));
        assert!(
            out.contains("❌ erro ao copiar: comando de clipboard saiu com 3"),
            "{out}"
        );
    }

    #[test]
    fn card_narrow_terminal_does_not_panic() {
        let p = Palette::of(Theme::Ansi);
        for w in 0..32 {
            let _ = card("FAKE_TOKEN", "abcdefgh", &Footer::Copied, &p, w);
        }
    }

    #[test]
    fn plain_card_has_no_value_color_in_mono() {
        let out = card(
            "HOME",
            "/tmp/x",
            &Footer::Copied,
            &Palette::of(Theme::Mono),
            80,
        );
        assert!(!out.contains("38;2;"), "{out:?}");
    }

    #[test]
    fn error_line_plain_and_colored() {
        assert_eq!(
            error_line("nenhuma variável casa com ", "zzz", None),
            "❌ nenhuma variável casa com zzz"
        );
        let dark = Palette::of(Theme::Dark);
        assert_eq!(strip(&error_line("x ", "y", Some(&dark))), "❌ x y");
        assert!(error_line("x ", "y", Some(&dark)).contains("\x1b[1;38;2;243;139;168my"));
    }

    #[test]
    fn usage_line_bolds_command() {
        let usage = "uso: get-env [nome]";
        assert_eq!(usage_line(usage, None), usage);
        let mono = Palette::of(Theme::Mono);
        assert_eq!(
            usage_line(usage, Some(&mono)),
            "uso: \x1b[1mget-env\x1b[0m [nome]"
        );
    }
}
