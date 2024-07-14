use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::{
    tui::{
        app::{Action, Mode},
        error::Error,
    },
    types::PatuiTest,
};

use super::{
    error::ErrorComponent,
    tests::{TestComponent, TestDetailComponent},
    Component,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MainMode {
    Test,
    TestDetail(i64),
}

#[derive(Debug)]
pub struct MainComponent<'a> {
    main_mode: MainMode,

    error_component: ErrorComponent,
    test_component: TestComponent<'a>,
    test_detail_component: TestDetailComponent,
}

impl<'a> MainComponent<'a> {
    pub fn new() -> Self {
        let error_component = ErrorComponent::new();
        let test_component = TestComponent::new();
        let test_detail_component = TestDetailComponent::new();

        Self {
            main_mode: MainMode::Test,

            error_component,
            test_component,
            test_detail_component,
        }
    }

    pub fn add_error(&mut self, error: Error) {
        self.error_component.add_error(error)
    }

    pub fn update_tests(&mut self, tests: Vec<PatuiTest>) {
        self.test_component.update_tests(tests);
    }

    pub fn update_test_detail(&mut self, test: PatuiTest) {
        self.test_detail_component.update_test_detail(test);
    }

    pub fn change_mode(&mut self, mode: &Mode) {
        match mode {
            Mode::Test(test_mode) => {
                self.main_mode = MainMode::Test;
                self.test_component.set_select_mode(test_mode.clone());
                self.test_component.set_popup_mode(test_mode.clone());
            }
            Mode::TestDetail(test_mode, id) => {
                self.main_mode = MainMode::TestDetail(*id);
                self.test_component.set_select_mode(test_mode.clone());
                self.test_component.set_popup_mode(test_mode.clone());
            }
        }
    }
}

impl<'a> Component for MainComponent<'a> {
    fn render(&self, f: &mut Frame, rect: Rect) {
        if let MainMode::TestDetail(_) = self.main_mode {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rect);
            self.test_component.render(f, chunks[0]);
            self.test_detail_component.render(f, chunks[1]);
        } else {
            self.test_component.render(f, rect);
        }

        if self.error_component.has_error() {
            self.error_component.render(f, rect);
        }
    }

    fn input(&mut self, key: &KeyEvent) -> Result<Vec<Action>> {
        if self.error_component.has_error() {
            self.error_component.input(key)
        } else {
            self.test_component.input(key)
        }
    }

    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        if let Action::ChangeMode(Mode::TestDetail(_, _)) = action {
            ret.extend(self.test_detail_component.update(action)?);
        }

        ret.extend(self.test_component.update(action)?);

        Ok(ret)
    }
}
