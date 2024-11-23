//! Data types used in the application, these are the types that are used to interact with the
//! database. Every type that is used in the database should be defined here. This is to ensure
//! that the types are consistent across the application and that the database schema is
//! consistent.

use std::{
    fmt::Display,
    ops::{AddAssign, SubAssign},
};

use eyre::Result;
use serde::{ser::SerializeStruct, Deserialize, Serialize};

use crate::types::{
    PatuiRunStatus, PatuiRunStep, PatuiRunStepDisplay, PatuiStep, PatuiTestDetails,
    PatuiTestEditable,
};

// IDs

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestId(i64);

impl Display for PatuiTestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for PatuiTestId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<PatuiTestId> for i64 {
    fn from(value: PatuiTestId) -> i64 {
        value.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestStepId(usize);

impl Display for PatuiTestStepId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<usize> for PatuiTestStepId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl From<PatuiTestStepId> for usize {
    fn from(value: PatuiTestStepId) -> usize {
        value.0
    }
}

impl PartialEq<usize> for PatuiTestStepId {
    fn eq(&self, other: &usize) -> bool {
        self.0 == *other
    }
}

impl AddAssign<usize> for PatuiTestStepId {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl SubAssign<usize> for PatuiTestStepId {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiInstanceId(i64);

impl Display for PatuiInstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for PatuiInstanceId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<PatuiInstanceId> for i64 {
    fn from(value: PatuiInstanceId) -> i64 {
        value.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunId(i64);

impl Display for PatuiRunId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for PatuiRunId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<PatuiRunId> for i64 {
    fn from(value: PatuiRunId) -> i64 {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiId {
    Test(PatuiTestId),
    Step(PatuiTestStepId),
}

impl Display for PatuiId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatuiId::Test(id) => write!(f, "Test({})", id),
            PatuiId::Step(id) => write!(f, "Step({})", id),
        }
    }
}

// Test templates

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PatuiTestDb {
    pub(crate) id: PatuiTestId,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) creation_date: String,
    pub(crate) last_updated: String,
    pub(crate) last_used_date: Option<String>,
    pub(crate) times_used: u32,
    pub(crate) steps: Vec<PatuiStep>,
}

impl From<PatuiTestDb> for PatuiTestDetails {
    fn from(test: PatuiTestDb) -> Self {
        PatuiTestDetails {
            name: test.name,
            description: test.description,
            creation_date: test.creation_date,
            steps: test.steps,
        }
    }
}

impl From<&PatuiTestDb> for PatuiTestDetails {
    fn from(test: &PatuiTestDb) -> Self {
        PatuiTestDetails {
            name: test.name.clone(),
            description: test.description.clone(),
            creation_date: test.creation_date.clone(),
            steps: test.steps.clone(),
        }
    }
}

impl From<PatuiTestDb> for PatuiTestMinDisplay {
    fn from(test: PatuiTestDb) -> Self {
        PatuiTestMinDisplay {
            id: test.id,
            name: test.name,
            description: test.description,
        }
    }
}

impl PatuiTestDb {
    pub(crate) fn new_from_details(id: PatuiTestId, details: PatuiTestDetails) -> Self {
        PatuiTestDb {
            id,
            name: details.name,
            description: details.description,
            creation_date: details.creation_date.clone(),
            last_updated: details.creation_date,
            last_used_date: None,
            times_used: 0,
            steps: details.steps,
        }
    }

    pub(crate) fn to_editable_yaml_string(&self) -> Result<String> {
        let yaml_test: PatuiTestEditable = self.into();

        Ok(serde_yaml::to_string(&yaml_test)?)
    }

    pub(crate) fn into_test_status(self, status: String) -> PatuiTestStatus {
        PatuiTestStatus {
            id: self.id,
            name: Some(self.name),
            description: Some(self.description),
            status,
        }
    }
}

impl Serialize for PatuiTestDb {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PatuiTest", 8)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("description", &self.description)?;
        state.serialize_field("creation_date", &self.creation_date)?;
        state.serialize_field("last_updated", &self.last_updated)?;
        state.serialize_field("last_used_date", &self.last_used_date)?;
        state.serialize_field("times_used", &self.times_used)?;
        state.serialize_field("steps", &self.steps)?;
        state.end()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct PatuiTestHashable<'a> {
    pub(crate) id: PatuiTestId,
    pub(crate) name: &'a str,
    pub(crate) description: &'a str,
    pub(crate) steps: Vec<&'a PatuiStep>,
}

impl<'a> From<&'a PatuiTestDb> for PatuiTestHashable<'a> {
    fn from(test: &'a PatuiTestDb) -> Self {
        PatuiTestHashable {
            id: test.id,
            name: &test.name,
            description: &test.description,
            steps: test.steps.iter().collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestMinDisplay {
    pub(crate) id: PatuiTestId,
    pub(crate) name: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestStatus {
    pub(crate) id: PatuiTestId,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) status: String,
}

// Test runs

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiInstance {
    pub(crate) id: PatuiInstanceId,
    pub(crate) test_id: PatuiTestId,
    pub(crate) hash: i64,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) creation_date: String,
    pub(crate) last_updated: String,
    pub(crate) steps: Vec<PatuiStep>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRun {
    pub(crate) id: PatuiRunId,
    pub(crate) instance: PatuiInstance,
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) status: PatuiRunStatus,
    pub(crate) step_run_details: Vec<PatuiRunStep>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunDisplay {
    pub(crate) id: PatuiRunId,
    pub(crate) instance: PatuiInstance,
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) status: PatuiRunStatus,
    pub(crate) step_run_details: Vec<PatuiRunStepDisplay>,
}

impl TryFrom<PatuiRun> for PatuiRunDisplay {
    type Error = eyre::Report;

    fn try_from(value: PatuiRun) -> std::result::Result<Self, Self::Error> {
        Ok(PatuiRunDisplay {
            id: value.id,
            instance: value.instance,
            start_time: value.start_time,
            end_time: value.end_time,
            status: value.status,
            step_run_details: value
                .step_run_details
                .into_iter()
                .map(|step| step.try_into())
                .collect::<Result<Vec<_>, Self::Error>>()?,
        })
    }
}
