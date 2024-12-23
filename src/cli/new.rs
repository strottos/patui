use std::{io::Read, sync::Arc};

use clap::{Args, Parser};
use eyre::Result;

use crate::{
    db::Database,
    runner::TestRunner,
    types::{PatuiRunDisplay, PatuiTestDetails},
};

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

fn get_template(template: Templates) -> Result<PatuiTestDetails> {
    match template {
        Templates::Default => Ok(PatuiTestDetails::default()),
        Templates::SimpleProcess => Ok(PatuiTestDetails::simple_process()),
        Templates::StreamingProcess => Ok(PatuiTestDetails::streaming_process()),
        Templates::SimpleSocket => Ok(PatuiTestDetails::simple_socket()),
        Templates::StreamingSocket => Ok(PatuiTestDetails::streaming_socket()),
        Templates::ComplexProcessAndSocket => Ok(PatuiTestDetails::complex_process_and_socket()),
    }
}

#[derive(Debug, Args)]
#[command(about = "Create new entity")]
pub(crate) struct Command {
    #[command(subcommand)]
    command: NewCommand,
}

impl Command {
    pub(crate) async fn handle(&self, db: Arc<Database>) -> Result<()> {
        match &self.command {
            NewCommand::Test(new_test) => new_test.handle(db).await,
            NewCommand::Run(new_run) => new_run.handle(db).await,
        }
    }
}

#[derive(Parser, Debug)]
pub(crate) enum NewCommand {
    Test(NewTest),
    Run(NewRun),
}

#[derive(Parser, Debug)]
#[command(about = "Create a new test")]
pub(crate) struct NewTest {
    // Use a standard template
    #[arg(short, long)]
    pub(crate) template: Option<Templates>,

    // Don't bring up editor
    #[arg(short, long)]
    pub(crate) no_edit: bool,

    // List of files containing yaml for tests, use '-' for stdin
    pub(crate) files: Vec<String>,
}

impl NewTest {
    pub(crate) async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let mut pending_tests = vec![];

        for file in &self.files {
            let contents = match file.as_str() {
                "-" => {
                    let mut buffer = String::new();
                    std::io::stdin().read_to_string(&mut buffer)?;
                    buffer
                }
                _ => std::fs::read_to_string(file)?,
            };

            let test = if self.no_edit {
                PatuiTestDetails::from_yaml_str(&contents)?
            } else {
                PatuiTestDetails::edit_yaml(contents)?
            };
            pending_tests.push(test);
        }

        if pending_tests.is_empty() {
            let template = if let Some(template) = self.template {
                get_template(template)?
            } else {
                get_template(Templates::Default)?
            };

            if self.no_edit {
                pending_tests.push(template);
            } else {
                let yaml_str = template.to_editable_yaml_string()?;
                let test = PatuiTestDetails::edit_yaml(yaml_str)?;
                pending_tests.push(test);
            }
        }

        if pending_tests.is_empty() {
            eprintln!("No tests to create, remove --no-edit or provide valid files");
            std::process::exit(1);
        }

        let mut edited_tests = vec![];

        for test in pending_tests.into_iter() {
            let test_name = test.name.clone();
            match db.new_test(test).await {
                Ok(test) => {
                    edited_tests.push(test.into_test_status("ok".to_string()));
                }
                Err(e) => eprintln!("err for test {}: {}", test_name, e),
            }
        }

        println!("{}", serde_json::to_string(&edited_tests)?);

        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(about = "Create a test run")]
pub(crate) struct NewRun {
    // Test ID to run
    #[arg(short, long)]
    pub(crate) test_id: i64,
}

impl NewRun {
    pub(crate) async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let test = db.get_test(self.test_id.into()).await?;
        let instance = db.get_or_new_instance(test).await?;
        let run = db.new_run(instance).await?;

        let runner = TestRunner::new(run);

        let run = runner.run_test().await?;

        let res = if let Ok(run_display) = run.clone().try_into() {
            serde_json::to_string::<PatuiRunDisplay>(&run_display)?
        } else {
            serde_json::to_string(&run)?
        };

        println!("{}", res);

        Ok(())
    }
}
