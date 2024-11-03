use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eyre::Result;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    text::Text,
    widgets::{Block, Borders, Padding},
    Frame,
};

use crate::{
    tui::{
        app::{
            Action, DbRead, EditorMode, HelpItem, PaneType, PopupMode, StatusChange, UpdateData,
        },
        widgets::{ScrollType, Table, TableHeader},
    },
    types::{PatuiTest, PatuiTestId},
};

use super::Pane;

#[derive(Debug)]
pub(crate) struct TestListPane<'a> {
    initialized: bool,
    loading: bool,

    tests: Vec<PatuiTest>,

    table: Table<'a>,
}

impl<'a> TestListPane<'a> {
    pub(crate) fn new() -> Self {
        let mut table = Table::new_with_elements(
            vec![vec!["Loading tests...".into()]],
            vec![TableHeader::new("Name".into(), 0, Constraint::Min(12))],
            vec![TableHeader::new("Name".into(), 0, Constraint::Min(12))],
            Some("Tests List"),
            false,
        );

        table.set_focus(true);

        Self {
            initialized: false,
            loading: false,

            tests: vec![],

            table,
        }
    }

    pub(crate) fn update_tests(&mut self, tests: Vec<PatuiTest>) {
        self.tests = tests;
        self.loading = false;
        self.initialized = true;

        self.table = Table::new_with_elements(
            self.tests
                .iter()
                .map(|test| {
                    vec![
                        Text::from(test.details.name.clone()),
                        Text::from(test.details.description.clone()),
                        Text::from(test.details.creation_date.clone()),
                    ]
                })
                .collect::<Vec<Vec<Text>>>(),
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
            Some("Tests List"),
            true,
        );
    }

    fn get_selected_test_id(&self) -> Option<PatuiTestId> {
        self.table
            .selected_idx()
            .map(|idx| self.tests[idx].id.into())
    }
}

impl<'a> Pane for TestListPane<'a> {
    fn render(&self, f: &mut Frame, rect: Rect) {
        f.render_widget(&self.table, rect);
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
            Action::StatusChange(StatusChange::Reset) => self.table.reset(),
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
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                actions.push(Action::EditorMode(EditorMode::CreateTest));
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Char('u'), KeyModifiers::NONE) => {
                if let Some(selected_test_id) = self.get_selected_test_id() {
                    actions.push(Action::PopupCreate(PopupMode::UpdateTest(selected_test_id)));
                }
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Char('e'), KeyModifiers::NONE) => {
                if let Some(selected_test_id) = self.get_selected_test_id() {
                    actions.push(Action::EditorMode(EditorMode::UpdateTest(selected_test_id)));
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                actions.push(Action::StatusChange(StatusChange::Reset));
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if let Some(id) = self.get_selected_test_id() {
                    actions.push(Action::StatusChange(
                        StatusChange::ModeChangeTestListWithDetails(id),
                    ));
                    actions.push(Action::PaneChange(PaneType::TestDetail));
                }
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL) => {
                self.table.scroll(ScrollType::FullPageDown);
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.table.scroll(ScrollType::FullPageUp);
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.table.scroll(ScrollType::HalfPageDown);
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.table.scroll(ScrollType::HalfPageUp);
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.table.scroll(ScrollType::Single(1));
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                self.table.scroll(ScrollType::Single(-1));
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Char('g'), KeyModifiers::NONE) => {
                self.table.scroll(ScrollType::Top);
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Char('G'), KeyModifiers::SHIFT) => {
                self.table.scroll(ScrollType::Bottom);
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                if self.table.navigate(-1) != 0 {
                    let Some(id) = self.get_selected_test_id() else {
                        panic!("No test selected");
                    };
                    actions.push(Action::StatusChange(
                        StatusChange::ModeChangeTestListWithDetails(id),
                    ));
                    actions.push(Action::ForceRedraw);
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                if self.table.navigate(1) != 0 {
                    let Some(id) = self.get_selected_test_id() else {
                        panic!("No test selected");
                    };
                    actions.push(Action::StatusChange(
                        StatusChange::ModeChangeTestListWithDetails(id),
                    ));
                    actions.push(Action::ForceRedraw);
                }
                actions.push(Action::ClearKeys);
            }
            _ => {}
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
            Some(id) => PaneType::TestDetail,
            None => PaneType::TestList,
        }
    }

    fn pane_title(&self) -> String {
        match self.get_selected_test_id() {
            Some(id) => format!("Tests (selected id = {})", id),
            None => "Tests".to_string(),
        }
    }

    fn set_focus(&mut self, focus: bool) {
        self.table.set_focus(focus);
    }
}
