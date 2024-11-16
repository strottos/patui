//! Types related to testing running processes.

use serde::{Deserialize, Serialize};
use tokio_util::bytes::Bytes;

use super::PatuiTimestamp;

fn step_process_wait_default() -> bool {
    true
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepProcess {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) tty: Option<(u16, u16)>,
    #[serde(default = "step_process_wait_default")]
    pub(crate) wait: bool,
    pub(crate) input: Option<String>,
    pub(crate) cwd: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunStepProcessResult {
    pub(crate) stdin: Vec<PatuiTimestamp<Bytes>>,
    pub(crate) stdout: Vec<PatuiTimestamp<Bytes>>,
    pub(crate) stderr: Vec<PatuiTimestamp<Bytes>>,
    pub(crate) exit_code: i32,
}
