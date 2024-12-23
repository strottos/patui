use std::{
    collections::HashMap,
    env,
    fs::create_dir_all,
    pin::Pin,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use clap::Parser;
use eyre::Result;
use serde::Serialize;
use tokio::sync::oneshot;
use tokio::{
    sync::{mpsc, RwLock},
    time::sleep,
};
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::{transport::Server, Code, Request, Response, Status};
use tracing_subscriber::{
    fmt::writer::BoxMakeWriter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};

use self::ptplugin::{
    get_info, init,
    plugin_service_server::{PluginService, PluginServiceServer},
    publish, run, subscribe, wait, PatuiStepData, StepRunner,
};

pub mod ptplugin {
    tonic::include_proto!("ptplugin");
}

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_NAME"),
    " ",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_DESCRIPTION"),
    ")"
);

// Copied from patui/src/types/steprs.rs, when we split out Patui into separate
// crates this problem will go away.
#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub(crate) enum PatuiStepDataFlavour {
    Null,
    Bool(bool),
    Bytes(Bytes),
    String(String),
    Integer(String),
    Float(String),
    Array(Vec<PatuiStepDataFlavour>),
    Map(HashMap<String, PatuiStepDataFlavour>),
    Set(Vec<PatuiStepDataFlavour>),
}

#[derive(Debug)]
pub(crate) struct MyPlugin {
    subscribers: Arc<
        RwLock<
            HashMap<String, Vec<mpsc::Sender<std::result::Result<subscribe::Response, Status>>>>,
        >,
    >,
    tasks: Arc<Mutex<Vec<tokio::task::JoinHandle<()>>>>,
    shutdown_signal: Arc<Mutex<Option<oneshot::Sender<()>>>>,
    echo_tx: Mutex<Option<mpsc::Sender<PatuiStepData>>>,
    echo_rx: Mutex<Option<mpsc::Receiver<PatuiStepData>>>,
}

impl MyPlugin {
    pub(crate) fn new(
        shutdown_signal: oneshot::Sender<()>,
        echo_tx: mpsc::Sender<PatuiStepData>,
        echo_rx: mpsc::Receiver<PatuiStepData>,
    ) -> Self {
        MyPlugin {
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            tasks: Arc::new(Mutex::new(Vec::new())),
            shutdown_signal: Arc::new(Mutex::new(Some(shutdown_signal))),
            echo_tx: Mutex::new(Some(echo_tx)),
            echo_rx: Mutex::new(Some(echo_rx)),
        }
    }
}

#[tonic::async_trait]
impl PluginService for MyPlugin {
    async fn get_info(
        &self,
        request: Request<get_info::Request>,
    ) -> std::result::Result<Response<get_info::Response>, Status> {
        tracing::info!("Request get_info: {:?}", request);

        let reply = get_info::Response {
            step_runner: Some(StepRunner {
                name: "test_patui_plugin".to_string(),
                description: "Test Patui Plugin, used for testing Patui only".to_string(),
                version: "0.1.0".to_string(),
                r#type: "test".to_string(),
                subscriptions: vec![],
            }),
        };
        Ok(Response::new(reply))
    }

    async fn init(
        &self,
        request: Request<init::Request>,
    ) -> std::result::Result<Response<init::Response>, Status> {
        tracing::info!("Request init: {:?}", request.remote_addr());

        Ok(Response::new(init::Response {
            diagnostics: vec![],
        }))
    }

