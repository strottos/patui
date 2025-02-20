use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
};

// Re-export modules for use in the generated code.
pub use async_stream;
pub use chrono;
pub use clap;
pub use convert_case;
pub use eyre::{eyre, Result};
pub use tokio;
pub use tokio_stream;
pub use tonic;
use tonic::Status;
pub use tracing;
pub use tracing_subscriber;

use tokio::sync::{broadcast, mpsc::Receiver};

pub use patui_core::{
    eval_patui_expr, get_expr_terms, EvalError, PatuiData, PatuiDataInner, PatuiEvent,
    PatuiEventWithTimestamp, PatuiExpr, PatuiStepResult, PatuiStepResultInner,
    PatuiStepResultStatus,
};
pub use patui_plugin_macros::main;

pub mod plugin_server {
    pub use patui_core::ptplugin::{
        get_info, init, plugin_service_client, plugin_service_server::*, receive_results, run,
        shutdown, StepRunner,
    };
}

#[derive(Clone, Debug)]
pub enum WakerType {
    Results,
    Done,
}

pub trait FunctionService {
    fn run(
        &self,
        step_name: String,
        args: HashMap<String, PatuiExpr>,
        results: Arc<RwLock<PatuiData>>,
        results_needed: Arc<Mutex<Vec<PatuiExpr>>>,
        waker_rx: broadcast::Receiver<WakerType>,
    ) -> (
        Receiver<Result<PatuiEvent, Status>>,
        Option<tokio::task::JoinHandle<()>>,
    );
}

#[cfg(feature = "test")]
pub use tests::{check_event_response, connect_plugin, run_plugin, shutdown_plugin, spawn_plugin};

#[cfg(feature = "test")]
mod tests {
    use std::{
        env,
        net::TcpListener,
        process::{Child, Command},
        time::Duration,
    };

    use assertor::*;
    use escargot::CargoBuild;
    use eyre::Result;
    use patui_core::{
        ptplugin::{plugin_service_client::PluginServiceClient, run, shutdown},
        PatuiEvent,
    };
    use tokio::time::timeout;
    use tonic::transport::Channel;

    pub async fn run_plugin(bin_name: &str) -> Result<(Child, PluginServiceClient<Channel>)> {
        let (child, port) = spawn_plugin(bin_name).await?;

        let client = connect_plugin(port).await;

        Ok((child, client))
    }

    pub async fn spawn_plugin(bin_name: &str) -> Result<(Child, u16)> {
        let port = get_unused_localhost_port()?;

        let plugin_binary = CargoBuild::new()
            .bin(bin_name)
            .current_target()
            .manifest_path("Cargo.toml")
            .target_dir("./target/test")
            .run()
            .unwrap();

        let mut cmd = Command::new(plugin_binary.path());
        let child = cmd
            .args(["--port", &port.to_string()])
            .env("PATUI_LOG", env::var("PATUI_LOG").unwrap_or("".to_string()))
            .env(
                "PATUI_LOG_FILE",
                env::var("PATUI_LOG_FILE").unwrap_or("".to_string()),
            )
            .env(
                "RUST_BACKTRACE",
                env::var("RUST_BACKTRACE").unwrap_or("".to_string()),
            )
            .spawn()
            .unwrap();

        assert_that!(child.id()).is_not_equal_to(0);

        Ok((child, port))
    }

    pub async fn connect_plugin(port: u16) -> PluginServiceClient<Channel> {
        for _ in 0..100 {
            let addr = format!("http://[::1]:{}", port);
            let client = PluginServiceClient::connect(addr).await;
            match client {
                Ok(c) => return c,
                Err(_) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }

        panic!("Failed to connect to the plugin");
    }

    fn get_unused_localhost_port() -> Result<u16> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        Ok(listener.local_addr()?.port())
    }

    pub async fn shutdown_plugin(mut child: Child, mut client: PluginServiceClient<Channel>) {
        let res = timeout(
            Duration::from_secs(1),
            client.shutdown(shutdown::Request {}),
        )
        .await;
        assert_that!(res).is_ok();
        let res = res.unwrap();
        assert_that!(res).is_ok();

        for _ in 0..50 {
            let status = child.try_wait();
            if status.is_ok() {
                let status = status.unwrap();
                if status.is_some() {
                    return;
                }
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        child.kill().unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        panic!("Failed to shutdown the plugin");
    }

    pub async fn check_event_response(
        response: Result<Result<Option<run::Response>, tonic::Status>, tokio::time::error::Elapsed>,
    ) -> PatuiEvent {
        assert_that!(response).is_ok();
        let response = response.unwrap();
        assert_that!(response).is_ok();
        let response = response.unwrap();
        assert_that!(response).is_some();
        let response = response.unwrap();
        assert_that!(response.data).is_some();
        let data = response.data;
        assert_that!(data).is_some();
        let event = data.unwrap().try_into();
        assert_that!(event).is_ok();
        tracing::info!("Got event: {:?}", event);
        event.unwrap()
    }
}
