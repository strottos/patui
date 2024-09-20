use std::{cell::Cell, cmp};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Cell as RatatuiCell, Row, Table as RatatuiTable, WidgetRef},
};

use super::patui_widget::ScrollType;

const SHORT_WIDTH_DISPLAY: u16 = 60;

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
#[derive(Clone, Debug, Default)]
pub(crate) struct Table<'a> {
    short_header: Vec<TableHeader<'a>>,
    long_header: Vec<TableHeader<'a>>,
    elements: Vec<Vec<Text<'a>>>,

    first_row: isize,
    selected_idx: isize,
    is_selected: bool,

    display_height: Cell<usize>,
}

impl<'a> Table<'a> {
    pub(crate) fn new_with_elements(
        elements: Vec<Vec<Text<'a>>>,
        short_header: Vec<TableHeader<'a>>,
        long_header: Vec<TableHeader<'a>>,
    ) -> Self {
        Self {
            short_header,
            long_header,
            elements,

            first_row: 0,
            selected_idx: -1,
            is_selected: false,

            display_height: Cell::new(24),
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
        if self.is_selected && self.selected_idx >= 0 {
            Some(self.selected_idx as usize)
        } else {
            None
        }
    }

    /// Get the number of tests left.
    pub(crate) fn elements_left(&self) -> Option<usize> {
        assert!(self.elements.len() <= self.selected_idx as usize);
        if self.is_selected && self.selected_idx >= 0 {
            Some(self.elements.len() - self.selected_idx as usize)
        } else {
            None
        }
    }

    pub(crate) fn set_unselected(&mut self) {
        self.is_selected = false;
    }

    /// Navigate the table. This function will move the selected index by `count` elements. If we
    /// don't have enough elements to move, we will move to the first or last element. This does
    /// not support horizontal navigation or wrapping.
    ///
    /// Returns the number of elements altered by, 0 implies nothing was changed.
    pub(crate) fn navigate(&mut self, count: isize) -> isize {
        let old_selected_idx = self.selected_idx;

        if self.elements.is_empty() || count == 0 {
            return 0;
        }

        // If we've not selected already just select the first element as we don't support
        // wrapping.
        if self.selected_idx == -1 && count < 0 {
            self.selected_idx = 0;
            self.is_selected = true;
            return 1;
        }

        self.selected_idx += count;

        if self.selected_idx < 0 {
            self.selected_idx = 0;
        } else if self.selected_idx >= self.elements.len() as isize {
            self.selected_idx = self.elements.len() as isize - 1;
        }

        if old_selected_idx != self.selected_idx {
            self.is_selected = true;
        }

        if self.selected_idx - self.first_row >= self.display_height.get() as isize - 2 {
            self.scroll(ScrollType::Single(
                self.selected_idx - self.display_height.get() as isize + 2,
            ));
        } else if self.selected_idx < self.first_row {
            self.scroll(ScrollType::Single(self.selected_idx - self.first_row));
        }

        self.selected_idx - old_selected_idx
    }

    pub(crate) fn scroll(&mut self, scroll_type: ScrollType) {
        match scroll_type {
            ScrollType::Single(count) => {
                self.first_row += count;
            }
            ScrollType::FullPageDown => {
                let display_height = self.display_height.get();
                self.first_row += display_height as isize;
            }
            ScrollType::FullPageUp => {
                let display_height = self.display_height.get();
                self.first_row -= display_height as isize;
            }
            ScrollType::HalfPageDown => {
                let display_height = self.display_height.get();
                self.first_row += (display_height / 2) as isize;
            }
            ScrollType::HalfPageUp => {
                let display_height = self.display_height.get();
                self.first_row -= (display_height / 2) as isize;
            }
            _ => unreachable!(),
        }

        self.first_row = cmp::min(
            self.elements.len() as isize - self.display_height.get() as isize + 2,
            self.first_row,
        );
        self.first_row = cmp::max(0, self.first_row);
    }
}

impl<'a> WidgetRef for Table<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let is_short = area.width < SHORT_WIDTH_DISPLAY;

