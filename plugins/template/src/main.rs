//! # Patui `template` Plugin
//!
//! This is a template plugin for Patui. It is a simple plugin that can be used as a starting point
//! for creating your own plugins. It is basically an echo plugin with an option to sleep for a bit
//! so we can test it properly. It's largely used for testing the macro for creating plugins.

mod functions;

use std::{collections::HashMap, pin::Pin, result::Result as StdResult, sync::Arc};

use clap::{arg, value_parser, ArgMatches};
use eyre::{eyre, Result};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::StreamExt;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

use ptplugin::{
    plugin_server::{self, PluginServiceServer},
    PatuiData, PatuiDataInner, PatuiEvent,
};

pub(crate) struct Plugin {
    shutdown_signal: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,

    results: std::sync::Arc<tokio::sync::Mutex<PatuiData>>,
    produced_results: Arc<tokio::sync::RwLock<Vec<PatuiEvent>>>,
    produced_results_waker:
        tokio::sync::Mutex<Option<(broadcast::Sender<()>, broadcast::Receiver<()>)>>,
}

impl Plugin {
    pub fn new(shutdown_signal: tokio::sync::oneshot::Sender<()>) -> Self {
        Self {
            shutdown_signal: tokio::sync::Mutex::new(Some(shutdown_signal)),

            results: std::sync::Arc::new(tokio::sync::Mutex::new(ptplugin::PatuiData::Pending(
                PatuiDataInner::Map(HashMap::new()),
            ))),
            produced_results: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            produced_results_waker: tokio::sync::Mutex::new(Some(broadcast::channel(1))),
        }
    }
}

fn initialise_logging() -> Result<()> {
    let filter = match std::env::var("PATUI_LOG") {
        Ok(log) => Some(log),
        Err(_) => return Ok(()),
    };
    let filter = filter.map_or_else(|| EnvFilter::default(), EnvFilter::new);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_file(false)
        .with_line_number(false)
        .with_target(true)
        .with_ansi(false)
        .without_time();

    Registry::default().with(filter).with(fmt_layer).init();

    Ok(())
}

fn clap_matches() -> ArgMatches {
    clap::command!()
        .arg(
            arg!(
                -p --port <FILE> "Sets a custom config file"
            )
            .required(true)
            .value_parser(value_parser!(u16)),
        )
        .after_help("This is a Patui plugin and should not be called directly, please use Patui to orchestrate.\nFor further information please visit https://patui.dev/docs")
        .get_matches()
}

#[tonic::async_trait]
impl plugin_server::PluginService for Plugin {
    async fn get_info(
        &self,
        request: tonic::Request<plugin_server::get_info::Request>,
    ) -> StdResult<tonic::Response<plugin_server::get_info::Response>, tonic::Status> {
        tracing::info!("Request get_info: {:?}", request);

        let reply = plugin_server::get_info::Response {
            step_runner: Some(plugin_server::StepRunner {
                name: "template".to_string(),
                description: "echo template".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                r#type: "template".to_string(),
                subscriptions: vec![],
            }),
        };
        Ok(tonic::Response::new(reply))
    }

    async fn init(
        &self,
        request: tonic::Request<plugin_server::init::Request>,
    ) -> StdResult<tonic::Response<plugin_server::init::Response>, tonic::Status> {
        tracing::info!("Request init: {:?}", request.remote_addr());

        Ok(tonic::Response::new(plugin_server::init::Response {
            diagnostics: vec![],
        }))
    }

    async fn run(
        &self,
        request: tonic::Request<plugin_server::run::Request>,
    ) -> StdResult<tonic::Response<plugin_server::run::Response>, tonic::Status> {
        let request = request.into_inner();

        tracing::info!("Request run {}", request.function);

        todo!();
    }

    type ProduceResultsStream = tokio_stream::wrappers::ReceiverStream<
        std::result::Result<plugin_server::produce_results::Request, tonic::Status>,
    >;

