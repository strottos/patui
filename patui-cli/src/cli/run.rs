use clap::{Args, Parser};
use miette::Result;

use super::Templates;

#[derive(Debug, Args)]
#[command(about = "Run one off tests")]
pub(crate) struct Command {
    #[command(subcommand)]
    command: RunCommand,
}

impl Command {
    pub(crate) async fn run(&self) -> Result<()> {
        match &self.command {
            RunCommand::Test(run_test) => run_test.run().await,
        }
    }
}

#[derive(Parser, Debug)]
pub(crate) enum RunCommand {
    Test(RunTest),
}

#[derive(Parser, Debug)]
#[command(about = "Run a one off test")]
pub(crate) struct RunTest {
    /// Use a standard template that you can optionally edit first
    #[arg(short, long)]
    pub(crate) template: Option<Templates>,

    // Don't bring up editor, default when specifying a file
    #[arg(short, long)]
    pub(crate) no_edit: bool,

    /// File to write test to
    #[arg(short, long)]
    pub(crate) yaml_out_file: Option<String>,

    /// File to write test output to
    #[arg(short, long)]
    pub(crate) output_file: Option<String>,

    /// File containing the test
    pub(crate) file: Option<String>,
}

impl RunTest {
    async fn run(&self) -> std::result::Result<(), miette::Error> {
        todo!()
    }
}
