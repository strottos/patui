use chrono::{DateTime, Local};
use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use indexmap::IndexMap;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};

use crate::{
    tui::{
        app::{Action, BreadcrumbDirection, DbChange, MainMode},
        components::{
            widgets::{Button, TextArea},
            Component, HelpItem, PopupComponent,
        },
        error::{Error, ErrorType},
    },
    types::PatuiTest,
};

#[derive(Debug)]
pub(crate) struct TestComponentEdit<'a> {
    test: Option<PatuiTest>,
    name_component: TextArea<'a>,
    desc_component: TextArea<'a>,
    selected_component_idx: usize,
    extra_components: IndexMap<String, TextArea<'a>>,
    edit_button: Button,
    cancel_button: Button,
}

impl<'a> TestComponentEdit<'a> {
    pub(crate) fn new() -> Self {
        let mut name_component = TextArea::new(
            "Name".to_string(),
            vec![Box::new(|x| {
                let text = x.get_text();
                if text.contains('\n') || text.contains('\r') || text.is_empty() {
                    return false;
                }
                true
            })],
        );
        name_component.selected(true);

        let desc_component = TextArea::new("Description".to_string(), vec![]);

        let edit_button = Button::new("Create".to_string());
        let cancel_button = Button::new("Cancel".to_string());

        Self {
            test: None,
            name_component,
            desc_component,
            extra_components: IndexMap::new(),
            selected_component_idx: 0,
            edit_button,
            cancel_button,
        }
    }

    pub(crate) fn new_update(test: PatuiTest) -> Result<Self> {
        let mut name_component = TextArea::new(
            "Name".to_string(),
            vec![Box::new(|x| {
                let text = x.get_text();
                if text.contains('\n') || text.contains('\r') || text.is_empty() {
                    return false;
                }
                true
            })],
        );
        name_component.selected(true);

        let desc_component = TextArea::new("Description".to_string(), vec![]);

        let edit_button = Button::new("Update".to_string());
        let cancel_button = Button::new("Cancel".to_string());

        Ok(Self {
            test: Some(test),
            name_component,
            desc_component,
            selected_component_idx: 0,
            extra_components: IndexMap::new(),
            edit_button,
            cancel_button,
        })
    }

    fn activate_selected(&mut self) {
        let num_components = self.num_components();
        let selected_component_idx = self.selected_component_idx;
        for i in 0..num_components - 2 {
            if let Some(component) = self.get_component(i) {
                component.selected(i == selected_component_idx);
            }
        }

        self.edit_button
            .selected(selected_component_idx == num_components - 2);
        self.cancel_button
            .selected(selected_component_idx == num_components - 1);
    }

    fn num_components(&self) -> usize {
        4 + self.extra_components.len()
    }

    fn get_component(&mut self, idx: usize) -> Option<&mut TextArea<'a>> {
        match idx {
            0 => Some(&mut self.name_component),
            _ => Some(&mut self.desc_component),
        }
    }

    fn selected_component(&mut self) -> Option<&mut TextArea<'a>> {
        self.get_component(self.selected_component_idx)
    }

    fn get_editable_components_mut(&mut self) -> Vec<&mut TextArea<'a>> {
        let ret = vec![&mut self.name_component, &mut self.desc_component];
        ret
    }

    fn is_valid(&mut self) -> bool {
        for (i, component) in self.get_editable_components_mut().iter_mut().enumerate() {
            component.validate();
            if !component.is_valid() {
                self.selected_component_idx = i;
                self.activate_selected();
                return false;
            }
        }

        true
    }

    fn is_ok_button(&self) -> bool {
        self.selected_component_idx == self.num_components() - 2
    }

    fn is_cancel_button(&self) -> bool {
        self.selected_component_idx == self.num_components() - 1
    }

    fn clear_components(&mut self) {
        self.name_component.clear();
        self.desc_component.clear();
        self.selected_component_idx = 0;
        self.activate_selected();
    }

    fn get_test_details(&self) -> Result<PatuiTest> {
        match self.test {
            Some(ref test) => {
                let now: DateTime<Local> = Local::now();
                let mut new_test = test.clone();
                new_test.name = self.name_component.get_text().clone();
                new_test.description = self.desc_component.get_text().clone();
                new_test.last_updated = now.format("%Y-%m-%d %H:%M:%S").to_string();
                Ok(new_test)
            }
            None => {
                let now: DateTime<Local> = Local::now();

                Ok(PatuiTest {
                    id: None,
                    name: self.name_component.get_text().clone(),
                    description: self.desc_component.get_text().clone(),
                    creation_date: now.format("%Y-%m-%d %H:%M:%S").to_string(),
                    last_updated: now.format("%Y-%m-%d %H:%M:%S").to_string(),
                    last_used_date: None,
                    times_used: 0,
                    steps: vec![],
                })
            }
        }
    }

    fn crupdate_test(&mut self, mode: &MainMode) -> Vec<Action> {
        if !self.is_valid() {
            return vec![];
        }
        match self.get_test_details() {
            Ok(test) => {
                self.clear_components();
                let mut ret = vec![Action::DbChange(DbChange::Test(test))];
                match mode {
                    MainMode::Test => ret.push(Action::ModeChange {
                        mode: MainMode::create_normal(),
                        breadcrumb_direction: BreadcrumbDirection::None,
                    }),
                    MainMode::TestDetail(id) => ret.push(Action::ModeChange {
                        mode: MainMode::create_test_detail(*id),
                        breadcrumb_direction: BreadcrumbDirection::None,
                    }),
                    MainMode::TestDetailSelected(id) => ret.push(Action::ModeChange {
                        mode: MainMode::create_test_detail_with_selected_id(*id),
                        breadcrumb_direction: BreadcrumbDirection::None,
                    }),
                }
                ret.push(Action::ClearKeys);
                ret
            }
            Err(e) => {
                vec![Action::Error(Error::new(
                    ErrorType::Error,
                    format!(
                        "A fatal error occurred getting the test details:\n\n{:?}",
                        e
                    ),
                ))]
            }
        }
    }
}

