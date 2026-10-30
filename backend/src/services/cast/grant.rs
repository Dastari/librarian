//! Bounded, revocable media/session/receiver-bound grants for Cast fetches.

use anyhow::{Context, Result};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, KeyInit, Mac};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;
const AUDIENCE: &str = "librarian-cast";
const VERSION: &str = "v2";

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CastGrantClaims {
    pub audience: String,
    pub session_id: String,
    pub media_file_id: String,
    pub user_id: String,
    pub receiver_address: String,
    pub receiver_device_id: String,
    pub receiver_port: u16,
    /// Actual authenticated issuing session, never an access/refresh credential.
    pub issuer_session_id: String,
    pub issuer_roles: Vec<String>,
    pub expires_at: i64,
}

impl CastGrantClaims {
    /// Allow seeks throughout a known title plus two hours of pauses. Unknown
    /// durations get four hours; all grants stop after 24 hours. This is a
    /// bounded receiver capability, independent of access JWT expiration. Every
    /// fetch and monitor checkpoint must recheck the persisted issuing session.
    pub fn expiry_for_duration(now_unix: i64, duration_seconds: Option<i32>) -> i64 {
        let duration = i64::from(duration_seconds.unwrap_or(0).max(0));
        now_unix.saturating_add((duration + 2 * 60 * 60).clamp(4 * 60 * 60, 24 * 60 * 60))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new(
        session_id: String,
        media_file_id: String,
        user_id: String,
        receiver_address: String,
        expires_at: i64,
        issuer_session_id: String,
        issuer_roles: Vec<String>,
        receiver_device_id: String,
        receiver_port: u16,
    ) -> Self {
        Self {
            audience: AUDIENCE.to_string(),
            session_id,
            media_file_id,
            user_id,
            receiver_address,
            expires_at,
            issuer_session_id,
            issuer_roles,
            receiver_device_id,
            receiver_port,
        }
    }
}

#[derive(Clone)]
pub struct CastGrantSigner {
    key: [u8; 32],
}

impl std::fmt::Debug for CastGrantSigner {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CastGrantSigner")
            .field("key", &"[REDACTED]")
            .finish()
    }
}

