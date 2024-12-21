use std::{cell::Cell, cmp};

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    text::{Line, Text as RatatuiText},
    widgets::{
        Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, WidgetRef,
    },
};

#[derive(Clone, Debug)]
pub(crate) struct Text {
    text: String,
    selectable: bool,
}

impl Text {
    pub(crate) fn new(text: String, selectable: bool) -> Self {
        Self { text, selectable }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TextDisplay {
    text: Vec<Text>,

    block_title: Option<String>,

    is_focussed: bool,
    is_selectable: bool,
    first_row: usize,
    selected_idx: Option<usize>,
    height: usize,
    num_display_lines: Cell<usize>,
}

impl TextDisplay {
    pub(crate) fn new_with_text(
        text: Vec<Text>,
        block_title: Option<String>,
        is_selectable: bool,
    ) -> Self {
        let height = text.iter().map(|t| t.text.split("\n").count()).sum();

        Self {
            text,

            block_title,

            is_focussed: false,
            is_selectable,
            first_row: 0,
            selected_idx: None,
            height,
            num_display_lines: Cell::new(24),
        }
    }

    pub(crate) fn is_selected(&self) -> bool {
        if self.is_selectable && self.selected_idx.is_some() {
            true
        } else {
            false
        }
    }

    pub(crate) fn set_unselected(&mut self) {
        self.selected_idx = None;
    }

    pub(crate) fn num_elements(&self) -> usize {
        self.text.iter().map(|t| t.text.split("\n").count()).sum()
    }

    pub(crate) fn num_display_lines(&self) -> usize {
        self.num_display_lines.get()
    }

    pub(crate) fn navigate(&mut self, mut count: isize) {
        if !self.is_selectable || count == 0 {
            return;
        }

        let forward = count > 0;

        // If nothing already selected selecte first selectable element.
        if self.selected_idx.is_none() {
            if count < 0 {
                return;
            }
            for i in 0..self.text.len() {
                if self.text[i].selectable {
                    self.selected_idx = Some(i);
                    count -= 1;
                    break;
                }
            }
        }

        // If we've not selected already just select the first element as we don't support
        // wrapping.
        let Some(old_selected_idx) = self.selected_idx else {
            return;
        };

        let mut count_abs = count.abs() as usize;

        let mut selected_idx = old_selected_idx;
        let mut new_selected_idx = selected_idx;

        while count_abs > 0
            && ((!forward && new_selected_idx > 0)
                || (forward && new_selected_idx + 1 < self.text.len()))
        {
            if forward {
                new_selected_idx += 1;
            } else {
                new_selected_idx -= 1;
            }
            if self.text[new_selected_idx].selectable {
                count_abs -= 1;
                selected_idx = new_selected_idx;
            }
        }

        if count_abs > 0 && !forward {
            self.selected_idx = None;
            self.first_row = 0;
            return;
        }

        self.selected_idx = Some(selected_idx);

        let num_display_lines = self.num_display_lines.get();
        let Some((selected_from, selected_to)) = self.get_selected_idx_range() else {
            return;
        };

        if forward {
            if selected_to >= self.first_row + num_display_lines {
                self.first_row = cmp::min(
                    selected_to - num_display_lines + 1,
                    self.height - num_display_lines,
                );
            }
        } else {
            if selected_from < self.first_row {
                self.first_row = selected_from;
            }
        }
    }

    pub(crate) fn set_focus(&mut self, is_focussed: bool) {
        self.is_focussed = is_focussed;
    }

    fn get_selected_idx_range(&self) -> Option<(usize, usize)> {
        let Some(selected_idx) = self.selected_idx else {
            return None;
        };

        let mut start_line = 0;

        for (i, text) in self.text.iter().take(selected_idx + 1).enumerate() {
            let text_size = text.text.split("\n").count();
            if selected_idx == i {
                return Some((start_line, start_line + text_size - 1));
            }
            start_line += text_size;
        }

        None
    }

    fn render_text(&self, area: Rect, buf: &mut Buffer) {
        let style = if !self.is_focussed || self.is_selected() {
            Style::default().fg(Color::DarkGray).bg(Color::Black)
        } else {
            Style::default().fg(Color::White).bg(Color::Black)
        };

        let elements_display_height = if self.block_title.is_some() {
            // -4 for block
            area.height as usize - 4
        } else {
            area.height as usize
        };
        self.num_display_lines.set(elements_display_height);

        let mut text = RatatuiText::default();

        let mut line_number = 0;

        for (idx, text_chunk) in self.text.iter().enumerate() {
            for line in text_chunk.text.lines() {
                if line_number < self.first_row {
                    line_number += 1;
                    continue;
                }
                if self.is_selected() && self.selected_idx == Some(idx) {
                    text.push_line(Line::from(line).style(style.fg(Color::White)));
                } else {
                    text.push_line(Line::from(line).style(style));
                }
                line_number += 1;
            }
        }

        let paragraph = Paragraph::new(text);

        let paragraph = if let Some(block_title) = self.block_title.as_ref() {
            paragraph.block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::symmetric(2, 1))
                    .title_alignment(Alignment::Center)
                    .title(block_title.to_string())
                    .style(style),
            )
        } else {
            paragraph
        };

        paragraph.render_ref(area, buf);
    }

    fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let num_elements = self.num_elements();
        let display_height = self.num_display_lines();

        let scrollbar_height = if num_elements <= display_height {
            0
        } else {
            num_elements + 1 - display_height
        };

        let mut scrollbar_state = ScrollbarState::new(scrollbar_height).position(self.first_row);

        scrollbar.render(area, buf, &mut scrollbar_state);
    }
}

impl WidgetRef for TextDisplay {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.render_text(area, buf);
        if self.block_title.is_some() {
            let scrollbar_area = Rect {
                x: area.x + area.width - 1,
                y: area.y + 1,
                width: 1,
                height: area.height - 2,
            };
            self.render_scrollbar(scrollbar_area, buf);
        }
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;
    use ratatui::{buffer::Buffer, layout::Rect, widgets::WidgetRef};
    use tracing_test::traced_test;

    use super::{Text, TextDisplay};

    #[traced_test]
    #[test]
    fn text_display_new() {
        let text = vec![
            Text::new(
                "Hello, World!\nHere's some selectable text".to_string(),
                true,
            ),
            Text::new(
                "Hello, World!\nHere's some non-selectable text".to_string(),
                false,
            ),
        ];
        let block_title = Some("Block Title".to_string());
        let text_display = TextDisplay::new_with_text(text, block_title, true);

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);

