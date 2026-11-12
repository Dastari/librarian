//! Devices: check-in, sign-out and the hooks that keep sessions consistent (design.md Q133).

use async_graphql::ErrorExtensions;
use graphql_orm::graphql::filters::{DateFilter, StringFilter};
use graphql_orm::graphql::orm::{
    ChangeAction, MutationContext, MutationEvent, MutationPhase, TransactionMode,
};

use crate::db::Database;
use crate::graphql::AuthUser;
use crate::graphql::entities::*;

use super::now;

/// What the server saw of the request: the client address (through a trusted proxy's
/// `X-Forwarded-For` only) and the user agent header. Request data, never a GraphQL field.
#[derive(Clone, Debug, Default)]
pub struct RequestClientInfo {
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

/// What a client says about itself when it checks in.
#[derive(Clone, Debug, Default)]
pub struct CheckIn {
    pub client_id: String,
    pub name: Option<String>,
    pub form_factor: Option<String>,
    pub platform: Option<String>,
    pub product: Option<String>,
    pub app_version: Option<String>,
}

/// A rough reading of a user agent, used when the client sends nothing better.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UserAgentFacts {
    pub product: Option<String>,
    pub platform: Option<String>,
    pub form_factor: &'static str,
}

/// Browser, platform and form factor from a user agent. Order matters: Edge and Opera say
/// Chrome, Chrome says Safari, iPadOS 13+ says Macintosh (the client corrects that one with
/// touch points), and TV platforms say Linux.
pub fn describe_user_agent(ua: &str) -> UserAgentFacts {
    let has = |needle: &str| ua.contains(needle);
    let tv = [
        ("Web0S", "webOS"),
        ("webOS", "webOS"),
        ("Tizen", "Tizen"),
        ("AFT", "Fire TV"),
        ("CrKey", "Chromecast"),
        ("Android TV", "Android TV"),
        ("GoogleTV", "Google TV"),
        ("AppleTV", "tvOS"),
        ("SMART-TV", "Smart TV"),
        ("SmartTV", "Smart TV"),
        ("BRAVIA", "Android TV"),
        ("PlayStation", "PlayStation"),
        ("Xbox", "Xbox"),
    ];
    let tv_platform = tv
        .iter()
        .find(|(needle, _)| has(needle))
        .map(|(_, platform)| *platform);
    let platform = if let Some(platform) = tv_platform {
        Some(platform)
    } else if has("iPad") {
        Some("iPadOS")
    } else if has("iPhone") || has("iPod") {
        Some("iOS")
    } else if has("Android") {
        Some("Android")
    } else if has("CrOS") {
        Some("ChromeOS")
    } else if has("Mac OS X") || has("Macintosh") {
        Some("macOS")
    } else if has("Windows") {
        Some("Windows")
    } else if has("Linux") {
        Some("Linux")
    } else {
        None
    };
    let product = if has("Edg/") || has("EdgA/") || has("EdgiOS/") {
        Some("Edge")
    } else if has("OPR/") || has("Opera") {
        Some("Opera")
    } else if has("SamsungBrowser") {
        Some("Samsung Internet")
    } else if has("Firefox/") || has("FxiOS/") {
        Some("Firefox")
    } else if has("Chrome/") || has("CriOS/") || has("Chromium/") {
        Some("Chrome")
    } else if has("Safari/") || has("AppleWebKit/") {
        Some("Safari")
    } else {
        None
    };
    let form_factor = if tv_platform.is_some() {
        "tv"
    } else if has("iPad") || (has("Android") && !has("Mobile")) || has("Tablet") {
        "tablet"
    } else if has("Mobi") || has("iPhone") || has("iPod") {
        "phone"
    } else if platform.is_some() {
        "desktop"
    } else {
        "unknown"
    };
    UserAgentFacts {
        product: product.map(str::to_owned),
        platform: platform.map(str::to_owned),
        form_factor,
    }
}

