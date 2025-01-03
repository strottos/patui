use std::{collections::HashMap, sync::Arc};

// Re-export modules for use in the generated code.
pub use async_stream;
pub use clap;
pub use convert_case;
pub use eyre::{eyre, Result};
pub use tokio;
pub use tokio_stream;
pub use tonic;
pub use tracing;
pub use tracing_subscriber;

use tokio::sync::{mpsc, Mutex};
use tonic::Status;

pub use patui_core::{
    eval_patui_expr, EvalError, PatuiData, PatuiDataInner, PatuiEvent, PatuiEventWithTimestamp,
    PatuiExpr,
};
pub use patui_plugin_macros::main;

pub mod plugin_server {
    pub use patui_core::ptplugin::{
        get_info, init, plugin_service_server::*, publish, run, shutdown, wait, StepRunner,
    };
}

pub trait FunctionService {
    fn run(
        &self,
        args: HashMap<String, String>,
        tx: mpsc::Sender<std::result::Result<plugin_server::run::Response, Status>>,
        results: Arc<Mutex<PatuiData>>,
        waker_rx: mpsc::Receiver<()>,
    ) -> Result<tokio::task::JoinHandle<()>>;
}
