//! Roda o binário com ambiente sintético. Nada aqui lê o ambiente real.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_get-env");

fn temp_path(test: &str) -> PathBuf {
    std::env::temp_dir().join(format!("get-env-cli-{}-{test}", std::process::id()))
}

fn run(args: &[&str], envs: &[(&str, &str)]) -> (String, String, i32) {
    let out = Command::new(BIN)
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("GET_ENV_LANG", "pt")
        .envs(envs.iter().copied())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    (
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
        out.status.code().unwrap(),
    )
}

/// Roda com stdout num TTY via `script`, que junta stdout e stderr num só
/// fluxo, começa com `^D\x08\x08` e termina linhas com `\r\n`.
fn run_tty(args: &[&str], envs: &[(&str, &str)], clip_cmd: &str) -> (String, i32) {
    let out = Command::new("/usr/bin/script")
        .args(["-q", "/dev/null", BIN])
        .args(args)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("GET_ENV_CLIP_CMD", clip_cmd)
        .env("GET_ENV_LANG", "pt")
        .envs(envs.iter().copied())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    let text = text.strip_prefix("^D\x08\x08").unwrap_or(&text);
    (text.replace("\r\n", "\n"), out.status.code().unwrap())
}

fn clip_into(path: &Path) -> String {
    format!("cat > '{}'", path.display())
}

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
fn unique_tty_prints_card_and_copies_value() {
    let clip = temp_path("unique-tty");
    let (out, rc) = run_tty(
        &["fake_tok"],
        &[
            ("FAKE_TOKEN", "abcdefgh"),
            ("OTHER", "1"),
            ("NO_COLOR", "1"),
        ],
        &clip_into(&clip),
    );
    let copied = std::fs::read_to_string(&clip).unwrap();
    std::fs::remove_file(&clip).unwrap();
    let out = strip(&out);
    assert!(out.contains("╭─ FAKE_TOKEN "), "{out:?}");
    assert!(out.contains("│ abcde…  🔒 mascarado"), "{out:?}");
    assert!(out.contains("✅ valor copiado"), "{out:?}");
    assert!(!out.contains("copiado: valor de"), "{out:?}");
    assert_eq!(rc, 0);
    assert_eq!(copied, "abcdefgh");
}

#[test]
fn unique_tty_uses_forced_theme() {
    let clip = temp_path("unique-theme");
    let (out, rc) = run_tty(
        &["fake_tok"],
        &[("FAKE_TOKEN", "abcdefgh"), ("GET_ENV_THEME", "dark")],
        &clip_into(&clip),
    );
    std::fs::remove_file(&clip).unwrap();
    assert!(out.contains("\x1b[38;2;180;190;254m"), "{out:?}");
    assert_eq!(rc, 0);
}

#[test]
fn pipe_output_has_no_escapes() {
    let file = temp_path("no-escapes.env");
    std::fs::write(&file, "FILE_VAR=ok\n").unwrap();
    let theme = ("GET_ENV_THEME", "dark");
    let vars = [
        ("FAKE_TOKEN", "abcdefgh"),
        ("OPENAI_API_KEY", "sk-123456"),
        ("OPENAI_ORG", "org1"),
        ("FAKE_MSG", "a\x1b]0;PWNED\x07\x1b[31mred"),
        theme,
    ];
    let cases: [&[&str]; 6] = [
        &["fake_tok"],
        &["fake_msg"],
        &["openai"],
        &["zzz"],
        &["-f", "/nao/existe.env"],
        &["-f", file.to_str().unwrap()],
    ];
    for args in cases {
        let (out, err, _) = run(args, &vars);
        assert!(
            !out.contains('\x1b') && !err.contains('\x1b'),
            "{args:?}: {out:?} {err:?}"
        );
    }
    std::fs::remove_file(&file).unwrap();
    let (out, _, rc) = run(&["fake_tok"], &vars);
    assert_eq!(out, "FAKE_TOKEN=abcde…\n");
    assert_eq!(rc, 0);
}

