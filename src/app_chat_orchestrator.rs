//! @efficiency-role: service-orchestrator
//! App Chat - Program Orchestration and Resolution

use crate::app::*;

use crate::app_chat_handlers::*;
use crate::app_chat_helpers::*;
use crate::*;

pub(crate) async fn build_program(
    runtime: &mut AppRuntime,
    line: &str,
    complexity_level: &str,
    tui: &mut crate::ui_terminal::TerminalUI,
) -> Program {
    build_program_with_temp(
        runtime,
        line,
        complexity_level,
        runtime.config.profiles.orchestrator_cfg.temperature,
        tui,
    )
    .await
}

pub(crate) async fn build_program_with_temp(
    runtime: &mut AppRuntime,
    line: &str,
    complexity_level: &str,
    _temperature: f64,
    tui: &mut crate::ui_terminal::TerminalUI,
) -> Program {
    // Tool-calling pipeline: model plans and executes tools directly (no Maestro)
    let context_hint = "SHELL";
    match crate::orchestration_core::run_tool_calling_pipeline(
        runtime,
        line,
        tui,
        context_hint,
        false, // evidence_required (always false in tool-calling-first routing)
        complexity_level,
    )
    .await
    {
        Ok(pipeline_result) => {
            trace(
                &runtime.args,
                &format!(
                    "tool_calling_pipeline: answer_len={} iterations={} tool_calls={} stopped={}",
                    pipeline_result.final_answer.len(),
                    pipeline_result.iterations,
                    pipeline_result.tool_calls_made,
                    pipeline_result.stopped_by_max,
                ),
            );
            // Task 767: Store direct loop summary for trace/summary use
            let ls = &pipeline_result.loop_summary;
            trace(
                &runtime.args,
                &format!(
                    "tool_calling_pipeline: loop_summary tools={} reads={} searches={} fails={} dups={} stop={}",
                    ls.tool_calls_made,
                    ls.successful_reads.len(),
                    ls.successful_searches.len(),
                    ls.failed_operations.len(),
                    ls.duplicate_suppressions,
                    ls.stop_reason,
                ),
            );
            // Return as a single Respond step for the execution framework
            Program {
                objective: line.to_string(),
                steps: vec![Step::Respond {
                    id: "r1".to_string(),
                    instructions: pipeline_result.final_answer,
                    common: StepCommon {
                        purpose: "respond to user".to_string(),
                        depends_on: Vec::new(),
                        success_condition: "user receives answer".to_string(),
                        parent_id: None,
                        depth: None,
                        unit_type: None,
                        is_read_only: true,
                        is_destructive: false,
                        is_concurrency_safe: true,
                        interrupt_behavior: InterruptBehavior::Graceful,
                    },
                }],
            }
        }
        Err(e) => {
            trace(
                &runtime.args,
                &format!("tool_calling_pipeline_failed error={}", e),
            );
            // Fallback: direct reply
            build_direct_reply_program(line)
        }
    }
}

fn build_direct_reply_program(line: &str) -> Program {
    Program {
        objective: line.to_string(),
        steps: vec![Step::Respond {
            id: "r1".to_string(),
            instructions: "I encountered an error and could not process your request.".to_string(),
            common: StepCommon {
                purpose: "direct grounded reply".to_string(),
                depends_on: Vec::new(),
                success_condition: "the user receives a direct truthful answer".to_string(),
                parent_id: None,
                depth: None,
                unit_type: None,
                is_read_only: true,
                is_destructive: false,
                is_concurrency_safe: true,
                interrupt_behavior: InterruptBehavior::Graceful,
            },
        }],
    }
}
