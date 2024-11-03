use std::{cell::Cell, cmp};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eyre::Result;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Style},
    widgets::{
        Block, Borders, Padding, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        WidgetRef,
    },
};
use tracing::debug;

use crate::tui::widgets::patui_widget::ScrollType;

use super::{patui_widget::PatuiWidget, Table};

#[derive(Clone, Debug)]
pub(crate) struct PatuiWidgetData<'a> {
    pub(crate) inner: PatuiWidget<'a>,
    pub(crate) starting_idx: usize,
}

impl<'a> PatuiWidgetData<'a> {
    fn is_selectable(&self) -> bool {
        self.inner.is_selectable()
    }

    fn inner_table_mut(&mut self) -> Option<&mut Table<'a>> {
        self.inner.inner_table_mut()
    }

    fn scrollable_height(&self) -> usize {
        self.inner.scrollable_height()
    }

    fn num_widgets(&self) -> usize {
        self.inner.num_widgets()
    }

    fn set_selected(&self, selected: bool) -> &Self {
        self.inner.set_selected(selected);
        self
    }
}

impl<'a> WidgetRef for PatuiWidgetData<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.inner.render_ref(area, buf);
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ScrollableArea<'a> {
    first_row: isize,
    first_col: isize,
    selected_idx: isize,
    display_height: Cell<usize>,
    display_width: Cell<usize>,
    widgets: Vec<PatuiWidgetData<'a>>,
    block: Option<(Block<'a>, Borders, Padding)>,
    style: Style,
}

