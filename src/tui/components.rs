mod bottom_bar;
mod error;
mod main;
mod top_bar;
mod widgets;

use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

use super::app::Action;

pub use bottom_bar::BottomBar;
pub use main::MainComponent;
pub use top_bar::TopBar;

pub trait Component: std::fmt::Debug {
    /// Render a component, must not fail in case we need to display errors
    fn render(&mut self, f: &mut Frame, rect: Rect);

    /// Take input for the component and optionally send back an action to perform
    fn input(&mut self, _key: KeyEvent) -> Result<Option<Action>> {
        Ok(None)
    }

    /// Update the component based on an action and optionally send back an action to perform
    fn update(&mut self, _action: &Action) -> Result<Option<Action>> {
        Ok(None)
    }
}
