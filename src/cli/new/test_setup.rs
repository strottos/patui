use std::{io::Write, sync::Arc};

use chrono::{DateTime, Local};
use clap::Parser;
use color_eyre::{eyre::bail, Result};

use crate::{
    db::Database,
    types::{
        InsertTestStatus, PatuiStep, PatuiStepAssertion, PatuiStepAssertionType, PatuiStepDetails,
        PatuiStepShell, PatuiTest,
    },
};

#[derive(Parser, Debug)]
#[command(about = "Create a new test")]
pub struct NewTest {
    #[clap(short, long)]
    pub name: String,

    #[clap(short, long)]
    pub description: String,
}

impl NewTest {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let now: DateTime<Local> = Local::now();

        let id = db
            .create_test(PatuiTest {
                id: None,
                name: self.name.clone(),
                description: self.description.clone(),
                creation_date: now.format("%Y-%m-%d %H:%M:%S").to_string(),
                last_updated: now.format("%Y-%m-%d %H:%M:%S").to_string(),
                last_used_date: None,
                times_used: 0,
                steps: vec![],
            })
            .await?;

        let output_status = InsertTestStatus {
            id,
            status: "ok".to_string(),
        };
        std::io::stdout().write_all(&serde_json::to_vec(&output_status)?)?;
        std::io::stdout().write_all(b"\n")?;

        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(about = "Create a new step for a given test")]
pub struct NewStep {
    #[clap(short, long)]
    pub test_id: i64,

    #[command(subcommand)]
    command: NewStepType,
}

impl NewStep {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        match &self.command {
            NewStepType::Shell(step) => step.handle(self.test_id, db).await?,
            NewStepType::Assertion(step) => step.handle(self.test_id, db).await?,
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
pub enum NewStepType {
    Shell(NewStepShell),
    Assertion(NewStepAssertion),
}

#[derive(Parser, Debug)]
#[command(about = "Create a new step for running a shell")]
pub struct NewStepShell {
    #[clap(short, long)]
    pub shell: Option<String>,

    #[clap(short, long)]
    pub location: Option<String>,

    #[clap(short, long)]
    pub contents: String,
}

impl NewStepShell {
    pub async fn handle(&self, test_id: i64, db: Arc<Database>) -> Result<()> {
        db.create_step(PatuiStep {
            id: None,
            test_id,
            details: PatuiStepDetails::Shell(PatuiStepShell {
                shell: self.shell.clone(),
                contents: self.contents.clone(),
                location: self.location.clone(),
            }),
        })
        .await?;
        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(about = "Create a new step for doing an assertion on something")]
pub struct NewStepAssertion {
    #[clap(short, long)]
    pub type_: String,

    #[clap(short, long)]
    pub lhs: String,

    #[clap(short, long)]
    pub rhs: String,
}

impl NewStepAssertion {
    pub async fn handle(&self, test_id: i64, db: Arc<Database>) -> Result<()> {
        db.create_step(PatuiStep {
            id: None,
            test_id,
            details: PatuiStepDetails::Assertion(PatuiStepAssertion {
                assertion: match &self.type_[..] {
                    "equal" => PatuiStepAssertionType::Equal,
                    "contains" => PatuiStepAssertionType::Contains,
                    _ => bail!("Invalid assertion type"),
                },
                negate: false,
                lhs: self.lhs.clone(),
                rhs: self.rhs.clone(),
            }),
        })
        .await?;

        Ok(())
    }
}
