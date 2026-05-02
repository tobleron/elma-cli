use std::sync::OnceLock;
use tiktoken_rs::CoreBPE;

static CL100K: OnceLock<CoreBPE> = OnceLock::new();

fn cl100k() -> &'static CoreBPE {
    CL100K.get_or_init(|| {
        tiktoken_rs::cl100k_base()
            .expect("cl100k_base BPE must be available (static feature)")
    })
}

pub fn count_tokens(text: &str) -> usize {
    cl100k().encode_with_special_tokens(text).len()
}

pub fn count_tokens_for_model(text: &str, tokenizer: crate::model_capabilities::TokenizerKind) -> usize {
    let _ = match tokenizer {
        crate::model_capabilities::TokenizerKind::Cl100kBase
        | crate::model_capabilities::TokenizerKind::Tiktoken
        | crate::model_capabilities::TokenizerKind::Anthropic
        | crate::model_capabilities::TokenizerKind::HuggingFace
        | crate::model_capabilities::TokenizerKind::Estimator
        | crate::model_capabilities::TokenizerKind::None => cl100k(),
    };
    cl100k().encode_with_special_tokens(text).len()
}
