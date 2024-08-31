use std::path::PathBuf;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_appender::non_blocking::WorkerGuard;
use anyhow::Result;

/// Initializes the logging system for the application
///
/// This function sets up the logging subscriber and layers based on the provided configuration
/// It supports logging to both the console and a rolling log file (if a file path is provided)
/// The log file is named `iqx-dm_{current_date}.log` and is located in the specified directory
/// The logging level is determined from the environment variable `RUST_LOG` or defaults to "info"
/// The `sqlx` library's logging level is set to "warn" to reduce noise
///
/// # Arguments
///
/// * `log_file_path`: An optional `PathBuf` specifying the directory where the log file should be created
///
/// # Returns
///
/// * `Ok(Some(WorkerGuard))`: If logging is initialized successfully with a file appender, the `WorkerGuard` is returned
/// * `Ok(None)`: If logging is initialized successfully without a file appender (console only)
/// * `Err(anyhow::Error)`: If there's an error initializing the logging system
pub fn init_logger(log_file_path: Option<PathBuf>) -> Result<Option<WorkerGuard>> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info,sqlx=warn"))?;

    let format = fmt::format()
        .with_timer(fmt::time::LocalTime::rfc_3339())
        .compact()
        .with_target(false)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(false);

    let subscriber = tracing_subscriber::registry().with(env_filter);

    if let Some(path) = log_file_path {
        std::fs::create_dir_all(&path)?;

        let file_name = format!(
            "iqx-dm_{}.log",
            chrono::Local::now().format("%Y-%m-%d")
        );
        let log_file_path = path.join(file_name);

        let file_appender = RollingFileAppender::new(
            Rotation::NEVER,
            path,
            log_file_path.file_name().unwrap(),
        );

        let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

        let file_layer = fmt::Layer::default()
            .event_format(format.clone())
            .with_writer(non_blocking);

        let console_layer = fmt::Layer::default()
            .event_format(format.clone())
            .event_format(format.with_ansi(true))
            .with_writer(std::io::stdout);

        let subscriber = subscriber.with(file_layer).with(console_layer);

        tracing::subscriber::set_global_default(subscriber)?;


        tracing::info!("Logging initialized successfully");
        Ok(Some(guard))
    } else {
        let console_layer = fmt::Layer::default()
            .event_format(format.clone())
            .event_format(format.with_ansi(true))
            .with_writer(std::io::stdout);

        let subscriber = subscriber.with(console_layer);
        tracing::subscriber::set_global_default(subscriber)?;

        tracing::info!("Logging initialized successfully (console only)");
        Ok(None)
    }
}