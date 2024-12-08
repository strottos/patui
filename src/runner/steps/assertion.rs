use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use eyre::Result;
use tokio::{sync::broadcast, task::JoinHandle};

use super::{init_subscribe_steps, PatuiStepRunner, PatuiStepRunnerTrait};
use crate::types::{PatuiExpr, PatuiStepAssertion, PatuiStepData};

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerAssertion {
    step_name: String,
    step: PatuiStepAssertion,

    receivers: Option<HashMap<PatuiExpr, broadcast::Receiver<PatuiStepData>>>,

    tasks: Vec<JoinHandle<()>>,
}

impl PatuiStepRunnerAssertion {
    pub(crate) fn new(step_name: String, step: &PatuiStepAssertion) -> Self {
        Self {
            step_name,
            step: step.clone(),
            receivers: None,
            tasks: vec![],
        }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerAssertion {
    fn init(
        &mut self,
        current_step_name: &str,
        step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
        let receivers = init_subscribe_steps(&self.step.expr, current_step_name, step_runners)?;
        self.receivers = Some(receivers);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;
    use tokio::sync::mpsc;
    use tracing_test::traced_test;

    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn assert_that_is_ok() {
        let assertion = PatuiStepAssertion {
            expr: "foo".try_into().unwrap(),
        };

        let mut runner = PatuiStepRunnerAssertion::new("main".to_string(), &assertion);

        let (tx, rx) = mpsc::channel(1);

        assert_that!(runner.run(tx.clone())).is_ok();
    }
}
