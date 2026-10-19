//! Idioma da saída: pt-BR ou inglês, detectado pelo ambiente.

use std::fmt::Display;
use std::path::Path;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Pt,
    En,
}

/// Idioma de um valor de locale; `None` quando ele não decide (vazio, `C`,
/// `POSIX`). Qualquer idioma que não seja português vira inglês.
fn from_locale(value: &str) -> Option<Lang> {
    let base = value.split(['.', '@']).next().unwrap_or("");
    if base.is_empty() || base == "C" || base == "POSIX" {
        return None;
    }
    Some(if language(base).eq_ignore_ascii_case("pt") {
        Lang::Pt
    } else {
        Lang::En
    })
}

/// `GET_ENV_LANG` só decide com `pt*` ou `en*`.
fn from_override(value: &str) -> Option<Lang> {
    let code = language(value);
    if code.eq_ignore_ascii_case("pt") || code.eq_ignore_ascii_case("en") {
        from_locale(value)
    } else {
        None
    }
}

/// O código do idioma: `pt` em `pt_BR.UTF-8`.
fn language(value: &str) -> &str {
    value.split(['_', '-', '.', '@']).next().unwrap_or("")
}

/// `GET_ENV_LANG` válido vence; depois `LC_ALL`, `LC_MESSAGES` e `LANG`;
/// depois o primeiro idioma do macOS; senão inglês.
pub fn choose(
    get_env_lang: Option<&str>,
    lc_all: Option<&str>,
    lc_messages: Option<&str>,
    lang: Option<&str>,
    apple: impl FnOnce() -> Option<String>,
) -> Lang {
    get_env_lang
        .and_then(from_override)
        .or_else(|| {
            [lc_all, lc_messages, lang]
                .into_iter()
                .flatten()
                .find_map(from_locale)
        })
        .or_else(|| apple().as_deref().and_then(from_locale))
        .unwrap_or(Lang::En)
}

/// Primeiro item da lista que `defaults read -g AppleLanguages` imprime.
pub fn parse_apple_languages(out: &str) -> Option<String> {
    out.lines()
        .map(|l| l.trim().trim_end_matches(',').trim_matches('"'))
        .find(|l| !l.is_empty() && *l != "(" && *l != ")")
        .map(str::to_string)
}

fn apple_languages(defaults: &str) -> Option<String> {
    let out = std::process::Command::new(defaults)
        .args(["read", "-g", "AppleLanguages"])
        .stdin(std::process::Stdio::null())
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_apple_languages(&String::from_utf8_lossy(&out.stdout))
}

pub fn detect() -> Lang {
    let var = |name| std::env::var(name).ok();
    let (get_env_lang, lc_all, lc_messages, lang) = (
        var("GET_ENV_LANG"),
        var("LC_ALL"),
        var("LC_MESSAGES"),
        var("LANG"),
    );
    choose(
        get_env_lang.as_deref(),
        lc_all.as_deref(),
        lc_messages.as_deref(),
        lang.as_deref(),
        || apple_languages("/usr/bin/defaults"),
    )
}

/// Idioma desta execução, detectado uma vez. Nos testes unitários é sempre
/// pt-BR, para não depender da máquina.
pub fn lang() -> Lang {
    if cfg!(test) {
        return Lang::Pt;
    }
    static LANG: OnceLock<Lang> = OnceLock::new();
    *LANG.get_or_init(detect)
}

/// Como as linhas de uma seção da ajuda são pintadas.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RowKind {
    Plain,
    Keys,
    Examples,
}

/// Seção da ajuda: título e linhas `(esquerda, direita)`.
pub struct Section {
    pub title: &'static str,
    pub kind: RowKind,
    pub rows: &'static [(&'static str, &'static str)],
}

pub struct HelpDoc {
    pub tagline: &'static str,
    pub sections: &'static [Section],
}

