mod steps;

use std::collections::HashMap;

use crate::db::{PatuiTest, PatuiTestId};
use steps::PatuiStepRunner;

use eyre::Result;

struct TestRunner<'a> {
    test: &'a PatuiTest,
}

impl<'a> TestRunner<'a> {
    fn plan(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use super::*;

    #[test]
    fn plan_basic() {
        let now = crate::utils::get_current_time_string();

        let test = PatuiTest {
            id: 1.into(),
            name: "Test 1".into(),
            description: "Test 1 description".into(),
            creation_date: now.clone(),
            last_updated: now,
            last_used_date: None,
            times_used: 0,
            steps: vec![],
        };
        let runner = TestRunner { test: &test };

        let ret = runner.plan();
    }

    #[test]
    fn run_basic_plan() {}
}