impl<'a> Component for TestComponentEdit<'a> {
    fn input(&mut self, key: &KeyEvent, mode: &MainMode) -> Result<Vec<Action>> {
        let mut ret = vec![];

        match (key.code, key.modifiers) {
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.selected_component_idx =
                    (self.selected_component_idx + 1) % self.num_components();
                self.activate_selected();
                ret.push(Action::ClearKeys);
            }
            (KeyCode::BackTab, KeyModifiers::SHIFT) => {
                self.selected_component_idx = (self.selected_component_idx + self.num_components()
                    - 1)
                    % self.num_components();
                self.activate_selected();
                ret.push(Action::ClearKeys);
            }
            (KeyCode::Enter, KeyModifiers::CONTROL) => {
                ret.extend(self.crupdate_test(mode));
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if self.is_ok_button() {
                    self.edit_button.pressed();
                    ret.extend(self.crupdate_test(mode));
                } else if self.is_cancel_button() {
                    self.clear_components();
                    // ret.push(Action::ChangeMode(Mode::Test(self.root_test_mode.clone())));
                    ret.push(Action::ClearKeys);
                }
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                self.clear_components();
                ret.push(Action::PopupClose);
                ret.push(Action::ClearKeys);
            }
            _ => {
                if let Some(selected_component) = self.selected_component() {
                    if selected_component.input(key) {
                        ret.push(Action::ClearKeys);
                    }
                }
            }
        }

        Ok(ret)
    }

    fn keys(&self, _mode: &MainMode) -> Vec<HelpItem> {
        let mut ret = vec![
            HelpItem::new("<Esc>", "Cancel", "Cancel"),
            HelpItem::new("<C-Enter>", "Submit", "Submit"),
            HelpItem::new("<Tab>", "Next Field", "Next Field"),
            HelpItem::new("<S-Tab>", "Previous Field", "Previous Field"),
        ];

        if self.is_ok_button() {
            ret.push(HelpItem::new("<Enter>", "Create/Update", "Press Button"));
        } else if self.is_cancel_button() {
            ret.push(HelpItem::new("<Enter>", "Cancel", "Press Button"));
        }

        ret
    }
}

impl<'a> PopupComponent for TestComponentEdit<'a> {
    fn render_inner(&self, f: &mut Frame, rect: Rect) {
        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Max(self.name_component.height()),
                    Constraint::Max(self.desc_component.height()),
                    Constraint::Min(1),
                    Constraint::Max(3),
                ]
                .as_ref(),
            )
            .split(rect);

        f.render_widget(self.name_component.widget(), inner[0]);
        f.render_widget(self.desc_component.widget(), inner[1]);

        let buttons_inner = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Min(0),
                    Constraint::Max(10),
                    Constraint::Max(1),
                    Constraint::Max(10),
                ]
                .as_ref(),
            )
            .split(inner[3]);

        f.render_widget(self.edit_button.widget(), buttons_inner[1]);
        f.render_widget(self.cancel_button.widget(), buttons_inner[3]);
    }
}
