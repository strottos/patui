use super::PatuiStepRunnerTrait;
use crate::types::PatuiStepWrite;

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerWrite {
    step: PatuiStepWrite,
}

impl PatuiStepRunnerWrite {
    pub(crate) fn new(step: &PatuiStepWrite) -> Self {
        Self { step: step.clone() }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerWrite {}
