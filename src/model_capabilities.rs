//! @efficiency-role: infra-config
//!
//! Model Capability Registry (Task 343)
//!
//! Provides model capability metadata and tokenizer adapters for exact token counting.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ModelCapabilities {
    pub model_name: String,
    pub context_window: u32,
    pub max_output_tokens: u32,
    pub supports_reasoning: bool,
    pub reasoning_format: Option<String>,
    pub reasoning_capability: ReasoningCapability,
    pub tokenizer: TokenizerType,
}

/// Distinguishes how a model handles reasoning/thinking output.
#[derive(Debug, Clone, PartialEq)]
pub enum ReasoningCapability {
    /// Dense non-thinking model — no separated reasoning block (e.g., Llama 3.2, Phi).
    Dense,
    /// Model that produces separated reasoning/thinking when `reasoning_format=auto` (e.g., Qwen, DeepSeek).
    Separated,
    /// Model capable of both dense and separated reasoning (future).
    Mixed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenizerType {
    Gpt2,
    Llama,
    DeepSeek,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct ModelCapabilityRegistry {
    models: HashMap<String, ModelCapabilities>,
}

impl Default for ModelCapabilityRegistry {
    fn default() -> Self {
        let mut registry = Self {
            models: HashMap::new(),
        };
        registry.seed_builtin_models();
        registry
    }
}

impl ModelCapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    fn seed_builtin_models(&mut self) {
        self.register_model(ModelCapabilities {
            model_name: "llama-cpp".to_string(),
            context_window: 4096,
            max_output_tokens: 2048,
            supports_reasoning: false,
            reasoning_format: None,
            reasoning_capability: ReasoningCapability::Dense,
            tokenizer: TokenizerType::Llama,
        });

        self.register_model(ModelCapabilities {
            model_name: "qwen-code".to_string(),
            context_window: 32768,
            max_output_tokens: 4096,
            supports_reasoning: false,
            reasoning_format: None,
            reasoning_capability: ReasoningCapability::Dense,
            tokenizer: TokenizerType::Gpt2,
        });

        self.register_model(ModelCapabilities {
            model_name: "deepseek-coder".to_string(),
            context_window: 128000,
            max_output_tokens: 8192,
            supports_reasoning: false,
            reasoning_format: None,
            reasoning_capability: ReasoningCapability::Dense,
            tokenizer: TokenizerType::DeepSeek,
        });

        self.register_model(ModelCapabilities {
            model_name: "llama_3.2_3b_instruct_q6_k_l.gguf".to_string(),
            context_window: 8192,
            max_output_tokens: 4096,
            supports_reasoning: false,
            reasoning_format: Some("none".to_string()),
            reasoning_capability: ReasoningCapability::Dense,
            tokenizer: TokenizerType::Llama,
        });
    }

    pub fn register_model(&mut self, caps: ModelCapabilities) {
        self.models.insert(caps.model_name.clone(), caps);
    }

    pub fn get(&self, model_name: &str) -> Option<&ModelCapabilities> {
        for (name, caps) in &self.models {
            if model_name.contains(name) {
                return Some(caps);
            }
        }
        None
    }

    pub fn estimate_tokens(&self, text: &str, model_name: &str) -> usize {
        let caps = self.get(model_name);
        match caps {
            Some(c) => c.tokenizer.count(text),
            None => Self::fallback_token_estimate(text),
        }
    }

    fn fallback_token_estimate(text: &str) -> usize {
        (text.chars().count() as usize + 3) / 4
    }
}

impl TokenizerType {
    pub fn count(&self, text: &str) -> usize {
        match self {
            TokenizerType::Gpt2 => Self::gpt2_estimate(text),
            TokenizerType::Llama => Self::llama_estimate(text),
            TokenizerType::DeepSeek => Self::deepseek_estimate(text),
            TokenizerType::Custom(_) => Self::fallback_estimate(text),
        }
    }

    fn gpt2_estimate(text: &str) -> usize {
        use regex::Regex;
        let word = Regex::new(r"\S+").unwrap();
        let mut count = 0;
        for _ in word.find_iter(text) {
            count += 1;
        }
        let punct = Regex::new(r"[^\w\s]").unwrap();
        count + punct.find_iter(text).count()
    }

    fn llama_estimate(text: &str) -> usize {
        text.chars().count() / 4
    }

    fn deepseek_estimate(text: &str) -> usize {
        text.chars().count() / 4
    }

