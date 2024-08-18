mod app;
mod components;
mod editor;
mod error;
mod terminal;
mod types;

pub(crate) use app::App;

use color_eyre::Result;

use self::terminal::Tui;

pub(crate) fn exit() -> Result<()> {
    let mut tui = Tui::new()?;
    tui.exit()?;
    Ok(())
}
