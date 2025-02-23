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

pub(crate) struct Json;

impl Json {
    pub fn new() -> Self {
        Self {}
    }
}

impl FunctionService for Json {
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

            tracing::debug!("Evaluating argument 'in' for static data: {:?}", r#in);

            loop {
                tracing::debug!("Evaluating argument 'in' for static data: {:?}", r#in);

                tracing::trace!("Locking read results");
                let results_clone = results.read().unwrap().clone();
                tracing::trace!("Unlocked and found read results: {:?}", results_clone);

                let data = match eval_patui_expr(&r#in, &results_clone) {
                    Ok(data) => data,
                    Err(e) => {
                        match e {
                            EvalError::DataNotFound(_) => (),
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

                match &data {
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
                        produce_results_tx
                            .send(Ok(PatuiEvent::Result(PatuiStepResult::new_stream_item(
                                format!("steps.{}.json.out", step_name).try_into().unwrap(),
                                true.into(),
                                0,
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
                            ))))
                            .await
                            .unwrap();
                    }
                    PatuiData::Known(PatuiDataInner::List(elems))
                    | PatuiData::Pending(PatuiDataInner::List(elems)) => {
                        tracing::debug!("Evaluating JSON: {:?}", elems);
                        for (i, elem) in elems.iter().enumerate() {
                            match &elem {
                                PatuiData::Known(PatuiDataInner::String(s)) => {
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
                                    produce_results_tx
                                        .send(Ok(PatuiEvent::Result(
                                            PatuiStepResult::new_stream_item(
                                                format!("steps.{}.json.out", step_name)
                                                    .try_into()
                                                    .unwrap(),
                                                true.into(),
                                                i,
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
                                            ),
                                        )))
                                        .await
                                        .unwrap();
                                }
                                _ => todo!(),
                            }
                        }
                    }
                    _ => panic!("Invalid JSON data, expected string or list: {:?}", data),
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
                    format!("steps.{}.json.out", step_name).try_into().unwrap(),
                    true.into(),
                    1,
                ))))
                .await
                .unwrap();
        });

        (produce_results_rx, Some(task))
    }
}

fn try_convert_serde_json_to_patui_data(value: serde_json::Value) -> Result<PatuiData, Status> {
    match value {
        serde_json::Value::Null => Ok(PatuiData::Known(PatuiDataInner::Null)),
        serde_json::Value::Bool(value) => Ok(PatuiData::Known(PatuiDataInner::Bool(value))),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                Ok(PatuiData::Known(PatuiDataInner::Integer(value)))
            // } else if let Some(value) = value.as_f64() {
            //     Ok(PatuiData::Known(PatuiDataInner::Decimal(value)))
            } else {
                Err(Status::internal("Invalid number"))
            }
        }
        serde_json::Value::String(value) => Ok(PatuiData::Known(PatuiDataInner::String(value))),
        serde_json::Value::Array(value) => {
            let value = value
                .into_iter()
                .map(try_convert_serde_json_to_patui_data)
                .collect::<Result<Vec<_>, Status>>()?;
            Ok(PatuiData::Known(PatuiDataInner::List(value)))
        }
        serde_json::Value::Object(value) => {
            let value = value
                .into_iter()
                .map(|(k, v)| Ok((k, try_convert_serde_json_to_patui_data(v)?)))
                .collect::<Result<HashMap<_, _>, Status>>()?;
            Ok(PatuiData::Known(PatuiDataInner::Map(value)))
        }
    }
}
