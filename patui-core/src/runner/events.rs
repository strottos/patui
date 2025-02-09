use std::ops::{Deref, Not};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    expr::PatuiData,
    ptplugin::{PatuiEventEncoding, ResultType as ProtoResultType},
    utils::get_current_timestamp,
    PatuiExpr,
};

#[derive(Debug, Error)]
pub enum PatuiEventError {
    #[error("Called as_result on non Result event kind")]
    NotResult,
}

/// Did the result indicate a failure, this should nearly always be true except for clear cases to
/// the contrary like assertions. If Patui receives false this causes a test to be marked as a
/// failure.
///
/// It may be tempting to think something like a network call failing should be a failure, but this
/// then prevents the user from checking the result of the network call. Instead, the user should
/// create assertions on this based on the test they are interested in, a network call failing
/// might be a legitimate result in some cases. However if some asserts a condition and that
/// condition isn't met then clearly this must mark the test as a failure.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PatuiResultSuccess(bool);

impl Deref for PatuiResultSuccess {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Not for PatuiResultSuccess {
    type Output = PatuiResultSuccess;

    fn not(self) -> Self::Output {
        PatuiResultSuccess(!self.0)
    }
}

impl From<bool> for PatuiResultSuccess {
    fn from(value: bool) -> Self {
        PatuiResultSuccess(value)
    }
}

// TODO: Figure out how best to use this.
/// The result type a plugin is using.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PatuiResultType {
    /// Append the results to the current results.
    Append,
    /// Set the results as the current results.
    Set,
    /// Confirm the results, this is used to confirm that the results are correct and should no
    /// longer be marked as pending.
    Confirm,
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
    ///
    /// The PatuiExpr is the list to append/set results for. The PatuiData is the results.
    /// ResultSuccess indicates whether we should consider this a failure, we recommend this to be
    /// true except for clear cases to the contrary like assertions. The ResultType is generally
    /// one of Append or Set depending on whether we're appending to a list or setting a value in a
    /// dictionary.
    Results(PatuiExpr, PatuiResultSuccess, PatuiResultType, PatuiData),

    /// We want to add a log message. Can be used for debugging, or to show the user what is
    /// happening in the test but cannot be checked in other steps, results should be used for
    /// this.
    Log(String),

    /// An error occurred. This is a hard failure and will stop the test.
    Error(String),

    /// The plugin is done, this must be the last event sent by the plugin, anything after this
    /// will be ignored.
    Done,
}

impl PatuiEvent {
    /// Get the results from the event
    #[cfg(test)]
    pub fn as_results(
        &self,
    ) -> Result<
        (
            &PatuiExpr,
            &PatuiResultSuccess,
            &PatuiResultType,
            &PatuiData,
        ),
        PatuiEventError,
    > {
        match self {
            PatuiEvent::Results(expr, success, result_type, res) => {
                Ok((expr, success, result_type, res))
            }
            _ => Err(PatuiEventError::NotResult),
        }
    }

    /// Check if the event is of type Results
    pub fn is_results(&self) -> bool {
        matches!(self, PatuiEvent::Results(_, _, _, _))
    }

    /// Check if the event is of type Log
    pub fn is_log(&self) -> bool {
        matches!(self, PatuiEvent::Log(_))
    }

    /// Check if the event is of type Failure
    pub fn is_failure(&self) -> bool {
        if let PatuiEvent::Results(_, res, _, _) = self {
            !**res
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
