mod app;
mod bottom_bar;
mod editor;
mod error;
mod panes;
mod popups;
mod terminal;
mod top_bar;
mod types;
mod widgets;

pub(crate) use app::App;

use eyre::Result;

use self::terminal::Tui;

pub(crate) fn exit() -> Result<()> {
    let mut tui = Tui::new()?;
    tui.exit()?;
    Ok(())
}
