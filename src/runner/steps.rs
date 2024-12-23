mod assertion;
mod plugin;
mod reader;
mod sender;
mod transform_stream;
mod writer;

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use eyre::{eyre, Result};
use tokio::sync::{broadcast, mpsc};

use self::{
    assertion::PatuiStepRunnerAssertion, plugin::PatuiStepRunnerPlugin,
    reader::PatuiStepRunnerRead, sender::PatuiStepRunnerSender,
    transform_stream::PatuiStepRunnerTransformStream, writer::PatuiStepRunnerWrite,
};
use crate::types::{
    expr::{ast::ExprKind, get_all_idents},
    PatuiEvent, PatuiExpr, PatuiStep, PatuiStepData, PatuiStepDetails,
};

#[derive(Debug)]
pub(crate) enum PatuiStepRunnerFlavour {
    Read(PatuiStepRunnerRead),
    Write(PatuiStepRunnerWrite),
    Sender(PatuiStepRunnerSender),
    TransformStream(PatuiStepRunnerTransformStream),
    Assertion(PatuiStepRunnerAssertion),
    Plugin(PatuiStepRunnerPlugin),
}

#[derive(Debug)]
pub(crate) struct PatuiStepRunner {
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
            PatuiStepDetails::Plugin(patui_step_plugin) => PatuiStepRunnerFlavour::Plugin(
                PatuiStepRunnerPlugin::new(step.name.clone(), patui_step_plugin),
            ),
        };

        Self { flavour }
    }

    pub(crate) async fn init(
        &mut self,
        current_step_name: &str,
        step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
        tracing::trace!("Initializing step runner: {:?}", self);

        match &mut self.flavour {
            PatuiStepRunnerFlavour::TransformStream(runner) => {
                runner.init(current_step_name, step_runners).await
            }
            PatuiStepRunnerFlavour::Read(runner) => {
                runner.init(current_step_name, step_runners).await
            }
            PatuiStepRunnerFlavour::Write(runner) => {
                runner.init(current_step_name, step_runners).await
            }
            PatuiStepRunnerFlavour::Assertion(runner) => {
                runner.init(current_step_name, step_runners).await
            }
            PatuiStepRunnerFlavour::Sender(runner) => {
                runner.init(current_step_name, step_runners).await
            }
            PatuiStepRunnerFlavour::Plugin(runner) => {
                runner.init(current_step_name, step_runners).await
            }
        }
    }

    pub(crate) fn run(&mut self, tx: mpsc::Sender<PatuiEvent>) -> Result<()> {
        match &mut self.flavour {
            PatuiStepRunnerFlavour::TransformStream(runner) => runner.run(tx),
            PatuiStepRunnerFlavour::Read(runner) => runner.run(tx),
            PatuiStepRunnerFlavour::Write(runner) => runner.run(tx),
            PatuiStepRunnerFlavour::Assertion(runner) => runner.run(tx),
            PatuiStepRunnerFlavour::Sender(runner) => runner.run(tx),
            PatuiStepRunnerFlavour::Plugin(runner) => runner.run(tx),
        }
    }

    pub(crate) async fn wait(&mut self) -> Result<()> {
        match &mut self.flavour {
            PatuiStepRunnerFlavour::TransformStream(runner) => runner.wait().await,
            PatuiStepRunnerFlavour::Read(runner) => runner.wait().await,
            PatuiStepRunnerFlavour::Write(runner) => runner.wait().await,
            PatuiStepRunnerFlavour::Assertion(runner) => runner.wait().await,
            PatuiStepRunnerFlavour::Sender(runner) => runner.wait().await,
            PatuiStepRunnerFlavour::Plugin(runner) => runner.wait().await,
        }
    }

    fn flavour_mut(&mut self) -> &mut PatuiStepRunnerFlavour {
        &mut self.flavour
    }
}

pub(crate) trait PatuiStepRunnerTrait {
    async fn init(
        &mut self,
        _current_step_name: &str,
        _step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
        Ok(())
    }

    fn run(&mut self, _tx: mpsc::Sender<PatuiEvent>) -> Result<()> {
        Ok(())
    }

    async fn subscribe(&mut self, _sub: &str) -> Result<broadcast::Receiver<PatuiStepData>> {
        Err(eyre!("Subscription not supported"))
    }

    async fn wait(&mut self) -> Result<()> {
        Ok(())
    }

    // fn check(&mut self, _action: &str) -> Result<PatuiStepData> {
    //     Err(eyre!("Checking not supported"))
    // }

    #[cfg(test)]
    fn test_set_receiver(
        &mut self,
        _sub_ref: &str,
        _rx: broadcast::Receiver<PatuiStepData>,
    ) -> Result<()> {
        Err(eyre!("Test set receiver not supported"))
    }
}

async fn init_subscribe_steps(
    expr: &PatuiExpr,
    current_step_name: &str,
    other_step_runners: &HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
) -> Result<HashMap<PatuiExpr, broadcast::Receiver<PatuiStepData>>> {
    let mut receivers = HashMap::new();

    for ident in get_all_idents(expr)?.iter() {
        tracing::trace!("Checking ident for subscribing: {:?}", ident.kind());
        let (ref_step, field) = match ident.kind() {
            ExprKind::Ident(_) => continue,
            ExprKind::Field(root_expr, field_ident) => match root_expr.kind() {
                ExprKind::Field(root_expr, sub_expr) => match root_expr.kind() {
                    ExprKind::Ident(root_ident) => {
                        if root_ident.value == "steps".to_string() {
                            (&sub_expr.value, &field_ident.value)
                        } else {
                            continue;
                        }
                    }
                    _ => continue,
                },
                _ => continue,
            },
            ExprKind::Index(_, _) => continue,
            ExprKind::Call(_, _) => continue,
            _ => {
                return Err(eyre::eyre!("Unrecognised ident kind: {}", ident));
            }
        };

        if let Some(step_runners) = other_step_runners.get(ref_step) {
            tracing::debug!("Subscription: {current_step_name} -> {ref_step}");
            tracing::trace!("Step Runners: {:?}", step_runners);

            for step_runner in step_runners {
                let mut step_runner = step_runner.lock().unwrap();
                match step_runner.flavour_mut() {
                    PatuiStepRunnerFlavour::TransformStream(patui_step_runner_transform_stream) => {
                        receivers.insert(
                            ident.clone(),
                            patui_step_runner_transform_stream.subscribe(&field).await?,
                        );
                    }
                    PatuiStepRunnerFlavour::Read(patui_step_runner_read) => {
                        receivers.insert(
                            ident.clone(),
                            patui_step_runner_read.subscribe(&field).await?,
                        );
                    }
                    PatuiStepRunnerFlavour::Write(_) => todo!(),
                    PatuiStepRunnerFlavour::Assertion(_) => {
                        todo!()
                    }
                    PatuiStepRunnerFlavour::Sender(_) => {}
                    PatuiStepRunnerFlavour::Plugin(_) => {
                        todo!()
                    }
                }
            }
        } else {
            return Err(eyre!("No step found for referenced step: `{}`", ref_step));
        }
    }

    Ok(receivers)
}
