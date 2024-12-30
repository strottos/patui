use std::{collections::HashMap, sync::Arc};

use crate::{
    expr::PatuiData,
    runner::{events::PatuiEvent, steps::PatuiStepRunnerError},
    templates::PatuiStepAssertion,
};

use super::PatuiStepRunner;

use tokio::{
    sync::{broadcast, mpsc, RwLock},
    task::JoinHandle,
};

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerAssertion {
    step_name: String,
    step: PatuiStepAssertion,

    receivers: Option<HashMap<String, broadcast::Receiver<PatuiData>>>,

    tasks: Vec<JoinHandle<()>>,

    results: Arc<RwLock<HashMap<String, Vec<PatuiData>>>>,
}

impl PatuiStepRunnerAssertion {
    pub(crate) fn new(
        step_name: String,
        step: &PatuiStepAssertion,
        results: Arc<RwLock<HashMap<String, Vec<PatuiData>>>>,
    ) -> Self {
        Self {
            step_name,
            step: step.clone(),

            receivers: None,

            tasks: vec![],

            results,
        }
    }
}

impl PatuiStepRunner for PatuiStepRunnerAssertion {
    async fn init(
        &mut self,
        _current_step_name: &str,
        _step_runners: HashMap<
            String,
            Vec<std::sync::Arc<tokio::sync::Mutex<super::PatuiStepRunners>>>,
        >,
    ) -> Result<(), PatuiStepRunnerError> {
        Ok(())
    }

    fn run(&mut self, tx: mpsc::Sender<PatuiEvent>) -> Result<(), PatuiStepRunnerError> {
        let Some(receivers) = self.receivers.take() else {
            panic!("Receivers not set, aborting");
        };

        let task = tokio::spawn(async move {
            let mut tasks = vec![];

            for (expr, mut receiver) in receivers.into_iter() {
                tracing::trace!("Checking receiver for expr: {:?}", expr);

                tasks.push(tokio::spawn(async move {
                    while let Ok(data) = receiver.recv().await {
                        tracing::trace!("Received data: {:?}", data);
                    }
                    tracing::trace!("Receiver done");
                }));
            }

            for task in tasks.drain(..) {
                task.await.unwrap();
            }
        });

        self.tasks.push(task);

        Ok(())
    }

    async fn subscribe(
        &mut self,
        sub: &str,
    ) -> Result<broadcast::Receiver<PatuiData>, PatuiStepRunnerError> {
        Err(PatuiStepRunnerError::SubscriptionNotSupported(
            sub.to_string(),
        ))
    }

    async fn wait(&mut self, _tx: mpsc::Sender<PatuiEvent>) -> Result<(), PatuiStepRunnerError> {
        Ok(())
    }

    #[cfg(test)]
    fn test_set_receiver(
        &mut self,
        path: &str,
        rx: broadcast::Receiver<PatuiData>,
    ) -> Result<(), PatuiStepRunnerError> {
        use crate::PatuiExpr;

        let path: PatuiExpr = path.try_into()?;
        let receivers = HashMap::from([(path.raw().to_string(), rx)]);
        self.receivers = Some(receivers);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use assertor::*;
    use tokio::{
        sync::{broadcast, mpsc, RwLock},
        time::timeout,
    };
    use tracing_test::traced_test;

    use crate::{
        expr::{PatuiData, PatuiDataInner},
        runner::{events::PatuiEventKind, steps::PatuiStepRunner},
        templates::PatuiStepAssertion,
    };

    use super::PatuiStepRunnerAssertion;

    #[traced_test]
    #[tokio::test]
    async fn single_channel_read_and_eval_null() {
        let mut main_step = PatuiStepRunnerAssertion::new(
            "main".to_string(),
            &PatuiStepAssertion {
                expr: "steps.test_input.out[0] == null".try_into().unwrap(),
            },
            Arc::new(RwLock::new(HashMap::new())),
        );

        let (input_tx, input_rx) = broadcast::channel(32);

        assert_that!(main_step.test_set_receiver("steps.test_input.out", input_rx)).is_ok();

        input_tx
            .send(PatuiData::Known(PatuiDataInner::Null))
            .unwrap();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let res = timeout(Duration::from_millis(500), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(matches!(res.value(), PatuiEventKind::Log(_))).is_equal_to(true);
    }
}