    async fn produce_results(
        &self,
        request: tonic::Request<plugin_server::produce_results::Init>,
    ) -> StdResult<tonic::Response<Self::ProduceResultsStream>, tonic::Status> {
        let request = request.into_inner();

        tracing::debug!("Request produce results stream");

        let client_id = request.client_id;

        let (produce_results_tx, produce_results_rx) = ptplugin::tokio::sync::mpsc::channel(16);

        let produced_results = self.produced_results.clone();

        let produced_results_waker_rx = self
            .produced_results_waker
            .lock()
            .await
            .as_ref()
            .unwrap()
            .0
            .subscribe();

        tokio::spawn(async move {
            let produce_results_tx = produce_results_tx;
            let mut produced_results_waker_rx = produced_results_waker_rx;
            let mut sent = 0;
            let counter = std::sync::atomic::AtomicU64::new(1);

            loop {
                {
                    let lock = produced_results.read().await;

                    for event in lock.iter().skip(sent) {
                        let result_id = counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        let result = event.try_into();
                        let request = match result {
                            Ok(r) => Ok(ptplugin::plugin_server::produce_results::Request {
                                result_id,
                                data: Some(r),
                                diagnostics: vec![],
                            }),
                            Err(e) => Err(ptplugin::tonic::Status::internal(format!(
                                "Error converting result: {}",
                                e
                            ))),
                        };
                        ptplugin::tracing::debug!("Sending event details: {:?}", request);

                        if let Err(e) = produce_results_tx.send(request).await {
                            tracing::error!("Error sending result: {:?}", e);
                            break;
                        }

                        sent += 1;
                    }
                }

                produced_results_waker_rx.recv().await.unwrap();
            }
        });

        Ok(tonic::Response::new(
            tokio_stream::wrappers::ReceiverStream::new(produce_results_rx),
        ))
    }

    async fn ack_result(
        &self,
        request: tonic::Request<plugin_server::ack_result::Request>,
    ) -> StdResult<tonic::Response<plugin_server::ack_result::Response>, tonic::Status> {
        let request = request.into_inner();

        tracing::trace!("Ack result: {}", request.result_id);
        todo!();
    }

    type ReceiveResultsStream = Pin<
        Box<
            dyn tokio_stream::Stream<
                    Item = StdResult<plugin_server::receive_results::Response, tonic::Status>,
                > + Send
                + 'static,
        >,
    >;

    async fn receive_results(
        &self,
        request: tonic::Request<tonic::Streaming<plugin_server::receive_results::Request>>,
    ) -> StdResult<tonic::Response<Self::ReceiveResultsStream>, tonic::Status> {
        let produced_results_waker_tx = self
            .produced_results_waker
            .lock()
            .await
            .as_ref()
            .unwrap()
            .0
            .clone();
        let mut stream = request.into_inner();
        let results = self.results.clone();

        let output = ptplugin::async_stream::try_stream! {
            loop {
                tracing::trace!("Setup receive results streams");
                let request = match stream.next().await {
                    Some(Ok(r)) => r,
                    Some(Err(e)) => {
                        ptplugin::tracing::error!("Error receiving results: {:?}", e);
                        break;
                    }
                    None => break,
                };
                ptplugin::tracing::debug!("Received results: {:?}", request);

                let data: ptplugin::PatuiData = request.results.unwrap().try_into().unwrap();
                {
                    let mut lock = results.lock().await;
                    *lock = data;
                }

                produced_results_waker_tx.send(()).unwrap();

                let result = ptplugin::plugin_server::receive_results::Response {
                    diagnostics: vec![],
                };

                yield result.clone();
            }
        };

        Ok(ptplugin::tonic::Response::new(
            Box::pin(output) as Self::ReceiveResultsStream
        ))
    }

    async fn wait(
        &self,
        request: tonic::Request<plugin_server::wait::Request>,
    ) -> std::result::Result<tonic::Response<plugin_server::wait::Response>, tonic::Status> {
        ptplugin::tracing::info!("Request wait: {:?}", request.remote_addr());

        todo!();
    }

    async fn shutdown(
        &self,
        request: tonic::Request<plugin_server::shutdown::Request>,
    ) -> StdResult<tonic::Response<plugin_server::shutdown::Response>, tonic::Status> {
        tracing::info!("Requesting shutdown: {:?}", request.remote_addr());

        let shutdown_tx = self.shutdown_signal.lock().await.take().unwrap();
        if let Err(e) = shutdown_tx.send(()) {
            panic!("Error sending shutdown signal: {:?}", e);
        }

        Ok(tonic::Response::new(plugin_server::shutdown::Response {}))
    }
}

fn main() -> Result<()> {
    initialise_logging()?;
    let matches = clap_matches();
    let port = matches.get_one::<u16>("port").unwrap();
    tracing::info!("Starting Patui Test Plugin on port {}", port);
    let addr = format!("[::1]:{}", port);
    let addr = addr.parse().unwrap();

    let body = async {
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let plugin_server = Plugin::new(shutdown_tx);
        tonic::transport::Server::builder()
            .add_service(PluginServiceServer::new(plugin_server))
            .serve_with_shutdown(addr, async {
                shutdown_rx.await.ok();
                tracing::info!("Shutting down");
            })
            .await
    };

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime")
        .block_on(body)
        .map_err(|e| eyre!("Error running server: {}", e))
}
