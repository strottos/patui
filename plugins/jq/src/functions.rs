use std::{collections::HashMap, sync::Arc};

use ptplugin::{
    eval_patui_expr,
    plugin_server::ResultType,
    tokio::{
        self,
        sync::{mpsc, Mutex},
    },
    FunctionService, PatuiData, PatuiDataInner, PatuiEvent, PatuiExpr, Result,
};

pub(crate) struct Transform;

impl FunctionService for Transform {
    fn run(
        &self,
        args: HashMap<String, String>,
        tx: mpsc::Sender<(String, PatuiEvent)>,
        results: Arc<Mutex<PatuiData>>,
        _waker_rx: mpsc::Receiver<()>,
    ) -> Result<tokio::task::JoinHandle<()>> {
        Ok(tokio::spawn(async move {
            let Some(r#in) = args.get("in") else {
                tx.send((
                    "Err".to_string(),
                    PatuiEvent::Error("Missing required argument 'in'".to_string()),
                ))
                .await
                .unwrap();
                return;
            };

            let r#in: PatuiExpr = match (r#in).try_into() {
                Ok(r) => r,
                Err(e) => {
                    tx.send((
                        "Err".to_string(),
                        PatuiEvent::Error(format!("Invalid argument 'in': {}", e)),
                    ))
                    .await
                    .unwrap();
                    return;
                }
            };

            tracing::debug!("Evaluating argument 'in' for static data: {:?}", r#in);

            let results = results.lock().await.clone();

            let data = match eval_patui_expr(&r#in, &results) {
                Ok(data) => data,
                Err(e) => {
                    tx.send((
                        "Err".to_string(),
                        PatuiEvent::Error(format!(
                            "Couldn't evaluate argument 'in' for static data: {}",
                            e
                        )),
                    ))
                    .await
                    .unwrap();
                    return;
                }
            };

            if !data.is_known() {
                tx.send((
                    "Err".to_string(),
                    PatuiEvent::Error("Data evaluated to be unknown data".to_string()),
                ))
                .await
                .unwrap();
                return;
            }

            let event =
                match &data {
                    PatuiData::Known(PatuiDataInner::String(s)) => {
                        tracing::debug!("Evaluating JSON: {}", s);
                        let json: serde_json::Value = match serde_json::from_str(s) {
                            Ok(json) => json,
                            Err(e) => {
                                tx.send((
                                    "Err".to_string(),
                                    PatuiEvent::Error(format!("Failed to parse JSON: {}", e)),
                                ))
                                .await
                                .unwrap();
                                return;
                            }
                        };
                        PatuiEvent::Results(
                            PatuiExpr::try_from("out").unwrap(),
                            true.into(),
                            ResultType::Append.into(),
                            match try_convert_serde_json_to_patui_data(json) {
                                Ok(json) => json,
                                Err(e) => {
                                    tx.send((
                                        "Err".to_string(),
                                        PatuiEvent::Error(format!(
                                            "Failed to convert JSON to static data: {}",
                                            e
                                        )),
                                    ))
                                    .await
                                    .unwrap();
                                    return;
                                }
                            },
                        )
                    }
                    _ => {
                        tx.send(("Err".to_string(), PatuiEvent::Error(
                        "Data evaluated to non-string static data must always be known data"
                            .to_string(),
                    )))
                    .await
                    .unwrap();
                        return;
                    }
                };

            tx.send(("out".to_string(), event)).await.unwrap();
        }))
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
                .collect::<Result<Vec<_>>>()?;
            Ok(PatuiData::Known(PatuiDataInner::List(value)))
        }
        serde_json::Value::Object(value) => {
            let value = value
                .into_iter()
                .map(|(k, v)| Ok((k, try_convert_serde_json_to_patui_data(v)?)))
                .collect::<Result<HashMap<_, _>>>()?;
            Ok(PatuiData::Known(PatuiDataInner::Map(value)))
        }
    }
}
