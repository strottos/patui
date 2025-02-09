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
use tonic::Status;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

use ptplugin::{
    plugin_server::{self, PluginServiceServer, ResultType},
    FunctionService, PatuiData, PatuiDataInner, PatuiEvent, WakerType,
};

pub(crate) struct Plugin {
    shutdown_signal: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,

    results: std::sync::Arc<tokio::sync::RwLock<PatuiData>>,
    produced_results: Arc<tokio::sync::RwLock<Vec<PatuiEvent>>>,
    produced_results_waker:
        tokio::sync::Mutex<Option<(broadcast::Sender<WakerType>, broadcast::Receiver<WakerType>)>>,
}

impl Plugin {
    pub fn new(shutdown_signal: tokio::sync::oneshot::Sender<()>) -> Self {
        Self {
            shutdown_signal: tokio::sync::Mutex::new(Some(shutdown_signal)),

            results: std::sync::Arc::new(tokio::sync::RwLock::new(ptplugin::PatuiData::Pending(
                PatuiDataInner::Map(HashMap::new()),
            ))),
            produced_results: Arc::new(tokio::sync::RwLock::new(Vec::new())),
            produced_results_waker: tokio::sync::Mutex::new(Some(broadcast::channel(16))),
        }
    }
}

fn initialise_logging() -> Result<()> {
    let filter = match std::env::var("PATUI_LOG") {
        Ok(log) => Some(log),
        Err(_) => return Ok(()),
    };
    let filter = filter.map_or(EnvFilter::default(), EnvFilter::new);

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
                -p --port <NUM> "Sets a custom port for the plugin to listen on"
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

    type RunStream = tokio_stream::wrappers::ReceiverStream<
        std::result::Result<plugin_server::run::Response, tonic::Status>,
    >;

    async fn run(
        &self,
        request: tonic::Request<plugin_server::run::Request>,
    ) -> StdResult<tonic::Response<Self::RunStream>, tonic::Status> {
        let request = request.into_inner();

        tracing::info!("Request run {}", request.function);

        let (send_patui_results_tx, send_patui_results_rx) = mpsc::channel(16);
        let results = self.results.clone();

        let produced_results_waker_rx = self
            .produced_results_waker
            .lock()
            .await
            .as_ref()
            .unwrap()
            .0
            .subscribe();

        let (produce_results_rx, run_task) = match request.function.as_str() {
            "echo" => {
                let args = request.args;
                let args = args.into_iter().collect::<HashMap<_, _>>();

                functions::Echo::run(request.step_name, args, results, produced_results_waker_rx)
            }
            s => {
                return Err(tonic::Status::invalid_argument(format!(
                    "Unknown function '{}'",
                    s
                )));
            }
        };

        // TODO: Assert this finishes after we get a Done event/in shutdown?
        tokio::spawn(async move {
            let send_patui_results_tx = send_patui_results_tx;
            // let mut produced_results = self.produced_results.clone();
            let mut produce_results_rx = produce_results_rx;

            while let Some(event_res) = produce_results_rx.recv().await {
                {
                    // TODO: Add results into self.produced_results/self.results?
                    let result = match event_res {
                        Ok(event) => Ok(plugin_server::run::Response {
                            data: Some(
                                (&event)
                                    .try_into()
                                    .expect("Should be able to encode any event"),
                            ),
                        }),
                        Err(e) => Err(tonic::Status::internal(format!("Error: {}", e))),
                    };

                    tracing::debug!("Sending event details: {:?}", result);

                    if let Err(e) = send_patui_results_tx.send(result).await {
                        tracing::error!("Error sending result: {:?}", e);
                        break;
                    }
                }
            }
        });

        Ok(tonic::Response::new(
            tokio_stream::wrappers::ReceiverStream::new(send_patui_results_rx),
        ))
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
            tracing::trace!("Setup receive results streams");
            loop {
                let request = match stream.next().await {
                    Some(Ok(r)) => r,
                    Some(Err(e)) => {
                        ptplugin::tracing::error!("Error receiving results: {:?}", e);
                        break;
                    }
                    None => break,
                };
                ptplugin::tracing::trace!("Received results: {:?}", request);

                let data: ptplugin::PatuiData = request.results.unwrap().try_into().unwrap();
                ptplugin::tracing::debug!("Received data: {:?}", data);
                {
                    let mut lock = results.write().await;
                    match request.r#type.try_into() {
                        Ok(ResultType::Append) => {
                            lock.append_to_list(vec!["steps".to_string(), request.step_name, request.function_name, request.result_name], data).unwrap();
                        }
                        _ => todo!(),
                    }
                    tracing::trace!("Results: {:?}", lock);

                    // Important we send this before unlocking the results as otherwise we might
                    // get a race condition trying to lock the results stream.
                    produced_results_waker_tx.send(WakerType::Results).unwrap();
                }

                let result = ptplugin::plugin_server::receive_results::Response {
                    diagnostics: vec![],
                };

                yield result.clone();
            }
            tracing::trace!("Finished receive results streams");
            produced_results_waker_tx.send(WakerType::Done).unwrap();
        };

        Ok(ptplugin::tonic::Response::new(
            Box::pin(output) as Self::ReceiveResultsStream
        ))
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
