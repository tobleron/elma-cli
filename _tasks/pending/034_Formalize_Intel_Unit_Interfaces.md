# Task 026: Formalize "Intel Unit" Interfaces

## Context
Intel units (reasoning functions) are currently disparate functions. Formalizing their interface will make them more composable and reliable.

## Objective
Create a trait or consistent internal structure for `IntelUnit`:
- Define standard `pre_flight` (context validation) and `post_flight` (result verification) steps.
- Standardize the way reasoning units handle errors and fallbacks.
- Update existing intel units in `src/intel.rs` (or its split counterparts) to follow this pattern.

## Success Criteria
- Improved reliability of reasoning unit calls.
- Consistent error handling across all "Intel" modules.
