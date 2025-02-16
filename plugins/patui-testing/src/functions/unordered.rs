use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use ptplugin::{
    tokio::{
        self,
        sync::{broadcast, mpsc, RwLock},
    },
    tonic, FunctionService, PatuiData, PatuiDataInner, PatuiEvent, PatuiResultType,
    PatuiResultTypeConfirm, WakerType,
};

pub(crate) struct UnorderedList;

impl UnorderedList {
    pub fn new(_results_fully_recieved: Arc<AtomicBool>) -> Self {
        Self {}
    }
}

impl FunctionService for UnorderedList {
    fn run(
        &self,
        step_name: String,
        _args: HashMap<String, String>,
        _results: Arc<RwLock<PatuiData>>,
        _waker_rx: broadcast::Receiver<WakerType>,
    ) -> (
        mpsc::Receiver<Result<PatuiEvent, tonic::Status>>,
        Option<tokio::task::JoinHandle<()>>,
    ) {
        let (produce_results_tx, produce_results_rx) = mpsc::channel(16);

        let task = tokio::spawn(async move {
            produce_results_tx
                .send(Ok(PatuiEvent::Result(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    PatuiResultType::List(0),
                    PatuiData::Known(PatuiDataInner::Integer(0)),
                )))
                .await
                .unwrap();

            produce_results_tx
                .send(Ok(PatuiEvent::Result(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    PatuiResultType::List(2),
                    PatuiData::Known(PatuiDataInner::Integer(2)),
                )))
                .await
                .unwrap();

            produce_results_tx
                .send(Ok(PatuiEvent::Result(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    PatuiResultType::List(4),
                    PatuiData::Known(PatuiDataInner::Integer(4)),
                )))
                .await
                .unwrap();

            produce_results_tx
                .send(Ok(PatuiEvent::Done(PatuiResultTypeConfirm::List(5))))
                .await
                .unwrap();

            produce_results_tx
                .send(Ok(PatuiEvent::Result(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    PatuiResultType::List(3),
                    PatuiData::Known(PatuiDataInner::Integer(3)),
                )))
                .await
                .unwrap();

            produce_results_tx
                .send(Ok(PatuiEvent::Result(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    PatuiResultType::List(1),
                    PatuiData::Known(PatuiDataInner::Integer(1)),
                )))
                .await
                .unwrap();
        });

        (produce_results_rx, Some(task))
    }
}
