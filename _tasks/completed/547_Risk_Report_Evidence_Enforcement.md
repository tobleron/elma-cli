# Task 547: Assumption vs Verified Fact Enforcement in Risk Reports

**Status:** pending
**Priority:** LOW
**Source:** Session analysis s_1777805162_306413000 (2026-05-03)
**Problem:** P13 — Medium Confidence

## Summary

In the session's final risk report, Risk 5 ("Session Persistence Completeness Uncertainty") cited `project_tmp/SESSION_PERSISTENCE_COMPLETION_REPORT.md` as evidence, but the model admitted in the next-step checklist that the file content was never read. The risk was labeled "Medium — data integrity risk" based purely on the fact that the file exists. An existing file is not evidence of a problem.

The session prompt explicitly asked to "distinguish confirmed facts from assumptions." The model produced a Confirmed/Assumptions section, but Risk 5 violated this contract — it was presented in the risk list as a verified risk while its own evidence was an assumption.

## Evidence

- `session.md` lines 162-166: Risk 5 cited file existence as evidence, claimed "content unknown"
- `session.md` lines 198-201: Assumptions section correctly noted this uncertainty
- But the risk list presented Risk 5 with equal weight to verified risks (1-4)
- The risk number and severity label gave it apparent parity with actually verified findings

## Root Cause

The analysis intel unit / prompt does not enforce that every risk in the numbered list must have a verified evidence link. A risk can be added to the list with "content unknown" evidence and receive a severity label without triggering a warning. The prompt says "verify whether each risk is real" but doesn't block unverified risks from appearing as numbered items.

## Implementation Plan

1. Update the risk report output schema or prompt to require an `evidence_status` field per risk:
   - `verified` — file read, search confirmed, or command output confirmed
   - `inferred` — logical deduction from confirmed facts, but not directly observed
   - `assumed` — cited without reading the source
2. Risks with `evidence_status=assumed` must be separated into a distinct "Unverified Signals" section, not numbered alongside verified risks
3. The numbered risk list must only contain `verified` or `inferred` risks
4. In the analysis prompt, add: _"Do not assign a severity label to any risk you have not directly verified by reading the relevant file or running a search. Unread files are not evidence."_

## Success Criteria

- [ ] Risk reports distinguish verified, inferred, and assumed findings
- [ ] Assumed findings do not receive severity labels or appear in the numbered risk list
- [ ] The model reads cited files before adding them as evidence
- [ ] Next-step checklist items do not include "verify risk X" if risk X was already presented as numbered

## Verification

```bash
# Run an audit session
# Review the final answer's risk list
# Verify all numbered risks have read/search confirmation in the tool trace
```
