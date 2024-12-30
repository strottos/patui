use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use strum_macros::{EnumDiscriminants, IntoStaticStr, VariantNames};
use thiserror::Error;

use crate::expr::{PatuiExpr, PatuiExprError};

#[derive(Debug, thiserror::Error)]
pub enum PatuiTestError {
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Step parsing error: {0}")]
    Step(#[from] PatuiStepError),
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
    pub(crate) description: Option<Option<String>>,
    pub(crate) steps: Option<Vec<PatuiStepEditable>>,
}

/// PatuiTest is all the details needed to run a test by Patui. This doesn't contain details about
/// storage, for example in a database.
///
/// The main interest is the `steps` field, which is a list of steps that need to be run in order.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct PatuiTest {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) steps: Vec<PatuiStep>,
}

impl Default for PatuiTest {
    fn default() -> Self {
        PatuiTest {
            name: "Default".to_string(),
            description: Some("Default template".to_string()),
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

impl PatuiTest {
    /// Get the name of the test
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the description of the test
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// This function is used to convert a YAML string into a `PatuiTest` struct.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, PatuiTestError> {
        let yaml_test = serde_yaml::from_str::<PatuiTestEditable>(yaml)?;

        let test = PatuiTest {
            name: yaml_test.name,
            description: yaml_test
                .description
                .unwrap_or_else(|| Some("".to_string())),
            steps: yaml_test
                .steps
                .map(|steps| steps.iter().map(|s| s.try_into()).collect())
                .unwrap_or_else(|| Ok(Vec::new()))?,
        };

        Ok(test)
    }

    /// Converts a `PatuiTest` struct into a YAML string.
    pub fn to_editable_yaml_string(&self) -> Result<String, PatuiTestError> {
        let yaml_test = PatuiTestEditable {
            name: self.name.clone(),
            description: Some(self.description.clone()),
            steps: Some(self.steps.iter().map(|step| step.into()).collect()),
        };

        Ok(serde_yaml::to_string(&yaml_test)?)
    }

    /// Gives us the the `simple_process` YAML template as a static string
    pub fn simple_process() -> &'static str {
        include_str!("../../templates/simple_process.yaml")
    }

    /// Gives us the the `streaming_process` YAML template as a static string
    pub fn streaming_process() -> &'static str {
        include_str!("../../templates/streaming_process.yaml")
    }

    /// Gives us the the `simple_socket` YAML template as a static string
    pub fn simple_socket() -> &'static str {
        todo!()
    }

    /// Gives us the the `streaming_socket` YAML template as a static string
    pub fn streaming_socket() -> &'static str {
        todo!()
    }

    /// Gives us the the `complex_process_and_socket` YAML template as a static string
    pub fn complex_process_and_socket() -> &'static str {
        todo!()
    }
}

impl TryFrom<&str> for PatuiTest {
    type Error = PatuiTestError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        PatuiTest::from_yaml_str(value)
    }
}

impl TryFrom<PatuiTest> for String {
    type Error = PatuiTestError;

    fn try_from(value: PatuiTest) -> Result<Self, Self::Error> {
        value.to_editable_yaml_string()
    }
}

impl TryFrom<&PatuiTest> for String {
    type Error = PatuiTestError;

    fn try_from(value: &PatuiTest) -> Result<Self, Self::Error> {
        value.to_editable_yaml_string()
    }
}

