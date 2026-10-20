//! Valores com cara de segredo, qualquer que seja o nome da variável.
//!
//! As regras vêm inteiras de `config/gitleaks.toml` do gitleaks (MIT,
//! Copyright (c) 2019 Zachary Rice; ver `NOTICE`), geradas em
//! `secrets/rules.rs` por `scripts/gen_secret_rules.py`. A checagem segue a
//! do gitleaks: prefiltro pelas keywords da regra, regex, grupo do segredo,
//! entropia mínima e allowlists. Só a URL com senha é regra própria.

// Gerado por script: o formatter não reescreve.
#[rustfmt::skip]
mod rules;

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use regex::{Regex, RegexBuilder};

pub struct Rule {
    /// Só os testes leem: nomeia a regra que não compila.
    #[cfg_attr(not(test), allow(dead_code))]
    pub id: &'static str,
    pub regex: &'static str,
    pub keywords: &'static [&'static str],
    pub entropy: Option<f64>,
    pub secret_group: usize,
    pub allowlists: &'static [Allowlist],
}

/// Onde o regex da allowlist é testado.
pub enum Target {
    Secret,
    Match,
    Line,
}

pub struct Allowlist {
    pub target: Target,
    /// `condition = "AND"`: todos os critérios precisam casar.
    pub and: bool,
    /// Tem critério de caminho, que um valor de variável nunca satisfaz.
    pub has_paths: bool,
    pub regexes: &'static [&'static str],
    pub stopwords: &'static [&'static str],
}

/// Regra própria: URL com usuário e senha (`esquema://usuário:senha@host`).
const URL_WITH_PASSWORD: &str = r"[A-Za-z][A-Za-z0-9+.-]*://[^\s:/@]+:[^\s/@]+@";

/// Regexes compilados sob demanda: só os que passam do prefiltro custam.
/// `None` guarda o que o crate `regex` não compila.
fn compiled(pattern: &'static str) -> Option<&'static Regex> {
    static CACHE: LazyLock<Mutex<HashMap<&'static str, Option<&'static Regex>>>> =
        LazyLock::new(Default::default);
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    *cache.entry(pattern).or_insert_with(|| {
        RegexBuilder::new(pattern)
            .size_limit(1 << 26)
            .build()
            .ok()
            .map(|re| &*Box::leak(Box::new(re)))
    })
}

/// O valor contém algo com formato de credencial conhecida.
pub fn looks_secret(value: &str) -> bool {
    if compiled(URL_WITH_PASSWORD).is_some_and(|re| re.is_match(value)) {
        return true;
    }
    let lower = value.to_lowercase();
    rules::RULES.iter().any(|rule| finds(rule, value, &lower))
}

fn finds(rule: &Rule, value: &str, lower: &str) -> bool {
    if !rule.keywords.is_empty() && !rule.keywords.iter().any(|k| lower.contains(k)) {
        return false;
    }
    let Some(re) = compiled(rule.regex) else {
        return false;
    };
    re.captures_iter(value).any(|caps| {
        let whole = caps.get(0).expect("grupo 0 existe");
        let secret = secret_of(rule, &caps).unwrap_or(whole.as_str());
        let line = line_around(value, whole.start(), whole.end());
        if rule.entropy.is_some_and(|min| entropy(secret) <= min) {
            return false;
        }
        let allowed = |a: &Allowlist| allows(a, secret, whole.as_str(), line);
        !(allows(&rules::GLOBAL, secret, whole.as_str(), line)
            || rule.allowlists.iter().any(allowed))
    })
}

/// O grupo `secretGroup`, ou o primeiro grupo não vazio, como no gitleaks.
fn secret_of<'a>(rule: &Rule, caps: &regex::Captures<'a>) -> Option<&'a str> {
    if rule.secret_group > 0 {
        return caps.get(rule.secret_group).map(|m| m.as_str());
    }
    caps.iter()
        .skip(1)
        .flatten()
        .map(|m| m.as_str())
        .find(|s| !s.is_empty())
}

fn line_around(value: &str, start: usize, end: usize) -> &str {
    let from = value[..start].rfind('\n').map_or(0, |i| i + 1);
    let to = value[end..].find('\n').map_or(value.len(), |i| end + i);
    &value[from..to]
}

/// Entropia de Shannon por caractere, em bits.
fn entropy(text: &str) -> f64 {
    let mut counts: HashMap<char, usize> = HashMap::new();
    for c in text.chars() {
        *counts.entry(c).or_default() += 1;
    }
    let len = text.chars().count() as f64;
    counts
        .values()
        .map(|&n| {
            let p = n as f64 / len;
            -p * p.log2()
        })
        .sum()
}

