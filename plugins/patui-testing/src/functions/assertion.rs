use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

use ptplugin::{
    eval_patui_expr,
    tokio::{
        self,
        sync::{broadcast, mpsc},
    },
    tonic::Status,
    tracing, EvalError, FunctionService, PatuiData, PatuiDataInner, PatuiEvent, PatuiExpr,
    PatuiStepResult, WakerType,
};

pub(crate) struct Assertion;

impl Assertion {
    pub fn new() -> Self {
        Self {}
    }
}

impl FunctionService for Assertion {
    fn run(
        &self,
        step_name: String,
        mut args: HashMap<String, PatuiExpr>,
        results: Arc<RwLock<PatuiData>>,
        results_needed: Arc<Mutex<Vec<PatuiExpr>>>,
        mut results_waker_rx: broadcast::Receiver<WakerType>,
    ) -> (
        mpsc::Receiver<Result<PatuiEvent, Status>>,
        Option<tokio::task::JoinHandle<()>>,
    ) {
        let (produce_results_tx, produce_results_rx) = mpsc::channel(16);

        let task = tokio::spawn(async move {
            let Some(expr) = args.remove("expr") else {
                produce_results_tx
                    .send(Err(Status::invalid_argument(
                        "Missing required argument 'expr'".to_string(),
                    )))
                    .await
                    .unwrap();
                return;
            };

            loop {
                tracing::debug!("Evaluating assertion: {:?}", expr);

                let results = results.read().unwrap().clone();

                tracing::debug!("Results: {:?}", results);

                match eval_patui_expr(&expr, &results) {
                    Ok(result) => {
                        let finished = result.is_known();
                        tracing::info!(
                            "Evaluated {} result: {:?}",
                            if finished { "finished" } else { "unfinished" },
                            result
                        );
                        let result = match result {
                            PatuiData::Known(patui_data_inner)
                            | PatuiData::PendingFixed(patui_data_inner)
                            | PatuiData::Pending(patui_data_inner) => match patui_data_inner {
                                PatuiDataInner::Bool(b) => PatuiStepResult::set_item(
                                    format!("steps.{}.assertion.result", step_name)
                                        .try_into()
                                        .unwrap(),
                                    b.into(),
                                    if finished {
                                        PatuiData::Known(PatuiDataInner::Bool(b))
                                    } else {
                                        PatuiData::Pending(PatuiDataInner::Bool(b))
                                    },
                                ),
                                _ => todo!(),
                            },
                            PatuiData::Unknown => todo!(),
                        };

                        produce_results_tx
                            .send(Ok(PatuiEvent::Result(result)))
                            .await
                            .unwrap();

                        if finished {
                            tracing::trace!("Sent assertion response");
                            break;
                        }
                    }
                    Err(e) => {
                        if let EvalError::DataNotFound(_) = e {
                            if results.is_known() {
                                tracing::debug!(
                                    "Results are known, but data not found, failing and bailing"
                                );
                                produce_results_tx
                                    .send(Ok(PatuiEvent::Error("Data not found".to_string())))
                                    .await
                                    .unwrap();
                                break;
                            } else {
                                tracing::debug!(
                                    "Results not fully known, will retry when more results come in"
                                );
                            }
                        } else {
                            produce_results_tx
                                .send(Err(Status::invalid_argument(format!(
                                    "Error evaluating 'expr': {}",
                                    e
                                ))))
                                .await
                                .unwrap();
                            return;
                        }
                    }
                }

                match results_waker_rx.recv().await {
                    Ok(waker_type) => match waker_type {
                        WakerType::Results => {
                            tracing::debug!("Received results waker to retry results");
                        }
                        WakerType::Done => {
                            tracing::debug!("Done receiving results, quitting");
                            break;
                        }
                    },
                    Err(e) => {
                        panic!("Error receiving waker: {:?}", e);
                    }
                }
            }

            tracing::info!("Finished assertion function");
        });

        (produce_results_rx, Some(task))
    }
}
