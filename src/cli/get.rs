use std::{io::Write, sync::Arc};

use clap::{Args, Parser};
use eyre::Result;

use crate::db::{Database, PatuiTestMinDisplay};

#[derive(Debug, Args)]
#[command(about = "Get an entity")]
pub(crate) struct Command {
    #[command(subcommand)]
    command: GetCommand,
}

impl Command {
    pub(crate) async fn handle(&self, db: Arc<Database>) -> Result<()> {
        match &self.command {
            GetCommand::Test(get_test) | GetCommand::Tests(get_test) => get_test.handle(db).await,
        }
    }
}

#[derive(Parser, Debug)]
pub(crate) enum GetCommand {
    Test(GetTest),
    // Alias for Test
    Tests(GetTest),
}

#[derive(Parser, Debug)]
#[command(about = "Get test details")]
pub(crate) struct GetTest {
    #[clap(short, long)]
    pub(crate) id: Option<i64>,
}

impl GetTest {
    pub(crate) async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let tests: Vec<PatuiTestMinDisplay> = match self.id {
            Some(id) => vec![db.get_test(id.into()).await?.into()],
            None => db
                .get_tests()
                .await?
                .into_iter()
                .map(|x| x.into())
                .collect::<Vec<_>>(),
        };

        std::io::stdout().write_all(&serde_json::to_vec(&tests)?)?;
        std::io::stdout().write_all(b"\n")?;

        Ok(())
    }
}
