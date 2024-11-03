use std::{cell::Cell, cmp};

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{
        Block, Borders, Cell as RatatuiCell, Padding, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Table as RatatuiTable, WidgetRef,
    },
};

use super::patui_widget::ScrollType;

const SHORT_WIDTH_DISPLAY: u16 = 60;

#[derive(Debug, Clone)]
pub(crate) struct SelectedData {
    selectable: bool,
    wrappable: bool,
    first_row: usize,
    selected_idx: isize,
    is_selected: bool,
    num_elements: usize,
    num_display_elements: Cell<usize>,
}

impl SelectedData {
    pub(crate) fn new(
        selectable: bool,
        wrappable: bool,
        num_elements: usize,
        num_display_elements: usize,
    ) -> Self {
        Self {
            selectable,
            wrappable,
            first_row: 0,
            selected_idx: -1,
            is_selected: false,
            num_elements,
            num_display_elements: Cell::new(num_display_elements),
        }
    }

    pub(crate) fn is_selectable(&self) -> bool {
        self.selectable
    }

    pub(crate) fn is_selected(&self) -> bool {
        if self.selectable && self.is_selected {
            true
        } else {
            false
        }
    }

    pub(crate) fn selected_idx(&self) -> Option<usize> {
        if self.is_selected() && self.selected_idx >= 0 {
            Some(self.selected_idx as usize)
        } else {
            None
        }
    }

    pub(crate) fn first_row(&self) -> usize {
        self.first_row
    }

    pub(crate) fn num_display_elements(&self) -> usize {
        self.num_display_elements.get()
    }

    pub(crate) fn set_unselected(&mut self) {
        if self.is_selected() {
            self.is_selected = false;
            self.selected_idx = -1;
        }
    }

    pub(crate) fn set_selected_idx(&mut self, selected_idx: usize) {
        if self.selectable {
            debug_assert!(selected_idx < self.num_elements);
            self.selected_idx = selected_idx as isize;
            self.is_selected = true;
        }
    }

    pub(crate) fn add_selected_idx(&mut self, count: isize) -> isize {
        if !self.selectable {
            return 0;
        }

        // If we've not selected already just select the first element as we don't support
        // wrapping.
        if self.selected_idx == -1 && count < 0 {
            self.set_selected_idx(0);
            return 1;
        }

        let old_selected_idx = self.selected_idx;

        self.selected_idx += count;

        if self.selected_idx < 0 {
            if !self.wrappable {
                self.selected_idx = 0;
            } else {
                self.selected_idx = self.num_elements as isize + self.selected_idx;
            }
        } else if self.selected_idx >= self.num_elements as isize {
            if !self.wrappable {
                self.selected_idx = self.num_elements as isize - 1;
            } else {
                self.selected_idx = self.selected_idx - self.num_elements as isize;
            }
        }

        debug_assert!(self.selected_idx >= 0 && self.selected_idx < self.num_elements as isize);
        self.is_selected = true;

        let min_first_row = cmp::max(
            0,
            self.selected_idx - self.num_display_elements.get() as isize + 1,
        ) as usize;
        self.first_row = cmp::max(self.first_row, min_first_row);

        let max_first_row = cmp::min(self.first_row, self.selected_idx as usize);
        self.first_row = cmp::min(self.first_row, max_first_row);

        self.selected_idx - old_selected_idx
    }

    pub(crate) fn set_first_row(&mut self, first_row: usize) {
        debug_assert!(first_row < self.num_elements);
        debug_assert!(
            !self.selectable || !self.is_selected || first_row < self.selected_idx as usize
        );
        self.first_row = first_row;
    }

