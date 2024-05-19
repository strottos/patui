use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

#[derive(Debug, Default, Clone)]
pub struct Button<'a> {
    text: &'a str,
    state: State,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum State {
    #[default]
    Normal,
    Selected,
    Pressed,
}

impl<'a> Button<'a> {
    pub fn text(mut self, text: &'a str) -> Self {
        self.text = text;
        self
    }

    pub fn selected(&mut self, selected: bool) {
        if selected {
            self.state = State::Selected;
        } else {
            self.state = State::Normal;
        }
    }

    pub fn pressed(&mut self) {
        self.state = State::Pressed;
    }

    pub(crate) fn widget(&'a self) -> impl Widget + 'a {
        ButtonWidget {
            text: Paragraph::new(self.text),
            style: match self.state {
                State::Normal => Style::default().fg(Color::DarkGray),
                State::Selected => Style::default().fg(Color::White),
                State::Pressed => Style::default().fg(Color::Black).bg(Color::White),
            },
        }
    }
}

pub struct ButtonWidget<'a> {
    text: Paragraph<'a>,
    style: Style,
}

impl<'a> Widget for ButtonWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL).style(self.style);
        self.text
            .block(block)
            .alignment(Alignment::Center)
            .render(area, buf);
    }
}
