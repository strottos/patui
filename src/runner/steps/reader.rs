use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use eyre::{eyre, Result};
use futures::StreamExt;
use tokio::{
    fs::File,
    io::BufReader,
    sync::{broadcast, mpsc},
    task::JoinHandle,
};
use tokio_util::io::ReaderStream;

use super::{init_subscribe_steps, PatuiStepRunner, PatuiStepRunnerTrait};
use crate::types::{
    expr::ast::{ExprKind, LitKind},
    PatuiEvent, PatuiExpr, PatuiStepData, PatuiStepDataFlavour, PatuiStepRead,
};

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerRead {
    step_name: String,
    step: PatuiStepRead,

    out: Option<(
        broadcast::Sender<PatuiStepData>,
        broadcast::Receiver<PatuiStepData>,
    )>,
    receivers: Option<HashMap<PatuiExpr, broadcast::Receiver<PatuiStepData>>>,

    tasks: Vec<JoinHandle<()>>,
}

impl PatuiStepRunnerRead {
    pub(crate) fn new(step_name: String, step: &PatuiStepRead) -> Self {
        Self {
            step_name,
            step: step.clone(),
            out: Some(broadcast::channel(1)),
            receivers: None,
            tasks: vec![],
        }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerRead {
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
            if matches!(step.r#in.kind(), ExprKind::Term(_)) {
                tracing::trace!("Reading from step: {:?}", step.r#in);
                let Some(mut receivers) = receivers else {
                    panic!("No receivers found");
                };
                let receiver = receivers.get_mut(&step.r#in).unwrap();

                let binding = receiver.recv().await.unwrap();
                let data = binding.data.as_bytes().unwrap();

                out_sender
                    .send(PatuiStepData::new(PatuiStepDataFlavour::Bytes(
                        data.clone(),
                    )))
                    .unwrap();

                tx.send(PatuiEvent::send_bytes(data.clone(), step_name))
                    .await
                    .unwrap();
            } else if let ExprKind::Lit(lit) = step.r#in.kind() {
                let file_name = match &lit.kind {
                    LitKind::Str(file_name) => file_name.clone(),
                    _ => unreachable!(),
                };

                tracing::trace!("Reading from file: {:?}", file_name);

                let mut reader =
                    ReaderStream::new(BufReader::new(File::open(file_name).await.unwrap()));

                while let Some(data) = reader.next().await {
                    tracing::trace!("Read data: {:?}", data);

                    let data = data.unwrap();

                    out_sender
                        .send(PatuiStepData::new(PatuiStepDataFlavour::Bytes(
                            data.clone(),
                        )))
                        .unwrap();

                    tx.send(PatuiEvent::send_bytes(data, step_name.clone()))
                        .await
                        .unwrap();
                }
            } else {
                panic!("Expression not supported for reader: {}", step.r#in);
            }
        });

        self.tasks.push(task);

        Ok(())
    }

    async fn subscribe(&mut self, sub: &str) -> Result<broadcast::Receiver<PatuiStepData>> {
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

    use crate::types::PatuiEventKind;

    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn read_from_another_step() {
        let mut main_step = PatuiStepRunnerRead::new(
            "main".to_string(),
            &PatuiStepRead {
                r#in: "steps.test_input.out".try_into().unwrap(),
            },
        );

        let output_rx = main_step.subscribe("out").await;

        assert_that!(output_rx).is_ok();
        let mut output_rx = output_rx.unwrap();

        let (input_tx, input_rx) = broadcast::channel(32);

        assert_that!(main_step.test_set_receiver("steps.test_input.out", input_rx)).is_ok();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        input_tx
            .send(PatuiStepData::new(PatuiStepDataFlavour::Bytes(
                Bytes::from("This string gets sent by the test send data step"),
            )))
            .unwrap();

        let res = timeout(Duration::from_millis(50), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(res.value()).is_equal_to(&PatuiEventKind::Bytes(Bytes::from(
            "This string gets sent by the test send data step",
        )));

        let recv = timeout(Duration::from_millis(50), output_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(*recv.data()).is_equal_to(&PatuiStepDataFlavour::Bytes(Bytes::from(
            "This string gets sent by the test send data step",
        )));

        drop(input_tx);
        drop(res_rx);

        assert_that!(main_step.wait().await).is_ok();
    }

    #[traced_test]
    #[tokio::test]
    async fn read_from_file() {
        let step = PatuiStepRead {
            r#in: "\"tests/data/test.txt\"".try_into().unwrap(),
        };
        let mut main_step = PatuiStepRunnerRead::new("main".to_string(), &step);

        let output_rx = main_step.subscribe("out").await;

        assert_that!(output_rx).is_ok();
        let mut output_rx = output_rx.unwrap();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let res = timeout(Duration::from_millis(50), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_some();
        let res = res.unwrap();
        assert_that!(res.value()).is_equal_to(&PatuiEventKind::Bytes(Bytes::from(
            "Hello, World!\nStuffmore\n",
        )));

        let recv = timeout(Duration::from_millis(50), output_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(*recv.data()).is_equal_to(&PatuiStepDataFlavour::Bytes(Bytes::from(
            "Hello, World!\nStuffmore\n",
        )));

        drop(res_rx);

        assert_that!(main_step.wait().await).is_ok();
    }
}
