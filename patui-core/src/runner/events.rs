use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{expr::PatuiData, utils::get_current_timestamp};

#[derive(Debug, Error)]
pub enum PatuiEventError {
    #[error("Called as_result on non Result event kind")]
    NotResult,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PatuiEventInner {
    Result(String, PatuiData),
    Log(String),
    Failure(String),
    Error(String),
}

impl PatuiEventInner {
    #[cfg(test)]
    pub fn as_result(&self) -> Result<&PatuiData, PatuiEventError> {
        match self {
            PatuiEventInner::Result(_, patui_step_data) => Ok(&patui_step_data),
            _ => Err(PatuiEventError::NotResult),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PatuiEvent {
    step_name: String,
    value: PatuiEventInner,
}

impl PatuiEvent {
    pub(crate) fn new(value: PatuiEventInner, step_name: String) -> Self {
        PatuiEvent { step_name, value }
    }

    pub(crate) fn value(&self) -> &PatuiEventInner {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiEventWithTimestamp {
    timestamp: i64,
    step_name: String,
    value: PatuiEventInner,
}

impl From<PatuiEvent> for PatuiEventWithTimestamp {
    fn from(event: PatuiEvent) -> Self {
        let now = get_current_timestamp();

        PatuiEventWithTimestamp {
            timestamp: now,
            step_name: event.step_name,
            value: event.value,
        }
    }
}

impl From<PatuiEventWithTimestamp> for PatuiEvent {
    fn from(event: PatuiEventWithTimestamp) -> Self {
        PatuiEvent {
            step_name: event.step_name,
            value: event.value,
        }
    }
}