    fn fallback_estimate(text: &str) -> usize {
        text.chars().count() / 4
    }
}

pub fn estimate_token_count(text: &str, model_name: &str) -> usize {
    static REGISTRY: std::sync::OnceLock<ModelCapabilityRegistry> = std::sync::OnceLock::new();
    let registry = REGISTRY.get_or_init(ModelCapabilityRegistry::new);
    registry.estimate_tokens(text, model_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_models() {
        let registry = ModelCapabilityRegistry::new();
        assert!(registry.get("llama-cpp").is_some());
        assert!(registry.get("qwen-code").is_some());
    }

    #[test]
    fn test_fallback_estimate() {
        let text = "hello world";
        let count = ModelCapabilityRegistry::fallback_token_estimate(text);
        assert!(count > 0);
    }

    #[test]
    fn test_estimate_tokens_with_known_model() {
        let registry = ModelCapabilityRegistry::new();
        let text = "Hello, world! This is a test.";
        let count = registry.estimate_tokens(text, "llama-cpp");
        assert!(count > 0);
    }

    #[test]
    fn test_estimate_tokens_with_unknown_model() {
        let registry = ModelCapabilityRegistry::new();
        let text = "Hello, world!";
        let count = registry.estimate_tokens(text, "unknown-model");
        assert!(count > 0);
    }

    #[test]
    fn test_gpt2_tokenizer() {
        let tokenizer = TokenizerType::Gpt2;
        let text = "Hello world!";
        let count = tokenizer.count(text);
        assert!(count >= 2);
    }

    #[test]
    fn test_llama32_registered() {
        let registry = ModelCapabilityRegistry::new();
        let caps = registry.get("llama_3.2_3b_instruct_q6_k_l.gguf");
        assert!(caps.is_some(), "Llama 3.2 should be registered");
        let caps = caps.unwrap();
        assert_eq!(caps.reasoning_capability, ReasoningCapability::Dense);
        assert_eq!(caps.supports_reasoning, false);
        assert_eq!(caps.reasoning_format, Some("none".to_string()));
    }

    #[test]
    fn test_reasoning_capability_variants_distinct() {
        assert_ne!(ReasoningCapability::Dense, ReasoningCapability::Separated);
        assert_ne!(ReasoningCapability::Separated, ReasoningCapability::Mixed);
        assert_ne!(ReasoningCapability::Mixed, ReasoningCapability::Dense);
    }

    // ── Llama 3.2 Profile Certification ──

    const LLAMA32_MODEL: &str = "llama_3.2_3b_instruct_q6_k_l.gguf";
    const HUIHUI_MODEL: &str = "Huihui-Qwen3.5-4B-Claude-4.6-Opus-abliterated.Q6_K.gguf";

    /// Path to a model's config directory relative to CARGO_MANIFEST_DIR/config/.
    fn model_cfg_dir(model: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("config")
            .join(model)
    }

    #[test]
    fn test_llama32_profile_directory_exists() {
        let dir = model_cfg_dir(LLAMA32_MODEL);
        assert!(
            dir.exists(),
            "Llama 3.2 profile directory must exist at {}",
            dir.display()
        );

        let elma_cfg = dir.join("_elma.config");
        assert!(elma_cfg.exists(), "Llama 3.2 must have _elma.config");

        let model_behavior = dir.join("model_behavior.toml");
        assert!(
            model_behavior.exists(),
            "Llama 3.2 must have model_behavior.toml"
        );
    }

    #[test]
    fn test_llama32_model_behavior_is_dense_non_thinking() {
        let dir = model_cfg_dir(LLAMA32_MODEL);
        let path = dir.join("model_behavior.toml");
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Cannot read {}: {}", path.display(), e));

        assert!(
            content.contains("preferred_reasoning_format = \"none\""),
            "Llama 3.2 must prefer reasoning_format=none (dense non-thinking)\n{}",
            content
        );
        assert!(
            content.contains("auto_reasoning_separated = false"),
            "Llama 3.2 must have auto_reasoning_separated=false (no separated thinking)\n{}",
            content
        );
        assert!(
            content.contains("none_final_clean = true"),
            "Llama 3.2 must have none_final_clean=true\n{}",
            content
        );
    }

    #[test]
    fn test_llama32_intel_units_use_none_reasoning() {
        let dir = model_cfg_dir(LLAMA32_MODEL);
        let mut errors = Vec::new();

        // Check key intel unit configs that are used by the basic smoke pack
        let critical_units = [
            "router.toml",
            "speech_act.toml",
            "mode_router.toml",
            "complexity_assessor.toml",
            "evidence_need_assessor.toml",
            "action_need_assessor.toml",
        ];
        for unit in &critical_units {
            let path = dir.join(unit);
            if !path.exists() {
                errors.push(format!("{}: missing config file", unit));
                continue;
            }
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            if !content.contains("reasoning_format = \"none\"") {
                errors.push(format!("{}: missing reasoning_format = \"none\"", unit));
            }
        }

        // Check all remaining .toml files in the directory (not _elma.config or model_behavior)
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.ends_with(".toml")
                    || name == "_elma.config"
                    || name == "model_behavior.toml"
                    || name == "router_calibration.toml"
                {
                    continue;
                }
                let content = std::fs::read_to_string(&entry.path()).unwrap_or_default();
                if !content.contains("reasoning_format = \"none\"") {
                    errors.push(format!("{}: missing reasoning_format = \"none\"", name));
                }
            }
        }

        if !errors.is_empty() {
            panic!(
                "Llama 3.2 profile certification FAILED:\n  {}",
                errors.join("\n  ")
            );
        }
    }

