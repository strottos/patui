use std::{collections::HashMap, sync::Arc};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

use crate::PatuiTest;

use super::{events::PatuiEventInner, steps::PatuiStepRunner};

#[derive(Debug, Error)]
pub(crate) enum PatuiRunError {}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunStatus {
    Pending,
    Passed,
    Failed,
    Error(String),
}

#[derive(Debug)]
pub struct PatuiRun {
    pub(crate) instance: PatuiTest,
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) status: PatuiRunStatus,
    pub(crate) events: Vec<PatuiEventInner>,
    pub(crate) step_runners: IndexMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
}

impl PatuiRun {
    fn new(instance: PatuiTest) -> Self {
        let mut step_runners = IndexMap::new();

        let results = Arc::new(RwLock::new(HashMap::new()));

        for step in &instance.steps {
            let name = step.name.clone();
            let entry = step_runners.entry(name).or_insert_with(Vec::new);
            entry.push(Arc::new(Mutex::new(PatuiStepRunner::new(
                step,
                results.clone(),
            ))));
        }

        Self {
            instance,
            start_time: crate::utils::get_current_time_string(),
            end_time: None,
            status: PatuiRunStatus::Pending,
            events: vec![],
            step_runners,
        }
    }

    async fn run_test(self) -> Result<PatuiRun, PatuiRunError> {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use assertor::*;
    use tokio::time::timeout;
    use tracing_test::traced_test;

    use crate::templates::PatuiStep;

    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn run_basic() {
        let now = crate::utils::get_current_time_string();

        let test_runner = PatuiRun::new(PatuiTest {
            name: "test".to_string(),
            description: Some("test".to_string()),
            steps: vec![],
        });

        let test_run = timeout(Duration::from_secs(5), test_runner.run_test()).await;
        assert_that!(test_run).is_ok();
        let test_run = test_run.unwrap();
        assert_that!(test_run).is_ok();
        let test_run = test_run.unwrap();

        assert_that!(&test_run.status).is_equal_to(&PatuiRunStatus::Passed);
        assert_that!(&test_run.events.len()).is_equal_to(&5);
    }
}
