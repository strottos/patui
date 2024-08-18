use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::{
    tui::app::{Action, BreadcrumbDirection, MainMode},
    types::PatuiTest,
};

use super::{
    tests::{TestComponent, TestDetailComponent},
    Component, HelpItem,
};

#[derive(Debug)]
pub(crate) struct Middle {
    test_component: TestComponent,
    test_detail_component: TestDetailComponent,
}

impl Middle {
    pub(crate) fn new() -> Self {
        let test_component = TestComponent::new();
        let test_detail_component = TestDetailComponent::new();

        Self {
            test_component,
            test_detail_component,
        }
    }

    pub(crate) fn update_tests(&mut self, tests: Vec<PatuiTest>) {
        self.test_component.update_tests(tests);
    }

    pub(crate) fn update_test_detail(&mut self, test: PatuiTest) {
        self.test_detail_component.update_test_detail(test);
    }

    pub(crate) fn render(&self, f: &mut Frame, rect: Rect, mode: &MainMode) {
        if mode.is_test_detail() || mode.is_test_detail_selected() {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(rect);
            self.test_component.render(f, chunks[0], mode);
            self.test_detail_component.render(f, chunks[1], mode);
        } else {
            self.test_component.render(f, rect, mode);
        }
    }
}

impl Component for Middle {
    fn input(&mut self, key: &KeyEvent, mode: &MainMode) -> Result<Vec<Action>> {
        if let MainMode::TestDetailSelected(_) = mode {
            let ret = self.test_detail_component.input(key, mode)?;
            return Ok(ret);
        } else if let MainMode::TestDetail(id) = mode {
            if let (KeyCode::Enter, KeyModifiers::NONE) = (key.code, key.modifiers) {
                return Ok(vec![Action::ModeChange {
                    mode: MainMode::create_test_detail_with_selected_id(*id),
                    breadcrumb_direction: BreadcrumbDirection::Forward,
                }]);
            }
        }

        self.test_component.input(key, mode)
    }

    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        if let Action::ModeChange { mode, .. } = action {
            if mode.is_test_detail() {
                ret.extend(self.test_detail_component.update(action)?);
            }
        }

        ret.extend(self.test_component.update(action)?);

        Ok(ret)
    }

    fn keys(&self, mode: &MainMode) -> Vec<HelpItem> {
        match mode {
            MainMode::Test => self.test_component.keys(mode),
            MainMode::TestDetail(_) => self.test_detail_component.keys(mode),
            MainMode::TestDetailSelected(_) => self.test_detail_component.keys(mode),
        }
    }
}
