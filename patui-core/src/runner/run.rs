use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::mpsc;
use tracing::Instrument;

use crate::{expr::PatuiExprQueryError, get_expr_terms, PatuiEvent, PatuiTest};

use super::{
    results::PatuiStepResult,
    steps::{PatuiStepRunner, PatuiStepRunnerError},
    PatuiEventWithTimestamp,
};

#[derive(Debug, Error)]
pub enum PatuiRunError {
    #[error("Step error: {0}")]
    StepError(#[from] PatuiStepRunnerError),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Getting terms: {0}")]
    GettingTerms(#[from] PatuiExprQueryError),
}

/// The status of a Patui test run.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum PatuiRunStatus {
    Pending,
    Passed,
    Failed,
    Error(String),
}

/// Run a Patui test
///
/// This struct is responsible for running a Patui test. It will run each step in the test and send
/// results from each step to other steps that depend on those results.
///
/// You must call `run_test` to start the test.
#[derive(Debug)]
pub struct PatuiRun {
    /// Indicates the time the test started
    pub start_time: String,

    /// Indicates the time the test finished. None here implies the test never finished.
    pub end_time: Option<String>,

    /// The status of the test, will be Pending until the test finishes at which point it will be
    /// moved into the appropriate status.
    pub status: PatuiRunStatus,

    /// A list of all the events that have happened during the test.
    pub events: Arc<Mutex<Vec<PatuiEventWithTimestamp>>>,

    pub(crate) instance: PatuiTest,
    pub(crate) results: HashMap<String, Vec<mpsc::Sender<PatuiStepResult>>>,
    pub(crate) step_runners: IndexMap<String, Vec<Arc<tokio::sync::Mutex<PatuiStepRunner>>>>,
}

impl PatuiRun {
    /// Create a new `PatuiRun` instance
    ///
    /// Handles all initial non-async setup for the test run.
    pub fn new(instance: PatuiTest) -> Self {
        let mut step_runners = IndexMap::new();

        for step in &instance.steps {
            let name = step.name.clone();
            let entry = step_runners.entry(name).or_insert_with(Vec::new);
            entry.push(Arc::new(tokio::sync::Mutex::new(PatuiStepRunner::new(
                step,
            ))));
        }

        Self {
            start_time: crate::utils::get_current_time_string(),
            end_time: None,
            status: PatuiRunStatus::Pending,
            events: Arc::new(Mutex::new(vec![])),

            instance,
            results: HashMap::new(),
            step_runners,
        }
    }

