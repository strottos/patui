use std::{io::Write, sync::Arc};

use clap::{Args, Parser};
use color_eyre::Result;

use crate::db::Database;

#[derive(Debug, Args)]
#[command(about = "Get an entity")]
pub struct Command {
    #[command(subcommand)]
    command: GetCommand,
}

impl Command {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        match &self.command {
            GetCommand::Test(get_test) => get_test.handle(db).await,
        }
    }
}

#[derive(Parser, Debug)]
pub enum GetCommand {
    Test(GetTest),
}

#[derive(Parser, Debug)]
#[command(about = "Get test details")]
pub struct GetTest {
    #[clap(short, long)]
    pub id: Option<i64>,
}

impl GetTest {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let tests = match self.id {
            Some(id) => vec![db.get_test(id).await?],
            None => db.get_tests().await?,
        };

        std::io::stdout().write_all(&serde_json::to_vec(&tests)?)?;
        std::io::stdout().write_all(b"\n")?;

        Ok(())
    }
}
