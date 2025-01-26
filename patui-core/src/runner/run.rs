use std::{collections::HashMap, sync::Arc};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, Mutex, RwLock};
use tracing::Level;

use crate::{expr::eval_patui_expr_ident, PatuiData, PatuiDataInner, PatuiTest};

use super::{
    events::PatuiEvent,
    steps::{PatuiStepRunner, PatuiStepRunnerError},
    PatuiEventWithTimestamp,
};

#[derive(Debug, Error)]
pub enum PatuiRunError {
    #[error("Step error: {0}")]
    StepError(#[from] PatuiStepRunnerError),
    #[error("Internal error: {0}")]
    InternalError(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub(crate) enum PatuiRunStatus {
    Pending,
    Passed,
    Failed,
    Error(String),
}

/// Run a Patui test
///
/// This struct is responsible for running a Patui test. It will run each step in the test and keep
/// track of the results.
///
/// You must call `run_test` to start the test.
#[derive(Debug)]
pub struct PatuiRun {
    pub(crate) instance: PatuiTest,
    pub(crate) start_time: String,
    pub(crate) end_time: Option<String>,
    pub(crate) status: PatuiRunStatus,
    pub(crate) events: Arc<Mutex<Vec<PatuiEventWithTimestamp>>>,
    pub(crate) results: Arc<RwLock<PatuiData>>,
    pub(crate) step_runners: IndexMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,

    waker_tx: broadcast::Sender<(String, String, String, PatuiData)>,
}

impl PatuiRun {
    /// Create a new `PatuiRun` instance
    ///
    /// Handles all initial non-async setup for the test run.
    pub fn new(instance: PatuiTest) -> Self {
        let mut step_runners = IndexMap::new();

        let (waker_tx, _) = broadcast::channel(32);

        let results = Arc::new(RwLock::new(PatuiData::Pending(PatuiDataInner::Map(
            HashMap::from([(
                "steps".to_string(),
                PatuiData::Pending(PatuiDataInner::Map(HashMap::new())),
            )]),
        ))));

        for step in &instance.steps {
            let name = step.name.clone();
            let entry = step_runners.entry(name).or_insert_with(Vec::new);
            entry.push(Arc::new(Mutex::new(PatuiStepRunner::new(
                step,
                waker_tx.subscribe(),
            ))));
        }

        Self {
            instance,
            start_time: crate::utils::get_current_time_string(),
            end_time: None,
            status: PatuiRunStatus::Pending,
            events: Arc::new(Mutex::new(vec![])),
            results,
            step_runners,

            waker_tx,
        }
    }

    /// Run the test
    ///
    /// This will perform additional setup and run the test itself whilst recording the results in
    /// `self`.
    pub async fn run_test(&mut self) -> Result<(), PatuiRunError> {
        let span = tracing::span!(
            Level::INFO,
            "running",
            test_name = self.instance.name.as_str()
        );
        let _guard = span.enter();

        let (tx, mut rx) = mpsc::channel(100);

        self.init_test().await?;

        let status = Arc::new(Mutex::new(PatuiRunStatus::Pending));
        let status_clone = status.clone();

        for (step_name, step_collection) in self.step_runners.iter() {
            for step_runner in step_collection {
                let mut lock = step_runner.lock().await;
                lock.run(tx.clone())?;
            }
        }

        let events = self.events.clone();
        let results = self.results.clone();
        let waker_tx = self.waker_tx.clone();

        let receive_task = tokio::spawn(async move {
            let status = status_clone;
            loop {
                let (step_name, function_name, events_res) = match rx.recv().await {
                    Some((step_name, function_name, events_res)) => {
                        (step_name, function_name, events_res)
                    }
                    None => {
                        tracing::debug!("Received None from channel");
                        break;
                    }
                };
                tracing::trace!(
                    "Received event from {} - {}: {:?}",
                    step_name,
                    function_name,
                    events_res
                );
                let mut lock = events.lock().await;
                lock.push(events_res.clone());
                if let PatuiEventWithTimestamp {
                    value: PatuiEvent::Results(elements, success, result_type, data),
                    ..
                } = events_res
                {
                    // TODO: Could read lock for a bit, maybe check if we see better performance if
                    // so, have to write lock soon after though.
                    let mut lock = results.write().await;
                    let result_name = eval_patui_expr_ident(&elements, &lock.clone()).unwrap();

                    let keys = vec![
                        "steps".to_string(),
                        step_name.clone(),
                        function_name.clone(),
                        result_name.clone(),
                    ];
                    tracing::trace!("Updating results {:?}: {:#?} - {:#?}", keys, *lock, data);

                    lock.append_to_list(keys, data.clone()).unwrap();
                    tracing::trace!("Results: {:#?}", *lock);

                    waker_tx
                        .send((step_name, function_name, result_name, data))
                        .unwrap();
                }
            }
        });

        for (step_name, step_collection) in self.step_runners.iter() {
            for step in step_collection {
                step.lock().await.wait(tx.clone()).await?;
            }
        }

        drop(tx);

        receive_task.await.map_err(|e| {
            PatuiRunError::InternalError(format!("Receive task never finished: {}", e))
        })?;

        Ok(())
    }

    async fn init_test(&mut self) -> Result<(), PatuiRunError> {
        for (name, step_collection) in self.step_runners.iter() {
            for step_runner in step_collection {
                let mut lock = step_runner.lock().await;

                let other_steps = self
                    .step_runners
                    .iter()
                    .filter(|(k, _)| *k != name)
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();

                // Make sure we have the other steps to ensure we don't try to relock this already
                // locked mutex for this step. The `Self` step must be treated differently.
                lock.init(name, other_steps).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use assertor::*;
    use tokio::time::timeout;
    use tracing_test::traced_test;

    use crate::{runner::run::PatuiRunStatus, templates::PatuiStep, PatuiTest};

    use super::PatuiRun;

    #[traced_test]
    #[tokio::test]
    async fn run_basic() {
        let mut test_runner = PatuiRun::new(PatuiTest {
            name: "test".to_string(),
            description: Some("test".to_string()),
            plugins: HashMap::new(),
            steps: vec![
                PatuiStep {
                    name: "static_data_json".to_string(),
                    plugin: "std".to_string(),
                    when: None,
                    depends_on: vec![],
                    function: "static_data".to_string(),
                    args: HashMap::from([(
                        "data".to_string(),
                        "[\"{\\\"a\\\": 1}\", \"{\\\"a\\\": 2}\", \"{\\\"a\\\": 3}\"]"
                            .try_into()
                            .unwrap(),
                    )]),
                },
                PatuiStep {
                    name: "json_objects".to_string(),
                    plugin: "jq".to_string(),
                    when: None,
                    depends_on: vec![],
                    function: "transform".to_string(),
                    args: HashMap::from([(
                        "in".to_string(),
                        "steps.static_data_json.static_data.static_data"
                            .try_into()
                            .unwrap(),
                    )]),
                },
                // PatuiStep {
                //     name: "assertions".to_string(),
                //     plugin: "std".to_string(),
                //     when: None,
                //     depends_on: vec![],
                //     function: "assertion".to_string(),
                //     args: HashMap::from([(
                //         "expr".to_string(),
                //         "steps.json_objects.transform.out.len() == 3 && steps.json_objects.transform.out[0].a == 1 && steps.json_objects.transform.out[1].a == 2 && steps.json_objects.transform.out[2].a == 3".try_into().unwrap(),
                //     )]),
                // },
            ],
        });

        let ret = timeout(Duration::from_secs(5), test_runner.run_test()).await;
        assert_that!(ret).is_ok();
        assert_that!(ret.unwrap()).is_ok();

        assert_that!(&test_runner.status).is_equal_to(&PatuiRunStatus::Passed);
        let events = test_runner.events.lock().await.clone();
        assert_that!(events.len()).is_equal_to(5);
    }
}