    /// Run the test
    ///
    /// This will perform additional setup and run the test itself whilst recording the results in
    /// `self`.
    pub async fn run_test(&mut self) -> Result<(), PatuiRunError> {
        let span = tracing::span!(
            tracing::Level::INFO,
            "running",
            test_name = self.instance.name.as_str()
        );
        let _guard = span.enter();

        self.init_test().await?;

        // let status = Arc::new(Mutex::new(PatuiRunStatus::Pending));
        // let status_clone = status.clone();

        let mut dependencies: HashMap<String, HashSet<String>> = HashMap::new();

        for (step_name, step_collection) in self.step_runners.iter() {
            for step_runner in step_collection {
                let lock = step_runner.lock().await;

                for arg_value in lock.step.args.values() {
                    let terms = get_expr_terms(arg_value)?;
                    for term in terms {
                        if term.len() < 4 {
                            continue;
                        }
                        if !term.first().unwrap().is_ident("steps".to_string()) {
                            continue;
                        }
                        let step_referenced = term.get(1).unwrap().as_ident_string().unwrap();
                        let entry = dependencies.entry(step_name.clone()).or_default();
                        entry.insert(step_referenced.clone());
                    }
                }
            }
        }

        tracing::trace!("Runner Dependencies: {:#?}", dependencies);

        self.start_time = crate::utils::get_current_time_string();

        let mut receivers = vec![];

        for (step_name, step_collection) in self.step_runners.iter() {
            for step_runner in step_collection {
                let (tx, rx) = mpsc::channel(100);
                tracing::trace!("Setting up step `{}`: {:?}", step_name, tx);

                let mut lock = step_runner.lock().await;

                for dependency_step_name in dependencies.remove(step_name).unwrap_or_default() {
                    tracing::trace!("Found dependency on `{}`", dependency_step_name);
                    let entry = self.results.entry(dependency_step_name).or_default();
                    entry.push(tx.clone());
                }

                // results.insert(lock.expr.clone(), tx);
                let receiver = lock.run(rx)?;
                tracing::trace!("Got receiver: {:?}", receiver);
                receivers.push((step_name, receiver));
            }
        }
        tracing::trace!("Results setup: {:?}", self.results);
        tracing::trace!("Receivers: {:?}", receivers);

        let events = self.events.clone();
        let results = self.results.clone();

        let mut receivers_tasks = vec![];
        for (step_name, rx) in receivers {
            let events = events.clone();
            let results = results.clone();
            let step_name_clone = step_name.clone();

            let span = tracing::info_span!("receiver_task", step_name = step_name.clone(),);

            receivers_tasks.push(tokio::spawn(
                async move {
                    let mut rx = rx;
                    let results = results.clone();
                    let step_name = step_name_clone;
                    while let Some(event) = rx.recv().await {
                        let results = results.clone();
                        tracing::trace!("Received event: {:?}", event);
                        let step_name = step_name.clone();
                        if let PatuiEventWithTimestamp {
                            value: PatuiEvent::Result(result),
                            ..
                        } = &event
                        {
                            tracing::debug!("Got result, sending to other steps: {:?}", result);
                            for tx in results.clone().get(&step_name).unwrap_or(&vec![]).iter() {
                                tx.send(result.clone()).await.unwrap();
                            }
                        }
                        let mut lock = events.lock().unwrap();
                        lock.push(event);
                    }
                    tracing::trace!("Done");
                    //     // let status = status_clone;
                    //     loop {
                    //         let (step_name, function_name, events_res) = match rx.recv().await {
                    //             Some((step_name, function_name, events_res)) => {
                    //                 (step_name, function_name, events_res)
                    //             }
                    //             None => {
                    //                 tracing::debug!("Received None from channel");
                    //                 break;
                    //             }
                    //         };
                    //         tracing::trace!(
                    //             "Received event from {} - {}: {:?}",
                    //             step_name,
                    //             function_name,
                    //             events_res
                    //         );
                    //         let mut lock = events.lock().await;
                    //         lock.push(events_res.clone());
                    //         if let PatuiEventWithTimestamp {
                    //             value: PatuiEvent::Result(result),
                    //             ..
                    //         } = events_res
                    //         {
                    //             // TODO: Could read lock for a bit, maybe check if we see better performance if
                    //             // so, have to write lock soon after though.
                    //             let mut lock = results.write().await;
                    //             // let result_name =
                    //             //     eval_patui_expr_ident(&result.location, &lock.clone()).unwrap();

                    //             // let keys = vec![
                    //             //     "steps".to_string(),
                    //             //     step_name.clone(),
                    //             //     function_name.clone(),
                    //             //     result_name.clone(),
                    //             // ];
                    //             // tracing::trace!(
                    //             //     "Updating results {:?}: {:#?} - {:#?}",
                    //             //     keys,
                    //             //     *lock,
                    //             //     result.details
                    //             // );

                    //             // match &result.details {
                    //             //     PatuiResultInner::StreamData(_, patui_data) => todo!(),
                    //             //     PatuiResultInner::MapElement(_, patui_data) => {
                    //             //         lock.append_to_list(keys, patui_data.clone()).unwrap()
                    //             //     }
                    //             //     PatuiResultInner::DoneList(num_items) => todo!(),
                    //             //     PatuiResultInner::DoneMap(keys) => todo!(),
                    //             // }
                    //             // tracing::trace!("Results: {:#?}", *lock);

                    //             // if let Err(e) = results_tx.send(result) {
                    //             //     tracing::error!("Failed to send results, giving up: {}", e);
                    //             //     break;
                    //             // }
                    //         }
                    //     }
                }
                .instrument(span),
            ));
        }

        for (step_name, step_collection) in self.step_runners.iter() {
            for step in step_collection {
                tracing::trace!("Waiting for step to finish - {}", step_name);
                step.lock().await.wait().await?;
                tracing::trace!("Step finished - {}", step_name);
            }
        }

        // drop(tx);

        for receiver_task in receivers_tasks.drain(..) {
            receiver_task.await.map_err(|e| {
                PatuiRunError::InternalError(format!("Receiver task never finished: {}", e))
            })?;
        }

        Ok(())
    }

    async fn init_test(&mut self) -> Result<(), PatuiRunError> {
        for (step_name, step_collection) in self.step_runners.iter() {
            for step_runner in step_collection {
                let mut lock = step_runner.lock().await;

                // Make sure we have the other steps to ensure we don't try to relock this already
                // locked mutex for this step. The `Self` step must be treated differently.
                lock.init(step_name).await?;
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

    #[cfg(feature = "integration_tests")]
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
                    plugin: "patui-testing-plugin".to_string(),
                    when: None,
                    depends_on: vec![],
                    function: "echo".to_string(),
                    args: HashMap::from([(
                        "in".to_string(),
                        r#"["{\"a\": 1}", "{\"a\": 2}", "{\"a\": 3}"]"#.try_into().unwrap(),
                    )]),
                },
                PatuiStep {
                    name: "json_objects".to_string(),
                    plugin: "patui-testing-plugin".to_string(),
                    when: None,
                    depends_on: vec![],
                    function: "json".to_string(),
                    args: HashMap::from([(
                        "in".to_string(),
                        "steps.static_data_json.echo.out"
                            .try_into()
                            .unwrap(),
                    )]),
                },
                PatuiStep {
                    name: "assertions".to_string(),
                    plugin: "patui-testing-plugin".to_string(),
                    when: None,
                    depends_on: vec![],
                    function: "assertion".to_string(),
                    args: HashMap::from([(
                        "expr".to_string(),
                        "steps.json_objects.json.out.len() == 3 && steps.json_objects.json.out[0].a == 1 && steps.json_objects.json.out[1].a == 2 && steps.json_objects.json.out[2].a == 3".try_into().unwrap(),
                    )]),
                },
            ],
        });

        let ret = timeout(Duration::from_secs(5), test_runner.run_test()).await;
        assert_that!(ret).is_ok();
        assert_that!(ret.unwrap()).is_ok();

        assert_that!(&test_runner.status).is_equal_to(&PatuiRunStatus::Passed);
        let events = test_runner.events.lock().unwrap().clone();
        assert_that!(events.len()).is_equal_to(5);
    }
}
