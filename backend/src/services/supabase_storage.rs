//! Supabase Storage client for artwork and thumbnails

use anyhow::Result;
use reqwest::Client;

/// Supabase Storage client
pub struct StorageClient {
    base_url: String,
    service_key: String,
    client: Client,
}

impl StorageClient {
    pub fn new(base_url: String, service_key: String) -> Self {
        Self {
            base_url,
            service_key,
            client: Client::new(),
        }
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

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.service_key))
            .header("Content-Type", content_type)
            .body(content.to_vec())
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(format!(
                "{}/storage/v1/object/public/{}/{}",
                self.base_url, bucket, path
            ))
        } else {
            anyhow::bail!("Failed to upload file: {}", resp.status())
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
