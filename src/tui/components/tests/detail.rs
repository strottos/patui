use crate::{
    tui::{
        app::{Action, DbRead, Mode, TestMode},
        components::Component,
    },
    types::PatuiTest,
};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Alignment,
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};

#[derive(Debug)]
pub struct TestDetailComponent {
    test: Option<PatuiTest>,
    step_highlight: Option<i64>,
}

impl TestDetailComponent {
    pub fn new() -> Self {
        Self {
            test: None,
            step_highlight: None,
        }
    }

    pub fn update_test_detail(&mut self, test: PatuiTest) {
        self.test = Some(test);
    }
}

impl Component for TestDetailComponent {
    fn render(&self, f: &mut ratatui::prelude::Frame, rect: ratatui::prelude::Rect) {
        let block = Block::new()
            .borders(Borders::ALL)
            .padding(Padding::symmetric(2, 1))
            .title_alignment(Alignment::Center)
            .title("Test Details");

        let details = match &self.test {
            Some(details) => {
                let mut items: Vec<String> = vec![];
                if let Some(id) = details.id {
                    items.push(format!("Id: {}", id));
                }
                items.push(format!("Name: {}", details.name));
                items.push(format!("Description: {}", details.description));

                for step in details.steps.iter() {
                    items.push(format!("{:#?}", step.details));
                }

                items.join("\n")
            }
            None => "Details not yet loaded...".to_string(),
        };
        let paragraphs = Paragraph::new(details)
            .wrap(Wrap { trim: false })
            .block(block);

        f.render_widget(paragraphs, rect);
    }

    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        if let Action::ChangeMode(Mode::TestDetail(_, id)) = action {
            if self.test.is_none() {
                ret.push(Action::DbRead(DbRead::TestDetail(*id)));
            }

            if let Some(test) = &self.test {
                if test.id != Some(*id) {
                    ret.push(Action::DbRead(DbRead::TestDetail(*id)));
                }
            }
        }

        Ok(ret)
    }

    fn input(&mut self, key: &KeyEvent) -> Result<Vec<Action>> {
        let mut actions = vec![];

        match (key.code, key.modifiers) {
            _ => {}
        }

        Ok(actions)
    }
}
