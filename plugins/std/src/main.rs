//! # Patui Standard Plugin
//!
//! The `patui-std` plugin is a standard plugin for the `patui` CLI. It provides a set of standard
//! commands that are useful for testing.
//!
//! The following are some of the more popular commands in the `patui-std` plugin:
//! - Assert: Asserts that a condition is true and fails the test if it is not.
//! - ReadFile: Reads a file from the filesystem that can be read by other steps.
//! - WriteFile: Writes a file to the filesystem with data from other steps.
//! - StaticData: Provides static data that can be used by other steps.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod server;

use std::env;

use clap::{command, Parser};
use miette::{IntoDiagnostic, Result};
use tokio::sync::oneshot;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

use crate::{ptplugin::plugin_service_server::PluginServiceServer, server::StdPlugin};

#[allow(missing_docs)]
pub mod ptplugin {
    tonic::include_proto!("ptplugin");
}

#[derive(Parser, Debug)]
#[command(author, version = version(), about)]
pub(crate) struct Cli {
    #[clap(short, long)]
    pub(crate) port: Option<String>,
}

const VERSION_MESSAGE: &str = concat!(
    env!("CARGO_PKG_NAME"),
    " ",
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("CARGO_PKG_DESCRIPTION"),
    ")"
);

fn version() -> String {
    let author = clap::crate_authors!();

    format!(
        "\
{VERSION_MESSAGE}

Authors: {author}"
    )
}

fn initialise_logging() -> Result<()> {
    let filter = match env::var("PATUI_LOG") {
        Ok(log) => Some(log),
        Err(_) => return Ok(()),
    };
    let var_name = EnvFilter::default();
    let filter = filter.map_or(var_name, EnvFilter::new);

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_file(true)
        .with_line_number(true)
        .with_target(true)
        .with_ansi(true);

    Registry::default().with(filter).with(fmt_layer).init();

    Ok(())
}

async fn do_main() -> Result<()> {
    tracing::info!("Starting Patui Test Plugin");

    let args = Cli::parse();

    let Some(port) = args.port else {
        tracing::error!("No port provided");
        std::process::exit(-1);
    };
    let addr = format!("[::1]:{}", port);
    let addr = addr.parse().unwrap();

    let (tx, rx) = oneshot::channel();

    let plugin = StdPlugin::new(tx);

    Server::builder()
        .add_service(PluginServiceServer::new(plugin))
        .serve_with_shutdown(addr, async {
            rx.await.ok();
            tracing::info!("Shutting down");
        })
        .await
        .into_diagnostic()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    initialise_logging()?;

    do_main().await
}
