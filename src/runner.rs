mod steps;

use std::sync::{Arc, Mutex};

use crate::{db::PatuiRun, types::PatuiRunStatus};

use eyre::Result;
use indexmap::IndexMap;
use tokio::sync::mpsc;

use self::steps::PatuiStepRunner;

pub(crate) struct TestRunner {
    pub(crate) run: PatuiRun,

    pub(crate) steps: IndexMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    // results: Vec<PatuiEvent>,
}

impl TestRunner {
    pub fn new(run: PatuiRun) -> Self {
        let mut steps = IndexMap::new();

        for step in &run.instance.steps {
            let name = step.name.clone();
            let entry = steps.entry(name).or_insert_with(Vec::new);
            entry.push(Arc::new(Mutex::new(PatuiStepRunner::new(&step))));
        }

        Self {
            run,
            steps,
            // results: vec![],
        }
    }

    pub(crate) async fn run_test(mut self) -> Result<PatuiRun> {
        let (tx, mut rx) = mpsc::channel(100);

        self.init_test().await?;

        for (_, step_collection) in self.steps.iter() {
            for step in step_collection {
                let mut step = step.lock().unwrap();
                step.run(tx.clone())?;
            }
        }

        drop(tx);

        let receive_task = tokio::spawn(async move {
            while let Some(res) = rx.recv().await {
                tracing::trace!("Received result: {:?}", res);
            }
        });

        for (_, step_collection) in self.steps.iter() {
            for step in step_collection {
                step.lock().unwrap().wait().await?;
            }
        }

        receive_task.await?;

        self.run.status = PatuiRunStatus::Passed;

        Ok(self.run)
    }

    async fn init_test(&mut self) -> Result<()> {
        for (name, step_collection) in self.steps.iter() {
            for step in step_collection {
                let mut step = step.lock().unwrap();

                let other_steps = self
                    .steps
                    .iter()
                    .filter(|(k, _)| *k != name)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                // Make sure we have the other steps to ensure we don't try to relock this already
                // locked mutex for this step. The `Self` step must be treated differently.
                step.init(name, other_steps).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use assertor::*;
    use tokio::time::timeout;
    use tracing_test::traced_test;

    use crate::{
        db::PatuiInstance,
        types::{
            PatuiStep, PatuiStepAssertion, PatuiStepDetails, PatuiStepRead,
            PatuiStepTransformStream, PatuiStepTransformStreamFlavour,
        },
    };

    use super::*;

    #[traced_test]
    #[tokio::test]
    async fn run_basic() {
        let now = crate::utils::get_current_time_string();

        let test_runner = TestRunner::new(PatuiRun {
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
                        name: "FooFile".to_string(),
                        when: None,
                        depends_on: vec![],
                        details: PatuiStepDetails::Read(PatuiStepRead {
                            r#in: "\"tests/data/test.json\"".try_into().unwrap(),
                        }),
                    },
                    PatuiStep {
                        name: "FooTransform".to_string(),
                        when: None,
                        depends_on: vec![],
                        details: PatuiStepDetails::TransformStream(PatuiStepTransformStream {
                            flavour: PatuiStepTransformStreamFlavour::Json,
                            r#in: "steps.FooFile.out".try_into().unwrap(),
                        }),
                    },
                    PatuiStep {
                        name: "FooAssertion".to_string(),
                        when: None,
                        depends_on: vec![],
                        details: PatuiStepDetails::Assertion(PatuiStepAssertion {
                            expr: "steps.FooTransform.out.len() == 1".try_into().unwrap(),
                        }),
                    },
                    // PatuiStep {
                    //     name: "FooAssertion".to_string(),
                    //     when: None,
                    //     depends_on: vec![],
                    //     details: PatuiStepDetails::Assertion(PatuiStepAssertion {
                    //         expr: "steps.FooTransform.out[0].baz[2] == 3".try_into().unwrap(),
                    //     }),
                    // },
                    // PatuiStep {
                    //     name: "FooAssertion".to_string(),
                    //     when: None,
                    //     depends_on: vec![],
                    //     details: PatuiStepDetails::Assertion(PatuiStepAssertion {
                    //         expr: "steps.FooTransform.out.bar[2] == \"c\"".try_into().unwrap(),
                    //     }),
                    // },
                ],
            },
            start_time: now,
            end_time: None,
            status: PatuiRunStatus::Pending,
            step_run_details: vec![],
        });

        let test_run = timeout(Duration::from_secs(5), test_runner.run_test()).await;
        assert_that!(test_run).is_ok();
        let test_run = test_run.unwrap();
        assert_that!(test_run).is_ok();
        let test_run = test_run.unwrap();

        assert_that!(&test_run.status).is_equal_to(&PatuiRunStatus::Passed);
    }
}
