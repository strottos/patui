use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table},
    Frame,
};

use crate::{
    tui::{
        app::{Action, BreadcrumbDirection, DbRead, MainMode, PopupMode},
        components::{Component, HelpItem},
    },
    types::PatuiTest,
};

const SHORT_WIDTH_DISPLAY: u16 = 60;

#[derive(Debug)]
pub struct TestComponent {
    initialized: bool,
    loading: bool,

    selected_idx: isize,

    tests: Vec<PatuiTest>,
}

impl TestComponent {
    pub fn new() -> Self {
        Self {
            initialized: false,
            loading: false,

            selected_idx: -1,

            tests: vec![],
        }
    }

    fn render_table(&self, f: &mut Frame, r: Rect, mode: &MainMode) {
        let style = if mode.is_test_detail_selected() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::proportional(1))
            .title("Tests")
            .title_alignment(Alignment::Center)
            .style(style);

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
                let style = if self.selected_idx == i as isize && mode.is_test_detail() {
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

    pub fn update_tests(&mut self, tests: Vec<PatuiTest>) {
        self.tests = tests;
        self.loading = false;
        self.initialized = true;
    }

    pub fn render(&self, f: &mut Frame, r: Rect, mode: &MainMode) {
        self.render_table(f, r, mode);
    }
}

impl Component for TestComponent {
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

    fn input(&mut self, key: &KeyEvent, _mode: &MainMode) -> Result<Vec<Action>> {
        let mut actions = vec![];

        // if self.popup_mode == PopupMode::Create {
        //     if key.code == KeyCode::Esc {
        //         // self.set_popup_mode(TestMode::Normal);
        //         actions.push(Action::ClearKeys);
        //     } else {
        //         actions.extend(self.create_test_component.input(key, mode)?);
        //     }
        // } else {
        match (key.code, key.modifiers) {
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                actions.push(Action::PopupCreate(PopupMode::CreateTest));
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Down, KeyModifiers::NONE) => {
                if !self.tests.is_empty() {
                    let selected_idx = (self.selected_idx + 1) % self.tests.len() as isize;
                    self.selected_idx = selected_idx;
                    actions.push(Action::ModeChange {
                        mode: MainMode::create_test_detail(
                            self.tests[selected_idx as usize].id.unwrap(),
                        ),
                        breadcrumb_direction: BreadcrumbDirection::Forward,
                    });
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Up, KeyModifiers::NONE) => {
                if !self.tests.is_empty() {
                    let selected_idx = (self.selected_idx + self.tests.len() as isize - 1)
                        % self.tests.len() as isize;
                    self.selected_idx = selected_idx;
                    actions.push(Action::ModeChange {
                        mode: MainMode::create_test_detail(
                            self.tests[selected_idx as usize].id.unwrap(),
                        ),
                        breadcrumb_direction: BreadcrumbDirection::Forward,
                    });
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                actions.push(Action::ModeChange {
                    mode: MainMode::create_normal(),
                    breadcrumb_direction: BreadcrumbDirection::Backward,
                });
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                // actions.extend(self.set_select_mode(TestMode::Select(self.selected_idx)));
                actions.push(Action::ClearKeys);
            }
            _ => {}
        }
        // }

        Ok(actions)
    }

    fn keys(&self, _mode: &MainMode) -> Vec<HelpItem> {
        vec![
            HelpItem::new("n", "New Test", "New Test"),
            HelpItem::new("u", "Update Test", "Update Test"),
            HelpItem::new("d", "Delete Test", "Delete Test"),
            HelpItem::new("↑ | ↓ | j | k", "Navigate", "Navigate"),
        ]
    }
}
