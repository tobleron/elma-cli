//! UI parity harness command-line tool.
//!
//! Usage: ui_parity_harness <fixture-name>
//!
//! Builds and runs a named fixture against the Elma binary,
//! writes the normalized output to stdout (for snapshot capture).

use anyhow::Result;
use clap::Parser;
use elma_cli_ui_parity::{self as ui_parity, Fixture};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[clap(name = "ui_parity_harness", about = "Runs a UI parity fixture against Elma")]
struct Args {
    /// Fixture name (without .yaml) from tests/fixtures/ui_parity/
    #[clap(value_parser)]
    fixture: String,

    /// Path to elma binary (default: target/debug/elma)
    #[clap(long, short = 'b')]
    binary: Option<PathBuf>,
}

impl Args {
    fn elma_binary(&self) -> PathBuf {
        self.binary.clone().unwrap_or_else(|| {
            PathBuf::from("target/debug/elma")
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Load fixture
    let fixture = Fixture::load(&args.fixture)
        .with_context(|| format!("loading fixture '{}'", args.fixture))?;

    // Run the harness (the heavy lifting is in our library)
    // For now, we'll call the library function that spawns Elma.
    // That function needs to be refactored to accept binary path.
    println!("Fixture: {} — {}", fixture.name, fixture.description);
    println!("Running against binary: {:?}", args.elma_binary());
    println!("Steps: {}", fixture.steps.len());

    // Placeholder — real harness integration next iteration
    println!("\n[HARNESS NOT YET CONNECTED TO LIVE ELMA BINARY]");
    println!("Task 167: Spec + harness skeleton complete. Live PTY driving in T169+.");

    // Show what we'd do:
    for (i, step) in fixture.steps.iter().enumerate() {
        println!("  step {}: input={:?} wait_for={:?} delay={:?}ms",
                 i, step.input, step.wait_for, step.delay_ms);
    }

    Ok(())
}
