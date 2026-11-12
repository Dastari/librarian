//! Optional second opinion for what rules can't settle, from TypeSafe's Jev decision model.
//!
//! Off unless enabled in settings, where its key is stored encrypted (`MatchingAssistant`). It only
//! ever chooses among things the rules already found and it never overrides them, so matching works
//! the same without it, just with more left for a person to review. File and title names leave this
//! machine when it is on.

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use anyhow::{Context, Result};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::services::http_client::{self, OutboundHttpProfile};

const ENDPOINT: &str = "https://api.typesafe.ai/v1/systemone";
const MODELS_ENDPOINT: &str = "https://api.typesafe.ai/v1/models";
pub const DEFAULT_MODEL: &str = "jev-latest";
const TIMEOUT: Duration = Duration::from_secs(10);
const RESPONSE_LIMIT: usize = 1024 * 1024;
/// The API takes at most this many options in one question.
const MAX_CHOICES: usize = 255;

#[derive(Debug, Deserialize)]
struct Response {
    #[serde(default)]
    answers: HashMap<String, Answer>,
}

#[derive(Debug, Deserialize)]
struct Answer {
    /// Probability of yes, for a yes/no question.
    noul: Option<f64>,
    /// The likeliest option, for a choice question.
    choice: Option<String>,
    confidence: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ModelList {
    #[serde(default)]
    models: Vec<ModelName>,
}

#[derive(Debug, Deserialize)]
struct ModelName {
    name: String,
}

#[derive(Clone)]
pub struct Jev {
    client: reqwest::Client,
    key: String,
    model: String,
}

impl Jev {
    fn new(key: String, model: String) -> Option<Self> {
        let client = http_client::outbound_client(OutboundHttpProfile::Metadata).ok()?;
        Some(Self {
            client,
            key,
            model: Some(model.trim().to_string())
                .filter(|model| !model.is_empty())
                .unwrap_or_else(|| DEFAULT_MODEL.to_string()),
        })
    }

    /// Available only when it is enabled and has a key that can be decrypted.
    pub async fn configured(services: &crate::services::ServicesManager) -> Option<Self> {
        let row = Self::stored(services).await?;
        if !row.enabled || !row.has_api_key {
            return None;
        }
        let key = Self::stored_key(services, &row).await?;
        Self::new(key, row.model)
    }

    async fn stored(
        services: &crate::services::ServicesManager,
    ) -> Option<crate::graphql::entities::MatchingAssistant> {
        let database = services.get_database().await?;
        crate::graphql::entities::MatchingAssistant::query(database.pool().pool())
            .limit(1)
            .fetch_all()
            .await
            .ok()?
            .into_iter()
            .next()
    }

    async fn stored_key(
        services: &crate::services::ServicesManager,
        row: &crate::graphql::entities::MatchingAssistant,
    ) -> Option<String> {
        let sources = services.get_sources().await?.get_manager().await?;
        match sources.encryption().decrypt_envelope(&row.api_key) {
            Ok(key) => Some(key),
            Err(_) => {
                tracing::warn!("The stored Jev key could not be decrypted");
                None
            }
        }
    }

    /// Check a key, the one given or the stored one, by listing the models it can use. This doesn't
    /// depend on it being enabled, so it can be checked before turning it on.
    pub async fn test(
        services: &crate::services::ServicesManager,
        key: Option<String>,
    ) -> Result<Vec<String>> {
        let key = match key
            .map(|key| key.trim().to_string())
            .filter(|key| !key.is_empty())
        {
            Some(key) => key,
            None => {
                let row = Self::stored(services)
                    .await
                    .filter(|row| row.has_api_key)
                    .context("Enter an API key first")?;
                Self::stored_key(services, &row)
                    .await
                    .context("The saved key could not be read. Enter it again.")?
            }
        };
        let client = http_client::outbound_client(OutboundHttpProfile::Metadata)?;
        let response = client
            .get(MODELS_ENDPOINT)
            .bearer_auth(&key)
            .timeout(TIMEOUT)
            .send()
            .await
            .context("Could not reach TypeSafe")?;
        anyhow::ensure!(
            response.status().is_success(),
            "TypeSafe refused the key: {}",
            response.status()
        );
        let listed: ModelList =
            http_client::response_json_limited(response, RESPONSE_LIMIT).await?;
        Ok(listed.models.into_iter().map(|model| model.name).collect())
    }

    async fn ask(&self, state: Value, questions: Value) -> Result<HashMap<String, Answer>> {
        let response = self
            .client
            .post(ENDPOINT)
            .bearer_auth(&self.key)
            .timeout(TIMEOUT)
            .json(&json!({ "model": self.model, "state": state, "questions": questions }))
            .send()
            .await
            .context("Could not reach Jev")?
            .error_for_status()
            .context("Jev refused the request")?;
        let parsed: Response = http_client::response_json_limited(response, RESPONSE_LIMIT).await?;
        Ok(parsed.answers)
    }

    /// The probability of yes for each named question, all about the same `state`.
    pub async fn yes_no(
        &self,
        state: Value,
        questions: &BTreeMap<String, String>,
    ) -> Result<HashMap<String, f64>> {
        let body: serde_json::Map<String, Value> = questions
            .iter()
            .map(|(name, instructions)| {
                (
                    name.clone(),
                    json!({ "type": "noul", "instructions": instructions }),
                )
            })
            .collect();
        Ok(self
            .ask(state, Value::Object(body))
            .await?
            .into_iter()
            .filter_map(|(name, answer)| Some((name, answer.noul?)))
            .collect())
    }

    /// For each named item, the likeliest of `choices` and how sure it is.
    pub async fn choose(
        &self,
        state: Value,
        instructions: &str,
        choices: &[String],
        items: &[String],
    ) -> Result<HashMap<String, (String, f64)>> {
        if choices.is_empty() || choices.len() > MAX_CHOICES || items.is_empty() {
            return Ok(HashMap::new());
        }
        let criteria: serde_json::Map<String, Value> = choices
            .iter()
            .map(|choice| (choice.clone(), Value::Null))
            .collect();
        let body: serde_json::Map<String, Value> = items
            .iter()
            .map(|item| {
                (
                    item.clone(),
                    json!({
                        "type": "choice",
                        "instructions": format!("{instructions} The file is: {item}"),
                        "criteria": criteria,
                    }),
                )
            })
            .collect();
        Ok(self
            .ask(state, Value::Object(body))
            .await?
            .into_iter()
            .filter_map(|(name, answer)| {
                Some((name, (answer.choice?, answer.confidence.unwrap_or(0.0))))
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_read_from_the_documented_response() {
        let parsed: Response = serde_json::from_value(json!({
            "model": "jev-1.13.0",
            "answers": {
                "same": { "type": "noul", "noul": 0.93 },
                "track": {
                    "type": "choice",
                    "choice": "So What",
                    "confidence": 0.7,
                    "probabilities": { "So What": 0.8, "Blue in Green": 0.2 }
                }
            },
            "usage": { "input_tokens": 120, "output_tokens": 12 }
        }))
        .expect("documented shape");
        assert_eq!(parsed.answers["same"].noul, Some(0.93));
        assert_eq!(parsed.answers["track"].choice.as_deref(), Some("So What"));
        assert_eq!(parsed.answers["track"].confidence, Some(0.7));
    }
}
