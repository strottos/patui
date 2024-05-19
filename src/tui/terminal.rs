use std::{
    io::Write,
    ops::{Deref, DerefMut},
    time::Duration,
};

use color_eyre::Result;
use crossterm::{
    event::{KeyEvent, KeyEventKind},
    terminal::LeaveAlternateScreen,
    ExecutableCommand,
};
use futures::{FutureExt, StreamExt};
use ratatui::backend::CrosstermBackend as Backend;
use tokio::{
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug)]
pub enum Event {
    Init,
    Tick,
    Render,
    Error,
    Key(KeyEvent),
    Resize(u16, u16),
}

type IO = std::io::Stdout;
fn stdout() -> IO {
    std::io::stdout()
}

pub struct Tui {
    terminal: ratatui::Terminal<Backend<IO>>,
    task: JoinHandle<()>,
    cancellation_token: CancellationToken,
    event_rx: UnboundedReceiver<Event>,
    event_tx: UnboundedSender<Event>,
    frame_rate: f64,
    tick_rate: f64,
}

impl Tui {
    pub fn new() -> Result<Self> {
        let terminal = ratatui::Terminal::new(Backend::new(stdout()))?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let cancellation_token = CancellationToken::new();
        let task = tokio::spawn(async move {});
        let tick_rate = 4.0;
        let frame_rate = 20.0;

        Ok(Self {
            terminal,
            cancellation_token,
            task,
            event_rx,
            event_tx,
            tick_rate,
            frame_rate,
        })
    }

    pub fn start(&mut self) {
        let tick_delay = std::time::Duration::from_secs_f64(1.0 / self.tick_rate);
        let render_delay = std::time::Duration::from_secs_f64(1.0 / self.frame_rate);
        self.cancel();
        self.cancellation_token = CancellationToken::new();

        let event_tx = self.event_tx.clone();
        let cancellation_token = self.cancellation_token.clone();
        self.task = tokio::spawn(async move {
            if let Err(e) = handle_events(
                event_tx.clone(),
                cancellation_token.clone(),
                tick_delay,
                render_delay,
            )
            .await
            {
                panic!("Error handling events, aborting: {:?}", e);
            }
        });
    }

    pub fn stop(&mut self) -> Result<()> {
        self.cancel();
        Ok(())
    }

    pub fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        stdout().execute(LeaveAlternateScreen)?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;

        self.start();

        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        self.stop()?;
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.flush()?;
            stdout().execute(LeaveAlternateScreen)?;
            crossterm::terminal::disable_raw_mode()?;
        }
        Ok(())
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    pub async fn next(&mut self) -> Option<Event> {
        self.event_rx.recv().await
    }
}

impl Deref for Tui {
    type Target = ratatui::Terminal<Backend<IO>>;

    fn deref(&self) -> &Self::Target {
        &self.terminal
    }
}

impl DerefMut for Tui {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.terminal
    }
}

impl Drop for Tui {
    fn drop(&mut self) {
        if let Err(e) = self.exit() {
            let _ = std::io::stderr().write_all(format!("Error exiting: {:?}", e).as_bytes());
        }
    }
}

async fn handle_events(
    event_tx: UnboundedSender<Event>,
    cancellation_token: CancellationToken,
    tick_delay: Duration,
    render_delay: Duration,
) -> Result<()> {
    let mut reader = crossterm::event::EventStream::new();
    let mut tick_interval = tokio::time::interval(tick_delay);
    let mut render_interval = tokio::time::interval(render_delay);

    event_tx.send(Event::Init)?;

    loop {
        let tick_delay = tick_interval.tick();
        let render_delay = render_interval.tick();
        let crossterm_event = reader.next().fuse();
        tokio::select! {
            _ = cancellation_token.cancelled() => break,
            event = crossterm_event => {
                match event {
                    Some(Ok(event)) => {
                        match event {
                            crossterm::event::Event::Key(key) => handle_key(key, event_tx.clone()).await?,
                            crossterm::event::Event::Mouse(_) => todo!(),
                            crossterm::event::Event::Resize(x, y) => event_tx.send(Event::Resize(x, y))?,
                            crossterm::event::Event::FocusGained => todo!(),
                            crossterm::event::Event::FocusLost => todo!(),
                            crossterm::event::Event::Paste(_) => todo!(),
                        }
                    }
                    Some(Err(_)) => {
                        event_tx.send(Event::Error)?;
                    }
                    None => {}
                }
            }
            _ = tick_delay => {
                event_tx.send(Event::Tick)?;
            }
            _ = render_delay => {
                event_tx.send(Event::Render)?;
            }
        }
    }

    Ok(())
}

async fn handle_key(key: KeyEvent, event_tx: UnboundedSender<Event>) -> Result<()> {
    if key.kind == KeyEventKind::Press {
        event_tx.send(Event::Key(key))?;
    }
    Ok(())
}
