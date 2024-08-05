use super::Component;

use ratatui::{layout::Rect, widgets::Paragraph, Frame};

#[derive(Debug)]
pub struct BottomBar {}

impl BottomBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self, f: &mut Frame, rect: Rect) {
        f.render_widget(Paragraph::new("Bottom Bar"), rect);
    }
}

impl Component for BottomBar {}