/// "Safari on iPad", "Chrome on Windows", "TV": the default name until someone renames it.
pub fn default_name(product: Option<&str>, platform: Option<&str>, form_factor: &str) -> String {
    let place = match (platform, form_factor) {
        (_, "tv") => {
            return match platform {
                Some(p) if p.ends_with("TV") => p.to_owned(),
                Some(p) => format!("{p} TV"),
                None => "TV".to_owned(),
            };
        }
        (Some("iPadOS"), _) => "iPad",
        (Some("iOS"), _) => "iPhone",
        (Some("macOS"), _) => "Mac",
        (Some(platform), _) => platform,
        (None, "phone") => "phone",
        (None, "tablet") => "tablet",
        (None, _) => "this device",
    };
    match product {
        Some(product) => format!("{product} on {place}"),
        None => match platform {
            Some(_) => place.to_owned(),
            None => "Unknown device".to_owned(),
        },
    }
}

fn eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_owned()),
        ..Default::default()
    }
}

fn db_error(error: impl std::fmt::Display) -> async_graphql::Error {
    async_graphql::Error::new(error.to_string())
}

fn bad_input(message: &str) -> async_graphql::Error {
    async_graphql::Error::new(message)
        .extend_with(|_, extensions| extensions.set("code", "BAD_USER_INPUT"))
}

fn clip(value: Option<String>, max: usize) -> Option<String> {
    value
        .map(|value| value.trim().chars().take(max).collect::<String>())
        .filter(|value| !value.is_empty())
}

const FORM_FACTORS: &[&str] = &["tv", "phone", "tablet", "desktop", "unknown"];

/// Add or refresh the caller's device for `client_id`. The user agent and address are what
/// the server saw; the rest is what the client said. A renamed device keeps its name.
pub async fn check_in(
    db: &Database,
    user: &AuthUser,
    session_id: Option<String>,
    seen: &RequestClientInfo,
    input: CheckIn,
) -> async_graphql::Result<Device> {
    let client_id = input.client_id.trim().to_owned();
    if client_id.is_empty()
        || client_id.len() > 128
        || !client_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(bad_input(
            "A device id is 1 to 128 letters, digits, dashes or underscores",
        ));
    }
    let user_agent = clip(seen.user_agent.clone(), 512);
    let facts = describe_user_agent(user_agent.as_deref().unwrap_or_default());
    let form_factor = input
        .form_factor
        .filter(|value| FORM_FACTORS.contains(&value.as_str()))
        .unwrap_or_else(|| facts.form_factor.to_owned());
    let platform = clip(input.platform, 64).or(facts.platform);
    let product = clip(input.product, 64).or(facts.product);
    let app_version = clip(input.app_version, 64);
    let name = clip(input.name, 80)
        .unwrap_or_else(|| default_name(product.as_deref(), platform.as_deref(), &form_factor));
    let now = now();

    let mut tx = MutationContext::begin(db, TransactionMode::StateMachine)
        .await
        .map_err(db_error)?;
    let existing = tx
        .query::<Device>()
        .filter(DeviceWhereInput {
            user_id: Some(eq(&user.user_id)),
            client_id: Some(eq(&client_id)),
            ..Default::default()
        })
        .fetch_one()
        .await
        .map_err(db_error)?;
    let device = match existing {
        Some(device) => tx
            .update_by_id::<Device>(
                &device.id,
                UpdateDeviceInput {
                    name: (!device.name_is_custom).then_some(name),
                    form_factor: Some(form_factor),
                    platform: Some(platform),
                    product: Some(product),
                    app_version: Some(app_version),
                    user_agent: Some(user_agent),
                    last_ip: Some(seen.ip.clone()),
                    last_seen_at: Some(now),
                    session_id: Some(session_id.or(device.session_id.clone())),
                    signed_out_at: Some(None),
                    ..Default::default()
                },
            )
            .await
            .map_err(db_error)?
            .ok_or_else(|| db_error("The device was removed while checking in"))?,
        None => tx
            .insert::<Device>(CreateDeviceInput {
                user_id: user.user_id.clone(),
                client_id,
                name,
                name_is_custom: false,
                form_factor,
                platform,
                product,
                app_version,
                user_agent,
                last_ip: seen.ip.clone(),
                last_seen_at: now,
                session_id,
                signed_out_at: None,
            })
            .await
            .map_err(db_error)?,
    };
    tx.commit_and_emit().await.map_err(db_error)?;
    Ok(device)
}

