//! Ollama API client for LLM-based filename parsing
//!
//! This service provides a fallback parser using local LLMs via Ollama
//! when the regex-based parser fails or has low confidence.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Configuration for the Ollama service
#[derive(Debug, Clone)]
pub struct OllamaConfig {
    pub url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub temperature: f32,
    pub max_tokens: u32,
    pub prompt_template: String,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:11434".to_string(),
            model: "qwen2.5-coder:7b".to_string(),
            timeout_seconds: 30,
            temperature: 0.1,
            max_tokens: 256,
            prompt_template: r#"Parse this media filename. Fill ALL fields. Use null if not found.
Clean the title (remove dots/underscores). Release group is after final hyphen.
Set type to "movie" or "tv" based on whether season/episode are present.

Filename: {filename}

{"type":null,"title":null,"year":null,"season":null,"episode":null,"resolution":null,"source":null,"video_codec":null,"audio":null,"hdr":null,"release_group":null,"edition":null}"#.to_string(),
        }
    }
}

/// Request body for Ollama generate API
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

/// Response from Ollama generate API
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
    done: bool,
    #[serde(default)]
    total_duration: u64,
    #[serde(default)]
    eval_count: u32,
}

/// Parsed result from LLM
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LlmParseResult {
    #[serde(rename = "type")]
    pub media_type: Option<String>,
    pub title: Option<String>,
    pub year: Option<i32>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub episode_end: Option<i32>,
    pub resolution: Option<String>,
    pub source: Option<String>,
    pub video_codec: Option<String>,
    pub audio: Option<String>,
    pub hdr: Option<String>,
    pub release_group: Option<String>,
    pub edition: Option<String>,
    #[serde(default)]
    pub complete_series: bool,
    #[serde(default)]
    pub confidence: f64,
}

/// Service for interacting with Ollama API
pub struct OllamaService {
    client: reqwest::Client,
    config: OllamaConfig,
}

impl OllamaService {
    /// Create a new OllamaService with the given configuration
    pub fn new(config: OllamaConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Create a new OllamaService with default configuration
    pub fn with_defaults() -> Self {
        Self::new(OllamaConfig::default())
    }

    /// Update configuration
    pub fn set_config(&mut self, config: OllamaConfig) {
        self.config = config;
        self.client = reqwest::Client::builder()
            .timeout(Duration::from_secs(self.config.timeout_seconds))
            .build()
            .expect("Failed to create HTTP client");
    }

    /// Test connection to Ollama server
    pub async fn test_connection(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.config.url);

        #[derive(Deserialize)]
        struct TagsResponse {
            models: Vec<ModelInfo>,
        }

        #[derive(Deserialize)]
        struct ModelInfo {
            name: String,
        }

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to connect to Ollama server")?;

        if !response.status().is_success() {
            anyhow::bail!("Ollama server returned status: {}", response.status());
        }

        let tags: TagsResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }

    /// Parse a filename using the LLM
    pub async fn parse_filename(&self, filename: &str) -> Result<LlmParseResult> {
        self.parse_filename_with_overrides(filename, None, None)
            .await
    }

    /// Parse a filename with optional model and prompt overrides
    pub async fn parse_filename_with_overrides(
        &self,
        filename: &str,
        model_override: Option<&str>,
        prompt_override: Option<&str>,
    ) -> Result<LlmParseResult> {
        let prompt_template = prompt_override.unwrap_or(&self.config.prompt_template);
        let prompt = prompt_template.replace("{filename}", filename);
        let model = model_override.unwrap_or(&self.config.model);

        let request = OllamaRequest {
            model: model.to_string(),
            prompt,
            stream: false,
            options: OllamaOptions {
                temperature: self.config.temperature,
                num_predict: self.config.max_tokens,
            },
        };

        let url = format!("{}/api/generate", self.config.url);

        debug!("Sending request to Ollama for filename: {}", filename);

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API error: {} - {}", status, body);
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .context("Failed to parse Ollama response")?;

        let duration_ms = ollama_response.total_duration / 1_000_000;
        info!(
            "LLM parsed '{}' in {}ms ({} tokens)",
            filename, duration_ms, ollama_response.eval_count
        );

        // Extract JSON from response (may be wrapped in ```json ... ```)
        let json_str = extract_json(&ollama_response.response)?;

        let result: LlmParseResult =
            serde_json::from_str(&json_str).context("Failed to parse LLM JSON output")?;

        Ok(result)
    }

    /// Get current configuration
    pub fn config(&self) -> &OllamaConfig {
        &self.config
    }
}

/// Extract JSON from a response that may be wrapped in markdown code fences
fn extract_json(response: &str) -> Result<String> {
    let trimmed = response.trim();

    // Check if wrapped in ```json ... ```
    if trimmed.starts_with("```") {
        let lines: Vec<&str> = trimmed.lines().collect();
        if lines.len() >= 3 {
            // Skip first line (```json) and last line (```)
            let json_lines: Vec<&str> = lines[1..lines.len() - 1]
                .iter()
                .filter(|l| !l.trim().is_empty())
                .copied()
                .collect();
            return Ok(json_lines.join("\n"));
        }
    }

    // Check if it starts with { (raw JSON)
    if trimmed.starts_with('{') {
        return Ok(trimmed.to_string());
    }

    // Try to find JSON object in the response
    if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            return Ok(trimmed[start..=end].to_string());
        }
    }

    warn!("Could not extract JSON from LLM response: {}", trimmed);
    anyhow::bail!("No valid JSON found in LLM response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_raw() {
        let input = r#"{"type": "movie", "title": "Test"}"#;
        let result = extract_json(input).unwrap();
        assert!(result.contains("movie"));
    }

    #[test]
    fn test_extract_json_fenced() {
        let input = r#"```json
{"type": "movie", "title": "Test"}
```"#;
        let result = extract_json(input).unwrap();
        assert!(result.contains("movie"));
    }

    #[test]
    fn test_extract_json_with_prefix() {
        let input = r#"Here is the parsed result:
{"type": "movie", "title": "Test"}"#;
        let result = extract_json(input).unwrap();
        assert!(result.contains("movie"));
    }

    #[test]
    fn test_default_config() {
        let config = OllamaConfig::default();
        assert_eq!(config.url, "http://localhost:11434");
        assert_eq!(config.model, "qwen2.5-coder:7b");
        assert!(config.prompt_template.contains("{filename}"));
    }
}
