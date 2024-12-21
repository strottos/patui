use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use eyre::{eyre, Result};
use tokio::{
    sync::{broadcast, mpsc},
    task::JoinHandle,
};

use crate::types::{
    expr::ast::{Expr, ExprKind},
    PatuiEvent, PatuiEventKind, PatuiStepData, PatuiStepDataFlavour, PatuiStepTransformStream,
};

use super::{init_subscribe_steps, PatuiStepRunner, PatuiStepRunnerTrait};

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerTransformStream {
    step_name: String,
    step: PatuiStepTransformStream,

    out: Option<(
        broadcast::Sender<PatuiStepData>,
        broadcast::Receiver<PatuiStepData>,
    )>,
    receivers: Option<HashMap<Expr, broadcast::Receiver<PatuiStepData>>>,

    tasks: Vec<JoinHandle<()>>,
}

impl PatuiStepRunnerTransformStream {
    pub(crate) fn new(step_name: String, step: &PatuiStepTransformStream) -> Self {
        Self {
            step: step.clone(),
            step_name,
            out: Some(broadcast::channel(1)),
            receivers: None,
            tasks: vec![],
        }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerTransformStream {
    async fn init(
        &mut self,
        current_step_name: &str,
        step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
        let receivers =
            init_subscribe_steps(&self.step.r#in, current_step_name, &step_runners).await?;
        self.receivers = Some(receivers);

        Ok(())
    }

    fn run(&mut self, tx: mpsc::Sender<PatuiEvent>) -> Result<()> {
        let step = self.step.clone();
        let step_name = self.step_name.clone();

        let out_sender = self.out.as_ref().unwrap().0.clone();
        let receivers = self.receivers.take();

        let task = tokio::spawn(async move {
            if matches!(step.r#in.expr.kind(), ExprKind::Term(_)) {
                tracing::trace!("Reading from step: {:?}", step.r#in);
                let Some(mut receivers) = receivers else {
                    panic!("No receivers found");
                };
                let receiver = receivers.get_mut(&step.r#in.expr).unwrap();

                while let Ok(chunk) = receiver.recv().await {
                    let data = match chunk {
                        PatuiStepData {
                            data: PatuiStepDataFlavour::Bytes(data),
                            ..
                        } => PatuiStepData::new(
                            serde_json::from_slice::<serde_json::Value>(&data)
                                .unwrap()
                                .try_into()
                                .unwrap(),
                        ),

                        PatuiStepData {
                            data: PatuiStepDataFlavour::String(data),
                            ..
                        } => PatuiStepData::new(
                            serde_json::from_str::<serde_json::Value>(&data)
                                .unwrap()
                                .try_into()
                                .unwrap(),
                        ),

                        _ => todo!(),
                    };

                    out_sender.send(data.clone()).unwrap();

                    tx.send(PatuiEvent::new(
                        PatuiEventKind::Log("Sent JSON".to_string()),
                        step_name.clone(),
                    ))
                    .await
                    .unwrap();
                }
            } else {
                panic!(
                    "Expression not supported for transforming streams: {}",
                    step.r#in
                );
            }
        });

        self.tasks.push(task);

        Ok(())
    }

    async fn subscribe(&mut self, sub: &str) -> Result<broadcast::Receiver<PatuiStepData>> {
        match sub {
            "out" => Ok(self.out.as_ref().unwrap().0.subscribe()),
            _ => Err(eyre!("Invalid subscription")),
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use assertor::*;
    use bytes::Bytes;
    use tokio::time::timeout;
    use tracing_test::traced_test;

    use crate::types::PatuiStepTransformStreamFlavour;

    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn step_transform_stream_simple_bytes_to_json() {
        let mut main_step = PatuiStepRunnerTransformStream::new(
            "main".to_string(),
            &PatuiStepTransformStream {
                flavour: PatuiStepTransformStreamFlavour::Json,
                r#in: "steps.test_input.out".try_into().unwrap(),
            },
        );

        let output_rx = main_step.subscribe("out").await;

        assert_that!(output_rx).is_ok();
        let mut output_rx = output_rx.unwrap();

        let (input_tx, input_rx) = broadcast::channel(32);

        assert_that!(main_step.test_set_receiver("steps.test_input.out", input_rx)).is_ok();

        input_tx
            .send(PatuiStepData::new(PatuiStepDataFlavour::Bytes(
                Bytes::from(r#"{"key": "value"}"#),
            )))
            .unwrap();

        let (res_tx, _) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let recv = timeout(Duration::from_millis(50), output_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv.data().is_object()).is_true();
        assert_that!(*recv.data()).is_equal_to(PatuiStepDataFlavour::Map(HashMap::from([(
            "key".into(),
            PatuiStepDataFlavour::String("value".into()),
        )])));
    }

    #[traced_test]
    #[tokio::test]
    async fn step_transform_stream_simple_string_to_json() {
        let mut main_step = PatuiStepRunnerTransformStream::new(
            "main".to_string(),
            &PatuiStepTransformStream {
                flavour: PatuiStepTransformStreamFlavour::Json,
                r#in: "steps.test_input.out".try_into().unwrap(),
            },
        );

        let output_rx = main_step.subscribe("out").await;

        assert_that!(output_rx).is_ok();
        let mut output_rx = output_rx.unwrap();

        let (input_tx, input_rx) = broadcast::channel(32);

        assert_that!(main_step.test_set_receiver("steps.test_input.out", input_rx)).is_ok();

        input_tx
            .send(PatuiStepData::new(PatuiStepDataFlavour::String(
                r#"{"key": "value"}"#.to_string(),
            )))
            .unwrap();

        let (res_tx, _res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let recv = timeout(Duration::from_millis(50), output_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv.data().is_object()).is_true();
        assert_that!(*recv.data()).is_equal_to(PatuiStepDataFlavour::Map(HashMap::from([(
            "key".into(),
            PatuiStepDataFlavour::String("value".into()),
        )])));
    }
}
