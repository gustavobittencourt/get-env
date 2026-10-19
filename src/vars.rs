//! De onde vêm as variáveis: o ambiente do processo ou um arquivo `.env`.

use std::ffi::OsString;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Var {
    pub name: String,
    pub value: String,
}

/// Faz `source` do arquivo e imprime `nome\0valor\0` de cada nome pedido.
/// O valor é cortado no primeiro NUL, como no ambiente real; sem o corte,
/// um NUL desalinharia os pares e um valor sairia no lugar do nome.
const SOURCE_SCRIPT: &str = r#"set -a; source "$1" >/dev/null; set +a; shift; for n in "$@"; do printf "%s\0%s\0" "$n" "${${(P)n}%%$'\0'*}"; done"#;

/// Converte pares do ambiente (não UTF-8 vira U+FFFD) e ordena por nome.
pub fn from_pairs(pairs: impl IntoIterator<Item = (OsString, OsString)>) -> Vec<Var> {
    let mut vars: Vec<Var> = pairs
        .into_iter()
        .map(|(name, value)| Var {
            name: name.to_string_lossy().into_owned(),
            value: value.to_string_lossy().into_owned(),
        })
        .collect();
    vars.sort_by(|a, b| a.name.cmp(&b.name));
    vars
}

pub fn from_env() -> Vec<Var> {
    from_pairs(std::env::vars_os())
}

/// Só as variáveis definidas no arquivo, com o valor que o `source` do zsh
/// produz, como a função `get_env` do profile.
pub fn from_file(path: &Path) -> Result<Vec<Var>, String> {
    if !path.is_file() {
        return Err(format!("arquivo não encontrado: {}", path.display()));
    }
    let content =
        std::fs::read(path).map_err(|e| format!("erro ao ler {}: {e}", path.display()))?;
    let names = defined_names(&String::from_utf8_lossy(&content));
    if names.is_empty() {
        return Ok(Vec::new());
    }

    let output = std::process::Command::new("/bin/zsh")
        .args(["-f", "-c", SOURCE_SCRIPT, "zsh"])
        .arg(path)
        .args(&names)
        .output()
        .map_err(|e| format!("erro ao rodar o zsh: {e}"))?;
    // O stderr do zsh não aparece: ele pode citar trechos do arquivo, e
    // portanto valores sem máscara.
    if !output.status.success() {
        let code = output
            .status
            .code()
            .map_or("sinal".to_string(), |c| c.to_string());
        return Err(format!(
            "erro ao ler {}: o zsh saiu com {code}",
            path.display()
        ));
    }

    let fields: Vec<&[u8]> = output.stdout.split(|&b| b == 0).collect();
    Ok(from_pairs(fields.as_chunks::<2>().0.iter().map(
        |[name, value]| {
            (
                OsString::from(String::from_utf8_lossy(name).into_owned()),
                OsString::from(String::from_utf8_lossy(value).into_owned()),
            )
        },
    )))
}

/// Nomes atribuídos no arquivo, na ordem e sem repetição. Mesma regra do
/// awk da `get_env`: ignora comentários, tira `export ` e exige `NOME=`.
fn defined_names(content: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim_start();
        let line = line
            .strip_prefix("export")
            .filter(|rest| rest.starts_with(char::is_whitespace))
            .map_or(line, str::trim_start);
        let name: String = line
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let starts_ok = name
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
        let after = line[name.len()..].trim_start();
        if starts_ok && after.starts_with('=') && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::ffi::OsStringExt;

    fn var(name: &str, value: &str) -> Var {
        Var {
            name: name.to_string(),
            value: value.to_string(),
        }
    }

    fn temp_file(test: &str, content: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("get-env-{}-{test}.env", std::process::id()));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn from_pairs_sorts_and_handles_non_utf8() {
        let pairs = vec![
            (OsString::from("B"), OsString::from("2")),
            (OsString::from("A"), OsString::from_vec(vec![0x66, 0xff])),
        ];
        assert_eq!(
            from_pairs(pairs),
            vec![var("A", "f\u{FFFD}"), var("B", "2")]
        );
    }

    #[test]
    fn from_file_matches_zsh_source() {
        let path = temp_file(
            "zsh-source",
            "# comentário\nexport A=\"x y\"\nB=$A-z   # fim de linha\nC='lit $A'\n\nD=1\nD=2\n",
        );
        let result = from_file(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            result,
            Ok(vec![
                var("A", "x y"),
                var("B", "x y-z"),
                var("C", "lit $A"),
                var("D", "2"),
            ])
        );
    }

    #[test]
    fn from_file_error_hides_zsh_output() {
        let path = temp_file(
            "zsh-error",
            "FAKE_TOKEN= sk-fakesecret123\nA=${x:?faltou}\n",
        );
        let result = from_file(&path);
        std::fs::remove_file(&path).unwrap();
        let err = result.unwrap_err();
        assert!(!err.contains("sk-fakesecret123"), "{err}");
        assert!(
            err.starts_with(&format!("erro ao ler {}", path.display())),
            "{err}"
        );
    }

    #[test]
    fn from_file_ignores_source_stdout() {
        let path = temp_file("zsh-echo", "echo carregando\nFAKE_TOKEN=abc\n");
        let result = from_file(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(result, Ok(vec![var("FAKE_TOKEN", "abc")]));
    }

    /// Byte nulo num valor não pode desalinhar os pares nome/valor (um
    /// segredo sairia no lugar do nome, sem máscara). No ambiente real o
    /// valor termina no NUL, e é assim que ele volta.
    #[test]
    fn from_file_value_with_nul_keeps_pairs_aligned() {
        let path = temp_file(
            "zsh-nul",
            "FAKE_NOTE=$'abc\\0xyz'\nFAKE_API_KEY=sk-fakesecret999\n",
        );
        let result = from_file(&path);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            result,
            Ok(vec![
                var("FAKE_API_KEY", "sk-fakesecret999"),
                var("FAKE_NOTE", "abc"),
            ])
        );
    }

    #[test]
    fn from_file_missing() {
        assert_eq!(
            from_file(Path::new("/nao/existe.env")),
            Err("arquivo não encontrado: /nao/existe.env".to_string())
        );
    }
}
