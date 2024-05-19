use super::Component;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug)]
pub struct TopBar {}

impl TopBar {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for TopBar {
    fn render(&mut self, f: &mut Frame, rect: Rect) {
        let rect = rect.inner(&Margin {
            vertical: 0,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(5), Constraint::Min(0)].as_ref())
            .split(rect);

        let left = Paragraph::new(Line::from("Foo > Bar")).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default()),
        );
        f.render_widget(left, rect); // r to make sure bottom border goes all the way

        let right = Paragraph::new(Line::from("FOO ")).alignment(Alignment::Right);
        f.render_widget(right, chunks[1]);
    }
}