    pub(crate) fn add_first_row(&mut self, shift: isize) {
        let mut first_row = self.first_row as isize;
        first_row += shift;
        first_row = cmp::max(0, first_row);
        first_row = cmp::min(
            cmp::max(
                0,
                self.num_elements as isize - self.num_display_elements.get() as isize,
            ),
            first_row,
        );
        self.first_row = first_row as usize;

        assert!(
            self.first_row
                <= cmp::max(
                    0,
                    self.num_elements as isize - self.num_display_elements.get() as isize
                ) as usize
        );

        self.selected_idx = cmp::max(self.first_row as isize, self.selected_idx);
        self.selected_idx = cmp::min(
            self.selected_idx,
            (self.first_row + self.num_display_elements.get() - 1) as isize,
        );
    }

    pub(crate) fn set_display_height(&self, height: usize) {
        self.num_display_elements.set(height);
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct TableHeader<'a> {
    text: Text<'a>,
    index: usize,
    constraint: Constraint,
}

impl<'a> TableHeader<'a> {
    pub(crate) fn new(text: Text<'a>, index: usize, constraint: Constraint) -> Self {
        Self {
            text,
            index,
            constraint,
        }
    }
}

/// For the table widget you need to pass in:
/// * The `short_header` that will be the elements displayed along with their index in the
///   `elements` vector when the width of the table is less than `SHORT_WIDTH_DISPLAY`.
/// * The `long_header` that will be the elements displayed along with their index in the
///   `elements` vector when the width of the table is greater than `SHORT_WIDTH_DISPLAY`.
/// * The `elements` that will be the data displayed in the table.
#[derive(Clone, Debug)]
pub(crate) struct Table<'a> {
    short_header: Vec<TableHeader<'a>>,
    long_header: Vec<TableHeader<'a>>,
    elements: Vec<Vec<Text<'a>>>,

    block_title: Option<&'a str>,

    selected_data: SelectedData,

    is_focussed: bool,
}

impl<'a> Table<'a> {
    pub(crate) fn new_with_elements(
        elements: Vec<Vec<Text<'a>>>,
        short_header: Vec<TableHeader<'a>>,
        long_header: Vec<TableHeader<'a>>,
        block_title: Option<&'a str>,
        is_selectable: bool,
    ) -> Self {
        let elements_len = elements.len();

        Self {
            short_header,
            long_header,
            elements,

            block_title,

            selected_data: SelectedData::new(is_selectable, false, elements_len, 24),

            is_focussed: false,
        }
    }

    pub(crate) fn scrollable_height(&self) -> usize {
        self.elements.len() + 2
    }

    pub(crate) fn num_elements(&self) -> usize {
        self.elements.len()
    }

    /// Get the selected index if we've selected something.
    pub(crate) fn selected_idx(&self) -> Option<usize> {
        self.selected_data.selected_idx()
    }

    /// Get the number of elements left.
    pub(crate) fn elements_left(&self) -> Option<usize> {
        self.selected_idx().map(|selected_idx| {
            debug_assert!(self.elements.len() <= selected_idx);
            self.elements.len() - selected_idx as usize
        })
    }

    /// Navigate the table. This function will move the selected index by `count` elements. If we
    /// don't have enough elements to move, we will move to the first or last element. This does
    /// not support horizontal navigation or wrapping.
    ///
    /// Returns the number of elements altered by, 0 implies nothing was changed.
    pub(crate) fn navigate(&mut self, count: isize) -> isize {
        if !self.selected_data.is_selectable() || self.elements.is_empty() || count == 0 {
            return 0;
        }

        self.selected_data.add_selected_idx(count)
    }

    pub(crate) fn scroll(&mut self, scroll_type: ScrollType) {
        let display_height = self.selected_data.num_display_elements();
        self.selected_data.add_first_row(match scroll_type {
            ScrollType::Single(count) => count,
            ScrollType::FullPageDown => display_height as isize,
            ScrollType::FullPageUp => -(display_height as isize),
            ScrollType::HalfPageDown => (display_height / 2) as isize,
            ScrollType::HalfPageUp => -((display_height / 2) as isize),
            _ => unreachable!(),
        });
    }

    pub(crate) fn set_focus(&mut self, focus: bool) {
        self.is_focussed = focus;
    }

    pub(crate) fn reset(&mut self) {
        self.selected_data.set_unselected();
    }

