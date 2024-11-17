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
    pub steps: Vec<PatuiStep>,
}

#[derive(Debug, Clone, Eq, PartialEq, Deserialize, EnumIter, EnumDiscriminants, VariantNames)]
#[strum(serialize_all = "snake_case")]
pub enum PatuiStep {
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

#[derive(Debug, Deserialize)]
pub struct PatuiRunStatus {
    pub id: i64,
    pub status: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PatuiInstance {
    pub(crate) id: i64,
    pub(crate) test_id: i64,
    pub(crate) hash: i64,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) creation_date: String,
    pub(crate) last_updated: String,
    pub(crate) steps: Vec<PatuiStep>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PatuiRun {
    pub(crate) id: i64,
    pub(crate) instance: PatuiInstance,
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) status: PatuiRunStatus,
    pub(crate) step_run_details: Vec<PatuiRunStep>,
}

#[derive(Debug, Deserialize)]
pub(crate) enum PatuiRunStepResult {}

#[derive(Debug, Deserialize)]
pub(crate) struct PatuiRunStep {
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) result: PatuiRunStepResult,
}
