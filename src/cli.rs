use clap::{Parser, Subcommand};

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_NAME"),
    " ",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_DESCRIPTION"),
    ")"
);

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Adds files to myapp
    NewScript { name: String },
}

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub struct Cli {
    #[clap(short, long)]
    pub db: Option<String>,

    #[command(subcommand)]
    pub subcommand: Option<Commands>,
}

fn version() -> String {
    let author = clap::crate_authors!();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}"
    )
}
