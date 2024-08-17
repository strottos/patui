/// Data types used in the application, these are the types that are used to interact with the
/// database. Every type that is used in the database should be defined here. This is to ensure
/// that the types are consistent across the application and that the database schema is
/// consistent. Every time we have an option wrapping an integer, it means that the field is an
/// auto-incrementing primary key or a foreign key, the option should always be the `Some` after
/// it has been flushed to the DB.
///
use color_eyre::Result;
use serde::{Deserialize, Serialize};
use strum::{EnumDiscriminants, EnumIter, VariantArray, VariantNames};

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct PatuiTest {
    pub id: Option<i64>,
    pub name: String,
    pub description: String,
    pub creation_date: String,
    pub last_updated: String,
    pub last_used_date: Option<String>,
    pub times_used: u32,
    pub steps: Vec<PatuiStepDetails>,
}

impl PatuiTest {
    pub fn to_editable_yaml_string(&self) -> Result<String> {
        let yaml_test = PatuiTestEditable {
            name: self.name.clone(),
            description: self.description.clone(),
            steps: self.steps.clone(),
        };

        Ok(serde_yaml::to_string(&yaml_test)?)
    }

    pub fn to_display_test(&self) -> Result<PatuiTestMinDisplay> {
        Ok(PatuiTestMinDisplay {
            id: self
                .id
                .ok_or_else(|| color_eyre::eyre::eyre!("No ID found"))?,
            name: self.name.clone(),
            description: self.description.clone(),
            steps: self.steps.clone(),
        })
    }

    pub fn edit_with_yaml(&mut self, yaml: &str) -> Result<()> {
        let yaml_test = serde_yaml::from_str::<PatuiTestEditable>(yaml)?;
        self.name = yaml_test.name;
        self.description = yaml_test.description;
        self.steps = yaml_test.steps;

        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct PatuiTestMinDisplay {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub steps: Vec<PatuiStepDetails>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, Serialize)]
pub struct PatuiTestEditable {
    pub name: String,
    pub description: String,
    pub steps: Vec<PatuiStepDetails>,
}

#[derive(
    Debug, Clone, Eq, PartialEq, Deserialize, Serialize, EnumIter, EnumDiscriminants, VariantNames,
)]
#[strum(serialize_all = "snake_case")]
pub enum PatuiStepDetails {
    Shell(PatuiStepShell),
    Assertion(PatuiStepAssertion),
}

impl PatuiStepDetails {
    pub fn to_str(&self) -> &'static str {
        match self {
            PatuiStepDetails::Shell(_) => "shell",
            PatuiStepDetails::Assertion(_) => "assertion",
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct PatuiStepShell {
    pub shell: Option<String>,
    pub contents: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize)]
pub struct PatuiStepAssertion {
    pub assertion: PatuiStepAssertionType,
    pub negate: bool,
    pub lhs: String,
    pub rhs: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, Serialize, EnumIter, VariantArray)]
pub enum PatuiStepAssertionType {
    #[default]
    Equal,
    Contains,
}

#[derive(Debug, Serialize)]
pub struct InsertTestStatus {
    pub id: i64,
    pub status: String,
}
