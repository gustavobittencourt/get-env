//! Cópia para o clipboard por um comando externo, `pbcopy` por padrão.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::i18n::lang;

/// Roda `cmd` com `sh -c` e entrega `text` no stdin, sem `\n` final.
pub fn copy_with(cmd: &str, text: &str) -> Result<(), String> {
    let mut child = Command::new("sh")
        .args(["-c", cmd])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .spawn()
        .map_err(|e| lang().clip_failed(&e))?;
    if let Some(mut stdin) = child.stdin.take() {
        // Comando que não lê o stdin fecha o pipe; o status abaixo decide.
        let _ = stdin.write_all(text.as_bytes());
    }
    let status = child.wait().map_err(|e| lang().clip_failed(&e))?;
    match status.code() {
        Some(0) => Ok(()),
        Some(code) => Err(lang().clip_exit(code)),
        None => Err(lang().clip_signal().to_string()),
    }
}

/// Usa `GET_ENV_CLIP_CMD`, ou `pbcopy` quando ela está vazia ou ausente.
pub fn copy(text: &str) -> Result<(), String> {
    let cmd = std::env::var("GET_ENV_CLIP_CMD")
        .ok()
        .filter(|c| !c.is_empty())
        .unwrap_or_else(|| "pbcopy".to_string());
    copy_with(&cmd, text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_with_writes_stdin_without_newline() {
        let path = std::env::temp_dir().join(format!("get-env-{}-clip", std::process::id()));
        let result = copy_with(&format!("cat > '{}'", path.display()), "abc");
        let content = std::fs::read_to_string(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        assert_eq!(result, Ok(()));
        assert_eq!(content, "abc");
    }

    #[test]
    fn copy_with_reports_failure() {
        assert_eq!(
            copy_with("exit 3", "x"),
            Err("comando de clipboard saiu com 3".to_string())
        );
    }
}
