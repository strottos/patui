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

pub(crate) struct TestRunner<'a> {
    pub(crate) run: &'a PatuiRun,
    pub(crate) db: Arc<Database>,
}

impl<'a> TestRunner<'a> {
    pub(crate) async fn run_test(&self) -> Result<Vec<PatuiRunStep>> {
        let mut results = vec![];
        let mut ops_vec = vec![];

        for step in self.run.instance.steps.iter() {
            let (result, ops) = self.run_step(step, &ops_vec[0..ops_vec.len()]).await?;
            ops_vec.push(Arc::new(ops));
            results.push(result);
        }

        Ok(results)
    }

    async fn run_step(
        &'a self,
        step: &'a PatuiStep,
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
