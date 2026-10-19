# get-env — idioma automático e `--help` — plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** o `get-env` fala pt-BR ou inglês conforme o ambiente e ganha um `--help` colorido e traduzido.

**Architecture:** um módulo `src/i18n.rs` com `enum Lang { Pt, En }`, a detecção como função pura (`choose`), o idioma global num `OnceLock` (fixo em `Pt` sob `cfg(test)`) e cada texto visível como método de `Lang`. A ajuda é um documento estático por idioma (`HelpDoc`) que `output::help` pinta com a `Palette`.

**Tech Stack:** Rust edition 2024, `ratatui` 0.30.2, `unicode-width` 0.2.2; sem dependência nova.

**Spec:** `docs/superpowers/specs/2026-10-19-get-env-i18n-help-design.md`

## Global Constraints

- Sem dependência nova no `Cargo.toml`.
- Idiomas pt-BR e en; fallback en.
- Ordem: `GET_ENV_LANG` (só `pt*`/`en*` decidem) > `LC_ALL` > `LC_MESSAGES` > `LANG` > primeiro item do `AppleLanguages` > en.
- Normalização: corta em `.` e `@` antes de comparar; `C` e `POSIX` (inclusive `C.UTF-8`) não decidem; prefixo do idioma sem caixa, separador `_` ou `-`.
- Nenhum teste lê o ambiente real nem chama `pbcopy`; clipboard por `GET_ENV_CLIP_CMD` para arquivo temporário.
- `--help` no stdout, rc 0, até 72 colunas nos dois idiomas, cor só com stdout em TTY, nenhum `\x1b` fora de TTY.
- Gate: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`.

## Review Focus

1. `LC_ALL=C.UTF-8` com `LANG=pt_BR.UTF-8` → pt-BR (o `C` não pode decidir en). Teste: `c_utf8_does_not_decide` (Task 1).
2. `GET_ENV_LANG=` vazio ou lixo não pode travar em en. Teste: `override_ignores_empty_and_unknown` (Task 1).
3. `get-env -f -h` e `get-env --bogus --help` → ajuda, rc 0, não erro. Teste: `help_wins_over_bad_args` (Task 2).
4. Ajuda em pipe sem nenhum escape, mesmo com `GET_ENV_THEME=dark`. Teste: `help_pipe_has_no_escapes` (Task 2).
5. Saída do `AppleLanguages` com item sem aspas (`en,`) ou lista vazia. Teste: `parses_apple_languages` (Task 1).

---

### Task 1: idioma detectado e todos os textos traduzidos

**Files:**
- Create: `src/i18n.rs`
- Modify: `src/main.rs`, `src/cli.rs`, `src/output.rs`, `src/clipboard.rs`, `src/vars.rs`, `src/tui/mod.rs`, `src/tui/app.rs`
- Test: `src/i18n.rs` (unitários), `tests/cli.rs`

**Interfaces:**
- Produces (em `src/i18n.rs`):
  - `#[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Lang { Pt, En }`
  - `pub fn choose(get_env_lang: Option<&str>, lc_all: Option<&str>, lc_messages: Option<&str>, lang: Option<&str>, apple: impl FnOnce() -> Option<String>) -> Lang`
  - `fn from_locale(value: &str) -> Option<Lang>` (None = não decide) e `fn from_override(value: &str) -> Option<Lang>` (só `pt*`/`en*`)
  - `pub fn parse_apple_languages(out: &str) -> Option<String>`: primeiro item da lista do `defaults`, sem espaços, aspas e vírgula
  - `fn apple_languages(defaults: &str) -> Option<String>`: roda `<defaults> read -g AppleLanguages` com stdin nulo
  - `pub fn detect() -> Lang` (lê as 4 variáveis; o closure chama `apple_languages("/usr/bin/defaults")`)
  - `pub fn lang() -> Lang`: `OnceLock` com `detect()`; sob `#[cfg(test)]` devolve sempre `Lang::Pt`
  - métodos de `Lang` (pt / en, texto exato):

