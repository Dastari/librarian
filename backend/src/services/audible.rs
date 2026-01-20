//! Audible/OpenLibrary API client for audiobook metadata
//!
//! This service combines:
//! - Audible API (unofficial) for audiobook-specific metadata
//! - OpenLibrary API for book information and cover art
//!
//! Note: Audible doesn't have an official public API, so we use web scraping or third-party APIs
//! OpenLibrary is fully open and free.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use super::rate_limiter::{RateLimitConfig, RateLimitedClient, RetryConfig, retry_async};

/// Combined audiobook metadata client
pub struct AudiobookMetadataClient {
    client: Arc<RateLimitedClient>,
    retry_config: RetryConfig,
}

/// OpenLibrary search result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLibrarySearchResult {
    #[serde(rename = "numFound")]
    pub num_found: i32,
    pub start: i32,
    pub docs: Vec<OpenLibraryWork>,
}

/// OpenLibrary work (book)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLibraryWork {
    pub key: String, // e.g., "/works/OL12345W"
    pub title: String,
    #[serde(rename = "author_name")]
    pub author_names: Option<Vec<String>>,
    #[serde(rename = "author_key")]
    pub author_keys: Option<Vec<String>>,
    #[serde(rename = "first_publish_year")]
    pub first_publish_year: Option<i32>,
    pub isbn: Option<Vec<String>>,
    pub subject: Option<Vec<String>>,
    pub language: Option<Vec<String>>,
    pub cover_i: Option<i64>, // Cover ID for building cover URL
    pub publisher: Option<Vec<String>>,
    #[serde(rename = "number_of_pages_median")]
    pub pages: Option<i32>,
}

