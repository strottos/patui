mod describe;
mod edit;
mod get;
mod new;
mod resources;

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
    /// Describe specific resource
    Describe(describe::Command),

    /// Create a new resource in a YAML file
    New(new::Command),

    /// Edit YAML configs in a file for resources
    Edit(edit::Command),

    /// Gets generic details about resource requested
    Get(get::Command),
}

impl Command {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        if let Err(e) = db.create_tables().await {
            panic!("Unexpected failure creating tables, aborting\nerror: {}", e);
        }

        match self {
            Command::Describe(subcommand) => subcommand.handle(db).await,
            Command::Edit(subcommand) => subcommand.handle(db).await,
            Command::Get(subcommand) => subcommand.handle(db).await,
            Command::New(subcommand) => subcommand.handle(db).await,
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
