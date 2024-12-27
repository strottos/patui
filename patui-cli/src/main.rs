//! # Patui
//!
//! The `patui` CLI is a command line tool to automate your testing workflow. If you like Terraform
//! or Ansible, you'll love `patui`. It's a simple, yet powerful tool to automate your testing.

#![deny(missing_debug_implementations)]
#![deny(missing_docs)]

mod cli;

use std::{env, fs::create_dir_all, sync::Arc};

use lazy_static::lazy_static;
use miette::{IntoDiagnostic, Result};
use tracing_subscriber::{
    fmt::writer::BoxMakeWriter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};

lazy_static! {
    /// Various constants used in the root application code
    pub(crate) static ref PROJECT_NAME: String = env!("CARGO_CRATE_NAME").to_uppercase().to_string();
    /// Various constants used in the root application code
    pub(crate) static ref LOG_ENV: String = format!("{}_LOG", PROJECT_NAME.clone());
    /// Various constants used in the root application code
    pub(crate) static ref LOG_FILE_ENV: String = format!("{}_LOG_FILE", PROJECT_NAME.clone());
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
        create_dir_all(parent).into_diagnostic()?;
    }
    let log_file = std::fs::File::create(path).into_diagnostic()?;

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
        .panic_section(
            "This is a bug. Please consider reporting it at the information given above.\nIf you can compile the code with debug information and running that, consider submitting that, but if not that's OK too.",
        )
        .capture_span_trace_by_default(true)
        .display_location_section(true)
        .display_env_section(false)
        .into_hooks();
    eyre_hook.install().into_diagnostic()?;
    std::panic::set_hook(Box::new(move |panic_info| {
        // TODO: Tui reset if needed.
        use human_panic::{handle_dump, print_msg, Metadata};
        let meta = Metadata::new(
            env!("CARGO_PKG_NAME").to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        )
        .authors(env!("CARGO_PKG_AUTHORS").replace(':', ", ").to_string())
        .homepage(env!("CARGO_PKG_REPOSITORY").to_string());

        let file_path = handle_dump(&meta, panic_info);
        // prints human-panic message
        print_msg(file_path, &meta).expect("human-panic: printing error message to console failed");
        eprintln!("{}", panic_hook.panic_report(panic_info)); // prints color-eyre stack trace to stderr
                                                              //
        let msg = format!("{}", panic_hook.panic_report(panic_info));
        tracing::error!("Error: {}", strip_ansi_escapes::strip_str(msg));

        std::process::exit(libc::EXIT_FAILURE);
    }));

    Ok(())
}

async fn do_main() -> Result<()> {
    tracing::info!("Starting Patui");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    initialise_logging()?;
    initialise_panic_handler()?;

    do_main().await
}
