use crate::tui::app::{Action, MainMode};

use super::Component;

use color_eyre::eyre::{eyre, Result};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug)]
pub(crate) struct TopBar {
    breadcrumb_name: Vec<String>,
    breadcrumb_mode: Vec<MainMode>,
    selected_idx: usize,
}

impl TopBar {
    pub(crate) fn new() -> Self {
        Self {
            breadcrumb_name: vec!["Tests".to_string()],
            breadcrumb_mode: vec![MainMode::Test],
            selected_idx: 0,
        }
    }

    pub(crate) fn push(&mut self, name: String, mode: MainMode) {
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

    pub(crate) fn pop(&mut self) {
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

    pub(crate) fn get_main_mode(&self) -> Result<&MainMode> {
        if let Some(elem) = self.breadcrumb_mode.get(self.selected_idx) {
            Ok(elem)
        } else {
            Err(eyre!("No breadcrumb mode found"))
        }
    }

    pub(crate) fn render(&self, f: &mut Frame, rect: Rect) {
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
        let mut ret = vec![];

        let level = match (key.code, key.modifiers) {
            (KeyCode::Char('1'), KeyModifiers::CONTROL) => 1,
            (KeyCode::Char('2'), KeyModifiers::CONTROL) => 2,
            (KeyCode::Char('3'), KeyModifiers::CONTROL) => 3,
            (KeyCode::Char('4'), KeyModifiers::CONTROL) => 4,
            (KeyCode::Char('5'), KeyModifiers::CONTROL) => 5,
            (KeyCode::Char('6'), KeyModifiers::CONTROL) => 6,
            (KeyCode::Char('7'), KeyModifiers::CONTROL) => 7,
            (KeyCode::Char('8'), KeyModifiers::CONTROL) => 8,
            (KeyCode::Char('9'), KeyModifiers::CONTROL) => 9,
            (KeyCode::Esc, KeyModifiers::NONE) => {
                ret.push(Action::ClearKeys);
                self.breadcrumb_name.len() - 1
            }
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

        Ok(ret)
    }

    fn keys(&self, _mode: &MainMode) -> Vec<crate::tui::components::HelpItem> {
        vec![crate::tui::components::HelpItem::new(
            "<C-num>",
            "Breadcrumb num",
            "Goto breadcrumb element <num>",
        )]
    }
}
