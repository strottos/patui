mod bottom_bar;
mod error;
mod middle;
mod tests;
mod top_bar;
mod widgets;

use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

use super::app::{Action, MainMode};

pub use bottom_bar::BottomBar;
pub use error::ErrorComponent;
pub use middle::Middle;
pub use tests::TestComponentCreate;
pub use top_bar::TopBar;

pub trait Component: std::fmt::Debug {
    /// Take input for the component and optionally send back an action to perform
    fn input(&mut self, _key: &KeyEvent, _mode: &MainMode) -> Result<Vec<Action>> {
        Ok(vec![])
    }

    /// Get the keys that the component is listening for
    fn keys(&self, _mode: &MainMode) -> Vec<(&str, &str)> {
        vec![]
    }

    /// Update the component based on an action and optionally send back actions to perform
    fn update(&mut self, _action: &Action) -> Result<Vec<Action>> {
        Ok(vec![])
    }
}

pub trait PopupComponent: Component {
    /// Render the component into the rect given
    fn render(&self, f: &mut Frame, rect: Rect);
}
