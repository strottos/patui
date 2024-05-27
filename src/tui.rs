mod app;
mod components;
mod error;
mod terminal;

pub use app::App;

use color_eyre::Result;

use self::terminal::Tui;

pub fn exit() -> Result<()> {
    let mut tui = Tui::new()?;
    tui.exit()?;
    Ok(())
}