fn allows(a: &Allowlist, secret: &str, whole: &str, line: &str) -> bool {
    let target = match a.target {
        Target::Secret => secret,
        Target::Match => whole,
        Target::Line => line,
    };
    let lower = secret.to_lowercase();
    let mut checks = Vec::new();
    if a.has_paths {
        checks.push(false);
    }
    if !a.regexes.is_empty() {
        checks.push(
            a.regexes
                .iter()
                .any(|r| compiled(r).is_some_and(|re| re.is_match(target))),
        );
    }
    if !a.stopwords.is_empty() {
        checks.push(a.stopwords.iter().any(|w| lower.contains(w)));
    }
    if a.and {
        !checks.is_empty() && checks.iter().all(|&c| c)
    } else {
        checks.iter().any(|&c| c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Texto variado (entropia alta) de `n` caracteres de `alphabet`.
    fn mixed(alphabet: &str, n: usize) -> String {
        alphabet.chars().cycle().take(n).collect()
    }

    const ALNUM: &str = "aB3dE5fG7hJ9kL1mN2pQ4rS6tU8vW0xYz";

    /// Os exemplos são montados em runtime, com o prefixo partido, para que
    /// o fonte não carregue nada com cara de segredo real.
    fn samples() -> Vec<(&'static str, String)> {
        let m = |n| mixed(ALNUM, n);
        vec![
            (
                "aws",
                format!("AK{}{}", "IA", mixed("QWERTYUPZXCV2345", 16)),
            ),
            ("github-pat", format!("gh{}{}", "p_", m(36))),
            ("github-oauth", format!("gh{}{}", "o_", m(36))),
            ("github-fine", format!("github{}{}", "_pat_", m(82))),
            (
                "slack-bot",
                format!("xo{}{}-{}", "xb-", "1029384756", "5647382910"),
            ),
            (
                "slack-user",
                format!(
                    "xo{}{}-{}-{}-{}",
                    "xp-",
                    "1029384756",
                    "5647382910",
                    "9182736450",
                    m(28)
                ),
            ),
            ("openai", format!("sk-{}T3Blb{}{}", m(20), "kFJ", m(20))),
            ("anthropic", format!("sk-ant-{}{}AA", "api03-", m(93))),
            (
                "anthropic-admin",
                format!("sk-ant-{}{}AA", "admin01-", m(93)),
            ),
            ("google", format!("AI{}{}", "za", m(35))),
            ("stripe", format!("sk_{}{}", "live_", m(30))),
            ("jwt", format!("ey{}.ey{}.{}", m(18), m(18), m(12))),
            ("url", format!("postgres://app:{}@db.local/app", "hunter2")),
            (
                "pem",
                format!(
                    "-----BEGIN RSA {}-----\n{}\n-----END RSA {}-----",
                    "PRIVATE KEY",
                    m(64),
                    "PRIVATE KEY"
                ),
            ),
        ]
    }

    #[test]
    fn known_formats_look_secret() {
        for (what, value) in samples() {
            assert!(looks_secret(&value), "{what}");
            // No meio de outro texto, como num cabeçalho `Authorization`.
            assert!(looks_secret(&format!("Bearer {value} ")), "{what}");
        }
    }

    #[test]
    fn plain_values_do_not() {
        for value in [
            "",
            "/usr/bin:/bin",
            "pt_BR.UTF-8",
            "https://example.com/a?b=c",
            "user@example.com",
            "sk-123456",
            "eyes.only",
            "xterm-256color",
            "/Users/x/.cargo/bin",
        ] {
            assert!(!looks_secret(value), "{value}");
        }
    }

    /// Repetitivo demais para a entropia mínima da regra: não é segredo.
    #[test]
    fn low_entropy_is_not_secret() {
        assert!(!looks_secret(&format!("gh{}{}", "p_", "a".repeat(36))));
    }

    /// Allowlist da regra: a chave AWS de exemplo da documentação.
    #[test]
    fn rule_allowlist_wins() {
        assert!(!looks_secret(&format!("AK{}QWERTYUPZEXAMPLE", "IA")));
        assert!(looks_secret(&format!("AK{}QWERTYUPZEXAMPLF", "IA")));
    }

    /// Stopword da allowlist global: o alfabeto não é segredo.
    #[test]
    fn global_allowlist_wins() {
        assert!(!looks_secret("token=abcdefghijklmnopqrstuvwxyz0123"));
        assert!(looks_secret(&format!("token={}", mixed(ALNUM, 30))));
    }

    /// Entropia e stopwords valem para o grupo do segredo, não para o
    /// `chave=` inteiro: `token` é stopword, mas fica fora do grupo.
    #[test]
    fn checks_run_on_the_secret_group() {
        assert!(!looks_secret("my_access_credential_token=1111111111"));
        assert!(looks_secret(&format!("my_token={}", mixed(ALNUM, 30))));
    }

    /// Todas as regras do gitleaks entram; nenhuma deixa de compilar.
    #[test]
    fn every_gitleaks_rule_compiles() {
        let broken: Vec<&str> = rules::RULES
            .iter()
            .filter(|r| compiled(r.regex).is_none())
            .map(|r| r.id)
            .collect();
        assert_eq!(broken, Vec::<&str>::new());
        assert_eq!(rules::SOURCE_RULES, 222);
        assert_eq!(rules::RULES.len(), 217);
    }

    #[test]
    fn every_allowlist_regex_compiles() {
        let all = rules::RULES
            .iter()
            .flat_map(|r| r.allowlists.iter())
            .chain(std::iter::once(&rules::GLOBAL));
        let broken: Vec<&str> = all
            .flat_map(|a| a.regexes.iter().copied())
            .filter(|r| compiled(r).is_none())
            .collect();
        assert_eq!(broken, Vec::<&str>::new());
    }
}
