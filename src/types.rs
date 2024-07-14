use serde::Serialize;

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

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub enum PatuiStepDetails {
    Shell(PatuiStepShell),
    Assertion(PatuiStepAssertion),
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct PatuiStepShell {
    pub shell: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct PatuiStepAssertion {
    pub assertion: String,
    pub lhs: String,
    pub rhs: String,
}

#[derive(Debug, Serialize)]
pub struct InsertTestStatus {
    pub id: i64,
    pub status: String,
}
