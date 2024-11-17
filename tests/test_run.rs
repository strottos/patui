mod types;
mod utils;

use assertor::*;
use tempfile::tempdir;

use self::{
    types::{PatuiRunStatus, PatuiTestEditStatus},
    utils::run_patui,
};

#[test]
fn test_run_test_instance() {
    let tmpdir = tempdir().unwrap();
    let mut db_path = tmpdir.path().to_path_buf();
    db_path.push("test.db");

    let output = run_patui(
        &["--db", db_path.to_str().unwrap(), "new", "test", "-n", "-"],
        #[cfg(target_os = "windows")]
        Some("name: List Test\ndescription: list test file\nsteps:\n  - !Process\n    command: Get-ChildItem\n    args:\n      - ./tests/data/test.txt\n"),
        #[cfg(not(target_os = "windows"))]
        Some("name: List Test\ndescription: list test file\nsteps:\n  - !Process\n    command: /usr/bin/env\n    args:\n      - ls\n      - ./tests/data/test.txt\n"),
    );

    let success = output.status.success();

    assert_that!(success);

    let test_insert_output: Vec<PatuiTestEditStatus> =
        serde_json::from_slice(&output.stdout).unwrap();
    let id = test_insert_output[0].id;

    let output = run_patui(
        &[
            "--db",
            db_path.to_str().unwrap(),
            "new",
            "run",
            "--test-id",
            &id.to_string(),
        ],
        None,
    );

    let success = output.status.success();

    assert_that!(success);

    eprintln!("Output: {:#?}", String::from_utf8_lossy(&output.stdout));
    let run_insert_output: PatuiRunStatus = serde_json::from_slice(&output.stdout).unwrap();
    let id = run_insert_output.id;

    let db = rusqlite::Connection::open(db_path).unwrap();
    let mut stmt = db
        .prepare("SELECT name, desc FROM run WHERE id = ?1")
        .unwrap();
    let mut rows = stmt.query(rusqlite::params![id]).unwrap();
    let row = rows.next();

    assert_that!(row.is_ok());
    assert!(row.as_ref().unwrap().is_some());
    let row = row.unwrap().unwrap();
    assert_that!(row.get(0)).is_equal_to(Ok("Default".to_string()));
    assert_that!(row.get(1)).is_equal_to(Ok("Default template".to_string()));
    let row = rows.next().unwrap();
    assert!(row.is_none());
}
