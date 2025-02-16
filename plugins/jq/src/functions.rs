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
    tonic::{self, Status},
    tracing, FunctionService, PatuiData, PatuiDataInner, PatuiEvent, PatuiExpr, PatuiResultType,
    PatuiResultTypeConfirm, WakerType,
};

pub(crate) struct Transform {
    results_fully_recieved: Arc<AtomicBool>,
}

impl Transform {
    pub fn new(results_fully_recieved: Arc<AtomicBool>) -> Self {
        Self {
            results_fully_recieved,
        }
    }
}

impl FunctionService for Transform {
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

            tracing::debug!("Evaluating argument 'in' for static data: {:?}", r#in);

            loop {
                tracing::debug!("Evaluating argument 'in' for static data: {:?}", r#in);

                tracing::trace!("Locking read results");
                let results_clone = results.read().await.clone();
                tracing::trace!("Unlocked and found read results: {:?}", results_clone);

                let data = match eval_patui_expr(&r#in, &results_clone) {
                    Ok(data) => data,
                    Err(e) => {
                        match e {
                            ptplugin::EvalError::DataNotFound => (),
                            _ => {
                                produce_results_tx
                                    .send(Err(Status::internal(format!(
                                        "Couldn't evaluate argument 'in' for static data: {}",
                                        e
                                    ))))
                                    .await
                                    .unwrap();
                            }
                        }
                        continue;
                    }
                };

                if !data.is_known() {
                    produce_results_tx
                        .send(Err(Status::internal(
                            "Data evaluated to be unknown data".to_string(),
                        )))
                        .await
                        .unwrap();
                    return;
                }

                let event = match &data {
                    PatuiData::Known(PatuiDataInner::String(s)) => {
                        tracing::debug!("Evaluating JSON: {}", s);
                        let json: serde_json::Value = match serde_json::from_str(s) {
                            Ok(json) => json,
                            Err(e) => {
                                produce_results_tx
                                    .send(Err(Status::internal(format!(
                                        "Failed to parse JSON: {}",
                                        e
                                    ))))
                                    .await
                                    .unwrap();
                                return;
                            }
                        };
                        PatuiEvent::Result(
                            PatuiExpr::try_from("out").unwrap(),
                            true.into(),
                            PatuiResultType::List(num_results_sent),
                            match try_convert_serde_json_to_patui_data(json) {
                                Ok(json) => json,
                                Err(e) => {
                                    produce_results_tx
                                        .send(Err(Status::internal(format!(
                                            "Failed to convert JSON to static data: {}",
                                            e
                                        ))))
                                        .await
                                        .unwrap();
                                    return;
                                }
                            },
                        )
                    }
                    _ => {
                        produce_results_tx.send(Err(Status::internal(
                            "Data evaluated to non-string static data must always be known data"
                                .to_string(),
                        )))
                        .await
                        .unwrap();
                        return;
                    }
                };

                produce_results_tx.send(Ok(event)).await.unwrap();
                num_results_sent += 1;

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
                .send(Ok(PatuiEvent::Done(PatuiResultTypeConfirm::List(
                    num_results_sent,
                ))))
                .await
                .unwrap();
        });

        (produce_results_rx, Some(task))
    }
}

fn try_convert_serde_json_to_patui_data(
    value: serde_json::Value,
) -> Result<PatuiData, eyre::Error> {
    match value {
        serde_json::Value::Null => Ok(PatuiData::Known(PatuiDataInner::Null)),
        serde_json::Value::Bool(value) => Ok(PatuiData::Known(PatuiDataInner::Bool(value))),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(PatuiData::Known(PatuiDataInner::Integer(value)))
            } else if let Some(value) = value.as_f64() {
                Ok(PatuiData::Known(PatuiDataInner::Decimal(value)))
            } else {
                Err(eyre::eyre!("Invalid number"))
            }
        }
        serde_json::Value::String(value) => Ok(PatuiData::Known(PatuiDataInner::String(value))),
        serde_json::Value::Array(value) => {
            let value = value
                .into_iter()
                .map(try_convert_serde_json_to_patui_data)
                .collect::<Result<Vec<_>, eyre::Error>>()?;
            Ok(PatuiData::Known(PatuiDataInner::List(value)))
        }
        serde_json::Value::Object(value) => {
            let value = value
                .into_iter()
                .map(|(k, v)| Ok((k, try_convert_serde_json_to_patui_data(v)?)))
                .collect::<Result<HashMap<_, _>, eyre::Error>>()?;
            Ok(PatuiData::Known(PatuiDataInner::Map(value)))
        }
    }
}
