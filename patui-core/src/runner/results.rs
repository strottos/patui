use std::ops::{Deref, Not};

use serde::{Deserialize, Serialize};

use crate::{ptplugin::PatuiResultEncoding, PatuiData, PatuiExpr};

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
pub enum PatuiStepResultStatus {
    /// Test passed
    Success,
    /// Test failed
    Fail,
    /// Test had an error occurd
    Error,
}

impl Deref for PatuiStepResultStatus {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        match self {
            PatuiStepResultStatus::Success => &true,
            PatuiStepResultStatus::Fail => &false,
            PatuiStepResultStatus::Error => &false,
        }
    }
}

impl Not for PatuiStepResultStatus {
    type Output = PatuiStepResultStatus;

    fn not(self) -> Self::Output {
        match self {
            PatuiStepResultStatus::Success => PatuiStepResultStatus::Fail,
            PatuiStepResultStatus::Fail => PatuiStepResultStatus::Success,
            PatuiStepResultStatus::Error => panic!("Cannot negate error"),
        }
    }
}

impl From<bool> for PatuiStepResultStatus {
    fn from(value: bool) -> Self {
        if value {
            PatuiStepResultStatus::Success
        } else {
            PatuiStepResultStatus::Fail
        }
    }
}

// TODO: Figure out how best to use this.
/// The result type a plugin is using.
///
/// DoneList and DoneMap are sent when the plugin is done, this must be the last event sent by the
/// plugin, anything after this will be ignored by Patui. We also specify the expected size of the
/// list or the expected keys to ensure we received all the results we expected.
///
/// When (and if) a plugin finishes it must confirm the expected size of the list or the keys
/// expected to have been set. If Patui finds something unexpected and these are not the case then
/// Patui will mark the test having errored.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PatuiStepResultInner {
    /// Set the results in a list, argument is the index.
    StreamData(usize, PatuiData),
    /// Set the results as a map, argument is the key.
    MapElement(String, PatuiData),
    /// Confirm the expected size of the list.
    DoneStream(usize),
    /// Confirm the keys expected to have been set for a map.
    DoneMap(Vec<String>),
}

impl PatuiStepResultInner {
    /// Returns true if the result is a DoneStream and the size of the stream, otherwise returns
    /// false and 0.
    pub fn is_done_stream(&self) -> (bool, usize) {
        match self {
            PatuiStepResultInner::DoneStream(stream_len) => (true, *stream_len),
            _ => (false, 0),
        }
    }
}

/// A result from running a step.
///
/// The PatuiExpr is the list to append/set results for. ResultSuccess indicates whether we should
/// consider this a failure, we recommend this to be true except for clear cases to the contrary
/// like assertions. The ResultType contains the results to set and information about the type of
/// result we are setting.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PatuiStepResult {
    pub(crate) expr: PatuiExpr,
    pub(crate) success: PatuiStepResultStatus,
    pub(crate) details: PatuiStepResultInner,
}

impl PatuiStepResult {
    /// Create a new list item result
    pub fn new_stream_item(
        expr: PatuiExpr,
        success: PatuiStepResultStatus,
        index: usize,
        data: PatuiData,
    ) -> Self {
        PatuiStepResult {
            expr,
            success,
            details: PatuiStepResultInner::StreamData(index, data),
        }
    }

    /// Create a new map item result
    pub fn new_map_item(
        expr: PatuiExpr,
        success: PatuiStepResultStatus,
        key: String,
        data: PatuiData,
    ) -> Self {
        PatuiStepResult {
            expr,
            success,
            details: PatuiStepResultInner::MapElement(key, data),
        }
    }

    /// Create a new list done result
    pub fn done_stream(expr: PatuiExpr, success: PatuiStepResultStatus, size: usize) -> Self {
        PatuiStepResult {
            expr,
            success,
            details: PatuiStepResultInner::DoneStream(size),
        }
    }

    /// Create a new map done result
    pub fn done_map(expr: PatuiExpr, success: PatuiStepResultStatus, keys: Vec<String>) -> Self {
        PatuiStepResult {
            expr,
            success,
            details: PatuiStepResultInner::DoneMap(keys),
        }
    }

    /// Get the expression from the result
    pub fn expr(&self) -> &PatuiExpr {
        &self.expr
    }

    /// Get the success from the result
    pub fn success(&self) -> &PatuiStepResultStatus {
        &self.success
    }

    /// Get the details from the result
    pub fn details(&self) -> &PatuiStepResultInner {
        &self.details
    }
}

impl TryFrom<PatuiStepResult> for PatuiResultEncoding {
    type Error = rmp_serde::encode::Error;

    fn try_from(value: PatuiStepResult) -> Result<Self, Self::Error> {
        Ok(PatuiResultEncoding {
            bytes: rmp_serde::to_vec(&value)?,
        })
    }
}

impl TryFrom<PatuiResultEncoding> for PatuiStepResult {
    type Error = rmp_serde::decode::Error;

    fn try_from(data: PatuiResultEncoding) -> Result<Self, Self::Error> {
        rmp_serde::from_read(data.bytes.as_slice())
    }
}
