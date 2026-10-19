//! Estado da tabela e o que cada tecla ou clique faz.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::geometry::Cell;
use crate::format::shell_line;
use crate::i18n::lang;
use crate::matching;
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

/// Mensagem no rodapé depois de uma ação, até a próxima tecla.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Status {
    Ok(String),
    Err(String),
}

pub struct App {
    all: Vec<Var>,
    /// Índices em `all` das linhas visíveis, na ordem do filtro.
    visible: Vec<usize>,
    pub filter: String,
    pub searching: bool,
    pub selected: usize,
    pub status: Option<Status>,
}

impl App {
    pub fn new(vars: Vec<Var>, filter: &str) -> Self {
        let mut app = Self {
            all: vars,
            visible: Vec::new(),
            filter: filter.to_string(),
            searching: false,
            selected: 0,
            status: None,
        };
        app.refilter();
        app
    }

    pub fn rows(&self) -> Vec<&Var> {
        self.visible.iter().map(|&i| &self.all[i]).collect()
    }

    pub fn len(&self) -> usize {
        self.visible.len()
    }

    /// Ctrl-C sai sempre, mesmo no modo busca; outras teclas com Ctrl não
    /// fazem nada (Ctrl-V não pode copiar o valor como `v`).
    pub fn on_key_event(&mut self, key: KeyEvent) -> Action {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') => Action::Quit,
                _ => Action::None,
            };
        }
        self.on_key(key.code)
    }

    pub fn on_key(&mut self, key: KeyCode) -> Action {
        self.status = None;
        if self.searching {
            self.on_search_key(key);
            return Action::None;
        }
        match key {
            KeyCode::Up => self.step(false),
            KeyCode::Down => self.step(true),
            KeyCode::Enter => return self.copy(CopyKind::Line),
            KeyCode::Char('n') => return self.copy(CopyKind::Name),
            KeyCode::Char('v') => return self.copy(CopyKind::Value),
            KeyCode::Char('/') => self.searching = true,
            KeyCode::Esc if !self.filter.is_empty() => {
                self.filter.clear();
                self.refilter();
            }
            KeyCode::Char('q') | KeyCode::Esc => return Action::Quit,
            _ => {}
        }
        Action::None
    }

    /// No modo busca as letras vão para o filtro.
    fn on_search_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.refilter();
            }
            KeyCode::Backspace => {
                if self.filter.pop().is_some() {
                    self.refilter();
                }
            }
            KeyCode::Enter | KeyCode::Down => self.searching = false,
            KeyCode::Esc => {
                self.filter.clear();
                self.refilter();
                self.searching = false;
            }
            _ => {}
        }
    }

    /// Move a seleção uma linha, sem sair da tabela.
    pub fn step(&mut self, down: bool) {
        self.selected = if down {
            (self.selected + 1).min(self.len().saturating_sub(1))
        } else {
            self.selected.saturating_sub(1)
        };
    }

    /// Todo clique numa linha a seleciona; o ícone copia a linha no clique
    /// simples, nome e valor só no duplo.
    pub fn on_click(&mut self, hit: Option<(usize, Cell)>, double: bool) -> Action {
        let Some((index, cell)) = hit.filter(|(index, _)| *index < self.len()) else {
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
        let var = self.current();
        match kind {
            CopyKind::Name => var.name.clone(),
            CopyKind::Value => var.value.clone(),
            CopyKind::Line => shell_line(&var.name, &var.value),
        }
    }

    pub fn copied_status(&self, kind: CopyKind) -> String {
        let what = match kind {
            CopyKind::Name => lang().word_name(),
            CopyKind::Value => lang().word_value(),
            CopyKind::Line => lang().word_line(),
        };
        lang().copied(what, &self.current().name)
    }

    fn current(&self) -> &Var {
        &self.all[self.visible[self.selected]]
    }

    /// Sem linhas visíveis, copiar não faz nada.
    fn copy(&self, kind: CopyKind) -> Action {
        if self.visible.is_empty() {
            Action::None
        } else {
            Action::Copy(kind)
        }
    }

    /// Recalcula as linhas visíveis com a regra do `matching::find` e volta
    /// a seleção para a primeira.
    fn refilter(&mut self) {
        self.selected = 0;
        if self.filter.is_empty() {
            self.visible = (0..self.all.len()).collect();
            return;
        }
        let names: Vec<&str> = self.all.iter().map(|v| v.name.as_str()).collect();
        self.visible = matching::find(&self.filter, &names)
            .into_iter()
            .filter_map(|name| self.all.iter().position(|v| v.name == name))
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::{KeyEvent, KeyModifiers};

    fn vars() -> Vec<Var> {
        let var = |name: &str, value: &str| Var {
            name: name.into(),
            value: value.into(),
        };
        vec![
            var("A", "1"),
            var("FAKE_TOKEN", "abcdefgh"),
            var("MSG", "olá mundo"),
        ]
    }

    fn app() -> App {
        App::new(vars(), "")
    }

    fn names(a: &App) -> Vec<&str> {
        a.rows().iter().map(|v| v.name.as_str()).collect()
    }

    #[test]
    fn initial_filter_limits_rows() {
        let a = App::new(vars(), "fake");
        assert_eq!(names(&a), ["FAKE_TOKEN"]);
    }

    #[test]
    fn slash_enters_search_and_letters_filter() {
        let mut a = app();
        assert_eq!(a.on_key(KeyCode::Char('/')), Action::None);
        assert!(a.searching);
        for c in "msg".chars() {
            assert_eq!(a.on_key(KeyCode::Char(c)), Action::None);
        }
        assert_eq!(a.filter, "msg");
        assert_eq!(names(&a), ["MSG"]);
    }

    #[test]
    fn n_v_q_type_while_searching() {
        let mut a = app();
        a.on_key(KeyCode::Char('/'));
        for c in ['n', 'v', 'q'] {
            assert_eq!(a.on_key(KeyCode::Char(c)), Action::None);
        }
        assert_eq!(a.filter, "nvq");
    }

    #[test]
    fn enter_and_down_leave_search_keeping_filter() {
        for key in [KeyCode::Enter, KeyCode::Down] {
            let mut a = app();
            a.on_key(KeyCode::Char('/'));
            a.on_key(KeyCode::Char('m'));
            assert_eq!(a.on_key(key), Action::None);
            assert!(!a.searching);
            assert_eq!(a.filter, "m");
        }
    }

    #[test]
    fn esc_in_search_clears_and_leaves() {
        let mut a = app();
        a.on_key(KeyCode::Char('/'));
        a.on_key(KeyCode::Char('m'));
        assert_eq!(a.on_key(KeyCode::Esc), Action::None);
        assert!(!a.searching);
        assert_eq!(a.len(), 3);
    }

    #[test]
    fn esc_outside_search_clears_filter_then_quits() {
        let mut a = App::new(vars(), "msg");
        assert_eq!(a.on_key(KeyCode::Esc), Action::None);
        assert_eq!(a.filter, "");
        assert_eq!(a.on_key(KeyCode::Esc), Action::Quit);
    }

    #[test]
    fn filter_change_resets_selection() {
        let mut a = app();
        a.selected = 2;
        a.on_key(KeyCode::Char('/'));
        a.on_key(KeyCode::Char('a'));
        assert_eq!(a.selected, 0);
    }

    #[test]
    fn empty_result_makes_copy_a_noop_and_backspace_recovers() {
        let mut a = app();
        a.on_key(KeyCode::Char('/'));
        for c in "zzz".chars() {
            a.on_key(KeyCode::Char(c));
        }
        assert_eq!(a.len(), 0);
        a.on_key(KeyCode::Enter);
        for k in [
            KeyCode::Enter,
            KeyCode::Char('n'),
            KeyCode::Char('v'),
            KeyCode::Down,
            KeyCode::Up,
        ] {
            assert_eq!(a.on_key(k), Action::None);
        }
        assert_eq!(a.on_click(Some((0, Cell::Icon)), false), Action::None);
        a.on_key(KeyCode::Char('/'));
        for _ in 0..3 {
            a.on_key(KeyCode::Backspace);
        }
        assert_eq!(a.len(), 3);
        assert_eq!(a.selected, 0);
    }

    #[test]
    fn ctrl_c_quits_even_while_searching() {
        let mut a = app();
        a.on_key(KeyCode::Char('/'));
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(a.on_key_event(ctrl_c), Action::Quit);
        assert_eq!(a.filter, "");
    }

    #[test]
    fn other_ctrl_letters_are_ignored() {
        let ctrl = |c| KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL);
        let mut a = app();
        for c in ['v', 'n', 'q'] {
            assert_eq!(a.on_key_event(ctrl(c)), Action::None, "{c}");
        }
        a.on_key(KeyCode::Char('/'));
        a.on_key(KeyCode::Char('m'));
        assert_eq!(a.on_key_event(ctrl('w')), Action::None);
        assert_eq!(a.filter, "m");
    }

    #[test]
    fn copy_uses_visible_row() {
        let mut a = App::new(vars(), "msg");
        assert_eq!(a.on_key(KeyCode::Enter), Action::Copy(CopyKind::Line));
        assert_eq!(a.copy_text(CopyKind::Line), "MSG='olá mundo'");
    }

    #[test]
    fn any_key_clears_status() {
        let mut a = app();
        a.status = Some(Status::Ok("copiado: nome de A".into()));
        a.on_key(KeyCode::Down);
        assert!(a.status.is_none());
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
