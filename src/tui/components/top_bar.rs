use super::Component;

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Padding, Paragraph},
    Frame,
};

#[derive(Debug)]
pub struct TopBar {
    breadcrumb: Vec<String>,
}

impl TopBar {
    pub fn new() -> Self {
        Self {
            breadcrumb: vec!["tests".to_string()],
        }
    }

    pub fn push(&mut self, path: String) {
        self.breadcrumb.push(path);
    }

    pub fn pop(&mut self) {
        self.breadcrumb.pop();
    }

    pub fn new_root(&mut self, path: String) {
        self.breadcrumb = vec![path];
    }
}

impl Component for TopBar {
    fn render(&self, f: &mut Frame, rect: Rect) {
        let rect = rect.inner(Margin {
            vertical: 0,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(5), Constraint::Min(0)].as_ref())
            .split(rect);

        let left = Paragraph::new(Line::from(" Foo > Bar")).block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default()),
        );
        f.render_widget(left, rect); // r to make sure bottom border goes all the way

        let mut breadcrumb = self.breadcrumb[0].clone();
        self.breadcrumb
            .iter()
            .enumerate()
            .skip(1)
            .for_each(|(i, path)| {
                if i != 0 {
                    breadcrumb += &format!(" > {}", path);
                }
            });
        let right = Paragraph::new(Line::from(breadcrumb))
            .alignment(Alignment::Right)
            .block(Block::new().padding(Padding::horizontal(1)));
        f.render_widget(right, chunks[1]);
    }
}
