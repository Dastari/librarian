use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct OllamaParserSettings {
    pub enabled: bool,
    pub ollama_url: String,
    pub model: String,
    pub timeout_seconds: u64,
    pub temperature: f64,
    pub max_tokens: u32,
    pub max_retries: u32,
    pub confidence_threshold: f64,
    pub prompt_template: Option<String>,
}

impl Default for OllamaParserSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            ollama_url: "http://localhost:11434".to_string(),
            model: "qwen2.5-coder:7b".to_string(),
            timeout_seconds: 30,
            temperature: 0.1,
            max_tokens: 256,
            max_retries: 2,
            confidence_threshold: 0.7,
            prompt_template: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OllamaParsedHint {
    pub media_type: Option<String>,
    pub title: Option<String>,
    pub year: Option<i32>,
    pub show_name: Option<String>,
    pub season: Option<i32>,
    pub episode: Option<i32>,
    pub artist_name: Option<String>,
    pub album_name: Option<String>,
    pub track_title: Option<String>,
    pub track_number: Option<i32>,
    pub author_name: Option<String>,
    pub audiobook_title: Option<String>,
    pub chapter_title: Option<String>,
    pub chapter_number: Option<i32>,
    pub confidence: Option<f64>,
}

impl OllamaParsedHint {
    pub fn confidence(&self) -> f64 {
        self.confidence.unwrap_or(0.0).clamp(0.0, 1.0)
    }

    pub fn to_match_source(&self, library_type: &str) -> Option<String> {
        match library_type {
            "movies" => {
                let title = self.title.as_deref()?.trim();
                if title.is_empty() {
                    return None;
                }
                Some(match self.year {
                    Some(year) => format!("{title} {year}"),
                    None => title.to_string(),
                })
            }
            "tv" => {
                let show = self.show_name.as_deref().or(self.title.as_deref())?.trim();
                let season = self.season?;
                let episode = self.episode?;
                if show.is_empty() {
                    return None;
                }
                Some(format!("{show}.S{season:02}E{episode:02}"))
            }
            "music" => {
                let title = self
                    .track_title
                    .as_deref()
                    .or(self.title.as_deref())?
                    .trim();
                if title.is_empty() {
                    return None;
                }
                let artist = self.artist_name.as_deref().unwrap_or("Unknown Artist");
                let album = self.album_name.as_deref().unwrap_or("Unknown Album");
                let track = self.track_number.unwrap_or(0);
                Some(format!("{artist}/{album}/{track:02} - {title}"))
            }
            "audiobooks" => {
                let book = self
                    .audiobook_title
                    .as_deref()
                    .or(self.title.as_deref())?
                    .trim();
                if book.is_empty() {
                    return None;
                }
                let author = self.author_name.as_deref().unwrap_or("Unknown Author");
                let chapter = self.chapter_number.unwrap_or(0);
                let chapter_title = self.chapter_title.as_deref().unwrap_or("Unknown Chapter");
                Some(format!("{author}/{book}/{chapter:02} - {chapter_title}"))
            }
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    response: Option<String>,
    message: Option<ChatMessage>,
}

#[derive(Debug, Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

#[derive(Debug, Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    options: GenerateOptions,
}

#[derive(Debug, Serialize)]
struct GenerateOptions {
    temperature: f64,
    num_predict: u32,
}

#[derive(Debug, Clone)]
pub struct OllamaClient {
    client: Option<Client>,
    initialization_error: Option<std::sync::Arc<str>>,
}

impl Default for OllamaClient {
    fn default() -> Self {
        match crate::services::http_client::outbound_client(
            crate::services::http_client::OutboundHttpProfile::LocalService,
        ) {
            Ok(client) => Self {
                client: Some(client),
                initialization_error: None,
            },
            Err(error) => Self {
                client: None,
                initialization_error: Some(std::sync::Arc::from(error.to_string())),
            },
        }
    }
}

impl OllamaClient {
    pub async fn parse_filename(
        &self,
        settings: &OllamaParserSettings,
        library_type: &str,
        filename: &str,
    ) -> Result<OllamaParsedHint> {
        let client = self.client.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "{}",
                self.initialization_error
                    .as_deref()
                    .unwrap_or("Ollama HTTP client is unavailable")
            )
        })?;
        let prompt = build_prompt(settings, library_type, filename);
        let request = GenerateRequest {
            model: &settings.model,
            prompt: &prompt,
            stream: false,
            options: GenerateOptions {
                temperature: settings.temperature,
                num_predict: settings.max_tokens,
            },
        };

        let base_url = settings.ollama_url.trim_end_matches('/');
        let url = format!("{base_url}/api/generate");
        let attempts = settings.max_retries.saturating_add(1);
        let mut last_error: Option<anyhow::Error> = None;

        for attempt in 0..attempts {
            let result = client
                .post(&url)
                .timeout(Duration::from_secs(settings.timeout_seconds.max(1)))
                .json(&request)
                .send()
                .await
                .with_context(|| {
                    format!(
                        "Ollama filename parse request failed for library_type={library_type}, filename='{filename}'"
                    )
                });

            match result {
                Ok(response) => {
                    if !response.status().is_success() {
                        last_error = Some(anyhow::anyhow!(
                            "Ollama filename parse failed for library_type={}, filename='{}': HTTP {}",
                            library_type,
                            filename,
                            response.status()
                        ));
                    } else {
                        let response: GenerateResponse =
                            crate::services::http_client::response_json_limited(
                                response,
                                crate::services::http_client::METADATA_RESPONSE_LIMIT,
                            )
                            .await
                            .with_context(|| {
                                format!(
                                    "Ollama filename parse response was not valid JSON for library_type={library_type}, filename='{filename}'"
                                )
                            })?;
                        let content = response
                            .response
                            .or_else(|| response.message.and_then(|m| m.content))
                            .ok_or_else(|| {
                                anyhow::anyhow!("Ollama response did not include content")
                            })?;
                        return parse_ollama_hint(&content);
                    }
                }
                Err(error) => {
                    last_error = Some(error);
                }
            }

            if attempt + 1 < attempts {
                tokio::time::sleep(Duration::from_millis(150 * (attempt as u64 + 1))).await;
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Ollama request failed")))
    }
}

