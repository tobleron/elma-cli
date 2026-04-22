# Stress Testing Sandbox Contract

This directory is a writable stress-test sandbox for Elma.

Rules:
- All stress prompts under [`_stress_testing/`](/Users/r2/elma-cli/_stress_testing) must explicitly keep reads, writes, searches, planning, and execution inside `_stress_testing/`.
- Preferred writable targets are [`_stress_testing/_opencode_for_testing/`](/Users/r2/elma-cli/_stress_testing/_opencode_for_testing) and [`_stress_testing/_claude_code_src/`](/Users/r2/elma-cli/_stress_testing/_claude_code_src).
- Stress prompts must not instruct Elma to inspect or modify the production codebase under `src/`, `config/`, `sessions/`, or other live repo paths.
- The runners [`run_stress_tests.sh`](/Users/r2/elma-cli/run_stress_tests.sh) and [`run_stress_tests_cli.sh`](/Users/r2/elma-cli/run_stress_tests_cli.sh) now reject prompts that do not explicitly anchor themselves to `_stress_testing/`.

Purpose:
- exercise autonomy incrementally
- verify prompt stability without risking Elma's own runtime
- keep stress failures reproducible and safe

Current active test set:
- [`STRESS_MINIMAL_REPO_SUMMARY.md`](/Users/r2/elma-cli/_stress_testing/STRESS_MINIMAL_REPO_SUMMARY.md)

Obsoleted stress scenario prompts are kept in:
- [`obsolete/`](/Users/r2/elma-cli/_stress_testing/obsolete)
