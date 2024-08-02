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
            GetCommand::Step(get_step) => get_step.handle(db).await,
        }
    }
}

#[derive(Parser, Debug)]
pub enum GetCommand {
    Test(GetTest),
    Step(GetStep),
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

#[derive(Parser, Debug)]
#[command(about = "Get step details")]
pub struct GetStep {
    #[clap(short, long)]
    pub test_id: i64,
}

impl GetStep {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let test = db.get_test(self.test_id).await?;
        let steps = db.get_steps(self.test_id).await?;

        let mut ret = serde_json::to_value(&test)?;
        ret.as_object_mut()
            .unwrap()
            .insert("steps".to_string(), serde_json::to_value(&steps)?);

        std::io::stdout().write_all(&serde_json::to_vec(&ret)?)?;
        std::io::stdout().write_all(b"\n")?;

        Ok(())
    }
}
