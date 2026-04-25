//! @efficiency-role: util-pure
//!
//! Structured logging with tracing.
//!
//! Provides span-based instrumentation and level-controlled output.
//! Can be configured via RUST_LOG environment variable.

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize the global tracing subscriber.
///
/// Output is directed to stderr.
/// Default level is INFO, unless verbose is true (DEBUG) or RUST_LOG is set.
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
