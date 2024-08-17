use std::sync::Arc;

use clap::{Args, Parser};
use color_eyre::Result;

use crate::{
    db::Database,
    types::{PatuiStepDetails, PatuiStepShell},
};

use super::resources;

#[derive(clap::ValueEnum, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[clap(rename_all = "lower")]
pub enum Templates {
    Default,
}

fn get_template(template: Templates) -> Result<resources::EditTest> {
    match template {
        Templates::Default => Ok(resources::EditTest {
            id: None,
            name: Some("Default".to_string()),
            description: Some("Default template".to_string()),
            steps: Some(vec![PatuiStepDetails::Shell(PatuiStepShell {
                shell: Some("bash".to_string()),
                contents: "echo 'Hello, world!'".to_string(),
                location: None,
            })]),
        }),
    }
}

#[derive(Debug, Args)]
#[command(about = "Create new entity")]
pub struct Command {
    #[command(subcommand)]
    command: NewCommand,
}

impl Command {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        match &self.command {
            NewCommand::Test(new_test) => new_test.handle(db).await,
        }
    }
}

#[derive(Parser, Debug)]
pub enum NewCommand {
    Test(NewTest),
}

#[derive(Parser, Debug)]
#[command(about = "Edit an existing test")]
pub struct NewTest {
    #[arg(short, long)]
    pub name: Option<String>,

    #[arg(short, long)]
    pub description: Option<String>,

    #[arg(short, long)]
    pub template: Option<Templates>,
}

impl NewTest {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let edit_test = match self.template {
            Some(template) => get_template(template)?,
            None => resources::EditTest {
                id: None,
                name: self.name.clone(),
                description: self.description.clone(),
                steps: Some(vec![]),
            },
        };

        edit_test.handle(db).await?;

        Ok(())
    }
}
