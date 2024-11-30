//! Data types used by the application separate from the database, usually inputs or outputs before
//! they are ready to be put into DB.

mod expr;
mod other;
mod process;
mod transform_stream;

use std::io::Read;

use bytes::Bytes;
use convert_case::{Case, Casing};
use edit::edit;
use eyre::{eyre, Result};
use rusqlite::{
    types::{ToSqlOutput, Value},
    ToSql,
};
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, IntoStaticStr, VariantArray, VariantNames};

use crate::{
    db::{PatuiInstance, PatuiRun, PatuiTestDb, PatuiTestId},
    utils::{get_current_time_string, get_current_timestamp},
};

pub(crate) use expr::PatuiExpr;
pub(crate) use other::{
    PatuiStepAssertion, PatuiStepAssertionEditable, PatuiStepRead, PatuiStepReadEditable,
    PatuiStepWrite, PatuiStepWriteEditable,
};
pub(crate) use process::{PatuiRunStepProcessResult, PatuiStepProcess, PatuiStepProcessEditable};
pub(crate) use transform_stream::{
    PatuiStepTransformStream, PatuiStepTransformStreamEditable, PatuiStepTransformStreamFlavour,
};

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
                details: PatuiStepDetails::Process(PatuiStepProcess {
                    command: "/usr/bin/env".to_string(),
                    args: vec!["ls".to_string(), "/".to_string()],
                    tty: Some((24, 80)),
                    wait: true,
                    r#in: None,
                    cwd: None,
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

// Test steps

/// PatuiStepEditable is to endable users ability to edit steps before they
/// are saved to the database, similar to PatuiTestEditable.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepEditable {
    pub(crate) name: String,
    pub(crate) when: Option<Option<String>>,
    pub(crate) depends_on: Option<Vec<PatuiStepEditable>>,
    pub(crate) details: PatuiStepDetailsEditable,
}

impl From<PatuiStep> for PatuiStepEditable {
    fn from(step: PatuiStep) -> Self {
        PatuiStepEditable {
            name: step.name,
            when: Some(step.when),
            depends_on: Some(step.depends_on.into_iter().map(|x| x.into()).collect()),
            details: match step.details {
                PatuiStepDetails::Process(process) => {
                    PatuiStepDetailsEditable::Process(PatuiStepProcessEditable {
                        command: process.command,
                        args: Some(process.args),
                        tty: Some(process.tty),
                        wait: Some(process.wait),
                        r#in: process.r#in.map(|x| Some(x.into())),
                        cwd: process.cwd.map(Some),
                    })
                }
                PatuiStepDetails::TransformStream(stream) => {
                    PatuiStepDetailsEditable::TransformStream(PatuiStepTransformStreamEditable {
                        r#in: stream.r#in.into(),
                        flavour: stream.flavour,
                    })
                }
                PatuiStepDetails::Assertion(assertion) => {
                    PatuiStepDetailsEditable::Assertion(PatuiStepAssertionEditable {
                        expr: assertion.expr.into(),
                    })
                }
                PatuiStepDetails::Read(patui_step_read) => {
                    PatuiStepDetailsEditable::Read(PatuiStepReadEditable {
                        r#in: patui_step_read.r#in.into(),
                    })
                }
                PatuiStepDetails::Write(patui_step_write) => {
                    PatuiStepDetailsEditable::Write(PatuiStepWriteEditable {
                        out: patui_step_write.out.into(),
                    })
                }
            },
        }
    }
}

impl From<&PatuiStep> for PatuiStepEditable {
    fn from(value: &PatuiStep) -> Self {
        PatuiStepEditable {
            name: value.name.clone(),
            when: Some(value.when.clone()),
            depends_on: Some(value.depends_on.iter().map(|x| x.into()).collect()),
            details: match &value.details {
                PatuiStepDetails::Process(process) => {
                    PatuiStepDetailsEditable::Process(PatuiStepProcessEditable {
                        command: process.command.clone(),
                        args: Some(process.args.clone()),
                        tty: Some(process.tty),
                        wait: Some(process.wait),
                        r#in: process.r#in.clone().map(|x| Some(x.into())),
                        cwd: process.cwd.clone().map(|x| Some(x)),
                    })
                }
                PatuiStepDetails::TransformStream(stream) => {
                    PatuiStepDetailsEditable::TransformStream(PatuiStepTransformStreamEditable {
                        r#in: stream.r#in.clone().into(),
                        flavour: stream.flavour.clone(),
                    })
                }
                PatuiStepDetails::Assertion(assertion) => {
                    PatuiStepDetailsEditable::Assertion(PatuiStepAssertionEditable {
                        expr: (&assertion.expr).into(),
                    })
                }
                PatuiStepDetails::Read(patui_step_read) => {
                    PatuiStepDetailsEditable::Read(PatuiStepReadEditable {
                        r#in: (&patui_step_read.r#in).into(),
                    })
                }
                PatuiStepDetails::Write(patui_step_write) => {
                    PatuiStepDetailsEditable::Write(PatuiStepWriteEditable {
                        out: (&patui_step_write.out).into(),
                    })
                }
            },
        }
    }
}

/// PatuiStep is the type used for steps after they have been saved to the
/// database. This is used for running tests and displaying them to the user.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStep {
    pub(crate) name: String,
    pub(crate) when: Option<String>,
    pub(crate) depends_on: Vec<PatuiStep>,
    pub(crate) details: PatuiStepDetails,
}

impl TryFrom<&PatuiStepEditable> for PatuiStep {
    type Error = eyre::Error;

