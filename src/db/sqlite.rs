use std::path::Path;

use eyre::Result;
use tokio_rusqlite::Connection;
use tracing::{debug, trace};

use super::types::{PatuiInstance, PatuiRun, PatuiTestDb, PatuiTestHashable, PatuiTestId};
use crate::{
    types::{PatuiRunStatus, PatuiRunStep, PatuiStep, PatuiTest, PatuiTestDetails},
    utils::get_current_time_string,
};

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

                    -- Holds the audit of the test details when it was ran
                    CREATE TABLE IF NOT EXISTS instance (
                        id INTEGER PRIMARY KEY,
                        test_id INTEGER NOT NULL,
                        hash INTEGER NOT NULL,
                        name TEXT NOT NULL,
                        desc TEXT NOT NULL,
                        creation_date TEXT NOT NULL,
                        last_updated TEXT NOT NULL,
                        steps BLOB NOT NULL DEFAULT '[]',
                        FOREIGN KEY (test_id) REFERENCES test(id)
                    );

                    CREATE INDEX IF NOT EXISTS idx_instance_hash ON instance (hash);

                    -- Holds details of the run of a test instance
                    CREATE TABLE IF NOT EXISTS run (
                        id INTEGER PRIMARY KEY,
                        instance_id INTEGER NOT NULL,
                        start_time TEXT NOT NULL,
                        end_time TEXT,
                        status TEXT NOT NULL,
                        step_run_details BLOB NOT NULL DEFAULT '[]',
                        FOREIGN KEY (instance_id) REFERENCES instance(id)
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

    pub(crate) async fn get_test(&self, id: PatuiTestId) -> Result<PatuiTestDb> {
        debug!("Getting test ({})...", id);

        let test = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT id, name, desc, creation_date, last_updated, last_used_date, times_used, steps FROM test WHERE id = ?1")?;

                let test = stmt.query_row([i64::from(id)], |row| {
                    let steps = sql_decode_steps(row.get(7)?)?;

                    Ok(PatuiTestDb {
                        id,
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

    pub(crate) async fn get_tests(&self) -> Result<Vec<PatuiTestDb>> {
        debug!("Getting tests...");

        let tests = self
            .conn
            .call(move |conn| {
                let mut stmt = conn.prepare("SELECT id, name, desc, creation_date, last_updated, last_used_date, times_used, steps FROM test")?;
                let tests = stmt
                    .query_map([], |row| {
                        let steps = sql_decode_steps(row.get(7)?)?;
                        let id: i64 = row.get(0)?;
                        Ok(PatuiTestDb {
                            id: id.into(),
                            name: row.get(1)?,
                            description: row.get(2)?,
                            creation_date: row.get(3)?,
                            last_updated: row.get(4)?,
                            last_used_date: row.get(5)?,
                            times_used: row.get(6)?,
                            steps,
                        })
                    })?
                .collect::<std::result::Result<Vec<PatuiTestDb>, rusqlite::Error>>()?;

                Ok(tests)
            })
            .await?;

        Ok(tests)
    }

    pub(crate) async fn new_test(&self, details: PatuiTestDetails) -> Result<PatuiTestDb> {
        debug!("New test");
        trace!("New test details {:?}", details);

        let test_clone = details.clone();

        let test_id = self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare("INSERT INTO test (name, desc, creation_date, last_updated, last_used_date, times_used, steps) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")?;

                let test_id = stmt.insert((
                    test_clone.name,
                    test_clone.description,
                    test_clone.creation_date.clone(),
                    test_clone.creation_date,
                    None::<String>,
                    0,
                    sql_encode_steps(&test_clone.steps)?,
                ))?;

                Ok(test_id)
            })
            .await?;

        Ok(PatuiTestDb::new_from_details(test_id.into(), details))
    }

    pub(crate) async fn edit_test(&self, test: &PatuiTest) -> Result<()> {
        debug!("Edit test");
        trace!("Edit test {:?}", test);

        let test_clone = test.clone();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare("UPDATE test SET name = ?1, desc = ?2, last_updated = ?3, steps = ?4 WHERE id = ?5")?;

                let id: i64 = test_clone.id.into();

                let now = get_current_time_string();

                stmt.execute((
                    test_clone.name,
                    test_clone.description,
                    now,
                    sql_encode_steps(&test_clone.steps)?,
                    id,
                ))?;

                Ok(())
            })
            .await?;

        Ok(())
    }

    pub(crate) async fn get_or_new_instance(&self, test: PatuiTestDb) -> Result<PatuiInstance> {
        debug!("Get or new instance");
        trace!("Get or new instance details {:?}", test);

        let instance_hash = get_test_hash(&test)?;

        let instance = self.get_instance(instance_hash, (&test).into()).await?;
        if let Some(instance) = instance {
            return Ok(instance);
        }

        let instance = self.conn.call(move |conn| {
            let mut stmt = conn.prepare("INSERT INTO instance (test_id, hash, name, desc, creation_date, last_updated, steps) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")?;

            let instance_id = stmt.insert((
                i64::from(test.id),
                instance_hash,
                &test.name,
                &test.description,
                &test.creation_date,
                &test.last_updated,
                sql_encode_steps(&test.steps)?,
            ))?;

            let instance = PatuiInstance {
                id: instance_id.into(),
                test_id: test.id,
                hash: instance_hash,
                name: test.name,
                description: test.description,
                creation_date: test.creation_date,
                last_updated: test.last_updated,
                steps: test.steps,
            };

            Ok(instance)
        }).await?;

        Ok(instance)
    }

    pub(crate) async fn new_run(&self, instance: PatuiInstance) -> Result<PatuiRun> {
        let instance_id = instance.id;

        let start_time = chrono::Local::now().to_string();
        let start_time_clone = start_time.clone();

        let run_id = self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare("INSERT INTO run (instance_id, start_time, end_time, status, step_run_details) VALUES (?1, ?2, ?3, ?4, ?5)")?;

                let run_id = stmt.insert((
                    i64::from(instance_id),
                    start_time_clone,
                    None::<String>,
                    PatuiRunStatus::Pending,
                    sql_encode_step_runs(&vec![])?,
                ))?;

                Ok(run_id)
            })
            .await?;

        Ok(PatuiRun {
            id: run_id.into(),
            instance,
            start_time,
            end_time: None,
            status: PatuiRunStatus::Pending,
            step_run_details: vec![],
        })
    }

    async fn get_instance(&self, hash: i64, test: PatuiTest) -> Result<Option<PatuiInstance>> {
        let instance = self.conn.call(move |conn| {
            let mut stmt = conn.prepare("SELECT id, test_id, name, desc, creation_date, last_updated, steps FROM instance WHERE hash = ?1")?;

            let mut rows = stmt.query([hash])?;

            while let Some(row) = rows.next()? {
                let test_id: i64 = row.get(1)?;
                let steps = sql_decode_steps(row.get(6)?)?;

                let possible_test = PatuiTest {
                    id: test_id.into(),
                    name: row.get(2)?,
                    description: row.get(3)?,
                    steps: steps.clone(),
                };

                if possible_test == test {
                    let id: i64 = row.get(0)?;

                    return Ok(Some(PatuiInstance {
                        id: id.into(),
                        test_id: test_id.into(),
                        hash,
                        name: row.get(2)?,
                        description: row.get(3)?,
                        creation_date: row.get(4)?,
                        last_updated: row.get(5)?,
                        steps,
                    }));
                }
            }

            Ok(None)
        }).await?;

        Ok(instance)
    }
}

