//! Tabela interativa com mouse: duplo clique copia a célula, Enter ou o
//! ícone copiam a linha, `/` filtra.

mod app;
mod click;
mod geometry;

use std::io;
use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseButton, MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::layout::{Constraint, Flex, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Cell as TableCell, HighlightSpacing, Padding, Paragraph, Row, Table,
    TableState,
};
use ratatui::{Frame, Terminal};

use crate::clipboard;
use crate::format::{display_value, fit, is_sensitive, width};
use crate::matching::highlight;
use crate::theme::Palette;
use crate::vars::Var;
use app::{Action, App, Status};
use click::ClickTracker;
use geometry::{Cell, Geometry, IconPos};

const DOUBLE_CLICK: Duration = Duration::from_millis(400);
const ICON: &str = "⧉";
const KEYS: [(&str, &str); 5] = [
    ("⏎", "linha"),
    ("n", "nome"),
    ("v", "valor"),
    ("/", "buscar"),
    ("q", "sair"),
];
const LOCK: &str = " 🔒";

/// Abre a tabela com todas as `vars` e `filter` como filtro inicial.
pub fn run(vars: Vec<Var>, filter: &str, palette: Palette) -> io::Result<()> {
    if vars.is_empty() {
        return Ok(());
    }
    let icon = match std::env::var("GET_ENV_ICON_POS").as_deref() {
        Ok("start") => IconPos::Start,
        _ => IconPos::End,
    };
    let name_width = vars
        .iter()
        .map(|v| width(&v.name))
        .max()
        .unwrap_or(0)
        .max("NOME".len());
    let name_width = u16::try_from(name_width).unwrap_or(u16::MAX);

    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(vars, filter);
    let mut state = TableState::default().with_selected(Some(0));
    let mut clicks = ClickTracker::new(DOUBLE_CLICK);

    loop {
        let size = terminal.size()?;
        let table = areas(Rect::new(0, 0, size.width, size.height)).table;
        let geometry = Geometry::new(table.width, name_width, icon);
        state.select(Some(app.selected));
        terminal.draw(|frame| render(frame, &app, &geometry, &palette, &mut state))?;

        let action = match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key_event(key),
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let hit = table
                        .contains(Position::new(mouse.column, mouse.row))
                        .then(|| {
                            geometry.hit(
                                mouse.column - table.x,
                                mouse.row - table.y,
                                state.offset(),
                                app.len(),
                            )
                        })
                        .flatten();
                    let double = hit.is_some_and(|target| clicks.register(target, Instant::now()));
                    app.on_click(hit, double)
                }
                MouseEventKind::ScrollDown => {
                    app.step(true);
                    Action::None
                }
                MouseEventKind::ScrollUp => {
                    app.step(false);
                    Action::None
                }
                _ => Action::None,
            },
            _ => Action::None,
        };

        match action {
            Action::Quit => return Ok(()),
            Action::Copy(kind) => {
                app.status = Some(match clipboard::copy(&app.copy_text(kind)) {
                    Ok(()) => Status::Ok(app.copied_status(kind)),
                    Err(e) => Status::Err(format!("erro ao copiar: {e}")),
                });
            }
            Action::None => {}
        }
    }
}

struct Areas {
    table: Rect,
    footer: Rect,
}

/// Moldura arredondada com 1 coluna de respiro dos lados.
fn frame_block<'a>() -> Block<'a> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .padding(Padding::horizontal(1))
}

/// Dentro da moldura: tabela, uma linha vazia e o rodapé.
fn areas(area: Rect) -> Areas {
    let [table, _, footer] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(frame_block().inner(area));
    Areas { table, footer }
}

fn title(app: &App) -> String {
    let count = format!(" get-env · {} variáveis ", app.len());
    if app.filter.is_empty() {
        count
    } else {
        format!("{count}· filtro: {} ", app.filter)
    }
}

fn render(
    frame: &mut Frame,
    app: &App,
    geometry: &Geometry,
    palette: &Palette,
    state: &mut TableState,
) {
    let area = frame.area();
    let Areas { table, footer } = areas(area);

    let title = title(app);
    let (head, rest) = title.split_at(" get-env ".len());
    let title = Line::from(vec![
        Span::styled(head.to_string(), palette.title),
        Span::styled(rest.to_string(), palette.secondary),
    ]);
    frame.render_widget(
        frame_block().border_style(palette.border).title(title),
        area,
    );

    let header = Row::new(geometry.order().map(|cell| match cell {
        Cell::Icon => "",
        Cell::Name => "NOME",
        Cell::Value => "VALOR",
    }))
    .style(palette.header);
    let rows = app.rows().into_iter().enumerate().map(|(i, var)| {
        let row = Row::new(geometry.order().map(|cell| match cell {
            Cell::Icon => TableCell::from(Span::styled(format!(" {ICON}"), palette.icon)),
            Cell::Name => TableCell::from(name_line(var, &app.filter, geometry, palette)),
            Cell::Value => TableCell::from(value_line(var, geometry, palette)),
        }));
        if i % 2 == 1 {
            row.style(palette.zebra)
        } else {
            row
        }
    });
    let widget = Table::new(rows, geometry.widths())
        .header(header)
        .column_spacing(1)
        .flex(Flex::Start)
        .highlight_spacing(HighlightSpacing::Never)
        .row_highlight_style(palette.selected);
    frame.render_stateful_widget(widget, table, state);

    if app.len() == 0 && table.height > 1 {
        let message = format!("❌ nenhuma variável casa com {}", app.filter);
        let line = Rect::new(table.x, table.y + 1, table.width, 1);
        frame.render_widget(Paragraph::new(Span::styled(message, palette.err)), line);
    }

    frame.render_widget(Paragraph::new(footer_line(app, palette)), footer);
}

