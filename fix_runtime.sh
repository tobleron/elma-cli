#!/bin/bash
FILES="src/app_chat_handlers.rs src/app_chat_helpers.rs src/app_chat_loop.rs src/app_chat_orchestrator.rs src/app.rs src/config_cmd.rs src/llm_config.rs src/optimization_eval.rs src/orchestration_core.rs src/process_group.rs src/project_guidance.rs src/session_paths.rs"

for FILE in $FILES; do
  sed -i '' -E 's/runtime\.client/runtime.config.client/g' "$FILE"
  sed -i '' -E 's/runtime\.chat_url/runtime.config.chat_url/g' "$FILE"
  sed -i '' -E 's/runtime\.model_id/runtime.config.model_id/g' "$FILE"
  sed -i '' -E 's/runtime\.model_cfg_dir/runtime.config.model_cfg_dir/g' "$FILE"
  sed -i '' -E 's/runtime\.ctx_max/runtime.config.ctx_max/g' "$FILE"
  sed -i '' -E 's/runtime\.profiles/runtime.config.profiles/g' "$FILE"
  sed -i '' -E 's/runtime\.execution_profile/runtime.config.execution_profile/g' "$FILE"
  
  sed -i '' -E 's/runtime\.session/runtime.state.session/g' "$FILE"
  sed -i '' -E 's/runtime\.messages/runtime.state.messages/g' "$FILE"
  sed -i '' -E 's/runtime\.goal_state/runtime.state.goal_state/g' "$FILE"
  sed -i '' -E 's/runtime\.execution_plan/runtime.state.execution_plan/g' "$FILE"
  sed -i '' -E 's/runtime\.active_runtime_task/runtime.state.active_runtime_task/g' "$FILE"
  sed -i '' -E 's/runtime\.last_stop_outcome/runtime.state.last_stop_outcome/g' "$FILE"
  sed -i '' -E 's/runtime\.last_evidence_summary/runtime.state.last_evidence_summary/g' "$FILE"
  sed -i '' -E 's/runtime\.turn_count/runtime.state.turn_count/g' "$FILE"
  sed -i '' -E 's/runtime\.retry_attempt/runtime.state.retry_attempt/g' "$FILE"
  
  sed -i '' -E 's/runtime\.repo/runtime.workspace.repo/g' "$FILE"
  sed -i '' -E 's/runtime\.ws_brief/runtime.workspace.ws_brief/g' "$FILE"
  sed -i '' -E 's/runtime\.ws([[:space:],\.;\)]|$)/runtime.workspace.ws\1/g' "$FILE"
  sed -i '' -E 's/runtime\.guidance/runtime.workspace.guidance/g' "$FILE"
  sed -i '' -E 's/runtime\.system_content/runtime.workspace.system_content/g' "$FILE"
  sed -i '' -E 's/runtime\.tool_registry/runtime.workspace.tool_registry/g' "$FILE"
  
  sed -i '' -E 's/runtime\.verbose/runtime.tui.verbose/g' "$FILE"
done
