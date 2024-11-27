mod other;
mod process;
mod transform_stream;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::types::{PatuiStep, PatuiStepData, PatuiStepDetails};

use self::{
    other::{PatuiStepRunnerAssertion, PatuiStepRunnerRead, PatuiStepRunnerWrite},
    process::PatuiStepRunnerProcess,
    transform_stream::PatuiStepRunnerTransformStream,
};

#[derive(Debug)]
pub(crate) enum PatuiStepRunnerFlavour {
    Process(PatuiStepRunnerProcess),
    TransformStream(PatuiStepRunnerTransformStream),
    Read(PatuiStepRunnerRead),
    Write(PatuiStepRunnerWrite),
    Assertion(PatuiStepRunnerAssertion),
}

#[derive(Debug)]
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
            PatuiStepDetails::Read(patui_step_read) => {
                PatuiStepRunnerFlavour::Read(PatuiStepRunnerRead::new(patui_step_read))
            }
            PatuiStepDetails::Write(patui_step_write) => {
                PatuiStepRunnerFlavour::Write(PatuiStepRunnerWrite::new(patui_step_write))
            }
            PatuiStepDetails::Assertion(patui_step_assertion) => PatuiStepRunnerFlavour::Assertion(
                PatuiStepRunnerAssertion::new(patui_step_assertion),
            ),
        };

        Self { flavour }
    }

    pub(crate) fn init(
        &mut self,
        step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
        tracing::trace!("Initializing step runner: {:#?}", self);
        tracing::trace!("Step runners: {:#?}", step_runners);

        match &mut self.flavour {
            PatuiStepRunnerFlavour::Process(runner) => runner.init(step_runners),
            PatuiStepRunnerFlavour::TransformStream(runner) => runner.init(step_runners),
            PatuiStepRunnerFlavour::Read(runner) => runner.init(step_runners),
            PatuiStepRunnerFlavour::Write(runner) => runner.init(step_runners),
            PatuiStepRunnerFlavour::Assertion(runner) => runner.init(step_runners),
        }
    }
}

pub(crate) trait PatuiStepRunnerTrait {
    fn init(
        &mut self,
        step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
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
