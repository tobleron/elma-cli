use std::sync::OnceLock;
use tiktoken_rs::CoreBPE;

use crate::model_capabilities::TokenizerKind;

static CL100K: OnceLock<CoreBPE> = OnceLock::new();

fn cl100k() -> &'static CoreBPE {
    CL100K.get_or_init(|| {
        tiktoken_rs::cl100k_base().expect("cl100k_base BPE must be available (static feature)")
    })
}

#[derive(Debug, Clone)]
pub struct TokenEstimate {
    pub count: usize,
    pub is_exact: bool,
    pub method: String,
}

impl TokenEstimate {
    pub fn label(&self) -> String {
        if self.is_exact {
            format_count(self.count)
        } else {
            format!("~{}", format_count(self.count))
        }
    }
}

fn format_count(n: usize) -> String {
    if n >= 1000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

pub fn count_tokens(text: &str) -> usize {
    cl100k().encode_with_special_tokens(text).len()
}

pub fn count_tokens_for_model(text: &str, tokenizer: TokenizerKind) -> TokenEstimate {
    match tokenizer {
        TokenizerKind::Cl100kBase | TokenizerKind::Tiktoken => TokenEstimate {
            count: cl100k().encode_with_special_tokens(text).len(),
            is_exact: true,
            method: "cl100k".to_string(),
        },
        TokenizerKind::Anthropic => TokenEstimate {
            count: cl100k().encode_with_special_tokens(text).len(),
            is_exact: false,
            method: "est anthropic".to_string(),
        },
        TokenizerKind::HuggingFace => TokenEstimate {
            count: char_based_estimate(text, 3.5),
            is_exact: false,
            method: "est hf".to_string(),
        },
        TokenizerKind::LlamaCpp => TokenEstimate {
            count: char_based_estimate(text, 2.8),
            is_exact: false,
            method: "est llama".to_string(),
        },
        TokenizerKind::Gemma => TokenEstimate {
            count: char_based_estimate(text, 3.8),
            is_exact: false,
            method: "est gemma".to_string(),
        },
        TokenizerKind::Qwen => TokenEstimate {
            count: char_based_estimate(text, 3.0),
            is_exact: false,
            method: "est qwen".to_string(),
        },
        TokenizerKind::Estimator | TokenizerKind::None => TokenEstimate {
            count: char_based_estimate(text, 4.0),
            is_exact: false,
            method: "est".to_string(),
        },
    }
}

fn char_based_estimate(text: &str, default_chars_per_token: f64) -> usize {
    let char_count = text.chars().count();
    if char_count == 0 {
        return 0;
    }
    let cjk = text
        .chars()
        .filter(|c| {
            let cp = *c as u32;
            (0x4E00..=0x9FFF).contains(&cp)
                || (0x3400..=0x4DBF).contains(&cp)
                || (0x20000..=0x2A6DF).contains(&cp)
                || (0x3040..=0x309F).contains(&cp)
                || (0x30A0..=0x30FF).contains(&cp)
                || (0xAC00..=0xD7AF).contains(&cp)
        })
        .count();
    let latin = char_count.saturating_sub(cjk);
    ((latin as f64 / default_chars_per_token) + (cjk as f64 / 1.5)).ceil() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens_cl100k() {
        let count = count_tokens("hello world");
        assert_eq!(count, 2);
    }

    #[test]
    fn test_count_tokens_empty() {
        let count = count_tokens("");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_count_tokens_for_model_exact_cl100k() {
        let est = count_tokens_for_model("hello world", TokenizerKind::Cl100kBase);
        assert_eq!(est.count, 2);
        assert!(est.is_exact);
        assert_eq!(est.method, "cl100k");
    }

    #[test]
    fn test_count_tokens_for_model_exact_tiktoken() {
        let est = count_tokens_for_model("hello world", TokenizerKind::Tiktoken);
        assert_eq!(est.count, 2);
        assert!(est.is_exact);
    }

    #[test]
    fn test_count_tokens_for_model_anthropic_fallback() {
        let est = count_tokens_for_model("hello world", TokenizerKind::Anthropic);
        assert!(est.count > 0);
        assert!(!est.is_exact);
        assert!(est.method.contains("anthropic"));
    }

    #[test]
    fn test_count_tokens_for_model_llama_cpp() {
        let est = count_tokens_for_model("hello world this is a test", TokenizerKind::LlamaCpp);
        assert!(est.count > 0);
        assert!(!est.is_exact);
        assert_eq!(est.method, "est llama");
    }

    #[test]
    fn test_count_tokens_for_model_gemma() {
        let est = count_tokens_for_model("hello world this is a test", TokenizerKind::Gemma);
        assert!(est.count > 0);
        assert!(!est.is_exact);
        assert_eq!(est.method, "est gemma");
    }

    #[test]
    fn test_count_tokens_for_model_qwen() {
        let est = count_tokens_for_model("hello world this is a test", TokenizerKind::Qwen);
        assert!(est.count > 0);
        assert!(!est.is_exact);
        assert_eq!(est.method, "est qwen");
    }

    #[test]
    fn test_count_tokens_for_model_estimator() {
        let est = count_tokens_for_model("hello world this is a test", TokenizerKind::Estimator);
        assert!(est.count > 0);
        assert!(!est.is_exact);
        assert_eq!(est.method, "est");
    }

    #[test]
    fn test_count_tokens_for_model_none() {
        let est = count_tokens_for_model("hello world this is a test", TokenizerKind::None);
        assert!(est.count > 0);
        assert!(!est.is_exact);
        assert_eq!(est.method, "est");
    }

    #[test]
    fn test_char_based_estimate_latin() {
        let count = char_based_estimate("hello world", 4.0);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_char_based_estimate_cjk() {
        let count = char_based_estimate("你好世界", 4.0);
        assert_eq!(count, 3);
    }

    #[test]
    fn test_char_based_estimate_mixed() {
        let count = char_based_estimate("hello 你好 world 世界", 4.0);
        assert_eq!(count, 6);
    }

    #[test]
    fn test_char_based_estimate_empty() {
        let count = char_based_estimate("", 4.0);
        assert_eq!(count, 0);
    }

    #[test]
    fn test_token_estimate_label_exact() {
        let est = count_tokens_for_model("hello world", TokenizerKind::Cl100kBase);
        assert_eq!(est.label(), "2");
    }

    #[test]
    fn test_token_estimate_label_estimated() {
        let est = count_tokens_for_model("hello world", TokenizerKind::LlamaCpp);
        assert!(est.label().starts_with("~"));
    }

    #[test]
    fn test_token_estimate_label_kilo() {
        let big = "a ".repeat(5000);
        let est = count_tokens_for_model(&big, TokenizerKind::Cl100kBase);
        assert!(est.label().contains("k"));
    }
}
