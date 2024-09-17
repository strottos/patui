use crossterm::event::{KeyCode, KeyEvent};
use eyre::Result;
use ratatui::{
    layout::Rect,
    widgets::{Paragraph, Wrap},
    Frame,
};

use crate::tui::app::{Action, PaneType};

use super::{HelpItem, PopupComponent};

#[derive(Debug)]
pub(crate) struct HelpComponent {
    help_items: Vec<HelpItem>,
}

impl HelpComponent {
    pub(crate) fn new(help_items: Vec<HelpItem>) -> Self {
        Self { help_items }
    }
}

impl PopupComponent for HelpComponent {
    fn render_inner(&self, f: &mut Frame, rect: Rect) {
        let items = self
            .help_items
            .iter()
            .map(|x| x.global_help())
            .collect::<Vec<_>>()
            .join("\n");
        let paragraphs = Paragraph::new(items).wrap(Wrap { trim: false });

        f.render_widget(paragraphs, rect);
    }

    fn input(&mut self, key: &KeyEvent, _mode: &PaneType) -> Result<Vec<Action>> {
        match (key.code, key.modifiers) {
            (KeyCode::Esc, _) => Ok(std::vec![Action::PopupClose, Action::ClearKeys]),
            _ => Ok(std::vec![]),
        }
    }

    fn keys(&self, _mode: &PaneType) -> Vec<HelpItem> {
        std::vec![]
    }
}
