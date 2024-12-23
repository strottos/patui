use crossterm::event::KeyEvent;
use eyre::Result;
use ratatui::{layout::Rect, Frame};

use super::app::{Action, HelpItem};

mod test_details;
mod test_list;

pub(crate) use test_details::TestDetailsPane;
pub(crate) use test_list::TestListPane;

pub(crate) trait Pane: std::fmt::Debug {
    /// Take input for the component and optionally send back an action to perform
    fn input(&mut self, _key: &KeyEvent) -> Result<Vec<Action>> {
        Ok(vec![])
    }

    /// Get the keys that the component is listening for
    fn keys(&self) -> Vec<HelpItem> {
        vec![]
    }

    /// Update the component based on an action and optionally send back actions to perform
    fn update(&mut self, _action: &Action) -> Result<Vec<Action>> {
        Ok(vec![])
    }

    fn set_focus(&mut self, _focus: bool);

    /// Render the component into the rect given
    fn render(&self, f: &mut Frame, rect: Rect);

    // fn pane_type(&self) -> PaneType;

    // fn pane_title(&self) -> String;
}
