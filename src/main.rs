mod cli;
mod clipboard;
mod format;
mod i18n;
mod matching;
mod output;
mod secrets;
mod theme;
mod tui;
mod vars;

use std::cell::OnceCell;
use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use format::masked_line;
use i18n::lang;
use output::Footer;
use theme::{Palette, Theme};
use vars::Var;

fn main() -> ExitCode {
    // O tema só é detectado quando alguma saída vai mesmo ser colorida.
    let theme = OnceCell::new();

    let argv: Vec<String> = std::env::args().skip(1).collect();
    if let Some(job) = clipboard::helper_job(&argv) {
        clipboard::run_helper(&job);
        return ExitCode::SUCCESS;
    }
    if cli::wants_help(&argv) {
        let palette = std::io::stdout().is_terminal().then(|| palette(&theme));
        // Pipe fechado (`get-env -h | head -1`) não é erro.
        let _ = write!(
            std::io::stdout(),
            "{}",
            output::help(lang().help(), palette.as_ref())
        );
        return ExitCode::SUCCESS;
    }

    let args = match cli::parse(argv) {
        Ok(args) => args,
        Err(usage) => {
            eprintln!(
                "{}",
                output::usage_line(&usage, stderr_palette(&theme).as_ref())
            );
            return ExitCode::from(2);
        }
    };

    let all = match &args.file {
        Some(path) => match vars::from_file(path) {
            Ok(vars) => vars,
            Err(e) => return report(&theme, &e, ""),
        },
        None => vars::from_env(),
    };

    let found = match &args.query {
        Some(query) => {
            let found = select(query, &all);
            if found.is_empty() {
                return report(&theme, lang().no_match(), query);
            }
            found
        }
        None => all.clone(),
    };

    let clear = clipboard::clear_after(
        args.no_clear,
        std::env::var("GET_ENV_CLIP_CLEAR").ok().as_deref(),
    );
    let tty = std::io::stdout().is_terminal();
    if args.query.is_some() && found.len() == 1 {
        if !tty {
            return print_masked(&found).unwrap_or(ExitCode::SUCCESS);
        }
        let var = &found[0];
        let copied = clipboard::copy(&var.value, clear);
        let footer = match &copied {
            Ok(()) => Footer::Copied,
            Err(e) => Footer::Failed(e.clone()),
        };
        let max_width = match ratatui::crossterm::terminal::size() {
            Ok((w, _)) if w > 0 => usize::from(w),
            _ => 80,
        };
        print!(
            "{}",
            output::card(&var.name, &var.value, &footer, &palette(&theme), max_width)
        );
        return if copied.is_ok() {
            ExitCode::SUCCESS
        } else {
            ExitCode::from(1)
        };
    }

    if tty {
        // A tabela recebe tudo; a query vira o filtro inicial da busca.
        let filter = args.query.as_deref().unwrap_or("");
        if let Err(e) = tui::run(all, filter, palette(&theme), clear) {
            return report(&theme, &lang().table_error(&e), "");
        }
    } else if let Some(code) = print_masked(&found) {
        return code;
    }
    ExitCode::SUCCESS
}

fn palette(theme: &OnceCell<Theme>) -> Palette {
    Palette::of(*theme.get_or_init(theme::detect))
}

/// Paleta para o stderr: só quando ele é um terminal.
fn stderr_palette(theme: &OnceCell<Theme>) -> Option<Palette> {
    std::io::stderr().is_terminal().then(|| palette(theme))
}

/// Escreve a linha de erro no stderr e devolve rc 1.
fn report(theme: &OnceCell<Theme>, text: &str, emphasis: &str) -> ExitCode {
    let line = output::error_line(text, emphasis, stderr_palette(theme).as_ref());
    eprintln!("{line}");
    ExitCode::from(1)
}

/// Imprime as linhas mascaradas. Pipe fechado (`get-env | head`) encerra
/// em silêncio com rc 0; outro erro de escrita devolve rc 1.
fn print_masked(vars: &[Var]) -> Option<ExitCode> {
    let mut out = std::io::stdout().lock();
    for var in vars {
        match writeln!(out, "{}", masked_line(&var.name, &var.value)) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                return Some(ExitCode::SUCCESS);
            }
            Err(e) => {
                let text = lang().write_error(&e);
                eprintln!("{}", output::error_line(&text, "", None));
                return Some(ExitCode::from(1));
            }
        }
    }
    None
}

/// Variáveis cujo nome casa com `query`, na ordem do `matching::find`.
fn select(query: &str, all: &[Var]) -> Vec<Var> {
    let names: Vec<&str> = all.iter().map(|v| v.name.as_str()).collect();
    matching::find(query, &names)
        .into_iter()
        .filter_map(|name| all.iter().find(|v| v.name == name).cloned())
        .collect()
}