/// Nome cortado na largura da coluna, com as letras casadas destacadas.
fn name_line(var: &Var, filter: &str, geometry: &Geometry, palette: &Palette) -> Line<'static> {
    let shown = fit(&var.name, usize::from(geometry.name_width));
    let marks = highlight(filter, &var.name);
    let cut = shown != var.name;
    let last = shown.chars().count().saturating_sub(1);
    Line::from(
        shown
            .chars()
            .enumerate()
            .map(|(i, c)| {
                let matched = marks.contains(&i) && !(cut && i == last);
                let style = if matched {
                    palette.name.patch(palette.matched)
                } else {
                    palette.name
                };
                Span::styled(c.to_string(), style)
            })
            .collect::<Vec<_>>(),
    )
}

/// Valor como no terminal; mascarado ganha cor própria e um cadeado.
fn value_line(var: &Var, geometry: &Geometry, palette: &Palette) -> Line<'static> {
    let shown = display_value(&var.name, &var.value);
    let width = usize::from(geometry.value_width);
    if !is_sensitive(&var.name) {
        return Line::from(Span::styled(fit(&shown, width), palette.value));
    }
    if width <= self::width(LOCK) {
        return Line::from(Span::styled(fit(&shown, width), palette.masked));
    }
    Line::from(vec![
        Span::styled(fit(&shown, width - self::width(LOCK)), palette.masked),
        Span::raw(LOCK),
    ])
}

/// Busca em curso, mensagem da última ação ou as teclas.
fn footer_line(app: &App, palette: &Palette) -> Line<'static> {
    if app.searching {
        return Line::from(vec![
            Span::styled(" / ", palette.key),
            Span::styled(format!("{}▌", app.filter), Style::new().bold()),
            Span::styled("   Esc limpa", palette.secondary),
        ]);
    }
    match &app.status {
        Some(Status::Ok(text)) => Line::styled(format!("✅ {text}"), palette.ok),
        Some(Status::Err(text)) => Line::styled(format!("❌ {text}"), palette.err),
        None => Line::from(
            KEYS.iter()
                .flat_map(|(key, label)| {
                    [
                        Span::styled(format!(" {key} "), palette.key),
                        Span::styled(format!(" {label}  "), palette.secondary),
                    ]
                })
                .collect::<Vec<_>>(),
        ),
    }
}

/// Tela alternativa, raw mode e mouse ligados enquanto vive; desfaz no
/// `Drop` e também antes de um panic ser impresso.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}