    fn render_table(&self, area: Rect, buf: &mut Buffer) {
        let style = if self.is_focussed {
            Style::default().fg(Color::White).bg(Color::Black)
        } else {
            Style::default().fg(Color::DarkGray).bg(Color::Black)
        };

        let is_short = area.width < SHORT_WIDTH_DISPLAY;

        let header = if is_short {
            self.short_header.iter()
        } else {
            self.long_header.iter()
        }
        .map(|header| RatatuiCell::from(header.text.clone()).style(style))
        .collect::<Row>()
        .height(2);

        let elements_display_height = if self.block_title.is_some() {
            // -6 for block and title
            area.height as usize - 6
        } else {
            // -2 for title
            area.height as usize - 2
        };
        self.selected_data
            .set_display_height(elements_display_height);

        let num_elems_to_display = cmp::min(elements_display_height, self.elements.len());

        let elems = self
            .elements
            .iter()
            .enumerate()
            .filter(|(i, _)| *i >= self.selected_data.first_row() as usize)
            .take(num_elems_to_display)
            .map(|(i, row)| {
                (
                    i,
                    row.iter()
                        .enumerate()
                        .filter_map(|(j, elem)| {
                            if (is_short
                                && self.short_header.iter().any(|header| j == header.index))
                                || (!is_short
                                    && self.long_header.iter().any(|header| j == header.index))
                            {
                                Some(RatatuiCell::from(elem.clone()))
                            } else {
                                None
                            }
                        })
                        .collect::<Row>()
                        .height(1),
                )
            })
            .map(|(i, row)| {
                if self.selected_data.selected_idx() == Some(i) {
                    row.style(style.fg(Color::Black).bg(Color::White))
                } else {
                    row.style(style)
                }
            })
            .collect::<Vec<Row>>();

        let table = if is_short {
            RatatuiTable::new(
                elems,
                self.short_header
                    .iter()
                    .map(|header| header.constraint)
                    .collect::<Vec<_>>(),
            )
        } else {
            RatatuiTable::new(
                elems,
                self.long_header
                    .iter()
                    .map(|header| header.constraint)
                    .collect::<Vec<_>>(),
            )
        }
        .header(header);

        let table = if let Some(block_title) = self.block_title {
            table.block(
                Block::new()
                    .borders(Borders::ALL)
                    .padding(Padding::symmetric(2, 1))
                    .title_alignment(Alignment::Center)
                    .title(block_title)
                    .style(style),
            )
        } else {
            table
        };

        table.render_ref(area, buf);
    }

    fn render_scrollbar(&self, area: Rect, buf: &mut Buffer) {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let num_elements = self.num_elements();
        let display_height = self.selected_data.num_display_elements();

        let scrollbar_height = if num_elements <= display_height {
            0
        } else {
            num_elements + 1 - display_height
        };

        let mut scrollbar_state =
            ScrollbarState::new(scrollbar_height).position(self.selected_data.first_row());

        scrollbar.render(area, buf, &mut scrollbar_state);
    }
}

