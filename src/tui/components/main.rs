use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Clear,
    Frame,
};

use crate::{
    tui::app::{Action, AppMode, MainMode, PopupMode},
    types::PatuiTest,
};

use super::{
    tests::{TestComponent, TestComponentCreate, TestDetailComponent},
    Component,
};

#[derive(Debug)]
pub struct MainComponent<'a> {
    test_component: TestComponent,
    test_detail_component: TestDetailComponent,
    test_create_component: TestComponentCreate<'a>,
}

impl<'a> MainComponent<'a> {
    pub fn new() -> Self {
        let test_component = TestComponent::new();
        let test_detail_component = TestDetailComponent::new();
        let test_create_component = TestComponentCreate::new();

        Self {
            test_component,
            test_detail_component,
            test_create_component,
        }
    }

    pub fn update_tests(&mut self, tests: Vec<PatuiTest>) {
        self.test_component.update_tests(tests);
    }

    pub fn update_test_detail(&mut self, test: PatuiTest) {
        self.test_detail_component.update_test_detail(test);
    }

    fn render_create_popup(&self, f: &mut Frame, r: Rect) {
        let popup_layout = Layout::vertical([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(r);

        let area = Layout::horizontal([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(popup_layout[1])[1];

        f.render_widget(Clear, area);

        self.test_create_component.render(f, area);
    }

    pub fn render(&self, f: &mut Frame, rect: Rect, mode: &AppMode) {
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

        if let Some(PopupMode::CreateTest) = mode.popup_mode() {
            self.render_create_popup(f, rect);
        }
    }
}

impl<'a> Component for MainComponent<'a> {
    fn input(&mut self, key: &KeyEvent, mode: &AppMode) -> Result<Vec<Action>> {
        if let Some(PopupMode::CreateTest) = mode.popup_mode() {
            return self.test_create_component.input(key, mode);
        } else if let MainMode::TestDetailSelected(id) = mode.main_mode() {
            if let (KeyCode::Esc, KeyModifiers::NONE) = (key.code, key.modifiers) {
                return Ok(vec![Action::ModeChange(
                    AppMode::create_test_detail_with_selected_id(*id),
                )]);
            }
            let ret = self.test_detail_component.input(key, mode)?;
            if !ret.is_empty() {
                return Ok(ret);
            }
        } else if let MainMode::TestDetail(id) = mode.main_mode() {
            if let (KeyCode::Enter, KeyModifiers::NONE) = (key.code, key.modifiers) {
                return Ok(vec![Action::ModeChange(
                    AppMode::create_test_detail_with_selected_id(*id),
                )]);
            }
        }

        self.test_component.input(key, mode)
    }

    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        if let Action::ModeChange(mode) = action {
            if mode.is_test_detail() {
                ret.extend(self.test_detail_component.update(action)?);
            }
        }

        ret.extend(self.test_component.update(action)?);

        Ok(ret)
    }
}