#[derive(Debug, Error)]
pub enum PatuiStepError {
    #[error("Error converting step: {0}")]
    Conversion(#[from] PatuiExprError),
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
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

#[derive(
    Debug, Clone, PartialEq, Deserialize, Serialize, EnumDiscriminants, IntoStaticStr, VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub(crate) enum PatuiStepDetails {
    Read(PatuiStepRead),
    Write(PatuiStepWrite),
    Sender(PatuiStepSender),
    Assertion(PatuiStepAssertion),
    Plugin(PatuiStepPlugin),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepRead {
    pub(crate) r#in: PatuiExpr,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepWrite {
    pub(crate) out: PatuiExpr,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertion {
    pub(crate) expr: PatuiExpr,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepSender {
    pub(crate) expr: PatuiExpr,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepPlugin {
    pub(crate) path: String, // TODO: Find a better solution when we're publishing plugins
    pub(crate) config: HashMap<String, PatuiExpr>,
    pub(crate) r#in: HashMap<String, PatuiExpr>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepTransformStreamFlavour {
    Utf8,
    #[default]
    Utf8Lines,
    Json,
    Yaml,
    Toml,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepTransformStream {
    pub(crate) r#in: PatuiExpr,
    pub(crate) flavour: PatuiStepTransformStreamFlavour,
}

// Editable types

/// PatuiStepEditable is to endable users ability to edit steps before they
/// are saved to the database, similar to PatuiTestEditable.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepEditable {
    pub(crate) name: String,
    pub(crate) when: Option<Option<String>>,
    pub(crate) depends_on: Option<Vec<PatuiStepEditable>>,
    pub(crate) details: PatuiStepDetailsEditable,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiStepDetailsEditable {
    Read(PatuiStepReadEditable),
    Write(PatuiStepWriteEditable),
    Assertion(PatuiStepAssertionEditable),
    Sender(PatuiStepSenderEditable),
    Plugin(PatuiStepPluginEditable),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepReadEditable {
    pub(crate) r#in: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepWriteEditable {
    pub(crate) out: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepAssertionEditable {
    pub(crate) expr: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepSenderEditable {
    pub(crate) expr: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepPluginEditable {
    pub(crate) path: String, // TODO: Find a better solution when we're publishing plugins
    pub(crate) config: Option<HashMap<String, String>>,
    pub(crate) r#in: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepTransformStreamEditable {
    pub(crate) r#in: String,
    pub(crate) flavour: PatuiStepTransformStreamFlavour,
}

impl TryFrom<&PatuiStepEditable> for PatuiStep {
    type Error = PatuiStepError;

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
                PatuiStepDetailsEditable::Sender(patui_step_sender_editable) => {
                    PatuiStepDetails::Sender(PatuiStepSender {
                        expr: (&patui_step_sender_editable.expr[..]).try_into()?,
                    })
                }
                PatuiStepDetailsEditable::Plugin(patui_step_plugin_editable) => {
                    PatuiStepDetails::Plugin(PatuiStepPlugin {
                        path: patui_step_plugin_editable.path.clone(),
                        config: match &patui_step_plugin_editable.config {
                            Some(config) => config
                                .iter()
                                .map(|(k, v)| match TryInto::<PatuiExpr>::try_into(&v[..]) {
                                    Ok(v) => Ok((k.clone(), v)),
                                    Err(e) => Err(e),
                                })
                                .collect::<Result<_, _>>()?,
                            None => HashMap::new(),
                        },
                        r#in: match &patui_step_plugin_editable.r#in {
                            Some(r#in) => r#in
                                .iter()
                                .map(|(k, v)| match TryInto::<PatuiExpr>::try_into(&v[..]) {
                                    Ok(v) => Ok((k.clone(), v)),
                                    Err(e) => Err(e),
                                })
                                .collect::<Result<_, _>>()?,
                            None => HashMap::new(),
                        },
                    })
                }
            },
        })
    }
}

impl From<&PatuiStep> for PatuiStepEditable {
    fn from(value: &PatuiStep) -> Self {
        PatuiStepEditable {
            name: value.name.clone(),
            when: Some(value.when.clone()),
            depends_on: Some(value.depends_on.iter().map(|x| x.into()).collect()),
            details: match &value.details {
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
                PatuiStepDetails::Sender(patui_step_sender) => {
                    PatuiStepDetailsEditable::Sender(PatuiStepSenderEditable {
                        expr: (&patui_step_sender.expr).into(),
                    })
                }
                PatuiStepDetails::Plugin(patui_step_plugin) => {
                    PatuiStepDetailsEditable::Plugin(PatuiStepPluginEditable {
                        path: patui_step_plugin.path.clone(),
                        config: Some(
                            patui_step_plugin
                                .config
                                .iter()
                                .map(|(k, v)| (k.clone(), v.into()))
                                .collect(),
                        ),
                        r#in: Some(
                            patui_step_plugin
                                .r#in
                                .iter()
                                .map(|(k, v)| (k.clone(), v.into()))
                                .collect(),
                        ),
                    })
                }
            },
        }
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

        let details = PatuiTest::from_yaml_str(&yaml).unwrap();

        assert_that!(details.name).is_equal_to("test name".to_string());
        assert_that!(details.description).is_equal_to(Some("test description".to_string()));
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

        let details = PatuiTest::from_yaml_str(&yaml).unwrap();

        assert_that!(details.name).is_equal_to("test name".to_string());
        assert_that!(details.description).is_equal_to(Some("test description".to_string()));
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

        let test = PatuiTest::from_yaml_str(&yaml);

        assert_that!(test).is_err();
    }

    #[test]
    fn test_simple_process_yaml() {
        let details: Result<PatuiTest, PatuiTestError> = PatuiTest::simple_process().try_into();
        assert_that!(details).is_ok();
        let details = details.unwrap();

        assert_that!(details.name).is_equal_to("simple_process".to_string());
        assert_that!(details.steps).has_length(5);
    }

    #[test]
    fn test_streaming_process_yaml() {
        let details: Result<PatuiTest, PatuiTestError> = PatuiTest::streaming_process().try_into();
        assert_that!(details).is_ok();
        let details = details.unwrap();

        assert_that!(details.name).is_equal_to("streaming_process".to_string());
        assert_that!(details.steps).has_length(9);
    }
}
