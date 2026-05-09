//! @efficiency-role: util-pure
//!
//! App Chat - Trace Functions

use crate::*;


pub(crate) fn trace_complexity(args: &Args, complexity: &ComplexityAssessment) {
    trace(
        args,
        &format!(
            "complexity={} pattern={} risk={}",
            if complexity.complexity.is_empty() {
                "UNKNOWN"
            } else {
                &complexity.complexity
            },
            if complexity.suggested_pattern.is_empty() {
                "unknown"
            } else {
                &complexity.suggested_pattern
            },
            if complexity.risk.is_empty() {
                "UNKNOWN"
            } else {
                &complexity.risk
            }
        ),
    );
}

pub(crate) fn trace_scope(args: &Args, scope: &ScopePlan) {
    let trivial_root_only = !scope.focus_paths.is_empty()
        && scope.focus_paths.iter().all(|path| {
            let path = path.trim();
            path.is_empty() || path == "." || path == "./"
        });
    if !scope.focus_paths.is_empty() && !trivial_root_only {
        operator_trace(
            args,
            &format!(
                "narrowing the scope{}",
                if scope.focus_paths.is_empty() {
                    String::new()
                } else {
                    format!(" to {}", scope.focus_paths.join(", "))
                }
            ),
        );
    }
    trace(
        args,
        &format!(
            "scope focus={} include={} exclude={} query={} reason={}",
            if scope.focus_paths.is_empty() {
                "-".to_string()
            } else {
                scope.focus_paths.join(",")
            },
            if scope.include_globs.is_empty() {
                "-".to_string()
            } else {
                scope.include_globs.join(",")
            },
            if scope.exclude_globs.is_empty() {
                "-".to_string()
            } else {
                scope.exclude_globs.join(",")
            },
            if scope.query_terms.is_empty() {
                "-".to_string()
            } else {
                scope.query_terms.join(",")
            },
            scope.reason
        ),
    );
}

pub(crate) fn trace_formula(args: &Args, formula: &FormulaSelection) {
    trace(
        args,
        &format!(
            "formula={} alt={} reason={}",
            if formula.primary.is_empty() {
                "unknown"
            } else {
                &formula.primary
            },
            if formula.alternatives.is_empty() {
                "-".to_string()
            } else {
                formula.alternatives.join(",")
            },
            if formula.memory_id.trim().is_empty() {
                formula.reason.clone()
            } else {
                format!("{} memory={}", formula.reason, formula.memory_id)
            }
        ),
    );
}
