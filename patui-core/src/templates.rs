use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::expr::{PatuiExpr, PatuiExprError};

#[derive(Debug, Error)]
pub enum PatuiTestError {
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Plugin parsing error: {0}")]
    Plugin(#[from] PatuiPluginError),
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
    pub(crate) plugins: Option<HashMap<String, PatuiPluginEditable>>,
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
    pub(crate) plugins: HashMap<String, PatuiPlugin>,
    pub(crate) steps: Vec<PatuiStep>,
}

impl Default for PatuiTest {
    fn default() -> Self {
        PatuiTest {
            name: "Default".to_string(),
            description: Some("Default template".to_string()),
            plugins: HashMap::new(),
            steps: vec![PatuiStep {
                name: "DefaultProcess".to_string(),
                when: None,
                depends_on: vec![],
                plugin: "default_plugin".to_string(),
                args: HashMap::new(),
                function: "default".to_string(),
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
            plugins: yaml_test
                .plugins
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
            plugins: Some(
                self.plugins
                    .iter()
                    .map(|(k, v)| (k.clone(), v.into()))
                    .collect::<HashMap<String, PatuiPluginEditable>>(),
            ),
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

// Plugins

/// Plugin Errors
#[derive(Debug, Error)]
pub enum PatuiPluginError {
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// PatuiPlugin is the type used for configuring plugins. It is optional in a PatuiTest but can be
/// used if the overrides to the default configuration are required.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiPlugin {
    pub(crate) name: String,
    pub(crate) path: Option<String>, // TODO: Find a better solution when we're publishing plugins
    pub(crate) config: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiPluginEditable {
    pub(crate) name: String,
    pub(crate) path: Option<Option<String>>,
    pub(crate) config: Option<HashMap<String, String>>,
}

impl TryFrom<&PatuiPluginEditable> for PatuiPlugin {
    type Error = PatuiPluginError;

    fn try_from(value: &PatuiPluginEditable) -> Result<Self, Self::Error> {
        Ok(PatuiPlugin {
            name: value.name.clone(),
            path: value.path.clone().unwrap_or(None),
            config: value.config.clone().unwrap_or_default(),
        })
    }
}

impl From<&PatuiPlugin> for PatuiPluginEditable {
    fn from(value: &PatuiPlugin) -> Self {
        PatuiPluginEditable {
            name: value.name.clone(),
            path: Some(value.path.clone()),
            config: Some(value.config.clone()),
        }
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

/// PatuiStep is the type used for configuring steps.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStep {
    pub(crate) name: String,
    pub(crate) plugin: String,
    pub(crate) function: String,
    pub(crate) args: HashMap<String, PatuiExpr>,
    pub(crate) when: Option<String>,
    pub(crate) depends_on: Vec<String>,
}

/// PatuiStepEditable is to endable users ability to edit steps before they
/// are saved to the database, similar to PatuiTestEditable.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) struct PatuiStepEditable {
    pub(crate) name: String,
    pub(crate) plugin: String, // TODO: Find a better solution when we're publishing plugins
    pub(crate) when: Option<Option<String>>,
    pub(crate) depends_on: Option<Vec<String>>,
    pub(crate) run: String,
    pub(crate) args: Option<HashMap<String, String>>,
}

impl TryFrom<&PatuiStepEditable> for PatuiStep {
    type Error = PatuiStepError;

    fn try_from(value: &PatuiStepEditable) -> Result<Self, Self::Error> {
        Ok(PatuiStep {
            name: value.name.clone(),
            plugin: value.plugin.clone(),
            when: value.when.clone().unwrap_or(None),
            depends_on: value.depends_on.clone().unwrap_or_default(),
            function: value.run.clone(),
            args: value
                .args
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
            plugin: value.plugin.clone(),
            when: Some(value.when.clone()),
            depends_on: Some(value.depends_on.iter().map(|x| x.into()).collect()),
            run: value.function.clone(),
            args: Some(
                value
                    .args
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
                plugin: foo
                when: foobar == barfoo
                run: foo
              - name: bar
                plugin: bar
                depends_on:
                  - foo
                run: bar
                args:
                  foo: foo.bar
            "#,
        );

        let details = PatuiTest::from_yaml_str(&yaml).unwrap();

        assert_that!(details.name).is_equal_to("test name".to_string());
        assert_that!(details.description).is_equal_to(Some("test description".to_string()));
        assert_that!(details.steps).has_length(2);
        assert_that!(details.steps[0]).is_equal_to(PatuiStep {
            name: "foo".to_string(),
            plugin: "foo".to_string(),
            when: Some("foobar == barfoo".to_string()),
            depends_on: vec![],
            function: "foo".to_string(),
            args: HashMap::new(),
        });
        assert_that!(details.steps[1]).is_equal_to(PatuiStep {
            name: "bar".to_string(),
            plugin: "bar".to_string(),
            when: None,
            depends_on: vec!["foo".to_string()],
            function: "bar".to_string(),
            args: HashMap::from([("foo".to_string(), "foo.bar".try_into().unwrap())]),
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
