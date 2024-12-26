use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};

#[derive(Debug, Default, Clone)]
pub(crate) struct Button {
    text: String,
    state: State,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) enum State {
    #[default]
    Normal,
    Selected,
    Pressed,
}

impl Button {
    pub(crate) fn new(text: String) -> Self {
        Self {
            text,
            state: State::Normal,
        }
    }

    pub(crate) fn selected(&mut self, selected: bool) {
        if selected {
            self.state = State::Selected;
        } else {
            self.state = State::Normal;
        }
    }

    pub(crate) fn pressed(&mut self) {
        self.state = State::Pressed;
    }

    pub(crate) fn widget(&self) -> impl Widget {
        ButtonWidget {
            text: Paragraph::new(self.text.clone()),
            style: match self.state {
                State::Normal => Style::default().fg(Color::DarkGray),
                State::Selected => Style::default().fg(Color::White),
                State::Pressed => Style::default().fg(Color::Black).bg(Color::White),
            },
        }
    }
}

pub(crate) struct ButtonWidget<'a> {
    text: Paragraph<'a>,
    style: Style,
}

impl<'a> Widget for ButtonWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL).style(self.style);
        Clear.render(area, buf);
        self.text
            .block(block)
            .alignment(Alignment::Center)
            .render(area, buf);
    }
}
