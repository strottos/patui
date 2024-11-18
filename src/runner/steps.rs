mod assertions;
mod process;
mod transform_stream;

use bytes::Bytes;
use eyre::{eyre, Result};
use process::PatuiStepRunnerProcess;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use transform_stream::PatuiStepRunnerTransformStream;

use crate::types::PatuiStepData;

pub(crate) enum PatuiStepRunnerFlavour {
    Process(PatuiStepRunnerProcess),
    TransformStream(PatuiStepRunnerTransformStream),
}

pub(crate) struct PatuiStepRunner {
    flavour: PatuiStepRunnerFlavour,
}

pub(crate) trait PatuiStepRunnerTrait {
    fn setup(&mut self) -> Result<()> {
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