// Need a custom hash for the test to be able to do faster database lookups for test details
fn get_test_hash(test: &PatuiTestDb) -> Result<i64> {
    let hashable_test = <PatuiTestHashable>::from(test);
    trace!("Test {:#?}", hashable_test);
    let encoded_test: Vec<u8> = bincode::serialize(&hashable_test).unwrap();
    let hash = blake3::hash(encoded_test.as_slice());
    let hash: u64 = hash
        .as_bytes()
        .iter()
        .take(8)
        .fold(0, |acc, &x| acc * 256 + x as u64);

    trace!("Hash {}", hash);
    Ok(hash as i64)
}

fn sql_decode_steps(steps: String) -> std::result::Result<Vec<PatuiStep>, rusqlite::Error> {
    let ret = serde_json::from_str(&steps)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    Ok(ret)
}

fn sql_encode_steps(steps: &Vec<PatuiStep>) -> std::result::Result<String, rusqlite::Error> {
    let ret = serde_json::to_string(steps)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    Ok(ret)
}

fn sql_encode_step_runs(
    run_steps: &Vec<PatuiRunStep>,
) -> std::result::Result<String, rusqlite::Error> {
    let ret = serde_json::to_string(run_steps)
        .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
    Ok(ret)
}

#[cfg(test)]
mod tests {
    use assertor::*;
    use rusqlite::Connection;
    use tempfile::tempdir;

