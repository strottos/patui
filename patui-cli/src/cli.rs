mod run;

use clap::Parser;
use miette::Result;
use patui_core::PatuiTest;

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_NAME"),
    " ",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_DESCRIPTION"),
    ")"
);

#[derive(clap::ValueEnum, Debug, Copy, Clone, PartialEq)]
#[clap(rename_all = "lower")]
pub(crate) enum Templates {
    Default,
    SimpleProcess,
    StreamingProcess,
    SimpleSocket,
    StreamingSocket,
    ComplexProcessAndSocket,
}

fn get_template(template: Templates) -> Result<PatuiTest> {
    match template {
        Templates::Default => Ok(PatuiTest::default()),
        Templates::SimpleProcess => Ok(PatuiTest::simple_process()),
        Templates::StreamingProcess => Ok(PatuiTest::streaming_process()),
        Templates::SimpleSocket => Ok(PatuiTest::simple_socket()),
        Templates::StreamingSocket => Ok(PatuiTest::streaming_socket()),
        Templates::ComplexProcessAndSocket => Ok(PatuiTest::complex_process_and_socket()),
    }
}

#[derive(Debug, Parser)]
pub(crate) enum Command {
    // /// Describe specific resource
    // Describe(describe::Command),

    // /// Create a new resource in a YAML file
    // New(new::Command),

    // /// Edit YAML configs in a file for resources
    // Edit(edit::Command),

    // /// Gets generic details about resource requested
    // Get(get::Command),
    /// Run a the test given without recording any information in a database
    Run(run::Command),
}

impl Command {
    pub(crate) async fn handle(&self) -> Result<()> {
        match self {
            // Command::Describe(cmd) => cmd.run(),
            // Command::New(cmd) => cmd.run(),
            // Command::Edit(cmd) => cmd.run(),
            // Command::Get(cmd) => cmd.run(),
            Command::Run(cmd) => cmd.run().await,
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub(crate) struct Cli {
    #[clap(short, long)]
    pub(crate) db: Option<String>,

    #[command(subcommand)]
    pub(crate) subcommand: Option<Command>,
}

fn version() -> String {
    let author = clap::crate_authors!();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}"
    )
}
