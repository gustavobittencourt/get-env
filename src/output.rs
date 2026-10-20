//! O que a CLI escreve no terminal: o cartão do resultado único e as
//! mensagens de erro.

use ratatui::style::{Modifier, Style};

use crate::format::{display_value, escape, fit, is_sensitive, width, wrap};
use crate::i18n::{HelpDoc, RowKind, lang};
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
    let sensitive = is_sensitive(name, value);
    let name_shown = escape(name);
    let body_width = width(&shown) + if sensitive { width(&masked_note) } else { 0 };
    let (foot, foot_style) = match footer {
        Footer::Copied => (lang().value_copied().to_string(), palette.ok),
        Footer::Failed(e) => (format!("❌ {}", lang().copy_failed(e)), palette.err),
    };
    let inner = (body_width + 2)
        .max(width(&name_shown) + 3)
        .max(width(&foot) + 6)
        .max(MIN_INNER)
        .min(max_width.max(10) - 2);
    let border = |text: &str| paint(text, palette.border);
    let value_style = if sensitive {
        palette.masked
    } else {
        palette.value
    };

    let title = fit(&name_shown, inner - 3);
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

/// Texto do `--help`; com paleta, cada papel na sua cor.
pub fn help(doc: &HelpDoc, palette: Option<&Palette>) -> String {
    let style = |text: &str, role: fn(&Palette) -> Style| match palette {
        Some(p) => paint(text, role(p)),
        None => text.to_string(),
    };
    let cell = |kind: RowKind, left: &str| match kind {
        RowKind::Keys => format!(" {left} "),
        _ => left.to_string(),
    };
    // Linhas sem descrição (as de uso) não alargam a coluna da esquerda.
    let column = doc
        .sections
        .iter()
        .flat_map(|s| {
            s.rows
                .iter()
                .filter(|(_, right)| !right.is_empty())
                .map(|(left, _)| width(&cell(s.kind, left)))
        })
        .max()
        .unwrap_or(0)
        + 2;

    let mut out = format!(
        "{} — {}\n",
        style("get-env", |p| p.title),
        style(doc.tagline, |p| p.value)
    );
    for section in doc.sections {
        out += &format!("\n{}\n", style(section.title, |p| p.header));
        for (left, right) in section.rows {
            let left = cell(section.kind, left);
            let left_role: fn(&Palette) -> Style = match section.kind {
                RowKind::Keys => |p| p.key,
                _ => |p| p.name,
            };
            out += &format!("  {}", style(&left, left_role));
            if !right.is_empty() {
                let right_role: fn(&Palette) -> Style = match section.kind {
                    RowKind::Examples => |p| p.secondary,
                    _ => |p| p.value,
                };
                out += &" ".repeat(column - width(&left));
                out += &style(right, right_role);
            }
            out += "\n";
        }
    }
    out
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
    use crate::i18n::Lang;
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
    fn card_escapes_control_chars_in_name() {
        let p = Palette::of(Theme::Mono);
        let out = card("A\u{7}B", "1", &Footer::Copied, &p, 80);
        assert!(!out.contains('\u{7}'), "{out:?}");
        assert!(strip(&out).starts_with("╭─ A\\x07B ─"), "{out:?}");
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
    fn help_fits_72_columns() {
        for lang in [Lang::Pt, Lang::En] {
            let doc = lang.help();
            let text = help(doc, None);
            for line in text.lines() {
                assert!(width(line) <= 72, "{lang:?} {}: {line}", width(line));
            }
            assert!(text.contains(doc.tagline), "{lang:?}");
            for section in doc.sections {
                assert!(text.contains(section.title), "{}", section.title);
                for (left, right) in section.rows {
                    assert!(text.contains(left) && text.contains(right), "{left}");
                }
            }
        }
    }

    #[test]
    fn help_plain_has_no_escapes_and_colored_has() {
        let plain = help(Lang::Pt.help(), None);
        assert!(!plain.contains('\x1b'), "{plain}");
        assert!(plain.starts_with("get-env — acha variáveis"), "{plain}");
        let colored = help(Lang::Pt.help(), Some(&Palette::of(Theme::Dark)));
        assert!(colored.contains("\x1b["), "{colored}");
        assert_eq!(strip(&colored), plain);
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
