//! Data types used by the application separate from the database, usually inputs or outputs before
//! they are ready to be put into DB.

pub(crate) mod expr;
pub(crate) mod steps;

use std::io::Read;

use bytes::Bytes;
use edit::edit;
use eyre::Result;
use rusqlite::{
    types::{ToSqlOutput, Value},
    ToSql,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::{PatuiInstance, PatuiRun, PatuiTestDb, PatuiTestId},
    utils::{get_current_time_string, get_current_timestamp},
};

pub(crate) use expr::PatuiExpr;
use steps::PatuiStepEditable;
pub(crate) use steps::{
    PatuiStep, PatuiStepAssertion, PatuiStepData, PatuiStepDataFlavour, PatuiStepDetails,
    PatuiStepRead, PatuiStepSender, PatuiStepTransformStream, PatuiStepWrite,
};

#[cfg(test)]
pub(crate) use steps::PatuiStepTransformStreamFlavour;

pub mod ptplugin {
    tonic::include_proto!("ptplugin");
}

/// PatuiTestEditable is for editing tests before they are saved to the
/// database. This is usually used for sending back to the user for them
/// to edit as strings in something like YAML format.
///
/// We often label things as optional when they have defaults for the
/// `Editable` types.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestEditable {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) steps: Option<Vec<PatuiStepEditable>>,
}

impl From<&PatuiTestDb> for PatuiTestEditable {
    fn from(test: &PatuiTestDb) -> Self {
        PatuiTestEditable {
            name: test.name.clone(),
            description: Some(test.description.clone()),
            steps: Some(test.steps.iter().map(|x| x.into()).collect()),
        }
    }
}

/// PatuiTest is the type used for tests after they have been saved to the
/// database. This is used for running tests and displaying them to the user.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PatuiTest {
    pub(crate) id: PatuiTestId,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) steps: Vec<PatuiStep>,
}

impl PatuiTest {
    pub(crate) fn edit_from_details(test_id: PatuiTestId, details: PatuiTestDetails) -> Self {
        PatuiTest {
            id: test_id,
            name: details.name,
            description: details.description,
            steps: details.steps,
        }
    }
}

impl From<PatuiTestDb> for PatuiTest {
    fn from(value: PatuiTestDb) -> Self {
        PatuiTest {
            id: value.id,
            name: value.name,
            description: value.description,
            steps: value.steps,
        }
    }
}

impl From<&PatuiTestDb> for PatuiTest {
    fn from(value: &PatuiTestDb) -> Self {
        PatuiTest {
            id: value.id.clone(),
            name: value.name.clone(),
            description: value.description.clone(),
            steps: value.steps.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestDetails {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) creation_date: String,
    pub(crate) steps: Vec<PatuiStep>,
}

impl Default for PatuiTestDetails {
    fn default() -> Self {
        let now = get_current_time_string();

        PatuiTestDetails {
            name: "Default".to_string(),
            description: "Default template".to_string(),
            creation_date: now.clone(),
            steps: vec![PatuiStep {
                name: "DefaultProcess".to_string(),
                when: None,
                depends_on: vec![],
                details: PatuiStepDetails::Read(PatuiStepRead {
                    r#in: "\"dir/file.txt\"".try_into().unwrap(),
                }),
            }],
        }
    }
}

impl PatuiTestDetails {
    pub(crate) fn from_yaml_str(yaml: &str) -> Result<Self> {
        let yaml_test = serde_yaml::from_str::<PatuiTestEditable>(yaml)?;

        let now = get_current_time_string();

        let test = PatuiTestDetails {
            name: yaml_test.name,
            description: yaml_test.description.unwrap_or_else(|| "".to_string()),
            creation_date: now,
            steps: yaml_test
                .steps
                .map(|steps| steps.iter().map(|s| s.try_into()).collect())
                .unwrap_or_else(|| Ok(Vec::new()))?,
        };

        Ok(test)
    }

    pub(crate) fn edit_yaml(mut yaml_str: String) -> Result<Self> {
        loop {
            yaml_str = edit(&yaml_str)?;
            match PatuiTestDetails::from_yaml_str(&yaml_str) {
                Ok(details) => {
                    return Ok(details);
                }
                Err(e) => {
                    eprintln!("Failed to parse yaml: {e}\nPress any key to continue editing or Ctrl-C to cancel...");
                    let buffer = &mut [0u8];
                    let _ = std::io::stdin().read_exact(buffer);
                }
            };
        }
    }

    pub(crate) fn to_editable_yaml_string(&self) -> Result<String> {
        let yaml_test = PatuiTestEditable {
            name: self.name.clone(),
            description: Some(self.description.clone()),
            steps: Some(self.steps.iter().map(|step| step.into()).collect()),
        };

        Ok(serde_yaml::to_string(&yaml_test)?)
    }

    pub(crate) fn simple_process() -> PatuiTestDetails {
        // NB: If any of these unwraps don't work the templates need updating,
        // tests should catch this.
        Self::from_yaml_str(include_str!("../templates/simple_process.yaml")).unwrap()
    }

    pub(crate) fn streaming_process() -> PatuiTestDetails {
        Self::from_yaml_str(include_str!("../templates/streaming_process.yaml")).unwrap()
    }

    pub(crate) fn simple_socket() -> PatuiTestDetails {
        todo!()
    }

    pub(crate) fn streaming_socket() -> PatuiTestDetails {
        todo!()
    }

    pub(crate) fn complex_process_and_socket() -> PatuiTestDetails {
        todo!()
    }
}

// Test runs

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunError {}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunStatus {
    Pending,
    Passed,
    Error(PatuiRunError),
}

impl ToSql for PatuiRunStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(match self {
            PatuiRunStatus::Pending => "pending".to_string(),
            PatuiRunStatus::Passed => "passed".to_string(),
            PatuiRunStatus::Error(_) => "error".to_string(),
        })))
    }
}

