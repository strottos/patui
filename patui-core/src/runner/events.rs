use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{expr::PatuiData, ptplugin::PatuiEventEncoding, utils::get_current_timestamp};

#[derive(Debug, Error)]
pub enum PatuiEventError {
    #[error("Called as_result on non Result event kind")]
    NotResult,
}

/// An event that can be sent from a plugin to Patui.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PatuiEvent {
    /// We found some results for a step
    Results(String, PatuiData),
    /// We want to log an event
    Log(String),
    /// We failed a step
    Failure(String, String),
    /// An error occurred
    Error(String),
}

impl PatuiEvent {
    /// Get the results from the event
    #[cfg(test)]
    pub fn as_result(&self) -> Result<&PatuiData, PatuiEventError> {
        match self {
            PatuiEvent::Results(_, patui_step_data) => Ok(&patui_step_data),
            _ => Err(PatuiEventError::NotResult),
        }
    }
}

/// An event with a timestamp
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PatuiEventWithTimestamp {
    timestamp: i64,
    value: PatuiEvent,
}

impl PatuiEventWithTimestamp {
    /// Get the event from the PatuiEventWithTimestamp
    pub fn value(&self) -> &PatuiEvent {
        &self.value
    }
}

impl From<PatuiEvent> for PatuiEventWithTimestamp {
    fn from(event: PatuiEvent) -> Self {
        let now = get_current_timestamp();

        PatuiEventWithTimestamp {
            timestamp: now,
            value: event,
        }
    }
}

impl TryFrom<PatuiEvent> for PatuiEventEncoding {
    type Error = rmp_serde::encode::Error;

    fn try_from(value: PatuiEvent) -> Result<Self, Self::Error> {
        Ok(PatuiEventEncoding {
            bytes: rmp_serde::to_vec(&value)?,
        })
    }
}

impl TryFrom<PatuiEventEncoding> for PatuiEventWithTimestamp {
    type Error = rmp_serde::decode::Error;

    fn try_from(data: PatuiEventEncoding) -> Result<Self, Self::Error> {
        Ok(PatuiEventWithTimestamp {
            timestamp: get_current_timestamp(),
            value: rmp_serde::from_read(data.bytes.as_slice())?,
        })
    }
}
