mod assertions;
mod process;
mod transform_stream;

use bytes::Bytes;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::types::{PatuiStep, PatuiStepData, PatuiStepDetails};

use self::{
    assertions::PatuiStepRunnerAssertion, process::PatuiStepRunnerProcess,
    transform_stream::PatuiStepRunnerTransformStream,
};

pub(crate) enum PatuiStepRunnerFlavour {
    Process(PatuiStepRunnerProcess),
    TransformStream(PatuiStepRunnerTransformStream),
    Assertion(PatuiStepRunnerAssertion),
}

pub(crate) struct PatuiStepRunner {
    flavour: PatuiStepRunnerFlavour,
}

impl PatuiStepRunner {
    pub(crate) fn new(step: &PatuiStep) -> Self {
        let flavour = match &step.details {
            PatuiStepDetails::Process(details) => {
                PatuiStepRunnerFlavour::Process(PatuiStepRunnerProcess::new(details))
            }
            PatuiStepDetails::TransformStream(details) => PatuiStepRunnerFlavour::TransformStream(
                PatuiStepRunnerTransformStream::new(details),
            ),
            PatuiStepDetails::Read(patui_step_read) => todo!(),
            PatuiStepDetails::Write(patui_step_write) => todo!(),
            PatuiStepDetails::Assertion(patui_step_assertion) => PatuiStepRunnerFlavour::Assertion(
                PatuiStepRunnerAssertion::new(patui_step_assertion),
            ),
        };

        Self { flavour }
    }

    pub(crate) fn init(&mut self) -> Result<()> {
        match &mut self.flavour {
            PatuiStepRunnerFlavour::Process(runner) => runner.init(),
            PatuiStepRunnerFlavour::TransformStream(runner) => runner.init(),
            PatuiStepRunnerFlavour::Assertion(runner) => runner.init(),
        }
    }
}

pub(crate) trait PatuiStepRunnerTrait {
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn run(&mut self) -> Result<bool> {
        Ok(true)
    }

    fn subscribe(&self, sub: &str) -> Result<broadcast::Receiver<PatuiStepData>> {
        Err(eyre!("Subscription not supported"))
    }

    fn publish(&self, publ: &str, data: PatuiStepData) -> Result<()> {
        Err(eyre!("Publishing not supported"))
    }

    async fn wait(&mut self, action: &str) -> Result<PatuiStepData> {
        Err(eyre!("Waiting not supported"))
    }

    fn check(&mut self, action: &str) -> Result<PatuiStepData> {
        Err(eyre!("Checking not supported"))
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use super::*;

    #[test]
    fn step_process() {}
}