// Result details

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiEventKind {
    Bytes(Bytes),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiEvent {
    timestamp: i64,
    step_name: String,
    value: PatuiEventKind,
}

impl PatuiEvent {
    pub(crate) fn new(value: PatuiEventKind, step_name: String) -> Self {
        let now = get_current_timestamp();

        PatuiEvent {
            timestamp: now,
            step_name,
            value,
        }
    }

    pub(crate) fn send_bytes(value: Bytes, step_name: String) -> Self {
        PatuiEvent::new(PatuiEventKind::Bytes(value), step_name)
    }

    #[cfg(test)]
    pub(crate) fn value(&self) -> &PatuiEventKind {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunStepResult {
    status: PatuiRunStatus,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunStep {
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) result: PatuiRunStepResult,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunStepDisplay {
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) result: PatuiRunStepResult,
}

impl TryFrom<PatuiRunStep> for PatuiRunStepDisplay {
    type Error = eyre::Report;

    fn try_from(value: PatuiRunStep) -> std::result::Result<Self, Self::Error> {
        Ok(PatuiRunStepDisplay {
            start_time: value.start_time,
            end_time: value.end_time,
            result: value.result,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunDisplay {
    pub(crate) id: i64,
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
            id: value.id.into(),
            instance: value.instance,
            start_time: value.start_time,
            end_time: value.end_time,
            status: value.status,
            step_run_details: value
                .step_run_details
                .into_iter()
                .map(|step| step.try_into())
                .collect::<Result<Vec<PatuiRunStepDisplay>>>()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assertor::*;
    use textwrap::dedent;

    #[test]
    fn test_from_simple_yaml_str() {
        let yaml = dedent(
            r#"
            name: test name
            description: test description
            steps: []
            "#,
        );

        let details = PatuiTestDetails::from_yaml_str(&yaml).unwrap();

        assert_that!(details.name).is_equal_to("test name".to_string());
        assert_that!(details.description).is_equal_to("test description".to_string());
        assert_that!(details.steps).is_empty();
    }

    #[test]
    fn test_from_yaml_str_with_steps() {
        let yaml = dedent(
            r#"
            name: test name
            description: test description
            steps:
              - name: foo
                depends_on: []
                details: !Read
                  in: "\"dir/file.txt\""
              - name: bar
                depends_on: []
                details: !Assertion
                  expr: foo == "bar"
            "#,
        );

        let details = PatuiTestDetails::from_yaml_str(&yaml).unwrap();

        assert_that!(details.name).is_equal_to("test name".to_string());
        assert_that!(details.description).is_equal_to("test description".to_string());
        assert_that!(details.steps).has_length(2);
        assert_that!(details.steps[0].details).is_equal_to(PatuiStepDetails::Read(PatuiStepRead {
            r#in: "\"dir/file.txt\"".try_into().unwrap(),
        }));
        assert_that!(details.steps[1].details).is_equal_to(PatuiStepDetails::Assertion(
            PatuiStepAssertion {
                expr: "foo == \"bar\"".try_into().unwrap(),
            },
        ));
    }

    #[test]
    fn test_from_bad_yaml_str_errors() {
        let yaml = dedent(
            r#"
            foo: bar
            "#,
        );

        let test = PatuiTestDetails::from_yaml_str(&yaml);

        assert_that!(test).is_err();
    }

    #[test]
    fn test_simple_process_yaml() {
        let details = PatuiTestDetails::simple_process();

        assert_that!(details.name).is_equal_to("simple_process".to_string());
        assert_that!(details.steps).has_length(5);
    }

    #[test]
    fn test_streaming_process_yaml() {
        let details = PatuiTestDetails::streaming_process();

        assert_that!(details.name).is_equal_to("streaming_process".to_string());
        assert_that!(details.steps).has_length(9);
    }
}
