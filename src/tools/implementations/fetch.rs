use crate::tools::types::{ToolExecutionResult};
use crate::tools::helpers::{emit_tool_start, emit_tool_result};

pub async fn exec_fetch(
    client: &reqwest::Client,
    av: &serde_json::Value,
    call_id: &str,
    mut tui: Option<&mut crate::ui_terminal::TerminalUI>,
) -> ToolExecutionResult {
    let url_str = av["url"].as_str().unwrap_or("").to_string();
    let format = av["format"].as_str().unwrap_or("text").to_string();
    let timeout_secs = av["timeout"].as_u64().unwrap_or(120);

    if url_str.is_empty() {
        return ToolExecutionResult::new_failed(call_id, "fetch", "Error: empty URL");
    }

    let parsed_url = match url::Url::parse(&url_str) {
        Ok(u) => u,
        Err(e) => {
            return ToolExecutionResult::new_failed(call_id, "fetch", &format!("Error: invalid URL: {}", e));
        }
    };

    let scheme = parsed_url.scheme();
    if scheme != "http" && scheme != "https" {
        return ToolExecutionResult::new_failed(call_id, "fetch", &format!("Error: only http and https schemes are allowed, got '{}'", scheme));
    }

    emit_tool_start(&mut tui, "fetch", &url_str);

    let request = client
        .get(parsed_url.as_str())
        .timeout(std::time::Duration::from_secs(timeout_secs.min(120)))
        .header("User-Agent", "ElmaCLI/1.0")
        .send();

    let response = match request.await {
        Ok(r) => r,
        Err(e) => {
            let msg = if e.is_timeout() {
                format!("Error: request timed out after {}s", timeout_secs)
            } else {
                format!("Error: request failed: {}", e)
            };
            emit_tool_result(&mut tui, "fetch", false, &msg);
            return ToolExecutionResult::new_failed(call_id, "fetch", &msg);
        }
    };

    let status = response.status();
    if !status.is_success() {
        let msg = format!("Error: HTTP {}", status);
        emit_tool_result(&mut tui, "fetch", false, &msg);
        return ToolExecutionResult::new_failed(call_id, "fetch", &msg);
    }

    let raw_bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            let msg = format!("Error: failed to read response body: {}", e);
            emit_tool_result(&mut tui, "fetch", false, &msg);
            return ToolExecutionResult::new_failed(call_id, "fetch", &msg);
        }
    };

    let capped = &raw_bytes[..raw_bytes.len().min(100_000)];

    let content = match format.as_str() {
        "markdown" => html2text::from_read(capped, 120)
            .unwrap_or_else(|_| String::from_utf8_lossy(capped).to_string()),
        "html" => String::from_utf8_lossy(capped).to_string(),
        _ => String::from_utf8_lossy(capped).to_string(),
    };

    let truncated = if raw_bytes.len() > 100_000 {
        format!(
            "{}\n\n[Content truncated at 100KB — fetched {} bytes total]",
            content,
            raw_bytes.len()
        )
    } else {
        content
    };

    emit_tool_result(&mut tui, "fetch", true, &truncated);
    ToolExecutionResult::new_ok(call_id, "fetch", &truncated)
}
