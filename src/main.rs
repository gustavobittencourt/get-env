mod cli;
mod clipboard;
mod format;
mod matching;
mod tui;
mod vars;

use std::io::{IsTerminal, Write};
use std::process::ExitCode;

use format::masked_line;
use vars::Var;

fn main() -> ExitCode {
    let args = match cli::parse(std::env::args().skip(1)) {
        Ok(args) => args,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::from(2);
        }
    };

    let all = match &args.file {
        Some(path) => match vars::from_file(path) {
            Ok(vars) => vars,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(1);
            }
        },
        None => vars::from_env(),
    };

    let found = match &args.query {
        Some(query) => {
            let found = select(query, all);
            if found.is_empty() {
                eprintln!("nenhuma variável casa com {query}");
                return ExitCode::from(1);
            }
            found
        }
        None => all,
    };

    let tty = std::io::stdout().is_terminal();
    if args.query.is_some() && found.len() == 1 {
        let var = &found[0];
        if let Some(code) = print_masked(&found) {
            return code;
        }
        if tty {
            if let Err(e) = clipboard::copy(&var.value) {
                eprintln!("erro ao copiar: {e}");
                return ExitCode::from(1);
            }
            eprintln!("copiado: valor de {}", var.name);
        }
        return ExitCode::SUCCESS;
    }

    if tty {
        if let Err(e) = tui::run(found) {
            eprintln!("erro na tabela: {e}");
            return ExitCode::from(1);
        }
    } else if let Some(code) = print_masked(&found) {
        return code;
    }
    ExitCode::SUCCESS
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
                eprintln!("erro ao escrever: {e}");
                return Some(ExitCode::from(1));
            }
        }
    }
    None
}

/// Variáveis cujo nome casa com `query`, na ordem do `matching::find`.
fn select(query: &str, all: Vec<Var>) -> Vec<Var> {
    let names: Vec<&str> = all.iter().map(|v| v.name.as_str()).collect();
    let order: Vec<String> = matching::find(query, &names)
        .into_iter()
        .map(str::to_string)
        .collect();
    let mut found: Vec<Var> = all
        .into_iter()
        .filter(|v| order.contains(&v.name))
        .collect();
    found.sort_by_key(|v| order.iter().position(|n| *n == v.name));
    found
}
