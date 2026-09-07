//! Stable public GraphQL errors and correlation-safe internal diagnostics.

use async_graphql::ErrorExtensions;
use uuid::Uuid;

pub(crate) const INTERNAL_ERROR_CODE: &str = "INTERNAL_ERROR";
pub(crate) const SERVICE_UNAVAILABLE_CODE: &str = "SERVICE_UNAVAILABLE";

pub(crate) fn public_error(code: &'static str, message: impl Into<String>) -> async_graphql::Error {
    async_graphql::Error::new(message.into()).extend_with(|_, extensions| {
        extensions.set("code", code);
    })
}

pub(crate) fn service_unavailable(service: &'static str) -> async_graphql::Error {
    public_error(
        SERVICE_UNAVAILABLE_CODE,
        format!("{service} service is temporarily unavailable"),
    )
}

/// Sanitize an internal failure for legacy payloads that expose an `error:
/// String` field rather than a top-level GraphQL error with extensions.
pub(crate) fn internal_message(
    operation: &'static str,
    source: &(dyn std::fmt::Display + Send + Sync),
) -> String {
    let correlation_id = Uuid::new_v4().to_string();
    tracing::error!(
        operation,
        correlation_id,
        error = %source,
        "Internal GraphQL payload operation failed"
    );
    format!("INTERNAL_ERROR: operation failed (reference {correlation_id})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_internal_message_does_not_expose_the_source() {
        let source =
            anyhow::anyhow!("database /secret/path failed: select password_hash from users");
        let message = internal_message("test.operation", &source);
        assert!(message.starts_with(INTERNAL_ERROR_CODE));
        assert!(!message.contains("secret"));
        assert!(!message.contains("SELECT"));
    }
}
