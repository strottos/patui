use std::{collections::HashMap, sync::Arc};

use crate::{
    runner::steps::init_subscribe_steps, types::steps::PatuiStepPlugin,
    utils::get_unused_localhost_port,
};

use eyre::{eyre, Result};
use tokio::{
    process::{Child, Command},
    sync::{broadcast, oneshot, Mutex},
    task::JoinHandle,
};
use tonic::{transport::Channel, Request};

use crate::types::ptplugin::{self, get_info, plugin_service_client::PluginServiceClient};

use super::{Expr, PatuiStepData, PatuiStepRunner, PatuiStepRunnerTrait};

#[derive(Debug)]
pub(crate) struct PatuiStepRunnerPlugin {
    step_name: String,
    step: PatuiStepPlugin,

    receivers: Option<HashMap<Expr, broadcast::Receiver<PatuiStepData>>>,
    tasks: Vec<JoinHandle<()>>,

    plugin_process: Option<Arc<Mutex<Child>>>,
    client_socket: Option<PluginServiceClient<Channel>>,

    run_tx: Option<oneshot::Sender<()>>,
    run_rx: Option<oneshot::Receiver<()>>,
}

impl PatuiStepRunnerPlugin {
    pub(crate) fn new(step_name: String, step: &PatuiStepPlugin) -> Self {
        let (run_tx, run_rx) = oneshot::channel();

        Self {
            step_name,
            step: step.clone(),

            receivers: None,
            tasks: vec![],

            plugin_process: None,
            client_socket: None,

            run_tx: Some(run_tx),
            run_rx: Some(run_rx),
        }
    }

    async fn run_process(&mut self) -> Result<()> {
        let mut cmd = Command::new(&self.step.path);
        let port = get_unused_localhost_port().await?;
        cmd.args(["--port", &format!("{}", port)]);

        self.plugin_process = Some(Arc::new(Mutex::new(cmd.spawn()?)));

        // TODO: This is a hack to wait for the plugin to start up, rework as polling at some point
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        let addr = format!("http://[::1]:{}", port);
        let mut client = PluginServiceClient::connect(addr).await?;

        let request = Request::new(get_info::Request {});

        let response = client.get_info(request).await?;

        tracing::debug!("Plugin info: {:?}", response);

        self.client_socket = Some(client);

        Ok(())
    }
}

impl PatuiStepRunnerTrait for PatuiStepRunnerPlugin {
    async fn init(
        &mut self,
        current_step_name: &str,
        step_runners: HashMap<String, Vec<Arc<std::sync::Mutex<PatuiStepRunner>>>>,
    ) -> Result<()> {
        let mut receivers = HashMap::new();
        for r#in in self.step.r#in.values() {
            let receivers_found =
                init_subscribe_steps(r#in, current_step_name, &step_runners).await?;
            receivers.extend(receivers_found);
        }
        self.receivers = Some(receivers);

        self.run_process().await?;

        Ok(())
    }

