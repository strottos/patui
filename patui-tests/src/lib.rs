use std::process::Output;

use assert_cmd::Command;
use escargot::CargoBuild;

pub fn run_patui(args: &[&str], stdin: Option<&str>) -> Output {
    let cli = CargoBuild::new()
        .bin("patui")
        .current_target()
        .manifest_path("../patui-bin/Cargo.toml")
        .target_dir("./target/debug")
        .run()
        .unwrap();

    let mut cmd = Command::from_std(std::process::Command::new(cli.path()));
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
