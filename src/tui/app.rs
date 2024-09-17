use std::{cmp, sync::Arc};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use eyre::Result;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Clear,
    Frame,
};
use tokio::sync::mpsc::{self, UnboundedSender};
use tracing::{debug, trace};

use super::{
    bottom_bar::BottomBar,
    error::{Error, ErrorType},
    panes::{Pane, TestDetailsPane},
    popups::{ErrorComponent, HelpComponent, PopupComponent, TestEditComponent},
    terminal::{Event, Tui},
    top_bar::TopBar,
};
use crate::db::Database;

pub(crate) use super::types::*;

#[derive(Debug)]
enum OtherPane {
    Left(usize),
    Right(usize),
    None,
}

#[derive(Debug)]
pub(crate) struct App {
    should_quit: bool,
    last_key_events: Vec<KeyEvent>,
    db: Arc<Database>,

    panes: Vec<Box<dyn Pane>>,
    popups: Vec<Popup>,
    selected_pane: usize,
    other_pane: OtherPane,
    top_bar: TopBar,
    bottom_bar: BottomBar,

    redraw: bool,
}

impl App {
    pub(crate) fn new(db: Arc<Database>) -> Result<Self> {
        let last_key_events = vec![];

        let top_bar = TopBar::new(vec!["Tests".to_string()]);
        let bottom_bar = BottomBar::new();

        let panes: Vec<Box<dyn Pane>> = vec![Box::new(super::panes::TestsPane::new())];

        Ok(Self {
            should_quit: false,
            last_key_events,
            db,

            panes,
            popups: vec![],
            selected_pane: 0,
            other_pane: OtherPane::None,
            top_bar,
            bottom_bar,

            redraw: true,
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
                    trace!("action {:?}", action);
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
        } else if self.last_key_events[self.last_key_events.len().saturating_sub(1)..]
            == vec![KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)]
        {
            self.last_key_events.clear();
            self.last_key_events.push(key);
        } else {
            let crumb_last_pane = &PaneType::Test;
            if let Some(popup) = self.popups.last_mut() {
                for action in popup.component.input(&key, crumb_last_pane)?.into_iter() {
                    action_tx.send(action)?;
                }
                for action in self.bottom_bar.input(&key, crumb_last_pane)?.into_iter() {
                    action_tx.send(action)?;
                }
            } else {
                let panes_len = self.panes.len();
                let effective_panes_len = cmp::min(panes_len, self.selected_pane + 1);
                for action in self.top_bar.input(&key, effective_panes_len)?.into_iter() {
                    action_tx.send(action)?;
                }
                let pane = self.panes.get_mut(self.selected_pane).unwrap();
                for action in pane.input(&key)?.into_iter() {
                    action_tx.send(action)?;
                }
                for action in self.bottom_bar.input(&key, crumb_last_pane)?.into_iter() {
                    action_tx.send(action)?;
                }
            }
        }

        Ok(())
    }

