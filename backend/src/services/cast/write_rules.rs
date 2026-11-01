//! Content rules for Cast rows, applied by the write input transform to every
//! create and update regardless of the path (generated mutation, internal
//! discovery upsert, seed). They hold for the stored values, so internal
//! writers that already produce valid rows are unaffected.
//!
//! Errors surface to the caller as the mutation's `error`; nothing here may
//! rely on a deferred hook, whose failures are only logged.

use async_graphql::{Error, ErrorExtensions, Result};

use super::{CastDeviceType, CastService};
use crate::graphql::entities::{
    CastDevice, CreateCastDeviceInput, CreateCastSettingInput, UpdateCastDeviceInput,
    UpdateCastSettingInput,
};

pub const DEFAULT_PORT: i32 = 8009;
const QUALITIES: [&str; 5] = ["original", "2160p", "1080p", "720p", "480p"];

fn invalid(message: impl Into<String>) -> Error {
    Error::new(message.into()).extend_with(|_, extensions| extensions.set("code", "BAD_USER_INPUT"))
}

/// `CHROMECAST`, `CHROMECAST_AUDIO`, `DLNA_RENDERER` or `UNKNOWN` (a discovered
/// receiver whose model was not recognised), in canonical upper case.
fn normalized_device_type(value: &str) -> Result<String> {
    let canonical = value.trim().to_ascii_uppercase();
    let known = [
        CastDeviceType::Chromecast,
        CastDeviceType::ChromecastAudio,
        CastDeviceType::DlnaRenderer,
        CastDeviceType::Unknown,
    ];
    known
        .iter()
        .find(|device_type| device_type.as_str() == canonical)
        .map(|device_type| device_type.as_str().to_string())
        .ok_or_else(|| {
            invalid(
                "Unsupported Cast device type; expected CHROMECAST, CHROMECAST_AUDIO, \
                 DLNA_RENDERER, or UNKNOWN",
            )
        })
}

fn validated_target(address: &str, port: i32) -> Result<String> {
    CastService::validate_target(address, port).map_err(|error| invalid(error.to_string()))?;
    Ok(address.trim().to_string())
}

pub fn prepare_device_create(input: &mut CreateCastDeviceInput) -> Result<()> {
    input.address = validated_target(&input.address, input.port)?;
    input.device_type = normalized_device_type(&input.device_type)?;
    if input.name.trim().is_empty() {
        input.name = format!("Cast Device ({})", input.address);
    } else {
        input.name = input.name.trim().to_string();
    }
    Ok(())
}

/// Only a write that sets the address or port is checked against the LAN
/// ranges, using the values the row will hold, so toggling a favourite never
/// fails on a legacy row.
pub fn prepare_device_update(
    existing: Option<&CastDevice>,
    input: &mut UpdateCastDeviceInput,
) -> Result<()> {
    if input.address.is_some() || input.port.is_some() {
        let address = input
            .address
            .clone()
            .or_else(|| existing.map(|row| row.address.clone()))
            .ok_or_else(|| invalid("Cast address is required"))?;
        let port = input
            .port
            .or_else(|| existing.map(|row| row.port))
            .unwrap_or(DEFAULT_PORT);
        let address = validated_target(&address, port)?;
        if input.address.is_some() {
            input.address = Some(address);
        }
    }
    if let Some(device_type) = input.device_type.as_deref() {
        input.device_type = Some(normalized_device_type(device_type)?);
    }
    if let Some(name) = input.name.as_deref() {
        if name.trim().is_empty() {
            return Err(invalid("Cast device name cannot be blank"));
        }
        input.name = Some(name.trim().to_string());
    }
    Ok(())
}

fn validate_interval(seconds: i32) -> Result<()> {
    if (5..=86_400).contains(&seconds) {
        Ok(())
    } else {
        Err(invalid(
            "Discovery interval must be between 5 and 86400 seconds",
        ))
    }
}

fn validate_volume(volume: f64) -> Result<()> {
    CastService::validate_volume(volume)
        .map(drop)
        .map_err(|error| invalid(error.to_string()))
}

fn normalized_quality(quality: &str) -> Result<String> {
    let canonical = quality.trim().to_ascii_lowercase();
    if QUALITIES.contains(&canonical.as_str()) {
        Ok(canonical)
    } else {
        Err(invalid(
            "Preferred quality must be original, 2160p, 1080p, 720p, or 480p",
        ))
    }
}

pub fn prepare_setting_create(input: &mut CreateCastSettingInput) -> Result<()> {
    validate_interval(input.discovery_interval_seconds)?;
    validate_volume(input.default_volume)?;
    if let Some(quality) = input.preferred_quality.as_deref() {
        input.preferred_quality = Some(normalized_quality(quality)?);
    }
    Ok(())
}

pub fn prepare_setting_update(input: &mut UpdateCastSettingInput) -> Result<()> {
    if let Some(seconds) = input.discovery_interval_seconds {
        validate_interval(seconds)?;
    }
    if let Some(volume) = input.default_volume {
        validate_volume(volume)?;
    }
    if let Some(Some(quality)) = input.preferred_quality.as_ref() {
        input.preferred_quality = Some(Some(normalized_quality(quality)?));
    }
    Ok(())
}
