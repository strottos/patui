use std::path::Path;

use eyre::Result;
use tokio_rusqlite::Connection;
use tracing::{debug, trace};

use crate::types::{PatuiStepDetails, PatuiTest, PatuiTestDetails, PatuiTestId};

#[derive(Debug, Clone)]
pub(crate) struct Database {
    conn: Connection,
}

impl Database {
    pub(crate) async fn new(path: &Path) -> Result<Self> {
        let conn = Connection::open(path).await?;

        Ok(Self { conn })
    }

    pub(crate) async fn create_tables(&self) -> Result<bool> {
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
                        times_used INTEGER NOT NULL DEFAULT 0,
                        steps BLOB NOT NULL DEFAULT '[]'
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

    pub(crate) async fn get_test(&self, id: PatuiTestId) -> Result<PatuiTest> {
        debug!("Getting test ({})...", id);

        let test = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT id, name, desc, creation_date, last_updated, last_used_date, times_used, steps FROM test WHERE id = ?1")?;

                let test = stmt.query_row([i64::from(id)], |row| {
                    let steps: String = row.get(7)?;
                    let steps: Vec<PatuiStepDetails> = serde_json::from_str(&steps).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                    Ok(PatuiTest {
                        id,
                        details: PatuiTestDetails {
                            name: row.get(1)?,
                            description: row.get(2)?,
                            creation_date: row.get(3)?,
                            last_updated: row.get(4)?,
                            last_used_date: row.get(5)?,
                            times_used: row.get(6)?,
                            steps,
                        }
                    })
                })?;

                Ok(test)
            })
            .await?;

