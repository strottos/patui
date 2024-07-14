use std::path::Path;

use color_eyre::Result;
use phf::phf_map;
use tokio_rusqlite::Connection;
use tracing::{debug, trace};

use crate::types::{PatuiStep, PatuiStepDetails, PatuiTest};

// Never reuse or change ID's after they're in main, always add new and delete old.
static STEP_TYPE_TO_TYPE_ID: phf::Map<&'static str, i64> = phf_map! {
    "shell" => 1,
    "assertion" => 2,
};

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
                        type_id INTEGER NOT NULL,
                        FOREIGN KEY (test_id) REFERENCES test (id)
                    );

                    CREATE TABLE IF NOT EXISTS step_script (
                        id INTEGER PRIMARY KEY,
                        step_id INTEGER NOT NULL,
                        shell TEXT,
                        script TEXT,
                        location TEXT,
                        FOREIGN KEY (step_id) REFERENCES step (id)
                    );

                    CREATE TABLE IF NOT EXISTS step_assertion (
                        id INTEGER PRIMARY KEY,
                        step_id INTEGER NOT NULL,
                        type_id INTEGER NOT NULL,
                        lhs TEXT NOT NULL,
                        rhs TEXT NOT NULL,
                        FOREIGN KEY (step_id) REFERENCES step (id)
                    );
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

    pub async fn create_step(&self, step: PatuiStep) -> Result<i64> {
        debug!("Create step...");
        trace!("Create step {:?}...", step);

        let step_id = self
            .conn
            .call(move |conn| {
                let mut stmt =
                    conn.prepare("INSERT INTO step (test_id, type_id) VALUES (?1, ?2)")?;
                let step_id = stmt.insert((step.test_id, step_type_to_type_id(&step.details)))?;

                Ok(step_id)
            })
            .await?;

        Ok(step_id)
    }
}

fn step_type_to_type_id(type_: &PatuiStepDetails) -> i64 {
    match type_ {
        PatuiStepDetails::Shell(_) => *STEP_TYPE_TO_TYPE_ID.get("shell").unwrap(),
        PatuiStepDetails::Assertion(_) => *STEP_TYPE_TO_TYPE_ID.get("assertion").unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;
    use tempfile::tempdir;

    use crate::types::{PatuiStep, PatuiStepDetails, PatuiStepShell, PatuiTest};

    use super::Database;

    async fn setup_db() -> (Database, tempfile::TempDir) {
        let tmpdir = tempdir().unwrap();
        let mut db_path = tmpdir.path().to_path_buf();
        db_path.push("test.db");

        let db = Database::new(&db_path).await.unwrap();
        db.create_tables().await.unwrap();
        (db, tmpdir)
    }

    #[tokio::test]
    async fn test_create_and_read_test() {
        let (db, _tmpdir) = setup_db().await;

        let res = db
            .create_test(PatuiTest {
                id: None,
                name: "test name".to_string(),
                description: "test description".to_string(),
                creation_date: "2021-01-01 00:00:00".to_string(),
                last_updated: "2021-01-01 00:00:00".to_string(),
                last_used_date: None,
                times_used: 0,
                steps: vec![],
            })
            .await;

        assert_that!(res).is_ok();
        let test_id = res.unwrap();
        assert_that!(test_id).is_greater_than(0);

        let test = db.get_test(test_id).await.unwrap();
        assert_that!(test.name).is_equal_to("test name".to_string());
        assert_that!(test.description).is_equal_to("test description".to_string());
        assert_that!(test.creation_date).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(test.last_updated).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(test.last_used_date).is_none();
        assert_that!(test.times_used).is_equal_to(0);
        assert_that!(test.id).is_equal_to(Some(test_id));

        let tests = db.get_tests().await.unwrap();
        assert_that!(tests).has_length(1);
        assert_that!(tests[0].name).is_equal_to("test name".to_string());
        assert_that!(tests[0].description).is_equal_to("test description".to_string());
        assert_that!(tests[0].creation_date).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(tests[0].last_updated).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(tests[0].last_used_date).is_none();
        assert_that!(tests[0].times_used).is_equal_to(0);
        assert_that!(tests[0].id).is_equal_to(Some(test_id));
    }

    #[tokio::test]
    async fn test_create_and_read_test_with_shell_step() {
        let (db, _tmpdir) = setup_db().await;

        let res = db
            .create_test(PatuiTest {
                id: None,
                name: "test name".to_string(),
                description: "test description".to_string(),
                creation_date: "2021-01-01 00:00:00".to_string(),
                last_updated: "2021-01-01 00:00:00".to_string(),
                last_used_date: None,
                times_used: 0,
                steps: vec![],
            })
            .await;

        assert_that!(res).is_ok();
        let test_id = res.unwrap();
        assert_that!(test_id).is_greater_than(0);

        let res = db
            .create_step(PatuiStep {
                id: None,
                test_id,
                details: PatuiStepDetails::Shell(PatuiStepShell {
                    shell: None,
                    text: "echo 'hello'".to_string(),
                }),
            })
            .await;

        assert_that!(res).is_ok();
        let step_id = res.unwrap();
        assert_that!(step_id).is_greater_than(0);
    }
}