/// Revoke every live refresh token of one login session, inside `tx`. Access tokens of
/// that session stop working at their next request (session authority is rechecked).
pub(crate) async fn revoke_login_session(
    tx: &mut MutationContext<'_>,
    session_id: &str,
    reason: &str,
) -> async_graphql::Result<i64> {
    tx.update_where::<RefreshToken>(
        RefreshTokenWhereInput {
            session_id: Some(eq(session_id)),
            revoked_at: Some(DateFilter {
                is_null: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        },
        UpdateRefreshTokenInput {
            revoked_at: Some(Some(now())),
            revocation_reason: Some(Some(reason.to_owned())),
            ..Default::default()
        },
    )
    .await
    .map_err(db_error)
}

/// End the open sessions of a device, except `keep`.
async fn end_device_sessions(
    tx: &mut MutationContext<'_>,
    device_id: &str,
    keep: Option<&str>,
) -> async_graphql::Result<i64> {
    tx.update_where::<PlaybackSession>(
        PlaybackSessionWhereInput {
            device_id: Some(eq(device_id)),
            id: keep.map(|keep| StringFilter {
                ne: Some(keep.to_owned()),
                ..Default::default()
            }),
            ended_at: Some(DateFilter {
                is_null: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        },
        UpdatePlaybackSessionInput {
            ended_at: Some(Some(now())),
            is_playing: Some(false),
            state: Some(Some("stopped".into())),
            ..Default::default()
        },
    )
    .await
    .map_err(db_error)
}

/// Sign a device out: its login session is revoked and its playback ends. The device stays
/// listed, marked signed out, until it checks in again or is forgotten.
pub async fn sign_out(db: &Database, device: &Device) -> async_graphql::Result<i64> {
    let mut tx = MutationContext::begin(db, TransactionMode::StateMachine)
        .await
        .map_err(db_error)?;
    let revoked = match &device.session_id {
        Some(session_id) => revoke_login_session(&mut tx, session_id, "AdminRevoked").await?,
        None => 0,
    };
    end_device_sessions(&mut tx, &device.id, None).await?;
    tx.update_by_id::<Device>(
        &device.id,
        UpdateDeviceInput {
            signed_out_at: Some(Some(now())),
            ..Default::default()
        },
    )
    .await
    .map_err(db_error)?;
    tx.commit_and_emit().await.map_err(db_error)?;
    Ok(revoked)
}

/// PlaybackSession After Created: a device plays one thing at a time, so starting a session
/// ends the device's other open ones (a closed tab that never said goodbye).
pub(crate) async fn end_other_device_sessions(
    tx: &mut MutationContext<'_>,
    event: &MutationEvent,
) -> async_graphql::Result<()> {
    if event.phase != MutationPhase::After || event.action != ChangeAction::Created {
        return Ok(());
    }
    let Some(session) = event.after::<PlaybackSession>()? else {
        return Ok(());
    };
    if let Some(device_id) = &session.device_id {
        end_device_sessions(tx, device_id, Some(&session.id)).await?;
    }
    Ok(())
}

/// Device Before Deleted ("Forget"): its login session is revoked and its sessions let go of
/// it. Plays keep their device name.
pub(crate) async fn forget_device(
    tx: &mut MutationContext<'_>,
    event: &MutationEvent,
) -> async_graphql::Result<()> {
    let Some(device) = event.before::<Device>()? else {
        return Ok(());
    };
    if let Some(session_id) = &device.session_id {
        revoke_login_session(tx, session_id, "AdminRevoked").await?;
    }
    end_device_sessions(tx, &device.id, None).await?;
    tx.update_where::<PlaybackSession>(
        PlaybackSessionWhereInput {
            device_id: Some(eq(&device.id)),
            ..Default::default()
        },
        UpdatePlaybackSessionInput {
            device_id: Some(None),
            ..Default::default()
        },
    )
    .await
    .map_err(db_error)?;
    Ok(())
}