        let header = if is_short {
            self.short_header.iter()
        } else {
            self.long_header.iter()
        }
        .map(|header| RatatuiCell::from(header.text.clone()))
        .collect::<Row>()
        .height(2);

        self.display_height.set(area.height as usize);

        let num_elems_to_display = cmp::min((area.height - 2) as usize, self.elements.len());

        let elems = self
            .elements
            .iter()
            .enumerate()
            .filter(|(i, _)| *i >= self.first_row as usize)
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
                if self.is_selected && self.selected_idx as usize == i {
                    row.style(Style::default().fg(Color::Black).bg(Color::White))
                } else {
                    row
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

        table.render_ref(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;
    use ratatui::{
        buffer::Buffer,
        layout::{Alignment, Constraint, Rect},
        text::Text,
        widgets::WidgetRef,
    };
    use tracing_test::traced_test;

    use crate::tui::widgets::patui_widget::ScrollType;

    use super::{Table, TableHeader};

    fn create_tests_table<'a>(num_tests: usize) -> Table<'a> {
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

        Table::new_with_elements(
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
        )
    }

    #[traced_test]
    #[test]
    fn test_display_table_short_width() {
        let table = create_tests_table(8);
        let rect = Rect::new(0, 0, 50, 10);
        let mut buffer = Buffer::empty(rect);

        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_display_table_normal() {
        let table = create_tests_table(8);
        let rect = Rect::new(0, 0, 80, 10);
        let mut buffer = Buffer::empty(rect);

        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_display_table_with_offsets() {
        let mut table = create_tests_table(40);
        table.first_row = 10;
        let rect = Rect::new(0, 0, 120, 20);
        let mut buffer = Buffer::empty(rect);

        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_display_table_with_selected_idx() {
        let mut table = create_tests_table(40);
        table.first_row = 10;
        table.selected_idx = 12;
        table.is_selected = true;
        let rect = Rect::new(0, 0, 120, 20);
        let mut buffer = Buffer::empty(rect);

        table.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scroll_table() {
        let mut table = create_tests_table(40);
        table.display_height.set(24);

        table.scroll(ScrollType::Single(10));

        assert_that!(table.first_row).is_equal_to(10);

        table.scroll(ScrollType::Single(-20));

        assert_that!(table.first_row).is_equal_to(0);

        table.scroll(ScrollType::FullPageDown);

        assert_that!(table.first_row).is_equal_to(18);

        table.scroll(ScrollType::FullPageUp);

        assert_that!(table.first_row).is_equal_to(0);

        table.scroll(ScrollType::HalfPageDown);

        assert_that!(table.first_row).is_equal_to(12);

        table.scroll(ScrollType::HalfPageUp);

        assert_that!(table.first_row).is_equal_to(0);
    }

    #[traced_test]
    #[test]
    fn test_navigate_table() {
        let mut table = create_tests_table(40);
        table.display_height.set(24);

        let ret = table.navigate(10);

        assert_that!(ret).is_equal_to(10);
        assert_that!(table.first_row).is_equal_to(0);
        assert_that!(table.selected_idx).is_equal_to(9);
        assert_that!(table.is_selected).is_equal_to(true);

        let ret = table.navigate(20);

        assert_that!(ret).is_equal_to(20);
        assert_that!(table.first_row).is_equal_to(7);
        assert_that!(table.selected_idx).is_equal_to(29);
        assert_that!(table.is_selected).is_equal_to(true);

        let ret = table.navigate(20);

        assert_that!(ret).is_equal_to(10);
        assert_that!(table.first_row).is_equal_to(18);
        assert_that!(table.selected_idx).is_equal_to(39);
        assert_that!(table.is_selected).is_equal_to(true);

        let ret = table.navigate(-20);

        assert_that!(ret).is_equal_to(-20);
        assert_that!(table.first_row).is_equal_to(18);
        assert_that!(table.selected_idx).is_equal_to(19);
        assert_that!(table.is_selected).is_equal_to(true);

        let ret = table.navigate(-20);

        assert_that!(ret).is_equal_to(-19);
        assert_that!(table.first_row).is_equal_to(0);
        assert_that!(table.selected_idx).is_equal_to(0);
        assert_that!(table.is_selected).is_equal_to(true);
    }
}
