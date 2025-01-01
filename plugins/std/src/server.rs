//! The server module contains the gRPC server implementation for the plugin service.

use std::{collections::HashMap, pin::Pin, sync::Arc};

use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{Request, Response, Status, Streaming};

use crate::functions::{AssertionFunction, FunctionService};
use patui_core::{
    ptplugin::{
        get_info, init, plugin_service_server::PluginService, publish, run, shutdown, wait,
        StepRunner,
    },
    PatuiData, PatuiDataInner,
};

#[derive(Debug)]
pub(crate) struct StdPlugin {
    tasks: Arc<Mutex<Vec<oneshot::Receiver<()>>>>,
    shutdown_signal: Mutex<Option<oneshot::Sender<()>>>,

    results: Arc<Mutex<PatuiData>>,

    waker_tx: std::sync::Mutex<Option<mpsc::Sender<()>>>,
    waker_rx: std::sync::Mutex<Option<mpsc::Receiver<()>>>,
}

impl StdPlugin {
    pub fn new(shutdown_signal: oneshot::Sender<()>) -> Self {
        let (waker_tx, waker_rx) = mpsc::channel(1);

        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            shutdown_signal: Mutex::new(Some(shutdown_signal)),

            results: Arc::new(Mutex::new(PatuiData::Pending(PatuiDataInner::Map(
                HashMap::new(),
            )))),

            waker_tx: std::sync::Mutex::new(Some(waker_tx)),
            waker_rx: std::sync::Mutex::new(Some(waker_rx)),
        }
    }
}

#[tonic::async_trait]
impl PluginService for StdPlugin {
    async fn get_info(
        &self,
        request: Request<get_info::Request>,
    ) -> Result<Response<get_info::Response>, Status> {
        tracing::info!("Request get_info: {:?}", request);

        let reply = get_info::Response {
            step_runner: Some(StepRunner {
                name: "patui_std".to_string(),
                description: "Patui Standard Plugin, standard Patui utilities like assertions and file manipulations".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                r#type: "std".to_string(),
                subscriptions: vec![],
            }),
        };
        Ok(Response::new(reply))
    }

    async fn init(
        &self,
        request: Request<init::Request>,
    ) -> Result<Response<init::Response>, Status> {
        tracing::info!("Request init: {:?}", request.remote_addr());

        Ok(Response::new(init::Response {
            diagnostics: vec![],
        }))
    }

    type RunStream = ReceiverStream<Result<run::Response, Status>>;

    async fn run(
        &self,
        request: Request<run::Request>,
    ) -> Result<Response<Self::RunStream>, Status> {
        let request = request.into_inner();

        tracing::info!("Request run {}", request.function);

        let function_service = match request.function.as_str() {
            "assertion" => AssertionFunction::new(),
            _ => {
                return Err(Status::unimplemented(
                    "Subscription for function and name not implemented",
                ))
            }
        };

        tracing::info!("Running function: {}", request.function);

        let (resp_tx, resp_rx) = mpsc::channel(32); // TODO: Configurable

        let waker_rx = self.waker_rx.lock().unwrap().take().unwrap();

        match function_service.run(request.args, resp_tx, self.results.clone(), waker_rx) {
            Ok(_) => {}
            Err(e) => {
                tracing::error!("Error running function: {:?}", e);
                return Err(Status::internal(format!("Error running function: {}", e)));
            }
        };

        Ok(Response::new(ReceiverStream::new(resp_rx)))
    }

    type PublishStream =
        Pin<Box<dyn Stream<Item = Result<publish::Response, Status>> + Send + 'static>>;

    async fn publish(
        &self,
        request: Request<Streaming<publish::Request>>,
    ) -> Result<Response<Self::PublishStream>, Status> {
        tracing::info!("Publish: {:?}", request.remote_addr());
        let mut stream = request.into_inner();
        let results = self.results.clone();
        let waker_tx = self.waker_tx.lock().unwrap().as_ref().unwrap().clone();

        let output = async_stream::try_stream! {
            while let Some(Ok(message)) = stream.next().await {
                tracing::info!("Message published: {:?}", message);

                let data: PatuiData = message.data.unwrap().try_into().unwrap();

                tracing::debug!("Data: {:?}", data);

                let mut lock = results.lock().await;
                *lock = data;
                waker_tx.send(()).await.unwrap();

                let result = publish::Response {
                    diagnostics: vec![],
                };

                yield result.clone();
            }

            tracing::info!("Publish stream ended");

            let mut lock = results.lock().await;
            *lock = lock.clone().to_known().unwrap();
            waker_tx.send(()).await.unwrap();
        };

        Ok(Response::new(Box::pin(output) as Self::PublishStream))
    }

    async fn wait(
        &self,
        request: Request<wait::Request>,
    ) -> Result<Response<wait::Response>, Status> {
        tracing::info!("Request wait: {:?}", request.remote_addr());

        let mut tasks = vec![];

        {
            let mut lock = self.tasks.lock().await;
            for task in lock.drain(..) {
                tasks.push(task);
            }
        }

        for task in tasks {
            tracing::info!("Waiting for task to complete");
            task.await.unwrap();
        }

        tracing::info!("Done waiting");

        Ok(Response::new(wait::Response {
            diagnostics: vec![],
        }))
    }

    async fn shutdown(
        &self,
        request: Request<shutdown::Request>,
    ) -> Result<Response<shutdown::Response>, Status> {
        tracing::info!("Requesting shutdown: {:?}", request.remote_addr());

        let shutdown_tx = self.shutdown_signal.lock().await.take().unwrap();
        shutdown_tx.send(()).unwrap();

        Ok(Response::new(shutdown::Response {}))
    }
}
