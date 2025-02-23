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
    tracing, FunctionService, PatuiData, PatuiEvent, PatuiExpr, PatuiStepResult, WakerType,
};

pub(crate) struct Echo;

impl Echo {
    pub fn new() -> Self {
        Self {}
    }
}

impl FunctionService for Echo {
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
            let Some(r#in) = args.remove("in") else {
                produce_results_tx
                    .send(Err(Status::invalid_argument(
                        "Missing required argument 'in'".to_string(),
                    )))
                    .await
                    .unwrap();
                return;
            };

            let mut num_results_sent = 0;

            loop {
                tracing::debug!("Evaluating argument 'in' for static data: {:?}", r#in);

                tracing::trace!("Locking read results");
                let results_clone = results.read().unwrap().clone();
                tracing::trace!("Unlocked and found read results: {:?}", results_clone);

                match eval_patui_expr(&r#in, &results_clone) {
                    Ok(eval) => match eval {
                        PatuiData::Known(patui_data_inner)
                        | PatuiData::PendingFixed(patui_data_inner)
                        | PatuiData::Pending(patui_data_inner) => match patui_data_inner {
                            ptplugin::PatuiDataInner::Null => todo!(),
                            ptplugin::PatuiDataInner::Bool(_) => todo!(),
                            ptplugin::PatuiDataInner::Bytes(_) => todo!(),
                            ptplugin::PatuiDataInner::String(_) => todo!(),
                            ptplugin::PatuiDataInner::Integer(_) => todo!(),
                            //ptplugin::PatuiDataInner::Decimal(_) => todo!(),
                            ptplugin::PatuiDataInner::List(patui_list) => {
                                for item in patui_list.iter().skip(num_results_sent) {
                                    produce_results_tx
                                        .send(Ok(PatuiEvent::Result(
                                            PatuiStepResult::new_stream_item(
                                                format!("steps.{}.echo.out", step_name)
                                                    .try_into()
                                                    .unwrap(),
                                                true.into(),
                                                num_results_sent,
                                                item.clone(),
                                            ),
                                        )))
                                        .await
                                        .unwrap();
                                    num_results_sent += 1;
                                }
                            }
                            ptplugin::PatuiDataInner::Map(_) => todo!(),
                            ptplugin::PatuiDataInner::Set(_) => todo!(),
                        },
                        PatuiData::Unknown => {}
                    },
                    Err(e) => match e {
                        ptplugin::EvalError::DataNotFound(_) => {}
                        _ => {
                            tracing::error!("Error evaluating results: {:?}", e);
                            produce_results_tx
                                .send(Err(Status::internal(format!(
                                    "Error evaluating results: {}",
                                    e
                                ))))
                                .await
                                .unwrap();
                            return;
                        }
                    },
                };

                if !results_needed.lock().unwrap().is_empty() {
                    match results_waker_rx.recv().await {
                        Ok(waker_type) => match waker_type {
                            WakerType::Results => {
                                tracing::debug!("Received results waker to retry results");
                                continue;
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

                tracing::debug!("Results fully received, checking if anything left");
                tracing::trace!("Locking read results for length check");
                let results_clone = results.read().unwrap().clone();
                tracing::trace!(
                    "Unlocked and found read results for length check: {:?}",
                    results_clone
                );
                let expr = r#in.new_expr_append(".len()").unwrap();

                match eval_patui_expr(&expr, &results_clone) {
                    Ok(eval) => match eval {
                        PatuiData::Known(patui_data_inner)
                        | PatuiData::PendingFixed(patui_data_inner)
                        | PatuiData::Pending(patui_data_inner) => match patui_data_inner {
                            ptplugin::PatuiDataInner::Integer(len) => {
                                if len != num_results_sent as i64 {
                                    continue;
                                }
                            }
                            _ => todo!(),
                        },
                        PatuiData::Unknown => todo!(),
                    },
                    Err(e) => {
                        tracing::warn!("Error evaluating argument 'in': {:?}", e);
                    }
                }

                break;
            }

            produce_results_tx
                .send(Ok(PatuiEvent::Result(PatuiStepResult::done_stream(
                    format!("steps.{}.echo.out", step_name).try_into().unwrap(),
                    true.into(),
                    num_results_sent,
                ))))
                .await
                .unwrap();
        });

        (produce_results_rx, Some(task))
    }
}
