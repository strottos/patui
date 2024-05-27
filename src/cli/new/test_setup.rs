use std::{io::Write, sync::Arc};

use chrono::{DateTime, Local};
use clap::Parser;
use color_eyre::Result;

use crate::{
    db::Database,
    types::{InsertTestStatus, PatuiTest},
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
