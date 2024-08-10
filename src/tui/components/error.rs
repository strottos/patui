use std::collections::VecDeque;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::Text,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use super::{widgets::Button, Component};
use crate::tui::{
    app::{Action, MainMode},
    error::Error,
};

#[derive(Debug, Default)]
pub struct ErrorComponent {
    errors: VecDeque<Error>,
}

impl ErrorComponent {
    pub fn new() -> Self {
        Self {
            errors: VecDeque::new(),
        }
    }

    pub fn add_error(&mut self, error: Error) {
        self.errors.push_back(error);
    }

    pub fn ack_error(&mut self) {
        self.errors.pop_front();
    }

    pub fn has_error(&self) -> bool {
        !self.errors.is_empty()
    }

    pub fn render(&self, f: &mut Frame, r: Rect) {
        if let Some(next_error) = self.errors.front() {
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(20),
                        Constraint::Percentage(60),
                        Constraint::Percentage(20),
                    ]
                    .as_ref(),
                )
                .split(r);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Percentage(20),
                        Constraint::Percentage(60),
                        Constraint::Percentage(20),
                    ]
                    .as_ref(),
                )
                .split(layout[1]);

            let error_widget = Paragraph::new(Text::from(next_error.display()))
                .alignment(Alignment::Center)
                .block(
                    Block::default()
                        .title(next_error.title())
                        .borders(Borders::ALL),
                )
                .wrap(Wrap { trim: false });

            f.render_widget(Clear, layout[1]);
            f.render_widget(error_widget, layout[1]);

            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Min(1), Constraint::Max(3), Constraint::Max(1)].as_ref())
                .split(layout[1]);
            let layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(75),
                        Constraint::Percentage(20),
                        Constraint::Percentage(5),
                    ]
                    .as_ref(),
                )
                .split(layout[1]);

            let mut ok_button = Button::new("OK".to_string());
            ok_button.selected(true);
            f.render_widget(ok_button.widget(), layout[1]);
        }
    }
}

impl Component for ErrorComponent {
    fn input(&mut self, key: &KeyEvent, _mode: &MainMode) -> Result<Vec<Action>> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Enter, KeyModifiers::NONE)
            | (KeyCode::Enter, KeyModifiers::CONTROL) => {
                self.ack_error();
                Ok(vec![])
            }
            _ => Ok(vec![]),
        }
    }
}
