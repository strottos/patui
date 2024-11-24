mod steps;

use std::sync::{Arc, Mutex};

use crate::db::PatuiRun;

use eyre::Result;

use self::steps::PatuiStepRunner;

pub(crate) struct TestRunner {
    pub(crate) run: PatuiRun,

    pub(crate) steps: Vec<Arc<Mutex<PatuiStepRunner>>>,
}

impl TestRunner {
    pub fn new(run: PatuiRun) -> Self {
        let steps = run
            .instance
            .steps
            .iter()
            .map(|step| Arc::new(Mutex::new(PatuiStepRunner::new(&step))))
            .collect();

        Self { run, steps }
    }

    pub(crate) async fn run_test(mut self) -> Result<PatuiRun> {
        self.init_test()?;

        todo!();
    }

    fn init_test(&mut self) -> Result<()> {
        self.steps
            .iter_mut()
            .try_for_each(|step| step.lock().unwrap().init())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use crate::{
        db::PatuiInstance,
        types::{
            PatuiRunStatus, PatuiStep, PatuiStepDataTransfer, PatuiStepDetails, PatuiStepProcess,
            PatuiStepTransformStream, PatuiStepTransformStreamFlavour,
        },
    };

    use super::*;

    #[test]
    fn plan_basic() {
        let now = crate::utils::get_current_time_string();

        let mut test_runner = TestRunner::new(PatuiRun {
            id: 1.into(),
            instance: PatuiInstance {
                id: 1.into(),
                test_id: 1.into(),
                hash: 123,
                name: "test".to_string(),
                description: "test".to_string(),
                creation_date: now.clone(),
                last_updated: now.clone(),
                steps: vec![
                    PatuiStep {
                        name: "FooProcess".to_string(),
                        when: None,
                        depends_on: vec![],
                        details: PatuiStepDetails::Process(PatuiStepProcess {
                            command: "foo".into(),
                            args: vec![],
                            tty: None,
                            wait: false,
                            input: None,
                            cwd: None,
                        }),
                    },
                    PatuiStep {
                        name: "FooTransform".to_string(),
                        when: None,
                        depends_on: vec![],
                        details: PatuiStepDetails::TransformStream(PatuiStepTransformStream {
                            flavour: PatuiStepTransformStreamFlavour::Json,
                            input: PatuiStepDataTransfer::None,
                        }),
                    },
                ],
            },
            start_time: now,
            end_time: None,
            status: PatuiRunStatus::Pending,
            step_run_details: vec![],
        });

        assert_that!(test_runner.init_test()).is_ok();
    }

    #[test]
    fn run_basic_plan() {}
}
