use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eyre::Result;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::Text,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::{
    app::{Action, PaneType},
    error::Error,
    widgets::Button,
};

use super::PopupComponent;

#[derive(Debug)]
pub(crate) struct ErrorComponent {
    error: Error,
}

impl ErrorComponent {
    pub(crate) fn new(error: Error) -> Self {
        Self { error }
    }
}

impl PopupComponent for ErrorComponent {
    fn render_inner(&self, f: &mut Frame, r: Rect) {
        let error_widget = Paragraph::new(Text::from(self.error.display()))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: false });

        f.render_widget(error_widget, r);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Max(3), Constraint::Max(1)].as_ref())
            .split(r);
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

    fn input(&mut self, key: &KeyEvent, _mode: &PaneType) -> Result<Vec<Action>> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL)
            | (KeyCode::Enter, KeyModifiers::NONE)
            | (KeyCode::Enter, KeyModifiers::CONTROL) => {
                Ok(vec![Action::PopupClose, Action::ClearKeys])
            }
            _ => Ok(vec![]),
        }
    }
}
