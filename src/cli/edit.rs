use std::sync::Arc;

use clap::{Args, Parser};
use color_eyre::Result;

use crate::db::Database;

use super::resources;

#[derive(Debug, Args)]
#[command(about = "Create new entity")]
pub struct Command {
    #[command(subcommand)]
    command: EditCommand,
}

impl Command {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        match &self.command {
            EditCommand::Test(new_test) => new_test.handle(db).await,
        }
    }
}

#[derive(Parser, Debug)]
pub enum EditCommand {
    Test(EditTest),
}

#[derive(Parser, Debug)]
#[command(about = "Edit an existing test")]
pub struct EditTest {
    #[clap(short, long)]
    pub id: i64,
}

impl EditTest {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let edit_test = resources::EditTest {
            id: Some(self.id),
            name: None,
            description: None,
            steps: None,
        };

        edit_test.handle(db).await?;

        Ok(())
    }
}
