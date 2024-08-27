use std::cell::Cell;

use ratatui::{buffer::Buffer, layout::Rect, widgets::WidgetRef};

#[derive(Debug)]
pub(crate) struct Text<'a> {
    inner: ratatui::text::Text<'a>,
    render_from_line: Cell<usize>,
}

impl<'a> Text<'a> {
    pub(crate) fn new() -> Self {
        Self {
            inner: ratatui::text::Text::default(),
            render_from_line: Cell::new(0),
        }
    }

    pub(crate) fn height(&self) -> usize {
        self.inner.height()
    }

    pub(crate) fn set_render_from_line(&self, line: usize) -> &Self {
        self.render_from_line.set(line);
        self
    }

    pub(crate) fn push_line(&mut self, line: ratatui::prelude::Line<'a>) {
        self.inner.push_line(line);
    }
}

impl WidgetRef for Text<'_> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let line_from = self.render_from_line.get();
        if line_from > 0 {
            ratatui::text::Text::from(
                self.inner
                    .lines
                    .clone()
                    .into_iter()
                    .skip(line_from)
                    .collect::<Vec<ratatui::text::Line>>(),
            )
            .render_ref(area, buf);
        } else {
            self.inner.render_ref(area, buf);
        }
    }
}