    async fn handle_action(&mut self, action: &Action, tui: &mut Tui) -> Result<Vec<Action>> {
        let mut extra_actions = vec![];

        match action {
            Action::Render => {
                if self.redraw {
                    tui.draw(|f| self.render(f))?;
                }
                self.redraw = false;
            }
            Action::Tick => {}
            Action::Resize(w, h) => {
                tui.resize(Rect::new(0, 0, *w, *h))?;
                self.redraw = true;
            }
            Action::Quit => self.should_quit = true,
            Action::Error(ref e) => {
                self.popups.push(Popup::new(
                    PopupMode::Error,
                    Box::new(ErrorComponent::new(e.clone())),
                ));
                self.redraw = true;
            }
            Action::ModeChange { ref mode } => {
                self.handle_mode_change(mode, &mut extra_actions).await?;
                self.redraw = true;
            }
            Action::PopupCreate(ref popup_mode) => {
                self.handle_popup_create(popup_mode).await?;
                self.redraw = true;
            }
            Action::PopupClose => {
                self.popups.pop();
                self.redraw = true;
            }
            Action::EditorMode(editor_mode) => {
                self.handle_editor_mode(editor_mode, tui, &mut extra_actions)
                    .await;
            }
            Action::DbRead(ref db_select) => {
                tracing::trace!("Got db select: {:?}", db_select);
                match db_select {
                    DbRead::Test => {
                        extra_actions.push(Action::UpdateData(UpdateData::Tests(
                            self.db.get_tests().await?,
                        )));
                    }
                    DbRead::TestDetail(id) => {
                        extra_actions.push(Action::UpdateData(UpdateData::TestDetail(
                            self.db.get_test(*id).await?,
                        )));
                    }
                };
                self.redraw = true;
            }
            Action::DbCreate(ref db_change) => {
                tracing::trace!("Got db change: {:?}", db_change);
                match db_change.clone() {
                    DbCreate::Test(details) => {
                        let test = self.db.new_test(details).await?;
                        extra_actions.push(Action::UpdateData(UpdateData::Tests(
                            self.db.get_tests().await?,
                        )));
                        extra_actions.push(Action::UpdateData(UpdateData::TestDetail(test)));
                    }
                };
                self.redraw = true;
            }
            Action::DbUpdate(ref db_change) => {
                tracing::trace!("Got db change: {:?}", db_change);
                match db_change.clone() {
                    DbUpdate::Test(test) => {
                        self.db.edit_test(&test).await?;
                        extra_actions.push(Action::UpdateData(UpdateData::Tests(
                            self.db.get_tests().await?,
                        )));
                        extra_actions.push(Action::UpdateData(UpdateData::TestDetail(test)));
                    }
                };
                self.redraw = true;
            }
            Action::PaneChange(pane_idx) => {
                self.handle_pane_change(*pane_idx, &mut extra_actions)
                    .await?;
                self.redraw = true;
            }
            Action::ClearKeys => self.last_key_events.clear(),
            Action::UpdateData(_) => {
                self.redraw = true;
            }
        }

        self.check_redraw();

        for action in self.top_bar.update(action)?.into_iter() {
            extra_actions.push(action);
        }
        for component in self.panes.iter_mut() {
            for action in component.update(action)?.into_iter() {
                extra_actions.push(action);
            }
        }

        Ok(extra_actions)
    }

    fn check_redraw(&mut self) {
        if self.redraw {
            return;
        }

        if let Action::Resize(_, _)
        | Action::Error(_)
        | Action::ModeChange { mode }
        | Action::PopupCreate(_)
        | Action::PopupClose
        | Action::DbRead(_)
        | Action::DbCreate(_)
        | Action::DbUpdate(_)
        | Action::PaneChange(_)
        | Action::UpdateDjta(_) = action
        {
            self.redraw = true;
        }
    }

    fn get_help(&self) -> Vec<HelpItem> {
        let crumb_last_pane = &PaneType::Test;
        let mut keys = self.bottom_bar.keys(crumb_last_pane);
        keys.extend(if let Some(popup) = self.popups.last() {
            popup.component.keys(crumb_last_pane)
        } else {
            let pane = self.panes.get(self.selected_pane).unwrap();
            pane.keys()
        });

        keys.extend(self.top_bar.keys(crumb_last_pane));

        keys
    }

    fn render(&mut self, f: &mut Frame) {
        let fsize = f.area();

        let chunks = Layout::default()
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

        self.top_bar.render(f, chunks[0]);

        match self.other_pane {
            OtherPane::Left(idx) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(chunks[1]);
                let pane = self.panes.get_mut(idx).unwrap();
                pane.set_focus(false);
                pane.render(f, chunks[0]);
                let pane = self.panes.get_mut(self.selected_pane).unwrap();
                pane.set_focus(true);
                pane.render(f, chunks[1]);
            }
            OtherPane::Right(idx) => {
                let chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .split(chunks[1]);
                let pane = self.panes.get_mut(self.selected_pane).unwrap();
                pane.set_focus(true);
                pane.render(f, chunks[0]);
                let pane = self.panes.get_mut(idx).unwrap();
                pane.set_focus(false);
                pane.render(f, chunks[1]);
            }
            OtherPane::None => {
                let pane = self.panes.get_mut(self.selected_pane).unwrap();
                pane.set_focus(true);
                pane.render(f, chunks[1]);
            }
        }

        // if self.error_component.has_error() {
        //     self.error_component.render(f, chunks[1]);
        // } else
        if let Some(popup) = self.popups.last() {
            self.render_create_popup(f, chunks[1], popup);
        }

        self.bottom_bar.render(f, chunks[2], self.get_help());
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

