use serde::Serialize;
use strum::{EnumDiscriminants, EnumIter, VariantArray, VariantNames};

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct PatuiTest {
    pub id: Option<i64>,
    pub name: String,
    pub description: String,
    pub creation_date: String,
    pub last_updated: String,
    pub last_used_date: Option<String>,
    pub times_used: u32,
    pub steps: Vec<PatuiStep>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct PatuiStep {
    pub id: Option<i64>,
    pub test_id: i64,
    pub details: PatuiStepDetails,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, EnumIter, EnumDiscriminants, VariantNames)]
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

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize)]
pub struct PatuiStepShell {
    pub shell: Option<String>,
    pub contents: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize)]
pub struct PatuiStepAssertion {
    pub assertion: PatuiStepAssertionType,
    pub negate: bool,
    pub lhs: String,
    pub rhs: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, EnumIter, VariantArray)]
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