#[test]
fn unique_pipe_prints_masked_no_clipboard() {
    let clip = temp_path("unique-pipe");
    let clip_cmd = clip_into(&clip);
    let (out, err, rc) = run(
        &["fake_tok"],
        &[("FAKE_TOKEN", "abcdefgh"), ("GET_ENV_CLIP_CMD", &clip_cmd)],
    );
    assert_eq!(out, "FAKE_TOKEN=abcde…\n");
    assert_eq!(err, "");
    assert_eq!(rc, 0);
    assert!(!clip.exists());
}

#[test]
fn zero_matches_rc1() {
    let (out, err, rc) = run(&["zzz"], &[]);
    assert_eq!(out, "");
    assert_eq!(err, "❌ nenhuma variável casa com zzz\n");
    assert_eq!(rc, 1);
}

#[test]
fn many_pipe_lists_masked() {
    let (out, _, rc) = run(
        &["openai"],
        &[("OPENAI_API_KEY", "sk-123456"), ("OPENAI_ORG", "org1")],
    );
    assert_eq!(out, "OPENAI_API_KEY=sk-12…\nOPENAI_ORG=org1\n");
    assert_eq!(rc, 0);
}

#[test]
fn no_query_pipe_lists_all_sorted() {
    let (out, _, rc) = run(&[], &[("B", "2"), ("A", "1")]);
    // O CoreFoundation, que vem com o AppKit do pasteboard, cria
    // `__CF_USER_TEXT_ENCODING` quando ela falta; num shell do macOS ela
    // já existe, só o ambiente zerado do teste a vê nascer.
    let out: String = out
        .lines()
        .filter(|l| !l.starts_with("__CF_USER_TEXT_ENCODING="))
        .map(|l| format!("{l}\n"))
        .collect();
    assert_eq!(out, "A=1\nB=2\nGET_ENV_LANG=pt\nPATH=/usr/bin:/bin\n");
    assert_eq!(rc, 0);
}

#[test]
fn file_flag_reads_only_file() {
    let file = temp_path("file-flag.env");
    std::fs::write(&file, "FILE_VAR=ok\n").unwrap();
    let (out, _, rc) = run(&["-f", file.to_str().unwrap()], &[("ENV_VAR", "no")]);
    std::fs::remove_file(&file).unwrap();
    assert_eq!(out, "FILE_VAR=ok\n");
    assert_eq!(rc, 0);
}

#[test]
fn missing_file_rc1() {
    let (out, err, rc) = run(&["-f", "/nao/existe.env"], &[]);
    assert_eq!(out, "");
    assert_eq!(err, "❌ arquivo não encontrado: /nao/existe.env\n");
    assert_eq!(rc, 1);
}

#[test]
fn clipboard_failure_rc1() {
    let (out, rc) = run_tty(
        &["fake_tok"],
        &[("FAKE_TOKEN", "abcdefgh"), ("NO_COLOR", "1")],
        "exit 3",
    );
    let out = strip(&out);
    assert!(
        out.contains("❌ erro ao copiar: comando de clipboard saiu com 3"),
        "{out:?}"
    );
    assert_eq!(rc, 1);
}

#[test]
fn bad_args_rc2() {
    let (_, err, rc) = run(&["a", "b"], &[]);
    assert_eq!(rc, 2);
    assert!(err.starts_with("uso: get-env"), "{err:?}");
    assert!(err.contains("[-h|--help]"), "{err:?}");
}

#[test]
fn english_by_override() {
    let (_, err, rc) = run(&["zzz"], &[("GET_ENV_LANG", "en")]);
    assert_eq!(err, "❌ no variable matches zzz\n");
    assert_eq!(rc, 1);
    let (_, err, _) = run(&["-f", "/nao/existe.env"], &[("GET_ENV_LANG", "en")]);
    assert_eq!(err, "❌ file not found: /nao/existe.env\n");
}

