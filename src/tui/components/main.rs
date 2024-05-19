use ratatui::{
    layout::{Constraint, Direction, Layout},
    widgets::{Block, Borders},
};

use crate::tui::error::Error;

use super::{error::ErrorComponent, Component};

#[derive(Debug)]
pub struct MainComponent {
    error_component: ErrorComponent,
}

impl MainComponent {
    pub fn new() -> Self {
        let error_component = ErrorComponent::new();

        Self { error_component }
    }

    pub fn add_error(&mut self, error: Error) {
        self.error_component.add_error(error)
    }
}

impl Component for MainComponent {
    fn render(&mut self, f: &mut ratatui::prelude::Frame, rect: ratatui::prelude::Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(rect);

        let left = Block::default().title("Left").borders(Borders::ALL);
        f.render_widget(left, chunks[0]);

        let right = Block::default().title("Right").borders(Borders::ALL);
        f.render_widget(right, chunks[1]);

        if self.error_component.has_error() {
            self.error_component.render(f, rect);
        }
    }
}