| método | pt | en |
|---|---|---|
| `masked() -> &str` | `🔒 mascarado` | `🔒 masked` |
| `value_copied() -> &str` | `✅ valor copiado` | `✅ value copied` |
| `copy_failed(e) -> String` | `erro ao copiar: {e}` | `copy failed: {e}` |
| `no_match() -> &str` | `nenhuma variável casa com ` | `no variable matches ` |
| `table_error(e)` | `erro na tabela: {e}` | `table error: {e}` |
| `write_error(e)` | `erro ao escrever: {e}` | `write error: {e}` |
| `usage() -> &str` | `uso: get-env [nome] [--file\|-f <arquivo.env>] [-h\|--help]` | `usage: get-env [name] [--file\|-f <file.env>] [-h\|--help]` |
| `clip_failed(e)` | `comando de clipboard falhou: {e}` | `clipboard command failed: {e}` |
| `clip_exit(code: i32)` | `comando de clipboard saiu com {code}` | `clipboard command exited with {code}` |
| `clip_signal() -> &str` | `comando de clipboard terminou por sinal` | `clipboard command killed by signal` |
| `file_not_found(path: &Path)` | `arquivo não encontrado: {path}` | `file not found: {path}` |
| `read_error(path, e)` | `erro ao ler {path}: {e}` | `error reading {path}: {e}` |
| `zsh_failed(e)` | `erro ao rodar o zsh: {e}` | `error running zsh: {e}` |
| `zsh_exit(path, code: &str)` | `erro ao ler {path}: o zsh saiu com {code}` | `error reading {path}: zsh exited with {code}` |
| `signal() -> &str` | `sinal` | `signal` |
| `var_count(n: usize)` | `1 variável` / `{n} variáveis` | `1 variable` / `{n} variables` |
| `filter() -> &str` | `filtro` | `filter` |
| `header_name()`, `header_value()` | `NOME`, `VALOR` | `NAME`, `VALUE` |
| `word_line()`, `word_name()`, `word_value()` | `linha`, `nome`, `valor` | `line`, `name`, `value` |
| `word_search()`, `word_quit()` | `buscar`, `sair` | `search`, `quit` |
| `esc_clears() -> &str` | `Esc limpa` | `Esc clears` |
| `copied(what, name)` | `copiado: {what} de {name}` | `copied: {what} of {name}` |

- [ ] **Step 1: testes unitários da detecção em `src/i18n.rs`** (falham: módulo não existe)
  - `override_wins`: `choose(Some("en"), None, None, Some("pt_BR.UTF-8"), || unreachable!()) == En`; `Some("pt")` com `LANG=en_US` → `Pt`; `Some("pt_PT")` → `Pt`.
  - `override_ignores_empty_and_unknown`: `Some("")` e `Some("klingon")` com `LANG=pt_BR.UTF-8` → `Pt`.
  - `posix_order`: `LC_ALL=en_US` vence `LANG=pt_BR`; `LC_MESSAGES=pt_BR` vence `LANG=en_US`; `LANG=pt-BR` → `Pt`.
  - `c_utf8_does_not_decide`: `LC_ALL` em `C`, `POSIX`, `C.UTF-8`, `POSIX.UTF-8`, `""` com `LANG=pt_BR.UTF-8` → `Pt`.
  - `unsupported_locale_is_english`: `LANG=fr_FR.UTF-8` → `En` sem chamar o closure (`|| unreachable!()`); `LANG=pt_BR.UTF-8@euro` → `Pt`.
  - `apple_decides_last`: sem variáveis, closure `Some("pt-BR")` → `Pt`; `Some("es-ES")` → `En`; `None` → `En`.
  - `parses_apple_languages`: `"(\n    \"pt-BR\",\n    \"en-US\"\n)\n"` → `Some("pt-BR")`; `"(\n    en,\n    fr\n)"` → `Some("en")`; `"(\n)"` → `None`.
  - `missing_defaults_binary_is_none`: `apple_languages("/nao/existe/defaults") == None`.
  - `english_catalog`: `En.var_count(1) == "1 variable"`, `En.var_count(3) == "3 variables"`, `Pt.var_count(1) == "1 variável"`, `En.copied("value", "X") == "copied: value of X"`, `En.no_match() == "no variable matches "`.

- [ ] **Step 2:** `cargo test i18n` → FAIL (não compila: `i18n` não existe).

- [ ] **Step 3:** implementar `src/i18n.rs` com as interfaces acima e `mod i18n;` em `main.rs`.

- [ ] **Step 4:** trocar cada texto fixo pela chamada `i18n::lang().<método>()`:
  - `output.rs`: `MASKED_NOTE` vira `format!("  {}", lang().masked())`; rodapé do cartão com `value_copied()` e `format!("❌ {}", lang().copy_failed(e))`.
  - `main.rs`: `no_match()`, `table_error(e)`, `write_error(e)`.
  - `cli.rs`: sai `const USAGE`; os erros devolvem `lang().usage().to_string()`.
  - `clipboard.rs`, `vars.rs`: os `Err` com `clip_*`, `file_not_found`, `read_error`, `zsh_failed`, `zsh_exit`, `signal()`.
  - `tui/mod.rs`: `KEYS` vira função `keys(lang) -> [(&str, &str); 5]` com `word_line/name/value/search/quit`; cabeçalho com `header_name()`/`header_value()` e `name_width` com `.max(width(lang().header_name()))`; título `" get-env · {var_count} "` e `"· {filter}: {f} "`; `copy_failed(e)`; linha de zero resultados com `no_match()`; `esc_clears()`.
  - `tui/app.rs`: `copied_status` usa `copied(word, name)`.
  - Unitários existentes seguem em pt; o de título passa a esperar `" get-env · 1 variável · filtro: fake "`.

