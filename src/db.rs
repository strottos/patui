use std::path::Path;

use color_eyre::Result;
use tokio_rusqlite::Connection;
use tracing::{debug, trace};

use crate::types::PatuiTest;

#[derive(Debug, Clone)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).await?;

        Ok(Self { conn })
    }

    pub async fn create_tables(&self) -> Result<()> {
        debug!("Creating tables...");

        self.conn
            .call(|conn| {
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS test (
                        id INTEGER PRIMARY KEY,
                        name TEXT NOT NULL,
                        desc TEXT NOT NULL,
                        creation_date TEXT NOT NULL,
                        last_updated TEXT NOT NULL,
                        last_used_date TEXT,
                        times_used INTEGER NOT NULL DEFAULT 0
                    );

                    CREATE TABLE IF NOT EXISTS step (
                        id INTEGER PRIMARY KEY,
                        test_id INTEGER NOT NULL,
                        FOREIGN KEY (test_id) REFERENCES test (id)
                    )
                    "#,
                )?;

                Ok(())
            })
            .await?;

        Ok(())
    }

    pub async fn get_test(&self, id: i64) -> Result<PatuiTest> {
        debug!("Getting test ({})...", id);

        let test = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT id, name, desc, creation_date, last_updated, last_used_date, times_used FROM test WHERE id = ?1")?;
                let test = stmt.query_row([id], |row| {
                    Ok(PatuiTest {
                        id: Some(row.get(0)?),
                        name: row.get(1)?,
                        description: row.get(2)?,
                        creation_date: row.get(3)?,
                        last_updated: row.get(4)?,
                        last_used_date: row.get(5)?,
                        times_used: row.get(6)?,
                        steps: vec![],
                    })
                })?;

                Ok(test)
            })
            .await?;

        Ok(test)
    }

    pub async fn get_tests(&self) -> Result<Vec<PatuiTest>> {
        debug!("Getting tests...");

        let tests = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT id, name, desc, creation_date, last_updated, last_used_date, times_used FROM test")?;
                let tests = stmt
                    .query_map([], |row| {
                        Ok(PatuiTest {
                            id: Some(row.get(0)?),
                            name: row.get(1)?,
                            description: row.get(2)?,
                            creation_date: row.get(3)?,
                            last_updated: row.get(4)?,
                            last_used_date: row.get(5)?,
                            times_used: row.get(6)?,
                            steps: vec![],
                        })
                    })?
                .collect::<std::result::Result<Vec<PatuiTest>, rusqlite::Error>>()?;

                Ok(tests)
            })
            .await?;

        Ok(tests)
    }

    pub async fn create_test(&self, test: PatuiTest) -> Result<i64> {
        debug!("Create test...");
        trace!("Create test {:?}...", test);

        let test_id = self.conn.call(move |conn| {
            let mut stmt = conn.prepare("INSERT INTO test (name, desc, creation_date, last_updated, last_used_date, times_used) VALUES (?1, ?2, ?3, ?4, ?5, ?6)")?;
            let test_id = stmt.insert((
                test.name.clone(),
                test.description.clone(),
                test.creation_date.clone(),
                test.last_updated.clone(),
                test.last_used_date.clone(),
                test.times_used,
            ))?;

            Ok(test_id)
        }).await?;

        Ok(test_id)
    }
}