    async fn handle_mode_change(
        &mut self,
        selected_pane: &PaneType,
        extra_actions: &mut Vec<Action>,
    ) -> Result<()> {
        match selected_pane {
            PaneType::Test => {
                self.selected_pane = 0;
                self.other_pane = OtherPane::None;
                while self.panes.len() > 1 {
                    self.panes.pop();
                }
            }
            PaneType::TestDetail(id) => {
                self.selected_pane = 0;
                while self.panes.len() > 1 {
                    self.panes.pop();
                }
                self.panes
                    .push(Box::new(TestDetailsPane::new(self.db.get_test(*id).await?)));
                self.other_pane = OtherPane::Right(1);
            }
            PaneType::TestDetailSelected(_) | PaneType::TestDetailStep(_, _) => {
                self.selected_pane = 1;
                self.other_pane = OtherPane::Left(0);
            }
        }

        let panes_len = self.panes.len();
        let effective_panes_len = cmp::min(panes_len, self.selected_pane + 1);
        extra_actions.push(Action::UpdateData(UpdateData::BreadcrumbTitles(
            self.panes[0..effective_panes_len]
                .iter()
                .map(|pane| pane.pane_title())
                .collect::<Vec<String>>(),
        )));

        self.popups.clear();

        Ok(())
    }

    async fn handle_editor_mode(
        &self,
        editor_mode: &EditorMode,
        tui: &mut Tui,
        ret: &mut Vec<Action>,
    ) {
        if let Err(e) = self.handle_editor_mode_inner(editor_mode, tui, ret).await {
            ret.push(Action::Error(Error::new(
                ErrorType::Error,
                format!(
                    "Error {} test, editor failure\n\n{}",
                    match editor_mode {
                        EditorMode::CreateTest => "creating",
                        EditorMode::UpdateTest(_) | EditorMode::UpdateTestStep(_, _) => "editing",
                    },
                    e
                ),
            )));
        }
    }

    async fn handle_editor_mode_inner(
        &self,
        editor_mode: &EditorMode,
        tui: &mut Tui,
        ret: &mut Vec<Action>,
    ) -> Result<()> {
        tracing::trace!("Got editor mode: {:?}", editor_mode);
        tui.exit()?;
        match editor_mode {
            EditorMode::CreateTest => {
                let test_details = super::editor::create_test()?;
                ret.push(Action::DbCreate(DbCreate::Test(test_details)));
            }
            EditorMode::UpdateTest(id) => {
                let test = self.db.get_test(*id).await?;
                let test = super::editor::edit_test(test)?;
                let test_id = test.id;
                ret.push(Action::DbUpdate(DbUpdate::Test(test)));
                ret.push(Action::DbRead(DbRead::TestDetail(test_id)));
            }
            EditorMode::UpdateTestStep(id, step_num) => {
                let test = self.db.get_test(*id).await?;
                let test = super::editor::edit_step(test, *step_num)?;
                let test_id = test.id;
                ret.push(Action::DbUpdate(DbUpdate::Test(test)));
                ret.push(Action::DbRead(DbRead::TestDetail(test_id)));
            }
        };
        tui.enter()?;

        Ok(())
    }

    async fn handle_popup_create(&mut self, popup_mode: &PopupMode) -> Result<()> {
        let component: Box<dyn PopupComponent> = match popup_mode {
            PopupMode::CreateTest => Box::new(TestEditComponent::new()),
            PopupMode::UpdateTest(id) => todo!(),
            // Box::new(TestEditComponent::new_update(
            //     self.db.get_test(*id).await?.details,
            // )?),
            PopupMode::Help => Box::new(HelpComponent::new(self.get_help())),
            PopupMode::Error => unreachable!(), // Handled elsewhere, use Action::Error
        };
        self.popups.push(Popup::new(popup_mode.clone(), component));

        Ok(())
    }

    async fn handle_pane_change(&mut self, pane_idx: usize, ret: &mut Vec<Action>) -> Result<()> {
        self.selected_pane = pane_idx;
        if pane_idx == 0 {
            self.other_pane = OtherPane::None;
            while self.panes.len() > 1 {
                self.panes.pop();
            }
        } else if pane_idx == 1 {
            while self.panes.len() > 2 {
                self.panes.pop();
            }
            if self.panes.len() == 2 {
                self.other_pane = OtherPane::Right(1);
            } else {
                self.other_pane = OtherPane::None;
            }
        } else {
            while self.panes.len() > pane_idx {
                self.panes.pop();
            }
            if self.panes.len() > pane_idx {
                self.other_pane = OtherPane::Left(pane_idx - 1);
            } else {
                self.other_pane = OtherPane::None;
            }
        }
        ret.push(Action::UpdateData(UpdateData::BreadcrumbTitles(
            self.panes
                .iter()
                .map(|pane| pane.pane_title())
                .collect::<Vec<String>>(),
        )));
        self.selected_pane = pane_idx;

        Ok(())
    }
}