- [ ] **Step 5: testes de CLI em `tests/cli.rs`**
  - `run` e `run_tty` passam `GET_ENV_LANG=pt` antes de `.envs(...)`; `no_query_pipe_lists_all_sorted` espera `"A=1\nB=2\nGET_ENV_LANG=pt\nPATH=/usr/bin:/bin\n"`; `bad_args_rc2` passa a exigir também `err.contains("[-h|--help]")`.
  - `english_by_override`: `GET_ENV_LANG=en`, `["zzz"]` → stderr `"❌ no variable matches zzz\n"`, rc 1; `["-f", "/nao/existe.env"]` → `"❌ file not found: /nao/existe.env\n"`.
  - `english_by_lang`: `GET_ENV_LANG=""`, `LANG=en_US.UTF-8`, `["zzz"]` → `"❌ no variable matches zzz\n"`.
  - `c_locale_defers_to_lang`: `GET_ENV_LANG=""`, `LC_ALL=C.UTF-8`, `LANG=pt_BR.UTF-8`, `["zzz"]` → `"❌ nenhuma variável casa com zzz\n"`.
  - `english_card_tty`: `run_tty(["fake_tok"], GET_ENV_LANG=en, NO_COLOR=1, FAKE_TOKEN=abcdefgh)` → contém `"🔒 masked"` e `"✅ value copied"`, rc 0.

- [ ] **Step 6:** `cargo fmt --check && cargo clippy -- -D warnings && cargo test` → tudo verde; contar testes (base: 87 + 12).

- [ ] **Step 7:** commit via `/commit`.

### Task 2: `--help`

**Files:**
- Modify: `src/i18n.rs`, `src/cli.rs`, `src/output.rs`, `src/main.rs`
- Test: `src/output.rs`, `src/cli.rs` (unitários), `tests/cli.rs`

**Interfaces:**
- Consumes: `Lang`, `lang()` da Task 1; `Palette`, `paint`, `width`.
- Produces:
  - `src/i18n.rs`: `pub enum RowKind { Plain, Keys, Examples }`, `pub struct Section { pub title: &'static str, pub kind: RowKind, pub rows: &'static [(&'static str, &'static str)] }`, `pub struct HelpDoc { pub tagline: &'static str, pub sections: &'static [Section] }`, `impl Lang { pub fn help(self) -> &'static HelpDoc }`.
  - `src/cli.rs`: `pub fn wants_help(args: &[String]) -> bool` (algum argumento é `-h` ou `--help`, em qualquer posição, inclusive depois de `-f`).
  - `src/output.rs`: `pub fn help(doc: &HelpDoc, palette: Option<&Palette>) -> String`.

Conteúdo pt (linhas `(esquerda, direita)`; o en é a tradução direta, com `USAGE`, `ARGUMENTS`, `BEHAVIOR`, `EXAMPLES`, `TABLE`, `VARIABLES`):

| seção | tipo | linhas |
|---|---|---|
| `USO` | Plain | `("get-env [nome] [-f\|--file <arquivo.env>]", "")`, `("get-env -h\|--help", "")` |
| `ARGUMENTOS` | Plain | `("nome", "parte do nome; aceita letras salteadas")`, `("-f, --file <arq>", "lê as variáveis do arquivo .env, não do ambiente")`, `("-h, --help", "mostra esta ajuda")` |
| `COMPORTAMENTO` | Plain | `("1 resultado", "mostra o cartão e copia o valor")`, `("vários ou sem nome", "abre a tabela interativa, filtrada pelo nome")`, `("em pipe", "imprime NOME=valor mascarado, sem copiar")` |
| `EXEMPLOS` | Examples | `("get-env openai_key", "acha OPENAI_API_KEY e copia o valor")`, `("get-env -f .env db", "procura só no .env")`, `("get-env \| grep AWS", "lista mascarada para outro comando")` |
| `TABELA` | Keys | `("↑ ↓", "move a seleção")`, `("⏎", "copia NOME='valor'")`, `("n", "copia o nome")`, `("v", "copia o valor")`, `("/", "busca; Esc limpa a busca")`, `("Esc", "limpa o filtro ou sai")`, `("q", "sai")`, `("⧉", "clique copia a linha")`, `("2× clique", "no nome ou no valor copia o que foi clicado")` |
| `VARIÁVEIS` | Plain | `("GET_ENV_LANG", "pt ou en; senão LC_ALL, LC_MESSAGES, LANG, macOS")`, `("GET_ENV_THEME", "dark, light ou ansi; senão o tema do macOS")`, `("GET_ENV_CLIP_CMD", "recebe a cópia no stdin (padrão: pbcopy)")`, `("NO_COLOR", "desliga as cores")` |