    #[test]
    fn test_llama32_profile_comparison_output() {
        let llama_dir = model_cfg_dir(LLAMA32_MODEL);
        let huihui_dir = model_cfg_dir(HUIHUI_MODEL);

        assert!(
            llama_dir.exists(),
            "Llama 3.2 profile directory must exist for comparison"
        );
        assert!(
            huihui_dir.exists(),
            "Huihui profile directory must exist for comparison"
        );

        // Compare model_behavior.toml
        let llama_behavior =
            std::fs::read_to_string(llama_dir.join("model_behavior.toml")).unwrap_or_default();
        let huihui_behavior =
            std::fs::read_to_string(huihui_dir.join("model_behavior.toml")).unwrap_or_default();

        let llama_lines: Vec<&str> = llama_behavior.lines().collect();
        let huihui_lines: Vec<&str> = huihui_behavior.lines().collect();

        eprintln!("=== Llama 3.2 vs Huihui/Qwen Profile Comparison ===");
        eprintln!();
        eprintln!("--- model_behavior.toml ---");
        eprintln!("  Llama 3.2:   {} lines", llama_lines.len());
        eprintln!("  Huihui:      {} lines", huihui_lines.len());

        // Check key differentiating fields
        let extract_field = |content: &str, field: &str| -> String {
            content
                .lines()
                .find(|l| l.trim().starts_with(field))
                .map(|l| l.trim().to_string())
                .unwrap_or_else(|| format!("{} = <missing>", field))
        };
        let fields = [
            "preferred_reasoning_format",
            "auto_reasoning_separated",
            "none_final_clean",
            "needs_text_finalizer",
        ];
        for field in &fields {
            eprintln!("  {}", field);
            eprintln!("    Llama 3.2: {}", extract_field(&llama_behavior, field));
            eprintln!("    Huihui:    {}", extract_field(&huihui_behavior, field));
        }

        // Verify differentiation
        assert!(
            llama_behavior.contains("preferred_reasoning_format = \"none\""),
            "Llama 3.2 should use none reasoning format (dense profile)"
        );
        assert!(
            huihui_behavior.contains("preferred_reasoning_format = \"auto\""),
            "Huihui should use auto reasoning format (thinking profile)"
        );
        assert!(
            llama_behavior.contains("auto_reasoning_separated = false"),
            "Llama 3.2 should have no separated thinking"
        );
        assert!(
            huihui_behavior.contains("auto_reasoning_separated = true"),
            "Huihui should have separated thinking"
        );
    }

    #[test]
    fn test_llama32_grammar_mappings_exist() {
        let config_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config");
        let mapping_path = config_root.join("grammar_mapping.toml");
        let mapping_content = std::fs::read_to_string(&mapping_path)
            .unwrap_or_else(|e| panic!("Cannot read grammar_mapping.toml: {}", e));

        // Profiles with explicit grammar mappings in grammar_mapping.toml
        let mapped_profiles = [
            "router",
            "speech_act",
            "mode_router",
            "evidence_need_assessor",
            "workflow_planner",
            "formula_selector",
            "selector",
            "scope_builder",
            "critic",
            "logical_reviewer",
            "efficiency_reviewer",
            "risk_reviewer",
        ];
        let mut missing = Vec::new();
        for profile in &mapped_profiles {
            if !mapping_content.contains(&format!("[{}]", profile)) {
                // Check built-in grammars as fallback
                let has_builtin =
                    crate::json_grammar::builtin_grammar_for_profile(profile).is_some();
                if !has_builtin {
                    missing.push(*profile);
                }
            }
        }
        assert!(
            missing.is_empty(),
            "Grammar mapping certification failed: missing mappings for: {}\n\
             These profiles must have either grammar_mapping.toml entries or built-in grammars.",
            missing.join(", ")
        );
    }

    // ── Runtime smoke harness (ignored; requires live LLM) ──

    /// Load a testing prompt from _testing_prompts/ by name.
    fn load_testing_prompt(name: &str) -> String {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("_testing_prompts")
            .join(name);
        std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Cannot read test prompt {}: {}", path.display(), e))
            .trim()
            .to_string()
    }