    fn run(&mut self, _tx: tokio::sync::mpsc::Sender<super::PatuiEvent>) -> Result<()> {
        let client_socket = self.client_socket.as_ref().unwrap().clone();

        let run_tx = self.run_tx.take().unwrap();
        let receivers = self.receivers.take();
        let step = self.step.clone();

        self.tasks.push(tokio::spawn(async move {
            tracing::info!("Running plugin");

            let client_socket = client_socket.clone();
            let request = Request::new(ptplugin::run::Request {});

            tracing::trace!("Requesting run");

            client_socket.clone().run(request).await.unwrap();
            run_tx.send(()).unwrap();

            let Some(receivers) = receivers else {
                panic!("No receivers found");
            };

            let mut tasks = vec![];

            for (r#in, receiver) in receivers.into_iter() {
                let client_socket = client_socket.clone();
                let name = step
                    .r#in
                    .iter()
                    .find(|(_, v)| v.expr == r#in)
                    .unwrap()
                    .0
                    .clone();

                tasks.push(tokio::spawn(async move {
                    let mut client_socket = client_socket.clone();
                    let outbound = async_stream::stream! {
                        let mut receiver = receiver;
                        while let Ok(data) = receiver.recv().await {
                            tracing::trace!("Got data from receiver: {:?}", data);

                            yield ptplugin::publish::Request {
                                name: name.clone(),
                                data: Some(data.try_into().unwrap()),
                            }
                        }
                    };

                    let mut response = client_socket
                        .publish(Request::new(outbound))
                        .await
                        .unwrap()
                        .into_inner();

                    let Ok(Some(resp)) = response.message().await else {
                        panic!("No response");
                    };
                    tracing::trace!("RESP = {:?}", resp);
                }));
            }

            for task in tasks.into_iter() {
                task.await.unwrap();
            }

            tracing::trace!("All tasks complete");
        }));

        Ok(())
    }

    async fn subscribe(
        &mut self,
        sub: &str,
    ) -> Result<tokio::sync::broadcast::Receiver<super::PatuiStepData>> {
        let request = Request::new(ptplugin::subscribe::Request {
            name: sub.to_string(),
        });

        let (tx, rx) = broadcast::channel(32); // TODO: Make this configurable

        let mut client_socket = self.client_socket.as_ref().unwrap().clone();
        let mut stream = client_socket.subscribe(request).await?.into_inner();

        drop(client_socket);

        let sub = sub.to_string();

        self.tasks.push(tokio::spawn(async move {
            let sub = sub;
            while let Ok(Some(response)) = stream.message().await {
                tracing::trace!(
                    "Got subscription message for sub '{}': {:?}",
                    &sub,
                    response
                );
                tx.send(response.data.unwrap().try_into().unwrap()).unwrap();
            }
        }));

        Ok(rx)
    }

    async fn wait(&mut self) -> Result<()> {
        self.run_rx.take().unwrap().await?;

        tracing::trace!("Waiting");

        let request = Request::new(ptplugin::wait::Request {});

        let mut client_socket = self.client_socket.as_ref().unwrap().clone();
        let response = client_socket.wait(request).await?.into_inner();
        tracing::trace!("Plugin wait response: {:?}", response);
        if !response.diagnostics.is_empty() {
            tracing::error!("Diagnostics: {:?}", response.diagnostics);
            todo!();
        }

        let Some(plugin_process) = self.plugin_process.take() else {
            return Err(eyre!("Plugin process not found"));
        };

        plugin_process.lock().await.kill().await.unwrap();

        tracing::trace!("Awaiting process completion");
        plugin_process.lock().await.wait().await.unwrap();
        tracing::trace!("Process complete");

        drop(client_socket);
        self.client_socket = None;

        for task in self.tasks.drain(..) {
            task.await?;
        }

        tracing::debug!("Plugin complete {}", self.step_name);

        Ok(())
    }

    #[cfg(test)]
    fn test_set_receiver(
        &mut self,
        sub_ref: &str,
        rx: broadcast::Receiver<PatuiStepData>,
    ) -> Result<()> {
        use super::PatuiExpr;

        let sub_ref_expr: PatuiExpr = sub_ref.try_into()?;
        let receivers = HashMap::from([(sub_ref_expr.expr, rx)]);
        self.receivers = Some(receivers);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{process::Command, time::Duration};

    use assertor::*;
    use lazy_static::lazy_static;
    use tokio::{sync::mpsc, time::timeout};
    use tracing_test::traced_test;

    use crate::types::PatuiStepDataFlavour;

    use super::*;

    lazy_static! {
        static ref COMPILED: std::sync::Mutex<bool> = std::sync::Mutex::new(false);
    }

    fn compile_program() {
        let mut lock = COMPILED.lock().unwrap();

        if *lock {
            return;
        }

        *lock = true;

        tracing::trace!("Compiling test plugin program");

        let output = Command::new("cargo")
            .arg("build")
            .current_dir("test_progs/test_plugin")
            .output()
            .unwrap();

        assert!(output.status.success());
    }

    #[traced_test]
    #[tokio::test]
    async fn test_simple_plugin() {
        compile_program();

        let mut main_step = PatuiStepRunnerPlugin::new(
            "main".to_string(),
            &PatuiStepPlugin {
                path: "./test_progs/test_plugin/target/debug/test_patui_plugin".to_string(),
                config: HashMap::new(),
                r#in: HashMap::new(),
            },
        );

        let res = timeout(
            Duration::from_secs(2),
            main_step.init("main", HashMap::new()),
        )
        .await;
        assert_that!(res).is_ok();
        assert_that!(res.unwrap()).is_ok();

        let output_res = timeout(Duration::from_secs(5), main_step.subscribe("out")).await;

        assert_that!(output_res).is_ok();
        let output_res = output_res.unwrap();
        assert_that!(output_res).is_ok();
        let mut output_rx = output_res.unwrap();

        let (res_tx, res_rx) = mpsc::channel(1);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let task = tokio::spawn(async move {
            let res = timeout(Duration::from_secs(2), main_step.wait()).await;
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_ok();
        });

        for expected_recv in [
            PatuiStepDataFlavour::Null,
            PatuiStepDataFlavour::Bool(true),
            PatuiStepDataFlavour::String("test".to_string()),
            PatuiStepDataFlavour::Array(vec![
                PatuiStepDataFlavour::Integer("1".to_string()),
                PatuiStepDataFlavour::Integer("2".to_string()),
                PatuiStepDataFlavour::Integer("3".to_string()),
            ]),
            PatuiStepDataFlavour::Map(HashMap::from([
                (
                    "a".to_string(),
                    PatuiStepDataFlavour::Integer("1".to_string()),
                ),
                (
                    "b".to_string(),
                    PatuiStepDataFlavour::Integer("2".to_string()),
                ),
            ])),
        ] {
            let recv = timeout(Duration::from_secs(10), output_rx.recv()).await;
            assert_that!(recv).is_ok();
            let recv = recv.unwrap();
            assert_that!(recv).is_ok();
            assert_that!(recv.unwrap().data).is_equal_to(&expected_recv);
        }

        drop(output_rx);
        drop(res_rx);

        assert_that!(task.await).is_ok();
    }

    #[traced_test]
    #[tokio::test]
    async fn test_echo_plugin() {
        compile_program();

        let mut main_step = PatuiStepRunnerPlugin::new(
            "main".to_string(),
            &PatuiStepPlugin {
                path: "./test_progs/test_plugin/target/debug/test_patui_plugin".to_string(),
                config: HashMap::new(),
                r#in: HashMap::from([(
                    "echo".to_string(),
                    "steps.test_input.out".try_into().unwrap(),
                )]),
            },
        );

