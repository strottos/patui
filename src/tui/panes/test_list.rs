use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eyre::Result;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    text::Text as RatatuiText,
    widgets::{Borders, Padding},
    Frame,
};

use crate::{
    tui::{
        app::{Action, DbRead, EditorMode, HelpItem, PaneType, PopupMode, UpdateData},
        widgets::{PatuiWidget, ScrollableArea, Table, TableHeader, Text},
    },
    types::PatuiTest,
};

use super::Pane;

#[derive(Debug)]
pub(crate) struct TestsPane<'a> {
    initialized: bool,
    loading: bool,

    tests: Vec<PatuiTest>,
    selected_idx: isize,

    is_focussed: bool,

    scrollable_area: PatuiWidget<'a>,
}

impl<'a> TestsPane<'a> {
    pub(crate) fn new() -> Self {
        let mut scrollable_area = ScrollableArea::new_patui_widget();

        scrollable_area.inner_scrollable_mut().unwrap().add_block(
            "Tests",
            Borders::ALL,
            Padding::symmetric(2, 1),
        );

        scrollable_area
            .inner_scrollable_mut()
            .unwrap()
            .add_widget(PatuiWidget::new_text(Text::new_with_text(
                RatatuiText::from("Loading tests...").alignment(Alignment::Left),
                false,
            )));

        Self {
            initialized: false,
            loading: false,

            tests: vec![],
            selected_idx: -1,

            is_focussed: false,

            scrollable_area,
        }
    }

    pub(crate) fn update_tests(&mut self, tests: Vec<PatuiTest>) {
        self.tests = tests;
        self.loading = false;
        self.initialized = true;

        let table = PatuiWidget::new_table(Table::new_with_elements(
            self.tests
                .iter()
                .map(|test| {
                    vec![
                        RatatuiText::from(test.details.name.clone()),
                        RatatuiText::from(test.details.description.clone()),
                        RatatuiText::from(test.details.creation_date.clone()),
                    ]
                })
                .collect::<Vec<Vec<RatatuiText>>>(),
            vec![
                TableHeader::new("Name".into(), 0, Constraint::Min(12)),
                TableHeader::new("Creation Date".into(), 2, Constraint::Max(19)),
            ],
            vec![
                TableHeader::new("Name".into(), 0, Constraint::Min(12)),
                TableHeader::new("Description".into(), 1, Constraint::Min(5)),
                TableHeader::new("Creation Date".into(), 2, Constraint::Max(19)),
                TableHeader::new("Last Used Date".into(), 3, Constraint::Max(19)),
                TableHeader::new("Times Used".into(), 4, Constraint::Max(10)),
            ],
        ));

        self.scrollable_area
            .inner_scrollable_mut()
            .unwrap()
            .set_widgets(vec![table]);
    }

    fn get_selected_test_id(&self) -> Option<i64> {
        if self.selected_idx == -1 {
            None
        } else {
            Some(self.tests[self.selected_idx as usize].id.into())
        }
    }
}

impl<'a> Pane for TestsPane<'a> {
    fn render(&self, f: &mut Frame, rect: Rect) {
        f.render_widget(&self.scrollable_area, rect);
    }

    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        match action {
            Action::Tick => {
                if !self.loading && !self.initialized {
                    self.loading = true;
                    ret.push(Action::DbRead(DbRead::Test));
                }
            }
            Action::UpdateData(UpdateData::Tests(tests)) => self.update_tests(tests.clone()),
            _ => (),
        }

        Ok(ret)
    }

    fn input(&mut self, key: &KeyEvent) -> Result<Vec<Action>> {
        let mut actions = vec![];

        match (key.code, key.modifiers) {
            (KeyCode::Char('n'), KeyModifiers::NONE) => {
                actions.push(Action::PopupCreate(PopupMode::CreateTest));
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                actions.push(Action::EditorMode(EditorMode::CreateTest));
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Char('u'), KeyModifiers::NONE) => {
                if let Some(test) = self.tests.get(self.selected_idx as usize) {
                    actions.push(Action::PopupCreate(PopupMode::UpdateTest(test.id)));
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Char('e'), KeyModifiers::NONE) => {
                if let Some(test) = self.tests.get(self.selected_idx as usize) {
                    actions.push(Action::EditorMode(EditorMode::UpdateTest(test.id)));
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                self.selected_idx = -1;
                actions.push(Action::ModeChange {
                    mode: PaneType::Test,
                });
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if let Some(id) = self.get_selected_test_id() {
                    actions.push(Action::ModeChange {
                        mode: PaneType::TestDetailSelected(id.into()),
                    });
                }
                actions.push(Action::ClearKeys);
            }
            _ => {
                if self
                    .scrollable_area
                    .inner_scrollable_mut()
                    .unwrap()
                    .input(key, false, true)?
                {
                    actions.push(Action::ClearKeys);
                }
            }
        }

        Ok(actions)
    }

    fn keys(&self) -> Vec<HelpItem> {
        vec![
            HelpItem::new("n", "New Test", "New Test"),
            HelpItem::new("C-n", "New Test Yaml", "Create new Test Yaml in Editor"),
            HelpItem::new("u", "Update Test", "Update Test"),
            HelpItem::new("e", "Edit Test Yaml", "Edit Test Yaml in Editor"),
            HelpItem::new("↑ | ↓ | j | k", "Navigate", "Navigate"),
            HelpItem::new(
                "C-e | C-y",
                "Line Forward / Backward",
                "Go forward or backwards a line of tests",
            ),
            HelpItem::new(
                "C-f | C-b",
                "Page Forward / Backward",
                "Go forward or backwards a page of tests",
            ),
            HelpItem::new(
                "C-d | C-u",
                "Half Page Forward / Backward",
                "Skip forward or backwards half a page of tests",
            ),
        ]
    }

    fn pane_type(&self) -> PaneType {
        match self.get_selected_test_id() {
            Some(id) => PaneType::TestDetail(id.into()),
            None => PaneType::Test,
        }
    }

    fn pane_title(&self) -> String {
        match self.get_selected_test_id() {
            Some(id) => format!("Tests (selected id = {})", id),
            None => "Tests".to_string(),
        }
    }

    fn set_focus(&mut self, focus: bool) {
        self.is_focussed = focus;
    }
}
