use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use ptplugin::{
    tokio::{
        self,
        sync::{broadcast, mpsc, RwLock},
    },
    tonic::Status,
    FunctionService, PatuiData, PatuiDataInner, PatuiEvent, PatuiStepResult, WakerType,
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
        mpsc::Receiver<Result<PatuiEvent, Status>>,
        Option<tokio::task::JoinHandle<()>>,
    ) {
        let (produce_results_tx, produce_results_rx) = mpsc::channel(16);

        let task = tokio::spawn(async move {
            produce_results_tx
                .send(Ok(PatuiEvent::Result(PatuiStepResult::new_stream_item(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    0,
                    PatuiData::Known(PatuiDataInner::Integer(0)),
                ))))
                .await
                .unwrap();

            produce_results_tx
                .send(Ok(PatuiEvent::Result(PatuiStepResult::new_stream_item(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    2,
                    PatuiData::Known(PatuiDataInner::Integer(2)),
                ))))
                .await
                .unwrap();

            produce_results_tx
                .send(Ok(PatuiEvent::Result(PatuiStepResult::new_stream_item(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    4,
                    PatuiData::Known(PatuiDataInner::Integer(4)),
                ))))
                .await
                .unwrap();

            produce_results_tx
                .send(Ok(PatuiEvent::Result(PatuiStepResult::done_stream(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    5,
                ))))
                .await
                .unwrap();

            produce_results_tx
                .send(Ok(PatuiEvent::Result(PatuiStepResult::new_stream_item(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    3,
                    PatuiData::Known(PatuiDataInner::Integer(3)),
                ))))
                .await
                .unwrap();

            produce_results_tx
                .send(Ok(PatuiEvent::Result(PatuiStepResult::new_stream_item(
                    format!("steps.{}.unordered_list.out", step_name)
                        .try_into()
                        .unwrap(),
                    true.into(),
                    1,
                    PatuiData::Known(PatuiDataInner::Integer(1)),
                ))))
                .await
                .unwrap();
        });

        (produce_results_rx, Some(task))
    }
}
