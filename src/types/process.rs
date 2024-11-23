//! Types related to testing running processes.

use serde::{Deserialize, Serialize};
use tokio_util::bytes::Bytes;

use super::PatuiTimestamp;

fn step_process_editable_wait_default() -> Option<bool> {
    Some(true)
}

fn step_process_wait_default() -> bool {
    true
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepProcessEditable {
    pub(crate) command: String,
    pub(crate) args: Option<Vec<String>>,
    pub(crate) tty: Option<Option<(u16, u16)>>,
    #[serde(default = "step_process_editable_wait_default")]
    pub(crate) wait: Option<bool>,
    pub(crate) input: Option<Option<String>>,
    pub(crate) cwd: Option<Option<String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepProcess {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) tty: Option<(u16, u16)>,
    #[serde(default = "step_process_wait_default")]
    pub(crate) wait: bool,
    pub(crate) input: Option<String>,
    pub(crate) cwd: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunStepProcessResult {
    pub(crate) stdin: Vec<PatuiTimestamp<Bytes>>,
    pub(crate) stdout: Vec<PatuiTimestamp<Bytes>>,
    pub(crate) stderr: Vec<PatuiTimestamp<Bytes>>,
    pub(crate) exit_code: i32,
}
