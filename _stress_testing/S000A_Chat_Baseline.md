# Stress Test S000A: Pure Conversational Baseline

## 1. The Test (Prompt)
"Hello Elma. Briefly explain your primary goal as a CLI agent."

## 2. Debugging Result Understanding
- **Success Criteria**: The agent identifies the `CHAT` route. It provides a concise explanation of its orchestration role without attempting to run shell commands or search files.
- **Common Failure Modes**:
    - Over-orchestration: Attempting to "search" for its own definition.
    - Hallucination: Claiming to be a different agent (like ChatGPT or Claude).

## 3. Bottleneck Detection
- **Routing Latency**: If the `routing` logic takes too long for a simple greeting.
- **Tone Consistency**: If the model ignores the `system_prompt` instructions for brevity.

## 4. Resolution & Iteration
- (Iterative refinement to be recorded here during execution)
