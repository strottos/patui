use std::process::Output;

use assert_cmd::Command;

pub(crate) fn run_patui(args: &[&str], stdin: Option<&str>) -> Output {
    let mut cmd = Command::cargo_bin("patui").unwrap();
    if let Some(stdin) = stdin {
        cmd.write_stdin(stdin);
    }
    let output = cmd.args(args).ok().unwrap();

    output
}
