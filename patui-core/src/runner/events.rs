use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{expr::PatuiData, ptplugin::PatuiEventEncoding, utils::get_current_timestamp};

use super::{PatuiStepResult, PatuiStepResultStatus};

#[derive(Debug, Error)]
pub enum PatuiEventError {
    #[error("Called as_result on non Result event kind")]
    NotResult,
}

/// An event that can be sent from a plugin to Patui.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PatuiEvent {
    /// General step data that should be stored in the global data store for the step. This can be
    /// anything and doesn't have to be appended to a list like a result.
    ///
    /// TODO: Is this useful? Just an idea currently to prevent having to do append in results.
    /// Does this even work?
    Data(PatuiData),

    /// We received some results for a step and want to append them to the current steps results.
    /// Each step keeps a list of results that are appended to.
    Result(PatuiStepResult),

    /// We want to add a log message. Can be used for debugging, or to show the user what is
    /// happening in the test but cannot be checked in other steps, results should be used for
    /// this.
    Log(String),

    /// An error occurred. This is a hard failure and will stop the test.
    Error(String),
}

impl PatuiEvent {
    /// Check if the event is of type Results
    pub fn is_results(&self) -> bool {
        matches!(self, PatuiEvent::Result(_))
    }

    /// Check if the event is of type Log
    pub fn is_log(&self) -> bool {
        matches!(self, PatuiEvent::Log(_))
    }

    /// Check if the event is of type Failure
    pub fn is_failure(&self) -> bool {
        if let PatuiEvent::Result(result) = self {
            result.success == PatuiStepResultStatus::Fail
        } else {
            false
        }
    }

    /// Check if the event is of type Error
    pub fn is_error(&self) -> bool {
        matches!(self, PatuiEvent::Error(_))
    }
}

/// An event with a timestamp
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PatuiEventWithTimestamp {
    timestamp: i64,
    pub(crate) value: PatuiEvent,
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

impl TryFrom<&PatuiEvent> for PatuiEventEncoding {
    type Error = rmp_serde::encode::Error;

    fn try_from(value: &PatuiEvent) -> Result<Self, Self::Error> {
        Ok(PatuiEventEncoding {
            bytes: rmp_serde::to_vec(value)?,
        })
    }
}

impl TryFrom<PatuiEventEncoding> for PatuiEvent {
    type Error = rmp_serde::decode::Error;

    fn try_from(data: PatuiEventEncoding) -> Result<Self, rmp_serde::decode::Error> {
        rmp_serde::from_read(data.bytes.as_slice())
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
