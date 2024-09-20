mod types;
mod utils;

use assertor::*;
use tempfile::tempdir;

use self::{
    types::{PatuiTest, PatuiTestEditStatus, PatuiTestMinDisplay},
    utils::run_patui,
};

#[test]
fn test_new_test() {
    let tmpdir = tempdir().unwrap();
    let mut db_path = tmpdir.path().to_path_buf();
    db_path.push("test.db");

    let output = run_patui(
        &["--db", db_path.to_str().unwrap(), "new", "test", "-n", "-"],
        Some("name: test name\ndescription: test description\nsteps: []\n"),
    );

    let success = output.status.success();

    assert_that!(success);

    let insert_output: Vec<PatuiTestEditStatus> = serde_json::from_slice(&output.stdout).unwrap();
    let id = insert_output[0].id;

    assert_that!(insert_output[0].status).is_equal_to("ok".to_string());

    let db = rusqlite::Connection::open(db_path).unwrap();
    let mut stmt = db
        .prepare("SELECT name, desc FROM test WHERE id = ?1")
        .unwrap();
    let mut rows = stmt.query(rusqlite::params![id]).unwrap();
    let row = rows.next();

    assert_that!(row.is_ok());
    assert!(row.as_ref().unwrap().is_some());
    let row = row.unwrap().unwrap();
    assert_that!(row.get(0)).is_equal_to(Ok("test name".to_string()));
    assert_that!(row.get(1)).is_equal_to(Ok("test description".to_string()));
    let row = rows.next().unwrap();
    assert!(row.is_none());
}

#[test]
fn test_new_test_with_default_template() {
    let tmpdir = tempdir().unwrap();
    let mut db_path = tmpdir.path().to_path_buf();
    db_path.push("test.db");

    let output = run_patui(
        &[
            "--db",
            db_path.to_str().unwrap(),
            "new",
            "test",
            "-n",
            "--template",
            "default",
        ],
        None,
    );

    let success = output.status.success();

    assert_that!(success);

    let insert_output: Vec<PatuiTestEditStatus> = serde_json::from_slice(&output.stdout).unwrap();
    let id = insert_output[0].id;

    assert_that!(insert_output[0].status).is_equal_to("ok".to_string());

    let db = rusqlite::Connection::open(db_path).unwrap();
    let mut stmt = db
        .prepare("SELECT name, desc FROM test WHERE id = ?1")
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

#[test]
fn test_get_tests() {
    let tmpdir = tempdir().unwrap();
    let mut db_path = tmpdir.path().to_path_buf();
    db_path.push("test.db");

    for i in 0..5 {
        let output = run_patui(
            &["--db", db_path.to_str().unwrap(), "new", "test", "-n", "-"],
            Some(&format!(
                "name: test name {}\ndescription: test description\nsteps: []\n",
                i + 1,
            )),
        );

        let success = output.status.success();

        assert!(success);
    }

    // Get first test
    let output = run_patui(
        &[
            "--db",
            db_path.to_str().unwrap(),
            "describe",
            "test",
            "--id",
            "1",
        ],
        None,
    );
    let success = output.status.success();
    assert!(success);

    eprintln!("{}", std::str::from_utf8(&output.stdout).unwrap());
    let test: PatuiTest = serde_json::from_slice(&output.stdout).unwrap();

    assert_that!(test.name).is_equal_to("test name 1".to_string());

    // Get all tests
    let output = run_patui(&["--db", db_path.to_str().unwrap(), "get", "tests"], None);
    let success = output.status.success();
    assert!(success);

    let tests: Vec<PatuiTestMinDisplay> = serde_json::from_slice(&output.stdout).unwrap();

    assert_that!(tests.len()).is_equal_to(5);
    assert_that!(tests.iter().map(|x| &x.name[..]).collect::<Vec<&str>>()).is_equal_to(vec![
        "test name 1",
        "test name 2",
        "test name 3",
        "test name 4",
        "test name 5",
    ]);
}

#[test]
fn test_new_test_with_shell() {
    let tmpdir = tempdir().unwrap();
    let mut db_path = tmpdir.path().to_path_buf();
    db_path.push("test.db");

    let output = run_patui(
        &["--db", db_path.to_str().unwrap(), "new", "test", "-n", "-"],
        Some("name: test name\ndescription: test description\nsteps:\n  - !Shell\n    shell: bash\n    contents: echo 'Hello, world!'\n"),
    );

    let success = output.status.success();
    assert!(success);

    // Get first test
    let output = run_patui(&["--db", db_path.to_str().unwrap(), "get", "tests"], None);
    let success = output.status.success();
    assert!(success);

    let tests: Vec<PatuiTestMinDisplay> = serde_json::from_slice(&output.stdout).unwrap();
    assert_that!(tests.len()).is_equal_to(1);

    let id = tests[0].id;

    // Describe test
    let output = run_patui(
        &[
            "--db",
            db_path.to_str().unwrap(),
            "describe",
            "test",
            "--id",
            &format!("{}", id),
        ],
        None,
    );
    let success = output.status.success();
    assert!(success);

    eprintln!("{}", std::str::from_utf8(&output.stdout).unwrap());
    let test: PatuiTest = serde_json::from_slice(&output.stdout).unwrap();

    assert_that!(test.name).is_equal_to("test name".to_string());
    assert_that!(test.description).is_equal_to("test description".to_string());
    assert_that!(test.steps).has_length(1);
    let shell_step = match &test.steps[0] {
        types::PatuiStepDetails::Shell(shell) => shell,
        types::PatuiStepDetails::Assertion(_) => panic!("Expected shell step"),
    };
    assert_that!(shell_step.shell).is_equal_to(Some("bash".to_string()));
    assert_that!(shell_step.contents).is_equal_to("echo 'Hello, world!'".to_string());
    assert_that!(shell_step.location).is_none();
}