        text_display.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        assert_that!(text_display.text.len()).is_equal_to(2);
        assert_that!(text_display.is_selectable).is_true();
        assert_that!(text_display.first_row).is_equal_to(0);
        assert_that!(text_display.selected_idx).is_equal_to(None);
        assert_that!(text_display.is_selected()).is_false();
        assert_that!(text_display.height).is_equal_to(4);
        assert_that!(text_display.num_display_lines.get()).is_equal_to(6);
    }

    fn get_big_text_display() -> TextDisplay {
        let text = vec![
            Text::new("Hello, World!\nHere's some selectable text\nWith lots of other text that's really quite big and certainly bigger thant the 50 characters\nwe're going to be displaying...".to_string(), true),
            Text::new("Hello, World!\nHere's some non-selectable text\nwithout much 1".to_string(), false),
            Text::new("Hello, World!\nHere's some more selectable text\nthat is separated by other\nnon-selectable text 1".to_string(), true),
            Text::new("Hello, World!\nHere's some non-selectable text\nwithout much 2".to_string(), false),
            Text::new("Hello, World!\nHere's some more selectable text\nthat is separated by other\nnon-selectable text 2".to_string(), true),
        ];
        let block_title = Some("Block Title".to_string());
        TextDisplay::new_with_text(text, block_title, true)
    }

    #[traced_test]
    #[test]
    fn big_text_display_new() {
        let text_display = get_big_text_display();

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);

        text_display.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        assert_that!(text_display.text.len()).is_equal_to(5);
        assert_that!(text_display.is_selectable).is_true();
        assert_that!(text_display.first_row).is_equal_to(0);
        assert_that!(text_display.selected_idx).is_equal_to(None);
        assert_that!(text_display.is_selected()).is_false();
        assert_that!(text_display.height).is_equal_to(18);
        assert_that!(text_display.num_display_lines.get()).is_equal_to(6);
    }

    #[traced_test]
    #[test]
    fn select_text() {
        let mut text_display = get_big_text_display();

        text_display.navigate(1);

        assert_that!(text_display.selected_idx).is_equal_to(Some(0));
        assert_that!(text_display.is_selected()).is_true();
        assert_that!(text_display.first_row).is_equal_to(0);

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);
        text_display.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        text_display.navigate(1);

        assert_that!(text_display.selected_idx).is_equal_to(Some(2));
        assert_that!(text_display.is_selected()).is_true();
        assert_that!(text_display.first_row).is_equal_to(5);

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);
        text_display.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        text_display.navigate(1);

        assert_that!(text_display.selected_idx).is_equal_to(Some(4));
        assert_that!(text_display.is_selected()).is_true();
        assert_that!(text_display.first_row).is_equal_to(12);

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);
        text_display.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        text_display.navigate(-1);

        assert_that!(text_display.first_row).is_equal_to(7);
        assert_that!(text_display.selected_idx).is_equal_to(Some(2));
        assert_that!(text_display.is_selected()).is_true();

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);
        text_display.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn select_text2() {
        let text = vec![
            Text::new(
                "Hello, World!\nHere's some inselectable text".to_string(),
                false,
            ),
            Text::new(
                "Hello, World!\nHere's some selectable text 1".to_string(),
                true,
            ),
            Text::new(
                "Hello, World!\nHere's some selectable text 2".to_string(),
                true,
            ),
            Text::new(
                "Hello, World!\nHere's some selectable text 3".to_string(),
                true,
            ),
            Text::new(
                "Hello, World!\nHere's some selectable text 4".to_string(),
                true,
            ),
            Text::new(
                "Hello, World!\nHere's some selectable text 5".to_string(),
                true,
            ),
            Text::new(
                "Hello, World!\nHere's some selectable text 6".to_string(),
                true,
            ),
        ];
        let block_title = Some("Block Title".to_string());
        let mut text_display = TextDisplay::new_with_text(text, block_title, true);

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);
        text_display.render_ref(rect, &mut buffer);

        text_display.navigate(5);

        assert_that!(text_display.selected_idx).is_equal_to(Some(5));
        assert_that!(text_display.is_selected()).is_true();
        assert_that!(text_display.first_row).is_equal_to(6);

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);
        text_display.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        text_display.navigate(-2);

        assert_that!(text_display.selected_idx).is_equal_to(Some(3));
        assert_that!(text_display.is_selected()).is_true();
        assert_that!(text_display.first_row).is_equal_to(6);

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);
        text_display.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        text_display.navigate(-2);

        assert_that!(text_display.selected_idx).is_equal_to(Some(1));
        assert_that!(text_display.is_selected()).is_true();
        assert_that!(text_display.first_row).is_equal_to(2);

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);
        text_display.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        text_display.navigate(-1);

        assert_that!(text_display.selected_idx).is_equal_to(None);
        assert_that!(text_display.is_selected()).is_false();
        assert_that!(text_display.first_row).is_equal_to(0);

        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);
        text_display.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn scroll_text() {}

    #[traced_test]
    #[test]
    fn scroll_and_select_text() {}
}
