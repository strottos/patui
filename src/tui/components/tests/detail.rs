use crate::{
    tui::{
        app::{Action, BreadcrumbDirection, DbRead, EditorMode, MainMode},
        components::{Component, HelpItem},
    },
    types::PatuiTest,
};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Alignment,
    prelude::{Frame, Rect},
    style::{Color, Style},
    text::{Line, Text},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};

#[derive(Debug)]
pub(crate) struct TestDetailComponent {
    test: PatuiTest,
    selected_step: usize,
}

impl TestDetailComponent {
    pub(crate) fn new() -> Self {
        Self {
            test: PatuiTest::default(), // To be replaced
            selected_step: 0,
        }
    }

    pub(crate) fn update_test_detail(&mut self, test: PatuiTest) {
        self.test = test;
    }

    pub(crate) fn render(&self, f: &mut Frame, rect: Rect, mode: &MainMode) {
        let style = if !mode.is_test_detail_selected() && !mode.is_test_detail_step() {
            Style::default().fg(Color::DarkGray)
        } else {
            Style::default()
        };

        let block = Block::new()
            .borders(Borders::ALL)
            .padding(Padding::symmetric(2, 1))
            .title_alignment(Alignment::Center)
            .title("Test Details")
            .style(style);

        let mut text = Text::default();

        match &self.test.id {
            Some(id) => {
                let style = if self.selected_step != 0 {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };

                text.push_line(Line::from(format!("Id: {}", id)).style(style));
                text.push_line(Line::from(format!("Name: {}", self.test.name)).style(style));
                text.push_line(
                    Line::from(format!("Description: {}\n", self.test.description)).style(style),
                );
                text.push_line(
                    Line::from(format!(
                        "Steps:{}",
                        if self.test.steps.is_empty() {
                            " []"
                        } else {
                            ""
                        }
                    ))
                    .style(style),
                );

                for (i, step) in self.test.steps.iter().enumerate() {
                    let style = if i + 1 == self.selected_step || self.selected_step == 0 {
                        Style::default()
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let yaml = step.get_display_yaml().unwrap();

                    yaml.into_iter().for_each(|line| {
                        text.push_line(Line::from(line).style(style));
                    });
                }
            }
            None => text.push_line("Details not yet loaded...".to_string()),
        };

        let paragraph = Paragraph::new(text).wrap(Wrap { trim: false }).block(block);
        f.render_widget(paragraph, rect);
    }
}

impl Component for TestDetailComponent {
    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        if let Action::ModeChange {
            mode: MainMode::TestDetail(id),
            ..
        } = action
        {
            if self.test.id != Some(*id) {
                ret.push(Action::DbRead(DbRead::TestDetail(*id)));
            }
        }

        Ok(ret)
    }

    fn input(&mut self, key: &KeyEvent, _mode: &MainMode) -> Result<Vec<Action>> {
        let mut actions = vec![];

        match (key.code, key.modifiers) {
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                if self.selected_step < self.test.steps.len() {
                    self.selected_step += 1;
                    actions.push(Action::ModeChange {
                        mode: MainMode::create_test_detail_step(
                            self.test.id.unwrap(),
                            self.selected_step,
                        ),
                        breadcrumb_direction: BreadcrumbDirection::Forward,
                    });
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                if self.selected_step > 0 {
                    self.selected_step -= 1;
                    if self.selected_step == 0 {
                        actions.push(Action::ModeChange {
                            mode: MainMode::create_test_detail(self.test.id.unwrap()),
                            breadcrumb_direction: BreadcrumbDirection::Backward,
                        });
                    } else {
                        actions.push(Action::ModeChange {
                            mode: MainMode::create_test_detail_step(
                                self.test.id.unwrap(),
                                self.selected_step,
                            ),
                            breadcrumb_direction: BreadcrumbDirection::Forward,
                        });
                    }
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Char('e'), KeyModifiers::NONE) => {
                actions.push(Action::EditorMode(EditorMode::UpdateTestStep(
                    self.test.id.unwrap(),
                    self.selected_step,
                )));
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                if self.selected_step != 0 {
                    self.selected_step = 0;
                    actions.push(Action::ClearKeys);
                }
            }
            _ => {}
        }

        Ok(actions)
    }

    fn keys(&self, _mode: &MainMode) -> Vec<HelpItem> {
        vec![
            HelpItem::new("n", "New Test", "New Test"),
            HelpItem::new("u", "Update Test", "Update Test"),
            HelpItem::new("d", "Delete Test", "Delete Test"),
            HelpItem::new("↑ | ↓", "Navigate", "Navigate"),
            HelpItem::new("<Enter>", "Select Test", "Select Test"),
        ]
    }
}
