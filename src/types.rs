//! Data types used in the application, these are the types that are used to interact with the
//! database. Every type that is used in the database should be defined here. This is to ensure
//! that the types are consistent across the application and that the database schema is
//! consistent.

use std::{fmt::Display, io::Read};

use convert_case::{Case, Casing};
use edit::edit;
use eyre::Result;
use rusqlite::{types::ToSqlOutput, ToSql};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter, IntoStaticStr, VariantArray, VariantNames};

// IDs

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Deserialize, Serialize)]
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiId {
    #[default]
    None,
    Test(PatuiTestId),
    Step(PatuiTestId, PatuiTestStepId),
}

impl Display for PatuiId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatuiId::None => write!(f, "None"),
            PatuiId::Test(id) => write!(f, "Test({})", id),
            PatuiId::Step(test_id, step_num) => {
                write!(f, "Step(test_id={}, step={})", test_id, step_num)
            }
        }
    }
}

// Test templates

#[allow(dead_code)]
#[derive(Debug, Clone, Eq, Hash, Deserialize, Serialize)]
pub(crate) struct PatuiTestDetails {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) creation_date: String,
    pub(crate) last_updated: String,
    pub(crate) last_used_date: Option<String>,
    pub(crate) times_used: u32,
    pub(crate) steps: Vec<PatuiStepDetails>,
}

impl Default for PatuiTestDetails {
    fn default() -> Self {
        let now: String = chrono::Local::now().to_string();

        PatuiTestDetails {
            name: "Default".to_string(),
            description: "Default template".to_string(),
            creation_date: now.clone(),
            last_updated: now,
            last_used_date: None,
            times_used: 0,
            steps: vec![PatuiStepDetails::Shell(PatuiStepShell {
                shell: Some("bash".to_string()),
                contents: "echo 'Hello, world!'".to_string(),
                location: None,
            })],
        }
    }
}

// We don't take columns that imply this was used into account, otherwise it's a
// different test.
impl PartialEq for PatuiTestDetails {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.description == other.description
            && self.creation_date == other.creation_date
            && self.last_updated == other.last_updated
            && self.steps == other.steps
    }
}

