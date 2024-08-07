use std::sync::Arc;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    Frame,
};
use tokio::sync::mpsc::{self, UnboundedSender};
use tracing::{debug, trace};

use super::{
    components::{BottomBar, Component, ErrorComponent, MainComponent, TopBar},
    error::{Error, ErrorType},
    terminal::{Event, Tui},
};
use crate::{db::Database, types::PatuiTest};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum MainMode {
    Test,
    TestDetail(i64),
    TestDetailSelected(i64),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PopupMode {
    CreateTest,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AppMode {
    main: MainMode,
    popup: Option<PopupMode>,
}

impl AppMode {
    pub fn create_normal() -> Self {
        Self {
            main: MainMode::Test,
            popup: None,
        }
    }

    pub fn create_test_detail(id: i64) -> Self {
        Self {
            main: MainMode::TestDetail(id),
            popup: None,
        }
    }

    pub fn create_test_detail_with_selected_id(id: i64) -> Self {
        Self {
            main: MainMode::TestDetailSelected(id),
            popup: None,
        }
    }

    pub fn clone_with_popup(&self, popup: PopupMode) -> Self {
        Self {
            main: self.main.clone(),
            popup: Some(popup),
        }
    }

    pub fn is_test(&self) -> bool {
        matches!(self.main, MainMode::Test)
    }

    pub fn is_test_detail(&self) -> bool {
        matches!(self.main, MainMode::TestDetail(_))
    }

    pub fn is_test_detail_selected(&self) -> bool {
        matches!(self.main, MainMode::TestDetailSelected(_))
    }

    pub fn main_mode(&self) -> &MainMode {
        &self.main
    }

    pub fn popup_mode(&self) -> Option<&PopupMode> {
        self.popup.as_ref()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DbRead {
    Test,
    TestDetail(i64),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DbChange {
    Test(PatuiTest),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Action {
    Tick,
    Render,
    ClearKeys,
    Resize(u16, u16),
    Quit,
    Error(Error),
    ModeChange(AppMode),
    DbRead(DbRead),
    DbChange(DbChange),
}

#[derive(Debug)]
pub struct App<'a> {
    should_quit: bool,
    last_key_events: Vec<KeyEvent>,
    db: Arc<Database>,

    mode: AppMode,

    top_bar: TopBar,
    main: MainComponent<'a>,
    bottom_bar: BottomBar,
    error_component: ErrorComponent,
}

impl<'a> App<'a> {
    pub fn new(db: Arc<Database>) -> Result<Self> {
        let last_key_events = vec![];

        let top_bar = TopBar::new();
        let main = MainComponent::new();
        let bottom_bar = BottomBar::new();
        let error_component = ErrorComponent::new();

        Ok(Self {
            should_quit: false,
            last_key_events,
            db,

            mode: AppMode {
                main: MainMode::Test,
                popup: None,
            },

            top_bar,
            main,
            bottom_bar,
            error_component,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        let db = self.db.clone();

        let (action_tx, mut action_rx) = mpsc::unbounded_channel();
        let action_tx_clone = action_tx.clone();

        tokio::spawn(async move {
            let action_tx = action_tx_clone;

            if let Err(e) = db.create_tables().await {
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
        });

        self.error_component.add_error(Error::new(
            ErrorType::Info,
            "Welcome to Patui! Press 'n' to create a new test".to_string(),
        ));

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

        // Always have a fallback to check for double ctrl-c and quit, can't be
        // overridden
        self.last_key_events.push(key);

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
            self.error_component.input(&key, &self.mode)?;
        } else {
            let mode = self.mode.clone();
            for component in self.get_root_components_mut() {
                for action in component.input(&key, &mode)?.into_iter() {
                    action_tx.send(action)?;
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
            Action::ModeChange(ref mode) => {
                self.mode = mode.clone();
            }
            Action::DbRead(ref db_select) => {
                tracing::trace!("Got db select: {:?}", db_select);
                match db_select {
                    DbRead::Test => {
                        self.main.update_tests(self.db.get_tests().await?);
                    }
                    DbRead::TestDetail(id) => {
                        self.main.update_test_detail(self.db.get_test(*id).await?);
                    }
                };
            }
            Action::DbChange(ref db_change) => {
                tracing::trace!("Got db change: {:?}", db_change);
                match db_change {
                    DbChange::Test(test) => {
                        self.db.create_test(test.clone()).await?;
                        self.main.update_tests(self.db.get_tests().await?);
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
        vec![&mut self.top_bar, &mut self.main, &mut self.bottom_bar]
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
        self.main.render(f, chunks_main[1], &self.mode);
        self.bottom_bar.render(f, chunks_main[2]);

        if self.error_component.has_error() {
            self.error_component.render(f, chunks_main[1]);
        }
    }
}
