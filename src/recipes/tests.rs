#[cfg(test)]
mod tests {
    use crate::recipes::{Recipe, RecipeLoader, RECIPE_SCHEMA_VERSION};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn create_test_recipes_dir() -> (TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().to_path_buf();
        (dir, path)
    }

    #[test]
    fn test_recipe_schema_version_constant() {
        assert_eq!(RECIPE_SCHEMA_VERSION, "1.0");
    }

    #[test]
    fn test_recipe_by_id_returns_none_for_empty_registry() {
        let result = Recipe::by_id("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_recipe_loader_not_found() {
        let (_dir, path) = create_test_recipes_dir();
        let loader = RecipeLoader::new(path);
        let result = loader.load("code_edit");
        assert!(result.is_err());
    }

    #[test]
    fn test_recipe_loader_list_empty() {
        let (_dir, path) = create_test_recipes_dir();
        let loader = RecipeLoader::new(path);
        let ids = loader.list_recipes();
        assert!(ids.is_empty());
    }

    #[test]
    fn test_formula_to_recipe_id_mapping() {
        use crate::recipes::formula_to_recipe_id;

        assert_eq!(
            formula_to_recipe_id("inspect_edit_verify_reply"),
            Some("code_edit".to_string())
        );
        assert_eq!(
            formula_to_recipe_id("inspect_summarize_reply"),
            Some("project_summary".to_string())
        );
        assert_eq!(
            formula_to_recipe_id("reply_only"),
            None
        );
    }
}