use crate::{
    tui::{
        app::{Action, DbRead, EditorMode, HelpItem, PaneType},
        widgets::{PatuiWidget, ScrollableArea, Text},
    },
    types::PatuiTest,
};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Alignment,
    prelude::{Frame, Rect},
    style::{Color, Style},
    text::Line,
    widgets::{Block, Borders, Padding},
};

use super::Pane;

#[derive(Debug)]
pub(crate) struct TestDetailsPane {
    test: PatuiTest,
    selected_step: usize,
}

impl TestDetailsPane {
    pub(crate) fn new(test: PatuiTest) -> Self {
        Self {
            test,
            selected_step: 0,
        }
    }
}

impl Pane for TestDetailsPane {
    fn render(&self, f: &mut Frame, rect: Rect, is_selected: bool) {
        let style = if is_selected {
            Style::default()
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let mut scrollable_area = ScrollableArea::new_patui_widget();

        let block = Block::new()
            .borders(Borders::ALL)
            .padding(Padding::symmetric(2, 1))
            .title_alignment(Alignment::Center)
            .title("Test Details")
            .style(style);
        scrollable_area.add_scrollable_block(block);

        match &self.test.id {
            Some(id) => {
                let style = if self.selected_step != 0 {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default()
                };

                let mut text = Text::new();

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
                scrollable_area.add_scrollable_widget(PatuiWidget::Text(text));

                for (i, step) in self.test.steps.iter().enumerate() {
                    let style = if i + 1 == self.selected_step || self.selected_step == 0 {
                        Style::default()
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let yaml = step.get_display_yaml().unwrap();

                    let mut text = Text::new();

                    yaml.into_iter().for_each(|line| {
                        text.push_line(Line::from(line).style(style));
                    });

                    scrollable_area.add_scrollable_widget(PatuiWidget::Text(text));
                }
            }
            None => {
                let mut text = Text::new();

                text.push_line(Line::from("Details not yet loaded...".to_string()));
                scrollable_area.add_scrollable_widget(PatuiWidget::Text(text));
            }
        };

        f.render_widget(&scrollable_area, rect);
    }

    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        if let Action::ModeChange {
            mode: PaneType::TestDetail(id),
            ..
        } = action
        {
            if self.test.id != Some(*id) {
                ret.push(Action::DbRead(DbRead::TestDetail(*id)));
            }
        }

        Ok(ret)
    }

    fn input(&mut self, key: &KeyEvent) -> Result<Vec<Action>> {
        let mut actions = vec![];

        match (key.code, key.modifiers) {
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                if self.selected_step < self.test.steps.len() {
                    self.selected_step += 1;
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                if self.selected_step > 0 {
                    self.selected_step -= 1;
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Char('e'), KeyModifiers::NONE) => {
                actions.push(Action::EditorMode(EditorMode::UpdateTestStep(
                    self.test.id()?,
                    self.selected_step,
                )));
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                if self.selected_step == 0 {
                    actions.push(Action::ModeChange {
                        mode: PaneType::TestDetail(self.test.id()?),
                    });
                } else {
                    self.selected_step = 0;
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
            HelpItem::new("u", "Update Test", "Update Test"),
            HelpItem::new("d", "Delete Test", "Delete Test"),
            HelpItem::new("↑ | ↓", "Navigate", "Navigate"),
            HelpItem::new("<Enter>", "Select Test", "Select Test"),
        ]
    }

    fn pane_type(&self) -> PaneType {
        match self.test.id {
            Some(id) => PaneType::TestDetail(id),
            None => unreachable!(), // Should never have a test here that isn't from the DB
        }
    }

    fn pane_title(&self) -> String {
        match self.test.id {
            Some(id) => format!("Test Details (id = {})", id),
            None => "Test Details".to_string(),
        }
    }
}