    async fn run(
        &self,
        request: Request<run::Request>,
    ) -> std::result::Result<Response<run::Response>, Status> {
        tracing::info!("Request run {:?}", request.remote_addr());

        let subscribers = self.subscribers.clone();
        let mut echo_rx = self.echo_rx.lock().unwrap().take().unwrap();

        self.tasks.lock().unwrap().push(tokio::spawn(async move {
            {
                let lock = subscribers.read().await;
                for (name, subscribers) in lock.iter() {
                    if name == "out" {
                        for bytes in [
                            rmp_serde::to_vec(&PatuiStepDataFlavour::Null).unwrap(),
                            rmp_serde::to_vec(&PatuiStepDataFlavour::Bool(true)).unwrap(),
                            rmp_serde::to_vec(&PatuiStepDataFlavour::String("test".to_string()))
                                .unwrap(),
                            rmp_serde::to_vec(&PatuiStepDataFlavour::Array(vec![
                                PatuiStepDataFlavour::Integer("1".to_string()),
                                PatuiStepDataFlavour::Integer("2".to_string()),
                                PatuiStepDataFlavour::Integer("3".to_string()),
                            ]))
                            .unwrap(),
                            rmp_serde::to_vec(&PatuiStepDataFlavour::Map(HashMap::from([
                                (
                                    "a".to_string(),
                                    PatuiStepDataFlavour::Integer("1".to_string()),
                                ),
                                (
                                    "b".to_string(),
                                    PatuiStepDataFlavour::Integer("2".to_string()),
                                ),
                            ])))
                            .unwrap(),
                        ] {
                            sleep(tokio::time::Duration::from_millis(10)).await;

                            for tx in subscribers.iter() {
                                tracing::debug!("Sending {:?}", bytes);
                                tx.send(Ok(subscribe::Response {
                                    data: Some(PatuiStepData {
                                        bytes: bytes.clone(),
                                    }),
                                    diagnostics: vec![],
                                }))
                                .await
                                .unwrap();
                            }
                        }
                    } else if name == "echo" {
                        while let Some(res) = echo_rx.recv().await {
                            tracing::trace!("Echoing {:?}", res);
                            let response = subscribe::Response {
                                data: Some(res),
                                diagnostics: vec![],
                            };
                            for tx in subscribers.iter() {
                                tx.send(Ok(response.clone())).await.unwrap();
                            }
                        }
                    }
                }
            }

            let mut lock = subscribers.write().await;
            lock.clear();
            tracing::info!("Cleared subscribers");
        }));

        Ok(Response::new(run::Response {}))
    }

    type PublishStream = Pin<
        Box<
            dyn Stream<Item = std::result::Result<publish::Response, tonic::Status>>
                + Send
                + 'static,
        >,
    >;

    async fn publish(
        &self,
        request: tonic::Request<tonic::Streaming<publish::Request>>,
    ) -> std::result::Result<tonic::Response<Self::PublishStream>, tonic::Status> {
        let mut stream = request.into_inner();
        let echo_tx = self.echo_tx.lock().unwrap().take().unwrap().clone();

        let output = async_stream::try_stream! {
            while let Some(Ok(message)) = stream.next().await {
                tracing::info!("Message published: {:?}", message);

                echo_tx.send(message.data.unwrap()).await.unwrap();

                let result = publish::Response {
                    diagnostics: vec![],
                };

                yield result.clone();
            }
        };

        Ok(Response::new(Box::pin(output) as Self::PublishStream))
    }

    type SubscribeStream = ReceiverStream<std::result::Result<subscribe::Response, Status>>;

    async fn subscribe(
        &self,
        request: Request<subscribe::Request>,
    ) -> std::result::Result<Response<Self::SubscribeStream>, Status> {
        tracing::info!("Request subscribe {:?}", request.remote_addr());

        let data = request.into_inner();

        if data.name != "out" && data.name != "echo" {
            return Err(Status::new(
                Code::InvalidArgument,
                "Only 'out' or 'echo' subscriptions are supported",
            ));
        }

        tracing::info!("Adding a subscription for: {:?}", data.name);

        let (tx, rx) = mpsc::channel(4);

        let mut lock = self.subscribers.write().await;
        let entry = lock.entry(data.name).or_insert_with(Vec::new);
        entry.push(tx);

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn wait(
        &self,
        request: Request<wait::Request>,
    ) -> std::result::Result<Response<wait::Response>, Status> {
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

        let shutdown_tx = self.shutdown_signal.lock().unwrap().take().unwrap();

        tokio::spawn(async {
            let _ = shutdown_tx.send(());
        });

        Ok(Response::new(wait::Response {
            diagnostics: vec![],
        }))
    }
}

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub(crate) struct Cli {
    #[clap(short, long)]
    pub(crate) port: Option<String>,
}

fn version() -> String {
    let author = clap::crate_authors!();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}"
    )
}

