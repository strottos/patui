use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct PatuiTest {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub creation_date: String,
    pub last_updated: String,
    pub last_used_date: Option<String>,
    pub times_used: u32,
    pub steps: Vec<PatuiStep>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PatuiStep {
    pub id: i64,
    pub test_id: i64,
    pub details: PatuiStepDetails,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
pub(crate) enum PatuiStepDetails {
    Shell(PatuiStepShell),
    Assertion(PatuiStepAssertion),
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
pub struct PatuiStepShell {
    pub shell: Option<String>,
    pub contents: String,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
pub struct PatuiStepAssertion {
    pub assertion: PatuiStepAssertionType,
    pub negate: bool,
    pub lhs: String,
    pub rhs: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize)]
pub enum PatuiStepAssertionType {
    Equal,
    Contains,
}

#[derive(Debug, Deserialize)]
pub(crate) struct InsertTestStatus {
    pub id: i64,
    pub status: String,
}