        let res = timeout(Duration::from_secs(2), main_step.run_process()).await;
        assert_that!(res).is_ok();
        assert_that!(res.unwrap()).is_ok();

        let output_res = timeout(Duration::from_secs(5), main_step.subscribe("echo")).await;

        assert_that!(output_res).is_ok();
        let output_res = output_res.unwrap();
        assert_that!(output_res).is_ok();
        let mut output_rx = output_res.unwrap();

        let (res_tx, _) = mpsc::channel(1);

        let (input_tx, input_rx) = broadcast::channel(32);

        assert_that!(main_step.test_set_receiver("steps.test_input.out", input_rx)).is_ok();

        input_tx
            .send(PatuiStepData::new(PatuiStepDataFlavour::Integer(
                r#"1"#.to_string(),
            )))
            .unwrap();
        input_tx
            .send(PatuiStepData::new(PatuiStepDataFlavour::Integer(
                r#"2"#.to_string(),
            )))
            .unwrap();
        input_tx
            .send(PatuiStepData::new(PatuiStepDataFlavour::Integer(
                r#"3"#.to_string(),
            )))
            .unwrap();

        drop(input_tx);

        assert_that!(main_step.run(res_tx.clone())).is_ok();

        let task = tokio::spawn(async move {
            let res = timeout(Duration::from_secs(5), main_step.wait()).await;
            assert_that!(res).is_ok();
            assert_that!(res.unwrap()).is_ok();
        });

        for expected in ["1", "2", "3"] {
            let recv = timeout(Duration::from_secs(2), output_rx.recv()).await;
            assert_that!(recv).is_ok();
            let recv = recv.unwrap();
            assert_that!(recv).is_ok();
            assert_that!(recv.unwrap().data)
                .is_equal_to(PatuiStepDataFlavour::Integer(expected.to_string()));
        }

        assert_that!(task.await).is_ok();
    }
}
