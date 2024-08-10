use super::Component;

use ratatui::{layout::Rect, widgets::Paragraph, Frame};

#[derive(Debug)]
pub struct BottomBar {}

impl BottomBar {
    pub fn new() -> Self {
        Self {}
    }

    pub fn render(&self, f: &mut Frame, rect: Rect, mut keys: Vec<(&str, &str)>) {
        keys.push(("<C-c> <C-c>", "Quit"));
        let keys = keys
            .iter()
            .map(|(key, desc)| format!("{}: {}", key, desc))
            .collect::<Vec<_>>();
        f.render_widget(Paragraph::new(keys.join(", ")), rect);
    }
}

impl Component for BottomBar {}