impl CastGrantSigner {
    /// Derive a purpose-separated Cast grant key from the application's signing
    /// secret. The user's access/refresh token is never used as a Cast credential.
    pub fn derive(application_secret: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"librarian-cast-grant-v2\0");
        hasher.update(application_secret);
        Self {
            key: hasher.finalize().into(),
        }
    }

    pub fn sign(&self, claims: &CastGrantClaims) -> Result<String> {
        let payload = serde_json::to_vec(claims).context("Failed to encode Cast grant")?;
        let payload = URL_SAFE_NO_PAD.encode(payload);
        let signed = format!("{VERSION}.{payload}");
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .map_err(|_| anyhow::anyhow!("Cast grant signing key is invalid"))?;
        mac.update(signed.as_bytes());
        let signature = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
        Ok(format!("{signed}.{signature}"))
    }

    pub fn verify(&self, grant: &str, now_unix: i64) -> Result<CastGrantClaims> {
        let mut parts = grant.split('.');
        let version = parts.next().context("Cast grant version is missing")?;
        let payload = parts.next().context("Cast grant payload is missing")?;
        let signature = parts.next().context("Cast grant signature is missing")?;
        if version != VERSION || parts.next().is_some() {
            anyhow::bail!("Unsupported Cast grant format");
        }
        let signature = URL_SAFE_NO_PAD
            .decode(signature)
            .context("Cast grant signature is malformed")?;
        let signed = format!("{version}.{payload}");
        let mut mac = HmacSha256::new_from_slice(&self.key)
            .map_err(|_| anyhow::anyhow!("Cast grant signing key is invalid"))?;
        mac.update(signed.as_bytes());
        mac.verify_slice(&signature)
            .context("Cast grant signature is invalid")?;

        let payload = URL_SAFE_NO_PAD
            .decode(payload)
            .context("Cast grant payload is malformed")?;
        let claims: CastGrantClaims =
            serde_json::from_slice(&payload).context("Cast grant claims are malformed")?;
        if claims.audience != AUDIENCE {
            anyhow::bail!("Cast grant audience is invalid");
        }
        // Required fields and v2 reject old grants without revocable authority.
        if claims.issuer_session_id.is_empty()
            || claims.issuer_roles.is_empty()
            || claims.user_id.is_empty()
            || claims.session_id.is_empty()
            || claims.media_file_id.is_empty()
            || claims.receiver_address.is_empty()
            || claims.receiver_device_id.is_empty()
            || claims.receiver_port == 0
        {
            anyhow::bail!("Cast grant authority is missing");
        }
        if claims.expires_at <= now_unix {
            anyhow::bail!("Cast grant has expired");
        }
        Ok(claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims() -> CastGrantClaims {
        CastGrantClaims::new(
            "session-1".to_string(),
            "media-1".to_string(),
            "user-1".to_string(),
            "192.168.1.10".to_string(),
            2_000,
            "issuer-session-1".to_string(),
            vec!["member".to_string()],
            "device-1".to_string(),
            8009,
        )
    }

    #[test]
    fn long_play_grants_survive_movie_seeks_but_have_a_hard_ceiling() {
        let signer = CastGrantSigner::derive(b"application-secret");
        let mut claims = claims();
        claims.expires_at = CastGrantClaims::expiry_for_duration(1_000, Some(3 * 60 * 60));
        let token = signer.sign(&claims).unwrap();
        assert!(signer.verify(&token, 1_000 + 3 * 60 * 60).is_ok());
        assert!(signer.verify(&token, claims.expires_at).is_err());
        assert_eq!(CastGrantClaims::expiry_for_duration(0, None), 4 * 60 * 60);
        assert_eq!(
            CastGrantClaims::expiry_for_duration(0, Some(-1)),
            4 * 60 * 60
        );
        assert_eq!(
            CastGrantClaims::expiry_for_duration(0, Some(i32::MAX)),
            24 * 60 * 60
        );
    }

    #[test]
    fn grants_without_issuing_authority_fail_closed() {
        let signer = CastGrantSigner::derive(b"application-secret");
        let mut claims = claims();
        claims.issuer_session_id.clear();
        assert!(
            signer
                .verify(&signer.sign(&claims).unwrap(), 1_000)
                .is_err()
        );
        claims.issuer_session_id = "issuer-session-1".into();
        claims.issuer_roles.clear();
        assert!(
            signer
                .verify(&signer.sign(&claims).unwrap(), 1_000)
                .is_err()
        );
        let legacy = signer.sign(&claims).unwrap().replacen("v2.", "v1.", 1);
        assert!(signer.verify(&legacy, 1_000).is_err());

        // Even a correctly signed v2 payload cannot omit the new bindings.
        claims.issuer_roles = vec!["member".into()];
        let mut payload = serde_json::to_value(&claims).unwrap();
        payload.as_object_mut().unwrap().remove("issuerSessionId");
        let signed = format!(
            "v2.{}",
            URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap())
        );
        let mut mac = HmacSha256::new_from_slice(&signer.key).unwrap();
        mac.update(signed.as_bytes());
        let missing_authority = format!(
            "{signed}.{}",
            URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
        );
        assert!(signer.verify(&missing_authority, 1_000).is_err());
    }

    #[test]
    fn grant_round_trips_and_is_bound_to_signature_and_expiry() {
        let signer = CastGrantSigner::derive(b"application-secret");
        let token = signer.sign(&claims()).unwrap();
        assert_eq!(signer.verify(&token, 1_000).unwrap(), claims());
        assert!(signer.verify(&token, 2_000).is_err());

        let mut tampered = token.into_bytes();
        let last = tampered.len() - 1;
        tampered[last] = if tampered[last] == b'A' { b'B' } else { b'A' };
        assert!(
            signer
                .verify(std::str::from_utf8(&tampered).unwrap(), 1_000)
                .is_err()
        );
        assert!(
            CastGrantSigner::derive(b"different-secret")
                .verify(&signer.sign(&claims()).unwrap(), 1_000,)
                .is_err()
        );
    }
}
