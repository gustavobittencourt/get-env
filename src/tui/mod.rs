//! Tabela interativa. Stub da Task 4: a Task 5 troca pelo laço com mouse.

use crate::format::masked_line;
use crate::vars::Var;

pub fn run(vars: Vec<Var>) -> std::io::Result<()> {
    for var in &vars {
        println!("{}", masked_line(&var.name, &var.value));
    }
    Ok(())
}
