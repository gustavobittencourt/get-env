//! Valores com cara de segredo, qualquer que seja o nome da variável.
//!
//! As regras marcadas `gitleaks:` são cópias literais de
//! `config/gitleaks.toml` do gitleaks (MIT, Copyright (c) 2019 Zachary
//! Rice), commit b58d3f102cf3a2c84cb7f923d05c25c9b1aed84b; ver `NOTICE`.

use std::sync::LazyLock;

use regex::RegexSet;

const PATTERNS: [&str; 13] = [
    // gitleaks: aws-access-token
    r"\b((?:A3T[A-Z0-9]|AKIA|ASIA|ABIA|ACCA)[A-Z2-7]{16})\b",
    // gitleaks: github-pat
    r"ghp_[0-9a-zA-Z]{36}",
    // gitleaks: github-oauth
    r"gho_[0-9a-zA-Z]{36}",
    // gitleaks: github-fine-grained-pat
    r"github_pat_\w{82}",
    // gitleaks: slack-bot-token
    r"xoxb-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*",
    // gitleaks: slack-user-token
    r"xox[pe](?:-[0-9]{10,13}){3}-[a-zA-Z0-9-]{28,34}",
    // gitleaks: openai-api-key
    r#"\b(sk-(?:proj|svcacct|admin)-(?:[A-Za-z0-9_-]{74}|[A-Za-z0-9_-]{58})T3BlbkFJ(?:[A-Za-z0-9_-]{74}|[A-Za-z0-9_-]{58})\b|sk-[a-zA-Z0-9]{20}T3BlbkFJ[a-zA-Z0-9]{20})(?:[\x60'"\s;]|\\[nr]|$)"#,
    // gitleaks: anthropic-api-key
    r#"\b(sk-ant-api03-[a-zA-Z0-9_\-]{93}AA)(?:[\x60'"\s;]|\\[nr]|$)"#,
    // gitleaks: anthropic-admin-api-key
    r#"\b(sk-ant-admin01-[a-zA-Z0-9_\-]{93}AA)(?:[\x60'"\s;]|\\[nr]|$)"#,
    // gitleaks: gcp-api-key
    r#"\b(AIza[\w-]{35})(?:[\x60'"\s;]|\\[nr]|$)"#,
    // gitleaks: jwt
    r#"\b(ey[a-zA-Z0-9]{17,}\.ey[a-zA-Z0-9\/\\_-]{17,}\.(?:[a-zA-Z0-9\/\\_-]{10,}={0,2})?)(?:[\x60'"\s;]|\\[nr]|$)"#,
    // gitleaks: private-key
    r"(?i)-----BEGIN[ A-Z0-9_-]{0,100}PRIVATE KEY(?: BLOCK)?-----[\s\S-]{64,}?KEY(?: BLOCK)?-----",
    // próprio: URL com usuário e senha (`esquema://usuário:senha@host`)
    r"[A-Za-z][A-Za-z0-9+.-]*://[^\s:/@]+:[^\s/@]+@",
];

static SET: LazyLock<RegexSet> =
    LazyLock::new(|| RegexSet::new(PATTERNS).expect("padrões válidos"));

/// O valor contém algo com formato de credencial conhecida.
pub fn looks_secret(value: &str) -> bool {
    SET.is_match(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Os exemplos são montados em runtime, com o prefixo partido, para que
    /// o fonte não carregue nada com cara de segredo real.
    fn samples() -> Vec<(&'static str, String)> {
        let r = |c: &str, n: usize| c.repeat(n);
        vec![
            ("aws", format!("AK{}{}", "IA", r("X", 16))),
            ("github-pat", format!("gh{}{}", "p_", r("a", 36))),
            ("github-oauth", format!("gh{}{}", "o_", r("a", 36))),
            ("github-fine", format!("github{}{}", "_pat_", r("a", 82))),
            (
                "slack-bot",
                format!("xo{}{}-{}", "xb-", r("1", 10), r("2", 10)),
            ),
            (
                "openai",
                format!("sk-{}T3Blb{}{}", r("a", 20), "kFJ", r("a", 20)),
            ),
            (
                "slack-user",
                format!(
                    "xo{}{}-{}-{}-{}",
                    "xp-",
                    r("1", 10),
                    r("2", 10),
                    r("3", 10),
                    r("a", 28)
                ),
            ),
            ("anthropic", format!("sk-ant-{}{}AA", "api03-", r("a", 93))),
            (
                "anthropic-admin",
                format!("sk-ant-{}{}AA", "admin01-", r("a", 93)),
            ),
            ("google", format!("AI{}{}", "za", r("a", 35))),
            (
                "jwt",
                format!("e{}.e{}.{}", r("y", 18), r("y", 18), r("c", 10)),
            ),
            ("url", format!("postgres://app:{}@db.local/app", "hunter2")),
            (
                "pem",
                format!(
                    "-----BEGIN RSA {}-----\n{}\n-----END RSA {}-----",
                    "PRIVATE KEY",
                    r("a", 64),
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
        ] {
            assert!(!looks_secret(value), "{value}");
        }
    }
}
