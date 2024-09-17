use crate::{
    tui::{
        app::{Action, DbRead, EditorMode, HelpItem, PaneType},
        widgets::{PatuiWidget, ScrollableArea, Text},
    },
    types::{PatuiTest, PatuiTestStepId},
};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eyre::Result;
use ratatui::{
    prelude::{Frame, Rect},
    text::Line,
    widgets::{Borders, Padding},
};

use super::Pane;

#[derive(Debug)]
pub(crate) struct TestDetailsPane<'a> {
    test: PatuiTest,
    selected_step: PatuiTestStepId,

    is_focussed: bool,

    scrollable_area: PatuiWidget<'a>,
}

impl<'a> TestDetailsPane<'a> {
    pub(crate) fn new(test: PatuiTest) -> Self {
        let mut scrollable_area = ScrollableArea::new_patui_widget();

        scrollable_area.inner_scrollable_mut().unwrap().add_block(
            "Test Details",
            Borders::ALL,
            Padding::symmetric(2, 1),
        );

        let mut text = Text::default();

        text.push_line(Line::from(format!("Id: {}", test.id)));
        text.push_line(Line::from(format!("Name: {}", test.details.name)));
        text.push_line(Line::from(format!(
            "Description: {}\n",
            test.details.description
        )));
        text.push_line(Line::from(format!(
            "Steps:{}",
            if test.details.steps.is_empty() {
                " []"
            } else {
                ""
            }
        )));
        scrollable_area
            .inner_scrollable_mut()
            .unwrap()
            .add_widget(PatuiWidget::new_text(text));

        for step in test.details.steps.iter() {
            let yaml = step.get_display_yaml().unwrap();

            let mut text = Text::new(true);

            yaml.into_iter().for_each(|line| {
                text.push_line(Line::from(line));
            });

            scrollable_area
                .inner_scrollable_mut()
                .unwrap()
                .add_widget(PatuiWidget::new_text(text));
        }

        Self {
            test,
            selected_step: 0.into(),

            is_focussed: false,

            scrollable_area,
        }
    }
}

impl<'a> Pane for TestDetailsPane<'a> {
    fn render(&self, f: &mut Frame, rect: Rect) {
        f.render_widget(&self.scrollable_area, rect);
    }

    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        if let Action::ModeChange {
            mode: PaneType::TestDetail(id),
            ..
        } = action
        {
            if self.test.id != *id {
                ret.push(Action::DbRead(DbRead::TestDetail(*id)));
            }
        }

        Ok(ret)
    }

    fn input(&mut self, key: &KeyEvent) -> Result<Vec<Action>> {
        let mut actions = vec![];

        match (key.code, key.modifiers) {
            (KeyCode::Char('e'), KeyModifiers::NONE) => {
                actions.push(Action::EditorMode(EditorMode::UpdateTestStep(
                    self.test.id,
                    self.selected_step,
                )));
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                if self.selected_step == 0.into() {
                    actions.push(Action::ModeChange {
                        mode: PaneType::TestDetail(self.test.id),
                    });
                } else {
                    self.selected_step = 0.into();
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
            HelpItem::new("u", "Update Test", "Update Test"),
            HelpItem::new("d", "Delete Test", "Delete Test"),
            HelpItem::new("↑ | ↓", "Navigate", "Navigate"),
            HelpItem::new("<Enter>", "Select Test", "Select Test"),
        ]
    }

    fn pane_type(&self) -> PaneType {
        PaneType::TestDetail(self.test.id)
    }

    fn pane_title(&self) -> String {
        format!("Test Details (id = {})", self.test.id)
    }

    fn set_focus(&mut self, focus: bool) {
        self.is_focussed = focus;
        self.scrollable_area
            .inner_scrollable_mut()
            .unwrap()
            .set_highlighted(focus);
    }
}
