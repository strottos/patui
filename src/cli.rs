mod get;
mod new;

use std::sync::Arc;

use clap::Parser;
use color_eyre::Result;

use crate::db::Database;

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_NAME"),
    " ",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_DESCRIPTION"),
    ")"
);

#[derive(Debug, Parser)]
pub enum Command {
    /// Adds files to myapp
    New(new::Command),
    Get(get::Command),
}

impl Command {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        if let Err(e) = db.create_tables().await {
            panic!("Unexpected failure creating tables, aborting\nerror: {}", e);
        }

        match self {
            Command::New(subcommand) => subcommand.handle(db).await,
            Command::Get(subcommand) => subcommand.handle(db).await,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    #[clap(short, long)]
    pub db: Option<String>,

    #[command(subcommand)]
    pub subcommand: Option<Command>,
}

fn version() -> String {
    let author = clap::crate_authors!();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}"
    )
}
