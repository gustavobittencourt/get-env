//! Tema de cores: Catppuccin Mocha ou Latte pelo tema do macOS, ANSI quando
//! não dá para saber, monocromático com `NO_COLOR`.

use ratatui::style::{Color, Modifier, Style};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Dark,
    Light,
    Ansi,
    Mono,
}

/// Aparência do macOS segundo o `defaults`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Appearance {
    Dark,
    Light,
    Unknown,
}

/// `NO_COLOR` não vazio vence; depois `GET_ENV_THEME` válido; senão o macOS.
pub fn choose(
    theme_var: Option<&str>,
    no_color: Option<&str>,
    macos: impl FnOnce() -> Appearance,
) -> Theme {
    if no_color.is_some_and(|v| !v.is_empty()) {
        return Theme::Mono;
    }
    match theme_var {
        Some("dark") => Theme::Dark,
        Some("light") => Theme::Light,
        Some("ansi") => Theme::Ansi,
        _ => match macos() {
            Appearance::Dark => Theme::Dark,
            Appearance::Light => Theme::Light,
            Appearance::Unknown => Theme::Ansi,
        },
    }
}

/// `defaults read -g AppleInterfaceStyle`: imprime `Dark` no tema escuro e
/// sai com erro dizendo que a chave não existe no claro.
pub fn macos_appearance(defaults: &str) -> Appearance {
    let Ok(out) = std::process::Command::new(defaults)
        .args(["read", "-g", "AppleInterfaceStyle"])
        .stdin(std::process::Stdio::null())
        .output()
    else {
        return Appearance::Unknown;
    };
    if out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "Dark" {
        Appearance::Dark
    } else if !out.status.success()
        && String::from_utf8_lossy(&out.stderr).contains("does not exist")
    {
        Appearance::Light
    } else {
        Appearance::Unknown
    }
}

pub fn detect() -> Theme {
    let theme_var = std::env::var("GET_ENV_THEME").ok();
    let no_color = std::env::var("NO_COLOR").ok();
    choose(theme_var.as_deref(), no_color.as_deref(), || {
        macos_appearance("/usr/bin/defaults")
    })
}

/// Estilo de cada papel na tela.
#[derive(Clone, Debug)]
pub struct Palette {
    pub border: Style,
    pub title: Style,
    pub header: Style,
    pub name: Style,
    pub matched: Style,
    pub value: Style,
    pub masked: Style,
    pub zebra: Style,
    pub selected: Style,
    pub key: Style,
    pub secondary: Style,
    pub ok: Style,
    pub err: Style,
    pub icon: Style,
}

/// Cores de uma variante do Catppuccin.
struct Catppuccin {
    lavender: Color,
    mauve: Color,
    blue: Color,
    teal: Color,
    peach: Color,
    text: Color,
    overlay0: Color,
    zebra: Color,
    base: Color,
    subtext0: Color,
    green: Color,
    red: Color,
}

const MOCHA: Catppuccin = Catppuccin {
    lavender: Color::Rgb(0xb4, 0xbe, 0xfe),
    mauve: Color::Rgb(0xcb, 0xa6, 0xf7),
    blue: Color::Rgb(0x89, 0xb4, 0xfa),
    teal: Color::Rgb(0x94, 0xe2, 0xd5),
    peach: Color::Rgb(0xfa, 0xb3, 0x87),
    text: Color::Rgb(0xcd, 0xd6, 0xf4),
    overlay0: Color::Rgb(0x6c, 0x70, 0x86),
    zebra: Color::Rgb(0x31, 0x32, 0x44), // surface0
    base: Color::Rgb(0x1e, 0x1e, 0x2e),
    subtext0: Color::Rgb(0xa6, 0xad, 0xc8),
    green: Color::Rgb(0xa6, 0xe3, 0xa1),
    red: Color::Rgb(0xf3, 0x8b, 0xa8),
};

