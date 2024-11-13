use std::fmt::Debug;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    prelude::Style,
    style::{Color, Modifier},
    widgets::{Block, Borders, Widget, WidgetRef},
};

type ValidateFn = Box<dyn Fn(&TextArea) -> bool>;

pub(crate) struct TextArea<'a> {
    inner: tui_textarea::TextArea<'a>,
    name: String,
    height: u16,
    is_valid: bool,
    selected: bool,
    validate: Vec<ValidateFn>,
    valid_entries: Vec<String>,
}

impl Debug for TextArea<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TextArea")
            .field("inner", &self.inner)
            .field("name", &self.name)
            .field("height", &self.height)
            .field("is_valid", &self.is_valid)
            .field("selected", &self.selected)
            .field("valid_entries", &self.valid_entries)
            .finish()
    }
}

impl<'a> TextArea<'a> {
    pub(crate) fn new(name: String, validate: Vec<ValidateFn>) -> Self {
        let mut inner = tui_textarea::TextArea::default();

        let block = Block::default()
            .title(name.clone())
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::DarkGray));
        inner.set_block(block);

        inner.set_cursor_line_style(Style::default());
        inner.set_cursor_style(Style::default());

        Self {
            inner,
            name,
            height: 3,
            is_valid: true,
            selected: false,
            validate,
            valid_entries: vec![],
        }
    }

    pub(crate) fn set_valid_entries(&mut self, mut valid_entries: Vec<String>) {
        valid_entries.sort();
        self.valid_entries = valid_entries;
        if self.validate.is_empty() {
            self.validate
                .push(Box::new(|s| s.valid_entries.contains(&s.get_text())));
        }
    }

    pub(crate) fn get_text(&'a self) -> String {
        self.inner.lines().join("\n")
    }

    pub(crate) fn height(&'a self) -> u16 {
        self.height
    }

    pub(crate) fn name(&'a self) -> &'a str {
        &self.name
    }

    pub(crate) fn is_valid(&'a self) -> bool {
        self.is_valid
    }

    pub(crate) fn clear(&mut self) {
        self.inner.select_all();
        self.inner.delete_line_by_head();
    }

    pub(crate) fn input(&mut self, key: &KeyEvent) -> bool {
        if !self.valid_entries.is_empty() {
            match &key.code {
                KeyCode::Up => {
                    let existing_text = self.get_text();
                    let text = self.valid_entries.iter().rfind(|x| x < &&existing_text);
                    let text = text
                        .unwrap_or_else(|| self.valid_entries.first().unwrap())
                        .clone();
                    self.set_text(text);
                    return true;
                }
                KeyCode::Down => {
                    let existing_text = self.get_text();
                    let text = self.valid_entries.iter().find(|x| x > &&existing_text);
                    let text = text
                        .unwrap_or_else(|| self.valid_entries.last().unwrap())
                        .clone();
                    self.set_text(text);
                    return true;
                }
                KeyCode::Enter => {
                    return false;
                }
                _ => {}
            }
        }
        let result = self.inner.input(*key);
        if result {
            self.validate();
        }
        result
    }

    fn check_is_valid(&self) -> bool {
        self.validate.iter().map(|f| f(self)).all(|x| {
            #[allow(clippy::bool_comparison)]
            (x == true)
        })
    }

    pub(crate) fn validate(&mut self) {
        self.is_valid = self.check_is_valid();
        self.setup_widget();
    }

    fn set_text(&mut self, text: String) {
        self.inner.select_all();
        self.inner.delete_line_by_head();
        self.inner.set_yank_text(text);
        self.inner.paste();
        self.validate();
    }

    fn setup_widget(&mut self) {
        let block = Block::default()
            .title(self.name.clone())
            .borders(Borders::ALL);
        let block = match (self.selected, self.is_valid) {
            (true, true) => block.style(Style::default()),
            (true, false) => block.style(Style::default().fg(Color::LightRed)),
            (false, true) => block.style(Style::default().fg(Color::DarkGray)),
            (false, false) => block.style(Style::default().fg(Color::Red)),
        };
        self.inner.set_block(block);
        if self.selected {
            self.inner
                .set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
            self.inner
                .set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        } else {
            self.inner.set_cursor_line_style(Style::default());
            self.inner.set_cursor_style(Style::default());
        }
    }

    pub(crate) fn selected(&mut self, selected: bool) {
        self.selected = selected;
        self.setup_widget();
    }
}

impl WidgetRef for TextArea<'_> {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        self.inner.render(area, buf);
    }
}
