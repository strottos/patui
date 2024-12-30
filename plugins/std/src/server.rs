//! The server module contains the gRPC server implementation for the plugin service.

use std::{
    pin::Pin,
    sync::{Arc, Mutex},
};

use tokio::sync::oneshot;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::{Request, Response, Status, Streaming};

use crate::ptplugin::{
    get_info, init, plugin_service_server::PluginService, publish, run, shutdown, subscribe, wait,
    StepRunner,
};

#[derive(Debug)]
pub(crate) struct StdPlugin {
    tasks: Arc<Mutex<Vec<oneshot::Receiver<()>>>>,
    shutdown_signal: Mutex<Option<oneshot::Sender<()>>>,
}

impl StdPlugin {
    pub fn new(shutdown_signal: oneshot::Sender<()>) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            shutdown_signal: Mutex::new(Some(shutdown_signal)),
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

    async fn run(&self, request: Request<run::Request>) -> Result<Response<run::Response>, Status> {
        tracing::info!("Request run: {:?}", request.remote_addr());

        Ok(Response::new(run::Response {
            diagnostics: vec![],
        }))
    }

    type PublishStream =
        Pin<Box<dyn Stream<Item = Result<publish::Response, Status>> + Send + 'static>>;

    async fn publish(
        &self,
        request: Request<Streaming<publish::Request>>,
    ) -> Result<Response<Self::PublishStream>, Status> {
        todo!();
    }

    type SubscribeStream = ReceiverStream<Result<subscribe::Response, Status>>;

    async fn subscribe(
        &self,
        request: Request<subscribe::Request>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        todo!();
    }

    async fn wait(
        &self,
        request: Request<wait::Request>,
    ) -> Result<Response<wait::Response>, Status> {
        tracing::info!("Request wait: {:?}", request.remote_addr());

        let mut tasks = vec![];

        {
            let mut lock = self.tasks.lock().unwrap();
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

        let shutdown_tx = self.shutdown_signal.lock().unwrap().take().unwrap();
        shutdown_tx.send(()).unwrap();

        Ok(Response::new(shutdown::Response {}))
    }
}
