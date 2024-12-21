use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use eyre::Result;
use tokio::sync::broadcast;

use super::{init_subscribe_steps, PatuiStepRunner, PatuiStepRunnerTrait};
use crate::types::{PatuiExpr, PatuiStepAssertion, PatuiStepData};

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerAssertion {
    step: PatuiStepAssertion,

    receivers: Option<HashMap<PatuiExpr, broadcast::Receiver<PatuiStepData>>>,
    // tasks: Vec<JoinHandle<()>>,
}

impl PatuiStepRunnerAssertion {
    pub(crate) fn new(_step_name: String, step: &PatuiStepAssertion) -> Self {
        Self {
            step: step.clone(),
            receivers: None,
            // tasks: vec![],
        }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerAssertion {
    async fn init(
        &mut self,
        current_step_name: &str,
        step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
        let receivers =
            init_subscribe_steps(&self.step.expr, current_step_name, step_runners).await?;
        self.receivers = Some(receivers);

        Ok(())
    }
}
