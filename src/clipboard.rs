//! Cópia para o clipboard. No macOS escreve direto no `NSPasteboard`,
//! marcado como confidencial, e agenda a limpeza; `GET_ENV_CLIP_CMD`
//! troca isso por um comando externo, sem limpeza.

use std::io::Write;
use std::process::{Command, Stdio};

use crate::i18n::lang;

/// Segundos até a limpeza quando nada diz o contrário.
pub const DEFAULT_CLEAR_SECS: u64 = 60;

/// Flag oculta do modo ajudante, que dorme e limpa o clipboard.
const HELPER_FLAG: &str = "--clear-clipboard-after";

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

/// Segundos até limpar: `--no-clear` ou `GET_ENV_CLIP_CLEAR=0` desligam;
/// valor ausente ou inválido fica no padrão.
pub fn clear_after(no_clear: bool, env: Option<&str>) -> Option<u64> {
    let secs = env
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(DEFAULT_CLEAR_SECS);
    (!no_clear && secs > 0).then_some(secs)
}

/// Usa `GET_ENV_CLIP_CMD` quando ela tem valor; senão o pasteboard do macOS,
/// e agenda a limpeza para daqui a `clear` segundos.
pub fn copy(text: &str, clear: Option<u64>) -> Result<(), String> {
    match std::env::var("GET_ENV_CLIP_CMD") {
        Ok(cmd) if !cmd.is_empty() => copy_with(&cmd, text),
        _ => copy_native(text, clear),
    }
}

#[cfg(not(target_os = "macos"))]
fn copy_native(text: &str, _clear: Option<u64>) -> Result<(), String> {
    copy_with("pbcopy", text)
}

#[cfg(target_os = "macos")]
fn copy_native(text: &str, clear: Option<u64>) -> Result<(), String> {
    // Gancho dos testes: um pasteboard com nome, para não tocar no do user.
    let board = std::env::var("GET_ENV_PASTEBOARD")
        .ok()
        .filter(|b| !b.is_empty());
    let count = pasteboard::write_concealed(board.as_deref(), text)?;
    if let Some(secs) = clear {
        spawn_clear(secs, count, board.as_deref());
    }
    Ok(())
}

/// Relança o próprio binário no modo ajudante, numa sessão nova (`setsid`)
/// para sobreviver ao fim do CLI e ao fechamento do terminal. O ajudante
/// recebe só o prazo e o `changeCount`, nunca o valor, e nenhum ambiente.
#[cfg(target_os = "macos")]
fn spawn_clear(secs: u64, count: isize, board: Option<&str>) {
    use std::os::unix::process::CommandExt;

    unsafe extern "C" {
        fn setsid() -> i32;
    }
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut cmd = Command::new(exe);
    cmd.args([
        HELPER_FLAG,
        &secs.to_string(),
        "--change-count",
        &count.to_string(),
    ]);
    if let Some(board) = board {
        cmd.args(["--pasteboard", board]);
    }
    cmd.env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: entre o fork e o exec só roda `setsid`, que é
    // async-signal-safe e não aloca.
    unsafe {
        cmd.pre_exec(|| {
            setsid();
            Ok(())
        });
    }
    // Sem o ajudante a cópia continua valendo; só não expira.
    let _ = cmd.spawn();
}

/// Pedido do modo ajudante, lido do argv sem o argv[0].
#[derive(Debug, PartialEq, Eq)]
pub struct ClearJob {
    pub secs: u64,
    pub count: isize,
    pub board: Option<String>,
}

/// `Some` só quando o argv é exatamente o do ajudante.
pub fn helper_job(argv: &[String]) -> Option<ClearJob> {
    let args: Vec<&str> = argv.iter().map(String::as_str).collect();
    let (secs, count, board) = match args.as_slice() {
        [HELPER_FLAG, secs, "--change-count", count] => (secs, count, None),
        [
            HELPER_FLAG,
            secs,
            "--change-count",
            count,
            "--pasteboard",
            board,
        ] => (secs, count, Some(board.to_string())),
        _ => return None,
    };
    Some(ClearJob {
        secs: secs.parse().ok()?,
        count: count.parse().ok()?,
        board,
    })
}

/// Dorme e limpa o clipboard só se ninguém escreveu nele desde a cópia.
pub fn run_helper(job: &ClearJob) {
    std::thread::sleep(std::time::Duration::from_secs(job.secs));
    #[cfg(target_os = "macos")]
    pasteboard::clear_if_unchanged(job.board.as_deref(), job.count);
}

#[cfg(target_os = "macos")]
pub mod pasteboard {
    //! O `NSPasteboard`: o geral, ou um com nome nos testes.

    use objc2::rc::Retained;
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::{NSData, NSString};

    use crate::i18n::lang;

