use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PatuiTest {
    pub name: String,
    pub description: String,
    pub creation_date: String,
    pub last_updated: String,
    pub last_used_date: Option<String>,
    pub times_used: u32,
    pub steps: Vec<PatuiStep>,
}

#[derive(Debug, Serialize)]
pub struct PatuiStep {}

#[derive(Debug, Serialize)]
pub struct InsertTestStatus {
    pub id: i64,
    pub status: String,
}
