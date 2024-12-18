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
    step: PatuiStepAssertion,

    receivers: Option<HashMap<PatuiExpr, broadcast::Receiver<PatuiStepData>>>,
    tasks: Vec<JoinHandle<()>>,
}

impl PatuiStepRunnerAssertion {
    pub(crate) fn new(_step_name: String, step: &PatuiStepAssertion) -> Self {
        Self {
            step: step.clone(),
            receivers: None,
            tasks: vec![],
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
            init_subscribe_steps(&self.step.expr, current_step_name, &step_runners).await?;
        self.receivers = Some(receivers);

        Ok(())
    }

    fn run(&mut self, tx: tokio::sync::mpsc::Sender<crate::types::PatuiEvent>) -> Result<()> {
        let step = self.step.clone();
        let receivers = self.receivers.take();

        let task = tokio::spawn(async move {
            tracing::trace!("Running assertion step with expr: {:?}", step.expr);
            let Some(mut receivers) = receivers else {
                panic!("No receivers found");
            };
        });

        self.tasks.push(task);

        Ok(())
    }

    async fn wait(&mut self) -> Result<()> {
        tracing::trace!("Waiting");

        for task in self.tasks.drain(..) {
            task.await?;
        }

        Ok(())
    }

    #[cfg(test)]
    fn test_set_receiver(
        &mut self,
        sub_ref: &str,
        rx: broadcast::Receiver<PatuiStepData>,
    ) -> Result<()> {
        let receivers = HashMap::from([(sub_ref.try_into().unwrap(), rx)]);
        self.receivers = Some(receivers);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use assertor::*;
    use bytes::Bytes;
    use tokio::{sync::mpsc, time::timeout};
    use tracing_test::traced_test;

    use crate::types::{PatuiEventKind, PatuiStepDataFlavour};

    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn single_channel_read_and_eval_null() {
        let mut main_step = PatuiStepRunnerAssertion::new(
            "main".to_string(),
            &PatuiStepAssertion {
                expr: "steps.test_input.out[0] == null".try_into().unwrap(),
            },
        );

        let (input_tx, input_rx) = broadcast::channel(32);

        assert_that!(main_step.test_set_receiver("steps.test_input.out", input_rx)).is_ok();

        input_tx
            .send(PatuiStepData::new(PatuiStepDataFlavour::Null))
            .unwrap();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let res = timeout(Duration::from_millis(50), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(res.value()).is_equal_to(&PatuiEventKind::Bytes(Bytes::from("ABC")));
    }

    #[traced_test]
    #[tokio::test]
    async fn single_channel_read_and_eval_null_fails() {
        let mut main_step = PatuiStepRunnerAssertion::new(
            "main".to_string(),
            &PatuiStepAssertion {
                expr: "steps.test_input.out[0] == null".try_into().unwrap(),
            },
        );

        let (input_tx, input_rx) = broadcast::channel(32);

        assert_that!(main_step.test_set_receiver("steps.test_input.out", input_rx)).is_ok();

        input_tx
            .send(PatuiStepData::new(PatuiStepDataFlavour::Bytes(
                Bytes::from("ABC"),
            )))
            .unwrap();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_err();

        let res = timeout(Duration::from_millis(50), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(res.value()).is_equal_to(&PatuiEventKind::Bytes(Bytes::from("ABC")));
    }
}