    /// Marca de conteúdo confidencial da convenção de nspasteboard.org:
    /// gerenciadores de clipboard não guardam nem mostram o item.
    pub const CONCEALED_TYPE: &str = "org.nspasteboard.ConcealedType";

    pub fn board(name: Option<&str>) -> Retained<NSPasteboard> {
        match name {
            Some(name) => NSPasteboard::pasteboardWithName(&NSString::from_str(name)),
            None => NSPasteboard::generalPasteboard(),
        }
    }

    /// Escreve o texto e a marca de confidencial num item só e devolve o
    /// `changeCount`, que só muda quando alguém volta a escrever.
    pub fn write_concealed(name: Option<&str>, text: &str) -> Result<isize, String> {
        let pb = board(name);
        pb.clearContents();
        // SAFETY: constante do AppKit, inicializada antes do `main`.
        let string_type = unsafe { NSPasteboardTypeString };
        let ok = pb.setString_forType(&NSString::from_str(text), string_type)
            && pb.setData_forType(Some(&NSData::new()), &NSString::from_str(CONCEALED_TYPE));
        if !ok {
            pb.clearContents();
            return Err(lang().clip_native_failed().to_string());
        }
        Ok(pb.changeCount())
    }

    /// Limpa só se o `changeCount` ainda é o da cópia; não lê o conteúdo.
    pub fn clear_if_unchanged(name: Option<&str>, count: isize) -> bool {
        let pb = board(name);
        if pb.changeCount() != count {
            return false;
        }
        pb.clearContents();
        true
    }
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

    #[test]
    fn clear_after_defaults_and_switches_off() {
        assert_eq!(clear_after(false, None), Some(60));
        assert_eq!(clear_after(false, Some("5")), Some(5));
        assert_eq!(clear_after(false, Some("lixo")), Some(60));
        assert_eq!(clear_after(false, Some("0")), None);
        assert_eq!(clear_after(true, None), None);
        assert_eq!(clear_after(true, Some("5")), None);
    }

    #[test]
    fn helper_job_parses_only_exact_argv() {
        let argv = |args: &[&str]| args.iter().map(|a| a.to_string()).collect::<Vec<_>>();
        assert_eq!(
            helper_job(&argv(&[HELPER_FLAG, "60", "--change-count", "7"])),
            Some(ClearJob {
                secs: 60,
                count: 7,
                board: None
            })
        );
        assert_eq!(
            helper_job(&argv(&[
                HELPER_FLAG,
                "1",
                "--change-count",
                "2",
                "--pasteboard",
                "b"
            ]))
            .and_then(|j| j.board),
            Some("b".to_string())
        );
        assert_eq!(
            helper_job(&argv(&[HELPER_FLAG, "x", "--change-count", "7"])),
            None
        );
        assert_eq!(helper_job(&argv(&["nome"])), None);
    }

    #[cfg(target_os = "macos")]
    mod native {
        use super::super::pasteboard::*;
        use objc2_app_kit::NSPasteboardTypeString;
        use objc2_foundation::NSString;

        /// Pasteboard com nome próprio: o clipboard do user fica intocado.
        struct Board(String);

        impl Board {
            fn new(test: &str) -> Self {
                Board(format!("get-env-test-{}-{test}", std::process::id()))
            }
            fn name(&self) -> Option<&str> {
                Some(&self.0)
            }
            fn text(&self) -> Option<String> {
                let string_type = unsafe { NSPasteboardTypeString };
                board(self.name())
                    .stringForType(string_type)
                    .map(|s| s.to_string())
            }
        }

        impl Drop for Board {
            fn drop(&mut self) {
                let pb = board(self.name());
                unsafe { objc2::msg_send![&*pb, releaseGlobally] }
            }
        }

        #[test]
        fn write_marks_value_as_concealed() {
            let b = Board::new("concealed");
            write_concealed(b.name(), "abc").unwrap();
            let types = board(b.name()).types().unwrap();
            let concealed = NSString::from_str(CONCEALED_TYPE);
            assert!(types.containsObject(&concealed));
            assert_eq!(b.text().as_deref(), Some("abc"));
        }

        #[test]
        fn clears_when_nobody_wrote_after() {
            let b = Board::new("unchanged");
            let count = write_concealed(b.name(), "abc").unwrap();
            assert!(clear_if_unchanged(b.name(), count));
            assert_eq!(b.text(), None);
        }

        /// O user copiou outra coisa (⌘C) antes do prazo: nada é limpo.
        #[test]
        fn keeps_what_the_user_copied_after() {
            let b = Board::new("changed");
            let count = write_concealed(b.name(), "abc").unwrap();
            write_concealed(b.name(), "do user").unwrap();
            assert!(!clear_if_unchanged(b.name(), count));
            assert_eq!(b.text().as_deref(), Some("do user"));
        }
    }
}