Tagline pt: `acha variáveis de ambiente pelo nome aproximado`; en: `finds environment variables by approximate name`. (Desvio da spec, chamada do plano: "tolera erro de digitação" virou "aceita letras salteadas", que é o que a busca faz.)

Layout: `get-env — <tagline>`, linha em branco, e cada seção como título, linhas com recuo de 2 e linha em branco entre seções. A coluna esquerda tem a largura da maior célula esquerda com direita não vazia, mais 2; célula `Keys` é desenhada como `" {k} "`. Cores: `get-env` em `title`, tagline em `value`, títulos em `header`, esquerda em `name` (`key` nas `Keys`), direita em `value` (`secondary` nas `Examples`). Sem paleta, texto puro.

- [ ] **Step 1: unitários** (falham)
  - `cli.rs` `help_flag_anywhere`: `wants_help` verdadeiro para `["-h"]`, `["--help"]`, `["--bogus", "-h"]`, `["a", "b", "--help"]`, `["-f", "-h"]`; falso para `["help"]`, `["-f", "x.env"]`, `[]`.
  - `output.rs` `help_fits_72_columns`: para `Lang::Pt.help()` e `Lang::En.help()`, toda linha de `help(doc, None)` tem `width <= 72`, e o texto contém todas as células das seções.
  - `output.rs` `help_plain_has_no_escapes_and_colored_has`: `help(Lang::Pt.help(), None)` sem `\x1b`, começa com `"get-env — acha variáveis"`; com `Some(&Palette::of(Theme::Dark))` contém `"\x1b["` e, sem escapes, é igual ao texto puro.

- [ ] **Step 2:** `cargo test help` → FAIL.
- [ ] **Step 3:** implementar `HelpDoc` pt/en, `wants_help` e `output::help`.
- [ ] **Step 4:** em `main.rs`, antes do `cli::parse`, coletar os argumentos; se `wants_help`, imprimir `output::help(i18n::lang().help(), paleta só se stdout for TTY)` e sair com `ExitCode::SUCCESS`, sem ler ambiente nem arquivo.
- [ ] **Step 5: testes de CLI**
  - `help_pipe_pt`: `run(["--help"], GET_ENV_THEME=dark)` → rc 0, stderr vazio, stdout começa com `"get-env — acha variáveis"`, contém `"USO"` e `"GET_ENV_LANG"`.
  - `help_pipe_has_no_escapes`: o mesmo, sem nenhum `\x1b` no stdout.
  - `help_english`: `GET_ENV_LANG=en`, `["-h"]` → contém `"USAGE"` e `"finds environment variables"`.
  - `help_wins_over_bad_args`: `["--bogus", "-h"]`, `["a", "b", "--help"]`, `["-f", "-h"]` → rc 0, stdout contém `"USO"`.
  - `help_tty_is_colored`: `run_tty(["--help"], GET_ENV_THEME=dark)` → contém `"\x1b[38;2;"`; sem escapes, contém `"TABELA"`.
- [ ] **Step 6:** gate completo, contar testes.
- [ ] **Step 7:** commit via `/commit`.

### Task 3: README e E2E

**Files:**
- Modify: `README.md`

- [ ] **Step 1:** README: `-h|--help` no bloco de uso; `GET_ENV_LANG` na tabela de configuração com a ordem de detecção; frase dizendo que o comando fala pt-BR ou inglês sozinho (en como fallback).
- [ ] **Step 2: E2E pelo binário** (`cargo build`, env sintético, clipboard em arquivo temporário), nos dois idiomas:
  - pipe: `env -i PATH=/usr/bin:/bin LANG=en_US.UTF-8 target/debug/get-env --help`, idem `LANG=pt_BR.UTF-8`, `zzz`, `-f /nao/existe.env`, `--bogus`;
  - TTY: `script -q /dev/null env -i PATH=/usr/bin:/bin GET_ENV_THEME=dark LANG=en_US.UTF-8 GET_ENV_CLIP_CMD="cat > /tmp/get-env-e2e-clip" FAKE_TOKEN=abcdefgh target/debug/get-env fake_tok` e o mesmo com `--help`;
  - sem variáveis POSIX: `env -i PATH=/usr/bin:/bin target/debug/get-env zzz` segue o `AppleLanguages` desta máquina (pt-BR).
- [ ] **Step 3:** commit via `/commit`.
