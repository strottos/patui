use std::{
    collections::HashMap,
    env::{self, current_exe},
    sync::Arc,
};

use thiserror::Error;
use tokio::{
    io::AsyncBufReadExt,
    process::{Child, Command},
    sync::{broadcast, mpsc, oneshot, Mutex},
    task::JoinHandle,
};
use tonic::{transport::Channel, Request};
use tracing::Instrument;

use crate::{
    expr::{PatuiData, PatuiExprError},
    ptplugin::{self, plugin_service_client::PluginServiceClient},
    templates::PatuiStep,
    utils::get_unused_localhost_port,
};

use super::PatuiEventWithTimestamp;

#[cfg(target_os = "windows")]
const PATH_SEPARATOR: char = ';';
#[cfg(not(target_os = "windows"))]
const PATH_SEPARATOR: char = ':';

#[derive(Debug, Error)]
pub enum PatuiStepRunnerError {
    #[error("Subscription not supported: {0}")]
    SubscriptionNotSupported(String),
    #[error("Test set receiver not supported")]
    TestSetReceiverNotSupported,
    #[error("Expression error: {0}")]
    ExprError(#[from] PatuiExprError),
    #[error("Ephemeral port error: {0}")]
    PortError(#[from] crate::utils::PortError),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("gRPC error: {0}")]
    GrpcError(#[from] tonic::Status),
    #[error("Internal error: {0}")]
    InternalError(String),
}

#[derive(Debug)]
pub(crate) struct PatuiStepRunner {
    step: PatuiStep,

    tasks: Vec<JoinHandle<()>>,

    plugin_process: Option<Arc<Mutex<Child>>>,
    client_socket: Option<PluginServiceClient<Channel>>,

    waker_rx: Option<broadcast::Receiver<PatuiData>>,

    // Used for signaling the wait step the run has been triggered.
    run_tx: Option<oneshot::Sender<()>>,
    run_rx: Option<oneshot::Receiver<()>>,
}

impl PatuiStepRunner {
    pub(crate) fn new(step: &PatuiStep, waker_rx: broadcast::Receiver<PatuiData>) -> Self {
        let (run_tx, run_rx) = oneshot::channel();

        Self {
            step: step.clone(),

            tasks: vec![],

            plugin_process: None,
            client_socket: None,

            waker_rx: Some(waker_rx),

            run_rx: Some(run_rx),
            run_tx: Some(run_tx),
        }
    }

    pub(crate) async fn init(
        &mut self,
        _current_step_name: &str,
        _step_runners: HashMap<String, Vec<Arc<Mutex<PatuiStepRunner>>>>,
    ) -> Result<(), PatuiStepRunnerError> {
        let span = tracing::info_span!("init", step_name = self.step.name);
        let _guard = span.enter();

        tracing::trace!("Initializing step runner: {:?}", self);

        let port = get_unused_localhost_port().await?;

        self.spawn_process(port).await?;
        self.connect_to_plugin(port).await?;

        let request = Request::new(ptplugin::get_info::Request {});

        let response = self
            .client_socket
            .as_mut()
            .unwrap()
            .get_info(request)
            .await?;

        tracing::debug!("Plugin info: {:?}", response.into_inner());

        // TODO: Implement here subscriptions of what to listen to, for now everything listens to
        // everything.

        Ok(())
    }

    pub(crate) fn run(
        &mut self,
        tx: mpsc::Sender<(Vec<String>, PatuiEventWithTimestamp)>,
    ) -> Result<(), PatuiStepRunnerError> {
        let span = tracing::info_span!(
            "run",
            step_name = self.step.name,
            plugin = self.step.plugin,
            function = self.step.function
        );

        tracing::trace!("Running step '{}'", self.step.name);

        let run_tx = self.run_tx.take().unwrap();
        let step = self.step.clone();
        let client_socket = self.client_socket.as_ref().unwrap().clone();
        let mut waker_rx = self.waker_rx.take().unwrap();

        self.tasks.push(tokio::spawn(
            async move {
                tracing::info!("Running plugin request: {:?}", step.function);

                let mut client_socket = client_socket.clone();

                let request = Request::new(ptplugin::produce_results::Init {});
                let mut results_stream = client_socket
                    .produce_results(request)
                    .await
                    .unwrap()
                    .into_inner();

                let request = Request::new(ptplugin::run::Request {
                    function: step.function.clone(),
                    args: step
                        .args
                        .iter()
                        .map(|(k, v)| (k.clone(), v.raw().to_string()))
                        .collect::<HashMap<_, _>>(),
                });
                let resp = client_socket.run(request).await.unwrap().into_inner();
                tracing::trace!("Plugin run response: {:?}", resp);

                let run_span = tracing::info_span!("run_stream");
                let step_name = step.name.clone();
                let function_name = step.function;
                let client_socket_clone = client_socket.clone();
                let run_task = tokio::spawn(
                    async move {
                        let mut client_socket = client_socket_clone;
                        loop {
                            let result = results_stream.message().await;
                            let result = match result {
                                Ok(Some(result)) => result,
                                _ => {
                                    tracing::debug!("Stream ended: {:?}", result);
                                    break;
                                }
                            };
                            tracing::trace!("Got response from plugin: {:?}", result);

                            let request = Request::new(ptplugin::ack_result::Request {
                                id: result.id,
                                name: result.name.clone(),
                            });
                            let resp = client_socket
                                .ack_result(request)
                                .await
                                .unwrap()
                                .into_inner();
                            tracing::trace!("Ack response: {:?}", resp);

                            let event = result.data.unwrap().try_into().unwrap();
                            if let Err(e) = tx
                                .send((
                                    vec![step_name.clone(), function_name.clone(), result.name],
                                    event,
                                ))
                                .await
                            {
                                tracing::error!("Failed to send event: {}", e);
                                break;
                            }
                        }
                    }
                    .instrument(run_span),
                );

                run_tx.send(()).unwrap();

                // Plugin receiving results from Patui
                let outbound = async_stream::stream! {
                    loop {
                        let results = waker_rx.recv().await;
                        let Ok(results) = results else {
                            tracing::trace!("Publishing problem: {:?}", results);
                            break;
                        };
                        tracing::trace!("Woke up: {:?}", results);

                        yield ptplugin::receive_results::Request {
                            results: Some(results.try_into().unwrap()),
                        }
                    }
                };

                let mut response = client_socket
                    .receive_results(Request::new(outbound))
                    .await
                    .unwrap()
                    .into_inner();

                // TODO: Handle errors
                loop {
                    let msg = response.message().await;
                    if let Ok(Some(resp)) = msg {
                        tracing::trace!("Got message: {:?}", resp);
                    } else {
                        tracing::trace!("Stream problem: {:?}", msg);
                        break;
                    }
                }

                tracing::trace!("Awaiting plugin run");

                run_task.await.unwrap();

                tracing::info!("{} - Plugin run finished", step.name);
            }
            .instrument(span),
        ));

        Ok(())
    }

    pub(crate) async fn wait(
        &mut self,
        tx: mpsc::Sender<(Vec<String>, PatuiEventWithTimestamp)>,
    ) -> Result<(), PatuiStepRunnerError> {
        let span = tracing::info_span!("wait", step_name = self.step.name);
        let _guard = span.enter();
        drop(tx);

        self.run_rx.take().unwrap().await.map_err(|e| {
            PatuiStepRunnerError::InternalError(format!(
                "Run step never finished to allow wait to proceed: {}",
                e
            ))
        })?;

        tracing::trace!("{} - Waiting", self.step.name);

        let request = Request::new(ptplugin::wait::Request {});

        let mut client_socket = self.client_socket.as_ref().unwrap().clone();
        let response = client_socket.wait(request).await?.into_inner();
        tracing::trace!("{} - Plugin wait response: {:?}", self.step.name, response);
        if !response.diagnostics.is_empty() {
            tracing::error!(
                "{} - Diagnostics: {:?}",
                self.step.name,
                response.diagnostics
            );
            todo!();
        }

        let Some(plugin_process) = self.plugin_process.take() else {
            return Err(PatuiStepRunnerError::InternalError(
                "Plugin process not found".to_string(),
            ));
        };

        plugin_process.lock().await.kill().await.unwrap();

        tracing::trace!("{} - Awaiting process completion", self.step.name);
        plugin_process.lock().await.wait().await.unwrap();
        tracing::trace!("{} - Process complete", self.step.name);

        drop(client_socket);
        self.client_socket = None;

        for task in self.tasks.drain(..) {
            task.await
                .map_err(|e| PatuiStepRunnerError::InternalError(format!("Task failed: {}", e)))?;
        }

        tracing::debug!(
            "Plugin '{}/{}' complete",
            self.step.plugin,
            self.step.function
        );

        Ok(())
    }

    async fn spawn_process(&mut self, port: u16) -> Result<(), PatuiStepRunnerError> {
        let program = self.get_plugin_location(&self.step.plugin)?;
        let mut cmd = Command::new(program);
        cmd.args(["--port", &format!("{}", port)]);
        #[cfg(test)]
        cmd.env("PATUI_LOG", "trace");
        #[cfg(not(test))]
        cmd.env(
            "PATUI_LOG",
            std::env::var("PATUI_LOG").unwrap_or_else(|_| "".to_string()),
        );

        cmd.stdout(std::process::Stdio::piped());

        let mut spawn = cmd.spawn()?;

        let stdout = spawn.stdout.take().unwrap();
        let span = tracing::info_span!("plugin_output", step_name = self.step.name);
        tokio::spawn(
            async move {
                let mut reader = tokio::io::BufReader::new(stdout).lines();
                while let Some(line) = reader.next_line().await.unwrap() {
                    if line.starts_with("ERROR") {
                        tracing::error!("Plugin stderr: {}", line.get(6..).unwrap());
                    } else if line.starts_with("WARN") {
                        tracing::warn!("Plugin stderr: {}", line.get(6..).unwrap());
                    } else if line.starts_with("INFO") {
                        tracing::info!("Plugin stdout: {}", line.get(5..).unwrap());
                    } else if line.starts_with("DEBUG") {
                        tracing::debug!("Plugin stdout: {}", line.get(6..).unwrap());
                    } else if line.starts_with("TRACE") {
                        tracing::trace!("Plugin stdout: {}", line.get(6..).unwrap());
                    } else {
                        tracing::trace!("Plugin stdout: {}", line);
                    }
                }
            }
            .instrument(span),
        );

        self.plugin_process = Some(Arc::new(Mutex::new(spawn)));

        Ok(())
    }

    async fn connect_to_plugin(&mut self, port: u16) -> Result<(), PatuiStepRunnerError> {
        for _ in 0..50 {
            let addr = format!("http://[::1]:{}", port);
            let client = PluginServiceClient::connect(addr).await;
            match client {
                Ok(c) => {
                    self.client_socket = Some(c);
                    return Ok(());
                }
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }

        Err(PatuiStepRunnerError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to connect to the plugin",
        )))
    }

    fn get_plugin_location(&self, plugin: &str) -> Result<String, PatuiStepRunnerError> {
        // If we're a full path just return it.
        if plugin.contains("/") || plugin.contains("\\") {
            return Ok(plugin.to_string());
        }

        // Try to find the plugin in the path somewhere, we use the following in order:
        // 1. The PATUI_PATH environment variable.
        // 2. The current executable path.
        // 3. The OS PATH environment variable.
        let mut search_path = "".to_string();
        if let Ok(patui_path) = env::var("PATUI_PATH") {
            search_path += &format!("{}{}", PATH_SEPARATOR, patui_path);
        }

        search_path += &current_exe()
            .map(|x| {
                x.parent()
                    .unwrap()
                    .to_str()
                    .map(|y| format!("{}{}", y, PATH_SEPARATOR))
            })?
            .unwrap_or_default();

        // Need an extra path for testing, since the binary is in the target/debug folder and the
        // tests in target/debug/deps.
        #[cfg(test)]
        {
            search_path += &current_exe()
                .map(|x| {
                    x.parent()
                        .unwrap()
                        .parent()
                        .unwrap()
                        .to_str()
                        .map(|y| format!("{}{}", y, PATH_SEPARATOR))
                })?
                .unwrap_or_default();
        }

        tracing::trace!("Search path: {:?}", search_path);

        search_path += &env::var("PATH").unwrap_or_else(|_| "".to_string());

        tracing::trace!("Searching path for plugin '{}': '{}'", plugin, search_path);

        for p in env::split_paths(&search_path) {
            for binary in [
                plugin,
                &format!("{}.exe", plugin),
                &format!("patui-{}", plugin),
                &format!("patui-{}.exe", plugin),
            ] {
                let candidate = p.join(binary);
                if candidate.exists() {
                    let candidate = candidate.to_string_lossy().to_string();
                    tracing::trace!("Found plugin at: '{}'", candidate);
                    return Ok(candidate);
                }
            }
        }

        Err(PatuiStepRunnerError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Plugin '{}' not found in PATH", plugin),
        )))
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc, time::Duration};

