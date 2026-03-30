# RUST MASTER PLAN
## 📚 LEGEND & DEFINITIONS
*   **LOC:** Total non-comment lines. (Lower is easier to read).
*   **Drag:** Estimated modification-risk multiplier. Higher Drag means edits are more likely to miss state, flow, or boundary details.
*   **Cognitive Capacity:** Inference energy required (Goal: < 100%).
*   **Read Tax:** Tokens and time overhead incurred when switching between many small files.
*   **AI Context Fog:** Regions of code with overlapping logic paths that cause model hallucination.

---

## 🛠️ SURGICAL REFACTOR TASKS (8)
- [ ] **../../src/defaults.rs**
  - *Reason:* [Nesting: 0.60, Density: 0.00, Coupling: 0.00] | Drag: 1.60 | LOC: 878/503  ⚠️ Trigger: Oversized beyond the preferred 350-450 LOC working band.
- [ ] **../../src/optimization.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.05, Coupling: 0.00] | Drag: 4.24 | LOC: 569/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.
- [ ] **../../src/routing.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.04, Coupling: 0.00] | Drag: 3.61 | LOC: 651/400  ⚠️ Trigger: Oversized beyond the preferred 350-450 LOC working band.
- [ ] **../../src/session.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.04, Coupling: 0.00] | Drag: 4.10 | LOC: 619/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.
- [ ] **../../src/ui.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.06, Coupling: 0.00] | Drag: 4.25 | LOC: 948/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.
- [ ] **../../src/types.rs**
  - *Reason:* [Nesting: 1.20, Density: 0.01, Coupling: 0.00] | Drag: 2.23 | LOC: 1363/400  ⚠️ Trigger: Oversized beyond the preferred 350-450 LOC working band.
- [ ] **../../src/orchestration.rs**
  - *Reason:* [Nesting: 3.00, Density: 0.03, Coupling: 0.00] | Drag: 4.17 | LOC: 1426/400  ⚠️ Trigger: Drag above target (2.60); keep the module within the 350-450 LOC working band if you extract helpers.
- [ ] **../../src/program.rs**
  - *Reason:* [Nesting: 2.40, Density: 0.09, Coupling: 0.00] | Drag: 3.65 | LOC: 676/400  ⚠️ Trigger: Oversized beyond the preferred 350-450 LOC working band.

---

