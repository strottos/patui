use crate::{
    tui::{
        app::{Action, DbRead, MainMode},
        components::{Component, HelpItem},
    },
    types::PatuiTest,
};

use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{
    layout::Alignment,
    prelude::{Frame, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Padding, Paragraph, Wrap},
};

#[derive(Debug)]
pub struct TestDetailComponent {
    test: Option<PatuiTest>,
}

impl TestDetailComponent {
    pub fn new() -> Self {
        Self { test: None }
    }

    pub fn update_test_detail(&mut self, test: PatuiTest) {
        self.test = Some(test);
    }

    pub fn render(&self, f: &mut Frame, rect: Rect, mode: &MainMode) {
        let style = if !mode.is_test_detail_selected() {
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
}

impl Component for TestDetailComponent {
    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        if let Action::ModeChange {
            mode: MainMode::TestDetail(id),
            ..
        } = action
        {
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

    fn input(&mut self, _key: &KeyEvent, _mode: &MainMode) -> Result<Vec<Action>> {
        let actions = vec![];

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
