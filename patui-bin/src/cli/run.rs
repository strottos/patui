use clap::{Args, Parser};
use miette::{IntoDiagnostic, Result};
use patui_core::PatuiTest;
use tokio::{
    fs::read_to_string,
    io::{stdin, AsyncReadExt},
};

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
    /// File to write test output to
    #[arg(short, long)]
    pub(crate) output_file: Option<String>,

    /// YAML file containing the test to run.  To read from stdin or write to stdout use the `-` value.
    pub(crate) file: String,
}

impl RunTest {
    async fn run(&self) -> Result<()> {
        let contents = match self.file.as_str() {
            "-" => {
                let mut buffer = String::new();
                stdin()
                    .read_to_string(&mut buffer)
                    .await
                    .into_diagnostic()?;
                buffer
            }
            _ => read_to_string(&self.file).await.into_diagnostic()?,
        };

        let test = PatuiTest::from_yaml_str(&contents).into_diagnostic()?;

        Ok(())
    }
}
