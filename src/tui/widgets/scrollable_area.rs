use std::{cell::Cell, cmp};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::BlockExt,
    widgets::{Block, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, WidgetRef},
};
use tracing::{debug, trace};

use super::patui_widget::PatuiWidget;

#[derive(Debug)]
pub(crate) struct ScrollableArea<'a> {
    first_row: isize,
    selected_rows: (isize, isize),
    display_height: Cell<usize>,
    widgets: Vec<PatuiWidget<'a>>,
    block: Option<Block<'a>>,
}

impl<'a> ScrollableArea<'a> {
    pub(crate) fn new_patui_widget() -> PatuiWidget<'a> {
        PatuiWidget::ScrollableArea(Self {
            first_row: 0,
            selected_rows: (-1, -1),
            display_height: Cell::new(24),
            widgets: vec![],
            block: None,
        })
    }

    pub(crate) fn add_widget(&mut self, widget: PatuiWidget<'a>) -> &mut Self {
        self.widgets.push(widget);
        self
    }

    pub(crate) fn set_widgets(&mut self, widgets: Vec<PatuiWidget<'a>>) -> &mut Self {
        self.widgets = widgets.into_iter().map(|widget| widget).collect();
        self
    }

    pub(crate) fn add_block(&mut self, block: Block<'a>) -> &mut Self {
        self.block = Some(block);
        self
    }

    /// Alters the selected_line by the count given with optional wrapping
    /// and if necessary scrolls the page to ensure the selected_line is
    /// visible.
    ///
    /// Returns true if changed else false
    pub(crate) fn navigate(&mut self, scroll_lines: isize, wrap_around: bool) {
        todo!()
    }

    pub(crate) fn scroll(&mut self, scroll_lines: isize) {
        self.first_row += scroll_lines;
        self.first_row = cmp::max(0, self.first_row);
        self.first_row = cmp::min(
            self.get_total_height() as isize - self.display_height.get() as isize,
            self.first_row,
        );
    }

    /// Renders the widgets in the scrollale area that should be visible.
    fn render_main(&self, area: Rect, buf: &mut Buffer) {
        let area_height = area.height as usize;

        self.display_height.set(area_height);

        let mut current_height = 0;
        let mut current_row = 0;

        for widget in &self.widgets {
            let widget_height = widget.scrollable_height();

            let skip_lines = if current_row < self.first_row as usize {
                self.first_row as usize - current_row
            } else {
                0
            };
            current_row += widget_height;

            if current_height + widget_height <= self.first_row as usize {
                continue;
            }

            let widget_area = Rect {
                x: area.x,
                y: area.y + current_height as u16,
                width: area.width,
                height: cmp::min(area_height - current_height, widget_height - skip_lines) as u16,
            };

            widget
                .set_render_from_line(skip_lines)
                .render_ref(widget_area, buf);

            current_height += widget_height - skip_lines;

            if current_height >= area_height {
                break;
            }
        }
    }

    /// Renders the scrollbar on the right side of the area.
    fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let scrollbar_height = if self.get_total_height() <= self.display_height.get() {
            0
        } else {
            self.get_total_height() - self.display_height.get()
        };

        let mut scrollbar_state =
            ScrollbarState::new(scrollbar_height).position(self.first_row as usize);

        scrollbar.render(area, buf, &mut scrollbar_state);
    }

    fn get_total_height(&self) -> usize {
        self.widgets
            .iter()
            .map(|widget| widget.scrollable_height())
            .sum()
    }
}

impl<'a> WidgetRef for ScrollableArea<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        debug!("Rendering scrollable area: {:?}", area);
        self.block.render_ref(area, buf);
        let inner = self.block.inner_if_some(area);
        let inner_main = Rect {
            x: inner.x,
            y: inner.y,
            width: inner.width - 1,
            height: inner.height,
        };
        self.render_main(inner_main, buf);
        let scrollbar_rect = if self.block.is_some() {
            // NB: The scrollbar is rendered on the right side of the block, we have to make
            // assumptions about the border so far, or more work needed.
            Rect {
                x: area.x + area.width - 2,
                y: area.y + 1,
                width: 1,
                height: area.height - 2,
            }
        } else {
            Rect {
                x: area.width - 1,
                y: area.y,
                width: 1,
                height: area.height,
            }
        };
        trace!("Rendering scrollbar rect: {:?}", scrollbar_rect);
        self.render_scrollbar(scrollbar_rect, buf);
        trace!("Rendered scrollable area: {:?}", buf);
    }
}

