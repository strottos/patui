use std::{io::Write, sync::Arc};

use chrono::Local;
use color_eyre::Result;
use edit::edit;

use crate::{
    db::Database,
    types::{InsertTestStatus, PatuiStepDetails, PatuiTest},
};

#[derive(Debug)]
pub struct EditTest {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub steps: Option<Vec<PatuiStepDetails>>,
}

impl EditTest {
    pub async fn handle(&self, db: Arc<Database>) -> Result<()> {
        let now = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        let mut test = match self.id {
            Some(id) => db.get_test(id).await?,
            None => {
                let name = self
                    .name
                    .clone()
                    .unwrap_or_else(|| "to_replace".to_string());
                let description = self.description.clone().unwrap_or_default();
                let steps = self.steps.clone().unwrap_or_default();

                PatuiTest {
                    id: None,
                    name,
                    description,
                    creation_date: now.clone(),
                    last_updated: now.clone(),
                    last_used_date: None,
                    times_used: 0,
                    steps,
                }
            }
        };

        if self.name.is_none() {
            let template = test.to_editable_yaml_string()?;
            let edited_template = edit(template.as_bytes())?;
            test.edit_with_yaml(&edited_template)?;
        }

        let id = db.edit_test(test).await?;
        let output_status = InsertTestStatus {
            id,
            status: "ok".to_string(),
        };
        std::io::stdout().write_all(&serde_json::to_vec(&output_status)?)?;
        std::io::stdout().write_all(b"\n")?;

        Ok(())
    }
}
