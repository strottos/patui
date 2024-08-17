mod bottom_bar;
mod error;
mod middle;
mod misc;
mod tests;
mod top_bar;
mod widgets;

use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Block,
    Frame,
};

use super::app::{Action, MainMode};

pub use bottom_bar::BottomBar;
pub use error::ErrorComponent;
pub use middle::Middle;
pub use misc::HelpComponent;
pub use tests::TestComponentEdit;
pub use top_bar::TopBar;

#[derive(Debug)]
pub struct HelpItem {
    pub keys: &'static str,
    pub minidesc: &'static str,
    pub desc: &'static str,
}

impl HelpItem {
    pub fn new(keys: &'static str, minidesc: &'static str, desc: &'static str) -> Self {
        Self {
            keys,
            minidesc,
            desc,
        }
    }

    pub fn bottom_bar_help(&self) -> String {
        format!("{}: {}", self.keys, self.minidesc)
    }

    pub fn global_help(&self) -> String {
        format!("{}: {}", self.keys, self.desc)
    }
}

pub trait Component: std::fmt::Debug {
    /// Take input for the component and optionally send back an action to perform
    fn input(&mut self, _key: &KeyEvent, _mode: &MainMode) -> Result<Vec<Action>> {
        Ok(vec![])
    }

    /// Get the keys that the component is listening for
    fn keys(&self, _mode: &MainMode) -> Vec<HelpItem> {
        vec![]
    }

    /// Update the component based on an action and optionally send back actions to perform
    fn update(&mut self, _action: &Action) -> Result<Vec<Action>> {
        Ok(vec![])
    }
}

pub trait PopupComponent: Component {
    /// Render the component into the rect given
    fn render_inner(&self, _f: &mut Frame, _rect: Rect) {}

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
}
