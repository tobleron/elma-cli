# Stress Test S001: Basic Connectivity and UI Formatting

## 1. The Test (Prompt)
"Hello, identify your current configuration, model, and the number of active profiles you have loaded. Ensure your response is formatted in a clean table and uses UI colors effectively."

## 2. Debugging Result Understanding
- **Success Criteria**: The agent correctly fetches its own configuration from `app_bootstrap_profiles.rs` or the `Args` struct and presents it in a Markdown table.
- **Common Failure Modes**: 
    - Model hallucinations regarding its own identity.
    - Failure to render UI colors/formatting correctly.
    - Truncated output due to low `max_tokens` in the `CHAT` profile.

## 3. Bottleneck Detection
- **Latency**: High time-to-first-token on simple greetings.
- **Context Loss**: If the model forgets it is an orchestrator and provides a generic LLM response.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
