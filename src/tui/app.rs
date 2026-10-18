//! Estado da tabela e o que cada tecla ou clique faz.

use ratatui::crossterm::event::KeyCode;

use super::geometry::Cell;
use crate::format::shell_line;
use crate::vars::Var;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CopyKind {
    Name,
    Value,
    Line,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    None,
    Quit,
    Copy(CopyKind),
}

pub struct App {
    pub vars: Vec<Var>,
    pub selected: usize,
    pub status: String,
}

impl App {
    pub fn new(vars: Vec<Var>) -> Self {
        Self {
            vars,
            selected: 0,
            status: String::new(),
        }
    }

    pub fn on_key(&mut self, key: KeyCode) -> Action {
        match key {
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.vars.len().saturating_sub(1))
            }
            KeyCode::Enter => return Action::Copy(CopyKind::Line),
            KeyCode::Char('n') => return Action::Copy(CopyKind::Name),
            KeyCode::Char('v') => return Action::Copy(CopyKind::Value),
            KeyCode::Char('q') | KeyCode::Esc => return Action::Quit,
            _ => {}
        }
        Action::None
    }

    /// Todo clique numa linha a seleciona; o ícone copia a linha no clique
    /// simples, nome e valor só no duplo.
    pub fn on_click(&mut self, hit: Option<(usize, Cell)>, double: bool) -> Action {
        let Some((index, cell)) = hit else {
            return Action::None;
        };
        self.selected = index;
        match (cell, double) {
            (Cell::Icon, _) => Action::Copy(CopyKind::Line),
            (Cell::Name, true) => Action::Copy(CopyKind::Name),
            (Cell::Value, true) => Action::Copy(CopyKind::Value),
            _ => Action::None,
        }
    }

    /// O que vai para o clipboard: sempre o valor real.
    pub fn copy_text(&self, kind: CopyKind) -> String {
        let var = &self.vars[self.selected];
        match kind {
            CopyKind::Name => var.name.clone(),
            CopyKind::Value => var.value.clone(),
            CopyKind::Line => shell_line(&var.name, &var.value),
        }
    }

    pub fn copied_status(&self, kind: CopyKind) -> String {
        let what = match kind {
            CopyKind::Name => "nome",
            CopyKind::Value => "valor",
            CopyKind::Line => "linha",
        };
        format!("copiado: {what} de {}", self.vars[self.selected].name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let var = |name: &str, value: &str| Var {
            name: name.into(),
            value: value.into(),
        };
        App::new(vec![
            var("A", "1"),
            var("FAKE_TOKEN", "abcdefgh"),
            var("MSG", "olá mundo"),
        ])
    }

    #[test]
    fn down_stops_at_last_row() {
        let mut a = app();
        for _ in 0..3 {
            a.on_key(KeyCode::Down);
        }
        assert_eq!(a.selected, 2);
        a.on_key(KeyCode::Up);
        assert_eq!(a.selected, 1);
    }

    #[test]
    fn enter_copies_quoted_line() {
        let mut a = app();
        a.selected = 2;
        assert_eq!(a.on_key(KeyCode::Enter), Action::Copy(CopyKind::Line));
        assert_eq!(a.copy_text(CopyKind::Line), "MSG='olá mundo'");
    }

    #[test]
    fn keys_n_v_q_esc() {
        let mut a = app();
        assert_eq!(a.on_key(KeyCode::Char('n')), Action::Copy(CopyKind::Name));
        assert_eq!(a.on_key(KeyCode::Char('v')), Action::Copy(CopyKind::Value));
        assert_eq!(a.on_key(KeyCode::Char('q')), Action::Quit);
        assert_eq!(a.on_key(KeyCode::Esc), Action::Quit);
    }

    #[test]
    fn value_copy_is_unmasked() {
        let mut a = app();
        a.selected = 1;
        assert_eq!(a.copy_text(CopyKind::Value), "abcdefgh");
        assert_eq!(a.copy_text(CopyKind::Name), "FAKE_TOKEN");
        assert_eq!(
            a.copied_status(CopyKind::Value),
            "copiado: valor de FAKE_TOKEN"
        );
        assert_eq!(
            a.copied_status(CopyKind::Name),
            "copiado: nome de FAKE_TOKEN"
        );
        assert_eq!(
            a.copied_status(CopyKind::Line),
            "copiado: linha de FAKE_TOKEN"
        );
    }

    #[test]
    fn double_click_copies_cell() {
        let mut a = app();
        assert_eq!(
            a.on_click(Some((1, Cell::Name)), true),
            Action::Copy(CopyKind::Name)
        );
        assert_eq!(a.selected, 1);
        assert_eq!(
            a.on_click(Some((2, Cell::Value)), true),
            Action::Copy(CopyKind::Value)
        );
        assert_eq!(a.selected, 2);
    }

    #[test]
    fn single_click_selects_only() {
        let mut a = app();
        a.selected = 2;
        assert_eq!(a.on_click(Some((0, Cell::Value)), false), Action::None);
        assert_eq!(a.selected, 0);
        assert_eq!(a.on_click(None, false), Action::None);
        assert_eq!(a.selected, 0);
    }

    #[test]
    fn icon_click_copies_line() {
        let mut a = app();
        assert_eq!(
            a.on_click(Some((2, Cell::Icon)), false),
            Action::Copy(CopyKind::Line)
        );
        assert_eq!(a.selected, 2);
    }
}
