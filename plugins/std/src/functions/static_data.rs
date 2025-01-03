use std::{collections::HashMap, sync::Arc};

use eyre::Result;
use tokio::sync::{mpsc, Mutex};

use ptplugin::{
    eval_patui_expr, plugin_server::run, FunctionService, PatuiData, PatuiDataInner, PatuiEvent,
    PatuiExpr,
};

pub(crate) struct StaticData;

impl FunctionService for StaticData {
    fn run(
        &self,
        args: HashMap<String, String>,
        tx: mpsc::Sender<std::result::Result<run::Response, tonic::Status>>,
        _results: Arc<Mutex<PatuiData>>,
        _waker_rx: mpsc::Receiver<()>,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let task = tokio::spawn(async move {
            let Some(r#in) = args.get("in") else {
                let _ = tx
                    .send(Err(tonic::Status::invalid_argument(
                        "Missing required argument 'in'",
                    )))
                    .await;
                return;
            };

            let r#in: PatuiExpr = match (r#in).try_into() {
                Ok(r) => r,
                Err(e) => {
                    let _ = tx
                        .send(Err(tonic::Status::invalid_argument(format!(
                            "Invalid argument 'in': {}",
                            e
                        ))))
                        .await;
                    return;
                }
            };

            let data = match eval_patui_expr(
                &r#in,
                &PatuiData::Known(PatuiDataInner::Map(HashMap::new())),
            ) {
                Ok(data) => data,
                Err(e) => {
                    let _ = tx
                        .send(Err(tonic::Status::invalid_argument(format!(
                            "Couldn't evaluate argument 'in' for static data: {}",
                            e
                        ))))
                        .await;
                    return;
                }
            };

            if !data.is_known() {
                let _ = tx
                    .send(Err(tonic::Status::invalid_argument(
                        "Invalid argument 'in', static data must always be known data",
                    )))
                    .await;
                return;
            }

            let events = match data {
                PatuiData::Known(PatuiDataInner::List(list)) => {
                    let mut ret = vec![];
                    for (i, item) in list.into_iter().enumerate() {
                        if !item.is_known() {
                            let _ = tx
                                .send(Err(tonic::Status::invalid_argument(format!(
                                    "Invalid argument 'in', item {} in list is not known data",
                                    i
                                ))))
                                .await;
                            return;
                        }
                        ret.push(PatuiEvent::Results("data".to_string(), item));
                    }
                    ret
                }
                _ => vec![PatuiEvent::Results("data".to_string(), data)],
            };

            for event in events.into_iter() {
                let event = match event.try_into() {
                    Ok(event) => event,
                    Err(e) => {
                        let _ = tx
                            .send(Err(tonic::Status::internal(format!(
                                "Error encoding event: {}",
                                e
                            ))))
                            .await;
                        return;
                    }
                };

                let _ = tx
                    .send(Ok(run::Response {
                        name: "static_data".to_string(),
                        data: Some(event),
                        diagnostics: vec![],
                    }))
                    .await;
            }
        });

        Ok(task)
    }
}