    fn try_from(value: &PatuiStepEditable) -> Result<Self, Self::Error> {
        Ok(PatuiStep {
            name: value.name.clone(),
            when: value.when.clone().unwrap_or(None),
            depends_on: value
                .depends_on
                .as_ref()
                .map(|x| x.iter().map(|x| x.try_into()).collect())
                .unwrap_or_else(|| Ok(Vec::new()))?,
            details: match &value.details {
                PatuiStepDetailsEditable::Process(process) => {
                    PatuiStepDetails::Process(PatuiStepProcess {
                        command: process.command.clone(),
                        args: process.args.clone().unwrap_or_else(Vec::new),
                        tty: process.tty.unwrap_or(None),
                        wait: process.wait.unwrap_or(true),
                        r#in: process
                            .r#in
                            .clone()
                            .unwrap_or(None)
                            .map(|x| x.try_into())
                            .map_or(Ok(None), |x| x.map(Some))?,
                        cwd: process.cwd.clone().map(|x| x.unwrap_or_else(String::new)),
                    })
                }
                PatuiStepDetailsEditable::TransformStream(stream) => {
                    PatuiStepDetails::TransformStream(PatuiStepTransformStream {
                        r#in: (&stream.r#in[..]).try_into()?,
                        flavour: stream.flavour.clone(),
                    })
                }
                PatuiStepDetailsEditable::Assertion(assertion) => {
                    PatuiStepDetails::Assertion(PatuiStepAssertion {
                        expr: (&assertion.expr[..]).try_into()?,
                    })
                }
                PatuiStepDetailsEditable::Read(patui_step_read_editable) => {
                    PatuiStepDetails::Read(PatuiStepRead {
                        r#in: (&patui_step_read_editable.r#in[..]).try_into()?,
                    })
                }
                PatuiStepDetailsEditable::Write(patui_step_write_editable) => {
                    PatuiStepDetails::Write(PatuiStepWrite {
                        out: (&patui_step_write_editable.out[..]).try_into()?,
                    })
                }
            },
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDetailsEditable {
    Process(PatuiStepProcessEditable),
    TransformStream(PatuiStepTransformStreamEditable),
    Read(PatuiStepReadEditable),
    Write(PatuiStepWriteEditable),
    Assertion(PatuiStepAssertionEditable),
}

#[derive(
    Debug, Clone, PartialEq, Deserialize, Serialize, EnumDiscriminants, IntoStaticStr, VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum PatuiStepDetails {
    Process(PatuiStepProcess),
    TransformStream(PatuiStepTransformStream),
    Read(PatuiStepRead),
    Write(PatuiStepWrite),
    Assertion(PatuiStepAssertion),
}

impl PatuiStepDetails {
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
            PatuiStepDetails::Process(process) => serde_yaml::to_string(process)?,
            PatuiStepDetails::TransformStream(stream) => serde_yaml::to_string(stream)?,
            PatuiStepDetails::Assertion(assertion) => serde_yaml::to_string(assertion)?,
            PatuiStepDetails::Read(patui_step_read) => serde_yaml::to_string(patui_step_read)?,
            PatuiStepDetails::Write(patui_step_write) => serde_yaml::to_string(patui_step_write)?,
        })
    }

    pub(crate) fn to_editable_yaml(&self) -> Result<String> {
        match self {
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
            _ => serde_yaml::from_str::<PatuiStepDetails>(yaml)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepData {
    pub(crate) timestamp: chrono::DateTime<chrono::Utc>,
    pub(crate) data: PatuiStepDataFlavour,
}

impl PatuiStepData {
    pub(crate) fn new(data: PatuiStepDataFlavour) -> Self {
        let timestamp = chrono::Utc::now();
        Self { timestamp, data }
    }

    pub(crate) fn into_data(self) -> PatuiStepDataFlavour {
        self.data
    }

    pub(crate) fn data(&self) -> &PatuiStepDataFlavour {
        &self.data
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDataFlavour {
    Bytes(Bytes),
    String(String),
    Number(i64),
    Json(serde_json::Value),
    Yaml(serde_yaml::Value),
}

impl PatuiStepDataFlavour {
    pub(crate) fn as_bytes(&self) -> Result<&Bytes> {
        match self {
            Self::Bytes(bytes) => Ok(bytes),
            _ => Err(eyre!("not bytes")),
        }
    }

    pub(crate) fn as_number(&self) -> Result<i64> {
        match self {
            Self::Number(number) => Ok(*number),
            _ => Err(eyre!("not number")),
        }
    }

    pub(crate) fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }

    pub(crate) fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }

    pub(crate) fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }

    pub(crate) fn is_json(&self) -> bool {
        matches!(self, Self::Json(_))
    }

    pub(crate) fn is_yaml(&self) -> bool {
        matches!(self, Self::Yaml(_))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDataTransfer {
    #[default]
    None,
    Fixed(PatuiStepDataFlavour),
    Ref(Box<(PatuiStep, String)>),
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunStepResult {
    Process(PatuiRunStepProcessResult),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiRunStep {
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) result: PatuiRunStepResult,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
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
    use self::expr::PatuiExpr;

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
                details: !Process
                  command: /bin/cat
                  in: "\"Hello, world!\""
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
        assert_that!(details.steps[0].details).is_equal_to(PatuiStepDetails::Process(
            PatuiStepProcess {
                command: "/bin/cat".to_string(),
                args: vec![],
                tty: None,
                wait: true,
                r#in: Some("\"Hello, world!\"".try_into().unwrap()),
                cwd: None,
            },
        ));
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
