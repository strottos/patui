mod app;
mod components;
mod error;
mod terminal;

pub(crate) use app::App;

use color_eyre::Result;

use self::terminal::Tui;

pub(crate) fn exit() -> Result<()> {
    let mut tui = Tui::new()?;
    tui.exit()?;
    Ok(())
}
