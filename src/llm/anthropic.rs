use anyhow::{Context, Result};
use serde_json::json;
use std::time::Duration;

use super::{EnrichmentRequest, EnrichmentResponse, LlmProvider, system_prompt, user_prompt};
use crate::config::Config;

pub struct AnthropicProvider {
    api_key: String,
    model: String,
    timeout: Duration,
}

impl AnthropicProvider {
    pub fn new(cfg: &Config) -> Self {
        AnthropicProvider {
            api_key: cfg.llm.api_key.clone().unwrap_or_default(),
            model: cfg.llm.model.clone(),
            timeout: Duration::from_secs(cfg.llm.timeout_secs),
        }
    }
}

impl LlmProvider for AnthropicProvider {
    fn enrich(&self, req: &EnrichmentRequest) -> Result<EnrichmentResponse> {
        let schema = schemars::schema_for!(EnrichmentResponse);
        let schema_val = serde_json::to_value(&schema)?;

        // Use tool-use pattern (reliable GA method for Anthropic structured output)
        let body = json!({
            "model": self.model,
            "max_tokens": 1024,
            "system": system_prompt(req),
            "messages": [
                {"role": "user", "content": user_prompt(req)}
            ],
            "tools": [{
                "name": "enrich_task",
                "description": "Return enrichment suggestions for a task",
                "input_schema": schema_val
            }],
            "tool_choice": {"type": "tool", "name": "enrich_task"}
        });

        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()?;

        let resp = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .context("Anthropic request failed")?;

        let resp_json: serde_json::Value = resp
            .error_for_status()
            .context("Anthropic returned an error")?
            .json()
            .context("Anthropic response was not valid JSON")?;

        // Extract tool_use input from the response
        let tool_input = resp_json["content"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|item| item["type"].as_str() == Some("tool_use"))
            })
            .and_then(|item| item.get("input"))
            .ok_or_else(|| anyhow::anyhow!("Unexpected Anthropic response shape"))?;

        let enrichment: EnrichmentResponse = serde_json::from_value(tool_input.clone())
            .context("Could not parse Anthropic enrichment JSON")?;
        Ok(enrichment)
    }
}
