use crate::tui::app::{Action, MainMode};

use super::Component;

use color_eyre::eyre::{eyre, Result};
use crossterm::event::KeyCode;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug)]
pub struct TopBar {
    breadcrumb_name: Vec<String>,
    breadcrumb_mode: Vec<MainMode>,
    selected_idx: usize,
}

impl TopBar {
    pub fn new() -> Self {
        Self {
            breadcrumb_name: vec!["Tests".to_string()],
            breadcrumb_mode: vec![MainMode::Test],
            selected_idx: 0,
        }
    }

    pub fn push(&mut self, name: String, mode: MainMode) {
        if let Some(last_mode) = &self.breadcrumb_mode.last() {
            // If changing ids
            if mode.matched(last_mode) {
                self.breadcrumb_name.pop();
                self.breadcrumb_mode.pop();
                self.selected_idx -= 1;
            }
        }

        self.selected_idx += 1;

        self.breadcrumb_name.push(name);
        self.breadcrumb_mode.push(mode);

        assert!(self.breadcrumb_name.len() == self.breadcrumb_mode.len());
        assert!(self.selected_idx < self.breadcrumb_name.len());
    }

    pub fn pop(&mut self) {
        if self.breadcrumb_name.len() > 1 {
            self.breadcrumb_name.pop();
            self.breadcrumb_mode.pop();

            if self.selected_idx > 0 {
                self.selected_idx -= 1;
            }
        }

        assert!(self.breadcrumb_name.len() == self.breadcrumb_mode.len());
        assert!(self.selected_idx < self.breadcrumb_name.len());
    }

    pub fn get_main_mode(&self) -> Result<&MainMode> {
        if let Some(elem) = self.breadcrumb_mode.get(self.selected_idx) {
            Ok(elem)
        } else {
            Err(eyre!("No breadcrumb mode found"))
        }
    }

    pub fn render(&self, f: &mut Frame, rect: Rect) {
        let rect = rect.inner(Margin {
            vertical: 0,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(5), Constraint::Min(0)].as_ref())
            .split(rect);

        assert!(self.breadcrumb_name.len() == self.breadcrumb_mode.len());
        assert!(self.selected_idx < self.breadcrumb_name.len());

        let breadcrumb = self
            .breadcrumb_name
            .iter()
            .enumerate()
            .map(|(i, breadcrumb_name)| {
                if self.selected_idx == i {
                    format!("({}) {}", i + 1, breadcrumb_name).white()
                } else {
                    format!("({}) {}", i + 1, breadcrumb_name).into()
                }
            })
            .intersperse(" > ".into())
            .collect::<Vec<_>>();

        let left = Paragraph::new(Line::from(breadcrumb))
            .alignment(Alignment::Left)
            .block(
                Block::default()
                    .borders(Borders::BOTTOM)
                    .border_style(Style::default()),
            );
        f.render_widget(left, rect);

        let right = Paragraph::new(Line::from("   Patui ")).alignment(Alignment::Right);
        f.render_widget(right, chunks[1]);
    }
}

impl Component for TopBar {
    fn input(&mut self, key: &crossterm::event::KeyEvent, _mode: &MainMode) -> Result<Vec<Action>> {
        let level = match key.code {
            KeyCode::Char('1') => 1,
            KeyCode::Char('2') => 2,
            KeyCode::Char('3') => 3,
            KeyCode::Char('4') => 4,
            KeyCode::Char('5') => 5,
            KeyCode::Char('6') => 6,
            KeyCode::Char('7') => 7,
            KeyCode::Char('8') => 8,
            KeyCode::Char('9') => 9,
            KeyCode::Esc => self.breadcrumb_name.len() - 1,
            _ => return Ok(vec![]),
        };

        if level >= self.breadcrumb_name.len() || level == 0 {
            return Ok(vec![]);
        }

        for _ in level..self.breadcrumb_name.len() {
            self.breadcrumb_name.pop();
            self.breadcrumb_mode.pop();
        }

        self.selected_idx = if level > 0 { level - 1 } else { 0 };

        Ok(vec![])
    }
}
