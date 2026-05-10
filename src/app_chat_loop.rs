//! @efficiency-role: service-orchestrator
//!
//! App Chat - Main Chat Loop Orchestration

use crate::app::*;
use crate::orchestration::chat::ChatStateMachine;
use crate::ui_terminal::TerminalUI;
use crate::*;

pub(crate) async fn run_chat_loop(runtime: &mut AppRuntime) -> Result<()> {
    let tui = TerminalUI::new(Some(runtime.state.session.root.clone()))
        .await
        .context("Failed to initialize Terminal UI")?;

    let state_machine = ChatStateMachine::new(runtime, tui);
    state_machine.run().await
}
