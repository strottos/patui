//! Data types used by the application separate from the database, usually inputs or outputs before
//! they are ready to be put into DB.

use std::io::Read;

use convert_case::{Case, Casing};
use edit::edit;
use eyre::Result;
use rusqlite::{types::ToSqlOutput, ToSql};
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter, IntoStaticStr, VariantArray, VariantNames};

use crate::db::PatuiInstance;

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiTestEditable {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) steps: Vec<PatuiStep>,
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
        let now: String = chrono::Local::now().to_string();

        PatuiTestDetails {
            name: "Default".to_string(),
            description: "Default template".to_string(),
            creation_date: now.clone(),
            steps: vec![PatuiStep::Process(PatuiStepProcess {
                process: "/usr/bin/env".to_string(),
                args: vec!["ls".to_string(), "/".to_string()],
            })],
        }
    }
}

impl PatuiTestDetails {
    pub(crate) fn from_yaml_str(yaml: &str) -> Result<Self> {
        let yaml_test = serde_yaml::from_str::<PatuiTestEditable>(yaml)?;

        let now: String = chrono::Local::now().to_string();

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
    Shell(PatuiStepShell),
    Assertion(PatuiStepAssertion),
}

impl PatuiStep {
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
            PatuiStep::Process(process) => serde_yaml::to_string(process)?,
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
pub(crate) struct PatuiStepProcess {
    pub(crate) process: String,
    pub(crate) args: Vec<String>,
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
        Ok(ToSqlOutput::Owned(rusqlite::types::Value::Text(
            match self {
                PatuiRunStatus::Pending => "pending".to_string(),
                PatuiRunStatus::Ok => "ok".to_string(),
                PatuiRunStatus::Error(_) => "error".to_string(),
            },
        )))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunStepProcess {
    pub(crate) stdout: String,
    pub(crate) stderr: Vec<String>,
    pub(crate) exit_code: i32,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunStep {
    start_time: String,
    end_time: String,
    details: PatuiRunStepDetails,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunStepDetails {
    Process(PatuiRunStepProcess),
}
