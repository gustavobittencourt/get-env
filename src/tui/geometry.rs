//! Onde fica cada coluna da tabela, para saber em que célula foi o clique.

use ratatui::layout::Constraint;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    Icon,
    Name,
    Value,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconPos {
    Start,
    End,
}

#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub icon: IconPos,
    pub name_width: u16,
    pub value_width: u16,
}

/// Corta `text` em `width` caracteres, terminando em `…` quando não cabe.
pub fn fit(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let head: String = text.chars().take(width - 1).collect();
    format!("{head}…")
}

const ICON_WIDTH: u16 = 3;
const SPACING: u16 = 1;

impl Geometry {
    /// Ícone com 3 colunas, nome com até metade da largura, valor no resto,
    /// 1 coluna de espaço entre elas.
    pub fn new(width: u16, name_width: u16, icon: IconPos) -> Geometry {
        let name_width = name_width.min(width / 2);
        let value_width = width.saturating_sub(ICON_WIDTH + name_width + 2 * SPACING);
        Geometry {
            icon,
            name_width,
            value_width,
        }
    }

    /// Larguras na ordem em que as colunas aparecem na tela.
    pub fn widths(&self) -> [Constraint; 3] {
        let [first, second, third] = self.order();
        [first, second, third].map(|cell| Constraint::Length(self.width_of(cell)))
    }

    pub fn order(&self) -> [Cell; 3] {
        match self.icon {
            IconPos::Start => [Cell::Icon, Cell::Name, Cell::Value],
            IconPos::End => [Cell::Name, Cell::Value, Cell::Icon],
        }
    }

    /// Célula sob o clique. Linha 0 é o cabeçalho; a linha `r` da tela é o
    /// índice `offset + r - 1`.
    pub fn hit(&self, column: u16, row: u16, offset: usize, len: usize) -> Option<(usize, Cell)> {
        let index = offset + usize::from(row.checked_sub(1)?);
        if index >= len {
            return None;
        }
        let mut start = 0;
        for cell in self.order() {
            let end = start + self.width_of(cell);
            if (start..end).contains(&column) {
                return Some((index, cell));
            }
            start = end + SPACING;
        }
        None
    }

    fn width_of(&self, cell: Cell) -> u16 {
        match cell {
            Cell::Icon => ICON_WIDTH,
            Cell::Name => self.name_width,
            Cell::Value => self.value_width,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_cuts_with_ellipsis() {
        assert_eq!(fit("abcdef", 4), "abc…");
        assert_eq!(fit("abc", 4), "abc");
        assert_eq!(fit("abc", 0), "");
    }

    #[test]
    fn hit_with_icon_at_start() {
        let g = Geometry::new(80, 20, IconPos::Start);
        assert_eq!(g.hit(1, 1, 0, 5), Some((0, Cell::Icon)));
        assert_eq!(g.hit(10, 2, 0, 5), Some((1, Cell::Name)));
        assert_eq!(g.hit(40, 1, 3, 5), Some((3, Cell::Value)));
    }

    #[test]
    fn hit_ignores_header_and_rows_past_data() {
        let g = Geometry::new(80, 20, IconPos::Start);
        assert_eq!(g.hit(10, 0, 0, 5), None);
        assert_eq!(g.hit(10, 9, 0, 5), None);
    }

    #[test]
    fn hit_with_icon_at_end() {
        let g = Geometry::new(80, 20, IconPos::End);
        assert_eq!(g.hit(79, 1, 0, 5), Some((0, Cell::Icon)));
        assert_eq!(g.hit(0, 1, 0, 5), Some((0, Cell::Name)));
    }

    #[test]
    fn narrow_terminal_does_not_panic() {
        let g = Geometry::new(10, 40, IconPos::End);
        for column in 0..=20 {
            let hit = g.hit(column, 1, 0, 1);
            assert!(hit.is_none() || hit.is_some_and(|(i, _)| i == 0));
        }
    }
}
