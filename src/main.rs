#![feature(iter_intersperse)]
#![deny(missing_debug_implementations)]

mod cli;
mod db;
mod tui;
mod types;
mod utils;

use std::{env, fs::create_dir_all, sync::Arc};

use clap::Parser;
use etcetera::{choose_app_strategy, AppStrategy, AppStrategyArgs};
use eyre::Result;
use lazy_static::lazy_static;
use tracing::{error, info};
use tracing_subscriber::{
    fmt::writer::BoxMakeWriter, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry,
};

use crate::cli::Cli;

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

fn initialise_panic_handler(is_tui: bool) -> Result<()> {
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
        if is_tui {
            if let Err(r) = crate::tui::exit() {
                error!("Unable to exit Terminal: {:?}", r);
            }
        }

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
        error!("Error: {}", strip_ansi_escapes::strip_str(msg));

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
    info!("Starting Patui");

    let args = Cli::parse();
    if args.subcommand.is_some() {
        initialise_panic_handler(false)?;
    } else {
        initialise_panic_handler(true)?;
    }

    let strategy = choose_app_strategy(AppStrategyArgs {
        top_level_domain: "rs".to_string(),
        author: "strottos".to_string(),
        app_name: "patui".to_string(),
    })?;

    let db_path = match args.db.map(|x| x.into()) {
        Some(path) => path,
        None => {
            let mut path = strategy.data_dir();
            create_dir_all(&path)?;
            path.push("patui.db");
            path
        }
    };

    let db = Arc::new(db::Database::new(&db_path).await?);

    if let Some(subcommand) = args.subcommand {
        subcommand.handle(db).await?;
    } else {
        // TUI time
        let mut app = tui::App::new(db)?;
        app.run().await?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    initialise_logging()?;

    do_main().await
}
