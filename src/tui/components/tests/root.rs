use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Cell, Clear, Padding, Paragraph, Row, Table},
    Frame,
};

use crate::{
    tui::{
        app::{Action, DbRead, Mode, TestMode},
        components::Component,
    },
    types::PatuiTest,
};

use super::create::TestComponentCreate;

const SHORT_WIDTH_DISPLAY: u16 = 60;

#[derive(Debug, Clone, Eq, PartialEq)]
enum SelectMode {
    Normal,
    Select,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum PopupMode {
    Normal,
    Create,
}

#[derive(Debug)]
pub struct TestComponent<'a> {
    initialized: bool,
    loading: bool,

    selected_idx: isize,
    select_mode: SelectMode,
    popup_mode: PopupMode,

    tests: Vec<PatuiTest>,

    create_test_component: TestComponentCreate<'a>,
}

impl<'a> TestComponent<'a> {
    pub fn new() -> Self {
        Self {
            initialized: false,
            loading: false,

            selected_idx: -1,
            select_mode: SelectMode::Normal,
            popup_mode: PopupMode::Normal,

            tests: vec![],

            create_test_component: TestComponentCreate::new(),
        }
    }

    fn render_table(&self, f: &mut Frame, r: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::proportional(1))
            .title("Tests")
            .title_alignment(Alignment::Center);

        if self.loading {
            f.render_widget(Paragraph::new("Retrieving data...").block(block), r);
            return;
        } else if !self.initialized {
            f.render_widget(Paragraph::new("Yet to initialize tests...").block(block), r);
            return;
        }

        if self.tests.is_empty() {
            f.render_widget(
                Paragraph::new("No tests found. You can create some by pressing CTRL+N.")
                    .block(block),
                r,
            );
            return;
        }

        let header = if r.width < SHORT_WIDTH_DISPLAY {
            ["Name", "Creation Date"]
                .into_iter()
                .map(Cell::from)
                .collect::<Row>()
                .height(2)
        } else {
            [
                "Name",
                "Description",
                "Creation Date",
                "Last Used",
                "Times Used",
            ]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .height(2)
        };

        let rows = self
            .tests
            .iter()
            .map(|test| {
                if r.width < SHORT_WIDTH_DISPLAY {
                    vec![test.name.clone(), test.creation_date.to_string()]
                } else {
                    let last_used_date = if let Some(last_used_date) = &test.last_used_date {
                        last_used_date.to_string()
                    } else {
                        "Never".to_string()
                    };
                    vec![
                        test.name.clone(),
                        test.description.clone(),
                        test.creation_date.to_string(),
                        last_used_date,
                        test.times_used.to_string(),
                    ]
                }
            })
            .enumerate()
            .map(|(i, row)| {
                let style =
                    if self.selected_idx == i as isize && self.select_mode != SelectMode::Normal {
                        Style::default().fg(Color::Black).bg(Color::White)
                    } else {
                        Style::default()
                    };

                Row::new(
                    row.into_iter()
                        .enumerate()
                        .map(|(j, s)| {
                            if j >= 4 {
                                Text::from(s).alignment(Alignment::Right)
                            } else {
                                Text::from(s)
                            }
                        })
                        .map(Cell::from)
                        .collect::<Vec<_>>(),
                )
                .height(1)
                .style(style)
            });

        let table = if r.width < SHORT_WIDTH_DISPLAY {
            Table::new(rows, [Constraint::Min(12), Constraint::Min(12)])
        } else {
            Table::new(
                rows,
                [
                    Constraint::Min(12),
                    Constraint::Min(12),
                    Constraint::Max(20),
                    Constraint::Max(20),
                    Constraint::Max(10),
                ],
            )
        }
        .header(header)
        .block(block);

        f.render_widget(table, r);
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

        self.create_test_component.render(f, area);
    }

    pub fn update_tests(&mut self, tests: Vec<PatuiTest>) {
        self.tests = tests;
        self.loading = false;
        self.initialized = true;
    }

    pub fn set_select_mode(&mut self, test_mode: TestMode) -> Vec<Action> {
        let mut ret = vec![];

        match test_mode {
            TestMode::Normal => {
                self.create_test_component.set_root_test_mode(test_mode);
                self.select_mode = SelectMode::Normal;
                ret.push(Action::ChangeMode(Mode::Test(TestMode::Normal)));
            }
            TestMode::Select(idx) => {
                self.create_test_component.set_root_test_mode(test_mode);
                let selected_idx = (idx + self.tests.len() as isize) % self.tests.len() as isize;
                self.selected_idx = selected_idx;
                self.select_mode = SelectMode::Select;
                ret.push(Action::ChangeMode(Mode::TestDetail(
                    TestMode::Select(selected_idx),
                    self.tests[selected_idx as usize].id.unwrap(),
                )));
            }
            TestMode::Create => {}
        };

        ret
    }

    pub fn set_popup_mode(&mut self, test_mode: TestMode) {
        match test_mode {
            TestMode::Create => {
                self.popup_mode = PopupMode::Create;
            }
            _ => {
                self.popup_mode = PopupMode::Normal;
            }
        };
    }
}

impl<'a> Component for TestComponent<'a> {
    fn render(&self, f: &mut Frame, r: Rect) {
        self.render_table(f, r);

        if self.popup_mode == PopupMode::Create {
            self.render_create_popup(f, r);
        }
    }

    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        if let Action::Tick = action {
            if !self.loading && !self.initialized {
                self.loading = true;
                ret.push(Action::DbRead(DbRead::Test));
            }
        }

        Ok(ret)
    }

    fn input(&mut self, key: KeyEvent) -> Result<Vec<Action>> {
        let mut actions = vec![];

        if self.popup_mode == PopupMode::Create {
            if key.code == KeyCode::Esc {
                self.set_popup_mode(TestMode::Normal);
                actions.push(Action::ClearKeys);
            } else {
                actions.extend(self.create_test_component.input(key)?);
            }
        } else {
            match (&key.code, &key.modifiers) {
                (KeyCode::Char('n'), &KeyModifiers::CONTROL) => {
                    self.set_popup_mode(TestMode::Create);
                    actions.push(Action::ClearKeys);
                }
                (KeyCode::Down, &KeyModifiers::NONE) => {
                    let selected_idx = (self.selected_idx + 1) % self.tests.len() as isize;
                    actions.extend(self.set_select_mode(TestMode::Select(selected_idx)));
                    actions.push(Action::ClearKeys);
                }
                (KeyCode::Up, &KeyModifiers::NONE) => {
                    let selected_idx = (self.selected_idx + self.tests.len() as isize - 1)
                        % self.tests.len() as isize;
                    actions.extend(self.set_select_mode(TestMode::Select(selected_idx)));
                    actions.push(Action::ClearKeys);
                }
                (KeyCode::Esc, &KeyModifiers::NONE) => {
                    actions.extend(self.set_select_mode(TestMode::Normal));
                }
                (KeyCode::Enter, &KeyModifiers::NONE) => {
                    actions.extend(self.set_select_mode(TestMode::Select(self.selected_idx)));
                    actions.push(Action::ClearKeys);
                }
                _ => {}
            }
        }

        Ok(actions)
    }
}
