use std::{collections::HashMap, sync::Arc};

use eyre::Result;
use tokio::{
    fs::read_to_string,
    sync::{mpsc, Mutex},
};

use ptplugin::{
    eval_patui_expr, plugin_server::run, FunctionService, PatuiData, PatuiDataInner, PatuiEvent,
    PatuiExpr,
};

pub(crate) struct FileRead;

impl FunctionService for FileRead {
    fn run(
        &self,
        args: HashMap<String, String>,
        tx: mpsc::Sender<std::result::Result<run::Response, tonic::Status>>,
        _results: Arc<Mutex<PatuiData>>,
        _waker_rx: mpsc::Receiver<()>,
    ) -> Result<tokio::task::JoinHandle<()>> {
        let task = tokio::spawn(async move {
            let Some(path) = args.get("path") else {
                let _ = tx
                    .send(Err(tonic::Status::invalid_argument(
                        "Missing required argument 'path'",
                    )))
                    .await;
                return;
            };

            let path: PatuiExpr = match path.try_into() {
                Ok(path) => path,
                Err(e1) => {
                    let path = format!("\"{}\"", path.replace("\"", "\\\""));
                    match path.try_into() {
                        Ok(path) => path,
                        Err(e2) => {
                            let _ = tx
                                .send(Err(tonic::Status::invalid_argument(format!(
                                    "Invalid argument for 'path': {e1}, {e2}",
                                ))))
                                .await;
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
                    let _ = tx
                        .send(Err(tonic::Status::invalid_argument(format!(
                            "Couldn't evaluate argument 'path' argument for reading file from: {}",
                            e
                        ))))
                        .await;
                    return;
                }
            };

            if !path.is_known() {
                let _ = tx
                    .send(Err(tonic::Status::invalid_argument(
                        "Invalid argument 'path', file read path must always be known data",
                    )))
                    .await;
                return;
            }

            let path = match path {
                PatuiData::Known(PatuiDataInner::String(s)) => s,
                _ => {
                    let _ = tx
                        .send(Err(tonic::Status::invalid_argument(
                            "Invalid argument 'path', file read path must be a string",
                        )))
                        .await;
                    return;
                }
            };

            let data = match read_to_string(&path).await {
                Ok(data) => data,
                Err(e) => {
                    let _ = tx
                        .send(Err(tonic::Status::invalid_argument(format!(
                            "Couldn't read file from path '{}': {}",
                            path, e
                        ))))
                        .await;
                    return;
                }
            };

            let event = PatuiEvent::Results(
                "file_data".to_string(),
                PatuiData::Known(PatuiDataInner::String(data)),
            );

            let event = match event.try_into() {
                Ok(event) => event,
                Err(e) => {
                    let _ = tx
                        .send(Err(tonic::Status::invalid_argument(format!(
                            "Couldn't convert file read event to response: {}",
                            e
                        ))))
                        .await;
                    return;
                }
            };

            let _ = tx
                .send(Ok(run::Response {
                    name: "file_read".to_string(),
                    diagnostics: vec![],
                    data: Some(event),
                }))
                .await;
        });

        Ok(task)
    }
}
