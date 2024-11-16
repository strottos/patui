use crate::{
    db::{PatuiTest, PatuiTestStepId},
    tui::{
        app::{Action, DbRead, EditorMode, HelpItem, PaneType, StatusChange},
        widgets::{Text, TextDisplay},
    },
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eyre::Result;
use ratatui::{
    layout::Alignment,
    prelude::{Frame, Rect},
    style::{Color, Style},
    widgets::{
        Block, Borders, Padding, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState,
        StatefulWidget, Widget, Wrap,
    },
};

use super::Pane;

#[derive(Debug)]
pub(crate) struct TestDetailsPane {
    test: PatuiTest,

    text_display: TextDisplay,
}

impl TestDetailsPane {
    pub(crate) fn new(test: PatuiTest) -> Self {
        let mut text = vec![];

        text.push(Text::new(
            format!(
                "Id: {}\nName: {}\nDescription: {}\nSteps:{}",
                test.id,
                test.name,
                test.description,
                if test.steps.is_empty() { " []" } else { "" }
            ),
            false,
        ));

        for (idx, step) in test.steps.iter().enumerate() {
            match step.get_display_yaml() {
                Ok(yaml) => {
                    text.push(Text::new(yaml, true));
                }
                Err(err) => {
                    text.push(Text::new(
                        format!(
                            "Err reading PatuiStep into yaml from step {}: {:?}\n\tErr: {}",
                            idx, step, err
                        ),
                        true,
                    ));
                }
            }
        }

        let text_display = TextDisplay::new_with_text(text, Some("Test Details".to_string()), true);

        Self { test, text_display }
    }
}

impl Pane for TestDetailsPane {
    fn render(&self, f: &mut Frame, rect: Rect) {
        f.render_widget(&self.text_display, rect);
    }

    fn input(&mut self, key: &KeyEvent) -> Result<Vec<Action>> {
        let mut actions = vec![];

        match (key.code, key.modifiers) {
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                self.text_display.navigate(1);
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                self.text_display.navigate(-1);
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
            }
            // (KeyCode::Char('e'), KeyModifiers::NONE) => {
            //     if let Some(selected_step) = self.selected_step {
            //         actions.push(Action::EditorMode(EditorMode::UpdateTestStep(
            //             self.test.id,
            //             selected_step,
            //         )));
            //     }
            // }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                if !self.text_display.is_selected() {
                    actions.push(Action::StatusChange(StatusChange::ModeChangeTestList));
                } else {
                    self.text_display.set_unselected();
                }
                actions.push(Action::ClearKeys);
                actions.push(Action::ForceRedraw);
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
        PaneType::TestDetail
    }

    fn pane_title(&self) -> String {
        format!("Test Details (id = {})", self.test.id)
    }

    fn set_focus(&mut self, is_focussed: bool) {
        self.text_display.set_focus(is_focussed);
    }
}
