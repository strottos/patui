//! Data types used by the application separate from the database, usually inputs or outputs before
//! they are ready to be put into DB.

mod process;
mod transform_stream;

use std::io::Read;

use convert_case::{Case, Casing};
use edit::edit;
use eyre::Result;
use rusqlite::{
    types::{ToSqlOutput, Value},
    ToSql,
};
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter, IntoStaticStr, VariantArray, VariantNames};

use crate::{
    db::{PatuiInstance, PatuiRun, PatuiTestDb, PatuiTestId},
    utils::{get_current_time_string, get_current_timestamp},
};

pub(crate) use process::{PatuiRunStepProcessResult, PatuiStepProcess};
pub(crate) use transform_stream::{PatuiStepTransformStream, PatuiStepTransformStreamFlavour};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestEditable {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) steps: Vec<PatuiStep>,
}

impl From<&PatuiTestDb> for PatuiTestEditable {
    fn from(test: &PatuiTestDb) -> Self {
        PatuiTestEditable {
            name: test.name.clone(),
            description: test.description.clone(),
            steps: test.steps.clone(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
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
            steps: vec![PatuiStep::Process(PatuiStepProcess {
                command: "/usr/bin/env".to_string(),
                args: vec!["ls".to_string(), "/".to_string()],
                tty: Some((24, 80)),
                wait: true,
                input: None,
                cwd: None,
            })],
        }
    }
}

impl PatuiTestDetails {
    pub(crate) fn from_yaml_str(yaml: &str) -> Result<Self> {
        let yaml_test = serde_yaml::from_str::<PatuiTestEditable>(yaml)?;

        let now = get_current_time_string();

        let test = PatuiTestDetails {
            name: yaml_test.name,
            description: yaml_test.description,
            creation_date: now,
            steps: yaml_test.steps,
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
            description: self.description.clone(),
            steps: self.steps.clone(),
        };

        Ok(serde_yaml::to_string(&yaml_test)?)
    }
}

// Test steps

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Deserialize,
    Serialize,
    EnumIter,
    EnumDiscriminants,
    IntoStaticStr,
    VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum PatuiStep {
    Process(PatuiStepProcess),
    TransformStream(PatuiStepTransformStream),
    Shell(PatuiStepShell),

    Assertion(PatuiStepAssertion),
}

impl PatuiStep {
    pub(crate) fn get_display_yaml(&self) -> Result<String> {
        let mut ret = String::new();

        let name: &'static str = self.into();
        ret += &format!("- {}:\n", name.to_case(Case::Pascal));
        let yaml = self.inner_yaml()?;
        yaml.lines().for_each(|line| {
            ret += &format!("    {}\n", line);
        });

        Ok(ret.trim().to_string())
    }

    pub(crate) fn inner_yaml(&self) -> Result<String> {
        Ok(match self {
            PatuiStep::Process(process) => serde_yaml::to_string(process)?,
            PatuiStep::TransformStream(stream) => todo!(),
            PatuiStep::Shell(shell) => serde_yaml::to_string(shell)?,
            PatuiStep::Assertion(assertion) => serde_yaml::to_string(assertion)?,
        })
    }

    pub(crate) fn to_editable_yaml(&self) -> Result<String> {
        match self {
            PatuiStep::Shell(shell) => Ok(serde_yaml::to_string(&shell.contents)?),
            _ => self.inner_yaml(),
        }
    }

    pub(crate) fn edit_yaml(mut yaml_str: String, step: &PatuiStep) -> Result<Self> {
        loop {
            yaml_str = edit(&yaml_str)?;
            match PatuiStep::from_yaml_str(&yaml_str, step) {
                Ok(step) => {
                    return Ok(step);
                }
                Err(e) => {
                    eprintln!("Failed to parse yaml: {e}\nPress any key to continue editing or Ctrl-C to cancel...");
                    let buffer = &mut [0u8];
                    let _ = std::io::stdin().read_exact(buffer);
                }
            };
        }
    }

    pub(crate) fn from_yaml_str(yaml: &str, step: &PatuiStep) -> Result<Self> {
        Ok(match step {
            PatuiStep::Shell(shell_step) => {
                let contents = serde_yaml::from_str::<PatuiStepShell>(yaml)?;
                PatuiStep::Shell(PatuiStepShell {
                    shell: shell_step.shell.clone(),
                    contents: contents.contents,
                    location: shell_step.location.clone(),
                })
            }
            _ => serde_yaml::from_str::<PatuiStep>(yaml)?,
        })
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepShell {
    pub(crate) shell: Option<String>,
    pub(crate) contents: String,
    pub(crate) location: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertion {
    pub(crate) assertion: PatuiStepAssertionType,
    pub(crate) negate: bool,
    pub(crate) lhs: String,
    pub(crate) rhs: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize, EnumIter, VariantArray)]
pub(crate) enum PatuiStepAssertionType {
    #[default]
    Equal,
    Contains,
}

// Test runs

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunError {}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunStatus {
    Pending,
    Ok,
    Error(PatuiRunError),
}

impl ToSql for PatuiRunStatus {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(match self {
            PatuiRunStatus::Pending => "pending".to_string(),
            PatuiRunStatus::Ok => "ok".to_string(),
            PatuiRunStatus::Error(_) => "error".to_string(),
        })))
    }
}

// Result details

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTimestamp<T> {
    timestamp: i64,
    value: T,
}

impl<T> PatuiTimestamp<T> {
    pub(crate) fn new(value: T) -> Self {
        let now = get_current_timestamp();

        PatuiTimestamp {
            timestamp: now,
            value,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunStepResult {
    Process(PatuiRunStepProcessResult),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunStep {
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) result: PatuiRunStepResult,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunStepResultDisplay {
    Process(),
}

impl TryFrom<PatuiRunStepResult> for PatuiRunStepResultDisplay {
    type Error = eyre::Report;

    fn try_from(value: PatuiRunStepResult) -> std::result::Result<Self, Self::Error> {
        match value {
            PatuiRunStepResult::Process(process) => Ok(PatuiRunStepResultDisplay::Process()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunStepDisplay {
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) result: PatuiRunStepResultDisplay,
}

impl TryFrom<PatuiRunStep> for PatuiRunStepDisplay {
    type Error = eyre::Report;

    fn try_from(value: PatuiRunStep) -> std::result::Result<Self, Self::Error> {
        Ok(PatuiRunStepDisplay {
            start_time: value.start_time,
            end_time: value.end_time,
            result: value.result.try_into()?,
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
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
              - !Shell
                shell: bash
                contents: echo 'Hello, world!'
              - !Assertion
                assertion: Equal
                negate: false
                lhs: foo
                rhs: bar
            "#,
        );

        let details = PatuiTestDetails::from_yaml_str(&yaml).unwrap();

        assert_that!(details.name).is_equal_to("test name".to_string());
        assert_that!(details.description).is_equal_to("test description".to_string());
        assert_that!(details.steps).has_length(2);
        assert_that!(details.steps[0]).is_equal_to(PatuiStep::Shell(PatuiStepShell {
            shell: Some("bash".to_string()),
            contents: "echo 'Hello, world!'".to_string(),
            location: None,
        }));
        assert_that!(details.steps[1]).is_equal_to(PatuiStep::Assertion(PatuiStepAssertion {
            assertion: PatuiStepAssertionType::Equal,
            negate: false,
            lhs: "foo".to_string(),
            rhs: "bar".to_string(),
        }));
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
}