impl<'a> ScrollableArea<'a> {
    pub(crate) fn new_patui_widget() -> PatuiWidget<'a> {
        PatuiWidget::new_scrollable_area(ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(24),
            display_width: Cell::new(24),
            widgets: vec![],
            block: None,
            style: Style::default().fg(Color::DarkGray),
        })
    }

    pub(crate) fn add_widget(&mut self, widget: PatuiWidget<'a>) -> &mut Self {
        self.widgets.push(PatuiWidgetData {
            inner: widget,
            starting_idx: self.get_total_height(),
        });
        self
    }

    pub(crate) fn set_widgets(&mut self, widgets: Vec<PatuiWidget<'a>>) -> &mut Self {
        self.widgets = widgets
            .into_iter()
            .map(|widget| PatuiWidgetData {
                inner: widget,
                starting_idx: self.get_total_height(),
            })
            .collect();
        self
    }

    pub(crate) fn add_block(
        &mut self,
        title: &'a str,
        borders: Borders,
        padding: Padding,
    ) -> &mut Self {
        let block = Block::new()
            .borders(borders)
            .padding(padding)
            .title_alignment(Alignment::Center)
            .title(title);
        self.block = Some((block, borders, padding));
        self
    }

    pub(crate) fn set_highlighted(&mut self, highlighted: bool) -> &mut Self {
        if highlighted {
            self.style = Style::default();
        } else {
            self.style = Style::default().fg(Color::DarkGray);
        }
        self
    }

    /// Alters the selected_line by the count given with optional wrapping
    /// and if necessary scrolls the page to ensure the selected_line is
    /// visible.
    ///
    /// Returns true if changed else false
    pub(crate) fn navigate(&mut self, mut count: isize, wrap_around: bool) -> bool {
        let old_selected_idx = self.selected_idx;

        if self.num_widgets() == 0 {
            return false;
        }

        // If we've not selected already just go to end otherwise it's confusing
        if self.selected_idx == -1 && count < 0 {
            for (i, widget) in self.widgets.iter().enumerate().rev() {
                if widget.is_selectable() {
                    self.selected_idx = i as isize;
                    self.first_row =
                        self.get_total_height() as isize - self.display_height.get() as isize;
                    count = 0;
                    break;
                }
            }
        }

        let mut new_selected_idx = self.selected_idx;

        // TODO: Can we refactor ths into one block? Ploughing on for now.
        if count > 0 {
            for _ in 0..count {
                new_selected_idx += 1;
                while new_selected_idx < self.num_widgets() as isize {
                    if let Some(widget) = self.get_widget(new_selected_idx as usize) {
                        if widget.is_selectable() {
                            self.selected_idx = new_selected_idx;
                            break;
                        }
                    }
                    new_selected_idx += 1;
                }

                if wrap_around && new_selected_idx == self.num_widgets() as isize {
                    new_selected_idx = 0;
                    while new_selected_idx < self.num_widgets() as isize {
                        if self
                            .get_widget(new_selected_idx as usize)
                            .unwrap()
                            .is_selectable()
                        {
                            self.selected_idx = new_selected_idx;
                            self.first_row = 0;
                            break;
                        }
                        new_selected_idx += 1;
                    }
                }
            }
        } else {
            for _ in 0..-count {
                new_selected_idx += -1;
                while new_selected_idx >= 0 {
                    if self
                        .get_widget(new_selected_idx as usize)
                        .unwrap()
                        .is_selectable()
                    {
                        self.selected_idx = new_selected_idx;
                        break;
                    }
                    new_selected_idx -= 1;
                }

                if wrap_around && new_selected_idx == -1 {
                    new_selected_idx = self.num_widgets() as isize - 1;
                    debug!("New Selected Index: {}", new_selected_idx);
                    while new_selected_idx >= 0 {
                        if self
                            .get_widget(new_selected_idx as usize)
                            .unwrap()
                            .is_selectable()
                        {
                            self.selected_idx = new_selected_idx;
                            self.first_row = self.get_total_height() as isize
                                - self.display_height.get() as isize;
                            break;
                        }
                        new_selected_idx -= 1;
                    }
                }
            }
        }

        debug!(
            "Selected Rows: {:?}, First Row: {}, Display Height: {}",
            self.get_selected_rows(),
            self.first_row,
            self.display_height.get()
        );
        if let Some((selected_row_from, selected_row_to)) = self.get_selected_rows() {
            if selected_row_to as isize - self.first_row >= self.display_height.get() as isize {
                self.scroll(ScrollType::Single(
                    selected_row_to as isize - self.display_height.get() as isize,
                ));
            } else if selected_row_from < self.first_row as usize {
                self.scroll(ScrollType::Single(
                    selected_row_from as isize - self.first_row,
                ));
            }

            let selected_idx = self.selected_idx;

            if let Some(widget) = self.get_widget_mut(selected_idx as usize) {
                let widget_starting_height = widget.starting_idx;
                if let Some(table) = widget.inner_table_mut() {
                    table.navigate(selected_idx - widget_starting_height as isize + 1);
                }
            }
        }

        debug!(
            "Navigation - Selected Index: {}, Selected Rows: {:?}",
            self.selected_idx,
            self.get_selected_rows(),
        );

        self.selected_idx != old_selected_idx
    }

    pub(crate) fn scroll(&mut self, scroll_type: ScrollType) {
        let shift = match scroll_type {
            ScrollType::Single(lines) => lines,
            ScrollType::HalfPageUp => -(self.display_height.get() as isize / 2),
            ScrollType::HalfPageDown => self.display_height.get() as isize / 2,
            ScrollType::HalfPageLeft => -(self.display_width.get() as isize / 2),
            ScrollType::HalfPageRight => self.display_width.get() as isize / 2,
            ScrollType::FullPageUp => -(self.display_height.get() as isize),
            ScrollType::FullPageDown => self.display_height.get() as isize,
            ScrollType::Top => todo!(),
            ScrollType::Bottom => todo!(),
        };
        let scroll_var_mut = match scroll_type {
            ScrollType::Single(_)
            | ScrollType::HalfPageUp
            | ScrollType::HalfPageDown
            | ScrollType::FullPageUp
            | ScrollType::FullPageDown
            | ScrollType::Top
            | ScrollType::Bottom => &mut self.first_row,
            ScrollType::HalfPageLeft | ScrollType::HalfPageRight => &mut self.first_col,
        };

        *scroll_var_mut += shift;
        *scroll_var_mut = cmp::max(0, *scroll_var_mut);
        match scroll_type {
            ScrollType::Single(_)
            | ScrollType::HalfPageUp
            | ScrollType::HalfPageDown
            | ScrollType::FullPageUp
            | ScrollType::FullPageDown => {
                self.first_row = cmp::min(
                    self.get_total_height() as isize - self.display_height.get() as isize,
                    self.first_row,
                );
            }
            _ => {}
        }

        assert!(self.first_row >= 0);
        assert!(
            self.first_row <= self.get_total_height() as isize - self.display_height.get() as isize
        );
    }

    pub(crate) fn get_selected_rows(&self) -> Option<(usize, usize)> {
        if self.selected_idx == -1 {
            return None;
        }

        let mut current_row = 0;
        for (i, widget) in self.widgets.iter().enumerate() {
            let widget_height = widget.scrollable_height();
            let widget_size = widget.num_widgets();
            debug!(
                "Widget: {}, Starting Index: {}, Widget Height: {}, Selected Idx {}",
                i, widget.starting_idx, widget_height, self.selected_idx,
            );
            // if i == self.selected_idx as usize {
            //     return Some((current_row, current_row + widget_height));
            // }
            if widget.starting_idx <= self.selected_idx as usize
                && (self.selected_idx as usize) < widget.starting_idx + widget_size
            {
                return Some((current_row, current_row + widget_height));
            }
            current_row += widget_height;
        }

        None
    }

    pub(crate) fn input(
        &mut self,
        key: &KeyEvent,
        wrap_vertical: bool,
        allow_horizontal_scroll: bool,
    ) -> Result<bool> {
        match (key.code, key.modifiers) {
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.scroll(ScrollType::FullPageDown);
            }
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.scroll(ScrollType::FullPageUp);
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.scroll(ScrollType::HalfPageDown);
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.scroll(ScrollType::HalfPageUp);
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.scroll(ScrollType::Single(1));
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                self.scroll(ScrollType::Single(-1));
            }
            (KeyCode::Char('g'), KeyModifiers::NONE) => {
                self.first_row = 0;
            }
            (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                self.first_row =
                    self.get_total_height() as isize - self.display_height.get() as isize;
            }
            (KeyCode::Left, KeyModifiers::NONE) if allow_horizontal_scroll => {
                self.scroll(ScrollType::HalfPageLeft);
            }
            (KeyCode::Right, KeyModifiers::NONE) if allow_horizontal_scroll => {
                self.scroll(ScrollType::HalfPageRight);
            }
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                return Ok(self.navigate(-1, wrap_vertical));
            }
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                return Ok(self.navigate(1, wrap_vertical));
            }
            _ => return Ok(false),
        }

        Ok(true)
    }

    /// Renders the widgets in the scrollale area that should be visible.
    fn render_main(&self, area: Rect, buf: &mut Buffer) {
        let area_height = area.height as usize;
        let area_width = area.width as usize;

        self.display_height.set(area_height);
        self.display_width.set(area_width);

        let mut current_height = 0;
        let mut current_row = 0;

        let selected_rows = self.get_selected_rows();

        for widget in &self.widgets {
            let widget_height = widget.scrollable_height();

            let skip_lines = if current_row < self.first_row as usize {
                self.first_row as usize - current_row
            } else {
                0
            };

            if current_row + widget_height <= self.first_row as usize {
                current_row += widget_height;
                continue;
            }

            let selected = if let Some((selected_row_from, selected_row_to)) = selected_rows {
                current_row >= selected_row_from && current_row < selected_row_to
            } else {
                false
            };

            if (current_height == 0 && current_row < self.first_row as usize) || self.first_col > 0
            {
                // Need to render into another buffer and then copy the cells into the main buffer
                // with the offsets specified
                let first_row = if current_height == 0 {
                    self.first_row as u16 - current_row as u16
                } else {
                    0
                };

                let render_area = Rect {
                    x: 0,
                    y: 0,
                    width: self.first_col as u16 + area.width,
                    height: widget_height as u16,
                };
                let mut new_buffer = Buffer::empty(render_area);

                widget
                    .set_selected(selected)
                    .render_ref(render_area, &mut new_buffer);

                for x in 0..area.width {
                    let height = if current_height == 0 {
                        widget_height as u16 - first_row
                    } else {
                        widget_height as u16
                    };
                    for y in 0..height {
                        if area.y + current_height as u16 + y >= area.y + area.height {
                            break;
                        }
                        let cell = &new_buffer[(x + self.first_col as u16, y + first_row)];
                        buf[(area.x + x, area.y + current_height as u16 + y)] = cell.clone();
                    }
                }
            } else {
                // Simple case
                let widget_area = Rect {
                    x: area.x,
                    y: area.y + current_height as u16,
                    width: area.width,
                    height: cmp::min(area_height - current_height, widget_height - skip_lines)
                        as u16,
                };

                widget.set_selected(selected).render_ref(widget_area, buf);
            }

            current_row += widget_height;
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
            self.get_total_height() + 1 - self.display_height.get()
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

    fn num_widgets(&self) -> usize {
        self.widgets.iter().map(|widget| widget.num_widgets()).sum()
    }

    fn get_widget(&self, selected_idx: usize) -> Option<&PatuiWidgetData<'a>> {
        assert!(selected_idx < self.num_widgets());
        self.widgets
            .iter()
            .filter(|widget| widget.starting_idx <= selected_idx)
            .last()
    }

    fn get_widget_mut(&mut self, selected_idx: usize) -> Option<&mut PatuiWidgetData<'a>> {
        assert!(selected_idx < self.num_widgets());
        self.widgets
            .iter_mut()
            .filter(|widget| widget.starting_idx <= selected_idx)
            .last()
    }
}

