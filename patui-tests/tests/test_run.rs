use patui_tests::run_patui;

use assertor::*;

#[test]
fn test_run_test_instance() {
    let output = run_patui(
        &["run", "test", "-"],
        #[cfg(target_os = "windows")]
        Some("name: List Test\ndescription: list test file\nsteps:\n  - !Process\n    command: pwsh\n    args:\n      - Get-ChildItem\n      - ./tests/data/test.txt\n"),
        #[cfg(not(target_os = "windows"))]
        Some("name: List Test\ndescription: list test file\nsteps:\n  - name: process\n    details: !Process\n      command: /usr/bin/env\n      args:\n        - ls\n        - ./tests/data/test.txt\n"),
    );

    let success = output.status.success();

    assert_that!(success).is_true();
}
