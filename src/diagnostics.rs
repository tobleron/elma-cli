use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum ElmaDiagnostic {
    #[error("invalid skill file: {name}")]
    #[help("skill files must have a .md extension and contain frontmatter")]
    InvalidSkillFile {
        name: String,
        #[label("this file")]
        span: SourceSpan,
    },

    #[error("config parse error: {err}")]
    #[diagnostic(code(elma::config::parse_error))]
    ConfigError {
        #[source_code]
        input: String,
        #[label("here")]
        span: Option<SourceSpan>,
        #[source]
        err: toml::de::Error,
    },

    #[error("json parse error")]
    #[diagnostic(code(elma::json::parse_error))]
    JsonParseError {
        #[label("invalid JSON here")]
        span: SourceSpan,
    },

    #[error("No API base URL configured")]
    #[diagnostic(code(elma::config::missing_base_url), help("Specify the base URL via --base-url, LLAMA_BASE_URL environment variable, or in elma.toml."))]
    MissingBaseUrl,

    #[error("Invalid mode combination")]
    #[diagnostic(
        code(elma::cli::invalid_mode_combination),
        help("Choose only one of --tune, --calibrate, --restore-base, or --restore-last.")
    )]
    InvalidModeCombination,

    #[error("Last-active profile snapshot not found for {model_id}")]
    #[diagnostic(
        code(elma::config::profile_snapshot_not_found),
        help("No previous snapshot was found at {path}. Try tuning the model first.")
    )]
    ProfileSnapshotNotFound { model_id: String, path: String },

    #[error("Model API timeout after {timeout_secs}s")]
    #[diagnostic(code(elma::api::timeout), help("The model API did not respond within the allocated time. Check your connection or the server status."))]
    ModelApiTimeout {
        timeout_secs: u64,
        last_error: String,
    },

    #[error("Model API error after 3 attempts")]
    #[diagnostic(
        code(elma::api::error),
        help("The model API failed repeatedly. Last error: {last_error}")
    )]
    ModelApiError { last_error: String },
}