static HELP_PT: HelpDoc = HelpDoc {
    tagline: "acha variáveis de ambiente pelo nome aproximado",
    sections: &[
        Section {
            title: "USO",
            kind: RowKind::Plain,
            rows: &[
                ("get-env [nome] [-f|--file <arquivo.env>]", ""),
                ("get-env -h|--help", ""),
            ],
        },
        Section {
            title: "ARGUMENTOS",
            kind: RowKind::Plain,
            rows: &[
                ("nome", "parte do nome; aceita letras salteadas"),
                (
                    "-f, --file <arq>",
                    "lê as variáveis do arquivo .env, não do ambiente",
                ),
                ("-h, --help", "mostra esta ajuda"),
            ],
        },
        Section {
            title: "COMPORTAMENTO",
            kind: RowKind::Plain,
            rows: &[
                ("1 resultado", "mostra o cartão e copia o valor"),
                (
                    "vários ou sem nome",
                    "abre a tabela interativa, filtrada pelo nome",
                ),
                ("em pipe", "imprime NOME=valor; segredos mascarados"),
            ],
        },
        Section {
            title: "EXEMPLOS",
            kind: RowKind::Examples,
            rows: &[
                ("get-env openai_key", "acha OPENAI_API_KEY e copia o valor"),
                ("get-env -f .env db", "procura só no .env"),
                ("get-env | grep AWS", "lista mascarada para outro comando"),
            ],
        },
        Section {
            title: "TABELA",
            kind: RowKind::Keys,
            rows: &[
                ("↑ ↓ j k", "move a seleção"),
                ("⏎", "copia NOME=valor"),
                ("n", "copia o nome"),
                ("v", "copia o valor"),
                ("/", "busca; Esc limpa a busca"),
                ("Esc", "limpa o filtro ou sai"),
                ("?", "mostra estas teclas"),
                ("q", "sai"),
                ("⧉", "clique copia a linha"),
                ("2× clique", "no nome ou no valor copia o que foi clicado"),
            ],
        },
        Section {
            title: "VARIÁVEIS",
            kind: RowKind::Plain,
            rows: &[
                (
                    "GET_ENV_LANG",
                    "pt ou en; senão LC_ALL, LC_MESSAGES, LANG, macOS",
                ),
                (
                    "GET_ENV_THEME",
                    "dark, light ou ansi; senão o tema do macOS",
                ),
                (
                    "GET_ENV_CLIP_CMD",
                    "recebe a cópia no stdin (padrão: pbcopy)",
                ),
                ("NO_COLOR", "desliga as cores"),
            ],
        },
    ],
};

static HELP_EN: HelpDoc = HelpDoc {
    tagline: "finds environment variables by approximate name",
    sections: &[
        Section {
            title: "USAGE",
            kind: RowKind::Plain,
            rows: &[
                ("get-env [name] [-f|--file <file.env>]", ""),
                ("get-env -h|--help", ""),
            ],
        },
        Section {
            title: "ARGUMENTS",
            kind: RowKind::Plain,
            rows: &[
                ("name", "part of the name; letters may be skipped"),
                (
                    "-f, --file <file>",
                    "reads the .env file instead of the environment",
                ),
                ("-h, --help", "shows this help"),
            ],
        },
        Section {
            title: "BEHAVIOR",
            kind: RowKind::Plain,
            rows: &[
                ("1 match", "shows the card and copies the value"),
                (
                    "many or no name",
                    "opens the interactive table, filtered by name",
                ),
                ("piped", "prints NAME=value; secrets masked"),
            ],
        },
        Section {
            title: "EXAMPLES",
            kind: RowKind::Examples,
            rows: &[
                (
                    "get-env openai_key",
                    "finds OPENAI_API_KEY and copies the value",
                ),
                ("get-env -f .env db", "searches only the .env"),
                ("get-env | grep AWS", "masked list for another command"),
            ],
        },
        Section {
            title: "TABLE",
            kind: RowKind::Keys,
            rows: &[
                ("↑ ↓ j k", "moves the selection"),
                ("⏎", "copies NAME=value"),
                ("n", "copies the name"),
                ("v", "copies the value"),
                ("/", "search; Esc clears the search"),
                ("Esc", "clears the filter or quits"),
                ("?", "shows these keys"),
                ("q", "quits"),
                ("⧉", "click copies the line"),
                ("2× click", "on the name or value copies what was clicked"),
            ],
        },
        Section {
            title: "VARIABLES",
            kind: RowKind::Plain,
            rows: &[
                (
                    "GET_ENV_LANG",
                    "pt or en; else LC_ALL, LC_MESSAGES, LANG, macOS",
                ),
                ("GET_ENV_THEME", "dark, light or ansi; else the macOS theme"),
                (
                    "GET_ENV_CLIP_CMD",
                    "receives the copy on stdin (default: pbcopy)",
                ),
                ("NO_COLOR", "turns colors off"),
            ],
        },
    ],
};

