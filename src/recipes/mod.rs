//! @efficiency-role: data-model
//!
//! Recipe System - Versioned external workflow recipes.
//! Task 451: Recipe And Subrecipe Workflow System.
//!
//! Recipes define repeatable task patterns without Rust code changes.
//! They are principle-first, not example-heavy, and small-model-friendly.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Recipe Schema Version
pub const RECIPE_SCHEMA_VERSION: &str = "1.0";

/// Recipe Definition - External repeatable workflow pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    /// Recipe identifier (e.g., "code_review", "bug_hunt")
    pub id: String,

    /// Schema version for compatibility
    pub version: String,

    /// Human-readable name
    pub name: String,

    /// Brief description
    pub description: String,

    /// What this recipe achieves (principle, not command list)
    pub objective: String,

    /// What must be true before this recipe runs
    pub preconditions: Vec<String>,

    /// Recipe stages (investigate, implement, verify, finalize)
    pub stages: Vec<RecipeStage>,

    /// Expected outputs produced
    pub outputs: Vec<String>,

    /// Verification steps after completion
    pub verification: Vec<String>,
}

/// Single stage in a recipe workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStage {
    /// Stage identifier (e.g., "investigate", "impl", "verify", "finalize")
    pub id: String,

    /// Stage name
    pub name: String,

    /// What this stage does (principle)
    pub objective: String,

    /// Tools allowed in this stage
    pub tools: Vec<String>,

    /// What this stage produces
    pub outputs: Vec<String>,

    /// How to verify stage completion
    pub verification: Vec<String>,
}

impl Recipe {
    /// Get all built-in recipes
    pub fn all() -> Vec<Recipe> {
        vec![]
    }

    /// Find recipe by ID
pub fn by_id(id: &str) -> Option<Recipe> {
        Self::all().into_iter().find(|r| r.id == id)
    }
}

mod loader;
pub use loader::{RecipeError, RecipeLoader};

#[cfg(test)]
mod tests;

/// Map formula name to recipe ID (if recipe exists)
/// Returns None if no matching recipe, Some(recipe_id) otherwise
pub fn formula_to_recipe_id(formula_name: &str) -> Option<String> {
    match formula_name {
        "inspect_edit_verify_reply" => Some("code_edit".to_string()),
        "inspect_summarize_reply" => Some("project_summary".to_string()),
        "inspect_decide_reply" => Some("code_review".to_string()),
        "plan_reply" => Some("implementation_plan".to_string()),
        "masterplan_reply" => Some("strategic_plan".to_string()),
        _ => None,
    }
}