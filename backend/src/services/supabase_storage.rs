//! Supabase Storage client for artwork and thumbnails

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Supabase Storage client
#[derive(Clone)]
pub struct StorageClient {
    base_url: String,
    service_key: String,
    client: Client,
}

#[derive(Debug, Serialize)]
struct CreateBucketRequest {
    id: String,
    name: String,
    public: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    file_size_limit: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allowed_mime_types: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct BucketInfo {
    id: String,
    name: String,
    public: bool,
}

impl StorageClient {
    pub fn new(base_url: String, service_key: String) -> Self {
        Self {
            base_url,
            service_key,
            client: Client::new(),
        }
    }

    /// Create a storage bucket if it doesn't already exist
    pub async fn ensure_bucket(&self, bucket_name: &str, public: bool) -> Result<()> {
        // First check if bucket exists
        let list_url = format!("{}/storage/v1/bucket", self.base_url);
        
        let resp = self
            .client
            .get(&list_url)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("apikey", &self.service_key)
            .send()
            .await
            .context("Failed to list buckets")?;

        if resp.status().is_success() {
            let buckets: Vec<BucketInfo> = resp.json().await.unwrap_or_default();
            if buckets.iter().any(|b| b.id == bucket_name || b.name == bucket_name) {
                debug!(bucket = %bucket_name, "Bucket already exists");
                return Ok(());
            }
        }

        // Bucket doesn't exist, create it
        info!(bucket = %bucket_name, public = %public, "Creating storage bucket");
        
        let create_url = format!("{}/storage/v1/bucket", self.base_url);
        let request = CreateBucketRequest {
            id: bucket_name.to_string(),
            name: bucket_name.to_string(),
            public,
            file_size_limit: Some(50 * 1024 * 1024), // 50MB limit for artwork
            allowed_mime_types: Some(vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/gif".to_string(),
                "image/webp".to_string(),
            ]),
        };

        let resp = self
            .client
            .post(&create_url)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("apikey", &self.service_key)
            .json(&request)
            .send()
            .await
            .context("Failed to create bucket")?;

        if resp.status().is_success() {
            info!(bucket = %bucket_name, "Bucket created successfully");
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            
            // Check if it's a "already exists" error (which is fine)
            if body.contains("already exists") || body.contains("Duplicate") {
                debug!(bucket = %bucket_name, "Bucket already exists (race condition)");
                Ok(())
            } else {
                warn!(bucket = %bucket_name, status = %status, body = %body, "Failed to create bucket");
                anyhow::bail!("Failed to create bucket: {} - {}", status, body)
            }
        }
    }

    /// Get the public URL for an object
    pub fn public_url(&self, bucket: &str, path: &str) -> String {
        format!(
            "{}/storage/v1/object/public/{}/{}",
            self.base_url, bucket, path
        )
    }

    /// Upload a file to a bucket
    pub async fn upload(
        &self,
        bucket: &str,
        path: &str,
        content: &[u8],
        content_type: &str,
    ) -> Result<String> {
        let url = format!(
            "{}/storage/v1/object/{}/{}",
            self.base_url, bucket, path
        );

        debug!(url = %url, size = content.len(), content_type = %content_type, "Uploading to Supabase Storage");

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("apikey", &self.service_key)
            .header("Content-Type", content_type)
            .header("x-upsert", "true") // Allow overwriting existing files
            .body(content.to_vec())
            .send()
            .await
            .context("Failed to send upload request")?;

        let status = resp.status();
        if status.is_success() {
            let public_url = format!(
                "{}/storage/v1/object/public/{}/{}",
                self.base_url, bucket, path
            );
            debug!(public_url = %public_url, "Upload successful");
            Ok(public_url)
        } else {
            let body = resp.text().await.unwrap_or_default();
            warn!(
                status = %status,
                body = %body,
                bucket = %bucket,
                path = %path,
                "Failed to upload to Supabase Storage"
            );
            anyhow::bail!("Failed to upload file: {} - {}", status, body)
        }
    }

    /// Generate a signed URL for private access
    pub async fn create_signed_url(
        &self,
        bucket: &str,
        path: &str,
        expires_in: i64,
    ) -> Result<String> {
        let url = format!(
            "{}/storage/v1/object/sign/{}/{}",
            self.base_url, bucket, path
        );

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .json(&serde_json::json!({ "expiresIn": expires_in }))
            .send()
            .await?;

        if resp.status().is_success() {
            let body: serde_json::Value = resp.json().await?;
            let signed_url = body["signedURL"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing signedURL in response"))?;
            Ok(format!("{}{}", self.base_url, signed_url))
        } else {
            anyhow::bail!("Failed to create signed URL: {}", resp.status())
        }
    }

    /// Delete a file from a bucket
    pub async fn delete(&self, bucket: &str, path: &str) -> Result<()> {
        let url = format!(
            "{}/storage/v1/object/{}/{}",
            self.base_url, bucket, path
        );

        let resp = self
            .client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("Failed to delete file: {}", resp.status())
        }
    }
}
