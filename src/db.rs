use std::path::Path;

use color_eyre::Result;
use strum::{VariantArray, VariantNames};
use tokio_rusqlite::Connection;
use tracing::{debug, trace};

use crate::types::{
    PatuiStep, PatuiStepAssertion, PatuiStepAssertionType, PatuiStepDetails, PatuiStepShell,
    PatuiTest,
};

// Never change these values, they are used in the database schema
fn step_type_to_type_id(type_: &'static str) -> i64 {
    PatuiStepDetails::VARIANTS
        .iter()
        .position(|v| *v == type_)
        .map(|v| v as i64)
        .unwrap()
}

fn step_type_id_to_string(id: i64) -> &'static str {
    PatuiStepDetails::VARIANTS
        .iter()
        .enumerate()
        .find(|(i, _)| *i as i64 == id)
        .unwrap()
        .1
}

fn assertion_type_to_db_type_id(type_: &PatuiStepAssertionType) -> i64 {
    PatuiStepAssertionType::VARIANTS
        .iter()
        .position(|v| v == type_)
        .map(|v| v as i64)
        .unwrap()
}

#[derive(Debug, Clone)]
pub struct Database {
    conn: Connection,
}

impl Database {
    pub async fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).await?;

        Ok(Self { conn })
    }

    pub async fn create_tables(&self) -> Result<bool> {
        debug!("Creating tables...");

        let ret = self
            .conn
            .call(|conn| {
                conn.execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS setup (
                        id INTEGER PRIMARY KEY
                    );

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

                    CREATE TABLE IF NOT EXISTS step_shell (
                        id INTEGER PRIMARY KEY,
                        step_id INTEGER NOT NULL,
                        shell TEXT,
                        contents TEXT NOT NULL,
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

                let mut stmt = conn.prepare(
                    "INSERT INTO setup (id) SELECT 1 WHERE NOT EXISTS(SELECT 1 FROM setup);",
                )?;
                let id = stmt.insert(());

                Ok(matches!(id, Ok(1)))
            })
            .await?;

        Ok(ret)
    }

    pub async fn get_test(&self, id: i64) -> Result<PatuiTest> {
        debug!("Getting test ({})...", id);

        let test = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT id, name, desc, creation_date, last_updated, last_used_date, times_used FROM test WHERE id = ?1")?;

                let steps = Self::_get_steps(conn, id)?;

                let test = stmt.query_row([id], |row| {
                    Ok(PatuiTest {
                        id: Some(row.get(0)?),
                        name: row.get(1)?,
                        description: row.get(2)?,
                        creation_date: row.get(3)?,
                        last_updated: row.get(4)?,
                        last_used_date: row.get(5)?,
                        times_used: row.get(6)?,
                        steps,
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

    pub async fn get_step(&self, test_id: i64, step_id: i64) -> Result<PatuiStep> {
        debug!("Getting step ({})...", step_id);

        let step = self.conn.call(move |conn| {
            let mut stmt = conn.prepare("SELECT id, test_id, type_id FROM step WHERE id = ?1 AND test_id = ?2")?;
            let step = stmt.query_row([step_id, test_id], |row| {
                let step_id = row.get(0)?;
                let test_id = row.get(1)?;
                let type_id = row.get(2)?;

                let details = match step_type_id_to_string(type_id) {
                    "shell" => {
                        let mut stmt = conn.prepare("SELECT shell, contents, location FROM step_shell WHERE step_id = ?1")?;
                        let shell = stmt.query_row([step_id], |row| {
                            Ok(PatuiStepShell {
                                shell: row.get(0)?,
                                contents: row.get(1)?,
                                location: row.get(2)?,
                            })
                        })?;

                        PatuiStepDetails::Shell(shell)
                    }
                    "assertion" => {
                        let mut stmt = conn.prepare("SELECT type_id, lhs, rhs FROM step_assertion WHERE step_id = ?1")?;
                        let assertion = stmt.query_row([step_id], |row| {
                            let assertion: i64 = row.get(0)?;
                            Ok(PatuiStepAssertion {
                                assertion: PatuiStepAssertionType::VARIANTS[assertion as usize].clone(),
                                negate: false,
                                lhs: row.get(1)?,
                                rhs: row.get(2)?,
                            })
                        })?;

                        PatuiStepDetails::Assertion(assertion)
                    }
                    _ => panic!("Unknown step type"),
                };

                Ok(PatuiStep {
                    id: Some(step_id),
                    test_id,
                    details,
                })
            })?;

            Ok(step)
        }).await?;

        Ok(step)
    }

    pub async fn get_steps(&self, test_id: i64) -> Result<Vec<PatuiStep>> {
        debug!("Getting tests...");

        let steps = self
            .conn
            .call(move |conn| Self::_get_steps(conn, test_id))
            .await?;

        Ok(steps)
    }

    pub fn _get_steps(
        conn: &rusqlite::Connection,
        test_id: i64,
    ) -> std::result::Result<Vec<PatuiStep>, tokio_rusqlite::Error> {
        let mut stmt = conn.prepare("SELECT id, test_id, type_id FROM step WHERE test_id = ?1")?;
        let steps = stmt
            .query_map([test_id], |row| {
                let step_id: i64 = row.get(0)?;
                let test_id = row.get(1)?;
                let type_id = row.get(2)?;

                let details = match step_type_id_to_string(type_id) {
                    "shell" => {
                        let mut stmt = conn.prepare(
                            "SELECT shell, contents, location FROM step_shell WHERE step_id = ?1",
                        )?;
                        let shell = stmt.query_row([step_id], |row| {
                            Ok(PatuiStepShell {
                                shell: row.get(0)?,
                                contents: row.get(1)?,
                                location: row.get(2)?,
                            })
                        })?;

                        PatuiStepDetails::Shell(shell)
                    }
                    "assertion" => {
                        let mut stmt = conn.prepare(
                            "SELECT type_id, lhs, rhs FROM step_assertion WHERE step_id = ?1",
                        )?;
                        let assertion = stmt.query_row([step_id], |row| {
                            let assertion: i64 = row.get(0)?;
                            Ok(PatuiStepAssertion {
                                assertion: PatuiStepAssertionType::VARIANTS[assertion as usize]
                                    .clone(),
                                negate: false,
                                lhs: row.get(1)?,
                                rhs: row.get(2)?,
                            })
                        })?;

                        PatuiStepDetails::Assertion(assertion)
                    }
                    _ => panic!("Unknown step type"),
                };

                Ok(PatuiStep {
                    id: Some(row.get(0)?),
                    test_id,
                    details,
                })
            })?
            .collect::<std::result::Result<Vec<PatuiStep>, rusqlite::Error>>()?;

        Ok(steps)
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

    pub async fn update_test(&self, test: PatuiTest) -> Result<i64> {
        debug!("Update test...");
        trace!("Update test {:?}...", test);

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
                let details = &step.details;

                let mut stmt =
                    conn.prepare("INSERT INTO step (test_id, type_id) VALUES (?1, ?2)")?;
                let step_id =
                    stmt.insert((step.test_id, step_type_to_type_id(details.to_str())))?;

                match details {
                    PatuiStepDetails::Shell(shell) => {
                        let mut stmt = conn.prepare("INSERT INTO step_shell (step_id, shell, contents, location) VALUES (?1, ?2, ?3, ?4)")?;
                        stmt.insert((
                            step_id,
                            shell.shell.clone(),
                            shell.contents.clone(),
                            shell.location.clone(),
                        ))?;
                    }
                    PatuiStepDetails::Assertion(assertion) => {
                        let mut stmt = conn.prepare("INSERT INTO step_assertion (step_id, type_id, lhs, rhs) VALUES (?1, ?2, ?3, ?4)")?;
                        stmt.insert((
                            step_id,
                            assertion_type_to_db_type_id(&assertion.assertion),
                            assertion.lhs.clone(),
                            assertion.rhs.clone(),
                        ))?;
                    }
                }

                Ok(step_id)
            })
            .await?;

        Ok(step_id)
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    use crate::{
        db::{assertion_type_to_db_type_id, step_type_to_type_id},
        types::{
            PatuiStep, PatuiStepAssertion, PatuiStepAssertionType, PatuiStepDetails,
            PatuiStepShell, PatuiTest,
        },
    };

    use super::Database;

    async fn setup_db() -> (Database, Connection, tempfile::TempDir) {
        let tmpdir = tempdir().unwrap();
        let mut db_path = tmpdir.path().to_path_buf();
        db_path.push("test.db");

        let db = Database::new(&db_path).await.unwrap();
        db.create_tables().await.unwrap();

        let db_test = Connection::open(db_path).unwrap();

        (db, db_test, tmpdir)
    }

    #[tokio::test]
    async fn test_create_and_read_test() {
        let (db, db_test, _tmpdir) = setup_db().await;

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

        // Check it went in the DB
        let mut stmt = db_test
            .prepare("SELECT name, desc FROM test WHERE id = ?1")
            .unwrap();
        let mut rows = stmt.query(rusqlite::params![test_id]).unwrap();
        let row = rows.next();
        assert_that!(row).is_ok();
        assert_that!(row.as_ref().unwrap().is_some()).is_true();
        let row = row.unwrap().unwrap();
        assert_that!(row.get(0)).is_equal_to(Ok("test name".to_string()));
        assert_that!(row.get(1)).is_equal_to(Ok("test description".to_string()));
        let row = rows.next().unwrap();
        assert_that!(row.is_none()).is_true();

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
    async fn test_create_and_read_test_with_steps() {
        let (db, db_test, _tmpdir) = setup_db().await;

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
                    contents: "echo 'hello'".to_string(),
                    location: None,
                }),
            })
            .await;

        assert_that!(res).is_ok();
        let step_id = res.unwrap();
        assert_that!(step_id).is_greater_than(0);

        // Check it went in the DB
        let mut stmt = db_test
            .prepare("SELECT test_id, type_id FROM step WHERE id = ?1")
            .unwrap();
        let mut rows = stmt.query(rusqlite::params![step_id]).unwrap();
        let row = rows.next();
        assert_that!(row).is_ok();
        assert_that!(row.as_ref().unwrap().is_some()).is_true();
        let row = row.unwrap().unwrap();
        assert_that!(row.get(0)).is_equal_to(Ok(test_id));
        assert_that!(row.get(1)).is_equal_to(Ok(step_type_to_type_id("shell")));
        let row = rows.next().unwrap();
        assert_that!(row.is_none()).is_true();

        let mut stmt = db_test
            .prepare("SELECT shell, contents, location FROM step_shell WHERE step_id = ?1")
            .unwrap();
        let mut rows = stmt.query(rusqlite::params![step_id]).unwrap();
        let row = rows.next();
        assert_that!(row).is_ok();
        assert_that!(row.as_ref().unwrap().is_some()).is_true();
        let row = row.unwrap().unwrap();
        assert_that!(row.get::<usize, Option<String>>(0)).is_equal_to(Ok(None));
        assert_that!(row.get::<usize, Option<String>>(1))
            .is_equal_to(Ok(Some("echo 'hello'".to_string())));
        assert_that!(row.get::<usize, Option<String>>(2)).is_equal_to(Ok(None));
        let row = rows.next().unwrap();
        assert_that!(row.is_none()).is_true();

        let res = db
            .create_step(PatuiStep {
                id: None,
                test_id,
                details: PatuiStepDetails::Assertion(PatuiStepAssertion {
                    assertion: PatuiStepAssertionType::Equal,
                    negate: false,
                    lhs: "foo".to_string(),
                    rhs: "bar".to_string(),
                }),
            })
            .await;

        assert_that!(res).is_ok();
        let previous_step_id = step_id;
        let step_id = res.unwrap();
        assert_that!(step_id).is_greater_than(previous_step_id);

        // Check it went in the DB
        let mut stmt = db_test
            .prepare("SELECT test_id, type_id FROM step WHERE id = ?1")
            .unwrap();
        let mut rows = stmt.query(rusqlite::params![step_id]).unwrap();
        let row = rows.next();
        assert_that!(row).is_ok();
        assert_that!(row.as_ref().unwrap().is_some()).is_true();
        let row = row.unwrap().unwrap();
        assert_that!(row.get(0)).is_equal_to(Ok(test_id));
        assert_that!(row.get(1)).is_equal_to(Ok(step_type_to_type_id("assertion")));
        let row = rows.next().unwrap();
        assert_that!(row.is_none()).is_true();

        let mut stmt = db_test
            .prepare("SELECT type_id, lhs, rhs FROM step_assertion WHERE step_id = ?1")
            .unwrap();
        let mut rows = stmt.query(rusqlite::params![step_id]).unwrap();
        let row = rows.next();
        assert_that!(row).is_ok();
        assert_that!(row.as_ref().unwrap().is_some()).is_true();
        let row = row.unwrap().unwrap();
        assert_that!(row.get(0)).is_equal_to(Ok(Some(assertion_type_to_db_type_id(
            &PatuiStepAssertionType::Equal,
        ))));
        assert_that!(row.get(1)).is_equal_to(Ok(Some("foo".to_string())));
        assert_that!(row.get(2)).is_equal_to(Ok(Some("bar".to_string())));
        let row = rows.next().unwrap();
        assert_that!(row.is_none()).is_true();

        // Select them back out again and check the results are right
        let step = db.get_step(test_id, step_id).await.unwrap();
        assert_that!(step.id).is_equal_to(Some(step_id));
        assert_that!(step.test_id).is_equal_to(test_id);
        assert_that!(step.details).is_equal_to(PatuiStepDetails::Assertion(PatuiStepAssertion {
            assertion: PatuiStepAssertionType::Equal,
            negate: false,
            lhs: "foo".to_string(),
            rhs: "bar".to_string(),
        }));

        let steps = db.get_steps(test_id).await.unwrap();
        assert_that!(steps).has_length(2);
        assert_that!(steps[0].id).is_equal_to(Some(previous_step_id));
        assert_that!(steps[0].test_id).is_equal_to(test_id);
        assert_that!(steps[0].details).is_equal_to(PatuiStepDetails::Shell(PatuiStepShell {
            shell: None,
            contents: "echo 'hello'".to_string(),
            location: None,
        }));

        assert_that!(steps[1].id).is_equal_to(Some(step_id));
        assert_that!(steps[1].test_id).is_equal_to(test_id);
        assert_that!(steps[1].details).is_equal_to(PatuiStepDetails::Assertion(
            PatuiStepAssertion {
                assertion: PatuiStepAssertionType::Equal,
                negate: false,
                lhs: "foo".to_string(),
                rhs: "bar".to_string(),
            },
        ));
    }
}
