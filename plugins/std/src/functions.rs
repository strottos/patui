use std::{collections::HashMap, sync::Arc};

use miette::{miette, IntoDiagnostic, Result};
use patui_core::{eval_patui_expr, EvalError, PatuiData, PatuiDataInner, PatuiEvent, PatuiExpr};
use tokio::sync::{mpsc, Mutex};
use tonic::Status;

use patui_core::ptplugin::run;

pub(crate) trait FunctionService {
    fn run(
        &self,
        args: HashMap<String, String>,
        tx: mpsc::Sender<std::result::Result<run::Response, Status>>,
        results: Arc<Mutex<PatuiData>>,
        waker_rx: mpsc::Receiver<()>,
    ) -> Result<tokio::task::JoinHandle<()>>;
}

pub(crate) struct AssertionFunction {}

impl AssertionFunction {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl FunctionService for AssertionFunction {
    fn run(
        &self,
        args: HashMap<String, String>,
        tx: mpsc::Sender<std::result::Result<run::Response, Status>>,
        results: Arc<Mutex<PatuiData>>,
        mut waker_rx: mpsc::Receiver<()>,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let Some(expr) = &args.get("expr") else {
            return Err(miette!("Missing required argument 'expr'"));
        };

        let expr: PatuiExpr = expr.as_str().try_into().into_diagnostic()?;

        Ok(tokio::spawn(async move {
            loop {
                tracing::debug!("Evaluating assertion: {:?}", expr);

                let results = results.lock().await.clone();

                tracing::debug!("Results: {:?}", results);

                match eval_patui_expr(&expr, &results) {
                    Ok(result) => {
                        tracing::info!("Evaluated result: {:?}", result);
                        let result = match result {
                            PatuiData::Known(inner) => match inner {
                                PatuiDataInner::Bool(b) => {
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
                                data: Some(r),
                                diagnostics: vec![],
                            }),
                            Err(e) => {
                                Err(Status::internal(format!("Error converting result: {}", e)))
                            }
                        };

                        tx.send(res).await.unwrap();
                    }
                    Err(e) => {
                        tracing::error!("Error evaluating expr: {:?}", e);
                        if let EvalError::DataNotFound = e {
                            if results.is_known() {
                                tx.send(Err(Status::not_found("Data not found".to_string())))
                                    .await
                                    .unwrap();
                            }
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
