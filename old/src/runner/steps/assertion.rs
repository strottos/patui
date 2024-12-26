use std::{collections::HashMap, sync::Arc};

use eyre::Result;
use tokio::{
    sync::{broadcast, mpsc, Mutex, RwLock},
    task::JoinHandle,
};

use super::{init_subscribe_steps, Expr, PatuiStepRunner, PatuiStepRunnerTrait};
use crate::types::{
    expr::{eval, EvalResult},
    PatuiEvent, PatuiEventKind, PatuiStepAssertion, PatuiStepData,
};

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerAssertion {
    step_name: String,
    step: PatuiStepAssertion,

    receivers: Option<HashMap<Expr, broadcast::Receiver<PatuiStepData>>>,

    tasks: Vec<JoinHandle<()>>,

    results: Arc<RwLock<HashMap<Expr, Vec<PatuiStepData>>>>,

    done: Arc<Mutex<bool>>,
}

impl PatuiStepRunnerAssertion {
    pub(crate) fn new(step_name: String, step: &PatuiStepAssertion) -> Self {
        Self {
            step_name,
            step: step.clone(),
            receivers: None,
            tasks: vec![],
            results: Arc::new(RwLock::new(HashMap::new())),
            done: Arc::new(Mutex::new(false)),
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

    fn run(&mut self, tx: mpsc::Sender<PatuiEvent>) -> Result<()> {
        let step_name = self.step_name.clone();
        let step = self.step.clone();
        let receivers = self.receivers.take();
        let results = self.results.clone();
        let done = self.done.clone();

        // Notify of results coming in
        let (notify_tx, mut notify_rx) = mpsc::channel(1);

        let task = tokio::spawn(async move {
            // Analyze results if possible, no guarantees it will be and if we haven't received
            // enough data we go back to waiting.
            let results = results.clone();
            let expr = &step.expr.expr;
            while notify_rx.recv().await.is_some() {
                let results = results.read().await.clone();
                match eval(expr, &results) {
                    Ok(EvalResult::Known(patui_step_data)) => {
                        do_known_result(patui_step_data, expr, step_name.clone(), tx.clone())
                            .await
                            .unwrap();
                        *done.lock().await = true;
                    }
                    Ok(EvalResult::Predictable(_patui_step_data)) => todo!(),
                    Ok(EvalResult::Unknown) => {}
                    Err(_err) => tx
                        .send(PatuiEvent::new(
                            PatuiEventKind::Failure("Assertion failure".to_string()),
                            step_name.clone(),
                        ))
                        .await
                        .unwrap(),
                }
            }
        });

        self.tasks.push(task);

        let results = self.results.clone();

        let task = tokio::spawn(async move {
            let Some(receivers) = receivers else {
                panic!("No receivers found");
            };
            let results = results;
            let notify_tx = notify_tx;

            let mut tasks = vec![];

            for (expr, mut receiver) in receivers.into_iter() {
                tracing::trace!("Checking receiver for expr: {:?}", expr);

                let results = results.clone();
                let notify_tx = notify_tx.clone();

                tasks.push(tokio::spawn(async move {
                    let results = results.clone();
                    let notify_tx = notify_tx;
                    while let Ok(data) = receiver.recv().await {
                        tracing::trace!("Received data: {:?}", data);
                        let mut lock = results.write().await;
                        let entry = lock.entry(expr.clone()).or_insert(vec![]);
                        entry.push(data);
                        drop(lock);
                        notify_tx.clone().send(()).await.unwrap();
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

    async fn wait(&mut self, tx: mpsc::Sender<PatuiEvent>) -> Result<()> {
        tracing::trace!("Waiting");

        for task in self.tasks.drain(..) {
            tracing::trace!("Awaiting task");
            task.await?;
        }
        tracing::trace!("Done tasks");

        if !*self.done.lock().await {
            let expr = &self.step.expr.expr;
            let results = self.results.read().await.clone();
            tracing::trace!("Checking final results: {:?}", results);
            match eval(expr, &results) {
                Ok(EvalResult::Known(patui_step_data)) => {
                    do_known_result(patui_step_data, expr, self.step_name.clone(), tx.clone())
                        .await
                        .unwrap();
                    *self.done.lock().await = true;
                }
                Ok(EvalResult::Predictable(_patui_step_data)) => todo!(),
                Ok(EvalResult::Unknown) => {}
                Err(_err) => tx
                    .send(PatuiEvent::new(
                        PatuiEventKind::Failure("Assertion failure".to_string()),
                        self.step_name.clone(),
                    ))
                    .await
                    .unwrap(),
            }
        }

        Ok(())
    }

    #[cfg(test)]
    fn test_set_receiver(
        &mut self,
        sub_ref: &str,
        rx: broadcast::Receiver<PatuiStepData>,
    ) -> Result<()> {
        use super::PatuiExpr;

        let sub_ref_expr: PatuiExpr = sub_ref.try_into()?;
        let receivers = HashMap::from([(sub_ref_expr.expr, rx)]);
        self.receivers = Some(receivers);

        Ok(())
    }
}

async fn do_known_result(
    patui_step_data: PatuiStepData,
    expr: &Expr,
    step_name: String,
    tx: mpsc::Sender<PatuiEvent>,
) -> Result<()> {
    match patui_step_data {
        PatuiStepData::Bool(b) => {
            if b {
                tx.send(PatuiEvent::new(
                    PatuiEventKind::Log(format!("Assertion passed: {:?}", expr)),
                    step_name.clone(),
                ))
                .await?;
            } else {
                tx.send(PatuiEvent::new(
                    PatuiEventKind::Failure(format!("Assertion failed: {:?}", expr,)),
                    step_name.clone(),
                ))
                .await?;
            }
        }
        _ => {
            tx.send(PatuiEvent::new(
                PatuiEventKind::Error(format!(
                    "Assertion evaluated to non-boolean: {:?}",
                    patui_step_data
                )),
                step_name.clone(),
            ))
            .await?
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use assertor::*;
    use bytes::Bytes;
    use tokio::{sync::mpsc, time::timeout};
    use tracing_test::traced_test;

    use crate::types::PatuiEventKind;

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

        input_tx.send(PatuiStepData::Null).unwrap();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let res = timeout(Duration::from_millis(500), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(matches!(res.value(), PatuiEventKind::Log(_))).is_equal_to(true);
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
            .send(PatuiStepData::Bytes(Bytes::from("ABC")))
            .unwrap();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let res = timeout(Duration::from_millis(500), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(matches!(res.value(), PatuiEventKind::Failure(_))).is_equal_to(true);
    }

    #[traced_test]
    #[tokio::test]
    async fn assert_step_data_len_zero() {
        let mut main_step = PatuiStepRunnerAssertion::new(
            "main".to_string(),
            &PatuiStepAssertion {
                expr: "steps.test_input.out.len() == 0".try_into().unwrap(),
            },
        );

        let (input_tx, input_rx) = broadcast::channel(32);

        assert_that!(main_step.test_set_receiver("steps.test_input.out", input_rx)).is_ok();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        drop(input_tx);

        let res = timeout(Duration::from_millis(2000), main_step.wait(res_tx.clone())).await;
        assert_that!(res).is_ok();
        assert_that!(res.unwrap()).is_ok();

        let res = timeout(Duration::from_millis(1000), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(res.value()).is_equal_to(&PatuiEventKind::Result(
            "Assertion".to_string(),
            PatuiStepData::Bool(true),
        ));
    }
}
