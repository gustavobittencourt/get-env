//! Tabela interativa com mouse: duplo clique copia a célula, Enter ou o
//! ícone copiam a linha.

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
use ratatui::layout::{Constraint, Flex, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::widgets::{HighlightSpacing, Paragraph, Row, Table, TableState};
use ratatui::{Frame, Terminal};

use crate::clipboard;
use crate::format::{display_value, fit, width};
use crate::vars::Var;
use app::{Action, App, Status};
use click::ClickTracker;
use geometry::{Cell, Geometry, IconPos};

const DOUBLE_CLICK: Duration = Duration::from_millis(400);
const ICON: &str = "⧉";
const HELP: &str = "Enter: linha · n: nome · v: valor · duplo clique: célula · q: sair";

pub fn run(vars: Vec<Var>) -> io::Result<()> {
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
    let mut app = App::new(vars, "");
    let mut state = TableState::default().with_selected(Some(0));
    let mut clicks = ClickTracker::new(DOUBLE_CLICK);

    loop {
        let size = terminal.size()?;
        let geometry = Geometry::new(size.width, name_width, icon);
        let table_height = size.height.saturating_sub(1);
        terminal.draw(|frame| render(frame, &app, &geometry, &mut state))?;

        let action = match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key_event(key),
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::Down(MouseButton::Left) => {
                    let hit = (mouse.row < table_height)
                        .then(|| geometry.hit(mouse.column, mouse.row, state.offset(), app.len()))
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
        state.select(Some(app.selected));

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

fn render(frame: &mut Frame, app: &App, geometry: &Geometry, state: &mut TableState) {
    let [table_area, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

    let cell_text = |cell: Cell, var: &Var| match cell {
        Cell::Icon => format!(" {ICON}"),
        Cell::Name => fit(&var.name, usize::from(geometry.name_width)),
        Cell::Value => fit(
            &display_value(&var.name, &var.value),
            usize::from(geometry.value_width),
        ),
    };
    let header = Row::new(geometry.order().map(|cell| match cell {
        Cell::Icon => "",
        Cell::Name => "NOME",
        Cell::Value => "VALOR",
    }))
    .bold();
    let rows = app
        .rows()
        .into_iter()
        .map(|var| Row::new(geometry.order().map(|cell| cell_text(cell, var))));

    let table = Table::new(rows, geometry.widths())
        .header(header)
        .column_spacing(1)
        .flex(Flex::Start)
        .highlight_spacing(HighlightSpacing::Never)
        .row_highlight_style(Style::new().reversed());
    frame.render_stateful_widget(table, table_area, state);

    let status = match &app.status {
        None => HELP,
        Some(Status::Ok(text) | Status::Err(text)) => text,
    };
    frame.render_widget(Paragraph::new(status).dim(), status_area);
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
    use ratatui::backend::TestBackend;

    fn draw(icon: IconPos) -> ratatui::buffer::Buffer {
        let app = App::new(
            vec![
                Var {
                    name: "FAKE_TOKEN".into(),
                    value: "abcdefgh".into(),
                },
                Var {
                    name: "HOME".into(),
                    value: "/tmp/fake".into(),
                },
            ],
            "",
        );
        let geometry = Geometry::new(40, 10, icon);
        let mut terminal = Terminal::new(TestBackend::new(40, 5)).unwrap();
        let mut state = TableState::default().with_selected(Some(0));
        terminal
            .draw(|frame| render(frame, &app, &geometry, &mut state))
            .unwrap();
        terminal.backend().buffer().clone()
    }

    fn text_at(buffer: &ratatui::buffer::Buffer, x: u16, y: u16, len: u16) -> String {
        (x..x + len).map(|x| buffer[(x, y)].symbol()).collect()
    }

    /// Desenho e `Geometry::hit` precisam concordar sobre onde cada célula está.
    #[test]
    fn rendered_cells_match_geometry() {
        for icon in [IconPos::Start, IconPos::End] {
            let buffer = draw(icon);
            let geometry = Geometry::new(40, 10, icon);
            let name_x = (0..40)
                .find(|&x| geometry.hit(x, 2, 0, 2) == Some((1, Cell::Name)))
                .unwrap();
            let value_x = (0..40)
                .find(|&x| geometry.hit(x, 2, 0, 2) == Some((1, Cell::Value)))
                .unwrap();
            let icon_x = (0..40)
                .find(|&x| geometry.hit(x, 2, 0, 2) == Some((1, Cell::Icon)))
                .unwrap();
            assert_eq!(text_at(&buffer, name_x, 2, 4), "HOME", "{icon:?}");
            assert_eq!(text_at(&buffer, value_x, 2, 9), "/tmp/fake", "{icon:?}");
            assert_eq!(text_at(&buffer, icon_x + 1, 2, 1), ICON, "{icon:?}");
            assert_eq!(text_at(&buffer, value_x, 1, 6), "abcde…", "{icon:?}");
        }
    }
}
