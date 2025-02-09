use std::{collections::HashMap, sync::Arc};

use ptplugin::{
    eval_patui_expr,
    tokio::{self, sync::mpsc},
    FunctionService, PatuiData, PatuiEvent, PatuiExpr, PatuiResultType, WakerType,
};
use tokio::sync::{broadcast, mpsc::Receiver, RwLock};
use tonic::Status;

pub(crate) struct Echo;

impl FunctionService for Echo {
    fn run(
        step_name: String,
        args: HashMap<String, String>,
        results: Arc<RwLock<PatuiData>>,
        mut results_waker_rx: broadcast::Receiver<WakerType>,
    ) -> (
        Receiver<Result<PatuiEvent, Status>>,
        Option<tokio::task::JoinHandle<()>>,
    ) {
        let (produce_results_tx, produce_results_rx) = mpsc::channel(16);

        let task = tokio::spawn(async move {
            let Some(r#in) = args.get("in") else {
                produce_results_tx
                    .send(Err(tonic::Status::invalid_argument(
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
                        .send(Err(tonic::Status::invalid_argument(format!(
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

                {
                    let results = results.read().await.clone();

                    tracing::trace!("Results: {:?}", results);

                    match eval_patui_expr(&r#in, &results) {
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
                                            .send(Ok(PatuiEvent::Results(
                                                format!("steps.{}.echo.out", step_name)
                                                    .try_into()
                                                    .unwrap(),
                                                true.into(),
                                                PatuiResultType::Append,
                                                item.clone(),
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

            produce_results_tx.send(Ok(PatuiEvent::Done)).await.unwrap();
        });

        (produce_results_rx, Some(task))
    }
}