const LATTE: Catppuccin = Catppuccin {
    lavender: Color::Rgb(0x72, 0x87, 0xfd),
    mauve: Color::Rgb(0x88, 0x39, 0xef),
    blue: Color::Rgb(0x1e, 0x66, 0xf5),
    teal: Color::Rgb(0x17, 0x92, 0x99),
    peach: Color::Rgb(0xfe, 0x64, 0x0b),
    text: Color::Rgb(0x4c, 0x4f, 0x69),
    overlay0: Color::Rgb(0x9c, 0xa0, 0xb0),
    zebra: Color::Rgb(0xe6, 0xe9, 0xef), // mantle
    base: Color::Rgb(0xef, 0xf1, 0xf5),
    subtext0: Color::Rgb(0x6c, 0x6f, 0x85),
    green: Color::Rgb(0x40, 0xa0, 0x2b),
    red: Color::Rgb(0xd2, 0x0f, 0x39),
};

impl Palette {
    pub fn of(theme: Theme) -> Palette {
        match theme {
            Theme::Dark => Self::catppuccin(&MOCHA),
            Theme::Light => Self::catppuccin(&LATTE),
            Theme::Ansi => Palette {
                border: Style::new().fg(Color::Blue),
                title: Style::new().fg(Color::Cyan).bold(),
                header: Style::new().fg(Color::Magenta).bold(),
                name: Style::new().fg(Color::Cyan),
                matched: Style::new().fg(Color::Yellow).bold().underlined(),
                value: Style::new(),
                masked: Style::new().dim(),
                zebra: Style::new().bg(Color::DarkGray),
                selected: Style::new().reversed().bold(),
                key: Style::new().fg(Color::Black).bg(Color::Cyan),
                secondary: Style::new().dim(),
                ok: Style::new().fg(Color::Green),
                err: Style::new().fg(Color::Red),
                icon: Style::new().fg(Color::Blue),
            },
            Theme::Mono => Palette {
                border: Style::new(),
                title: Style::new().bold(),
                header: Style::new().bold(),
                name: Style::new(),
                matched: Style::new().bold().underlined(),
                value: Style::new(),
                masked: Style::new().dim(),
                zebra: Style::new(),
                selected: Style::new().reversed(),
                key: Style::new().reversed(),
                secondary: Style::new().dim(),
                ok: Style::new(),
                err: Style::new(),
                icon: Style::new(),
            },
        }
    }

    fn catppuccin(c: &Catppuccin) -> Palette {
        Palette {
            border: Style::new().fg(c.lavender),
            title: Style::new().fg(c.mauve).bold(),
            header: Style::new().fg(c.blue).bold(),
            name: Style::new().fg(c.teal),
            matched: Style::new().fg(c.peach).bold().underlined(),
            value: Style::new().fg(c.text),
            masked: Style::new().fg(c.overlay0),
            zebra: Style::new().bg(c.zebra),
            selected: Style::new().fg(c.base).bg(c.mauve).bold(),
            key: Style::new().fg(c.base).bg(c.mauve),
            secondary: Style::new().fg(c.subtext0),
            ok: Style::new().fg(c.green),
            err: Style::new().fg(c.red),
            icon: Style::new().fg(c.blue),
        }
    }
}

/// `text` com o estilo em códigos SGR; estilo vazio devolve o texto puro.
pub fn paint(text: &str, style: Style) -> String {
    let mut codes: Vec<String> = [
        (Modifier::BOLD, "1"),
        (Modifier::DIM, "2"),
        (Modifier::ITALIC, "3"),
        (Modifier::UNDERLINED, "4"),
        (Modifier::REVERSED, "7"),
    ]
    .iter()
    .filter(|(m, _)| style.add_modifier.contains(*m))
    .map(|(_, code)| code.to_string())
    .collect();
    codes.extend(style.fg.and_then(|c| color_code(c, 0)));
    codes.extend(style.bg.and_then(|c| color_code(c, 10)));
    if codes.is_empty() {
        return text.to_string();
    }
    format!("\x1b[{}m{text}\x1b[0m", codes.join(";"))
}

