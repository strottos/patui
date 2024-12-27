use edit::edit;
use miette::{IntoDiagnostic, Result};
use tokio::{
    fs::OpenOptions,
    io::{stdin, AsyncReadExt, AsyncWriteExt},
};

use patui_core::PatuiTest;

pub async fn edit_file_with_test(mut yaml_str: String, file_path: &str) -> Result<PatuiTest> {
    let test = loop {
        yaml_str = edit(&yaml_str).into_diagnostic()?;
        match PatuiTest::from_yaml_str(&yaml_str) {
            Ok(test) => {
                // TODO: Validate test
                break test;
            }
            Err(e) => {
                eprintln!("Failed to parse yaml: {e}\nPress any key to continue editing or Ctrl-C to cancel...");
                let buffer = &mut [0u8];
                let _ = stdin().read_exact(buffer).await;
            }
        };
    };

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)
        .await
        .into_diagnostic()?;

    file.write_all(yaml_str.as_bytes())
        .await
        .into_diagnostic()?;

    Ok(test)
}
