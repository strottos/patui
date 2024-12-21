use super::PatuiStepRunnerTrait;
use crate::types::PatuiStepWrite;

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerWrite {
    // step: PatuiStepWrite,
}

impl PatuiStepRunnerWrite {
    pub(crate) fn new(_step: &PatuiStepWrite) -> Self {
        Self {}
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerWrite {}