fn restore() {
    let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::style::{Color, Modifier};

    const W: u16 = 40;
    const H: u16 = 8;

    fn vars() -> Vec<Var> {
        vec![
            Var {
                name: "FAKE_TOKEN".into(),
                value: "abcdefgh".into(),
            },
            Var {
                name: "HOME".into(),
                value: "/tmp/fake".into(),
            },
        ]
    }

    fn table_area(w: u16, h: u16) -> Rect {
        areas(Rect::new(0, 0, w, h)).table
    }

    fn draw(icon: IconPos, app: &App, w: u16, h: u16) -> Buffer {
        let palette = Palette::of(Theme::Dark);
        let geometry = Geometry::new(table_area(w, h).width, 10, icon);
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        let mut state = TableState::default().with_selected(Some(app.selected));
        terminal
            .draw(|frame| render(frame, app, &geometry, &palette, &mut state))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn text_at(buffer: &Buffer, x: u16, y: u16, len: u16) -> String {
        (x..x + len).map(|x| buffer[(x, y)].symbol()).collect()
    }

    /// Texto de uma linha, pulando a célula que segue um caractere largo.
    fn row_text(buffer: &Buffer, y: u16) -> String {
        let mut text = String::new();
        let mut x = 0;
        while x < buffer.area.width {
            let symbol = buffer[(x, y)].symbol();
            text.push_str(symbol);
            x += u16::try_from(width(symbol).max(1)).unwrap();
        }
        text
    }

    /// Coluna, relativa à tabela, da célula `cell` na linha de dados `row`.
    fn column_of(geometry: &Geometry, row: u16, cell: Cell) -> u16 {
        (0..W)
            .find(|&x| geometry.hit(x, row + 1, 0, 2) == Some((usize::from(row), cell)))
            .unwrap()
    }

    /// Desenho e `Geometry::hit` precisam concordar sobre onde cada célula está.
    #[test]
    fn rendered_cells_match_geometry() {
        let table = table_area(W, H);
        for icon in [IconPos::Start, IconPos::End] {
            let buffer = draw(icon, &App::new(vars(), ""), W, H);
            let geometry = Geometry::new(table.width, 10, icon);
            let y = table.y + 2;
            let name_x = table.x + column_of(&geometry, 1, Cell::Name);
            let value_x = table.x + column_of(&geometry, 1, Cell::Value);
            let icon_x = table.x + column_of(&geometry, 1, Cell::Icon);
            assert_eq!(text_at(&buffer, name_x, y, 4), "HOME", "{icon:?}");
            assert_eq!(text_at(&buffer, value_x, y, 9), "/tmp/fake", "{icon:?}");
            assert_eq!(text_at(&buffer, icon_x + 1, y, 1), ICON, "{icon:?}");
            assert_eq!(text_at(&buffer, value_x, y - 1, 6), "abcde…", "{icon:?}");
        }
    }

    #[test]
    fn title_counts_visible_rows_and_shows_filter() {
        assert_eq!(
            title(&App::new(vars(), "fake")),
            " get-env · 1 variáveis · filtro: fake "
        );
        assert_eq!(title(&App::new(vars(), "")), " get-env · 2 variáveis ");
        let buffer = draw(IconPos::End, &App::new(vars(), ""), W, H);
        let top = row_text(&buffer, 0);
        assert!(top.starts_with('╭'), "{top}");
        assert!(top.contains("get-env · 2 variáveis"), "{top}");
    }

    #[test]
    fn zebra_on_odd_rows_only() {
        let table = table_area(W, H);
        let buffer = draw(IconPos::End, &App::new(vars(), ""), W, H);
        assert_eq!(
            buffer[(table.x, table.y + 2)].bg,
            Color::Rgb(0x31, 0x32, 0x44)
        );
        assert_eq!(
            buffer[(table.x, table.y + 1)].bg,
            Color::Rgb(0xcb, 0xa6, 0xf7)
        );
    }

    #[test]
    fn matched_letters_are_underlined() {
        let table = table_area(W, H);
        let buffer = draw(IconPos::End, &App::new(vars(), "tok"), W, H);
        let y = table.y + 1;
        for x in 5..8 {
            assert!(
                buffer[(table.x + x, y)]
                    .modifier
                    .contains(Modifier::UNDERLINED),
                "{x}"
            );
        }
        assert!(!buffer[(table.x, y)].modifier.contains(Modifier::UNDERLINED));
    }

    #[test]
    fn masked_value_has_lock() {
        let table = table_area(W, H);
        let mut app = App::new(vars(), "");
        app.selected = 1;
        let buffer = draw(IconPos::End, &app, W, H);
        let geometry = Geometry::new(table.width, 10, IconPos::End);
        let x = table.x + column_of(&geometry, 0, Cell::Value);
        let y = table.y + 1;
        assert_eq!(text_at(&buffer, x, y, 6), "abcde…");
        assert_eq!(buffer[(x + 7, y)].symbol(), "🔒");
        assert_eq!(buffer[(x, y)].fg, Color::Rgb(0x6c, 0x70, 0x86));
    }

    #[test]
    fn zero_matches_shows_error_row() {
        let table = table_area(W, H);
        let buffer = draw(IconPos::End, &App::new(vars(), "zzz"), W, H);
        let row = row_text(&buffer, table.y + 1);
        assert!(row.contains("❌ nenhuma variável casa com zzz"), "{row}");
    }

    #[test]
    fn footer_shows_keys_status_and_search() {
        // As 5 pastilhas ocupam ~53 colunas; em 40 o rodapé é cortado.
        const WIDE: u16 = 60;
        let footer_y = areas(Rect::new(0, 0, WIDE, H)).footer.y;
        let buffer = draw(IconPos::End, &App::new(vars(), ""), WIDE, H);
        let footer = row_text(&buffer, footer_y);
        for part in ["⏎", "linha", "/", "buscar", "q", "sair"] {
            assert!(footer.contains(part), "{part}: {footer}");
        }

        let mut app = App::new(vars(), "");
        app.status = Some(Status::Ok("copiado: valor de HOME".into()));
        let footer = row_text(&draw(IconPos::End, &app, WIDE, H), footer_y);
        assert!(footer.contains("✅ copiado: valor de HOME"), "{footer}");

        let mut app = App::new(vars(), "ho");
        app.searching = true;
        let footer = row_text(&draw(IconPos::End, &app, WIDE, H), footer_y);
        assert!(footer.contains("/ ho▌"), "{footer}");
        assert!(footer.contains("Esc limpa"), "{footer}");
    }

    #[test]
    fn narrow_terminal_draws_without_panic() {
        for w in 0..14 {
            draw(IconPos::End, &App::new(vars(), ""), w, 6);
        }
    }
}