impl<'a> WidgetRef for ScrollableArea<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let (main_area, scrollbar_area) = if let Some((block, _, _)) = &self.block {
            block.clone().style(self.style).render_ref(area, buf);
            let inner = block.inner(area);
            let inner_main = Rect {
                x: inner.x,
                y: inner.y,
                width: inner.width - 1,
                height: inner.height,
            };
            let scrollbar_rect = Rect {
                x: area.x + area.width - 2,
                y: area.y + 1,
                width: 1,
                height: area.height - 2,
            };
            (inner_main, scrollbar_rect)
        } else {
            let inner_main = Rect {
                x: area.x,
                y: area.y,
                width: area.width - 1,
                height: area.height,
            };
            let scrollbar_rect = Rect {
                x: area.x + area.width - 1,
                y: area.y,
                width: 1,
                height: area.height,
            };
            (inner_main, scrollbar_rect)
        };
        self.render_main(main_area, buf);
        self.render_scrollbar(scrollbar_area, buf);
    }
}

// impl<'a> PatuiWidget for ScrollableArea<'a> {
// fn scrollable_height(&self) -> usize {
//     self.widgets
//         .iter()
//         .map(|widget| widget.scrollable_height())
//         .sum()
// }

// }

#[cfg(test)]
mod tests {
    use crate::tui::widgets::{patui_widget::TestWidget, table::TableHeader, Table, Text};

