use crate::*;

pub(crate) async fn fetch_first_model_id(client: &reqwest::Client, base_url: &Url) -> Result<String> {
    let url = base_url
        .join("/v1/models")
        .context("Failed to build /v1/models URL")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("Failed to read /v1/models body")?;
    if !status.is_success() {
        anyhow::bail!("GET /v1/models returned HTTP {status}: {text}");
    }
    let parsed: ModelsList = serde_json::from_str(&text).context("Invalid JSON from /v1/models")?;
    let list = parsed
        .data
        .or(parsed.models)
        .unwrap_or_default()
        .into_iter();
    for item in list {
        if let Some(id) = item.id.or(item.name).or(item.model) {
            if !id.trim().is_empty() {
                return Ok(id);
            }
        }
    }
    anyhow::bail!("No model ids found in /v1/models response")
}

pub(crate) async fn fetch_all_model_ids(client: &reqwest::Client, base_url: &Url) -> Result<Vec<String>> {
    let url = base_url
        .join("/v1/models")
        .context("Failed to build /v1/models URL")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("Failed to read /v1/models body")?;
    if !status.is_success() {
        anyhow::bail!("GET /v1/models returned HTTP {status}: {text}");
    }
    let parsed: ModelsList = serde_json::from_str(&text).context("Invalid JSON from /v1/models")?;
    let mut out = Vec::new();
    let list = parsed.data.or(parsed.models).unwrap_or_default();
    for item in list {
        if let Some(id) = item.id.or(item.name).or(item.model) {
            let id = id.trim().to_string();
            if !id.is_empty() && !out.contains(&id) {
                out.push(id);
            }
        }
    }
    if out.is_empty() {
        anyhow::bail!("No model ids found in /v1/models response");
    }
    Ok(out)
}

pub(crate) async fn fetch_ctx_max(client: &reqwest::Client, base_url: &Url) -> Result<Option<u64>> {
    // Best-effort, ordered by "most likely runtime truth":
    // 1) /slots[0].n_ctx (runtime ctx size)
    // 2) /props.default_generation_settings.n_ctx (runtime default)
    // 3) /v1/models meta.n_ctx_train (training ctx, can be larger than runtime)

    if let Ok(url) = base_url.join("/slots") {
        if let Ok(resp) = client.get(url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        let n = v
                            .get(0)
                            .and_then(|s| s.get("n_ctx"))
                            .and_then(|x| x.as_u64());
                        if n.is_some() {
                            return Ok(n);
                        }
                    }
                }
            }
        }
    }

    if let Ok(url) = base_url.join("/props") {
        if let Ok(resp) = client.get(url).send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        let n = v
                            .get("default_generation_settings")
                            .and_then(|d| d.get("n_ctx"))
                            .and_then(|x| x.as_u64());
                        if n.is_some() {
                            return Ok(n);
                        }
                    }
                }
            }
        }
    }

    let url = base_url
        .join("/v1/models")
        .context("Failed to build /v1/models URL")?;
    let resp = client
        .get(url)
        .send()
        .await
        .context("GET /v1/models failed")?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("Failed to read /v1/models body")?;
    if !status.is_success() {
        return Ok(None);
    }
    let v: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Ok(None),
    };
    Ok(v.get("data")
        .and_then(|d| d.get(0))
        .and_then(|m| m.get("meta"))
        .and_then(|meta| meta.get("n_ctx_train"))
        .and_then(|x| x.as_u64()))
}