/// OpenLibrary author
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLibraryAuthor {
    pub key: String, // e.g., "/authors/OL12345A"
    pub name: String,
    pub bio: Option<OpenLibraryText>,
    pub photos: Option<Vec<i64>>,
    pub birth_date: Option<String>,
    pub death_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenLibraryText {
    Simple(String),
    Complex { value: String },
}

impl OpenLibraryText {
    pub fn value(&self) -> &str {
        match self {
            OpenLibraryText::Simple(s) => s,
            OpenLibraryText::Complex { value } => value,
        }
    }
}

/// Audiobook search result (unified)
#[derive(Debug, Clone, Serialize)]
pub struct AudiobookSearchResult {
    pub title: String,
    pub author_name: Option<String>,
    pub year: Option<i32>,
    pub cover_url: Option<String>,
    pub openlibrary_id: Option<String>,
    pub isbn: Option<String>,
    pub description: Option<String>,
}

/// Audiobook details
#[derive(Debug, Clone, Serialize)]
pub struct AudiobookDetails {
    pub title: String,
    pub subtitle: Option<String>,
    pub author_name: String,
    pub author_id: Option<String>,
    pub year: Option<i32>,
    pub description: Option<String>,
    pub cover_url: Option<String>,
    pub openlibrary_id: Option<String>,
    pub isbn: Option<String>,
    pub subjects: Vec<String>,
    pub publishers: Vec<String>,
    pub language: Option<String>,
}

impl AudiobookMetadataClient {
    /// Create a new audiobook metadata client
    pub fn new() -> Self {
        Self {
            // OpenLibrary has generous rate limits
            client: Arc::new(RateLimitedClient::new(
                "openlibrary",
                RateLimitConfig {
                    requests_per_second: 10,
                    burst_size: 20,
                },
            )),
            retry_config: RetryConfig {
                max_retries: 3,
                initial_interval: Duration::from_millis(500),
                max_interval: Duration::from_secs(10),
                multiplier: 2.0,
            },
        }
    }

    /// Search for audiobooks/books
    pub async fn search(&self, query: &str) -> Result<Vec<AudiobookSearchResult>> {
        debug!("Searching OpenLibrary for '{}'", query);

        let url = "https://openlibrary.org/search.json";
        let client = self.client.clone();
        let query_owned = query.to_string();
        let retry_config = self.retry_config.clone();

        let result = retry_async(
            || {
                let url = url.to_string();
                let client = client.clone();
                let q = query_owned.clone();
                async move {
                    let query_params = [
                        ("q", q),
                        ("limit", "25".to_string()),
                        ("fields", "key,title,author_name,author_key,first_publish_year,isbn,cover_i,subject,publisher".to_string()),
                    ];

                    let response = client
                        .get_with_query(&url, &query_params)
                        .await?;

                    if response.status().as_u16() == 503 {
                        warn!("OpenLibrary rate limit hit, will retry");
                        anyhow::bail!("Rate limited (503)");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("OpenLibrary search failed with status: {}", response.status());
                    }

                    let results: OpenLibrarySearchResult = response
                        .json()
                        .await
                        .context("Failed to parse OpenLibrary search results")?;

                    let audiobooks: Vec<AudiobookSearchResult> = results
                        .docs
                        .into_iter()
                        .map(|work| {
                            let cover_url = work.cover_i.map(|id| {
                                format!("https://covers.openlibrary.org/b/id/{}-L.jpg", id)
                            });

                            AudiobookSearchResult {
                                title: work.title,
                                author_name: work.author_names.and_then(|a| a.into_iter().next()),
                                year: work.first_publish_year,
                                cover_url,
                                openlibrary_id: Some(work.key.replace("/works/", "")),
                                isbn: work.isbn.and_then(|isbns| isbns.into_iter().next()),
                                description: None, // Would need separate API call
                            }
                        })
                        .collect();

                    Ok(audiobooks)
                }
            },
            &retry_config,
            "openlibrary_search",
        )
        .await?;

        debug!(count = result.len(), "OpenLibrary search returned results");
        Ok(result)
    }

    /// Get author details from OpenLibrary
    pub async fn get_author(&self, author_id: &str) -> Result<OpenLibraryAuthor> {
        debug!("Fetching author {} from OpenLibrary", author_id);

        // Normalize author ID (ensure it has the /authors/ prefix)
        let normalized_id = if author_id.starts_with("OL") {
            format!("/authors/{}", author_id)
        } else {
            author_id.to_string()
        };

        let url = format!("https://openlibrary.org{}.json", normalized_id);
        let client = self.client.clone();
        let retry_config = self.retry_config.clone();

        retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                async move {
                    let response = client.get(&url).await?;

                    if response.status().as_u16() == 404 {
                        anyhow::bail!("Author not found on OpenLibrary");
                    }

                    if !response.status().is_success() {
                        anyhow::bail!("OpenLibrary get author failed with status: {}", response.status());
                    }

                    let author: OpenLibraryAuthor = response
                        .json()
                        .await
                        .context("Failed to parse OpenLibrary author")?;

                    Ok(author)
                }
            },
            &retry_config,
            "openlibrary_get_author",
        )
        .await
    }

    /// Get work (book) details from OpenLibrary
    pub async fn get_work(&self, work_id: &str) -> Result<AudiobookDetails> {
        debug!("Fetching work {} from OpenLibrary", work_id);

        // Normalize work ID
        let normalized_id = if work_id.starts_with("OL") {
            work_id.to_string()
        } else if work_id.starts_with("/works/") {
            work_id.replace("/works/", "")
        } else {
            work_id.to_string()
        };

        let url = format!("https://openlibrary.org/works/{}.json", normalized_id);
        let client = self.client.clone();
        let retry_config = self.retry_config.clone();

        #[derive(Deserialize)]
        struct WorkResponse {
            title: String,
            subtitle: Option<String>,
            description: Option<OpenLibraryText>,
            subjects: Option<Vec<String>>,
            covers: Option<Vec<i64>>,
            authors: Option<Vec<AuthorRef>>,
            first_publish_date: Option<String>,
        }

        #[derive(Deserialize)]
        struct AuthorRef {
            author: AuthorKey,
        }

        #[derive(Deserialize)]
        struct AuthorKey {
            key: String,
        }

        let work: WorkResponse = retry_async(
            || {
                let url = url.clone();
                let client = client.clone();
                async move {
                    let response = client.get(&url).await?;

                    if !response.status().is_success() {
                        anyhow::bail!("OpenLibrary get work failed with status: {}", response.status());
                    }

                    response
                        .json()
                        .await
                        .context("Failed to parse OpenLibrary work")
                }
            },
            &retry_config,
            "openlibrary_get_work",
        )
        .await?;

        // Get first author details
        let (author_name, author_id) = if let Some(ref authors) = work.authors {
            if let Some(first_author) = authors.first() {
                let author_key = first_author.author.key.clone();
                let author = self.get_author(&author_key).await.ok();
                (
                    author.as_ref().map(|a| a.name.clone()).unwrap_or_else(|| "Unknown".to_string()),
                    Some(author_key),
                )
            } else {
                ("Unknown".to_string(), None)
            }
        } else {
            ("Unknown".to_string(), None)
        };

        // Get cover URL
        let cover_url = work.covers.and_then(|covers| {
            covers.into_iter().next().map(|id| {
                format!("https://covers.openlibrary.org/b/id/{}-L.jpg", id)
            })
        });

        // Parse year from first_publish_date
        let year = work.first_publish_date.and_then(|d| {
            d.split_whitespace()
                .last()
                .and_then(|y| y.parse().ok())
        });

        Ok(AudiobookDetails {
            title: work.title,
            subtitle: work.subtitle,
            author_name,
            author_id,
            year,
            description: work.description.map(|d| d.value().to_string()),
            cover_url,
            openlibrary_id: Some(normalized_id),
            isbn: None,
            subjects: work.subjects.unwrap_or_default(),
            publishers: vec![],
            language: None,
        })
    }

    /// Get cover image URL for a work
    pub fn get_cover_url(cover_id: i64, size: &str) -> String {
        // Size can be S, M, or L
        format!("https://covers.openlibrary.org/b/id/{}-{}.jpg", cover_id, size)
    }

    /// Get author photo URL
    pub fn get_author_photo_url(photo_id: i64, size: &str) -> String {
        format!("https://covers.openlibrary.org/a/id/{}-{}.jpg", photo_id, size)
    }
}

impl Default for AudiobookMetadataClient {
    fn default() -> Self {
        Self::new()
    }
}

impl OpenLibraryWork {
    /// Get primary author name
    pub fn primary_author(&self) -> Option<String> {
        self.author_names.as_ref().and_then(|a| a.first().cloned())
    }

    /// Get cover URL in specified size (S, M, L)
    pub fn cover_url(&self, size: &str) -> Option<String> {
        self.cover_i.map(|id| AudiobookMetadataClient::get_cover_url(id, size))
    }
}
