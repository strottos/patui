mod bottom_bar;
mod error;
mod main;
mod tests;
mod top_bar;
mod widgets;

use color_eyre::Result;
use crossterm::event::KeyEvent;

use super::app::{Action, AppMode};

pub use bottom_bar::BottomBar;
pub use error::ErrorComponent;
pub use main::MainComponent;
pub use top_bar::TopBar;

pub trait Component: std::fmt::Debug {
    /// Take input for the component and optionally send back an action to perform
    fn input(&mut self, _key: &KeyEvent, _mode: &AppMode) -> Result<Vec<Action>> {
        Ok(vec![])
    }

    /// Update the component based on an action and optionally send back an action to perform
    fn update(&mut self, _action: &Action) -> Result<Vec<Action>> {
        Ok(vec![])
    }
}