    use assertor::*;
    use tokio::{
        sync::{broadcast, mpsc, RwLock},
        time::timeout,
    };
    use tracing_test::traced_test;

    use crate::{templates::PatuiStep, PatuiData, PatuiDataInner, PatuiExpr};

    use super::PatuiStepRunner;

    #[traced_test]
    #[tokio::test]
    async fn run_static_data_step() {
        let (waker_tx, waker_rx) = broadcast::channel(1);

        let mut step_runner = PatuiStepRunner::new(
            &PatuiStep {
                name: "static_data_test".to_string(),
                plugin: "std".to_string(),
                function: "static_data".to_string(),
                args: HashMap::from([(
                    "data".to_string(),
                    "[{\"a\": 1}, {\"a\": 2}, {\"a\": 3}]".try_into().unwrap(),
                )]),
                when: None,
                depends_on: vec![],
            },
            waker_rx,
        );

        let res = timeout(
            Duration::from_secs(2),
            step_runner.init("main", HashMap::new()),
        )
        .await;
        assert_that!(res).is_ok();
        assert_that!(res.unwrap()).is_ok();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(step_runner.run(res_tx.clone())).is_ok();

        let task = tokio::spawn(async move {
            let res = timeout(Duration::from_secs(2), step_runner.wait(res_tx)).await;
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_ok();
        });

        for expected_recv in [
            PatuiDataInner::Map(HashMap::from([(
                "a".to_string(),
                PatuiData::Known(PatuiDataInner::Integer(1)),
            )])),
            PatuiDataInner::Map(HashMap::from([(
                "a".to_string(),
                PatuiData::Known(PatuiDataInner::Integer(2)),
            )])),
            PatuiDataInner::Map(HashMap::from([(
                "a".to_string(),
                PatuiData::Known(PatuiDataInner::Integer(3)),
            )])),
        ] {
            let recv = timeout(Duration::from_secs(1), res_rx.recv()).await;
            assert_that!(recv).is_ok();
            let recv = recv.unwrap();
            assert_that!(recv).is_some();
            let recv = recv.unwrap();

            let value = &recv.1.value;
            assert_that!(value.is_results()).is_true();
            assert_that!(value.as_results().unwrap()).is_equal_to((
                &PatuiExpr::try_from("data").unwrap(),
                &PatuiData::Known(expected_recv),
            ));
        }

        let res = timeout(Duration::from_secs(1), res_rx.recv()).await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_none();

        drop(waker_tx);
        drop(res_rx);

        let ret = timeout(Duration::from_secs(2), task).await;
        assert_that!(ret).is_ok();
        let ret = ret.unwrap();
        assert_that!(ret).is_ok();
    }

