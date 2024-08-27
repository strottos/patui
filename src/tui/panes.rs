use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

use super::app::{Action, HelpItem, PaneType};

mod test_details;
mod test_list;

pub(crate) use test_details::TestDetailsPane;
pub(crate) use test_list::TestsPane;

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

    /// Render the component into the rect given
    fn render(&self, f: &mut Frame, rect: Rect, is_selected: bool);

    fn pane_type(&self) -> PaneType;

    fn pane_title(&self) -> String;
}

pub(crate) trait ScrollablePane: Pane {
    fn navigate(&mut self) -> Result<Vec<Action>>;

    fn scroll_down(&mut self) -> Result<Vec<Action>>;

    fn update_scroll(&mut self, _action: &Action) -> Result<Vec<Action>> {
        Ok(vec![])
    }

    fn keys_scroll(&self) -> Vec<HelpItem> {
        vec![]
    }
}
