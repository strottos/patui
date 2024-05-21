mod test_setup;

use std::sync::Arc;

use clap::{Args, Parser};
use color_eyre::Result;

use crate::db::Database;

use test_setup::NewTest;

#[derive(Debug, Args)]
#[command(about = "Create new entity")]
pub struct Command {
    #[command(subcommand)]
    command: NewCommand,
}

impl Command {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        match &self.command {
            NewCommand::Test(new_test) => new_test.handle(db).await,
        }
    }
}

#[derive(Parser, Debug)]
pub enum NewCommand {
    Test(NewTest),
}
