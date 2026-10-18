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

#[test]
fn unique_tty_prints_masked_and_copies_value() {
    let clip = temp_path("unique-tty");
    let (out, rc) = run_tty(
        &["fake_tok"],
        &[("FAKE_TOKEN", "abcdefgh"), ("OTHER", "1")],
        &clip_into(&clip),
    );
    let copied = std::fs::read_to_string(&clip).unwrap();
    std::fs::remove_file(&clip).unwrap();
    assert!(out.contains("FAKE_TOKEN=abcde…\n"), "{out:?}");
    assert!(out.contains("copiado: valor de FAKE_TOKEN\n"), "{out:?}");
    assert_eq!(rc, 0);
    assert_eq!(copied, "abcdefgh");
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
    assert_eq!(err, "nenhuma variável casa com zzz\n");
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
    assert_eq!(out, "A=1\nB=2\nPATH=/usr/bin:/bin\n");
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
    assert_eq!(err, "arquivo não encontrado: /nao/existe.env\n");
    assert_eq!(rc, 1);
}

#[test]
fn clipboard_failure_rc1() {
    let (out, rc) = run_tty(&["fake_tok"], &[("FAKE_TOKEN", "abcdefgh")], "exit 3");
    assert!(
        out.contains("erro ao copiar: comando de clipboard saiu com 3\n"),
        "{out:?}"
    );
    assert_eq!(rc, 1);
}

#[test]
fn bad_args_rc2() {
    let (_, err, rc) = run(&["a", "b"], &[]);
    assert_eq!(rc, 2);
    assert!(err.starts_with("uso: get-env"), "{err:?}");
}
