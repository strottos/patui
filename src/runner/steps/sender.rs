use std::collections::HashMap;

use bytes::Bytes;
use eyre::{eyre, Result};
use tokio::{sync::broadcast, task::JoinHandle};

use super::PatuiStepRunnerTrait;
use crate::types::{
    expr::ast::{ExprKind, LitKind},
    PatuiEvent, PatuiExpr, PatuiStepData, PatuiStepDataFlavour, PatuiStepSender,
};

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerSender {
    step_name: String,
    step: PatuiStepSender,
    out: Option<(
        broadcast::Sender<PatuiStepData>,
        broadcast::Receiver<PatuiStepData>,
    )>,
    receivers: Option<HashMap<PatuiExpr, broadcast::Receiver<PatuiStepData>>>,
    tasks: Vec<JoinHandle<()>>,
}

impl PatuiStepRunnerSender {
    pub(crate) fn new(step: &PatuiStepSender) -> Self {
        Self {
            step_name: "sender".to_string(),
            step: step.clone(),
            // TODO: Tune this parameter, configurable maybe? Probably should perf test.
            out: Some(broadcast::channel(32)),
            receivers: None,
            tasks: vec![],
        }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerSender {
    fn run(&mut self, tx: tokio::sync::mpsc::Sender<PatuiEvent>) -> Result<()> {
        let step = self.step.clone();
        let step_name = self.step_name.clone();
        let mut out_sender = self.out.as_ref().unwrap().0.clone();

        let task = tokio::spawn(async move {
            tracing::trace!("Running sender step with expr: {:?}", step.expr);
            if let ExprKind::List(elems) = step.expr.kind() {
                for elem in elems {
                    if let ExprKind::Lit(lit) = elem.kind() {
                        let data = match &lit.kind {
                            LitKind::Bool(_) => todo!(),
                            LitKind::Bytes(bytes) => {
                                tracing::trace!("Sending bytes: {:?}", bytes);
                                out_sender
                                    .send(PatuiStepData::new(PatuiStepDataFlavour::Bytes(
                                        bytes.clone(),
                                    )))
                                    .unwrap();

                                tx.send(PatuiEvent::send_bytes(bytes.clone(), step_name.clone()))
                                    .await
                                    .unwrap();
                            }
                            LitKind::Integer(_) => todo!(),
                            LitKind::Decimal(_) => todo!(),
                            LitKind::Str(_) => todo!(),
                            LitKind::Token(_) => todo!(),
                        };
                    } else {
                        todo!();
                    }
                    // Milli sleep to allow channel to process receiving and hopefully prevent
                    // flooding.
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                }
            } else if let ExprKind::Lit(lit) = step.expr.kind() {
                let data = match &lit.kind {
                    LitKind::Bool(_) => todo!(),
                    LitKind::Bytes(bytes) => {
                        out_sender
                            .send(PatuiStepData::new(PatuiStepDataFlavour::Bytes(
                                bytes.clone(),
                            )))
                            .unwrap();

                        tx.send(PatuiEvent::send_bytes(bytes.clone(), step_name))
                            .await
                            .unwrap();
                    }
                    LitKind::Integer(_) => todo!(),
                    LitKind::Decimal(_) => todo!(),
                    LitKind::Str(string) => {
                        out_sender
                            .send(PatuiStepData::new(PatuiStepDataFlavour::String(
                                string.clone(),
                            )))
                            .unwrap();

                        tx.send(PatuiEvent::send_bytes(
                            Bytes::from(string.clone()),
                            step_name,
                        ))
                        .await
                        .unwrap();
                    }
                    LitKind::Token(_) => todo!(),
                };
            } else if let ExprKind::Ident(ident) = step.expr.kind() {
                todo!();
            } else {
                todo!();
            }
            tracing::trace!("Done sending data");
        });

        self.tasks.push(task);

        Ok(())
    }

    fn subscribe(&self, sub: &str) -> Result<broadcast::Receiver<PatuiStepData>> {
        match sub {
            "out" => Ok(self.out.as_ref().unwrap().0.subscribe()),
            _ => Err(eyre!("Invalid subscription {}", sub)),
        }
    }

    async fn wait(&mut self) -> Result<()> {
        tracing::trace!("Waiting");

        for task in self.tasks.drain(..) {
            task.await?;
        }
        self.out = None;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{Arc, Mutex},
        time::Duration,
    };

    use assertor::*;
    use bytes::Bytes;
    use tokio::{sync::mpsc, time::timeout};
    use tracing_test::traced_test;

    use crate::{
        runner::steps::PatuiStepRunner,
        types::{PatuiEventKind, PatuiStep, PatuiStepDetails},
    };

    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn send_single_data() {
        let step = PatuiStepSender {
            expr: "b\"ABC\"".try_into().unwrap(),
        };
        let mut main_step = PatuiStepRunnerSender::new(&step);

        let output_rx = main_step.subscribe("out");

        assert_that!(output_rx).is_ok();
        let mut output_rx = output_rx.unwrap();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx)).is_ok();

        let res = timeout(Duration::from_millis(50), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(res.value()).is_equal_to(&PatuiEventKind::Bytes(Bytes::from("ABC")));

        assert_that!(main_step.wait().await).is_ok();

        let recv = timeout(Duration::from_millis(50), output_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(*recv.data()).is_equal_to(&PatuiStepDataFlavour::Bytes(Bytes::from("ABC")));
    }

    #[traced_test]
    #[tokio::test]
    async fn send_multiple_data() {
        let step = PatuiStepSender {
            expr: "[b\"123\", b\"abc\", b\"ABC\"]".try_into().unwrap(),
        };
        let mut main_step = PatuiStepRunnerSender::new(&step);

        let output_rx = main_step.subscribe("out");

        assert_that!(output_rx).is_ok();
        let mut output_rx = output_rx.unwrap();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let res = timeout(Duration::from_millis(50), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(res.value()).is_equal_to(&PatuiEventKind::Bytes(Bytes::from("123")));
        let res = timeout(Duration::from_millis(50), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(res.value()).is_equal_to(&PatuiEventKind::Bytes(Bytes::from("abc")));
        let res = timeout(Duration::from_millis(50), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(res.value()).is_equal_to(&PatuiEventKind::Bytes(Bytes::from("ABC")));

        assert_that!(main_step.wait().await).is_ok();

        let recv = timeout(Duration::from_millis(50), output_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(*recv.data()).is_equal_to(&PatuiStepDataFlavour::Bytes(Bytes::from("123")));

        let recv = timeout(Duration::from_millis(50), output_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(*recv.data()).is_equal_to(&PatuiStepDataFlavour::Bytes(Bytes::from("abc")));

        let recv = timeout(Duration::from_millis(50), output_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(*recv.data()).is_equal_to(&PatuiStepDataFlavour::Bytes(Bytes::from("ABC")));
    }
}
