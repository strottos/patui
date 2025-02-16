use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use ptplugin::{
    eval_patui_expr,
    tokio::{self, sync::mpsc},
    EvalError, FunctionService, PatuiData, PatuiDataInner, PatuiEvent, PatuiExpr, PatuiResultType,
    PatuiResultTypeConfirm, Result, WakerType,
};
use tokio::sync::{broadcast, RwLock};
use tonic::Status;

pub(crate) struct Assertion {
    results_fully_recieved: Arc<AtomicBool>,
}

impl Assertion {
    pub fn new(results_fully_recieved: Arc<AtomicBool>) -> Self {
        Self {
            results_fully_recieved,
        }
    }
}

impl FunctionService for Assertion {
    fn run(
        &self,
        step_name: String,
        args: HashMap<String, String>,
        results: Arc<RwLock<PatuiData>>,
        mut results_waker_rx: broadcast::Receiver<WakerType>,
    ) -> (
        mpsc::Receiver<Result<PatuiEvent, Status>>,
        Option<tokio::task::JoinHandle<()>>,
    ) {
        let (produce_results_tx, produce_results_rx) = mpsc::channel(16);

        let results_fully_recieved = self.results_fully_recieved.clone();

        let task = tokio::spawn(async move {
            let Some(expr) = &args.get("expr") else {
                produce_results_tx
                    .send(Err(Status::invalid_argument(
                        "Missing expr argument".to_string(),
                    )))
                    .await
                    .unwrap();

                return;
            };

            let expr: PatuiExpr = match expr.as_str().try_into() {
                Ok(expr) => expr,
                Err(e) => {
                    produce_results_tx
                        .send(Err(Status::invalid_argument(format!(
                            "Invalid expr: {}",
                            e
                        ))))
                        .await
                        .unwrap();
                    return;
                }
            };

            let mut num_results_sent = 0;

            loop {
                tracing::debug!("Evaluating assertion: {:?}", expr);

                let results = results.read().await.clone();

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
                            PatuiData::Known(inner) => match inner {
                                PatuiDataInner::Bool(b) => PatuiEvent::Result(
                                    PatuiExpr::try_from("assertion").unwrap(),
                                    b.into(),
                                    PatuiResultType::List(num_results_sent),
                                    PatuiData::Known(PatuiDataInner::Bool(b)),
                                ),
                                _ => PatuiEvent::Error(format!(
                                    "Assertion error, evaluated to type {}: {}",
                                    inner, expr,
                                )),
                            },
                            PatuiData::Pending(inner) => match inner {
                                PatuiDataInner::Bool(b) => {
                                    // We know the result, after it's sent we're done
                                    if b {
                                        PatuiEvent::Log("assertion expected to pass".to_string())
                                    } else {
                                        PatuiEvent::Log("assertion expected to faile".to_string())
                                    }
                                }
                                _ => PatuiEvent::Error(format!(
                                    "Assertion error, evaluated to type {}: {}",
                                    inner, expr,
                                )),
                            },
                            PatuiData::Unknown => PatuiEvent::Error("Data not found".to_string()),
                        };

                        produce_results_tx.send(Ok(result)).await.unwrap();

                        if finished {
                            tracing::trace!("Sent assertion response");
                            break;
                        }
                    }
                    Err(e) => {
                        if let EvalError::DataNotFound = e {
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
                            panic!("Error evaluating expr with unhandled eval error: {:?}", e);
                        }
                    }
                };

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

                num_results_sent += 1;
            }

            tracing::info!("Finished assertion function");

            produce_results_tx
                .send(Ok(PatuiEvent::Done(PatuiResultTypeConfirm::List(
                    num_results_sent,
                ))))
                .await
                .unwrap();
        });

        (produce_results_rx, Some(task))
    }
}
