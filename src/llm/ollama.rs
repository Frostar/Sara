use anyhow::{Context, Result};
use serde_json::json;
use std::time::Duration;

use super::{EnrichmentRequest, EnrichmentResponse, LlmProvider, system_prompt, user_prompt};
use crate::config::Config;

pub struct OllamaProvider {
    base_url: String,
    model: String,
    timeout: Duration,
}

impl OllamaProvider {
    pub fn new(cfg: &Config) -> Self {
        OllamaProvider {
            base_url: cfg
                .llm
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost:11434".to_string()),
            model: cfg.llm.model.clone(),
            timeout: Duration::from_secs(cfg.llm.timeout_secs),
        }
    }
}

impl LlmProvider for OllamaProvider {
    fn enrich(&self, req: &EnrichmentRequest) -> Result<EnrichmentResponse> {
        let schema = schemars::schema_for!(EnrichmentResponse);
        let schema_val = serde_json::to_value(&schema)?;

        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt(req)},
                {"role": "user",   "content": user_prompt(req)}
            ],
            "format": schema_val,
            "options": { "temperature": 0 },
            "stream": false
        });

        let client = reqwest::blocking::Client::builder()
            .timeout(self.timeout)
            .build()?;

        let url = format!("{}/api/chat", self.base_url);
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .context("Ollama request failed — is Ollama running?")?;

        if resp.status() == 404 {
            anyhow::bail!(
                "Ollama model '{}' not found. Run: ollama pull {}",
                self.model, self.model
            );
        }

        let resp_json: serde_json::Value = resp
            .error_for_status()
            .context("Ollama returned an error")?
            .json()
            .context("Ollama response was not valid JSON")?;

        let content = resp_json["message"]["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Unexpected Ollama response shape"))?;

        let enrichment: EnrichmentResponse =
            serde_json::from_str(content).context("Could not parse Ollama enrichment JSON")?;
        Ok(enrichment)
    }
}
