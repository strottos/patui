use std::collections::HashMap;

use serde::{Deserialize, Serialize};
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
                path: "default_plugin".to_string(),
                config: HashMap::new(),
                r#in: HashMap::new(),
                run: "default".to_string(),
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

// Steps

/// Step Errors
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
    pub(crate) path: String, // TODO: Find a better solution when we're publishing plugins
    pub(crate) when: Option<String>,
    pub(crate) depends_on: Vec<String>,
    pub(crate) config: HashMap<String, String>,
    pub(crate) run: String,
    pub(crate) r#in: HashMap<String, PatuiExpr>,
}

/// PatuiStepEditable is to endable users ability to edit steps before they
/// are saved to the database, similar to PatuiTestEditable.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepEditable {
    pub(crate) name: String,
    pub(crate) path: String, // TODO: Find a better solution when we're publishing plugins
    pub(crate) when: Option<Option<String>>,
    pub(crate) depends_on: Option<Vec<String>>,
    pub(crate) config: Option<HashMap<String, String>>,
    pub(crate) run: String,
    pub(crate) r#in: Option<HashMap<String, String>>,
}

impl TryFrom<&PatuiStepEditable> for PatuiStep {
    type Error = PatuiStepError;

    fn try_from(value: &PatuiStepEditable) -> Result<Self, Self::Error> {
        Ok(PatuiStep {
            name: value.name.clone(),
            path: value.path.clone(),
            when: value.when.clone().unwrap_or(None),
            depends_on: value.depends_on.clone().unwrap_or_default(),
            config: value.config.clone().unwrap_or_default(),
            run: value.run.clone(),
            r#in: value
                .r#in
                .as_ref()
                .map(|x| {
                    x.iter()
                        .map(|(k, v)| match v.try_into() {
                            Ok(v) => Ok((k.clone(), v)),
                            Err(e) => Err(e),
                        })
                        .collect()
                })
                .unwrap_or_else(|| Ok(HashMap::new()))?,
        })
    }
}

impl From<&PatuiStep> for PatuiStepEditable {
    fn from(value: &PatuiStep) -> Self {
        PatuiStepEditable {
            name: value.name.clone(),
            path: value.path.clone(),
            when: Some(value.when.clone()),
            depends_on: Some(value.depends_on.iter().map(|x| x.into()).collect()),
            config: Some(value.config.clone()),
            run: value.run.clone(),
            r#in: Some(
                value
                    .r#in
                    .iter()
                    .map(|(k, v)| (k.clone(), v.into()))
                    .collect(),
            ),
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
                path: foo
                when: foobar == barfoo
                run: foo
              - name: bar
                path: bar
                depends_on:
                  - foo
                run: bar
                config:
                  foo: bar
                in:
                  foo: foo.bar
            "#,
        );

        let details = PatuiTest::from_yaml_str(&yaml).unwrap();

        assert_that!(details.name).is_equal_to("test name".to_string());
        assert_that!(details.description).is_equal_to(Some("test description".to_string()));
        assert_that!(details.steps).has_length(2);
        assert_that!(details.steps[0]).is_equal_to(PatuiStep {
            name: "foo".to_string(),
            path: "foo".to_string(),
            when: Some("foobar == barfoo".to_string()),
            depends_on: vec![],
            config: HashMap::new(),
            run: "foo".to_string(),
            r#in: HashMap::new(),
        });
        assert_that!(details.steps[1]).is_equal_to(PatuiStep {
            name: "bar".to_string(),
            path: "bar".to_string(),
            when: None,
            depends_on: vec!["foo".to_string()],
            config: HashMap::from([("foo".to_string(), "bar".to_string())]),
            run: "bar".to_string(),
            r#in: HashMap::from([("foo".to_string(), "foo.bar".try_into().unwrap())]),
        });
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
