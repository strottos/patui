use std::process::Output;

use assert_cmd::Command;
use serde::Deserialize;
use tempfile::tempdir;

#[derive(Debug, Deserialize)]
struct InsertOutput {
    id: i64,
    status: String,
}

#[derive(Debug, Deserialize)]
pub struct PatuiStep {}

#[derive(Debug, Deserialize)]
struct PatuiTest {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub creation_date: String,
    pub last_updated: String,
    pub last_used_date: Option<String>,
    pub times_used: u32,
    pub steps: Vec<PatuiStep>,
}

fn run_patui(args: &[&str]) -> Output {
    let mut cmd = Command::cargo_bin("patui").unwrap();
    let output = cmd.args(args).ok().unwrap();
    output
}

#[test]
fn test_new_test() {
    let tmpdir = tempdir().unwrap();
    let mut db_path = tmpdir.path().to_path_buf();
    db_path.push("test.db");

    let output = run_patui(&[
        "--db",
        db_path.to_str().unwrap(),
        "new",
        "test",
        "--name",
        "test name",
        "--description",
        "test description",
    ]);

    let success = output.status.success();

    assert!(success);

    let insert_output: InsertOutput = serde_json::from_slice(&output.stdout).unwrap();
    let id = insert_output.id;

    assert_eq!(insert_output.status, "ok");

    let db = rusqlite::Connection::open(db_path).unwrap();
    let mut stmt = db
        .prepare("SELECT name, desc FROM test WHERE id = ?1")
        .unwrap();
    let mut rows = stmt.query(rusqlite::params![id]).unwrap();
    let row = rows.next();

    assert!(row.is_ok());
    assert!(row.as_ref().unwrap().is_some());
    let row = row.unwrap().unwrap();
    assert_eq!(row.get(0), Ok("test name".to_string()));
    assert_eq!(row.get(1), Ok("test description".to_string()));
    let row = rows.next().unwrap();
    assert!(row.is_none());
}

#[test]
fn test_get_tests() {
    let tmpdir = tempdir().unwrap();
    let mut db_path = tmpdir.path().to_path_buf();
    db_path.push("test.db");

    for i in 0..5 {
        let output = run_patui(&[
            "--db",
            db_path.to_str().unwrap(),
            "new",
            "test",
            "--name",
            &format!("test name {}", i + 1),
            "--description",
            "test description",
        ]);

        let success = output.status.success();

        assert!(success);
    }

    // Get first test
    let output = run_patui(&[
        "--db",
        db_path.to_str().unwrap(),
        "get",
        "test",
        "--id",
        "1",
    ]);
    let success = output.status.success();
    assert!(success);

    let tests: Vec<PatuiTest> = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(tests.len(), 1);
    assert_eq!(
        tests.iter().map(|x| &x.name[..]).collect::<Vec<&str>>(),
        vec!["test name 1",]
    );

    // Get all tests
    let output = run_patui(&["--db", db_path.to_str().unwrap(), "get", "test"]);
    let success = output.status.success();
    assert!(success);

    let tests: Vec<PatuiTest> = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(tests.len(), 5);
    assert_eq!(
        tests.iter().map(|x| &x.name[..]).collect::<Vec<&str>>(),
        vec![
            "test name 1",
            "test name 2",
            "test name 3",
            "test name 4",
            "test name 5"
        ]
    );
}
