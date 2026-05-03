//! @efficiency-role: util-pure
//!
//! Structured logging with tracing.
//!
//! Provides span-based instrumentation and level-controlled output.
//! Can be configured via RUST_LOG environment variable.
//! Session-specific log files are created when a session path is provided.

use std::path::Path;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize the global tracing subscriber.
///
/// Output is directed to stderr.
/// Default level is INFO, unless verbose is true (DEBUG) or RUST_LOG is set.
/// If `session_log_path` is provided, logs are also written to that file.
pub fn init_logging(verbose: bool) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(if verbose {
            "elma_cli=debug,elma=debug"
        } else {
            "elma_cli=info,elma=info"
        })
    });

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr).with_ansi(true))
        .with(filter)
        .init();
}

/// Initialize a session-specific log file writer.
/// Call after init_logging() and after the session path is known.
pub fn init_session_log(session_root: &Path) {
    let log_path = session_root.join("session.log");
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(file) = std::fs::File::create(&log_path) {
        let file_layer = fmt::layer()
            .with_writer(std::sync::Mutex::new(file))
            .with_ansi(false)
            .with_target(false);
        tracing_subscriber::registry()
            .with(file_layer)
            .init();
    }
}
