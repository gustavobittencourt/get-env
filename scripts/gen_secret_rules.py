
"""Gera `src/secrets/rules.rs` a partir do `config/gitleaks.toml` do gitleaks.

Uso: python3 scripts/gen_secret_rules.py <gitleaks.toml> <commit>

Regras sem `regex` ou presas a `path` (nome de arquivo) ficam de fora:
um valor de variável de ambiente não tem caminho. Entrada de allowlist que
casa com o regex da própria regra (uma chave de exemplo literal) também sai,
para o repositório não carregar strings com cara de chave; o efeito é só
mascarar esses exemplos.
"""

import re
import sys
import tomllib


QUANTIFIER = re.compile(r"\{\d+(?:,\d*)?\}")
ESCAPED = 0


def rust_regex(pattern: str) -> str:
    """Escapa `{` que não abre quantificador: o RE2 do Go o lê como literal,
    o crate `regex` recusa o padrão."""
    global ESCAPED
    out, i, in_class = [], 0, False
    while i < len(pattern):
        c = pattern[i]
        if c == "\\":
            out.append(pattern[i : i + 2])
            i += 2
            continue
        if c == "[" and not in_class:
            in_class = True
        elif c == "]" and in_class:
            in_class = False
        elif c == "{" and not in_class and not QUANTIFIER.match(pattern, i):
            out.append("\\{")
            ESCAPED += 1
            i += 1
            continue
        out.append(c)
        i += 1
    return "".join(out)


def raw(text: str) -> str:
    """Literal `r#"…"#` com cercas suficientes para o texto."""
    hashes = 1
    while '"' + "#" * hashes in text:
        hashes += 1
    fence = "#" * hashes
    return f'r{fence}"{text}"{fence}'


def strs(items: list[str]) -> str:
    return "&[" + ", ".join(raw(i) for i in items) + "]"


def literal_keys(rule: dict, regexes: list[str]) -> list[str]:
    """Allowlist sem as entradas que o regex da regra detecta."""
    try:
        own = re.compile(rule["regex"])
    except re.error:
        return regexes
    return [r for r in regexes if not own.search(r)]


def allowlist(a: dict) -> str:
    target = {"secret": "Secret", "match": "Match", "line": "Line"}[a.get("regexTarget", "secret")]
    return (
        "Allowlist { "
        f"target: Target::{target}, "
        f"and: {'true' if a.get('condition', 'OR').upper() == 'AND' else 'false'}, "
        f"has_paths: {'true' if a.get('paths') else 'false'}, "
        f"regexes: {strs([rust_regex(r) for r in a.get('regexes', [])])}, "
        f"stopwords: {strs(a.get('stopwords', []))} "
        "}"
    )


def allowlists(rule: dict) -> list[dict]:
    out = []
    for a in rule.get("allowlists", []):
        kept = literal_keys(rule, a.get("regexes", []))
        dropped = len(a.get("regexes", [])) - len(kept)
        if dropped:
            print(f"{rule['id']}: {dropped} chave(s) de exemplo fora da allowlist", file=sys.stderr)
        out.append({**a, "regexes": kept})
    return out


def main() -> None:
    path, commit = sys.argv[1], sys.argv[2]
    config = tomllib.load(open(path, "rb"))
    total = len(config["rules"])
    rules = [r for r in config["rules"] if "regex" in r and "path" not in r]
    out = [
        "// Gerado por scripts/gen_secret_rules.py; não edite à mão.",
        f"// Fonte: gitleaks config/gitleaks.toml, commit {commit} (MIT,",
        "// Copyright (c) 2019 Zachary Rice; ver NOTICE).",
        f"// {total} regras no arquivo; {len(rules)} aqui (fora: sem regex ou presas a path).",
        "",
        "use super::{Allowlist, Rule, Target};",
        "",
        "#[cfg(test)]",
        f"pub const SOURCE_RULES: usize = {total};",
        "",
        f"pub static GLOBAL: Allowlist = {allowlist(config.get('allowlist', {}))};",
        "",
        "pub static RULES: &[Rule] = &[",
    ]
    for r in rules:
        entropy = r.get("entropy")
        out.append(
            "    Rule { "
            f"id: {raw(r['id'])}, "
            f"regex: {raw(rust_regex(r['regex']))}, "
            f"keywords: {strs(r.get('keywords', []))}, "
            f"entropy: {'None' if entropy is None else f'Some({float(entropy)!r})'}, "
            f"secret_group: {r.get('secretGroup', 0)}, "
            f"allowlists: &[{', '.join(allowlist(a) for a in allowlists(r))}] "
            "},"
        )
    out.append("];")
    print(f"{ESCAPED} `{{` literal(is) escapado(s) para o crate regex", file=sys.stderr)
    sys.stdout.write("\n".join(out) + "\n")


if __name__ == "__main__":
    main()

