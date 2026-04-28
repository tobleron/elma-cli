use crate::registry::{RegistryBuilder, ToolDefinitionExt};

pub(crate) fn register(builder: &mut RegistryBuilder) {
    builder.insert(
        ToolDefinitionExt::new(
            "fetch",
            "Fetch raw content from a URL as text, markdown, or html (max 100KB); no AI processing. For analysis or extraction use agentic_fetch. Fetches are security-gated: only http/https, no private IPs, text-based content types only.",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "The URL to fetch content from"},
                    "timeout": {"type": "integer", "description": "Optional timeout in seconds (max 120)"},
                    "format": {"type": "string", "description": "The format to return the content in (text, markdown, or html)"}
                },
                "required": ["url", "format"]
            }),
            vec![
                "fetch URL content",
                "download web page",
                "get HTTP content",
                "read URL",
                "retrieve web content",
            ],
        )
        .not_deferred(),
    );
}
