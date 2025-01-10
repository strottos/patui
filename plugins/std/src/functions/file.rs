use std::{collections::HashMap, sync::Arc};

use ptplugin::{
    eval_patui_expr,
    tokio::{
        self,
        fs::read_to_string,
        sync::{mpsc, Mutex},
    },
    FunctionService, PatuiData, PatuiDataInner, PatuiEvent, PatuiExpr, Result,
};

pub(crate) struct FileRead;

impl FunctionService for FileRead {
    fn run(
        &self,
        args: HashMap<String, String>,
        tx: mpsc::Sender<(String, PatuiEvent)>,
        _results: Arc<Mutex<PatuiData>>,
        _waker_rx: mpsc::Receiver<()>,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let task = tokio::spawn(async move {
            let Some(path) = args.get("path") else {
                tx.send((
                    "Err".to_string(),
                    PatuiEvent::Error("Missing required argument 'path'".to_string()),
                ))
                .await
                .unwrap();
                return;
            };

            let path: PatuiExpr = match path.try_into() {
                Ok(path) => path,
                Err(e1) => {
                    let path = format!("\"{}\"", path.replace("\"", "\\\""));
                    match path.try_into() {
                        Ok(path) => path,
                        Err(e2) => {
                            tx.send((
                                "Err".to_string(),
                                PatuiEvent::Error(format!(
                                    "Invalid argument for 'path': {e1}, {e2}"
                                )),
                            ))
                            .await
                            .unwrap();
                            return;
                        }
                    }
                }
            };

            let path = match eval_patui_expr(
                &path,
                &PatuiData::Known(PatuiDataInner::Map(HashMap::new())),
            ) {
                Ok(path) => path,
                Err(e) => {
                    tx.send((
                        "Err".to_string(),
                        PatuiEvent::Error(format!(
                            "Couldn't evaluate argument 'path' argument for reading file from: {}",
                            e
                        )),
                    ))
                    .await
                    .unwrap();
                    return;
                }
            };

            if !path.is_known() {
                tx.send((
                    "Err".to_string(),
                    PatuiEvent::Error(
                        "Invalid argument 'path', file read path must always be known data"
                            .to_string(),
                    ),
                ))
                .await
                .unwrap();
                return;
            }

            let path = match path {
                PatuiData::Known(PatuiDataInner::String(s)) => s,
                _ => {
                    tx.send((
                        "Err".to_string(),
                        PatuiEvent::Error(
                            "Invalid argument 'path', file read path must be a string".to_string(),
                        ),
                    ))
                    .await
                    .unwrap();
                    return;
                }
            };

            let data = match read_to_string(&path).await {
                Ok(data) => data,
                Err(e) => {
                    tx.send((
                        "Err".to_string(),
                        PatuiEvent::Error(format!(
                            "Couldn't read file from path '{}': {}",
                            path, e
                        )),
                    ))
                    .await
                    .unwrap();
                    return;
                }
            };

            let event = PatuiEvent::Results(
                PatuiExpr::try_from("file_data").unwrap(),
                PatuiData::Known(PatuiDataInner::String(data)),
            );

            let event = match event.try_into() {
                Ok(event) => event,
                Err(e) => {
                    tx.send((
                        "Err".to_string(),
                        PatuiEvent::Error(format!(
                            "Couldn't convert file read event to response: {}",
                            e
                        )),
                    ))
                    .await
                    .unwrap();
                    return;
                }
            };

            tx.send(("file_read".to_string(), event)).await.unwrap();
        });

        Ok(task)
    }
}
