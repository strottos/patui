use std::sync::Arc;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Clear,
    Frame,
};
use tokio::sync::mpsc::{self, UnboundedSender};
use tracing::{debug, trace};

use super::{
    components::{
        BottomBar, Component, ErrorComponent, HelpComponent, HelpItem, Middle, PopupComponent,
        TestComponentEdit, TopBar,
    },
    error::{Error, ErrorType},
    terminal::{Event, Tui},
};
use crate::db::Database;

pub(crate) use super::types::*;

#[derive(Debug)]
pub(crate) struct App {
    should_quit: bool,
    last_key_events: Vec<KeyEvent>,
    db: Arc<Database>,

    popups: Vec<Popup>,

    top_bar: TopBar,
    middle: Middle,
    bottom_bar: BottomBar,
    error_component: ErrorComponent,
}

impl App {
    pub(crate) fn new(db: Arc<Database>) -> Result<Self> {
        let last_key_events = vec![];

        let top_bar = TopBar::new();
        let middle = Middle::new();
        let bottom_bar = BottomBar::new();
        let error_component = ErrorComponent::new();

        Ok(Self {
            should_quit: false,
            last_key_events,
            db,

            popups: vec![],

            top_bar,
            middle,
            bottom_bar,
            error_component,
        })
    }

    pub(crate) async fn run(&mut self) -> Result<()> {
        let db = self.db.clone();

        let (action_tx, mut action_rx) = mpsc::unbounded_channel();
        let action_tx_clone = action_tx.clone();

        tokio::spawn(async move {
            let action_tx = action_tx_clone;

            match db.create_tables().await {
                Ok(created_tables) => {
                    if created_tables {
                        let _ = action_tx.send(Action::Error(Error::new(
                            ErrorType::Info,
                            "Welcome to Patui! Press 'n' to create a new test".to_string(),
                        )));
                    }
                }
                Err(e) => {
                    // Panic if we can't send an error for display, we're in a bad state, might as well panic.
                    if let Err(send_e) = action_tx.send(Action::Error(Error::new(
                        ErrorType::Error,
                        format!("{}", e),
                    ))) {
                        panic!(
                            "Unexpected failure to send error, aborting\nerror: {}\nsend_err: {}",
                            e, send_e,
                        );
                    }
                }
            }
        });

        let mut tui = Tui::new()?;
        tui.enter()?;

        loop {
            if let Some(e) = tui.next().await {
                match e {
                    Event::Tick => action_tx.send(Action::Tick)?,
                    Event::Render => action_tx.send(Action::Render)?,
                    Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
                    Event::Key(key) => {
                        self.handle_keys(key, &action_tx)?;
                    }
                    _ => {}
                }
            }

            while let Ok(action) = action_rx.try_recv() {
                if action != Action::Tick && action != Action::Render {
                    debug!("action {:?}", action);
                }
                for action in self.handle_action(&action, &mut tui).await? {
                    action_tx.send(action)?;
                }
            }

            if self.should_quit {
                tui.stop()?;
                break;
            }
        }

        tui.exit()?;

        Ok(())
    }

    fn handle_keys(&mut self, key: KeyEvent, action_tx: &UnboundedSender<Action>) -> Result<()> {
        trace!("Pressed key: {:?}", key);
        trace!("Last key events: {:?}", self.last_key_events);

        self.last_key_events.push(key);

        // Always have a fallback to check for double ctrl-c and quit, can't be
        // overridden
        if self.last_key_events[self.last_key_events.len().saturating_sub(2)..]
            == vec![
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
            ]
        {
            debug!("Got double ctrl-c, quitting");
            action_tx.send(Action::Quit)?;
            self.last_key_events.clear();
        } else if self.last_key_events[self.last_key_events.len().saturating_sub(2)..]
            == vec![
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
            ]
        {
            self.last_key_events.clear();
        } else if self.error_component.has_error() {
            let main_mode = self.main_mode().clone();
            self.error_component.input(&key, &main_mode)?;
        } else {
            let mode = self.main_mode().clone();
            if let Some(popup) = self.popups.last_mut() {
                for action in popup.component.input(&key, &mode)?.into_iter() {
                    action_tx.send(action)?;
                }
                for action in self.bottom_bar.input(&key, &mode)?.into_iter() {
                    action_tx.send(action)?;
                }
            } else {
                for component in self.get_root_components_mut() {
                    for action in component.input(&key, &mode)?.into_iter() {
                        action_tx.send(action)?;
                    }
                }
            }
        }

        Ok(())
    }