    #[traced_test]
    #[tokio::test]
    async fn run_simple_known_true_assertion_step() {
        let (waker_tx, waker_rx) = broadcast::channel(1);

        let mut step_runner = PatuiStepRunner::new(
            &PatuiStep {
                name: "assertion_test".to_string(),
                plugin: "std".to_string(),
                function: "assertion".to_string(),
                args: HashMap::from([("expr".to_string(), "foo.bar == 1".try_into().unwrap())]),
                when: None,
                depends_on: vec![],
            },
            waker_rx,
        );

        let res = timeout(
            Duration::from_secs(2),
            step_runner.init("main", HashMap::new()),
        )
        .await;
        assert_that!(res).is_ok();
        assert_that!(res.unwrap()).is_ok();

        let (res_tx, mut res_rx) = mpsc::channel(1);

        assert_that!(step_runner.run(res_tx.clone())).is_ok();

        let task = tokio::spawn(async move {
            let res = timeout(Duration::from_secs(2), step_runner.wait(res_tx)).await;
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_ok();
        });

        waker_tx
            .send(PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                "foo".to_string(),
                PatuiData::Known(PatuiDataInner::Map(HashMap::from([(
                    "bar".to_string(),
                    PatuiData::Known(PatuiDataInner::Integer(1)),
                )]))),
            )]))))
            .unwrap();

        let recv = timeout(Duration::from_secs(1), res_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_some();
        let recv = recv.unwrap();

        let value = &recv.1.value;
        assert_that!(value.is_results()).is_true();
        assert_that!(value.as_results().unwrap()).is_equal_to((
            &PatuiExpr::try_from("assertion").unwrap(),
            &PatuiData::Known(PatuiDataInner::Bool(true)),
        ));

        assert_that!(task.await).is_ok();
    }
}
