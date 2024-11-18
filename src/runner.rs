mod steps;

use std::sync::Arc;

use crate::db::{Database, PatuiRun};

use eyre::Result;

pub(crate) struct TestRunner {
    pub(crate) run: PatuiRun,
    pub(crate) db: Arc<Database>,
}

impl TestRunner {
    pub(crate) async fn run_test(self) -> Result<PatuiRun> {
        todo!();
    }

    // fn plan_test(&mut self) -> Result<PatuiRunPlan> {
    //     todo!()
    // }
}

#[cfg(test)]
mod tests {
    use assertor::*;

    use crate::{
        db::PatuiInstance,
        types::{
            PatuiStep, PatuiStepDetails, PatuiStepProcess, PatuiStepTransformStream,
            PatuiStepTransformStreamFlavour,
        },
    };

    use super::*;

    #[test]
    fn plan_basic() {
        let now = crate::utils::get_current_time_string();

        let test_runner = TestRunner {
            run: PatuiRun {
                id: 1.into(),
                instance: PatuiInstance {
                    id: 1.into(),
                    test_id: 1.into(),
                    hash: 123,
                    name: "test".to_string(),
                    description: "test".to_string(),
                    creation_date: now.clone(),
                    last_updated: now,
                    steps: vec![
                        PatuiStep {
                            name: "FooProcess".to_string(),
                            depends_on: todo!(),
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
                            depends_on: vec![],
                            details: PatuiStepDetails::TransformStream(PatuiStepTransformStream {
                                flavour: PatuiStepTransformStreamFlavour::Json,
                                input: todo!(),
                            }),
                        },
                    ],
                },
                start_time: todo!(),
                end_time: todo!(),
                status: todo!(),
                step_run_details: todo!(),
            },
            db: todo!(),
        };
    }

    #[test]
    fn run_basic_plan() {}
}