    async fn handle_action(&mut self, action: &Action, tui: &mut Tui) -> Result<Vec<Action>> {
        let mut ret = vec![];

        match action {
            Action::Render => {
                tui.draw(|f| self.render(f))?;
            }
            Action::Tick => {}
            Action::Resize(w, h) => {
                tui.resize(Rect::new(0, 0, *w, *h))?;
            }
            Action::Quit => self.should_quit = true,
            Action::Error(ref e) => self.error_component.add_error(e.clone()),
            Action::ModeChange {
                ref mode,
                ref breadcrumb_direction,
            } => {
                if *breadcrumb_direction == BreadcrumbDirection::Forward {
                    self.top_bar.push(
                        match mode {
                            MainMode::Test => "Tests".to_string(),
                            MainMode::TestDetail(id) => {
                                format!("Test {}", id)
                            }
                            MainMode::TestDetailSelected(id) => {
                                format!("Test Details {}", id)
                            }
                            MainMode::TestDetailStep(id, step_num) => {
                                format!("Test Details {} Step {}", id, step_num)
                            }
                        },
                        mode.clone(),
                    );
                } else if *breadcrumb_direction == BreadcrumbDirection::Backward {
                    self.top_bar.pop();
                }
                self.popups.clear();
            }
            Action::PopupCreate(ref popup_mode) => {
                let component: Box<dyn PopupComponent> = match popup_mode {
                    PopupMode::CreateTest => Box::new(TestComponentEdit::new()),
                    PopupMode::UpdateTest(id) => {
                        Box::new(TestComponentEdit::new_update(self.db.get_test(*id).await?)?)
                    }
                    PopupMode::Help => Box::new(HelpComponent::new(self.get_help())),
                };
                self.popups.push(Popup::new(popup_mode.clone(), component));
            }
            Action::PopupClose => {
                self.popups.pop();
            }
            Action::EditorMode(editor_mode) => {
                tracing::trace!("Got editor mode: {:?}", editor_mode);
                tui.exit()?;
                let test_result = match editor_mode {
                    EditorMode::CreateTest => super::editor::create_test(),
                    EditorMode::UpdateTest(id) => {
                        let test = self.db.get_test(*id).await?;
                        super::editor::edit_test(test)
                    }
                    EditorMode::UpdateTestStep(id, step_num) => {
                        let test = self.db.get_test(*id).await?;
                        super::editor::edit_step(test, *step_num)
                    }
                };
                trace!("Got test result: {:?}", test_result);
                match test_result {
                    Ok(test) => {
                        trace!("Changing DB: {:?}", test);
                        let id = test.id;
                        ret.push(Action::DbChange(DbChange::Test(test)));
                        if let Some(id) = id {
                            ret.push(Action::DbRead(DbRead::TestDetail(id)));
                        }
                    }
                    Err(e) => {
                        ret.push(Action::Error(Error::new(
                            ErrorType::Error,
                            format!(
                                "Error {} test, editor failure\n\n{}",
                                match editor_mode {
                                    EditorMode::CreateTest => "creating",
                                    EditorMode::UpdateTest(_)
                                    | EditorMode::UpdateTestStep(_, _) => "editing",
                                },
                                e
                            ),
                        )));
                    }
                }
                tui.enter()?;
            }
            Action::DbRead(ref db_select) => {
                tracing::trace!("Got db select: {:?}", db_select);
                match db_select {
                    DbRead::Test => {
                        self.middle.update_tests(self.db.get_tests().await?);
                    }
                    DbRead::TestDetail(id) => {
                        self.middle.update_test_detail(self.db.get_test(*id).await?);
                    }
                };
            }
            Action::DbChange(ref db_change) => {
                tracing::trace!("Got db change: {:?}", db_change);
                match db_change.clone() {
                    DbChange::Test(test) => {
                        self.db.edit_test(test).await?;
                        self.middle.update_tests(self.db.get_tests().await?);
                    }
                };
            }
            Action::ClearKeys => self.last_key_events.clear(),
        }

        for component in self.get_root_components_mut() {
            for action in component.update(action)?.into_iter() {
                ret.push(action);
            }
        }

        Ok(ret)
    }

    fn get_root_components_mut(&mut self) -> Vec<&mut dyn Component> {
        vec![&mut self.top_bar, &mut self.middle, &mut self.bottom_bar]
    }

    fn get_help(&self) -> Vec<HelpItem> {
        let main_mode = &self.main_mode();
        let mut keys = self.bottom_bar.keys(main_mode);
        keys.extend(if self.error_component.has_error() {
            self.error_component.keys(main_mode)
        } else if let Some(popup) = self.popups.last() {
            popup.component.keys(main_mode)
        } else {
            self.middle.keys(main_mode)
        });

        keys.extend(self.top_bar.keys(main_mode));

        keys
    }

    fn main_mode(&self) -> &MainMode {
        self.top_bar.get_main_mode().unwrap()
    }

    fn render(&mut self, f: &mut Frame) {
        let fsize = f.size();

        let chunks_main = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(2),
                    Constraint::Min(5),
                    Constraint::Length(1),
                ]
                .as_ref(),
            )
            .split(fsize);

        self.top_bar.render(f, chunks_main[0]);
        self.middle.render(f, chunks_main[1], self.main_mode());

        if self.error_component.has_error() {
            self.error_component.render(f, chunks_main[1]);
        } else if let Some(popup) = self.popups.last() {
            self.render_create_popup(f, chunks_main[1], popup);
        }

        self.bottom_bar.render(f, chunks_main[2], self.get_help());
    }

    fn render_create_popup(&self, f: &mut Frame, r: Rect, popup: &Popup) {
        let popup_layout = Layout::vertical([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(r);

        let area = Layout::horizontal([
            Constraint::Percentage(10),
            Constraint::Percentage(80),
            Constraint::Percentage(10),
        ])
        .split(popup_layout[1])[1];

        f.render_widget(Clear, area);

        let title = popup.mode.title();

        popup.component.render(f, area, title);
    }
}
