//! @efficiency-role: orchestrator
//!
//! Recipe Loader - Load and validate external recipe files.
//! Task 451: Recipe And Subrecipe Workflow System.

use crate::recipes::Recipe;
use crate::recipes::RECIPE_SCHEMA_VERSION;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecipeError {
    #[error("Recipe not found: {0}")]
    NotFound(String),

    #[error("Invalid recipe schema version: expected {expected}, got {got}")]
    InvalidVersion { expected: String, got: String },

    #[error("Recipe validation failed: {0}")]
    ValidationFailed(String),

    #[error("Failed to read recipe file: {0}")]
    ReadError(String),
}

/// Recipe Loader - Loads recipes from files with validation
pub struct RecipeLoader {
    /// Directory containing recipe files
    recipes_dir: PathBuf,
}

impl RecipeLoader {
    /// Create a new recipe loader
    pub fn new(recipes_dir: PathBuf) -> Self {
        Self { recipes_dir }
    }

    /// Load a recipe by ID (e.g., "code_review" -> recipes/code_review.toml)
    pub fn load(&self, id: &str) -> Result<Recipe, RecipeError> {
        let path = self.recipes_dir.join(format!("{}.toml", id));
        self.load_from_path(&path)
    }

    /// Load a recipe from a specific path
    pub fn load_from_path(&self, path: &PathBuf) -> Result<Recipe, RecipeError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| RecipeError::ReadError(e.to_string()))?;

        let recipe: Recipe = toml::from_str(&content)
            .map_err(|e| RecipeError::ValidationFailed(e.to_string()))?;

        self.validate(&recipe)?;
        Ok(recipe)
    }

    /// Validate recipe schema and structure
    fn validate(&self, recipe: &Recipe) -> Result<(), RecipeError> {
        if recipe.version != RECIPE_SCHEMA_VERSION {
            return Err(RecipeError::InvalidVersion {
                expected: RECIPE_SCHEMA_VERSION.to_string(),
                got: recipe.version.to_string(),
            });
        }

        if recipe.id.is_empty() {
            return Err(RecipeError::ValidationFailed(
                "Recipe ID cannot be empty".to_string(),
            ));
        }

        if recipe.stages.is_empty() {
            return Err(RecipeError::ValidationFailed(
                "Recipe must have at least one stage".to_string(),
            ));
        }

        for stage in &recipe.stages {
            if stage.id.is_empty() {
                return Err(RecipeError::ValidationFailed(
                    "Stage ID cannot be empty".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// List all available recipe IDs in the recipes directory
    pub fn list_recipes(&self) -> Vec<String> {
        let mut ids = vec![];
        if let Ok(entries) = std::fs::read_dir(&self.recipes_dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.path().file_stem() {
                    if let Some(s) = name.to_str() {
                        ids.push(s.to_string());
                    }
                }
            }
        }
        ids.sort();
        ids
    }
}