    use crate::types::{
        PatuiStep, PatuiStepAssertion, PatuiStepAssertionType, PatuiStepShell, PatuiTestDetails,
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
            steps: vec![],
        };

        let res = db.new_test(test).await;

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
        assert_that!(test.name).is_equal_to("test name".to_string());
        assert_that!(test.description).is_equal_to("test description".to_string());
        assert_that!(test.creation_date).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(test.last_updated).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(test.last_used_date).is_none();
        assert_that!(test.times_used).is_equal_to(0);

        let tests = db.get_tests().await.unwrap();
        assert_that!(tests).has_length(1);
        assert_that!(tests[0].name).is_equal_to("test name".to_string());
        assert_that!(tests[0].description).is_equal_to("test description".to_string());
        assert_that!(tests[0].creation_date).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(tests[0].last_updated).is_equal_to("2021-01-01 00:00:00".to_string());
        assert_that!(tests[0].last_used_date).is_none();
        assert_that!(tests[0].times_used).is_equal_to(0);
        let new_test_id: i64 = tests[0].id.into();
        assert_that!(new_test_id).is_equal_to(test_id);
    }

    #[tokio::test]
    async fn test_create_and_read_test_with_steps() {
        let (db, db_test, _tmpdir) = setup_db().await;

        let test = PatuiTestDetails {
            name: "test name".to_string(),
            description: "test description".to_string(),
            creation_date: "2021-01-01 00:00:00".to_string(),
            steps: vec![
                PatuiStep::Shell(PatuiStepShell {
                    shell: Some("bash".to_string()),
                    contents: "echo 'hello'".to_string(),
                    location: None,
                }),
                PatuiStep::Assertion(PatuiStepAssertion {
                    assertion: PatuiStepAssertionType::Equal,
                    negate: false,
                    lhs: "foo".to_string(),
                    rhs: "bar".to_string(),
                }),
            ],
        };

        let res = db.new_test(test).await;

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

        let steps: Vec<PatuiStep> =
            serde_json::from_str(&row.get::<usize, String>(2).unwrap()).unwrap();
        assert_that!(steps).has_length(2);
        assert_that!(steps.first()).is_equal_to(Some(&PatuiStep::Shell(PatuiStepShell {
            shell: Some("bash".to_string()),
            contents: "echo 'hello'".to_string(),
            location: None,
        })));
        assert_that!(steps.get(1)).is_equal_to(Some(&PatuiStep::Assertion(PatuiStepAssertion {
            assertion: PatuiStepAssertionType::Equal,
            negate: false,
            lhs: "foo".to_string(),
            rhs: "bar".to_string(),
        })));

        let row = rows.next().unwrap();
        assert_that!(row.is_none()).is_true();
    }

    // TODO: Update test
}