    use super::*;
    use assertor::*;
    use ratatui::{
        layout::Constraint,
        style::Stylize,
        text::{Line, Text as RatatuiText},
        widgets::{Borders, Padding},
    };
    use tracing_test::traced_test;

    fn get_widget_calls<'a>(scrollable_area: &'a ScrollableArea<'a>) -> Vec<TestWidget<'a>> {
        scrollable_area
            .widgets
            .iter()
            .map(|widget| widget.inner.get_test_inner().unwrap().clone())
            .collect::<Vec<_>>()
    }

    #[traced_test]
    #[test]
    fn test_scrollable_area_simple() {
        let mut scrollable_area = ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(24),
            display_width: Cell::new(24),
            widgets: vec![],
            block: None,
            style: Style::default(),
        };
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            3,
            1,
            RatatuiText::from(vec![
                Line::from("Test1"),
                Line::from("Test2"),
                Line::from("Test3"),
            ]),
        )));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new(6, 1)));
        let rect = Rect::new(0, 0, 10, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        assert_that!(scrollable_area.display_height.get()).is_equal_to(10);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(2);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(1);
        assert_that!(test_widgets[0].calls.borrow()[0]).is_equal_to(Rect::new(0, 0, 9, 3));
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(test_widgets[1].calls.borrow()[0]).is_equal_to(Rect::new(0, 3, 9, 6));

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scrollable_area_with_overlaps() {
        let mut scrollable_area = ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(24),
            display_width: Cell::new(24),
            widgets: vec![],
            block: None,
            style: Style::default(),
        };
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new(3, 1)));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new(6, 1)));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new(7, 1)));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new(3, 1)));
        let rect = Rect::new(0, 0, 10, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(4);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(1);
        assert_that!(test_widgets[0].calls.borrow()[0]).is_equal_to(Rect::new(0, 0, 9, 3));
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(test_widgets[1].calls.borrow()[0]).is_equal_to(Rect::new(0, 3, 9, 6));
        assert_that!(*test_widgets[2].calls.borrow()).has_length(1);
        assert_that!(test_widgets[2].calls.borrow()[0]).is_equal_to(Rect::new(0, 9, 9, 1));
        assert_that!(*test_widgets[3].calls.borrow()).has_length(0);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scrollable_area_with_different_first_line() {
        let mut scrollable_area = ScrollableArea {
            first_row: 5,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(24),
            display_width: Cell::new(24),
            widgets: vec![],
            block: None,
            style: Style::default(),
        };
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new(3, 1)));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new(6, 1)));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new(7, 1)));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new(3, 1)));
        let rect = Rect::new(0, 0, 10, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(4);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(0);
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(test_widgets[1].calls.borrow()[0]).is_equal_to(Rect::new(0, 0, 9, 6));
        assert_that!(*test_widgets[2].calls.borrow()).has_length(1);
        assert_that!(test_widgets[2].calls.borrow()[0]).is_equal_to(Rect::new(0, 4, 9, 6));
        assert_that!(*test_widgets[3].calls.borrow()).has_length(0);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_single_line_scroll() {
        let widgets = (0..12)
            .map(|i| PatuiWidgetData {
                inner: PatuiWidget::new_test(TestWidget::new(1, 1)),
                starting_idx: i,
            })
            .collect::<Vec<_>>();
        let mut scrollable_area = ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(5),
            display_width: Cell::new(24),
            widgets,
            block: None,
            style: Style::default(),
        };
        let rect = Rect::new(0, 0, 5, 5);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);
        scrollable_area.scroll(ScrollType::Single(5));

        assert_that!(scrollable_area.first_row).is_equal_to(5);

        scrollable_area.scroll(ScrollType::Single(5));

        assert_that!(scrollable_area.first_row).is_equal_to(7);

        scrollable_area.render_ref(rect, &mut buffer);
        insta::assert_debug_snapshot!(buffer);

        scrollable_area.scroll(ScrollType::Single(5));

        assert_that!(scrollable_area.first_row).is_equal_to(7);

        scrollable_area.scroll(ScrollType::Single(-5));

        assert_that!(scrollable_area.first_row).is_equal_to(2);

        scrollable_area.scroll(ScrollType::Single(-5));

        assert_that!(scrollable_area.first_row).is_equal_to(0);

        scrollable_area.scroll(ScrollType::Single(-5));

        assert_that!(scrollable_area.first_row).is_equal_to(0);
    }

    #[traced_test]
    #[test]
    fn test_page_scroll() {
        let widgets = (0..12)
            .map(|i| PatuiWidgetData {
                inner: PatuiWidget::new_test(TestWidget::new(5, 1)),
                starting_idx: i,
            })
            .collect::<Vec<_>>();
        let mut scrollable_area = ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(20),
            display_width: Cell::new(24),
            widgets,
            block: None,
            style: Style::default(),
        };
        let rect = Rect::new(0, 0, 20, 20);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);
        scrollable_area.scroll(ScrollType::HalfPageUp);

        assert_that!(scrollable_area.first_row).is_equal_to(0);

        scrollable_area.scroll(ScrollType::FullPageUp);

        assert_that!(scrollable_area.first_row).is_equal_to(0);

        scrollable_area.scroll(ScrollType::FullPageDown);

        assert_that!(scrollable_area.first_row).is_equal_to(20);

        scrollable_area.scroll(ScrollType::HalfPageDown);

        assert_that!(scrollable_area.first_row).is_equal_to(30);

        scrollable_area.scroll(ScrollType::FullPageDown);

        assert_that!(scrollable_area.first_row).is_equal_to(40);

        scrollable_area.scroll(ScrollType::Single(-5));
        scrollable_area.scroll(ScrollType::HalfPageUp);
        scrollable_area.render_ref(rect, &mut buffer);

        assert_that!(scrollable_area.first_row).is_equal_to(25);
        insta::assert_debug_snapshot!(buffer);

        scrollable_area.scroll(ScrollType::FullPageUp);

        assert_that!(scrollable_area.first_row).is_equal_to(5);

        scrollable_area.scroll(ScrollType::FullPageUp);

        assert_that!(scrollable_area.first_row).is_equal_to(0);

        scrollable_area.render_ref(rect, &mut buffer);
    }

    #[traced_test]
    #[test]
    fn test_simple_render_block() {
        let mut scrollable_area = ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(5),
            display_width: Cell::new(24),
            widgets: vec![],
            block: Some((
                Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::symmetric(2, 1)),
                Borders::ALL,
                Padding::symmetric(2, 1),
            )),
            style: Style::default(),
        };
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test1"),
                Line::from("Test2"),
                Line::from("Test3"),
                Line::from("Test4"),
            ]),
        )));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test5"),
                Line::from("Test6"),
                Line::from("Test7"),
                Line::from("Test8"),
            ]),
        )));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test9"),
                Line::from("Test10"),
                Line::from("Test11"),
                Line::from("Test12"),
            ]),
        )));
        let rect = Rect::new(2, 2, 20, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(3);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(1);
        assert_that!(test_widgets[0].calls.borrow()[0]).is_equal_to(Rect::new(5, 4, 13, 4));
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(test_widgets[1].calls.borrow()[0]).is_equal_to(Rect::new(5, 8, 13, 2));
        assert_that!(*test_widgets[2].calls.borrow()).has_length(0);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_simple_render_block_with_offset() {
        let mut scrollable_area = ScrollableArea {
            first_row: 4,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(5),
            display_width: Cell::new(24),
            widgets: vec![],
            block: Some((
                Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::symmetric(2, 1)),
                Borders::ALL,
                Padding::symmetric(2, 1),
            )),
            style: Style::default(),
        };
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test1".red()),
                Line::from("Test2".blue()),
                Line::from("Test3".green()),
                Line::from("Test4".yellow()),
            ]),
        )));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test5".red()),
                Line::from("Test6".blue()),
                Line::from("Test7".green()),
                Line::from("Test8".yellow()),
            ]),
        )));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test9".red()),
                Line::from("Test10".blue()),
                Line::from("Test11".green()),
                Line::from("Test12".yellow()),
            ]),
        )));
        let rect = Rect::new(2, 2, 20, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(3);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(0);
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(test_widgets[1].calls.borrow()[0]).is_equal_to(Rect::new(5, 4, 13, 4));
        assert_that!(*test_widgets[2].calls.borrow()).has_length(1);
        assert_that!(test_widgets[2].calls.borrow()[0]).is_equal_to(Rect::new(5, 8, 13, 2));

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_simple_render_block_with_offset_into_widget() {
        let mut scrollable_area = ScrollableArea {
            first_row: 6,
            first_col: 2,
            selected_idx: -1,
            display_height: Cell::new(5),
            display_width: Cell::new(24),
            widgets: vec![],
            block: Some((
                Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::symmetric(2, 1)),
                Borders::ALL,
                Padding::symmetric(2, 1),
            )),
            style: Style::default(),
        };
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test1".red()),
                Line::from("Test2".blue()),
                Line::from("Test3".green()),
                Line::from("Test4".yellow()),
            ]),
        )));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test5".red()),
                Line::from("Test6".blue()),
                Line::from("Test7".green()),
                Line::from("Test8".yellow()),
            ]),
        )));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test9".red()),
                Line::from("Test10".blue()),
                Line::from("Test11".green()),
                Line::from("Test12".yellow()),
            ]),
        )));
        let rect = Rect::new(2, 2, 20, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(3);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(0);
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(*test_widgets[2].calls.borrow()).has_length(1);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_simple_render_block_with_offset_into_top_and_bottom_widget() {
        let mut scrollable_area = ScrollableArea {
            first_row: 3,
            first_col: 2,
            selected_idx: -1,
            display_height: Cell::new(5),
            display_width: Cell::new(24),
            widgets: vec![],
            block: Some((
                Block::default()
                    .borders(Borders::ALL)
                    .padding(Padding::symmetric(2, 1)),
                Borders::ALL,
                Padding::symmetric(2, 1),
            )),
            style: Style::default(),
        };
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test1".red()),
                Line::from("Test2".blue()),
                Line::from("Test3".green()),
                Line::from("Test4".yellow()),
            ]),
        )));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test5".red()),
                Line::from("Test6".blue()),
                Line::from("Test7".green()),
                Line::from("Test8".yellow()),
            ]),
        )));
        scrollable_area.add_widget(PatuiWidget::new_test(TestWidget::new_with_text(
            4,
            1,
            RatatuiText::from(vec![
                Line::from("Test9".red()),
                Line::from("Test10".blue()),
                Line::from("Test11".green()),
                Line::from("Test12".yellow()),
            ]),
        )));
        let rect = Rect::new(2, 2, 20, 10);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        let test_widgets = get_widget_calls(&scrollable_area);
        assert_that!(test_widgets).has_length(3);
        assert_that!(*test_widgets[0].calls.borrow()).has_length(1);
        assert_that!(*test_widgets[1].calls.borrow()).has_length(1);
        assert_that!(*test_widgets[2].calls.borrow()).has_length(1);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_render_text_widgets() {
        let widgets = (0..10)
            .map(|i| PatuiWidgetData {
                inner: PatuiWidget::new_text(
                    RatatuiText::from(vec![
                        Line::from(format!("Test{}", 6 * i + 1)),
                        Line::from(format!("Test{}", 6 * i + 2)),
                        Line::from(format!("Test{}", 6 * i + 3)),
                        Line::from(format!("Test{}", 6 * i + 4)),
                        Line::from(format!("Test{}", 6 * i + 5)),
                        Line::from(format!("Test{}", 6 * i + 6)),
                    ])
                    .into(),
                ),
                starting_idx: i,
            })
            .collect::<Vec<_>>();
        let scrollable_area = ScrollableArea {
            first_row: 2,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(5),
            display_width: Cell::new(24),
            widgets,
            block: None,
            style: Style::default(),
        };
        let rect = Rect::new(2, 2, 60, 30);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_first_navigation_text_down() {
        let widgets = (0..10)
            .map(|i| PatuiWidgetData {
                inner: PatuiWidget::new_text(Text::new_with_text(
                    RatatuiText::from(vec![
                        Line::from(format!("Test{}", 6 * i + 1)),
                        Line::from(format!("Test{}", 6 * i + 2)),
                        Line::from(format!("Test{}", 6 * i + 3)),
                        Line::from(format!("Test{}", 6 * i + 4)),
                        Line::from(format!("Test{}", 6 * i + 5)),
                        Line::from(format!("Test{}", 6 * i + 6)),
                    ]),
                    i % 2 == 1,
                )),
                starting_idx: i,
            })
            .collect::<Vec<_>>();
        let mut scrollable_area = ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(30),
            display_width: Cell::new(60),
            widgets,
            block: None,
            style: Style::default(),
        };

        let ret = scrollable_area.navigate(1, false);

        assert_that!(ret).is_equal_to(true);
        assert_that!(scrollable_area.selected_idx).is_equal_to(1);
        assert_that!(scrollable_area.get_selected_rows()).is_equal_to(Some((6, 12)));
        assert_that!(scrollable_area.first_row).is_equal_to(0);

        let rect = Rect::new(0, 0, 60, 30);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_first_navigation_text_up() {
        let widgets = (0..11)
            .map(|i| PatuiWidgetData {
                inner: PatuiWidget::new_text(Text::new_with_text(
                    RatatuiText::from(vec![
                        Line::from(format!("Test{}", 6 * i + 1)),
                        Line::from(format!("Test{}", 6 * i + 2)),
                        Line::from(format!("Test{}", 6 * i + 3)),
                        Line::from(format!("Test{}", 6 * i + 4)),
                        Line::from(format!("Test{}", 6 * i + 5)),
                        Line::from(format!("Test{}", 6 * i + 6)),
                    ]),
                    i % 2 == 1,
                )),
                starting_idx: i,
            })
            .collect::<Vec<_>>();
        let mut scrollable_area = ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(30),
            display_width: Cell::new(60),
            widgets,
            block: None,
            style: Style::default(),
        };

        let ret = scrollable_area.navigate(-1, false);

        assert_that!(ret).is_equal_to(true);
        assert_that!(scrollable_area.selected_idx).is_equal_to(9);
        assert_that!(scrollable_area.get_selected_rows()).is_equal_to(Some((54, 60)));
        assert_that!(scrollable_area.first_row).is_equal_to(36);

        let rect = Rect::new(0, 0, 60, 30);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_simple_navigation() {
        let widgets = (0..11)
            .map(|i| PatuiWidgetData {
                inner: PatuiWidget::new_text(Text::new_with_text(
                    RatatuiText::from(vec![
                        Line::from(format!("Test{}", 6 * i + 1)),
                        Line::from(format!("Test{}", 6 * i + 2)),
                        Line::from(format!("Test{}", 6 * i + 3)),
                        Line::from(format!("Test{}", 6 * i + 4)),
                        Line::from(format!("Test{}", 6 * i + 5)),
                        Line::from(format!("Test{}", 6 * i + 6)),
                    ]),
                    i % 2 == 1,
                )),
                starting_idx: i,
            })
            .collect::<Vec<_>>();
        let mut scrollable_area = ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: 0,
            display_height: Cell::new(30),
            display_width: Cell::new(60),
            widgets,
            block: None,
            style: Style::default(),
        };

        let ret = scrollable_area.navigate(2, false);

        assert_that!(ret).is_equal_to(true);
        assert_that!(scrollable_area.selected_idx).is_equal_to(3);
        assert_that!(scrollable_area.get_selected_rows()).is_equal_to(Some((18, 24)));
        assert_that!(scrollable_area.first_row).is_equal_to(0);

        let rect = Rect::new(0, 0, 60, 30);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_simple_navigation_with_wrapping_forward() {
        let widgets = (0..11)
            .map(|i| PatuiWidgetData {
                inner: PatuiWidget::new_text(Text::new_with_text(
                    RatatuiText::from(vec![
                        Line::from(format!("Test{}", 6 * i + 1)),
                        Line::from(format!("Test{}", 6 * i + 2)),
                        Line::from(format!("Test{}", 6 * i + 3)),
                        Line::from(format!("Test{}", 6 * i + 4)),
                        Line::from(format!("Test{}", 6 * i + 5)),
                        Line::from(format!("Test{}", 6 * i + 6)),
                    ]),
                    i % 2 == 1,
                )),
                starting_idx: i,
            })
            .collect::<Vec<_>>();
        let mut scrollable_area = ScrollableArea {
            first_row: 36,
            first_col: 0,
            selected_idx: 9,
            display_height: Cell::new(30),
            display_width: Cell::new(60),
            widgets,
            block: None,
            style: Style::default(),
        };

        let ret = scrollable_area.navigate(1, true);

        assert_that!(ret).is_equal_to(true);
        assert_that!(scrollable_area.selected_idx).is_equal_to(1);
        assert_that!(scrollable_area.get_selected_rows()).is_equal_to(Some((6, 12)));
        assert_that!(scrollable_area.first_row).is_equal_to(0);

        let rect = Rect::new(0, 0, 60, 30);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_simple_navigation_with_wrapping_backward() {
        let widgets = (0..11)
            .map(|i| PatuiWidgetData {
                inner: PatuiWidget::new_text(Text::new_with_text(
                    RatatuiText::from(vec![
                        Line::from(format!("Test{}", 6 * i + 1)),
                        Line::from(format!("Test{}", 6 * i + 2)),
                        Line::from(format!("Test{}", 6 * i + 3)),
                        Line::from(format!("Test{}", 6 * i + 4)),
                        Line::from(format!("Test{}", 6 * i + 5)),
                        Line::from(format!("Test{}", 6 * i + 6)),
                    ]),
                    i % 2 == 1,
                )),
                starting_idx: i,
            })
            .collect::<Vec<_>>();
        let mut scrollable_area = ScrollableArea {
            first_row: 6,
            first_col: 0,
            selected_idx: 1,
            display_height: Cell::new(30),
            display_width: Cell::new(60),
            widgets,
            block: None,
            style: Style::default(),
        };

        let ret = scrollable_area.navigate(-1, true);

        assert_that!(ret).is_equal_to(true);
        assert_that!(scrollable_area.selected_idx).is_equal_to(9);
        assert_that!(scrollable_area.get_selected_rows()).is_equal_to(Some((54, 60)));
        assert_that!(scrollable_area.first_row).is_equal_to(36);

        let rect = Rect::new(0, 0, 60, 30);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    // TODO: Scroll leaves selection visible?

    // TODO: Test with widget bigger than display with selection and navigation

    fn scrollable_with_table_widget<'a>() -> ScrollableArea<'a> {
        let mut widgets = (0..4)
            .map(|i| PatuiWidgetData {
                inner: PatuiWidget::new_text(Text::new_with_text(
                    RatatuiText::from(vec![
                        Line::from(format!("Test{}", 5 * i + 1)),
                        Line::from(format!("Test{}", 5 * i + 2)),
                        Line::from(format!("Test{}", 5 * i + 3)),
                        Line::from(format!("Test{}", 5 * i + 4)),
                        Line::from(format!("Test{}", 5 * i + 5)),
                    ]),
                    i % 2 == 1,
                )),
                starting_idx: i,
            })
            .collect::<Vec<_>>();
        widgets.push(PatuiWidgetData {
            inner: PatuiWidget::new_table(Table::new_with_elements(
                vec![
                    vec![
                        RatatuiText::from("Test1_1"),
                        RatatuiText::from("Test1_2"),
                        RatatuiText::from("Test1_3"),
                    ],
                    vec![
                        RatatuiText::from("Test2_1"),
                        RatatuiText::from("Test2_2"),
                        RatatuiText::from("Test2_3"),
                    ],
                    vec![
                        RatatuiText::from("Test3_1"),
                        RatatuiText::from("Test3_2"),
                        RatatuiText::from("Test3_3"),
                    ],
                ],
                vec![
                    TableHeader::new(RatatuiText::from("Header1"), 0, Constraint::Min(10)),
                    TableHeader::new(RatatuiText::from("Header2"), 1, Constraint::Min(10)),
                ],
                vec![
                    TableHeader::new(RatatuiText::from("Header1"), 0, Constraint::Min(10)),
                    TableHeader::new(RatatuiText::from("Header2"), 1, Constraint::Min(10)),
                    TableHeader::new(RatatuiText::from("Header3"), 2, Constraint::Min(10)),
                ],
                None,
                false,
            )),
            starting_idx: 4,
        });
        widgets.push(PatuiWidgetData {
            inner: PatuiWidget::new_text(Text::new_with_text(
                RatatuiText::from(vec![
                    Line::from("Test21"),
                    Line::from("Test22"),
                    Line::from("Test23"),
                    Line::from("Test24"),
                    Line::from("Test25"),
                ]),
                true,
            )),
            starting_idx: 7,
        });

        ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(24),
            display_width: Cell::new(60),
            widgets,
            block: None,
            style: Style::default(),
        }
    }

    #[traced_test]
    #[test]
    fn test_scrollable_with_table_widget_1() {
        let scrollable_area = scrollable_with_table_widget();

        let rect = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scrollable_with_table_widget_2() {
        let mut scrollable_area = scrollable_with_table_widget();

        let rect = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.navigate(1, false);
        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scrollable_with_table_widget_3() {
        let mut scrollable_area = scrollable_with_table_widget();

        let rect = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.navigate(2, false);
        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scrollable_with_table_widget_4() {
        let mut scrollable_area = scrollable_with_table_widget();

        let rect = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.navigate(3, false);
        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scrollable_with_table_widget_5() {
        let mut scrollable_area = scrollable_with_table_widget();

        let rect = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.navigate(4, false);
        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scrollable_with_table_widget_6() {
        let mut scrollable_area = scrollable_with_table_widget();

        let rect = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.navigate(5, false);
        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scrollable_with_table_widget_7() {
        let mut scrollable_area = scrollable_with_table_widget();

        let rect = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.navigate(6, false);
        scrollable_area.render_ref(rect, &mut buffer);

        insta::assert_debug_snapshot!(buffer);
    }

    #[traced_test]
    #[test]
    fn test_scrollable_with_table_select() {
        let mut scrollable_area = ScrollableArea {
            first_row: 0,
            first_col: 0,
            selected_idx: -1,
            display_height: Cell::new(24),
            display_width: Cell::new(60),
            widgets: vec![PatuiWidgetData {
                inner: PatuiWidget::new_table(Table::new_with_elements(
                    vec![
                        vec![
                            RatatuiText::from("Test1_1"),
                            RatatuiText::from("Test1_2"),
                            RatatuiText::from("Test1_3"),
                        ],
                        vec![
                            RatatuiText::from("Test2_1"),
                            RatatuiText::from("Test2_2"),
                            RatatuiText::from("Test2_3"),
                        ],
                        vec![
                            RatatuiText::from("Test3_1"),
                            RatatuiText::from("Test3_2"),
                            RatatuiText::from("Test3_3"),
                        ],
                    ],
                    vec![
                        TableHeader::new(RatatuiText::from("Header1"), 0, Constraint::Min(10)),
                        TableHeader::new(RatatuiText::from("Header2"), 1, Constraint::Min(10)),
                    ],
                    vec![
                        TableHeader::new(RatatuiText::from("Header1"), 0, Constraint::Min(10)),
                        TableHeader::new(RatatuiText::from("Header2"), 1, Constraint::Min(10)),
                        TableHeader::new(RatatuiText::from("Header3"), 2, Constraint::Min(10)),
                    ],
                    None,
                    false,
                )),
                starting_idx: 4,
            }],
            block: None,
            style: Style::default(),
        };

        let rect = Rect::new(0, 0, 80, 24);
        let mut buffer = Buffer::empty(rect);

        scrollable_area.navigate(1, false);
        scrollable_area.render_ref(rect, &mut buffer);

        assert_that!(scrollable_area.selected_idx).is_equal_to(0);

        insta::assert_debug_snapshot!(buffer);
    }
}
