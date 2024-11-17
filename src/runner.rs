mod steps;

use std::sync::Arc;

use crate::db::{Database, PatuiRun};

use eyre::Result;

pub(crate) struct TestRunner {
    pub(crate) run: PatuiRun,
    pub(crate) db: Arc<Database>,
}

impl TestRunner {
    pub(crate) async fn run_test(mut self) -> Result<PatuiRun> {
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use super::*;

    #[test]
    fn plan_basic() {
        let now = crate::utils::get_current_time_string();
    }

    #[test]
    fn run_basic_plan() {}
}
