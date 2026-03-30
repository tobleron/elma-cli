# RUST MASTER PLAN
## 📚 LEGEND & DEFINITIONS
*   **LOC:** Total non-comment lines. (Lower is easier to read).
*   **Drag:** Estimated modification-risk multiplier. Higher Drag means edits are more likely to miss state, flow, or boundary details.
*   **Cognitive Capacity:** Inference energy required (Goal: < 100%).
*   **Read Tax:** Tokens and time overhead incurred when switching between many small files.
*   **AI Context Fog:** Regions of code with overlapping logic paths that cause model hallucination.

---

## 🛠️ SURGICAL REFACTOR TASKS (5)
- [ ] **../../src/tool_discovery.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.05, Coupling: 0.01] | Drag: 4.19 | LOC: 404/400  ⚠️ Trigger: Drag above target (2.60) with file already at 404 LOC.
- [ ] **../../src/execution_steps_shell_exec.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.06, Coupling: 0.00] | Drag: 4.28 | LOC: 413/400  ⚠️ Trigger: Drag above target (2.60) with file already at 413 LOC.
- [ ] **../../src/optimization_tune.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.06, Coupling: 0.00] | Drag: 4.26 | LOC: 484/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.
- [ ] **../../src/types_core.rs**
  - *Reason:* [Nesting: 1.20, Density: 0.01, Coupling: 0.00] | Drag: 2.26 | LOC: 620/400  ⚠️ Trigger: Oversized beyond the preferred 350-450 LOC working band.
- [ ] **../../src/orchestration_loop.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.05, Coupling: 0.01] | Drag: 4.27 | LOC: 533/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.

---

