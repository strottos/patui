use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

use crate::{
    tui::{
        app::{Action, Mode},
        error::Error,
    },
    types::PatuiTest,
};

use super::{error::ErrorComponent, tests::TestComponent, Component};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MainMode {
    Test,
}

#[derive(Debug)]
pub struct MainComponent<'a> {
    main_mode: MainMode,

    error_component: ErrorComponent,
    test_component: TestComponent<'a>,
}

impl<'a> MainComponent<'a> {
    pub fn new() -> Self {
        let error_component = ErrorComponent::new();
        let test_component = TestComponent::new();

        Self {
            main_mode: MainMode::Test,

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

    pub fn change_mode(&mut self, mode: &Mode) {
        match mode {
            Mode::Test(test_mode) => {
                self.main_mode = MainMode::Test;
                self.test_component.set_select_mode(test_mode.clone());
                self.test_component.set_popup_mode(test_mode.clone());
            }
        }
    }
}

impl<'a> Component for MainComponent<'a> {
    fn render(&self, f: &mut Frame, rect: Rect) {
        self.test_component.render(f, rect);

        if self.error_component.has_error() {
            self.error_component.render(f, rect);
        }
    }

    fn input(&mut self, key: KeyEvent) -> Result<Vec<Action>> {
        self.test_component.input(key)
    }

    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        self.test_component.update(action)
    }
}
