use color_eyre::Result;
use ratatui::{
    layout::{Alignment, Constraint, Rect},
    text::Text,
    widgets::{Block, Borders, Cell, Padding, Paragraph, Row, Table},
    Frame,
};

use crate::{
    tui::app::{Action, DbSelect},
    types::PatuiTest,
};

use super::Component;

const SHORT_WIDTH_DISPLAY: u16 = 60;

#[derive(Debug)]
pub struct TestComponent {
    initialized: bool,
    loading: bool,

    tests: Vec<PatuiTest>,
}

impl TestComponent {
    pub fn new() -> Self {
        Self {
            initialized: false,
            loading: false,

            tests: vec![],
        }
    }

    fn render_table(&self, f: &mut Frame, r: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::symmetric(2, 1))
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
            .map(|row| {
                Row::new(
                    row.into_iter()
                        .enumerate()
                        .map(|(i, s)| {
                            if i >= 4 {
                                Text::from(s).alignment(Alignment::Right)
                            } else {
                                Text::from(s)
                            }
                        })
                        .map(Cell::from)
                        .collect::<Vec<_>>(),
                )
                .height(1)
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
}

impl Component for TestComponent {
    fn render(&mut self, f: &mut Frame, r: Rect) {
        self.render_table(f, r);
    }

    fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        match action {
            Action::Tick => {
                if !self.loading && !self.initialized {
                    self.loading = true;
                    Ok(Some(Action::DbSelect(DbSelect::Tests)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }
}