#[test]
fn english_by_lang() {
    let (_, err, rc) = run(&["zzz"], &[("GET_ENV_LANG", ""), ("LANG", "en_US.UTF-8")]);
    assert_eq!(err, "❌ no variable matches zzz\n");
    assert_eq!(rc, 1);
}

#[test]
fn c_locale_defers_to_lang() {
    let envs = [
        ("GET_ENV_LANG", ""),
        ("LC_ALL", "C.UTF-8"),
        ("LANG", "pt_BR.UTF-8"),
    ];
    let (_, err, _) = run(&["zzz"], &envs);
    assert_eq!(err, "❌ nenhuma variável casa com zzz\n");
}

#[test]
fn english_card_tty() {
    let clip = temp_path("english-card");
    let (out, rc) = run_tty(
        &["fake_tok"],
        &[
            ("FAKE_TOKEN", "abcdefgh"),
            ("GET_ENV_LANG", "en"),
            ("NO_COLOR", "1"),
        ],
        &clip_into(&clip),
    );
    std::fs::remove_file(&clip).unwrap();
    let out = strip(&out);
    assert!(out.contains("🔒 masked"), "{out:?}");
    assert!(out.contains("✅ value copied"), "{out:?}");
    assert_eq!(rc, 0);
}

#[test]
fn help_pipe_pt() {
    let (out, err, rc) = run(&["--help"], &[("GET_ENV_THEME", "dark")]);
    assert_eq!(rc, 0);
    assert_eq!(err, "");
    assert!(out.starts_with("get-env — acha variáveis"), "{out:?}");
    assert!(
        out.contains("USO") && out.contains("GET_ENV_LANG"),
        "{out:?}"
    );
}

#[test]
fn help_pipe_has_no_escapes() {
    let (out, _, _) = run(&["--help"], &[("GET_ENV_THEME", "dark")]);
    assert!(!out.contains('\x1b'), "{out:?}");
}

#[test]
fn help_english() {
    let (out, _, rc) = run(&["-h"], &[("GET_ENV_LANG", "en")]);
    assert_eq!(rc, 0);
    assert!(out.contains("USAGE"), "{out:?}");
    assert!(out.contains("finds environment variables"), "{out:?}");
}

#[test]
fn help_wins_over_bad_args() {
    for args in [&["--bogus", "-h"][..], &["a", "b", "--help"], &["-f", "-h"]] {
        let (out, _, rc) = run(args, &[]);
        assert_eq!(rc, 0, "{args:?}");
        assert!(out.contains("USO"), "{args:?}: {out:?}");
    }
}

#[test]
fn help_tty_is_colored() {
    let clip = temp_path("help-tty");
    let (out, rc) = run_tty(&["--help"], &[("GET_ENV_THEME", "dark")], &clip_into(&clip));
    assert_eq!(rc, 0);
    assert!(out.contains("\x1b[38;2;"), "{out:?}");
    assert!(strip(&out).contains("TABELA"), "{out:?}");
    assert!(!clip.exists());
}

#[test]
fn help_closed_pipe_exits_cleanly() {
    let mut child = Command::new(BIN)
        .arg("--help")
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("GET_ENV_LANG", "pt")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(!err.contains("panicked"), "{err}");
    assert_eq!(out.status.code(), Some(0));
}

#[test]
fn closed_pipe_exits_cleanly() {
    let vars: Vec<(String, String)> = (0..3000)
        .map(|i| (format!("VAR_{i:05}"), "x".repeat(40)))
        .collect();
    let mut child = Command::new(BIN)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .envs(vars)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(!err.contains("panicked"), "{err}");
    assert_eq!(out.status.code(), Some(0));
}

