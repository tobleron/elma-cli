# RUST MASTER PLAN
## 📚 LEGEND & DEFINITIONS
*   **LOC:** Total non-comment lines. (Lower is easier to read).
*   **Drag:** Estimated modification-risk multiplier. Higher Drag means edits are more likely to miss state, flow, or boundary details.
*   **Cognitive Capacity:** Inference energy required (Goal: < 100%).
*   **Read Tax:** Tokens and time overhead incurred when switching between many small files.
*   **AI Context Fog:** Regions of code with overlapping logic paths that cause model hallucination.

---

## 🛠️ SURGICAL REFACTOR TASKS (20)
- [ ] **../../src/execution_ladder.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.04, Coupling: 0.01] | Drag: 3.58 | LOC: 784/450  ⚠️ Trigger: Oversized beyond the preferred 390-490 LOC working band.
- [ ] **../../src/intel_trait.rs**
  - *Reason:* [Nesting: 1.80, Density: 0.01, Coupling: 0.00] | Drag: 2.87 | LOC: 691/450  ⚠️ Trigger: Oversized beyond the preferred 390-490 LOC working band.
- [ ] **../../src/types_core.rs**
  - *Reason:* [Nesting: 1.20, Density: 0.01, Coupling: 0.00] | Drag: 2.24 | LOC: 862/577  ⚠️ Trigger: Oversized beyond the preferred 550-650 LOC working band.
- [ ] **../../src/optimization_tune.rs**
  - *Reason:* [Nesting: 3.60, Density: 0.06, Coupling: 0.00] | Drag: 4.88 | LOC: 646/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 510-610 LOC working band if you extract helpers.
- [ ] **../../src/guardrails.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.03, Coupling: 0.00] | Drag: 3.43 | LOC: 632/450  ⚠️ Trigger: Oversized beyond the preferred 390-490 LOC working band.
- [ ] **../../src/app_chat_core.rs**
  - *Reason:* [Nesting: 3.60, Density: 0.04, Coupling: 0.00] | Drag: 4.69 | LOC: 3168/450  ⚠️ Trigger: Oversized beyond the preferred 470-570 LOC working band.
- [ ] **../../src/execution_steps.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.03, Coupling: 0.01] | Drag: 3.56 | LOC: 705/450  ⚠️ Trigger: Oversized beyond the preferred 510-610 LOC working band.
- [ ] **../../src/intel_narrative.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.01, Coupling: 0.00] | Drag: 3.41 | LOC: 1086/450  ⚠️ Trigger: Oversized beyond the preferred 510-610 LOC working band.
- [ ] **../../src/json_parser.rs**
  - *Reason:* [Nesting: 3.60, Density: 0.07, Coupling: 0.01] | Drag: 4.78 | LOC: 799/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 310-410 LOC working band if you extract helpers.
- [ ] **../../src/orchestration_planning.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.04, Coupling: 0.01] | Drag: 3.50 | LOC: 740/450  ⚠️ Trigger: Oversized beyond the preferred 510-610 LOC working band.
- [ ] **../../src/routing_parse.rs**
  - *Reason:* [Nesting: 7.20, Density: 0.09, Coupling: 0.00] | Drag: 8.49 | LOC: 476/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 310-410 LOC working band if you extract helpers.
- [ ] **../../src/ui_chat.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.05, Coupling: 0.01] | Drag: 4.19 | LOC: 595/450  ⚠️ Trigger: Drag above target (2.60) with file already at 595 LOC.
- [ ] **../../src/orchestration_helpers.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.02, Coupling: 0.00] | Drag: 3.44 | LOC: 844/450  ⚠️ Trigger: Oversized beyond the preferred 310-410 LOC working band.
- [ ] **../../src/json_error_handler.rs**
  - *Reason:* [Nesting: 3.60, Density: 0.05, Coupling: 0.00] | Drag: 4.85 | LOC: 1119/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 510-610 LOC working band if you extract helpers.
- [ ] **../../src/orchestration_loop.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.04, Coupling: 0.01] | Drag: 4.25 | LOC: 723/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 510-610 LOC working band if you extract helpers.
- [ ] **../../src/intel_units.rs**
  - *Reason:* [Nesting: 1.20, Density: 0.02, Coupling: 0.00] | Drag: 2.22 | LOC: 2167/450  ⚠️ Trigger: Oversized beyond the preferred 390-490 LOC working band.
- [ ] **../../src/verification.rs**
  - *Reason:* [Nesting: 4.20, Density: 0.03, Coupling: 0.00] | Drag: 5.41 | LOC: 776/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 390-490 LOC working band if you extract helpers.
- [ ] **../../src/program_policy.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.05, Coupling: 0.00] | Drag: 3.55 | LOC: 1104/450  ⚠️ Trigger: Oversized beyond the preferred 390-490 LOC working band.
- [ ] **../../src/json_tuning.rs**
  - *Reason:* [Nesting: 4.20, Density: 0.07, Coupling: 0.00] | Drag: 5.48 | LOC: 647/450  ⚠️ Trigger: Drag above target (2.60); keep the module within the preferred 510-610 LOC working band if you extract helpers.
- [ ] **../../src/defaults_evidence.rs**
  - *Reason:* [Nesting: 4.20, Density: 0.02, Coupling: 0.00] | Drag: 5.24 | LOC: 851/450  ⚠️ Trigger: Oversized beyond the preferred 550-650 LOC working band.

---

