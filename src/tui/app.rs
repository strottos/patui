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
    components::{BottomBar, Component, MainComponent, TopBar},
    error::{Error, ErrorType},
    terminal::{Event, Tui},
};
use crate::{db::Database, types::PatuiTest};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TestMode {
    Normal,
    Select(isize),
    Create,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Mode {
    Test(TestMode),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DbRead {
    Test,
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
    ChangeMode(Mode),
    DbRead(DbRead),
    DbChange(DbChange),
}

#[derive(Debug)]
pub struct App<'a> {
    should_quit: bool,
    last_key_events: Vec<KeyEvent>,
    db: Arc<Database>,

    top_bar: TopBar,
    main: MainComponent<'a>,
    bottom_bar: BottomBar,
}

impl<'a> App<'a> {
    pub fn new(db: Arc<Database>) -> Result<Self> {
        let last_key_events = vec![];

        let top_bar = TopBar::new();
        let main_app = MainComponent::new();
        let bottom_bar = BottomBar::new();

        Ok(Self {
            should_quit: false,
            last_key_events,
            db,

            top_bar,
            main: main_app,
            bottom_bar,
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
        } else {
            for component in self.get_root_components_mut() {
                for action in component.input(key)?.into_iter() {
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
            Action::Error(ref e) => {
                self.main.add_error(e.clone());
            }
            Action::ChangeMode(ref mode) => {
                self.main.change_mode(mode);
            }
            Action::DbRead(ref db_select) => {
                tracing::trace!("Got db select: {:?}", db_select);
                match db_select {
                    DbRead::Test => {
                        self.main.update_tests(self.db.get_tests().await?);
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
        self.main.render(f, chunks_main[1]);
        self.bottom_bar.render(f, chunks_main[2]);
    }
}