impl<'a> WidgetRef for Table<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.render_table(area, buf);
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
    use std::cmp;

    use assertor::*;
    use ratatui::{
        buffer::Buffer,
        layout::{Alignment, Constraint, Rect},
        text::Text,
        widgets::WidgetRef,
    };
    use tracing_test::traced_test;

    use crate::tui::widgets::patui_widget::ScrollType;

    use super::{SelectedData, Table, TableHeader};

    fn create_tests_table<'a>(
        num_tests: usize,
        block_title: Option<&'a str>,
        is_selectable: bool,
    ) -> Table<'a> {
        let now = "2024-08-31 11:00:00";

        let tests = (0..num_tests)
            .map(|i| {
                Vec::from([
                    Text::from(format!("test{}", i)),
                    Text::from(format!("test description {}", i)),
                    Text::from(now).alignment(Alignment::Right),
                    Text::from(now),
                    "".into(),
                    Text::from("0").alignment(Alignment::Right),
                ])
            })
            .collect();

        let mut table = Table::new_with_elements(
            tests,
            vec![
                TableHeader::new("Name".into(), 0, Constraint::Min(12)),
                TableHeader::new(
                    Text::from("Creation Date").alignment(Alignment::Right),
                    2,
                    Constraint::Max(20),
                ),
            ],
            vec![
                TableHeader::new("Name".into(), 0, Constraint::Min(12)),
                TableHeader::new("Description".into(), 1, Constraint::Min(12)),
                TableHeader::new("Creation Date".into(), 3, Constraint::Max(20)),
                TableHeader::new("Last Used".into(), 4, Constraint::Max(20)),
                TableHeader::new("Times Used".into(), 5, Constraint::Max(10)),
            ],
            block_title,
            is_selectable,
        );

        table.is_focussed = true;

        table
    }

    #[traced_test]
    #[test]
    fn add_selected_idx_non_wrappable() {
        let mut selected_data = SelectedData::new(true, false, 20, 10);

        assert_that!(selected_data.add_selected_idx(10)).is_equal_to(10);
        assert_that!(selected_data.selected_idx).is_equal_to(9);
        assert_that!(selected_data.add_selected_idx(10)).is_equal_to(10);
        assert_that!(selected_data.selected_idx).is_equal_to(19);
        assert_that!(selected_data.add_selected_idx(10)).is_equal_to(0);
        assert_that!(selected_data.selected_idx).is_equal_to(19);
    }

    #[traced_test]
    #[test]
    fn add_selected_idx_non_wrappable_reverse() {
        let mut selected_data = SelectedData::new(true, false, 20, 10);

        assert_that!(selected_data.add_selected_idx(-10)).is_equal_to(1);
        assert_that!(selected_data.selected_idx).is_equal_to(0);
        assert_that!(selected_data.add_selected_idx(10)).is_equal_to(10);
        assert_that!(selected_data.selected_idx).is_equal_to(10);
        assert_that!(selected_data.add_selected_idx(-5)).is_equal_to(-5);
        assert_that!(selected_data.selected_idx).is_equal_to(5);
    }

    #[traced_test]
    #[test]
    fn add_selected_idx_wrappable() {
        let mut selected_data = SelectedData::new(true, true, 20, 10);

        assert_that!(selected_data.add_selected_idx(10)).is_equal_to(10);
        assert_that!(selected_data.selected_idx).is_equal_to(9);
        assert_that!(selected_data.add_selected_idx(10)).is_equal_to(10);
        assert_that!(selected_data.selected_idx).is_equal_to(19);
        assert_that!(selected_data.add_selected_idx(10)).is_equal_to(-10);
        assert_that!(selected_data.selected_idx).is_equal_to(9);
    }

    #[traced_test]
    #[test]
    fn add_selected_idx_wrappable_reverse() {
        let mut selected_data = SelectedData::new(true, true, 20, 10);

        assert_that!(selected_data.add_selected_idx(-10)).is_equal_to(1);
        assert_that!(selected_data.selected_idx).is_equal_to(0);
        assert_that!(selected_data.add_selected_idx(-10)).is_equal_to(10);
        assert_that!(selected_data.selected_idx).is_equal_to(10);
        assert_that!(selected_data.add_selected_idx(-5)).is_equal_to(-5);
        assert_that!(selected_data.selected_idx).is_equal_to(5);
    }

    #[traced_test]
    #[test]
    fn add_first_row_non_wrappable() {
        let mut selected_data = SelectedData::new(true, false, 20, 10);

        selected_data.add_first_row(10);
        assert_that!(selected_data.first_row).is_equal_to(10);
        assert_that!(selected_data.selected_idx()).is_equal_to(None);
        selected_data.add_first_row(10);
        assert_that!(selected_data.first_row).is_equal_to(10);
        assert_that!(selected_data.selected_idx()).is_equal_to(None);
    }

    #[traced_test]
    #[test]
    fn add_first_row_non_wrappable_reverse() {
        let mut selected_data = SelectedData::new(true, false, 20, 10);

        selected_data.add_first_row(-10);
        assert_that!(selected_data.first_row).is_equal_to(0);
        assert_that!(selected_data.selected_idx()).is_equal_to(None);
        selected_data.add_first_row(10);
        assert_that!(selected_data.first_row).is_equal_to(10);
        assert_that!(selected_data.selected_idx()).is_equal_to(None);
        selected_data.add_first_row(-5);
        assert_that!(selected_data.first_row).is_equal_to(5);
        assert_that!(selected_data.selected_idx()).is_equal_to(None);
    }

    #[traced_test]
    #[test]
    fn add_first_row_with_selection() {
        let mut selected_data = SelectedData::new(true, false, 20, 10);

        selected_data.add_selected_idx(1);
        assert_that!(selected_data.first_row).is_equal_to(0);
        assert_that!(selected_data.selected_idx()).is_equal_to(Some(0));

        selected_data.add_first_row(10);
        assert_that!(selected_data.first_row).is_equal_to(10);
        assert_that!(selected_data.selected_idx()).is_equal_to(Some(10));
        selected_data.add_first_row(10);
        assert_that!(selected_data.first_row).is_equal_to(10);
        assert_that!(selected_data.selected_idx()).is_equal_to(Some(10));
    }

    #[traced_test]
    #[test]
    fn add_first_row_reverse_with_selection() {
        let mut selected_data = SelectedData::new(true, true, 20, 10);

        selected_data.add_selected_idx(-1);
        assert_that!(selected_data.first_row).is_equal_to(0);
        assert_that!(selected_data.selected_idx()).is_equal_to(Some(0));
        selected_data.add_selected_idx(-1);
        assert_that!(selected_data.first_row).is_equal_to(10);
        assert_that!(selected_data.selected_idx()).is_equal_to(Some(19));

        selected_data.add_first_row(-10);
        assert_that!(selected_data.first_row).is_equal_to(0);
        assert_that!(selected_data.selected_idx()).is_equal_to(Some(9));
        selected_data.add_first_row(10);
        assert_that!(selected_data.first_row).is_equal_to(10);
        assert_that!(selected_data.selected_idx()).is_equal_to(Some(10));
        selected_data.add_first_row(-5);
        assert_that!(selected_data.first_row).is_equal_to(5);
        assert_that!(selected_data.selected_idx()).is_equal_to(Some(10));
    }

    #[traced_test]
    #[test]
    fn test_display_table_short_width() {
        let table = create_tests_table(8, None, false);
        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);

        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_display_table_normal() {
        let table = create_tests_table(8, None, false);
        let rect = Rect::new(0, 0, 80, 10);
        let mut buffer = Buffer::empty(rect);

        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_display_table_with_offsets() {
        let mut table = create_tests_table(40, None, false);
        table.selected_data.set_first_row(10);
        let rect = Rect::new(0, 0, 120, 20);
        let mut buffer = Buffer::empty(rect);

        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_display_table_with_selected_idx() {
        let mut table = create_tests_table(40, None, true);
        table.selected_data.set_first_row(10);
        table.selected_data.set_selected_idx(12);
        let rect = Rect::new(0, 0, 120, 20);
        let mut buffer = Buffer::empty(rect);

        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scroll_table() {
        let mut table = create_tests_table(40, None, true);
        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        // Tests
        table.scroll(ScrollType::Single(10));

        assert_that!(table.selected_data.first_row()).is_equal_to(10);
        assert_that!(table.selected_data.is_selected()).is_equal_to(false);

        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        table.scroll(ScrollType::Single(-20));

        assert_that!(table.selected_data.first_row()).is_equal_to(0);
        assert_that!(table.selected_data.is_selected()).is_equal_to(false);

        table.scroll(ScrollType::FullPageDown);

        assert_that!(table.selected_data.first_row()).is_equal_to(18);
        assert_that!(table.selected_data.is_selected()).is_equal_to(false);

        table.scroll(ScrollType::FullPageUp);

        assert_that!(table.selected_data.first_row()).is_equal_to(0);
        assert_that!(table.selected_data.is_selected()).is_equal_to(false);

        table.scroll(ScrollType::HalfPageDown);

        assert_that!(table.selected_data.first_row()).is_equal_to(11);
        assert_that!(table.selected_data.is_selected()).is_equal_to(false);

        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        table.scroll(ScrollType::HalfPageUp);

        assert_that!(table.selected_data.first_row()).is_equal_to(0);
        assert_that!(table.selected_data.is_selected()).is_equal_to(false);
    }

    #[traced_test]
    #[test]
    fn test_navigate_table() {
        let mut table = create_tests_table(40, None, true);
        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        let ret = table.navigate(10);

        assert_that!(ret).is_equal_to(10);
        assert_that!(table.selected_data.first_row()).is_equal_to(0);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(9));

        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        let ret = table.navigate(20);

        assert_that!(ret).is_equal_to(20);
        assert_that!(table.selected_data.first_row()).is_equal_to(8);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(29));

        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        let ret = table.navigate(20);

        assert_that!(ret).is_equal_to(10);
        assert_that!(table.selected_data.first_row()).is_equal_to(18);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(39));

        let ret = table.navigate(-20);

        assert_that!(ret).is_equal_to(-20);
        assert_that!(table.selected_data.first_row()).is_equal_to(18);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(19));

        let ret = table.navigate(-20);

        assert_that!(ret).is_equal_to(-19);
        assert_that!(table.selected_data.first_row()).is_equal_to(0);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(0));

        table.navigate(50);
        table.navigate(-10);

        assert_that!(table.selected_data.first_row()).is_equal_to(18);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(29));

        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scroll_with_selection_table() {
        let mut table = create_tests_table(40, None, true);
        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        let ret = table.navigate(1);

        assert_that!(ret).is_equal_to(1);
        assert_that!(table.selected_data.first_row()).is_equal_to(0);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(0));

        // Test scroll
        table.scroll(ScrollType::Single(10));

        assert_that!(table.selected_data.first_row()).is_equal_to(10);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(10));

        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        table.scroll(ScrollType::Single(-20));

        assert_that!(table.selected_data.first_row()).is_equal_to(0);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(10));

        table.scroll(ScrollType::FullPageDown);

        assert_that!(table.selected_data.first_row()).is_equal_to(18);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(18));

        table.scroll(ScrollType::FullPageUp);

        assert_that!(table.selected_data.first_row()).is_equal_to(0);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(18));

        table.scroll(ScrollType::HalfPageDown);

        assert_that!(table.selected_data.first_row()).is_equal_to(11);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(18));

        table.scroll(ScrollType::HalfPageUp);

        assert_that!(table.selected_data.first_row()).is_equal_to(0);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(18));

        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);

        table.scroll(ScrollType::HalfPageDown);
        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_bordered_table() {
        let mut table = create_tests_table(40, Some("My Table"), true);
        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        let ret = table.navigate(18);

        assert_that!(ret).is_equal_to(18);
        assert_that!(table.selected_data.first_row()).is_equal_to(0);
        assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(17));

        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);

        for i in 1..11 {
            let ret = table.navigate(1);

            assert_that!(ret).is_equal_to(1);
            assert_that!(table.selected_data.first_row()).is_equal_to(i);
            assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(17 + i));
        }

        for i in 1..25 {
            let ret = table.navigate(-1);

            tracing::trace!("i: {i}");
            assert_that!(ret).is_equal_to(-1);
            assert_that!(table.selected_data.first_row()).is_equal_to(cmp::min(10, 27 - i));
            assert_that!(table.selected_data.selected_idx()).is_equal_to(Some(27 - i));
        }

        table.navigate(5);

        let rect = Rect::new(0, 0, 120, 24);
        let mut buffer = Buffer::empty(rect);
        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }
}
