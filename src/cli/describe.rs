use std::{io::Write, sync::Arc};

use clap::{Args, Parser};
use color_eyre::Result;

use crate::db::Database;

#[derive(Debug, Args)]
#[command(about = "Get an entity")]
pub(crate) struct Command {
    #[command(subcommand)]
    command: DescribeCommand,
}

impl Command {
    pub(crate) async fn handle(&self, db: Arc<Database>) -> Result<()> {
        match &self.command {
            DescribeCommand::Test(describe_test) | DescribeCommand::Tests(describe_test) => {
                describe_test.handle(db).await
            }
        }
    }
}

#[derive(Parser, Debug)]
pub(crate) enum DescribeCommand {
    Test(DescribeTest),
    // Alias for Test
    Tests(DescribeTest),
}

#[derive(Parser, Debug)]
#[command(about = "Get test details")]
pub(crate) struct DescribeTest {
    #[clap(short, long)]
    pub(crate) id: i64,
}

impl DescribeTest {
    pub(crate) async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let tests = db.get_test(self.id).await?;

        std::io::stdout().write_all(&serde_json::to_vec(&tests)?)?;
        std::io::stdout().write_all(b"\n")?;

        Ok(())
    }
}
