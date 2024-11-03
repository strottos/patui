use crate::tui::app::{Action, PaneType, UpdateData};

use crossterm::event::{KeyCode, KeyModifiers};
use eyre::Result;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Margin, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use super::app::HelpItem;

#[derive(Debug)]
pub(crate) struct TopBar {
    panes_titles: Vec<String>,
    selected_idx: usize,
}

impl TopBar {
    pub(crate) fn new(panes_titles: Vec<String>) -> Self {
        Self {
            panes_titles,
            selected_idx: 0,
        }
    }

    pub(crate) fn render(&self, f: &mut Frame, rect: Rect) {
        let rect = rect.inner(Margin {
            vertical: 0,
            horizontal: 1,
        });

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(5), Constraint::Length(10)].as_ref())
            .split(rect);

        let breadcrumb = self
            .panes_titles
            .iter()
            .enumerate()
            .map(|(i, pane)| {
                let title = Span::from(pane);
                if self.selected_idx == i {
                    title.white()
                } else {
                    title
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

    pub(crate) fn input(
        &mut self,
        key: &crossterm::event::KeyEvent,
        panes_len: usize,
    ) -> Result<Vec<Action>> {
        let mut ret = vec![];

        let level: usize = match (key.code, key.modifiers) {
            (KeyCode::Char('1'), KeyModifiers::CONTROL) => 1,
            (KeyCode::Char('2'), KeyModifiers::CONTROL) => 2,
            (KeyCode::Char('3'), KeyModifiers::CONTROL) => 3,
            (KeyCode::Char('4'), KeyModifiers::CONTROL) => 4,
            (KeyCode::Char('5'), KeyModifiers::CONTROL) => 5,
            (KeyCode::Char('6'), KeyModifiers::CONTROL) => 6,
            (KeyCode::Char('7'), KeyModifiers::CONTROL) => 7,
            (KeyCode::Char('8'), KeyModifiers::CONTROL) => 8,
            (KeyCode::Char('9'), KeyModifiers::CONTROL) => 9,
            _ => return Ok(vec![]),
        };

        if level >= panes_len || level == 0 {
            return Ok(vec![]);
        }

        // Should always be at least 0 because level == 0 handled above
        self.selected_idx = level - 1;

        // TODO
        // ret.push(Action::PaneChange(level));

        Ok(ret)
    }

    pub(crate) fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        match action {
            Action::PaneChange(level) => {
                // TODO
                //self.selected_idx = *level - 1;
            }
            Action::UpdateData(UpdateData::BreadcrumbTitles(titles)) => {
                self.panes_titles = titles.clone();
            }
            _ => {}
        }

        Ok(vec![])
    }

    pub(crate) fn keys(&self, _mode: &PaneType) -> Vec<HelpItem> {
        vec![
            HelpItem::new("<C-num>", "Breadcrumb num", "Goto breadcrumb element <num>"),
            HelpItem::new("<Esc>", "Revert level", "Go up a level in the breadcrumb"),
        ]
    }
}
