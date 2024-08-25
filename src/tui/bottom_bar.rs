use color_eyre::eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Rect,
    widgets::{Paragraph, Wrap},
    Frame,
};

use super::app::{Action, HelpItem, PaneType, PopupMode};

#[derive(Debug)]
pub(crate) struct BottomBar {}

impl BottomBar {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn render(&self, f: &mut Frame, rect: Rect, mut keys: Vec<HelpItem>) {
        keys.push(HelpItem::new("<C-c> <C-c>", "Quit", "Quit"));
        let keys = keys
            .iter()
            .map(|item| item.bottom_bar_help())
            .collect::<Vec<_>>();
        f.render_widget(
            Paragraph::new(keys.join(", ")).wrap(Wrap { trim: true }),
            rect,
        );
    }

    pub(crate) fn input(&mut self, key: &KeyEvent, _mode: &PaneType) -> Result<Vec<Action>> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('?'), KeyModifiers::CONTROL)
            | (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                Ok(std::vec![Action::PopupCreate(PopupMode::Help)])
            }
            _ => Ok(std::vec![]),
        }
    }

    pub(crate) fn keys(&self, _mode: &PaneType) -> Vec<HelpItem> {
        std::vec![HelpItem::new("C-? | C-h", "Help Popup", "Help Popup")]
    }
}