/// Código SGR da cor; `offset` 10 dá a cor de fundo.
fn color_code(color: Color, offset: u8) -> Option<String> {
    let base = match color {
        Color::Rgb(r, g, b) => return Some(format!("{};2;{r};{g};{b}", 38 + offset)),
        Color::Black => 30,
        Color::Red => 31,
        Color::Green => 32,
        Color::Yellow => 33,
        Color::Blue => 34,
        Color::Magenta => 35,
        Color::Cyan => 36,
        Color::Gray => 37,
        Color::DarkGray => 90,
        Color::LightRed => 91,
        Color::LightGreen => 92,
        Color::LightYellow => 93,
        Color::LightBlue => 94,
        Color::LightMagenta => 95,
        Color::LightCyan => 96,
        Color::White => 97,
        _ => return None,
    };
    Some((base + offset).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_color_wins_over_theme_var() {
        assert_eq!(
            choose(Some("dark"), Some("1"), || unreachable!()),
            Theme::Mono
        );
    }

    #[test]
    fn empty_no_color_is_ignored() {
        assert_eq!(
            choose(Some("light"), Some(""), || unreachable!()),
            Theme::Light
        );
    }

    #[test]
    fn theme_var_forces_palette() {
        for (v, t) in [
            ("dark", Theme::Dark),
            ("light", Theme::Light),
            ("ansi", Theme::Ansi),
        ] {
            assert_eq!(choose(Some(v), None, || unreachable!()), t);
        }
    }

    #[test]
    fn macos_decides_when_no_override() {
        assert_eq!(choose(Some("roxo"), None, || Appearance::Dark), Theme::Dark);
        assert_eq!(choose(None, None, || Appearance::Light), Theme::Light);
        assert_eq!(choose(None, None, || Appearance::Unknown), Theme::Ansi);
    }

    #[test]
    fn missing_defaults_binary_is_unknown() {
        assert_eq!(
            macos_appearance("/nao/existe/defaults"),
            Appearance::Unknown
        );
    }

    #[test]
    fn paint_emits_sgr() {
        let s = Style::new()
            .fg(Color::Rgb(1, 2, 3))
            .bg(Color::Rgb(4, 5, 6))
            .bold();
        assert_eq!(paint("x", s), "\x1b[1;38;2;1;2;3;48;2;4;5;6mx\x1b[0m");
        assert_eq!(
            paint("x", Style::new().fg(Color::Black).bg(Color::Cyan)),
            "\x1b[30;46mx\x1b[0m"
        );
        assert_eq!(
            paint("x", Style::new().dim().underlined().reversed()),
            "\x1b[2;4;7mx\x1b[0m"
        );
        assert_eq!(paint("x", Style::new()), "x");
    }

    #[test]
    fn mocha_and_latte_roles() {
        let m = Palette::of(Theme::Dark);
        assert_eq!(m.border.fg, Some(Color::Rgb(0xb4, 0xbe, 0xfe)));
        assert_eq!(
            m.selected,
            Style::new()
                .fg(Color::Rgb(0x1e, 0x1e, 0x2e))
                .bg(Color::Rgb(0xcb, 0xa6, 0xf7))
                .bold()
        );
        assert_eq!(m.zebra.bg, Some(Color::Rgb(0x31, 0x32, 0x44)));
        let l = Palette::of(Theme::Light);
        assert_eq!(l.zebra.bg, Some(Color::Rgb(0xe6, 0xe9, 0xef)));
        assert_eq!(l.err.fg, Some(Color::Rgb(0xd2, 0x0f, 0x39)));
    }

    #[test]
    fn ansi_masked_is_dim_not_bright_black() {
        let a = Palette::of(Theme::Ansi);
        assert_eq!(a.masked, Style::new().dim());
        assert_eq!(a.zebra.bg, Some(Color::DarkGray));
    }

    #[test]
    fn mono_has_no_color_and_no_zebra() {
        let p = Palette::of(Theme::Mono);
        assert_eq!(p.zebra, Style::new());
        assert_eq!(p.selected, Style::new().reversed());
        assert_eq!(p.matched, Style::new().bold().underlined());
        for s in [&p.border, &p.title, &p.name, &p.ok, &p.err, &p.key] {
            assert!(s.fg.is_none() && s.bg.is_none(), "{s:?}");
        }
    }
}