        Ok(test)
    }

    pub(crate) async fn get_tests(&self) -> Result<Vec<PatuiTest>> {
        debug!("Getting tests...");

        let tests = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT id, name, desc, creation_date, last_updated, last_used_date, times_used, steps FROM test")?;
                let tests = stmt
                    .query_map([], |row| {
                        let steps: String = row.get(7)?;
                        let steps: Vec<PatuiStepDetails> = serde_json::from_str(&steps).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                        let id: i64 = row.get(0)?;
                        Ok(PatuiTest {
                            id: id.into(),
                            details: PatuiTestDetails {
                                name: row.get(1)?,
                                description: row.get(2)?,
                                creation_date: row.get(3)?,
                                last_updated: row.get(4)?,
                                last_used_date: row.get(5)?,
                                times_used: row.get(6)?,
                                steps,
                            }
                        })
                    })?
                .collect::<std::result::Result<Vec<PatuiTest>, rusqlite::Error>>()?;

                Ok(tests)
            })
            .await?;

        Ok(tests)
    }

    pub(crate) async fn new_test(&self, details: &PatuiTestDetails) -> Result<PatuiTest> {
        debug!("New test");
        trace!("New test details {:?}", details);

        let test_clone = details.clone();

        let test_id = self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare("INSERT INTO test (name, desc, creation_date, last_updated, last_used_date, times_used, steps) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")?;

                let test_id = stmt.insert((
                    test_clone.name.clone(),
                    test_clone.description.clone(),
                    test_clone.creation_date.clone(),
                    test_clone.last_updated.clone(),
                    test_clone.last_used_date.clone(),
                    test_clone.times_used.clone(),
                    serde_json::to_string(&test_clone.steps)
                        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                ))?;

                Ok(test_id)
            })
            .await?;

        Ok(PatuiTest {
            id: test_id.into(),
            details: details.clone(),
        })
    }

    pub(crate) async fn edit_test(&self, test: &PatuiTest) -> Result<()> {
        debug!("Edit test");
        trace!("Edit test {:?}", test);

        let test_clone = test.clone();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare("UPDATE test SET name = ?1, desc = ?2, creation_date = ?3, last_updated = ?4, last_used_date = ?5, times_used = ?6, steps = ?7 WHERE id = ?8")?;

                let id: i64 = test_clone.id.into();

                stmt.execute((
                    test_clone.details.name,
                    test_clone.details.description,
                    test_clone.details.creation_date,
                    test_clone.details.last_updated,
                    test_clone.details.last_used_date,
                    test_clone.details.times_used,
                    serde_json::to_string(&test_clone.details.steps).map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?,
                    id,
                ))?;

                Ok(())
            })
            .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    use crate::types::{
        PatuiStepAssertion, PatuiStepAssertionType, PatuiStepDetails, PatuiStepShell,
        PatuiTestDetails,
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

        let test = PatuiTestDetails {
            name: "test name".to_string(),
            description: "test description".to_string(),
            creation_date: "2021-01-01 00:00:00".to_string(),
            last_updated: "2021-01-01 00:00:00".to_string(),
            last_used_date: None,
            times_used: 0,
            steps: vec![],
        };

        let res = db.new_test(&test).await;

        assert_that!(res).is_ok();
        let test = res.unwrap();
        let test_id: i64 = test.id.into();
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

        let test = db.get_test(test_id.into()).await.unwrap();
        assert_that!(test.details.name).is_equal_to("test name".to_string());
        assert_that!(test.details.description).is_equal_to("test description".to_string());
        assert_that!(test.details.creation_date).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(test.details.last_updated).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(test.details.last_used_date).is_none();
        assert_that!(test.details.times_used).is_equal_to(0);

        let tests = db.get_tests().await.unwrap();
        assert_that!(tests).has_length(1);
        assert_that!(tests[0].details.name).is_equal_to("test name".to_string());
        assert_that!(tests[0].details.description).is_equal_to("test description".to_string());
        assert_that!(tests[0].details.creation_date).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(tests[0].details.last_updated).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(tests[0].details.last_used_date).is_none();
        assert_that!(tests[0].details.times_used).is_equal_to(0);
        let new_test_id: i64 = tests[0].id.clone().into();
        assert_that!(new_test_id).is_equal_to(test_id);
    }

    #[tokio::test]
    async fn test_create_and_read_test_with_steps() {
        let (db, db_test, _tmpdir) = setup_db().await;

        let test = PatuiTestDetails {
            name: "test name".to_string(),
            description: "test description".to_string(),
            creation_date: "2021-01-01 00:00:00".to_string(),
            last_updated: "2021-01-01 00:00:00".to_string(),
            last_used_date: None,
            times_used: 0,
            steps: vec![
                PatuiStepDetails::Shell(PatuiStepShell {
                    shell: Some("bash".to_string()),
                    contents: "echo 'hello'".to_string(),
                    location: None,
                }),
                PatuiStepDetails::Assertion(PatuiStepAssertion {
                    assertion: PatuiStepAssertionType::Equal,
                    negate: false,
                    lhs: "foo".to_string(),
                    rhs: "bar".to_string(),
                }),
            ],
        };

        let res = db.new_test(&test).await;

        assert_that!(res).is_ok();
        let test = res.unwrap();
        let test_id: i64 = test.id.into();
        assert_that!(test_id).is_greater_than(0);

        // Check it went in the DB
        let mut stmt = db_test
            .prepare("SELECT name, desc, steps FROM test WHERE id = ?1")
            .unwrap();
        let mut rows = stmt.query(rusqlite::params![test_id]).unwrap();

        // Check first step
        let row = rows.next();
        assert_that!(row).is_ok();
        assert_that!(row.as_ref().unwrap().is_some()).is_true();
        let row = row.unwrap().unwrap();

        let steps: Vec<PatuiStepDetails> =
            serde_json::from_str(&row.get::<usize, String>(2).unwrap()).unwrap();
        assert_that!(steps).has_length(2);
        assert_that!(steps.first()).is_equal_to(Some(&PatuiStepDetails::Shell(PatuiStepShell {
            shell: Some("bash".to_string()),
            contents: "echo 'hello'".to_string(),
            location: None,
        })));
        assert_that!(steps.get(1)).is_equal_to(Some(&PatuiStepDetails::Assertion(
            PatuiStepAssertion {
                assertion: PatuiStepAssertionType::Equal,
                negate: false,
                lhs: "foo".to_string(),
                rhs: "bar".to_string(),
            },
        )));

        let row = rows.next().unwrap();
        assert_that!(row.is_none()).is_true();
    }

    // TODO: Update test
}
