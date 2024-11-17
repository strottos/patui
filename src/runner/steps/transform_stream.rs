use eyre::{eyre, Result};
use tokio::sync::broadcast;

use crate::types::{PatuiStepTransformStream, PatuiStepTransformStreamFlavour};

use super::{PatuiStepData, PatuiStepDataFlavour, PatuiStepRunnerTrait};

pub(crate) struct PatuiStepRunnerTransformStream {
    step: PatuiStepTransformStream,

    input: (
        broadcast::Receiver<PatuiStepData>,
        broadcast::Sender<PatuiStepData>,
    ),
    output: (
        broadcast::Sender<PatuiStepData>,
        broadcast::Receiver<PatuiStepData>,
    ),
}

impl PatuiStepRunnerTransformStream {
    pub(crate) fn new(step: &PatuiStepTransformStream) -> Self {
        let (input_tx, input_rx) = broadcast::channel(1);
        let (output_tx, output_rx) = broadcast::channel(1);

        Self {
            step: step.clone(),
            input: (input_rx, input_tx),
            output: (output_tx, output_rx),
        }
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerTransformStream {
    fn setup(&mut self) -> Result<()> {
        Ok(())
    }

    fn run(&mut self) -> Result<bool> {
        let input_rx = self.input.1.subscribe();
        let output_tx = self.output.0.clone();

        tokio::spawn(async move {
            let mut input_rx = input_rx;

            while let Ok(chunk) = input_rx.recv().await {
                tracing::trace!("Received data: {:?}", chunk);
                // TODO: Streaming
                let data = match chunk {
                    PatuiStepData {
                        data: PatuiStepDataFlavour::Bytes(data),
                        ..
                    } => PatuiStepData::new(PatuiStepDataFlavour::Json(
                        serde_json::from_slice(&data).unwrap(),
                    )),

                    PatuiStepData {
                        data: PatuiStepDataFlavour::String(data),
                        ..
                    } => PatuiStepData::new(PatuiStepDataFlavour::Json(
                        serde_json::from_str(&data).unwrap(),
                    )),

                    _ => todo!(),
                };
                if let Err(e) = output_tx.send(data) {
                    panic!("Failed to send data");
                }
            }
        });

        Ok(true)
    }

    fn subscribe(&self, sub: &str) -> Result<broadcast::Receiver<PatuiStepData>> {
        match sub {
            "output" => Ok(self.output.0.subscribe()),
            _ => Err(eyre!("Invalid subscription")),
        }
    }

    fn publish(&self, publ: &str, data: PatuiStepData) -> Result<()> {
        match publ {
            "input" => {
                self.input
                    .1
                    .send(data)
                    .map_err(|_| eyre!("Failed to send data"))?;
            }
            _ => return Err(eyre!("Invalid publication")),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use assertor::*;
    use bytes::Bytes;
    use tokio::time::timeout;
    use tracing_test::traced_test;

    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn step_transform_stream_simple_bytes_to_json() {
        let mut step_runner_transform_stream =
            PatuiStepRunnerTransformStream::new(&PatuiStepTransformStream {
                flavour: PatuiStepTransformStreamFlavour::Json,
            });

        let output = step_runner_transform_stream.subscribe("output");

        assert_that!(output).is_ok();
        let mut output = output.unwrap();

        assert_that!(step_runner_transform_stream.setup()).is_ok();
        assert_that!(step_runner_transform_stream.run()).is_ok();

        let input = PatuiStepData::new(PatuiStepDataFlavour::Bytes(Bytes::from(
            "{\"key\": \"value\"}".to_string(),
        )));
        assert_that!(step_runner_transform_stream.publish("input", input.clone())).is_ok();

        let recv = timeout(Duration::from_millis(50), output.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv.data().is_json()).is_true();
        assert_that!(*recv.data()).is_equal_to(PatuiStepDataFlavour::Json(serde_json::json!(
            {"key": "value"}
        )));
    }

    #[traced_test]
    #[tokio::test]
    async fn step_transform_stream_simple_string_to_json() {
        let mut step_runner_transform_stream =
            PatuiStepRunnerTransformStream::new(&PatuiStepTransformStream {
                flavour: PatuiStepTransformStreamFlavour::Json,
            });

        let output = step_runner_transform_stream.subscribe("output");

        assert_that!(output).is_ok();
        let mut output = output.unwrap();

        assert_that!(step_runner_transform_stream.setup()).is_ok();
        assert_that!(step_runner_transform_stream.run()).is_ok();

        let input = PatuiStepData::new(PatuiStepDataFlavour::String(
            "{\"key\": \"value\"}".to_string(),
        ));
        assert_that!(step_runner_transform_stream.publish("input", input.clone())).is_ok();

        let recv = timeout(Duration::from_millis(50), output.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv.data().is_json()).is_true();
        assert_that!(*recv.data()).is_equal_to(PatuiStepDataFlavour::Json(serde_json::json!(
            {"key": "value"}
        )));
    }
}
