use std::{cmp, sync::RwLock};

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Alignment, Constraint, Margin, Rect},
    style::{Color, Style},
    text::Text,
    widgets::{
        Block, Borders, Cell, Padding, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table,
    },
    Frame,
};

use crate::{
    tui::app::{Action, DbRead, EditorMode, HelpItem, PaneType, PopupMode, UpdateData},
    types::PatuiTest,
};

use super::Pane;

const SHORT_WIDTH_DISPLAY: u16 = 60;

#[derive(Debug)]
pub(crate) struct TestsPane {
    initialized: bool,
    loading: bool,

    num_tests_to_display: RwLock<usize>,
    first_row: isize,
    selected_idx: isize,

    tests: Vec<PatuiTest>,
}

impl TestsPane {
    pub(crate) fn new() -> Self {
        Self {
            initialized: false,
            loading: false,

            first_row: 0,
            num_tests_to_display: RwLock::new(17),
            selected_idx: -1,

            tests: vec![],
        }
    }

    fn render_table(&self, f: &mut Frame, r: Rect, is_selected: bool) {
        let style = if is_selected {
            Style::default()
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .padding(Padding::new(1, 2, 1, 1))
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
                Paragraph::new(
                    "No tests found. You can create some by pressing `n` or <ctrl>-`n`.",
                )
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

        let num_tests_to_display = cmp::min((r.height - 7) as usize, self.tests.len());
        *self.num_tests_to_display.write().unwrap() = num_tests_to_display;

        let rows = self
            .tests
            .iter()
            .enumerate()
            .filter(|(i, _)| {
                *i >= self.first_row as usize
                    && *i < (self.first_row as usize) + num_tests_to_display + 1
            })
            .map(|(i, test)| {
                (
                    i,
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
                    },
                )
            })
            .map(|(i, row)| {
                let style = if self.selected_idx == i as isize && is_selected {
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

    fn render_scrollbar(&self, f: &mut Frame, r: Rect) {
        let num_tests_to_display = *self.num_tests_to_display.read().unwrap();
        let total_tests = self.tests.len();

        if total_tests <= num_tests_to_display {
            return;
        }

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut scrollbar_state = ScrollbarState::new(total_tests - num_tests_to_display)
            .position(self.first_row as usize);

        f.render_stateful_widget(
            scrollbar,
            r.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut scrollbar_state,
        );
    }

    pub(crate) fn update_tests(&mut self, tests: Vec<PatuiTest>) {
        self.tests = tests;
        self.loading = false;
        self.initialized = true;
    }

    /// Alters the selected_idx by the count given with optional wrapping and if necessary scrolls
    /// the page to ensure the selected_idx is visible.
    ///
    /// Returns true if changed else false
    pub(crate) fn navigate(&mut self, count: isize, wrap: bool) -> bool {
        let old_selected_idx = self.selected_idx;

        if self.tests.is_empty() {
            return false;
        }

        // On first navigation, if selected_idx is -1, set it to 0, otherwise we
        // select second last element.
        if count < 0 && self.selected_idx == -1 {
            self.selected_idx = 0;
        }

        let mut selected_idx = self.selected_idx + count;
        if !wrap {
            if selected_idx < 0 {
                selected_idx = 0;
            } else if selected_idx >= self.tests.len() as isize {
                selected_idx = self.tests.len() as isize - 1;
            }
        }
        // handle wrap around as mod doesn't work as expected for negative numbers
        while selected_idx < 0 {
            selected_idx += self.tests.len() as isize;
        }
        self.selected_idx = selected_idx % self.tests.len() as isize;

        let num_tests_to_display = *self.num_tests_to_display.read().unwrap() as isize;

        if self.selected_idx - self.first_row >= num_tests_to_display {
            self.first_row = cmp::max(self.selected_idx - num_tests_to_display, 0);
        } else if self.selected_idx < self.first_row {
            self.first_row = self.selected_idx;
        }

        assert!(self.selected_idx >= self.first_row);

        self.selected_idx != old_selected_idx
    }

    /// Scrolls the page keeping any selected_idx in the same part of the view.
    /// NB: Wrapping is always off for this.
    ///
    /// Returns true if selected_idx is visible else false
    pub(crate) fn scroll_page(&mut self, count: isize) -> bool {
        let old_selected_idx = self.selected_idx;

        if self.tests.is_empty() {
            return false;
        }

        let old_position_idx = cmp::max(self.selected_idx - self.first_row, 0);
        let num_tests_to_display = *self.num_tests_to_display.read().unwrap() as isize;

        self.first_row += count;
        if self.first_row < 0 {
            self.first_row = 0;
        } else if self.first_row >= self.tests.len() as isize - num_tests_to_display {
            self.first_row = self.tests.len() as isize - num_tests_to_display - 1;
        }

        if self.selected_idx - self.first_row >= num_tests_to_display
            || self.selected_idx < self.first_row
        {
            self.selected_idx = self.first_row + old_position_idx;
        }

        assert!(self.selected_idx >= self.first_row);

        self.selected_idx != old_selected_idx
    }

    fn get_selected_test_id(&self) -> Option<i64> {
        if self.selected_idx == -1 {
            None
        } else {
            self.tests[self.selected_idx as usize].id
        }
    }
}

impl Pane for TestsPane {
    fn render(&self, f: &mut Frame, r: Rect, is_selected: bool) {
        self.render_table(f, r, is_selected);
        self.render_scrollbar(f, r);
    }

    fn update(&mut self, action: &Action) -> Result<Vec<Action>> {
        let mut ret = vec![];

        match action {
            Action::Tick => {
                if !self.loading && !self.initialized {
                    self.loading = true;
                    ret.push(Action::DbRead(DbRead::Test));
                }
            }
            Action::UpdateData(UpdateData::Tests(tests)) => self.update_tests(tests.clone()),
            _ => (),
        }

        Ok(ret)
    }

    fn input(&mut self, key: &KeyEvent) -> Result<Vec<Action>> {
        let mut actions = vec![];

        match (key.code, key.modifiers) {
            (KeyCode::Char('n'), KeyModifiers::NONE) => {
                actions.push(Action::PopupCreate(PopupMode::CreateTest));
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Char('n'), KeyModifiers::CONTROL) => {
                actions.push(Action::EditorMode(EditorMode::CreateTest));
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Char('u'), KeyModifiers::NONE) => {
                if let Some(test) = self.tests.get(self.selected_idx as usize) {
                    if let Some(id) = test.id {
                        actions.push(Action::PopupCreate(PopupMode::UpdateTest(id)));
                    }
                    actions.push(Action::ClearKeys);
                }
            }
            (KeyCode::Char('e'), KeyModifiers::NONE) => {
                if let Some(test) = self.tests.get(self.selected_idx as usize) {
                    if let Some(id) = test.id {
                        actions.push(Action::EditorMode(EditorMode::UpdateTest(id)));
                    }
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Down, KeyModifiers::NONE) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
                if self.navigate(1, true) {
                    actions.push(Action::ModeChange {
                        mode: self.pane_type(),
                    });
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Up, KeyModifiers::NONE) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
                if self.navigate(-1, true) {
                    actions.push(Action::ModeChange {
                        mode: self.pane_type(),
                    });
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Char('f'), KeyModifiers::CONTROL)
            | (KeyCode::Char('b'), KeyModifiers::CONTROL)
            | (KeyCode::Char('d'), KeyModifiers::CONTROL)
            | (KeyCode::Char('u'), KeyModifiers::CONTROL)
            | (KeyCode::Char('e'), KeyModifiers::CONTROL)
            | (KeyCode::Char('y'), KeyModifiers::CONTROL) => {
                let scroll_count = match key.code {
                    KeyCode::Char('f') => *self.num_tests_to_display.read().unwrap() as isize,
                    KeyCode::Char('b') => -(*self.num_tests_to_display.read().unwrap() as isize),
                    KeyCode::Char('d') => (*self.num_tests_to_display.read().unwrap() as isize) / 2,
                    KeyCode::Char('u') => {
                        -(*self.num_tests_to_display.read().unwrap() as isize) / 2
                    }
                    KeyCode::Char('e') => 1,
                    KeyCode::Char('y') => -1,
                    _ => unreachable!(),
                };
                if self.scroll_page(scroll_count) {
                    // TODO: ^^^ && mode.is_test_detail() {
                    actions.push(Action::ModeChange {
                        mode: self.pane_type(),
                    });
                }
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Char('g'), KeyModifiers::NONE)
            | (KeyCode::Char('G'), KeyModifiers::SHIFT)
            | (KeyCode::Char('H'), KeyModifiers::SHIFT)
            | (KeyCode::Char('M'), KeyModifiers::SHIFT)
            | (KeyCode::Char('L'), KeyModifiers::SHIFT) => {
                match key.code {
                    KeyCode::Char('g') => {
                        self.first_row = 0;
                        self.selected_idx = 0;
                    }
                    KeyCode::Char('G') => {
                        self.first_row = self.tests.len() as isize
                            - *self.num_tests_to_display.read().unwrap() as isize
                            - 1;
                        self.selected_idx = self.tests.len() as isize - 1;
                    }
                    KeyCode::Char('H') => {
                        self.selected_idx = self.first_row;
                    }
                    KeyCode::Char('M') => {
                        self.selected_idx = self.first_row
                            + (*self.num_tests_to_display.read().unwrap() as isize) / 2;
                    }
                    KeyCode::Char('L') => {
                        self.selected_idx =
                            self.first_row + (*self.num_tests_to_display.read().unwrap() as isize);
                    }
                    _ => unreachable!(),
                }
                actions.push(Action::ModeChange {
                    mode: self.pane_type(),
                });
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Esc, KeyModifiers::NONE) => {
                self.selected_idx = -1;
                actions.push(Action::ModeChange {
                    mode: PaneType::Test,
                });
                actions.push(Action::ClearKeys);
            }
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if let Some(id) = self.get_selected_test_id() {
                    actions.push(Action::ModeChange {
                        mode: PaneType::TestDetailSelected(id),
                    });
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
            HelpItem::new("C-n", "New Test Yaml", "Create new Test Yaml in Editor"),
            HelpItem::new("u", "Update Test", "Update Test"),
            HelpItem::new("e", "Edit Test Yaml", "Edit Test Yaml in Editor"),
            HelpItem::new("↑ | ↓ | j | k", "Navigate", "Navigate"),
            HelpItem::new(
                "C-e | C-y",
                "Line Forward / Backward",
                "Go forward or backwards a line of tests",
            ),
            HelpItem::new(
                "C-f | C-b",
                "Page Forward / Backward",
                "Go forward or backwards a page of tests",
            ),
            HelpItem::new(
                "C-d | C-u",
                "Half Page Forward / Backward",
                "Skip forward or backwards half a page of tests",
            ),
        ]
    }

    fn pane_type(&self) -> PaneType {
        match self.get_selected_test_id() {
            Some(id) => PaneType::TestDetail(id),
            None => PaneType::Test,
        }
    }

    fn pane_title(&self) -> String {
        match self.get_selected_test_id() {
            Some(id) => format!("Tests (selected id = {})", id),
            None => "Tests".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use super::*;

    fn create_test(id: i64) -> PatuiTest {
        PatuiTest {
            id: Some(id),
            name: format!("test {}", id),
            description: "test".to_string(),
            creation_date: "2021-01-01".to_string(),
            last_updated: "2021-01-02".to_string(),
            last_used_date: Some("2021-01-02".to_string()),
            times_used: 1,
            steps: vec![],
        }
    }

    fn create_test_component(num_tests: i64) -> TestsPane {
        let mut ret = TestsPane::new();

        let mut tests = vec![];
        for i in 0..num_tests {
            tests.push(create_test(i));
        }

        ret.update_tests(tests);

        ret
    }

    #[test]
    fn test_single_navigate_down_with_wrap() {
        let mut test_component = create_test_component(3);

        let res = test_component.navigate(1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(0);

        let res = test_component.navigate(1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(1);

        let res = test_component.navigate(1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(2);

        let res = test_component.navigate(1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(0);

        let res = test_component.navigate(1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(1);
    }

    #[test]
    fn test_single_navigate_up_with_wrap() {
        let mut test_component = create_test_component(5);

        let res = test_component.navigate(-1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(4);

        let res = test_component.navigate(-1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(3);

        let res = test_component.navigate(-1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(2);
    }

    #[test]
    fn test_single_navigate_down_trivial_cases() {
        let mut test_component = create_test_component(0);

        let res = test_component.navigate(1, true);
        assert_that!(res).is_false();

        let mut test_component = create_test_component(1);

        let res = test_component.navigate(1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(0);

        let res = test_component.navigate(1, true);
        assert_that!(res).is_false();
        assert_that!(test_component.selected_idx).is_equal_to(0);
    }

    #[test]
    fn test_single_navigate_without_wrap() {
        let mut test_component = create_test_component(2);

        let res = test_component.navigate(1, false);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(0);

        let res = test_component.navigate(-1, false);
        assert_that!(res).is_false();
        assert_that!(test_component.selected_idx).is_equal_to(0);

        let res = test_component.navigate(1, false);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(1);

        let res = test_component.navigate(1, false);
        assert_that!(res).is_false();
        assert_that!(test_component.selected_idx).is_equal_to(1);
    }

    #[test]
    fn test_navigate_scroll() {
        let mut test_component = create_test_component(21);

        let res = test_component.navigate(1, false);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(0);
        assert_that!(test_component.first_row).is_equal_to(0);

        let res = test_component.navigate(1, false);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(1);
        assert_that!(test_component.first_row).is_equal_to(0);

        let res = test_component.navigate(16, false);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(17);
        assert_that!(test_component.first_row).is_equal_to(0);

        let res = test_component.navigate(1, false);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(18);
        assert_that!(test_component.first_row).is_equal_to(1);

        let res = test_component.navigate(2, false);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(20);
        assert_that!(test_component.first_row).is_equal_to(3);

        let res = test_component.navigate(1, false);
        assert_that!(res).is_false();
        assert_that!(test_component.selected_idx).is_equal_to(20);
        assert_that!(test_component.first_row).is_equal_to(3);

        let res = test_component.navigate(1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(0);
        assert_that!(test_component.first_row).is_equal_to(0);

        let res = test_component.navigate(-1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(20);
        assert_that!(test_component.first_row).is_equal_to(3);

        let res = test_component.navigate(-1, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(19);
        assert_that!(test_component.first_row).is_equal_to(3);

        let res = test_component.navigate(-17, true);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(2);
        assert_that!(test_component.first_row).is_equal_to(2);

        let res = test_component.navigate(19, false);
        assert_that!(res).is_true();
        assert_that!(test_component.selected_idx).is_equal_to(20);
        assert_that!(test_component.first_row).is_equal_to(3);
    }

    #[test]
    fn test_scroll_page() {
        let mut test_component = create_test_component(29);

        let res = test_component.scroll_page(10);
        assert_that!(res).is_true();
        assert_that!(test_component.first_row).is_equal_to(10);
        assert_that!(test_component.selected_idx).is_equal_to(10);

        let res = test_component.scroll_page(10);
        assert_that!(res).is_true();
        assert_that!(test_component.first_row).is_equal_to(11);
        assert_that!(test_component.selected_idx).is_equal_to(11);

        let res = test_component.scroll_page(10);
        assert_that!(res).is_false();
        assert_that!(test_component.first_row).is_equal_to(11);
        assert_that!(test_component.selected_idx).is_equal_to(11);

        let res = test_component.scroll_page(-10);
        assert_that!(res).is_false();
        assert_that!(test_component.first_row).is_equal_to(1);
        assert_that!(test_component.selected_idx).is_equal_to(11);

        let res = test_component.scroll_page(-10);
        assert_that!(res).is_false();
        assert_that!(test_component.first_row).is_equal_to(0);
        assert_that!(test_component.selected_idx).is_equal_to(11);
    }

    #[test]
    fn test_scroll_page_leaves_selected_index_position() {
        let mut test_component = create_test_component(70);

        let res = test_component.navigate(1, false);
        assert_that!(res).is_true();
        assert_that!(test_component.first_row).is_equal_to(0);
        assert_that!(test_component.selected_idx).is_equal_to(0);

        let res = test_component.navigate(5, false);
        assert_that!(res).is_true();
        assert_that!(test_component.first_row).is_equal_to(0);
        assert_that!(test_component.selected_idx).is_equal_to(5);

        let res = test_component.scroll_page(17);
        assert_that!(res).is_true();
        assert_that!(test_component.first_row).is_equal_to(17);
        assert_that!(test_component.selected_idx).is_equal_to(22);

        let res = test_component.scroll_page(-17);
        assert_that!(res).is_true();
        assert_that!(test_component.first_row).is_equal_to(0);
        assert_that!(test_component.selected_idx).is_equal_to(5);

        let res = test_component.scroll_page(9);
        assert_that!(res).is_true();
        assert_that!(test_component.first_row).is_equal_to(9);
        assert_that!(test_component.selected_idx).is_equal_to(14);

        let res = test_component.scroll_page(-9);
        assert_that!(res).is_false();
        assert_that!(test_component.first_row).is_equal_to(0);
        assert_that!(test_component.selected_idx).is_equal_to(14);
    }
}
