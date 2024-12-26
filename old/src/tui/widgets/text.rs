use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Text as RatatuiText},
    widgets::WidgetRef,
};

#[derive(Clone, Debug, Default)]
pub(crate) struct Text<'a> {
    inner: RatatuiText<'a>,
    selectable: bool,
    is_selected: Cell<bool>,
}

impl<'a> Text<'a> {
    pub(crate) fn new(selectable: bool) -> Self {
        Self {
            inner: RatatuiText::default(),
            selectable,
            is_selected: Cell::new(false),
        }
    }

    pub(crate) fn new_with_text(inner: RatatuiText<'a>, selectable: bool) -> Self {
        Self {
            inner,
            selectable,
            is_selected: Cell::new(false),
        }
    }

    pub(crate) fn is_selectable(&self) -> bool {
        self.selectable
    }

    pub(crate) fn set_selected(&self, selected: bool) {
        self.is_selected.set(selected);
    }
}

impl WidgetRef for Text<'_> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        if self.is_selected.get() {
            self.inner
                .clone()
                .style(Style::default().fg(Color::Black).bg(Color::White))
                .render_ref(area, buf);
        } else {
            self.inner.render_ref(area, buf);
        };
    }
}

impl<'a> From<&'a str> for Text<'a> {
    fn from(text: &'a str) -> Text<'a> {
        Self {
            inner: RatatuiText::from(text),
            selectable: false,
            is_selected: Cell::new(false),
        }
    }
}

impl<'a> From<String> for Text<'a> {
    fn from(text: String) -> Text<'a> {
        Self {
            inner: RatatuiText::from(text),
            selectable: false,
            is_selected: Cell::new(false),
        }
    }
}

impl<'a> From<RatatuiText<'a>> for Text<'a> {
    fn from(text: RatatuiText<'a>) -> Text<'a> {
        Self {
            inner: text,
            selectable: false,
            is_selected: Cell::new(false),
        }
    }
}

impl<'a, 'b> From<Vec<Line<'a>>> for Text<'b>
where
    'a: 'b,
{
    fn from(lines: Vec<Line<'a>>) -> Text<'b> {
        Self {
            inner: RatatuiText::from(lines),
            selectable: false,
            is_selected: Cell::new(false),
        }
    }
}

impl<'a> Deref for Text<'a> {
    type Target = RatatuiText<'a>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> DerefMut for Text<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