    /// Smoke test that loads the Llama 3.2 profile, processes testing prompts
    /// through the routing intel units, and compares output against the Huihui profile.
    ///
    /// Run with:
    ///   cargo test llama32_smoke_comparison -- --ignored --nocapture
    ///
    /// Prerequisites: Both models must be available via a running llama.cpp server.
    #[tokio::test]
    #[ignore]
    async fn llama32_smoke_comparison() {
        let config_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("config");
        let llama_dir = config_root.join(LLAMA32_MODEL);
        let huihui_dir = config_root.join(HUIHUI_MODEL);

        let llama_profiles = crate::app_bootstrap_profiles::load_profiles(&llama_dir)
            .unwrap_or_else(|e| {
                panic!("Failed to load Llama 3.2 profiles: {}", e);
            });
        let huihui_profiles = crate::app_bootstrap_profiles::load_profiles(&huihui_dir)
            .unwrap_or_else(|e| {
                panic!("Failed to load Huihui profiles: {}", e);
            });

        let client = reqwest::Client::new();

        // Test prompts from the basic smoke pack
        let prompts = vec![
            (
                "02_list_files",
                load_testing_prompt("02_list_current_directory.txt"),
            ),
            (
                "03_shell_smoke",
                load_testing_prompt("03_shell_visibility_smoke.txt"),
            ),
            (
                "04_search_read",
                load_testing_prompt("04_search_and_read_smoke.txt"),
            ),
        ];

        eprintln!();
        eprintln!("=== Llama 3.2 vs Huihui/Qwen Routing Comparison ===");
        eprintln!();

        for (name, prompt) in &prompts {
            eprintln!("--- {} ---", name);
            eprintln!("  prompt: {}", &prompt[..prompt.len().min(80)]);

            // Test router (gate) output
            let router_req = crate::intel_trait::build_intel_system_user_request(
                &llama_profiles.router_cfg,
                prompt.clone(),
            );
            eprintln!(
                "  Llama router:    {}",
                crate::intel_trait::execute_intel_text_for_profile(
                    &client,
                    &llama_profiles.router_cfg,
                    router_req,
                )
                .await
                .unwrap_or_else(|e| format!("<error: {}>", e))
            );

            let router_req = crate::intel_trait::build_intel_system_user_request(
                &huihui_profiles.router_cfg,
                prompt.clone(),
            );
            eprintln!(
                "  Huihui router:   {}",
                crate::intel_trait::execute_intel_text_for_profile(
                    &client,
                    &huihui_profiles.router_cfg,
                    router_req,
                )
                .await
                .unwrap_or_else(|e| format!("<error: {}>", e))
            );

            // Test speech_act
            let speech_req = crate::intel_trait::build_intel_system_user_request(
                &llama_profiles.speech_act_cfg,
                prompt.clone(),
            );
            eprintln!(
                "  Llama speech:    {}",
                crate::intel_trait::execute_intel_text_for_profile(
                    &client,
                    &llama_profiles.speech_act_cfg,
                    speech_req,
                )
                .await
                .unwrap_or_else(|e| format!("<error: {}>", e))
            );

            let speech_req = crate::intel_trait::build_intel_system_user_request(
                &huihui_profiles.speech_act_cfg,
                prompt.clone(),
            );
            eprintln!(
                "  Huihui speech:   {}",
                crate::intel_trait::execute_intel_text_for_profile(
                    &client,
                    &huihui_profiles.speech_act_cfg,
                    speech_req,
                )
                .await
                .unwrap_or_else(|e| format!("<error: {}>", e))
            );

            // Test complexity assessment (DSL output)
            let dsl_llama = crate::intel_trait::execute_intel_dsl_from_user_content(
                &client,
                &llama_profiles.complexity_cfg,
                prompt.clone(),
            )
            .await;
            match dsl_llama {
                Ok(v) => eprintln!("  Llama complexity: {}", serde_json::to_string(&v).unwrap()),
                Err(e) => eprintln!("  Llama complexity: <error: {}>", e),
            }

            let dsl_huihui = crate::intel_trait::execute_intel_dsl_from_user_content(
                &client,
                &huihui_profiles.complexity_cfg,
                prompt.clone(),
            )
            .await;
            match dsl_huihui {
                Ok(v) => {
                    eprintln!(
                        "  Huihui complexity: {}",
                        serde_json::to_string(&v).unwrap()
                    )
                }
                Err(e) => eprintln!("  Huihui complexity: <error: {}>", e),
            }
            eprintln!();
        }

        eprintln!("=== End Comparison ===");
    }
}