impl Lang {
    pub fn help(self) -> &'static HelpDoc {
        self.pick(&HELP_PT, &HELP_EN)
    }

    fn pick<T>(self, pt: T, en: T) -> T {
        match self {
            Lang::Pt => pt,
            Lang::En => en,
        }
    }

    pub fn masked(self) -> &'static str {
        self.pick("🔒 mascarado", "🔒 masked")
    }

    pub fn value_copied(self) -> &'static str {
        self.pick("✅ valor copiado", "✅ value copied")
    }

    pub fn copy_failed(self, e: &str) -> String {
        self.pick(format!("erro ao copiar: {e}"), format!("copy failed: {e}"))
    }

    pub fn no_match(self) -> &'static str {
        self.pick("nenhuma variável casa com ", "no variable matches ")
    }

    pub fn table_error(self, e: &dyn Display) -> String {
        self.pick(format!("erro na tabela: {e}"), format!("table error: {e}"))
    }

    pub fn write_error(self, e: &dyn Display) -> String {
        self.pick(
            format!("erro ao escrever: {e}"),
            format!("write error: {e}"),
        )
    }

    pub fn usage(self) -> &'static str {
        self.pick(
            "uso: get-env [nome] [--file|-f <arquivo.env>] [-h|--help]",
            "usage: get-env [name] [--file|-f <file.env>] [-h|--help]",
        )
    }

    pub fn clip_failed(self, e: &dyn Display) -> String {
        self.pick(
            format!("comando de clipboard falhou: {e}"),
            format!("clipboard command failed: {e}"),
        )
    }

    pub fn clip_exit(self, code: i32) -> String {
        self.pick(
            format!("comando de clipboard saiu com {code}"),
            format!("clipboard command exited with {code}"),
        )
    }

    pub fn clip_signal(self) -> &'static str {
        self.pick(
            "comando de clipboard terminou por sinal",
            "clipboard command killed by signal",
        )
    }

    pub fn file_not_found(self, path: &Path) -> String {
        let path = path.display();
        self.pick(
            format!("arquivo não encontrado: {path}"),
            format!("file not found: {path}"),
        )
    }

    pub fn read_error(self, path: &Path, e: &dyn Display) -> String {
        let path = path.display();
        self.pick(
            format!("erro ao ler {path}: {e}"),
            format!("error reading {path}: {e}"),
        )
    }

    pub fn zsh_failed(self, e: &dyn Display) -> String {
        self.pick(
            format!("erro ao rodar o zsh: {e}"),
            format!("error running zsh: {e}"),
        )
    }

    pub fn zsh_exit(self, path: &Path, code: &str) -> String {
        let path = path.display();
        self.pick(
            format!("erro ao ler {path}: o zsh saiu com {code}"),
            format!("error reading {path}: zsh exited with {code}"),
        )
    }

    pub fn signal(self) -> &'static str {
        self.pick("sinal", "signal")
    }

    pub fn var_count(self, n: usize) -> String {
        match (self, n) {
            (Lang::Pt, 1) => "1 variável".to_string(),
            (Lang::Pt, n) => format!("{n} variáveis"),
            (Lang::En, 1) => "1 variable".to_string(),
            (Lang::En, n) => format!("{n} variables"),
        }
    }

    pub fn filter(self) -> &'static str {
        self.pick("filtro", "filter")
    }

    pub fn header_name(self) -> &'static str {
        self.pick("NOME", "NAME")
    }

    pub fn header_value(self) -> &'static str {
        self.pick("VALOR", "VALUE")
    }

    pub fn word_line(self) -> &'static str {
        self.pick("linha", "line")
    }

    pub fn word_name(self) -> &'static str {
        self.pick("nome", "name")
    }

    pub fn word_value(self) -> &'static str {
        self.pick("valor", "value")
    }

    pub fn word_search(self) -> &'static str {
        self.pick("buscar", "search")
    }

    pub fn word_quit(self) -> &'static str {
        self.pick("sair", "quit")
    }

    pub fn esc_clears(self) -> &'static str {
        self.pick("Esc limpa", "Esc clears")
    }
    pub fn word_help(self) -> &'static str {
        self.pick("ajuda", "help")
    }
    pub fn any_key_closes(self) -> &'static str {
        self.pick("qualquer tecla fecha", "any key closes")
    }

    pub fn copied(self, what: &str, name: &str) -> String {
        self.pick(
            format!("copiado: {what} de {name}"),
            format!("copied: {what} of {name}"),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Lang::{En, Pt};

    fn by_lang(value: &str) -> Lang {
        choose(None, None, None, Some(value), || unreachable!())
    }

    #[test]
    fn override_wins() {
        let pt_br = Some("pt_BR.UTF-8");
        assert_eq!(choose(Some("en"), None, None, pt_br, || unreachable!()), En);
        assert_eq!(
            choose(Some("pt"), None, None, Some("en_US"), || unreachable!()),
            Pt
        );
        assert_eq!(
            choose(Some("pt_PT"), None, None, Some("en_US"), || unreachable!()),
            Pt
        );
    }

    #[test]
    fn override_ignores_empty_and_unknown() {
        for v in ["", "klingon"] {
            assert_eq!(
                choose(Some(v), None, None, Some("pt_BR.UTF-8"), || unreachable!()),
                Pt,
                "{v:?}"
            );
        }
    }

    #[test]
    fn posix_order() {
        assert_eq!(
            choose(None, Some("en_US"), None, Some("pt_BR"), || unreachable!()),
            En
        );
        assert_eq!(
            choose(None, None, Some("pt_BR"), Some("en_US"), || unreachable!()),
            Pt
        );
        assert_eq!(by_lang("pt-BR"), Pt);
    }

    #[test]
    fn c_utf8_does_not_decide() {
        for v in ["C", "POSIX", "C.UTF-8", "POSIX.UTF-8", ""] {
            assert_eq!(
                choose(None, Some(v), None, Some("pt_BR.UTF-8"), || unreachable!()),
                Pt,
                "{v:?}"
            );
        }
    }

    #[test]
    fn unsupported_locale_is_english() {
        assert_eq!(by_lang("fr_FR.UTF-8"), En);
        assert_eq!(by_lang("pt_BR.UTF-8@euro"), Pt);
    }

    #[test]
    fn apple_decides_last() {
        assert_eq!(choose(None, None, None, None, || Some("pt-BR".into())), Pt);
        assert_eq!(choose(None, None, None, None, || Some("es-ES".into())), En);
        assert_eq!(choose(None, None, None, None, || None), En);
    }

    #[test]
    fn parses_apple_languages() {
        assert_eq!(
            parse_apple_languages("(\n    \"pt-BR\",\n    \"en-US\"\n)\n"),
            Some("pt-BR".to_string())
        );
        assert_eq!(
            parse_apple_languages("(\n    en,\n    fr\n)"),
            Some("en".to_string())
        );
        assert_eq!(parse_apple_languages("(\n)"), None);
    }

    #[test]
    fn missing_defaults_binary_is_none() {
        assert_eq!(apple_languages("/nao/existe/defaults"), None);
    }

    #[test]
    fn help_describes_copy_and_pipe_truthfully() {
        let row = |lang: Lang, title: &str, left: &str| {
            let section = lang.help().sections.iter().find(|s| s.title == title);
            section
                .unwrap()
                .rows
                .iter()
                .find(|(l, _)| *l == left)
                .unwrap()
                .1
        };
        assert_eq!(row(Pt, "TABELA", "⏎"), "copia NOME=valor");
        assert_eq!(row(En, "TABLE", "⏎"), "copies NAME=value");
        assert_eq!(
            row(Pt, "COMPORTAMENTO", "em pipe"),
            "imprime NOME=valor; segredos mascarados"
        );
        assert_eq!(
            row(En, "BEHAVIOR", "piped"),
            "prints NAME=value; secrets masked"
        );
    }

    #[test]
    fn help_lists_vim_keys_and_help_key() {
        let keys = |lang: Lang| {
            let section = lang
                .help()
                .sections
                .iter()
                .find(|s| s.kind == RowKind::Keys);
            section
                .unwrap()
                .rows
                .iter()
                .map(|(l, _)| *l)
                .collect::<Vec<_>>()
        };
        for lang in [Pt, En] {
            assert!(keys(lang).contains(&"↑ ↓ j k"), "{lang:?}");
            assert!(keys(lang).contains(&"?"), "{lang:?}");
        }
        assert_eq!(En.word_help(), "help");
        assert_eq!(Pt.word_help(), "ajuda");
        assert_eq!(En.any_key_closes(), "any key closes");
    }

    #[test]
    fn english_catalog() {
        assert_eq!(En.var_count(1), "1 variable");
        assert_eq!(En.var_count(3), "3 variables");
        assert_eq!(Pt.var_count(1), "1 variável");
        assert_eq!(Pt.var_count(3), "3 variáveis");
        assert_eq!(En.copied("value", "X"), "copied: value of X");
        assert_eq!(En.no_match(), "no variable matches ");
    }
}
