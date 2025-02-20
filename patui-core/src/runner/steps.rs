use std::{
    collections::HashMap,
    env::{self, current_exe},
    sync::{Arc, Mutex},
};

use thiserror::Error;
use tokio::{
    io::AsyncBufReadExt,
    process::{Child, Command},
    sync::mpsc,
    task::JoinHandle,
};
use tonic::{transport::Channel, Request};
use tracing::Instrument;

use crate::{
    expr::PatuiExprError,
    ptplugin::{self, plugin_service_client::PluginServiceClient},
    templates::PatuiStep,
    utils::get_unused_localhost_port,
};

use super::{results::PatuiStepResult, PatuiEventWithTimestamp};

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
}

impl PatuiStepRunner {
    pub(crate) fn new(step: &PatuiStep) -> Self {
        Self {
            step: step.clone(),

            tasks: vec![],

            plugin_process: None,
            client_socket: None,
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
        mut results_rx: mpsc::Receiver<PatuiStepResult>,
    ) -> Result<mpsc::Receiver<PatuiEventWithTimestamp>, PatuiStepRunnerError> {
        let span = tracing::info_span!(
            "run",
            step_name = self.step.name,
            plugin = self.step.plugin,
            function = self.step.function
        );

        tracing::trace!("Running step '{}'", self.step.name);

        let (receive_results_tx, receive_results_rx) = mpsc::channel(256);

        // // let run_tx = self.run_tx.take().unwrap();
        let step = self.step.clone();
        let client_socket = self.client_socket.as_ref().unwrap().clone();
        // let mut results_rx = self.results_rx.take().unwrap();

        self.tasks.push(tokio::spawn(
            async move {
                tracing::info!("Running plugin request: {:?}", step.function);

                let mut client_socket = client_socket.clone();

                // Plugin receiving results from Patui
                let outbound = async_stream::stream! {
                    loop {
                        let result = match results_rx.recv().await {
                            Some(result) => result,
                            None => {
                                break;
                            }
                        };
                        tracing::trace!("Sending plugin result to plugin: {:?}", result);

                        yield ptplugin::receive_results::Request {
                            result: Some(result.try_into().unwrap()),
                        }
                    }
                };

                let receive_results_stream = client_socket
                    .receive_results(Request::new(outbound))
                    .await
                    .unwrap()
                    .into_inner();

                let step_name = step.name;
                let function_name = step.function;

                let request = Request::new(ptplugin::run::Request {
                    step_name: step_name.clone(),
                    function: function_name.clone(),
                    args: step
                        .args
                        .iter()
                        .map(|(k, v)| (k.clone(), v.raw().to_string()))
                        .collect::<HashMap<_, _>>(),
                });

                tracing::trace!("Sending run request");
                let mut run_results_stream = client_socket.run(request).await.unwrap().into_inner();
                tracing::trace!("Plugin run response: {:?}", run_results_stream);

                loop {
                    let result = run_results_stream.message().await;
                    let result = match result {
                        Ok(Some(result)) => result,
                        _ => {
                            panic!("TODO: Results stream ended: {:?}", result);
                        }
                    };
                    tracing::trace!("Got result from plugin: {:?}", result);

                    let event = result.data.unwrap().try_into().unwrap();
                    tracing::trace!("Got event from plugin: {:?}", event);
                    if let Err(e) = receive_results_tx.send(event).await {
                        tracing::error!("Failed to send event: {}", e);
                        break;
                    }
                }

                //         //         // TODO: Handle errors
                //         //         loop {
                //         //             let msg = response.message().await;
                //         //             match msg {
                //         //                 Ok(Some(resp)) => tracing::trace!("Got message: {:?}", resp),
                //         //                 Ok(None) => {
                //         //                     tracing::info!("Stream ended");
                //         //                     break;
                //         //                 }
                //         //                 Err(e) => {
                //         //                     tracing::error!("Stream problem: {:?}", e);
                //         //                     break;
                //         //                 }
                //         //             }
                //         //         }

                //         //         tracing::trace!("Awaiting plugin run");

                //         //         run_task.await.unwrap();

                //         //         tracing::info!("{} - Plugin run finished", step.name);
            }
            .instrument(span),
        ));

        Ok(receive_results_rx)
    }

    // pub(crate) async fn wait(
    //     &mut self,
    //     tx: mpsc::Sender<(String, String, PatuiEventWithTimestamp)>,
    // ) -> Result<(), PatuiStepRunnerError> {
    //     // let span = tracing::info_span!("wait", step_name = self.step.name);
    //     // let _guard = span.enter();
    //     // drop(tx);

    //     // self.run_rx.take().unwrap().await.map_err(|e| {
    //     //     PatuiStepRunnerError::InternalError(format!(
    //     //         "Run step never finished to allow wait to proceed: {}",
    //     //         e
    //     //     ))
    //     // })?;

    //     // tracing::trace!("{} - Waiting", self.step.name);

    //     // let request = Request::new(ptplugin::wait::Request { client_id: 1 });

    //     // let mut client_socket = self.client_socket.as_ref().unwrap().clone();
    //     // let response = client_socket.wait(request).await?.into_inner();
    //     // tracing::trace!("{} - Plugin wait response: {:?}", self.step.name, response);
    //     // if !response.diagnostics.is_empty() {
    //     //     tracing::error!(
    //     //         "{} - Diagnostics: {:?}",
    //     //         self.step.name,
    //     //         response.diagnostics
    //     //     );
    //     //     todo!();
    //     // }

    //     // let Some(plugin_process) = self.plugin_process.take() else {
    //     //     return Err(PatuiStepRunnerError::InternalError(
    //     //         "Plugin process not found".to_string(),
    //     //     ));
    //     // };

    //     // plugin_process.lock().await.kill().await.unwrap();

    //     // tracing::trace!("{} - Awaiting process completion", self.step.name);
    //     // plugin_process.lock().await.wait().await.unwrap();
    //     // tracing::trace!("{} - Process complete", self.step.name);

    //     // drop(client_socket);
    //     // self.client_socket = None;

    //     // for task in self.tasks.drain(..) {
    //     //     task.await
    //     //         .map_err(|e| PatuiStepRunnerError::InternalError(format!("Task failed: {}", e)))?;
    //     // }

    //     // tracing::debug!(
    //     //     "Plugin '{}/{}' complete",
    //     //     self.step.plugin,
    //     //     self.step.function
    //     // );

    //     Ok(())
    // }

    async fn spawn_process(&mut self, port: u16) -> Result<(), PatuiStepRunnerError> {
        let program = self.get_plugin_location(&self.step.plugin)?;
        let mut cmd = Command::new(program);
        cmd.args(["--port", &format!("{}", port)]);
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

    // We choose a low number to retry here as on fast systems these things can come up remarkably
    // quickly.
    async fn connect_to_plugin(&mut self, port: u16) -> Result<(), PatuiStepRunnerError> {
        for _ in 0..1000 {
            let addr = format!("http://[::1]:{}", port);
            let client = PluginServiceClient::connect(addr).await;
            match client {
                Ok(c) => {
                    tracing::trace!("Connected");
                    self.client_socket = Some(c);
                    return Ok(());
                }
                Err(_) => {
                    tracing::trace!("Failed to connect to plugin, retrying in 5ms...");
                    tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
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
    use std::{collections::HashMap, time::Duration};

    use assertor::*;
    use tokio::{sync::mpsc, time::timeout};
    use tracing_test::traced_test;

    use crate::{templates::PatuiStep, PatuiData, PatuiDataInner, PatuiEvent, PatuiStepResult};

    use super::PatuiStepRunner;

    #[cfg(feature = "integration_tests")]
    #[traced_test]
    #[tokio::test]
    async fn run_static_data_step() {
        let mut step_runner = PatuiStepRunner::new(&PatuiStep {
            name: "static_data_test".to_string(),
            plugin: "testing-plugin".to_string(),
            function: "echo".to_string(),
            args: HashMap::from([(
                "in".to_string(),
                "[{\"a\": 1}, {\"a\": 2}, {\"a\": 3}]".try_into().unwrap(),
            )]),
            when: None,
            depends_on: vec![],
        });

        let res = timeout(
            Duration::from_secs(2),
            step_runner.init("main", HashMap::new()),
        )
        .await;
        assert_that!(res).is_ok();
        assert_that!(res.unwrap()).is_ok();

        // Patui sending results to the plugin, nothing for this test
        let (_, send_res_rx) = mpsc::channel(1);

        let run_res = step_runner.run(send_res_rx);
        assert_that!(run_res).is_ok();
        // Patui receiving new results from the plugin
        let mut receive_results_rx = run_res.unwrap();

        for (idx, expected_recv) in [
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
        ]
        .into_iter()
        .enumerate()
        {
            let recv = timeout(Duration::from_secs(2), receive_results_rx.recv()).await;
            assert_that!(recv).is_ok();
            let recv = recv.unwrap();
            assert_that!(recv).is_some();
            let recv = recv.unwrap();
            let event = recv.value;
            tracing::trace!("Event: {:?}", event);
            assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
                "steps.static_data_test.echo.out".try_into().unwrap(),
                true.into(),
                idx,
                PatuiData::Known(expected_recv),
            )));
        }

        let recv = timeout(Duration::from_secs(2), receive_results_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_some();
        let recv = recv.unwrap();
        let event = recv.value;
        tracing::trace!("Event: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
            "steps.static_data_test.echo.out".try_into().unwrap(),
            true.into(),
            3,
        )));
    }

    #[cfg(feature = "integration_tests")]
    #[traced_test]
    #[tokio::test]
    async fn run_simple_known_true_assertion_step() {
        let mut step_runner = PatuiStepRunner::new(&PatuiStep {
            name: "static_data_test".to_string(),
            plugin: "testing-plugin".to_string(),
            function: "echo".to_string(),
            args: HashMap::from([(
                "in".to_string(),
                "steps.foo.bar.results".try_into().unwrap(),
            )]),
            when: None,
            depends_on: vec![],
        });

        let res = timeout(
            Duration::from_secs(2),
            step_runner.init("main", HashMap::new()),
        )
        .await;
        assert_that!(res).is_ok();
        assert_that!(res.unwrap()).is_ok();

        // Patui sending results to the plugin
        let (send_res_tx, mut send_res_rx) = mpsc::channel(32);

        let run_res = step_runner.run(send_res_rx);
        assert_that!(run_res).is_ok();
        // Patui receiving new results from the plugin
        let mut receive_results_rx = run_res.unwrap();

        let task = tokio::spawn(async move {
            send_res_tx
                .send(PatuiStepResult::new_stream_item(
                    "steps.foo.bar.results".try_into().unwrap(),
                    true.into(),
                    0,
                    PatuiData::Known(PatuiDataInner::Integer(0)),
                ))
                .await
                .unwrap();

            send_res_tx
                .send(PatuiStepResult::new_stream_item(
                    "steps.foo.bar.results".try_into().unwrap(),
                    true.into(),
                    1,
                    PatuiData::Known(PatuiDataInner::Integer(1)),
                ))
                .await
                .unwrap();

            send_res_tx
                .send(PatuiStepResult::new_stream_item(
                    "steps.foo.bar.results".try_into().unwrap(),
                    true.into(),
                    2,
                    PatuiData::Known(PatuiDataInner::Integer(2)),
                ))
                .await
                .unwrap();

            send_res_tx
                .send(PatuiStepResult::done_stream(
                    "steps.foo.bar.results".try_into().unwrap(),
                    true.into(),
                    3,
                ))
                .await
                .unwrap();
        });

        for (idx, expected_recv) in [
            PatuiDataInner::Integer(0),
            PatuiDataInner::Integer(1),
            PatuiDataInner::Integer(2),
        ]
        .into_iter()
        .enumerate()
        {
            let recv = timeout(Duration::from_secs(2), receive_results_rx.recv()).await;
            assert_that!(recv).is_ok();
            let recv = recv.unwrap();
            assert_that!(recv).is_some();
            let recv = recv.unwrap();
            let event = recv.value;
            tracing::trace!("Event: {:?}", event);
            assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::new_stream_item(
                "steps.static_data_test.echo.out".try_into().unwrap(),
                true.into(),
                idx,
                PatuiData::Known(expected_recv),
            )));
        }

        let recv = timeout(Duration::from_secs(2), receive_results_rx.recv()).await;
        assert_that!(recv).is_ok();
        let recv = recv.unwrap();
        assert_that!(recv).is_some();
        let recv = recv.unwrap();
        let event = recv.value;
        tracing::trace!("Event: {:?}", event);
        assert_that!(event).is_equal_to(PatuiEvent::Result(PatuiStepResult::done_stream(
            "steps.static_data_test.echo.out".try_into().unwrap(),
            true.into(),
            3,
        )));

        panic!("logs");
    }
}
