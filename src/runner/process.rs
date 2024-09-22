use std::{process::Stdio, sync::Arc};

use crate::types::{PatuiRunStepProcessResult, PatuiStepProcess, PatuiTimestamp};

use super::TestRunner;

use bytes::Bytes;
use eyre::Result;
use tokio::process::Command;

#[derive(Debug)]
pub(crate) struct PatuiRunStepProcessOps {
    pub(crate) stdin: Stdio,
    pub(crate) stdout: Stdio,
    pub(crate) stderr: Stdio,
}

impl<'a> TestRunner<'a> {
    pub(crate) async fn spawn_process(
        &self,
        process: &PatuiStepProcess,
    ) -> Result<(PatuiRunStepProcessResult, PatuiRunStepProcessOps)> {
        let mut cmd = Command::new(&process.command);
        cmd.args(&process.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if process.wait {
            self.spawn_process_and_wait(&mut cmd).await
        } else {
            todo!()
        }
    }

    async fn spawn_process_and_wait(
        &self,
        cmd: &mut Command,
    ) -> Result<(PatuiRunStepProcessResult, PatuiRunStepProcessOps)> {
        let output = cmd.output().await?;

        let ops = PatuiRunStepProcessOps {
            stdin: Stdio::null(),
            stdout: Stdio::null(),
            stderr: Stdio::null(),
        };

        let mut result = PatuiRunStepProcessResult {
            stdin: vec![],
            stdout: vec![PatuiTimestamp::new(Bytes::from(output.stdout))],
            stderr: vec![PatuiTimestamp::new(Bytes::from(output.stderr))],
            exit_code: output.status.code().unwrap_or(-1),
        };

        // let output = cmd.output()?;

        // result.output = output.stdout;
        // result.error = output.stderr;
        // result.exit_code = output.status.code().unwrap_or(-1);

        Ok((result, ops))
    }
}
