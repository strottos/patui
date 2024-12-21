mod assertion;
mod reader;
mod sender;
mod transform_stream;
mod writer;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use eyre::{eyre, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};

use self::{
    assertion::PatuiStepRunnerAssertion, reader::PatuiStepRunnerRead,
    sender::PatuiStepRunnerSender, transform_stream::PatuiStepRunnerTransformStream,
    writer::PatuiStepRunnerWrite,
};
use crate::types::{
    expr::{
        ast::{Expr, ExprKind, TermParts},
        get_all_terms,
    },
    PatuiEvent, PatuiExpr, PatuiStep, PatuiStepData, PatuiStepDetails,
};

#[derive(Debug)]
pub(crate) enum PatuiStepRunnerFlavour {
    Read(PatuiStepRunnerRead),
    Write(PatuiStepRunnerWrite),
    TransformStream(PatuiStepRunnerTransformStream),
    Assertion(PatuiStepRunnerAssertion),
    Sender(PatuiStepRunnerSender),
}

#[derive(Debug)]
pub(crate) struct PatuiStepRunner {
    name: String,
    flavour: PatuiStepRunnerFlavour,
}

impl PatuiStepRunner {
    pub(crate) fn new(step: &PatuiStep) -> Self {
        let flavour = match &step.details {
            PatuiStepDetails::TransformStream(patui_step_transform_strema) => {
                PatuiStepRunnerFlavour::TransformStream(PatuiStepRunnerTransformStream::new(
                    step.name.clone(),
                    patui_step_transform_strema,
                ))
            }
            PatuiStepDetails::Read(patui_step_read) => PatuiStepRunnerFlavour::Read(
                PatuiStepRunnerRead::new(step.name.clone(), patui_step_read),
            ),
            PatuiStepDetails::Write(patui_step_write) => {
                PatuiStepRunnerFlavour::Write(PatuiStepRunnerWrite::new(patui_step_write))
            }
            PatuiStepDetails::Assertion(patui_step_assertion) => PatuiStepRunnerFlavour::Assertion(
                PatuiStepRunnerAssertion::new(step.name.clone(), patui_step_assertion),
            ),
            PatuiStepDetails::Sender(patui_step_sender) => {
                PatuiStepRunnerFlavour::Sender(PatuiStepRunnerSender::new(patui_step_sender))
            }
        };

        Self {
            name: step.name.clone(),
            flavour,
        }
    }

    pub(crate) fn init(
        &mut self,
        current_step_name: &str,
        step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
        tracing::trace!("Initializing step runner: {:?}", self);

        match &mut self.flavour {
            PatuiStepRunnerFlavour::TransformStream(runner) => {
                runner.init(current_step_name, step_runners)
            }
            PatuiStepRunnerFlavour::Read(runner) => runner.init(current_step_name, step_runners),
            PatuiStepRunnerFlavour::Write(runner) => runner.init(current_step_name, step_runners),
            PatuiStepRunnerFlavour::Assertion(runner) => {
                runner.init(current_step_name, step_runners)
            }
            PatuiStepRunnerFlavour::Sender(patui_step_runner_sender) => todo!(),
        }
    }

    pub(crate) fn run(&mut self, tx: mpsc::Sender<PatuiEvent>) -> Result<()> {
        match &mut self.flavour {
            PatuiStepRunnerFlavour::TransformStream(runner) => runner.run(tx),
            PatuiStepRunnerFlavour::Read(runner) => runner.run(tx),
            PatuiStepRunnerFlavour::Write(runner) => runner.run(tx),
            PatuiStepRunnerFlavour::Assertion(runner) => runner.run(tx),
            PatuiStepRunnerFlavour::Sender(runner) => runner.run(tx),
        }
    }

    pub(crate) async fn wait(&mut self) -> Result<()> {
        match &mut self.flavour {
            PatuiStepRunnerFlavour::TransformStream(runner) => runner.wait().await,
            PatuiStepRunnerFlavour::Read(runner) => runner.wait().await,
            PatuiStepRunnerFlavour::Write(runner) => runner.wait().await,
            PatuiStepRunnerFlavour::Assertion(runner) => runner.wait().await,
            PatuiStepRunnerFlavour::Sender(runner) => runner.wait().await,
        }
    }

    fn flavour(&self) -> &PatuiStepRunnerFlavour {
        &self.flavour
    }
}

pub(crate) trait PatuiStepRunnerTrait {
    fn init(
        &mut self,
        current_step_name: &str,
        step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
        Ok(())
    }

    fn run(&mut self, _tx: mpsc::Sender<PatuiEvent>) -> Result<()> {
        Ok(())
    }

    fn subscribe(&self, sub: &str) -> Result<broadcast::Receiver<PatuiStepData>> {
        Err(eyre!("Subscription not supported"))
    }

    async fn wait(&mut self) -> Result<()> {
        Ok(())
    }

    fn check(&mut self, action: &str) -> Result<PatuiStepData> {
        Err(eyre!("Checking not supported"))
    }

    #[cfg(test)]
    fn test_set_receiver(
        &mut self,
        sub_ref: &str,
        rx: broadcast::Receiver<PatuiStepData>,
    ) -> Result<()> {
        Err(eyre!("Test set receiver not supported"))
    }
}

fn init_subscribe_steps(
    expr: &PatuiExpr,
    current_step_name: &str,
    other_step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
) -> Result<HashMap<Expr, broadcast::Receiver<PatuiStepData>>> {
    let mut receivers = HashMap::new();

    for term in get_all_terms(expr)?.iter() {
        tracing::trace!("Checking term for subscribing: {:?}", term.kind());
        let (ref_step_name, sub_name) = match term.kind() {
            ExprKind::Term(term) => {
                if term.values.first() == Some(&TermParts::Ident("steps".to_string())) {
                    if let Some(TermParts::Ident(ref_step_name)) = term.values.get(1) {
                        if let Some(TermParts::Ident(sub_name)) = term.values.get(2) {
                            (ref_step_name, sub_name)
                        } else {
                            return Err(eyre!("Invalid term for subscribing: {:?}", term));
                        }
                    } else {
                        return Err(eyre!("Invalid term for subscribing: {:?}", term));
                    }
                } else {
                    continue;
                }
            }
            _ => unreachable!(),
        };

        if let Some(step_runners) = other_step_runners.get(ref_step_name) {
            tracing::debug!("Subscription: {current_step_name} -> {ref_step_name}");
            tracing::trace!("Step Runners: {:?}", step_runners);

            for step_runner in step_runners {
                let mut step_runner = step_runner.lock().unwrap();
                match step_runner.flavour() {
                    PatuiStepRunnerFlavour::TransformStream(patui_step_runner_transform_stream) => {
                        receivers.insert(
                            term.clone(),
                            patui_step_runner_transform_stream.subscribe(&sub_name)?,
                        );
                    }
                    PatuiStepRunnerFlavour::Read(patui_step_runner_read) => {
                        receivers
                            .insert(term.clone(), patui_step_runner_read.subscribe(&sub_name)?);
                    }
                    PatuiStepRunnerFlavour::Write(patui_step_runner_write) => todo!(),
                    PatuiStepRunnerFlavour::Assertion(patui_step_runner_assertion) => {
                        receivers.insert(
                            term.clone(),
                            patui_step_runner_assertion.subscribe(&sub_name)?,
                        );
                    }
                    PatuiStepRunnerFlavour::Sender(patui_step_runner_sender) => {}
                }
            }
        } else {
            return Err(eyre!(
                "No step found for referenced step: `{}`",
                ref_step_name
            ));
        }
    }

    Ok(receivers)
}
