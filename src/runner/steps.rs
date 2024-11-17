mod assertions;
mod process;
mod transform_stream;

use bytes::Bytes;
use eyre::{eyre, Result};
use process::PatuiStepRunnerProcess;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use transform_stream::PatuiStepRunnerTransformStream;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepData {
    timestamp: chrono::DateTime<chrono::Utc>,
    data: PatuiStepDataFlavour,
}

impl PatuiStepData {
    fn new(data: PatuiStepDataFlavour) -> Self {
        let timestamp = chrono::Utc::now();
        Self { timestamp, data }
    }

    fn into_data(self) -> PatuiStepDataFlavour {
        self.data
    }

    fn data(&self) -> &PatuiStepDataFlavour {
        &self.data
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDataFlavour {
    Bytes(Bytes),
    String(String),
    Number(i64),
    Json(serde_json::Value),
    Yaml(serde_yaml::Value),
}

impl PatuiStepDataFlavour {
    fn as_bytes(&self) -> Result<&Bytes> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            _ => Err(eyre!("not bytes")),
        }
    }

    fn as_number(&self) -> Result<i64> {
        match self {
            Self::Number(number) => Ok(*number),
            _ => Err(eyre!("not number")),
        }
    }

    fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }

    fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    fn is_yaml(&self) -> bool {
        matches!(self, Self::Yaml(_))
    }
}

pub(crate) enum PatuiStepRunnerFlavour {
    Process(PatuiStepRunnerProcess),
    TransformStream(PatuiStepRunnerTransformStream),
}

pub(crate) struct PatuiStepRunner {
    flavour: PatuiStepRunnerFlavour,
}

pub(crate) trait PatuiStepRunnerTrait {
    fn setup(&mut self) -> Result<()> {
        Ok(())
    }

    fn run(&mut self) -> Result<bool> {
        Ok(true)
    }

    fn subscribe(&self, sub: &str) -> Result<broadcast::Receiver<PatuiStepData>> {
        Err(eyre!("Subscription not supported"))
    }

    fn publish(&self, publ: &str, data: PatuiStepData) -> Result<()> {
        Err(eyre!("Publishing not supported"))
    }

    async fn wait(&mut self, action: &str) -> Result<PatuiStepData> {
        Err(eyre!("Waiting not supported"))
    }

    fn check(&mut self, action: &str) -> Result<PatuiStepData> {
        Err(eyre!("Checking not supported"))
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use super::*;

    #[test]
    fn step_process() {}
}
