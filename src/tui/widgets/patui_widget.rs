use std::fmt::Debug;

use ratatui::{buffer::Buffer, layout::Rect, widgets::WidgetRef};

use super::{text::Text, ScrollableArea, Table};

#[cfg(test)]
pub(crate) use tests::TestWidget;

#[derive(Debug)]
pub(crate) enum ScrollType {
    Single(isize),
    HalfPageUp,
    HalfPageDown,
    HalfPageLeft,
    HalfPageRight,
    FullPageUp,
    FullPageDown,
}

#[derive(Clone, Debug)]
pub(crate) struct PatuiWidget<'a> {
    inner: PatuiWidgetInner<'a>,
}

impl<'a> PatuiWidget<'a> {
    pub(crate) fn new_text(text: Text<'a>) -> PatuiWidget<'a> {
        Self {
            inner: PatuiWidgetInner::Text(text),
        }
    }

    pub(crate) fn new_scrollable_area(scrollable_area: ScrollableArea<'a>) -> PatuiWidget<'a> {
        Self {
            inner: PatuiWidgetInner::ScrollableArea(scrollable_area),
        }
    }

    pub(crate) fn new_table(table: Table<'a>) -> PatuiWidget<'a> {
        Self {
            inner: PatuiWidgetInner::Table(table),
        }
    }

    pub(crate) fn scrollable_height(&self) -> usize {
        match &self.inner {
            PatuiWidgetInner::Text(text) => text.height(),
            // PatuiWidget::Button(button) => button.scrollable_height(),
            PatuiWidgetInner::ScrollableArea(_) => todo!(),
            PatuiWidgetInner::Table(table) => table.scrollable_height(),
            #[cfg(test)]
            PatuiWidgetInner::TestWidget(test_widget) => test_widget.height,
        }
    }

    pub(crate) fn num_widgets(&self) -> usize {
        match &self.inner {
            PatuiWidgetInner::Text(_) => 1,
            // PatuiWidget::Button(button) => button.num_widgets(),
            PatuiWidgetInner::ScrollableArea(_) => todo!(),
            PatuiWidgetInner::Table(table) => table.num_elements(),
            #[cfg(test)]
            PatuiWidgetInner::TestWidget(test_widget) => test_widget.num_widgets,
        }
    }

    pub(crate) fn is_selectable(&self) -> bool {
        match &self.inner {
            PatuiWidgetInner::Text(text) => text.is_selectable(),
            PatuiWidgetInner::ScrollableArea(_) => todo!(),
            PatuiWidgetInner::Table(_) => true,
            #[cfg(test)]
            PatuiWidgetInner::TestWidget(_) => false,
        }
    }

    pub(crate) fn set_selected(&self, selected: bool) -> &Self {
        match &self.inner {
            PatuiWidgetInner::Text(text) => {
                text.set_selected(selected);
            }
            PatuiWidgetInner::ScrollableArea(_) => todo!(),
            PatuiWidgetInner::Table(_) => {
                // table.set_selected(selected);
            }
            #[cfg(test)]
            PatuiWidgetInner::TestWidget(_) => {}
        }

        self
    }

    pub(crate) fn inner_scrollable(&self) -> &ScrollableArea<'a> {
        match &self.inner {
            PatuiWidgetInner::ScrollableArea(scrollable_area) => scrollable_area,
            _ => panic!("Cannot get reference to non-scrollable area"),
        }
    }

    pub(crate) fn inner_scrollable_mut(&mut self) -> Option<&mut ScrollableArea<'a>> {
        match &mut self.inner {
            PatuiWidgetInner::ScrollableArea(scrollable_area) => Some(scrollable_area),
            _ => None,
        }
    }

    pub(crate) fn inner_table_mut(&mut self) -> Option<&mut Table<'a>> {
        match &mut self.inner {
            PatuiWidgetInner::Table(table) => Some(table),
            _ => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn new_test(test: TestWidget<'a>) -> PatuiWidget<'a> {
        Self {
            inner: PatuiWidgetInner::TestWidget(test),
        }
    }

    #[cfg(test)]
    pub(crate) fn get_test_inner(&self) -> Option<&TestWidget> {
        match &self.inner {
            PatuiWidgetInner::TestWidget(test_widget) => Some(test_widget),
            _ => None,
        }
    }
}

impl<'a> WidgetRef for PatuiWidget<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        match &self.inner {
            PatuiWidgetInner::Text(text) => text.render_ref(area, buf),
            // PatuiWidgetInner::Button(button) => button.render_ref(area, buf),
            PatuiWidgetInner::ScrollableArea(scrollable_area) => {
                scrollable_area.render_ref(area, buf)
            }
            PatuiWidgetInner::Table(table) => table.render_ref(area, buf),
            #[cfg(test)]
            PatuiWidgetInner::TestWidget(test_widget) => test_widget.render_ref(area, buf),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum PatuiWidgetInner<'a> {
    Text(Text<'a>),
    // Button(Button),
    ScrollableArea(ScrollableArea<'a>),
    Table(Table<'a>),
    #[cfg(test)]
    TestWidget(TestWidget<'a>),
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use ratatui::{buffer::Buffer, layout::Rect, text::Text, widgets::WidgetRef};

    #[derive(Clone, Debug)]
    pub(crate) struct TestWidget<'a> {
        pub height: usize,
        pub num_widgets: usize,
        pub calls: RefCell<Vec<Rect>>,
        pub text: Text<'a>,
    }

    impl<'a> TestWidget<'a> {
        pub(crate) fn new(height: usize, num_widgets: usize) -> Self {
            Self {
                height,
                num_widgets,
                calls: RefCell::new(vec![]),
                text: Text::from(""),
            }
        }

        pub(crate) fn new_with_text(
            height: usize,
            num_widgets: usize,
            text: Text<'a>,
        ) -> TestWidget<'a> {
            Self {
                height,
                num_widgets,
                calls: RefCell::new(vec![]),
                text,
            }
        }
    }

    impl<'a> WidgetRef for TestWidget<'a> {
        fn render_ref(&self, area: Rect, buf: &mut Buffer) {
            self.calls.borrow_mut().push(area);
            self.text.render_ref(area, buf);
        }
    }
}