impl PatuiTestDetails {
    pub(crate) fn from_yaml_str(yaml: &str) -> Result<Self> {
        let yaml_test = serde_yaml::from_str::<PatuiTestEditable>(yaml)?;

        let now: String = chrono::Local::now().to_string();

        let test = PatuiTestDetails {
            name: yaml_test.name,
            description: yaml_test.description,
            creation_date: now.clone(),
            last_updated: now,
            last_used_date: None,
            times_used: 0,
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

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub(crate) struct PatuiTest {
    pub(crate) id: PatuiTestId,
    pub(crate) details: PatuiTestDetails,
}

impl PatuiTest {
    pub(crate) fn to_min_display_test(&self) -> Result<PatuiTestMinDisplay> {
        Ok(PatuiTestMinDisplay {
            id: self.id.clone(),
            name: self.details.name.clone(),
            description: self.details.description.clone(),
        })
    }

    pub(crate) fn to_edited_test(&self, status: String) -> PatuiTestEditStatus {
        PatuiTestEditStatus {
            id: self.id,
            name: Some(self.details.name.clone()),
            description: Some(self.details.description.clone()),
            status,
        }
    }
}

impl Serialize for PatuiTest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("PatuiTest", 8)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("name", &self.details.name)?;
        state.serialize_field("description", &self.details.description)?;
        state.serialize_field("creation_date", &self.details.creation_date)?;
        state.serialize_field("last_updated", &self.details.last_updated)?;
        state.serialize_field("last_used_date", &self.details.last_used_date)?;
        state.serialize_field("times_used", &self.details.times_used)?;
        state.serialize_field("steps", &self.details.steps)?;
        state.end()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestMinDisplay {
    pub(crate) id: PatuiTestId,
    pub(crate) name: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestEditStatus {
    pub(crate) id: PatuiTestId,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) status: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestEditable {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) steps: Vec<PatuiStepDetails>,
}

// Test steps

#[derive(
    Debug,
    Clone,
    Eq,
    PartialEq,
    Hash,
    Deserialize,
    Serialize,
    EnumIter,
    EnumDiscriminants,
    IntoStaticStr,
    VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum PatuiStepDetails {
    Process(PatuiStepProcess),
    Shell(PatuiStepShell),
    Assertion(PatuiStepAssertion),
}

impl PatuiStepDetails {
    pub(crate) fn get_display_yaml(&self) -> Result<Vec<String>> {
        let mut ret = vec![];

        let name: &'static str = self.into();
        ret.push(format!("- {}:", name.to_case(Case::Pascal)));
        let yaml = self.inner_yaml()?;
        yaml.lines().for_each(|line| {
            ret.push(format!("    {}", line));
        });

        Ok(ret)
    }

    pub(crate) fn inner_yaml(&self) -> Result<String> {
        Ok(match self {
            PatuiStepDetails::Process(process) => serde_yaml::to_string(process)?,
            PatuiStepDetails::Shell(shell) => serde_yaml::to_string(shell)?,
            PatuiStepDetails::Assertion(assertion) => serde_yaml::to_string(assertion)?,
        })
    }

    pub(crate) fn to_editable_yaml(&self) -> Result<String> {
        match self {
            PatuiStepDetails::Shell(shell) => Ok(serde_yaml::to_string(&shell.contents)?),
            _ => self.inner_yaml(),
        }
    }

    pub(crate) fn edit_yaml(mut yaml_str: String, step: &PatuiStepDetails) -> Result<Self> {
        loop {
            yaml_str = edit(&yaml_str)?;
            match PatuiStepDetails::from_yaml_str(&yaml_str, step) {
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

    pub(crate) fn from_yaml_str(yaml: &str, step: &PatuiStepDetails) -> Result<Self> {
        Ok(match step {
            PatuiStepDetails::Shell(shell_step) => {
                let contents = serde_yaml::from_str::<PatuiStepShell>(yaml)?;
                PatuiStepDetails::Shell(PatuiStepShell {
                    shell: shell_step.shell.clone(),
                    contents: contents.contents,
                    location: shell_step.location.clone(),
                })
            }
            _ => serde_yaml::from_str::<PatuiStepDetails>(yaml)?,
        })
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub(crate) struct PatuiStepProcess {
    pub(crate) process: String,
    pub(crate) args: Vec<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub(crate) struct PatuiStepShell {
    pub(crate) shell: Option<String>,
    pub(crate) contents: String,
    pub(crate) location: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Hash, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertion {
    pub(crate) assertion: PatuiStepAssertionType,
    pub(crate) negate: bool,
    pub(crate) lhs: String,
    pub(crate) rhs: String,
}

#[derive(
    Debug, Clone, Default, Eq, PartialEq, Hash, Deserialize, Serialize, EnumIter, VariantArray,
)]
pub(crate) enum PatuiStepAssertionType {
    #[default]
    Equal,
    Contains,
}

// Test runs

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiInstance {
    pub(crate) id: PatuiInstanceId,
    pub(crate) test_id: PatuiTestId,
    pub(crate) hash: i64,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) creation_date: String,
    pub(crate) last_updated: String,
    pub(crate) steps: Vec<PatuiStepDetails>,
}

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
        Ok(ToSqlOutput::from(match self {
            PatuiRunStatus::Pending => 0,
            PatuiRunStatus::Ok => 1,
            PatuiRunStatus::Error(_) => 2, // TODO: Errors
        }))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunStepDetails {}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunDetails {
    pub(crate) instance: PatuiInstance,
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) status: PatuiRunStatus,
    pub(crate) step_run_details: Vec<PatuiRunStepDetails>,
}

impl PatuiRunDetails {
    pub(crate) fn new(instance: PatuiInstance) -> Self {
        let now: String = chrono::Local::now().to_string();

        PatuiRunDetails {
            instance,
            start_time: now,
            end_time: None,
            status: PatuiRunStatus::Pending,
            step_run_details: vec![],
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRun {
    pub(crate) id: PatuiRunId,
    pub(crate) details: PatuiRunDetails,
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
        assert_that!(details.steps[0]).is_equal_to(PatuiStepDetails::Shell(PatuiStepShell {
            shell: Some("bash".to_string()),
            contents: "echo 'Hello, world!'".to_string(),
            location: None,
        }));
        assert_that!(details.steps[1]).is_equal_to(PatuiStepDetails::Assertion(
            PatuiStepAssertion {
                assertion: PatuiStepAssertionType::Equal,
                negate: false,
                lhs: "foo".to_string(),
                rhs: "bar".to_string(),
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
}
