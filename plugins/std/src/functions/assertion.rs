use std::{collections::HashMap, sync::Arc};

use ptplugin::{
    eval_patui_expr,
    plugin_server::ResultType,
    tokio::{
        self,
        sync::{mpsc, Mutex},
    },
    EvalError, FunctionService, PatuiData, PatuiDataInner, PatuiEvent, PatuiExpr, Result,
};

pub(crate) struct Assertion;

impl FunctionService for Assertion {
    fn run(
        &self,
        args: HashMap<String, String>,
        tx: mpsc::Sender<(String, PatuiEvent)>,
        results: Arc<Mutex<PatuiData>>,
        mut waker_rx: mpsc::Receiver<()>,
    ) -> Result<tokio::task::JoinHandle<()>> {
        Ok(tokio::spawn(async move {
            let Some(expr) = &args.get("expr") else {
                tx.send((
                    "Err".to_string(),
                    PatuiEvent::Error("Missing expr argument".to_string()),
                ))
                .await
                .unwrap();

                return;
            };

            let expr: PatuiExpr = match expr.as_str().try_into() {
                Ok(expr) => expr,
                Err(e) => {
                    tx.send((
                        "Err".to_string(),
                        PatuiEvent::Error(format!("Invalid expr: {}", e)),
                    ))
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
                                PatuiDataInner::Bool(b) => PatuiEvent::Results(
                                    PatuiExpr::try_from("assertion").unwrap(),
                                    b.into(),
                                    ResultType::Append.into(),
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

                        tx.send(("eval".to_string(), result)).await.unwrap();

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
                                tx.send((
                                    "Err".to_string(),
                                    PatuiEvent::Error("Data not found".to_string()),
                                ))
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
