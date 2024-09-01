use std::sync::Arc;

use clap::{Args, Parser};
use color_eyre::Result;

use crate::{db::Database, types::PatuiTestDetails};

#[derive(Debug, Args)]
#[command(about = "Create new entity")]
pub(crate) struct Command {
    #[command(subcommand)]
    command: EditCommand,
}

impl Command {
    pub(crate) async fn handle(&self, db: Arc<Database>) -> Result<()> {
        match &self.command {
            EditCommand::Test(new_test) => new_test.handle(db).await,
        }
    }
}

#[derive(Parser, Debug)]
pub(crate) enum EditCommand {
    Test(EditTest),
}

#[derive(Parser, Debug)]
#[command(about = "Edit an existing test")]
pub(crate) struct EditTest {
    #[clap(short, long)]
    pub(crate) id: i64,
}

impl EditTest {
    pub(crate) async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let mut test = db.get_test(self.id.into()).await?;

        let yaml_str = test.details.to_editable_yaml_string()?;
        test.details = PatuiTestDetails::edit_yaml(yaml_str)?;

        db.edit_test(&test).await?;
        eprintln!("Successfully saved test: {}", test.details.name);

        Ok(())
    }
}
