use clap::{Args, Parser};
use miette::{IntoDiagnostic, Result};
use tokio::fs::read_to_string;

use crate::util::edit_file_with_test;

#[derive(Debug, Args)]
#[command(about = "Run one off tests")]
pub(crate) struct Command {
    #[command(subcommand)]
    command: EditCommand,
}

impl Command {
    pub(crate) async fn run(&self) -> Result<()> {
        match &self.command {
            EditCommand::Test(edit_test) => edit_test.run().await,
        }
    }
}

#[derive(Parser, Debug)]
pub(crate) enum EditCommand {
    Test(RunTest),
}

#[derive(Parser, Debug)]
#[command(about = "Edit and validate an existing test")]
pub(crate) struct RunTest {
    /// File containing the test to edit. Must be an existing file.
    pub(crate) file: String,
}

impl RunTest {
    async fn run(&self) -> Result<()> {
        let yaml_str = read_to_string(&self.file).await.into_diagnostic()?;

        let test = edit_file_with_test(yaml_str, &self.file).await?;

        println!(
            "Successfully edited test '{}' at: {}",
            test.name(),
            self.file
        );

        Ok(())
    }
}
