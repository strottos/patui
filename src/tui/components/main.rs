use color_eyre::Result;
use ratatui::{layout::Rect, Frame};

use crate::{
    tui::{app::Action, error::Error},
    types::PatuiTest,
};

use super::{error::ErrorComponent, tests::TestComponent, Component};

#[derive(Debug)]
pub struct MainComponent {
    error_component: ErrorComponent,
    test_component: TestComponent,
}

impl MainComponent {
    pub fn new() -> Self {
        let error_component = ErrorComponent::new();
        let test_component = TestComponent::new();

        Self {
            error_component,
            test_component,
        }
    }

    pub fn add_error(&mut self, error: Error) {
        self.error_component.add_error(error)
    }

    pub fn update_tests(&mut self, tests: Vec<PatuiTest>) {
        self.test_component.update_tests(tests);
    }
}

impl Component for MainComponent {
    fn render(&mut self, f: &mut Frame, rect: Rect) {
        self.test_component.render(f, rect);

        if self.error_component.has_error() {
            self.error_component.render(f, rect);
        }
    }

    fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        self.test_component.update(action)
    }
}