fn initialise_logging() -> Result<()> {
    let now = chrono::offset::Local::now();
    let filter = match env::var("PATUI_LOG") {
        Ok(log) => Some(log),
        Err(_) => return Ok(()),
    };
    let path = env::var("PATUI_LOG_FILE")
        .unwrap_or_else(|_| "patui-log-${datetime}.log".to_string())
        .replace("${timestamp}", &now.timestamp().to_string())
        .replace("${datetime}", &now.format("%Y%m%d%H%M%S").to_string());

    let path = std::path::Path::new(&path);
    if let Some(parent) = path.parent() {
        create_dir_all(parent)?;
    }
    let log_file = std::fs::File::create(path)?;

    let var_name = EnvFilter::default();
    let filter = filter.map_or(var_name, EnvFilter::new);
    let writer = BoxMakeWriter::new(Arc::new(log_file));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_writer(writer)
        .with_target(true)
        .with_ansi(true);

    Registry::default().with(filter).with(fmt_layer).init();

    Ok(())
}

fn initialise_panic_handler() -> Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default()
        .panic_section(format!(
            "This is a bug. Consider reporting it at {}",
            env!("CARGO_PKG_REPOSITORY")
        ))
        .capture_span_trace_by_default(true)
        .display_location_section(true)
        .display_env_section(false)
        .into_hooks();
    eyre_hook.install()?;
    std::panic::set_hook(Box::new(move |panic_info| {
        #[cfg(not(debug_assertions))]
        {
            use human_panic::{handle_dump, print_msg, Metadata};
            let meta = Metadata::new(
                env!("CARGO_PKG_NAME").to_string(),
                env!("CARGO_PKG_VERSION").to_string(),
            )
            .authors(env!("CARGO_PKG_AUTHORS").replace(':', ", ").to_string())
            .homepage(env!("CARGO_PKG_HOMEPAGE").to_string());

            let file_path = handle_dump(&meta, panic_info);
            // prints human-panic message
            print_msg(file_path, &meta)
                .expect("human-panic: printing error message to console failed");
            eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
        }
        let msg = format!("{}", panic_hook.panic_report(panic_info));
        tracing::error!("Error: {}", strip_ansi_escapes::strip_str(msg));

        #[cfg(debug_assertions)]
        {
            // Better Panic stacktrace that is only enabled when debugging.
            better_panic::Settings::auto()
                .most_recent_first(false)
                .lineno_suffix(true)
                .verbosity(better_panic::Verbosity::Full)
                .create_panic_handler()(panic_info);
        }

        std::process::exit(libc::EXIT_FAILURE);
    }));

    Ok(())
}

async fn do_main() -> Result<()> {
    tracing::info!("Starting Patui Test Plugin");

    let args = Cli::parse();

    let Some(port) = args.port else {
        tracing::error!("No port provided");
        std::process::exit(libc::EXIT_FAILURE);
    };
    let addr = format!("[::1]:{}", port);
    let addr = addr.parse().unwrap();

    let (tx, rx) = oneshot::channel();
    let (echo_tx, echo_rx) = mpsc::channel(100);

    let plugin = MyPlugin::new(tx, echo_tx, echo_rx);

    Server::builder()
        .add_service(PluginServiceServer::new(plugin))
        .serve_with_shutdown(addr, async {
            rx.await.ok();
            tracing::info!("Shutting down");
        })
        .await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    initialise_logging()?;
    initialise_panic_handler()?;

    do_main().await
}
