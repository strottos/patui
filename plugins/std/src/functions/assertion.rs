use std::{collections::HashMap, sync::Arc};

use eyre::Result;
use tokio::sync::{mpsc, Mutex};
use tonic::Status;

use ptplugin::{
    eval_patui_expr, plugin_server::run, EvalError, FunctionService, PatuiData, PatuiDataInner,
    PatuiEvent, PatuiExpr,
};

pub(crate) struct Assertion;

impl FunctionService for Assertion {
    fn run(
        &self,
        args: HashMap<String, String>,
        tx: mpsc::Sender<std::result::Result<run::Response, Status>>,
        results: Arc<Mutex<PatuiData>>,
        mut waker_rx: mpsc::Receiver<()>,
    ) -> Result<tokio::task::JoinHandle<()>> {
        Ok(tokio::spawn(async move {
            let Some(expr) = &args.get("expr") else {
                tx.send(Err(Status::invalid_argument("Missing expr argument")))
                    .await
                    .unwrap();

                return;
            };

            let expr: PatuiExpr = match expr.as_str().try_into() {
                Ok(expr) => expr,
                Err(e) => {
                    tx.send(Err(Status::invalid_argument(format!(
                        "Invalid expr: {}",
                        e
                    ))))
                    .await
                    .unwrap();
                    return;
                }
            };

            loop {
                tracing::debug!("Evaluating assertion: {:?}", expr);

                let results = results.lock().await.clone();

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
                                PatuiDataInner::Bool(b) => {
                                    // We know the result, after it's sent we're done
                                    if b {
                                        PatuiEvent::Results(
                                            "assertion".to_string(),
                                            PatuiData::Known(PatuiDataInner::Bool(true)),
                                        )
                                    } else {
                                        PatuiEvent::Failure(
                                            "assertion".to_string(),
                                            format!("evaluated expr to false: {}", expr),
                                        )
                                    }
                                }
                                _ => PatuiEvent::Error(format!(
                                    "Assertion error, evaluated to type {}: {}",
                                    inner, expr,
                                )),
                            },
                            PatuiData::Pending(inner) => todo!(),
                            PatuiData::Unknown => todo!(),
                        };

                        let res = match result.try_into() {
                            Ok(r) => Ok(run::Response {
                                name: "eval".to_string(),
                                data: Some(r),
                                diagnostics: vec![],
                            }),
                            Err(e) => {
                                Err(Status::internal(format!("Error converting result: {}", e)))
                            }
                        };

                        tx.send(res).await.unwrap();

                        if finished {
                            break;
                        }
                    }
                    Err(e) => {
                        if let EvalError::DataNotFound = e {
                            if results.is_known() {
                                tracing::debug!(
                                    "Results are known, but data not found, failing and bailing"
                                );
                                tx.send(Err(Status::not_found("Data not found".to_string())))
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

                match waker_rx.recv().await {
                    Some(_) => {}
                    None => break,
                }
            }

            tracing::info!("Finished assertion function");
        }))
    }
}
