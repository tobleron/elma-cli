# RUST MASTER PLAN
## 📚 LEGEND & DEFINITIONS
*   **LOC:** Total non-comment lines. (Lower is easier to read).
*   **Drag:** Estimated modification-risk multiplier. Higher Drag means edits are more likely to miss state, flow, or boundary details.
*   **Cognitive Capacity:** Inference energy required (Goal: < 100%).
*   **Read Tax:** Tokens and time overhead incurred when switching between many small files.
*   **AI Context Fog:** Regions of code with overlapping logic paths that cause model hallucination.

---

## 🛠️ SURGICAL REFACTOR TASKS (11)
- [ ] **../../src/json_tuning.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.06, Coupling: 0.00] | Drag: 3.64 | LOC: 622/450  ⚠️ Trigger: Oversized beyond the preferred 510-610 LOC working band.
- [ ] **../../src/orchestration_loop.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.03, Coupling: 0.00] | Drag: 3.56 | LOC: 1036/450  ⚠️ Trigger: Oversized beyond the preferred 510-610 LOC working band.
- [ ] **../../src/routing_parse.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.09, Coupling: 0.00] | Drag: 4.30 | LOC: 464/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 310-410 LOC working band if you extract helpers.
- [ ] **../../src/app_chat_builders_advanced.rs**
  - *Reason:* [Nesting: 1.80, Density: 0.02, Coupling: 0.01] | Drag: 2.82 | LOC: 576/450  ⚠️ Trigger: Oversized beyond the preferred 470-570 LOC working band.
- [ ] **../../src/app_chat_loop.rs**
  - *Reason:* [Nesting: 3.60, Density: 0.07, Coupling: 0.01] | Drag: 4.83 | LOC: 693/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 510-610 LOC working band if you extract helpers.
- [ ] **../../src/orchestration_planning.rs**
  - *Reason:* [Nesting: 1.80, Density: 0.04, Coupling: 0.01] | Drag: 2.92 | LOC: 644/450  ⚠️ Trigger: Oversized beyond the preferred 510-610 LOC working band.
- [ ] **../../src/intel_narrative.rs**
  - *Reason:* [Nesting: 1.20, Density: 0.00, Coupling: 0.01] | Drag: 2.20 | LOC: 692/450  ⚠️ Trigger: Oversized beyond the preferred 310-410 LOC working band.
- [ ] **../../src/optimization_tune.rs**
  - *Reason:* [Nesting: 3.60, Density: 0.05, Coupling: 0.00] | Drag: 4.75 | LOC: 603/450  ⚠️ Trigger: Drag above target (2.60) with file already at 603 LOC.
- [ ] **../../src/verification.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.04, Coupling: 0.00] | Drag: 4.28 | LOC: 500/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 390-490 LOC working band if you extract helpers.
- [ ] **../../src/execution_steps.rs**
  - *Reason:* [Nesting: 1.80, Density: 0.02, Coupling: 0.01] | Drag: 2.94 | LOC: 812/450  ⚠️ Trigger: Oversized beyond the preferred 470-570 LOC working band.
- [ ] **../../src/intel_trait.rs**
  - *Reason:* [Nesting: 1.80, Density: 0.01, Coupling: 0.00] | Drag: 2.87 | LOC: 627/473  ⚠️ Trigger: Oversized beyond the preferred 550-650 LOC working band.

---

