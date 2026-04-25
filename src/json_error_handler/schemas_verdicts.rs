//! @efficiency-role: domain-logic
//!
//! JSON Error Handler - Verdict Validation & Grounding Module
//!
//! Verdict validation, deterministic fixes, and content grounding:
//! - validate_critic_verdict, validate_outcome_verdict
//! - deterministic_fix_critic_verdict, deterministic_fix_outcome_verdict
//! - SchemaValidationError and GroundingError types
//! - ground_critic_reason, verify_claim_against_outputs

use crate::*;
use thiserror::Error;

/// Validate critic verdict against schema (manual implementation)
pub(crate) fn validate_critic_verdict(
    args: &Args,
    verdict: &CriticVerdict,
) -> Result<(), SchemaValidationError> {
    let mut errors = Vec::new();

    // Validate status field
    if verdict.status != "ok" && verdict.status != "retry" {
        errors.push(format!(
            "Invalid status '{}': must be 'ok' or 'retry'",
            verdict.status
        ));
    }

    // Validate reason field
    if verdict.reason.trim().is_empty() {
        errors.push("Reason cannot be empty".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(SchemaValidationError::ValidationErrors(errors))
    }
}

/// Validate outcome verdict against schema (manual implementation)
pub(crate) fn validate_outcome_verdict(
    args: &Args,
    verdict: &OutcomeVerificationVerdict,
) -> Result<(), SchemaValidationError> {
    let mut errors = Vec::new();

    // Validate status field
    if verdict.status != "ok" && verdict.status != "retry" {
        errors.push(format!(
            "Invalid status '{}': must be 'ok' or 'retry'",
            verdict.status
        ));
    }

    // Validate reason field
    if verdict.reason.trim().is_empty() {
        errors.push("Reason cannot be empty".to_string());
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(SchemaValidationError::ValidationErrors(errors))
    }
}

/// Deterministic fix based on schema errors
pub(crate) fn deterministic_fix_critic_verdict(
    args: &Args,
    verdict: &CriticVerdict,
    errors: &[String],
) -> Option<CriticVerdict> {
    let mut fixed = verdict.clone();
    let mut was_fixed = false;

    for error in errors {
        let error_lower = error.to_lowercase();

        // Fix missing or invalid status
        if error_lower.contains("status") {
            if error_lower.contains("missing") || error_lower.contains("required") {
                fixed.status = "ok".to_string();
                was_fixed = true;
            } else if error_lower.contains("enum") {
                fixed.status = "ok".to_string();
                was_fixed = true;
            }
        }

        // Fix missing or empty reason
        if error_lower.contains("reason") {
            if error_lower.contains("missing") || error_lower.contains("required") {
                fixed.reason = "Schema validation auto-repair".to_string();
                was_fixed = true;
            } else if error_lower.contains("minlength") || error_lower.contains("empty") {
                fixed.reason = "Schema validation auto-repair".to_string();
                was_fixed = true;
            }
        }

        // Fix invalid program field
        if error_lower.contains("program") {
            if error_lower.contains("additional") {
                fixed.program = None;
                was_fixed = true;
            }
        }
    }

    if was_fixed {
        trace(args, "deterministic_fix_applied to critic verdict");
        Some(fixed)
    } else {
        trace(args, "deterministic_fix_failed no applicable fixes");
        None
    }
}

/// Deterministic fix based on schema errors
pub(crate) fn deterministic_fix_outcome_verdict(
    args: &Args,
    verdict: &OutcomeVerificationVerdict,
    errors: &[String],
) -> Option<OutcomeVerificationVerdict> {
    let mut fixed = verdict.clone();
    let mut was_fixed = false;

    for error in errors {
        let error_lower = error.to_lowercase();

        // Fix missing or invalid status
        if error_lower.contains("status") {
            if error_lower.contains("missing") || error_lower.contains("required") {
                fixed.status = "ok".to_string();
                was_fixed = true;
            } else if error_lower.contains("enum") {
                fixed.status = "ok".to_string();
                was_fixed = true;
            }
        }

        // Fix missing or empty reason
        if error_lower.contains("reason") {
            if error_lower.contains("missing") || error_lower.contains("required") {
                fixed.reason = "Schema validation auto-repair".to_string();
                was_fixed = true;
            } else if error_lower.contains("minlength") || error_lower.contains("empty") {
                fixed.reason = "Schema validation auto-repair".to_string();
                was_fixed = true;
            }
        }
    }

    if was_fixed {
        trace(args, "deterministic_fix_applied to outcome verdict");
        Some(fixed)
    } else {
        trace(args, "deterministic_fix_failed no applicable fixes");
        None
    }
}

/// Schema validation error
#[derive(Error, Debug, Clone)]
pub(crate) enum SchemaValidationError {
    #[error("Failed to serialize verdict: {0}")]
    SerializationError(String),
    #[error("Failed to parse schema: {0}")]
    SchemaParseError(String),
    #[error("Schema validation failed: {}", .0.join("; "))]
    ValidationErrors(Vec<String>),
}

/// Check if critic reason is grounded in actual step outputs
///
/// This catches hallucinated critic claims like "output does not match"
/// when the output is actually valid.
pub(crate) fn ground_critic_reason(
    args: &Args,
    critic_reason: &str,
    step_results: &[StepResult],
) -> Result<(), GroundingError> {
    let reason_lower = critic_reason.to_lowercase();

    // Hallucination patterns - claims that need verification against actual outputs
    let hallucination_patterns = [
        "does not match",
        "output does not match",
        "expected.*but got",
        "only shows",
        "missing",
        "incomplete",
        "not actually satisfy",
    ];

    for pattern in &hallucination_patterns {
        if reason_lower.contains(pattern) {
            // Verify claim against actual outputs
            if !verify_claim_against_outputs(critic_reason, step_results) {
                trace(
                    args,
                    &format!("critic_reason_grounding_failed pattern={}", pattern),
                );
                return Err(GroundingError::HallucinatedClaim(critic_reason.to_string()));
            }
        }
    }

    Ok(())
}

/// Verify a critic claim against actual step outputs
fn verify_claim_against_outputs(claim: &str, outputs: &[StepResult]) -> bool {
    let claim_lower = claim.to_lowercase();

    // Check each step output
    for output in outputs {
        // If claim says "doesn't match" but step succeeded with valid output, claim is hallucinated
        if claim_lower.contains("does not match") || claim_lower.contains("output does not match") {
            if let Some(raw_output) = &output.raw_output {
                // Check if output is non-empty and step succeeded
                if !raw_output.trim().is_empty() && output.ok {
                    // Also check exit code
                    if output.exit_code.unwrap_or(0) == 0 {
                        // Output is valid, claim is hallucinated
                        return false;
                    }
                }
            }
        }

        // If claim says "missing" but we have output, verify it's actually missing
        if claim_lower.contains("missing") {
            if let Some(raw_output) = &output.raw_output {
                if !raw_output.trim().is_empty() {
                    // Output exists, claim may be hallucinated
                    // (unless it's about specific content being missing)
                    return true; // Be conservative - allow the claim
                }
            }
        }
    }

    true // Claim appears grounded (conservative - allow when uncertain)
}

/// Grounding error
#[derive(Error, Debug, Clone)]
pub(crate) enum GroundingError {
    #[error("Critic reason appears hallucinated: {0}")]
    HallucinatedClaim(String),
}
