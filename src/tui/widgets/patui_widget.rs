use std::fmt::Debug;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Block, WidgetRef},
};

use super::{ScrollableArea, Text};

#[cfg(test)]
pub(crate) use tests::TestWidget;

#[derive(Debug)]
pub(crate) enum PatuiWidget<'a> {
    Text(Text<'a>),
    // Button(Button),
    ScrollableArea(ScrollableArea<'a>),
    #[cfg(test)]
    TestWidget(TestWidget),
}

impl<'a> WidgetRef for PatuiWidget<'a> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        match self {
            PatuiWidget::Text(text) => text.render_ref(area, buf),
            // PatuiWidget::Button(button) => button.render_ref(area, buf),
            PatuiWidget::ScrollableArea(scrollable_area) => scrollable_area.render_ref(area, buf),
            #[cfg(test)]
            PatuiWidget::TestWidget(test_widget) => test_widget.render_ref(area, buf),
        }
    }
}

impl<'a> PatuiWidget<'a> {
    pub(crate) fn scrollable_height(&self) -> usize {
        match self {
            PatuiWidget::Text(text) => text.height(),
            // PatuiWidget::Button(button) => button.scrollable_height(),
            PatuiWidget::ScrollableArea(_) => todo!(),
            #[cfg(test)]
            PatuiWidget::TestWidget(test_widget) => test_widget.height,
        }
    }

    pub(crate) fn num_widgets(&self) -> usize {
        match self {
            PatuiWidget::Text(_) => 1,
            // PatuiWidget::Button(button) => button.num_widgets(),
            PatuiWidget::ScrollableArea(_) => todo!(),
            #[cfg(test)]
            PatuiWidget::TestWidget(test_widget) => test_widget.num_widgets,
        }
    }

    pub(crate) fn set_render_from_line(&self, line: usize) -> &Self {
        match self {
            PatuiWidget::Text(text) => {
                text.set_render_from_line(line);
            }
            // PatuiWidget::Button(button) => button.set_render_from_line(line),
            PatuiWidget::ScrollableArea(scrollable_area) => todo!(),
            #[cfg(test)]
            PatuiWidget::TestWidget(test_widget) => {
                test_widget.set_render_from_line(line);
            }
        }
        self
    }

    pub(crate) fn add_scrollable_widget(&mut self, widget: PatuiWidget<'a>) -> &mut Self {
        match self {
            PatuiWidget::ScrollableArea(scrollable_area) => scrollable_area.add_widget(widget),
            _ => panic!("Cannot add widget to non-scrollable area"),
        };
        self
    }

    pub(crate) fn set_scrollable_widgets(&mut self, widgets: Vec<PatuiWidget<'a>>) -> &mut Self {
        match self {
            PatuiWidget::ScrollableArea(scrollable_area) => scrollable_area.set_widgets(widgets),
            _ => panic!("Cannot add widget to non-scrollable area"),
        };
        self
    }

    pub(crate) fn add_scrollable_block(&mut self, block: Block<'a>) -> &mut Self {
        match self {
            PatuiWidget::ScrollableArea(scrollable_area) => scrollable_area.add_block(block),
            _ => panic!("Cannot add block to non-scrollable area"),
        };
        self
    }

    #[cfg(test)]
    pub(crate) fn get_test_inner(&self) -> Option<&TestWidget> {
        match self {
            PatuiWidget::TestWidget(test_widget) => Some(test_widget),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};

    use ratatui::{buffer::Buffer, layout::Rect, widgets::WidgetRef};

    #[derive(Clone, Debug)]
    pub(crate) struct TestWidget {
        pub height: usize,
        pub num_widgets: usize,
        pub render_from_line: Cell<usize>,
        pub calls: RefCell<Vec<Rect>>,
    }

    impl TestWidget {
        pub(crate) fn new(height: usize, num_widgets: usize) -> Self {
            Self {
                height,
                num_widgets,
                render_from_line: Cell::new(0),
                calls: RefCell::new(vec![]),
            }
        }

        pub(crate) fn set_render_from_line(&self, line: usize) -> &Self {
            self.render_from_line.set(line);
            self
        }
    }

    impl WidgetRef for TestWidget {
        fn render_ref(&self, area: Rect, _buf: &mut Buffer) {
            self.calls.borrow_mut().push(area);
        }
    }
}