pub fn build_prompt(settings: &OllamaParserSettings, library_type: &str, filename: &str) -> String {
    if let Some(template) = settings
        .prompt_template
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        return template
            .replace("{library_type}", library_type)
            .replace("{filename}", filename);
    }

    format!(
        "Parse this {library_type} media filename into JSON only. Filename: {filename}\n\
         Return only one JSON object with camelCase keys. Supported keys: mediaType, title, year, \
         showName, season, episode, artistName, albumName, trackTitle, trackNumber, authorName, \
         audiobookTitle, chapterTitle, chapterNumber, confidence. Confidence must be 0.0 to 1.0. \
         Do not include explanations or markdown."
    )
}

pub fn parse_ollama_hint(content: &str) -> Result<OllamaParsedHint> {
    let json = extract_json_object(content)
        .ok_or_else(|| anyhow::anyhow!("Ollama response did not contain a JSON object"))?;
    let hint: OllamaParsedHint =
        serde_json::from_str(json).context("Ollama JSON object did not match parser schema")?;
    Ok(hint)
}

fn extract_json_object(content: &str) -> Option<&str> {
    let trimmed = content.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        return Some(trimmed);
    }

    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end > start {
        Some(&trimmed[start..=end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_movie_generate_response_fixture() {
        let raw = include_str!("../../tests/fixtures/ollama/movie_response.json");
        let response: GenerateResponse = serde_json::from_str(raw).expect("fixture parses");
        let hint = parse_ollama_hint(response.response.as_deref().unwrap()).expect("hint parses");
        assert_eq!(hint.title.as_deref(), Some("The Hunt for Red October"));
        assert_eq!(hint.year, Some(1990));
        assert!(hint.confidence() >= 0.9);
        assert_eq!(
            hint.to_match_source("movies").as_deref(),
            Some("The Hunt for Red October 1990")
        );
    }

    #[test]
    fn parses_tv_generate_response_fixture() {
        let raw = include_str!("../../tests/fixtures/ollama/tv_response.json");
        let response: GenerateResponse = serde_json::from_str(raw).expect("fixture parses");
        let hint = parse_ollama_hint(response.response.as_deref().unwrap()).expect("hint parses");
        assert_eq!(hint.show_name.as_deref(), Some("Chicago Fire"));
        assert_eq!(hint.season, Some(14));
        assert_eq!(hint.episode, Some(8));
        assert_eq!(
            hint.to_match_source("tv").as_deref(),
            Some("Chicago Fire.S14E08")
        );
    }

    #[test]
    fn rejects_invalid_response_fixture() {
        let raw = include_str!("../../tests/fixtures/ollama/invalid_response.json");
        let response: GenerateResponse = serde_json::from_str(raw).expect("fixture parses");
        assert!(parse_ollama_hint(response.response.as_deref().unwrap()).is_err());
    }

    #[test]
    fn prompt_template_replaces_tokens() {
        let settings = OllamaParserSettings {
            prompt_template: Some("Parse {library_type}: {filename}".to_string()),
            ..Default::default()
        };
        assert_eq!(
            build_prompt(&settings, "movies", "Example.2024.mkv"),
            "Parse movies: Example.2024.mkv"
        );
    }
}
