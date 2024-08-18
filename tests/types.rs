use serde::Deserialize;
use strum::{EnumDiscriminants, EnumIter, VariantArray, VariantNames};

#[derive(Debug, Deserialize)]
pub struct PatuiTest {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub creation_date: String,
    pub last_updated: String,
    pub last_used_date: Option<String>,
    pub times_used: u32,
    pub steps: Vec<PatuiStepDetails>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, EnumIter, EnumDiscriminants, VariantNames)]
#[strum(serialize_all = "snake_case")]
pub enum PatuiStepDetails {
    Shell(PatuiStepShell),
    Assertion(PatuiStepAssertion),
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize)]
pub struct PatuiStepShell {
    pub shell: Option<String>,
    pub contents: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize)]
pub struct PatuiStepAssertion {
    pub assertion: PatuiStepAssertionType,
    pub negate: bool,
    pub lhs: String,
    pub rhs: String,
}

#[derive(Debug, Clone, Default, Eq, PartialEq, Deserialize, EnumIter, VariantArray)]
pub enum PatuiStepAssertionType {
    #[default]
    Equal,
    Contains,
}

#[derive(Debug, Deserialize)]
pub struct PatuiTestEditStatus {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub struct PatuiTestMinDisplay {
    pub id: i64,
    pub name: String,
    pub description: String,
}