// impl<'a> PatuiWidget for ScrollableArea<'a> {
// fn scrollable_height(&self) -> usize {
//     self.widgets
//         .iter()
//         .map(|widget| widget.scrollable_height())
//         .sum()
// }

// fn num_widgets(&self) -> usize {
//     self.widgets.iter().map(|widget| widget.num_widgets()).sum()
// }
// }

#[cfg(test)]
mod tests {
    use crate::tui::widgets::patui_widget::TestWidget;

    use super::*;
    use assertor::*;
    use ratatui::widgets::{Borders, Padding};

    fn get_widget_calls(scrollable_area: &ScrollableArea) -> Vec<TestWidget> {
        scrollable_area
            .widgets
            .iter()
            .map(|widget| widget.get_test_inner().unwrap().clone())
            .collect::<Vec<_>>()
    }

    #[test]
    fn test_scrollable_area_simple() {
        let widgets = vec![
            PatuiWidget::TestWidget(TestWidget::new(3, 1)),
            PatuiWidget::TestWidget(TestWidget::new(6, 1)),
        ];
        let scrollable_area = ScrollableArea {
            first_row: 0,
            selected_rows: (-1, -1),
            display_height: Cell::new(24),
            widgets,
            block: None,
        };
        let rect = Rect::new(0, 0, 10, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        assert_that!(scrollable_area.display_height.get()).is_equal_to(10);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(2);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(1);
        assert_that!(test_widgets[0].calls.borrow()[0]).is_equal_to(Rect::new(0, 0, 9, 3));
        assert_that!(test_widgets[0].render_from_line.get()).is_equal_to(0);
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(test_widgets[1].calls.borrow()[0]).is_equal_to(Rect::new(0, 3, 9, 6));
        assert_that!(test_widgets[1].render_from_line.get()).is_equal_to(0);

        insta::assert_debug_snapshot!(buffer);
    }

    #[test]
    fn test_scrollable_area_with_overlaps() {
        let widgets = vec![
            PatuiWidget::TestWidget(TestWidget::new(3, 1)),
            PatuiWidget::TestWidget(TestWidget::new(6, 1)),
            PatuiWidget::TestWidget(TestWidget::new(7, 1)),
            PatuiWidget::TestWidget(TestWidget::new(3, 1)),
        ];
        let scrollable_area = ScrollableArea {
            first_row: 0,
            selected_rows: (-1, -1),
            display_height: Cell::new(24),
            widgets,
            block: None,
        };
        let rect = Rect::new(0, 0, 10, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(4);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(1);
        assert_that!(test_widgets[0].calls.borrow()[0]).is_equal_to(Rect::new(0, 0, 9, 3));
        assert_that!(test_widgets[0].render_from_line.get()).is_equal_to(0);
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(test_widgets[1].calls.borrow()[0]).is_equal_to(Rect::new(0, 3, 9, 6));
        assert_that!(test_widgets[1].render_from_line.get()).is_equal_to(0);
        assert_that!(*test_widgets[2].calls.borrow()).has_length(1);
        assert_that!(test_widgets[2].calls.borrow()[0]).is_equal_to(Rect::new(0, 9, 9, 1));
        assert_that!(test_widgets[2].render_from_line.get()).is_equal_to(0);
        assert_that!(*test_widgets[3].calls.borrow()).has_length(0);

        insta::assert_debug_snapshot!(buffer);
    }

    #[test]
    fn test_scrollable_area_with_different_first_line() {
        let widgets = vec![
            PatuiWidget::TestWidget(TestWidget::new(3, 1)),
            PatuiWidget::TestWidget(TestWidget::new(6, 1)),
            PatuiWidget::TestWidget(TestWidget::new(7, 1)),
            PatuiWidget::TestWidget(TestWidget::new(3, 1)),
        ];
        let scrollable_area = ScrollableArea {
            first_row: 5,
            selected_rows: (-1, -1),
            display_height: Cell::new(24),
            widgets,
            block: None,
        };
        let rect = Rect::new(0, 0, 10, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(4);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(0);
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(test_widgets[1].calls.borrow()[0]).is_equal_to(Rect::new(0, 0, 9, 4));
        assert_that!(test_widgets[1].render_from_line.get()).is_equal_to(2);
        assert_that!(*test_widgets[2].calls.borrow()).has_length(1);
        assert_that!(test_widgets[2].calls.borrow()[0]).is_equal_to(Rect::new(0, 4, 9, 6));
        assert_that!(test_widgets[2].render_from_line.get()).is_equal_to(0);
        assert_that!(*test_widgets[3].calls.borrow()).has_length(0);

        insta::assert_debug_snapshot!(buffer);
    }

    #[test]
    fn test_simple_scroll() {
        let widgets = (0..12)
            .map(|_| PatuiWidget::TestWidget(TestWidget::new(1, 1)))
            .collect::<Vec<_>>();
        let mut scrollable_area = ScrollableArea {
            first_row: 0,
            selected_rows: (-1, -1),
            display_height: Cell::new(5),
            widgets,
            block: None,
        };
        let rect = Rect::new(0, 0, 5, 5);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);
        scrollable_area.scroll(5);

        assert_that!(scrollable_area.first_row).is_equal_to(5);
        assert_that!(scrollable_area.selected_rows).is_equal_to((-1, -1));

        scrollable_area.scroll(5);

        assert_that!(scrollable_area.first_row).is_equal_to(7);
        assert_that!(scrollable_area.selected_rows).is_equal_to((-1, -1));

        scrollable_area.render_ref(rect, &mut buffer);
        insta::assert_debug_snapshot!(buffer);

        scrollable_area.scroll(5);

        assert_that!(scrollable_area.first_row).is_equal_to(7);
        assert_that!(scrollable_area.selected_rows).is_equal_to((-1, -1));

        scrollable_area.scroll(-5);

        assert_that!(scrollable_area.first_row).is_equal_to(2);
        assert_that!(scrollable_area.selected_rows).is_equal_to((-1, -1));

        scrollable_area.scroll(-5);

        assert_that!(scrollable_area.first_row).is_equal_to(0);
        assert_that!(scrollable_area.selected_rows).is_equal_to((-1, -1));

        scrollable_area.scroll(-5);

        assert_that!(scrollable_area.first_row).is_equal_to(0);
        assert_that!(scrollable_area.selected_rows).is_equal_to((-1, -1));
    }

    #[test]
    fn test_simple_render_block() {
        let widgets = vec![
            PatuiWidget::TestWidget(TestWidget::new(4, 1)),
            PatuiWidget::TestWidget(TestWidget::new(4, 1)),
            PatuiWidget::TestWidget(TestWidget::new(4, 1)),
        ];
        let scrollable_area = ScrollableArea {
            first_row: 0,
            selected_rows: (-1, -1),
            display_height: Cell::new(5),
            widgets,
            block: Some(
                Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::symmetric(2, 1)),
            ),
        };
        let rect = Rect::new(2, 2, 10, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(3);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(1);
        assert_that!(test_widgets[0].calls.borrow()[0]).is_equal_to(Rect::new(5, 4, 3, 4));
        assert_that!(test_widgets[0].render_from_line.get()).is_equal_to(0);
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(test_widgets[1].calls.borrow()[0]).is_equal_to(Rect::new(5, 8, 3, 2));
        assert_that!(test_widgets[1].render_from_line.get()).is_equal_to(0);
        assert_that!(*test_widgets[2].calls.borrow()).has_length(0);

        insta::assert_debug_snapshot!(buffer);
    }
}