/// Valor com formato de credencial sai mascarado no pipe mesmo com nome
/// inocente; montado em runtime para o fonte não parecer segredo.
#[test]
fn pipe_masks_secret_shaped_value() {
    let url = format!("postgres://app:{}@db.local/app", "hunter2");
    let aws = format!("AK{}{}", "IA", "X".repeat(16));
    let (out, _, rc) = run(&[], &[("DB", url.as_str()), ("CLOUD_ID", aws.as_str())]);
    assert!(out.contains("DB=postg…\n"), "{out:?}");
    assert!(out.contains("CLOUD_ID=AKIAX…\n"), "{out:?}");
    assert!(!out.contains("hunter2"), "{out:?}");
    assert_eq!(rc, 0);
}

/// Cópia pelo pasteboard nativo, num pasteboard com nome próprio para não
/// tocar no clipboard do user. `GET_ENV_CLIP_CMD` vazio escolhe o nativo.
#[cfg(target_os = "macos")]
mod native_clipboard {
    use super::run_tty;
    use objc2::rc::Retained;
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    use objc2_foundation::NSString;
    use std::time::{Duration, Instant};

    struct Board(String);

    impl Board {
        fn new(test: &str) -> Self {
            Board(format!("get-env-cli-{}-{test}", std::process::id()))
        }

        fn pb(&self) -> Retained<NSPasteboard> {
            NSPasteboard::pasteboardWithName(&NSString::from_str(&self.0))
        }

        fn text(&self) -> Option<String> {
            let string_type = unsafe { NSPasteboardTypeString };
            self.pb().stringForType(string_type).map(|s| s.to_string())
        }

        /// O user aperta ⌘C em outra coisa.
        fn user_copies(&self, text: &str) {
            let pb = self.pb();
            pb.clearContents();
            let string_type = unsafe { NSPasteboardTypeString };
            assert!(pb.setString_forType(&NSString::from_str(text), string_type));
        }

        /// Copia FAKE_TOKEN pelo binário, com stdout num TTY.
        fn copy_fake(&self, args: &[&str], clear_secs: &str) {
            let mut all = vec!["fake_tok"];
            all.extend_from_slice(args);
            let (out, rc) = run_tty(
                &all,
                &[
                    ("FAKE_TOKEN", "abcdefgh"),
                    ("GET_ENV_PASTEBOARD", &self.0),
                    ("GET_ENV_CLIP_CLEAR", clear_secs),
                ],
                "",
            );
            assert_eq!(rc, 0, "{out:?}");
            assert_eq!(self.text().as_deref(), Some("abcdefgh"));
        }

        /// Espera até `secs` segundos o pasteboard ficar vazio.
        fn wait_cleared(&self, secs: u64) -> bool {
            let deadline = Instant::now() + Duration::from_secs(secs);
            while Instant::now() < deadline {
                if self.text().is_none() {
                    return true;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            false
        }
    }

    impl Drop for Board {
        fn drop(&mut self) {
            let pb = self.pb();
            unsafe { objc2::msg_send![&*pb, releaseGlobally] }
        }
    }

    #[test]
    fn copied_value_is_cleared_after_deadline() {
        let b = Board::new("deadline");
        b.copy_fake(&[], "1");
        assert!(b.wait_cleared(10), "{:?}", b.text());
    }

    #[test]
    fn user_copy_before_deadline_is_kept() {
        let b = Board::new("user-copy");
        b.copy_fake(&[], "1");
        b.user_copies("do user");
        assert!(!b.wait_cleared(3));
        assert_eq!(b.text().as_deref(), Some("do user"));
    }

    #[test]
    fn no_clear_flag_keeps_value() {
        let b = Board::new("no-clear");
        b.copy_fake(&["--no-clear"], "1");
        assert!(!b.wait_cleared(3));
    }

    #[test]
    fn zero_seconds_keeps_value() {
        let b = Board::new("zero");
        b.copy_fake(&[], "0");
        assert!(!b.wait_cleared(3));
    }
}
