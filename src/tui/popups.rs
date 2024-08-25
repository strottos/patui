mod error;
mod help;
mod test_edit;

use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Block,
    Frame,
};

use super::app::{Action, HelpItem, PaneType};

pub(crate) use error::ErrorComponent;
pub(crate) use help::HelpComponent;
pub(crate) use test_edit::TestEditComponent;

pub(crate) trait PopupComponent: std::fmt::Debug {
    /// Render the component into the rect given
    fn render_inner(&self, _f: &mut Frame, _rect: Rect);

    /// Render the component into the rect given
    fn render(&self, f: &mut Frame, rect: Rect, title: &str) {
        let block = Block::bordered().title(title);

        f.render_widget(block, rect);

        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(2),
                    Constraint::Min(1),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(rect)[1];

        let inner = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Length(2),
                    Constraint::Min(1),
                    Constraint::Length(2),
                ]
                .as_ref(),
            )
            .split(inner)[1];

        self.render_inner(f, inner);
    }

    /// Take input for the component and optionally send back an action to perform
    fn input(&mut self, _key: &KeyEvent, _mode: &PaneType) -> Result<Vec<Action>> {
        Ok(vec![])
    }

    /// Get the keys that the component is listening for
    fn keys(&self, _mode: &PaneType) -> Vec<HelpItem> {
        vec![]
    }

    // TODO: Needed?
    // fn update(&mut self, _action: &Action) -> Result<Vec<Action>> {
    //     Ok(vec![])
    // }
}
