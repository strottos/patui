use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use ptplugin::{
    eval_patui_expr,
    tokio::{
        self,
        sync::{broadcast, mpsc, RwLock},
    },
    tonic::Status,
    tracing, FunctionService, PatuiData, PatuiEvent, PatuiExpr, PatuiStepResult, WakerType,
};

pub(crate) struct Echo {
    results_fully_recieved: Arc<AtomicBool>,
}

impl Echo {
    pub fn new(results_fully_recieved: Arc<AtomicBool>) -> Self {
        Self {
            results_fully_recieved,
        }
    }
}

impl FunctionService for Echo {
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

        // TODO: How do we know this is true? Need to make Patui very clear if it's when the
        // receive_results socket drops. Conversely, it would be good to make it Patui's burden
        // once rather than every plugins responsibility.
        let results_fully_recieved = self.results_fully_recieved.clone();

        let task = tokio::spawn(async move {
            let Some(r#in) = args.get("in") else {
                produce_results_tx
                    .send(Err(Status::invalid_argument(
                        "Missing required argument 'in'".to_string(),
                    )))
                    .await
                    .unwrap();
                return;
            };

            let r#in: PatuiExpr = match (r#in).try_into() {
                Ok(r) => r,
                Err(e) => {
                    produce_results_tx
                        .send(Err(Status::invalid_argument(format!(
                            "Invalid argument 'in': {}",
                            e
                        ))))
                        .await
                        .unwrap();
                    return;
                }
            };

            let mut num_results_sent = 0;

            loop {
                tracing::debug!("Evaluating argument 'in' for static data: {:?}", r#in);

                tracing::trace!("Locking read results");
                let results_clone = results.read().await.clone();
                tracing::trace!("Unlocked and found read results: {:?}", results_clone);

                match eval_patui_expr(&r#in, &results_clone) {
                    Ok(eval) => match eval {
                        PatuiData::Known(patui_data_inner)
                        | PatuiData::Pending(patui_data_inner) => match patui_data_inner {
                            ptplugin::PatuiDataInner::Null => todo!(),
                            ptplugin::PatuiDataInner::Bool(_) => todo!(),
                            ptplugin::PatuiDataInner::Bytes(_) => todo!(),
                            ptplugin::PatuiDataInner::String(_) => todo!(),
                            ptplugin::PatuiDataInner::Integer(_) => todo!(),
                            ptplugin::PatuiDataInner::Decimal(_) => todo!(),
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
                    Err(e) => {
                        tracing::warn!("Error evaluating argument 'in': {:?}", e);
                    }
                };

                if results_fully_recieved.load(std::sync::atomic::Ordering::Relaxed) {
                    tracing::debug!("Results fully received, checking if anything left");

                    tracing::trace!("Locking read results for length check");
                    let results_clone = results.read().await.clone();
                    tracing::trace!(
                        "Unlocked and found read results for length check: {:?}",
                        results_clone
                    );
                    let expr = r#in.new_expr_append(".len()").unwrap();

                    match eval_patui_expr(&expr, &results_clone) {
                        Ok(eval) => match eval {
                            PatuiData::Known(patui_data_inner)
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
