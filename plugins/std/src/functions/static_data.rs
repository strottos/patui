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

pub(crate) struct StaticData;

impl FunctionService for StaticData {
    fn run(
        &self,
        args: HashMap<String, String>,
        tx: mpsc::Sender<(String, PatuiEvent)>,
        _results: Arc<Mutex<PatuiData>>,
        _waker_rx: mpsc::Receiver<()>,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let task = tokio::spawn(async move {
            let Some(data) = args.get("data") else {
                tx.send((
                    "Err".to_string(),
                    PatuiEvent::Error("Missing required argument 'data'".to_string()),
                ))
                .await
                .unwrap();
                return;
            };

            let data: PatuiExpr = match data.try_into() {
                Ok(data) => data,
                Err(e) => {
                    tx.send((
                        "Err".to_string(),
                        PatuiEvent::Error(format!("Invalid argument 'data': {}", e)),
                    ))
                    .await
                    .unwrap();
                    return;
                }
            };

            let data = match eval_patui_expr(
                &data,
                &PatuiData::Known(PatuiDataInner::Map(HashMap::new())),
            ) {
                Ok(data) => data,
                Err(e) => {
                    tx.send((
                        "Err".to_string(),
                        PatuiEvent::Error(format!(
                            "Couldn't evaluate argument 'data' for static data: {}",
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
                    PatuiEvent::Error(
                        "Invalid argument 'data', static data must always be known data"
                            .to_string(),
                    ),
                ))
                .await
                .unwrap();
                return;
            }

            let events = match data {
                PatuiData::Known(PatuiDataInner::List(list)) => {
                    let mut ret = vec![];
                    for (i, item) in list.into_iter().enumerate() {
                        if !item.is_known() {
                            tx.send((
                                "Err".to_string(),
                                PatuiEvent::Error(format!(
                                    "Invalid argument 'data', item {} in list is not known data",
                                    i
                                )),
                            ))
                            .await
                            .unwrap();
                            return;
                        }
                        ret.push(PatuiEvent::Results(
                            PatuiExpr::try_from("data").unwrap(),
                            true.into(),
                            ResultType::Append.into(),
                            item,
                        ));
                    }
                    ret
                }
                _ => vec![PatuiEvent::Results(
                    PatuiExpr::try_from("data").unwrap(),
                    true.into(),
                    ResultType::Append.into(),
                    data,
                )],
            };

            for event in events.into_iter() {
                tx.send(("static_data".to_string(), event)).await.unwrap();
            }
        });

        Ok(task)
    }
}
