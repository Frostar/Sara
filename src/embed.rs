use anyhow::{Context, Result};
use reqwest::blocking::Client;
use rusqlite::Connection;
use uuid::Uuid;

use crate::config::Config;

pub fn embed_text(cfg: &Config, text: &str) -> Result<Vec<f32>> {
    let emb = &cfg.embeddings;
    let base = emb
        .base_url
        .as_deref()
        .unwrap_or("http://localhost:11434");
    let url = format!("{base}/api/embeddings");
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    let body = serde_json::json!({
        "model": emb.model,
        "prompt": text,
    });
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .context("Embeddings request failed (is Ollama running?)")?;
    let json: serde_json::Value = resp.json()?;
    let embedding = json
        .get("embedding")
        .and_then(|v| v.as_array())
        .context("No embedding in response")?;
    Ok(embedding
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect())
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 {
        0.0
    } else {
        dot / (na * nb)
    }
}

pub fn embed_and_store(conn: &Connection, cfg: &Config, ref_uuid: &Uuid, text: &str) -> Result<()> {
    if text.trim().is_empty() {
        return Ok(());
    }
    match embed_text(cfg, text) {
        Ok(vec) => {
            crate::db::upsert_embedding(conn, ref_uuid, &vec)?;
            Ok(())
        }
        Err(e) => {
            eprintln!("Warning: could not embed: {e}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_identical_vectors() {
        let v = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&v, &v) - 1.0).abs() < 0.001);
    }
}
