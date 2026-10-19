//! Onde fica cada coluna da tabela, para saber em que célula foi o clique.

use ratatui::layout::Constraint;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cell {
    Icon,
    Name,
    Value,
}

#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    pub name_width: u16,
    pub value_width: u16,
}

const ICON_WIDTH: u16 = 3;
const SPACING: u16 = 1;

impl Geometry {
    /// Ícone com 3 colunas, nome com até metade da largura, valor no resto,
    /// 1 coluna de espaço entre elas.
    pub fn new(width: u16, name_width: u16) -> Geometry {
        let name_width = name_width.min(width / 2);
        let value_width = width.saturating_sub(ICON_WIDTH + name_width + 2 * SPACING);
        Geometry {
            name_width,
            value_width,
        }
    }

    /// Larguras na ordem em que as colunas aparecem na tela.
    pub fn widths(&self) -> [Constraint; 3] {
        let [first, second, third] = self.order();
        [first, second, third].map(|cell| Constraint::Length(self.width_of(cell)))
    }

    /// Ícone de copiar antes do nome (escolha do user no UAT).
    pub fn order(&self) -> [Cell; 3] {
        [Cell::Icon, Cell::Name, Cell::Value]
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
    fn hit_with_icon_at_start() {
        let g = Geometry::new(80, 20);
        assert_eq!(g.hit(1, 1, 0, 5), Some((0, Cell::Icon)));
        assert_eq!(g.hit(10, 2, 0, 5), Some((1, Cell::Name)));
        assert_eq!(g.hit(40, 1, 3, 5), Some((3, Cell::Value)));
    }

    #[test]
    fn hit_ignores_header_and_rows_past_data() {
        let g = Geometry::new(80, 20);
        assert_eq!(g.hit(10, 0, 0, 5), None);
        assert_eq!(g.hit(10, 9, 0, 5), None);
    }

    #[test]
    fn narrow_terminal_does_not_panic() {
        let g = Geometry::new(10, 40);
        for column in 0..=20 {
            let hit = g.hit(column, 1, 0, 1);
            assert!(hit.is_none() || hit.is_some_and(|(i, _)| i == 0));
        }
    }
}
