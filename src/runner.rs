mod process;

use std::sync::Arc;

use crate::{
    db::{Database, PatuiRun},
    types::{PatuiRunStep, PatuiRunStepResult, PatuiStep},
    utils::get_current_time_string,
};

use eyre::Result;

use self::process::PatuiRunStepProcessOps;

enum PatuiRunStepOps {
    Process(PatuiRunStepProcessOps),
}

pub(crate) struct TestRunner {
    pub(crate) run: PatuiRun,
    pub(crate) db: Arc<Database>,
}

impl TestRunner {
    pub(crate) async fn run_test(mut self) -> Result<PatuiRun> {
        let mut results = vec![];
        let mut ops_vec = vec![];

        for step in self.run.instance.steps.iter() {
            let (result, ops) = self.run_step(step, &ops_vec[0..ops_vec.len()]).await?;
            ops_vec.push(Arc::new(ops));
            results.push(result);
        }

        self.run.step_run_details = results;

        Ok(self.run)
    }

    async fn run_step(
        &self,
        step: &PatuiStep,
        ops: &[Arc<PatuiRunStepOps>],
    ) -> Result<(PatuiRunStep, PatuiRunStepOps)> {
        let start_time = get_current_time_string();

        let (result, ops) = match step {
            PatuiStep::Process(process) => {
                let (result, ops) = self.spawn_process(process).await?;
                (
                    PatuiRunStepResult::Process(result),
                    PatuiRunStepOps::Process(ops),
                )
            }
            PatuiStep::Shell(_) => todo!(),
            PatuiStep::Assertion(_) => todo!(),
        };

        Ok((
            PatuiRunStep {
                start_time,
                end_time: Some(get_current_time_string()),
                result,
            },
            ops,
        ))
    }
}
