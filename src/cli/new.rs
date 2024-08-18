use std::{io::Read, sync::Arc};

use clap::{Args, Parser};
use color_eyre::Result;

use crate::{
    db::Database,
    types::{PatuiStepDetails, PatuiStepShell, PatuiTest},
};

#[derive(clap::ValueEnum, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[clap(rename_all = "lower")]
pub(crate) enum Templates {
    Default,
}

fn get_template(template: Templates) -> Result<PatuiTest> {
    let now = chrono::Local::now().to_string();

    match template {
        Templates::Default => Ok(PatuiTest {
            id: None,
            name: "Default".to_string(),
            description: "Default template".to_string(),
            creation_date: now.clone(),
            last_updated: now,
            last_used_date: None,
            times_used: 0,
            steps: vec![PatuiStepDetails::Shell(PatuiStepShell {
                shell: Some("bash".to_string()),
                contents: "echo 'Hello, world!'".to_string(),
                location: None,
            })],
        }),
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
        }
    }
}

#[derive(Parser, Debug)]
pub(crate) enum NewCommand {
    Test(NewTest),
}

#[derive(Parser, Debug)]
#[command(about = "Edit an existing test")]
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
        let mut tests = vec![];

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
                PatuiTest::from_yaml_str(&contents)?
            } else {
                PatuiTest::edit_yaml(contents)?
            };
            tests.push(test);
        }

        if tests.is_empty() {
            let template = if let Some(template) = self.template {
                get_template(template)?
            } else {
                get_template(Templates::Default)?
            };

            if self.no_edit {
                tests.push(template);
            } else {
                let yaml_str = template.to_editable_yaml_string()?;
                let test = PatuiTest::edit_yaml(yaml_str)?;
                tests.push(test);
            }
        }

        if tests.is_empty() {
            eprintln!("No tests to create, remove --no-edit or provide valid files");
            std::process::exit(1);
        }

        let mut edited_tests = vec![];

        for test in tests.iter_mut() {
            match db.edit_test(test).await {
                Ok(_) => edited_tests.push(test.to_edited_test("ok".to_string())),
                Err(e) => edited_tests.push(test.to_edited_test(format!("err: {}", e))),
            }
            edited_tests.push(test.to_edited_test("ok".to_string()));
        }

        println!("{}", serde_json::to_string(&edited_tests)?);

        Ok(())
    }
}
