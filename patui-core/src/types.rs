use serde::{Deserialize, Serialize};

use crate::utils::get_current_time_string;

use self::steps::{PatuiStep, PatuiStepDetails, PatuiStepEditable, PatuiStepRead};

pub(crate) mod expr;
pub(crate) mod steps;

#[derive(Debug, thiserror::Error)]
pub enum PatuiTestError {
    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Step parsing error: {0}")]
    Step(#[from] steps::PatuiStepError),
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PatuiTest {
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) creation_date: String,
    pub(crate) steps: Vec<PatuiStep>,
}

impl Default for PatuiTest {
    fn default() -> Self {
        let now = get_current_time_string();

        PatuiTest {
            name: "Default".to_string(),
            description: Some("Default template".to_string()),
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

impl PatuiTest {
    /// This function is used to convert a YAML string into a `PatuiTest` struct.
    pub fn from_yaml_str(yaml: &str) -> Result<Self, PatuiTestError> {
        let yaml_test = serde_yaml::from_str::<PatuiTestEditable>(yaml)?;

        let now = get_current_time_string();

        let test = PatuiTest {
            name: yaml_test.name,
            description: yaml_test
                .description
                .unwrap_or_else(|| Some("".to_string())),
            creation_date: now,
            steps: yaml_test
                .steps
                .map(|steps| steps.iter().map(|s| s.try_into()).collect())
                .unwrap_or_else(|| Ok(Vec::new()))?,
        };

        Ok(test)
    }

    /// Wrap the `simple_process` YAML template into a `PatuiTest` struct.
    pub fn simple_process() -> PatuiTest {
        // NB: If any of these unwraps don't work the templates need updating,
        // tests should catch this.
        Self::from_yaml_str(include_str!("../../templates/simple_process.yaml")).unwrap()
    }

    /// Wrap the `streaming_process` YAML template into a `PatuiTest` struct.
    pub fn streaming_process() -> PatuiTest {
        Self::from_yaml_str(include_str!("../../templates/streaming_process.yaml")).unwrap()
    }

    /// Wrap the `simple_socket` YAML template into a `PatuiTest` struct.
    pub fn simple_socket() -> PatuiTest {
        todo!()
    }

    /// Wrap the `streaming_socket` YAML template into a `PatuiTest` struct.
    pub fn streaming_socket() -> PatuiTest {
        todo!()
    }

    /// Wrap the `complex_process_and_socket` YAML template into a `PatuiTest` struct.
    pub fn complex_process_and_socket() -> PatuiTest {
        todo!()
    }
}
