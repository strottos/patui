use clap::{Args, Parser};
use miette::Result;

use crate::util::edit_file_with_test;

use super::Templates;

#[derive(Debug, Args)]
#[command(about = "Create new entity")]
pub(crate) struct Command {
    #[command(subcommand)]
    command: NewCommand,
}

impl Command {
    pub(crate) async fn run(&self) -> Result<()> {
        match &self.command {
            NewCommand::Test(new_test) => new_test.run().await,
        }
    }
}

#[derive(Parser, Debug)]
pub(crate) enum NewCommand {
    Test(NewTest),
}

#[derive(Parser, Debug)]
#[command(about = "Run a one off test")]
pub(crate) struct NewTest {
    /// Use a standard template that you can optionally edit first
    #[arg(short, long)]
    pub(crate) template: Option<Templates>,

    /// File to output test to. To write to stdout use the `-` value..
    pub(crate) file: String,
}

impl NewTest {
    async fn run(&self) -> Result<()> {
        let yaml_str = match self.template {
            Some(template) => template.get_template()?,
            None => "".to_string(),
        };

        let test = edit_file_with_test(yaml_str, &self.file).await?;

        println!(
            "Successfully created test '{}' at: {}",
            test.name(),
            self.file
        );

        Ok(())
    }
}
