use std::{process::Stdio, sync::Arc};

use crate::types::{PatuiRunStepProcessResult, PatuiStepProcess};

use super::TestRunner;

use eyre::Result;

#[derive(Debug, Clone)]
pub(crate) struct PatuiRunStepProcessOps {
    pub(crate) stdin: Arc<Stdio>,
    pub(crate) stdout: Arc<Stdio>,
    pub(crate) stderr: Arc<Stdio>,
}

impl<'a> TestRunner<'a> {
    pub(crate) async fn spawn_process(
        &self,
        process: &PatuiStepProcess,
    ) -> Result<(PatuiRunStepProcessResult, PatuiRunStepProcessOps)> {
        todo!();
        // let mut result = PatuiStepResult::default();

        // let mut cmd = Command::new(&step.command);
        // cmd.arg(&step.args);

        // let output = cmd.output()?;

        // result.output = output.stdout;
        // result.error = output.stderr;
        // result.exit_code = output.status.code().unwrap_or(-1);

        // Ok(result)
    }
}
