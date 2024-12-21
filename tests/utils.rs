#![allow(dead_code)]

use std::process::Output;

use assert_cmd::Command;

pub(crate) fn run_patui(args: &[&str], stdin: Option<&str>) -> Output {
    let mut cmd = Command::cargo_bin("patui").unwrap();
    if let Some(stdin) = stdin {
        cmd.write_stdin(stdin);
    }
    let output = match cmd
        .args(args)
        .env("PATUI_LOG", "trace")
        .env("PATUI_LOG_FILE", "./target/test_logs/patui.log.${datetime}")
        .ok()
    {
        Ok(output) => output,
        Err(e) => panic!(
            "Err: {:#?}\n{}",
            e,
            String::from_utf8(e.as_output().unwrap().stderr.clone()).unwrap()
        ),
    };

    output
}
