//! Real typed user/session records and generated mutations; receiver status is
//! an explicit fake only at the transport boundary. No network receiver needed.
use super::*;
use crate::db::Database;
use crate::graphql::entities::*;
use crate::services::auth::{AuthConfig, RegisterInput};
use crate::services::cast::{CastService, CastServiceConfig};
use crate::services::graphql::GraphqlServiceConfig;
use crate::services::{DatabaseServiceConfig, ServicesManager};
use async_graphql::{EmptySubscription, MergedObject, Schema};
use graphql_orm::graphql::auth::AuthSubject;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TestQuery;
#[Object]
impl TestQuery {
    async fn ping(&self) -> bool {
        true
    }
}
#[derive(Default, MergedObject)]
struct TestMutations(
    LibraryMutations,
    MediaFileMutations,
    CastDeviceMutations,
    CastSessionMutations,
    CastMutations,
);
type TestSchema = Schema<TestQuery, TestMutations, EmptySubscription>;

struct Fixture {
    _temp: tempfile::TempDir,
    services: Arc<ServicesManager>,
    db: Database,
    schema: TestSchema,
    authority: SessionAuthority,
    refresh_token: String,
    claims: CastGrantClaims,
    library_id: String,
}
impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let services = ServicesManager::builder()
            .add_service(DatabaseServiceConfig {
                database_url: format!("sqlite://{}", temp.path().join("cast.db").display()),
                connect_timeout: std::time::Duration::from_secs(5),
            })
            .add_service(AuthConfig::for_tests())
            .add_service(GraphqlServiceConfig { server_port: 0 })
            .add_service(CastServiceConfig {
                auto_discovery: false,
                ..Default::default()
            })
            .start()
            .await
            .unwrap();
        let auth = services.get_auth().await.unwrap();
        let login = auth
            .register(RegisterInput {
                email: "cast-owner@example.test".into(),
                name: "Cast Owner".into(),
                password: "cast-fixture-password-only".into(),
                invite_token: None,
            })
            .await
            .unwrap();
        let authority = SessionAuthority(
            auth.authenticate_access_token(&login.tokens.access_token)
                .await
                .unwrap(),
        );
        let db = services.get_database().await.unwrap().pool().clone();
        let schema = Schema::build(TestQuery, TestMutations::default(), EmptySubscription)
            .data(db.clone())
            .data(services.clone())
            .finish();
        // Fixture creation goes through generated mutations with explicit trusted
        // setup authority. Actual grant admission uses the real registered user
        // and persisted issuing RefreshToken; no missing-authority bypass exists.
        let library_id = create(&schema, "Library", json!({
            "userId": authority.0.user_id, "name":"Cast films", "path":"/cast-fixture",
            "libraryType":"movies", "autoScan":false,"autoOrganize":false,
            "namingPattern":"", "scanIntervalMinutes":60,"watchForChanges":false,"scanning":false
        })).await;
        let file_id = create(
            &schema,
            "MediaFile",
            json!({
                "libraryId":library_id,"path":"/cast-fixture/movie.mp4","size":1000,
                "duration":10800,"container":"mp4","videoCodec":"h264","audioCodec":"aac",
                "isHdr":false,"addedAt":chrono::Utc::now().to_rfc3339()
            }),
        )
        .await;
        let device_id = create(
            &schema,
            "CastDevice",
            json!({
                "name":"Test receiver", "address":"192.168.1.50","port":8009,
                "deviceType":"CHROMECAST", "isFavorite":false,"isManual":true,
                "enabled":true,"playbackSupported":true
            }),
        )
        .await;
        let expires_at =
            CastGrantClaims::expiry_for_duration(chrono::Utc::now().timestamp(), Some(10800));
        let session_id = create(&schema, "CastSession", json!({
            "userId":authority.0.user_id, "mediaFileId":file_id,"deviceId":device_id,
            "streamUrl":format!("/api/media/{file_id}/stream"),"receiverAddress":"192.168.1.50",
            "grantExpiresAt":chrono::DateTime::from_timestamp(expires_at, 0).unwrap().to_rfc3339(),
            "receiverTransportId":"transport-1","receiverSessionId":"receiver-session-1","mediaSessionId":7,
            "playerState":"PLAYING","currentPosition":0.0,"duration":10800.0,
            "volume":0.5,"isMuted":false,"startedAt":chrono::Utc::now().to_rfc3339()
        })).await;
        let claims = CastGrantClaims::new(
            session_id,
            file_id,
            authority.0.user_id.clone(),
            "192.168.1.50".into(),
            expires_at,
            authority.0.session_id.to_string(),
            authority.0.roles.clone(),
            device_id,
            8009,
        );
        assert!(
            validate_cast_grant_authority(&services, &claims)
                .await
                .is_ok()
        );
        services.get_cast().await.unwrap().manage_session(
            claims.clone(),
            "transport-1".into(),
            "receiver-session-1".into(),
            7,
        );
        Self {
            _temp: temp,
            services,
            db,
            schema,
            authority,
            refresh_token: login.tokens.refresh_token,
            claims,
            library_id,
        }
    }
    async fn command(&self, field: &str, args: &str) -> async_graphql::Response {
        self.schema
            .execute(
                Request::new(format!(
                    "mutation {{ {field}(sessionId:\"{}\"{args}) {{ success error }} }}",
                    self.claims.session_id,
                ))
                .data(AuthUser::from(self.authority.0.clone()))
                .data(self.authority.clone())
                .data(self.authority.subject()),
            )
            .await
    }

    async fn record_controls(&self) -> (Arc<CastService>, Arc<AtomicUsize>) {
        let cast = self.services.get_cast().await.unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let recorded = Arc::clone(&calls);
        cast.set_test_control_transport(Arc::new(move |address, port, _action| {
            assert_eq!(address, "192.168.1.50");
            assert_eq!(port, 8009);
            recorded.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }));
        (cast, calls)
    }

    async fn assert_all_controls_denied(&self) {
        for (field, args) in [
            ("castPlay", ""),
            ("castPause", ""),
            ("castSeek", ",position:3600"),
            ("castSetVolume", ",volume:0.75"),
            ("castSetMuted", ",muted:true"),
            ("castStop", ""),
        ] {
            let response = self.command(field, args).await;
            assert!(
                !response.errors.is_empty(),
                "{field} was admitted: {response:?}"
            );
        }
    }
    async fn checkpoint(&self, state: &str, position: f64, finished: bool) -> anyhow::Result<bool> {
        checkpoint_cast_status(
            &self.services,
            &self.claims,
            fake_status(state, position, finished),
            "transport-1",
            7,
        )
        .await
    }
    async fn progress(&self) -> Vec<PlaybackProgress> {
        PlaybackProgress::query(self.db.pool())
            .filter(PlaybackProgressWhereInput {
                media_file_id: Some(cast_eq(&self.claims.media_file_id)),
                ..Default::default()
            })
            .fetch_all()
            .await
            .unwrap()
    }
}
async fn create(schema: &TestSchema, entity: &str, input: Value) -> String {
    let field = format!("create{entity}");
    let node = format!("{}{}", entity[..1].to_ascii_lowercase(), &entity[1..]);
    let response = schema.execute(Request::new(format!(
        "mutation($input:Create{entity}Input!) {{ {field}(input:$input) {{ success error {node} {{ id }} }} }}"
    )).variables(Variables::from_json(json!({"input":input})))
        .data(AuthSubject::from_parts("system",vec!["admin".into()],vec![],None))
        .data(AuthUser::system_admin())).await;
    assert!(response.errors.is_empty(), "{response:?}");
    let data = response.data.into_json().unwrap();
    assert_eq!(data[&field]["success"], true, "{data}");
    data[&field][&node]["id"].as_str().unwrap().into()
}
fn fake_status(
    state: &str,
    position: f64,
    finished: bool,
) -> crate::services::cast::CastRemoteStatus {
    crate::services::cast::CastRemoteStatus {
        player_state: state.into(),
        finished,
        current_position: position,
        duration: Some(10800.0),
        volume: 0.5,
        is_muted: false,
    }
}

#[tokio::test]
async fn cast_mint_requires_the_actual_authenticated_session_authority() {
    let f = Fixture::new().await;
    let response = f.schema.execute(Request::new(format!(
        "mutation {{ castMedia(input:{{deviceId:\"{}\",mediaFileId:\"{}\"}}) {{ success error }} }}",
        f.claims.receiver_device_id, f.claims.media_file_id
    )).data(AuthUser::from(f.authority.0.clone()))).await;
    assert!(!response.errors.is_empty());
    assert!(
        response.errors[0].message.contains("SessionAuthority"),
        "{response:?}"
    );
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn rotation_preserves_cast_but_logout_revokes_the_real_session_family() {
    let f = Fixture::new().await;
    let auth = f.services.get_auth().await.unwrap();
    auth.refresh_token(&f.refresh_token).await.unwrap();
    assert!(
        validate_cast_grant_authority(&f.services, &f.claims)
            .await
            .is_ok()
    );
    f.checkpoint("PLAYING", 1800.0, false).await.unwrap();
    auth.logout(&f.refresh_token).await.unwrap();
    assert!(
        validate_cast_grant_authority(&f.services, &f.claims)
            .await
            .is_err()
    );
    assert!(f.checkpoint("PLAYING", 1900.0, false).await.is_err());
    assert_eq!(f.progress().await[0].current_position, 1800.0);
    retire_cast_monitor(&f.services, &f.claims).await;
    assert_eq!(
        CastSession::get(f.db.pool(), &f.claims.session_id)
            .await
            .unwrap()
            .unwrap()
            .player_state,
        "DISCONNECTED"
    );
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn account_disable_prevents_both_receiver_access_and_checkpoint_writes() {
    let f = Fixture::new().await;
    User::update_by_id(
        &f.db,
        &f.claims.user_id,
        UpdateUserInput {
            is_active: Some(false),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        validate_cast_grant_authority(&f.services, &f.claims)
            .await
            .is_err()
    );
    assert!(f.checkpoint("PLAYING", 100.0, false).await.is_err());
    assert!(f.progress().await.is_empty());
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn owner_role_and_persisted_issuing_claims_are_rechecked() {
    let f = Fixture::new().await;
    let mut invalid = f.claims.clone();
    invalid.issuer_roles = vec!["unknown".into()];
    assert!(
        validate_cast_grant_authority(&f.services, &invalid)
            .await
            .is_err()
    );
    invalid.issuer_session_id = uuid::Uuid::new_v4().to_string();
    invalid.issuer_roles = f.claims.issuer_roles.clone();
    assert!(
        validate_cast_grant_authority(&f.services, &invalid)
            .await
            .is_err()
    );
    let token = RefreshToken::query(f.db.pool())
        .filter(RefreshTokenWhereInput {
            session_id: Some(cast_eq(&f.claims.issuer_session_id)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .unwrap()
        .remove(0);
    RefreshToken::update_by_id(
        &f.db,
        &token.id,
        UpdateRefreshTokenInput {
            roles: Some(Some("[]".into())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        validate_cast_grant_authority(&f.services, &f.claims)
            .await
            .is_err()
    );
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn library_reassignment_cannot_write_foreign_progress_even_after_a_status_poll() {
    let f = Fixture::new().await;
    f.checkpoint("PLAYING", 125.0, false).await.unwrap();
    let other = User::insert(
        &f.db,
        CreateUserInput {
            username: "other-cast-owner".into(),
            email: None,
            password_hash: "fixture-no-login".into(),
            role: "member".into(),
            display_name: None,
            avatar_url: None,
            is_active: true,
            last_login_at: None,
        },
    )
    .await
    .unwrap();
    // Status has already been fetched; ownership changes before checkpoint commit.
    let pending_status = fake_status("PLAYING", 150.0, false);
    Library::update_by_id(
        &f.db,
        &f.library_id,
        UpdateLibraryInput {
            user_id: Some(other.id.clone()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        validate_cast_grant_authority(&f.services, &f.claims)
            .await
            .is_err()
    );
    assert!(
        checkpoint_cast_status(&f.services, &f.claims, pending_status, "transport-1", 7)
            .await
            .is_err()
    );
    let rows = f.progress().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].user_id, f.claims.user_id);
    assert_eq!(rows[0].current_position, 125.0);
    assert_ne!(rows[0].user_id, other.id);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn receiver_and_session_rebinding_fail_closed() {
    let f = Fixture::new().await;
    let mut claims = f.claims.clone();
    claims.receiver_port = 8010;
    assert!(
        validate_cast_grant_authority(&f.services, &claims)
            .await
            .is_err()
    );
    claims = f.claims.clone();
    claims.receiver_device_id = uuid::Uuid::new_v4().to_string();
    assert!(
        validate_cast_grant_authority(&f.services, &claims)
            .await
            .is_err()
    );
    CastDevice::update_by_id(
        &f.db,
        &f.claims.receiver_device_id,
        crate::graphql::entities::UpdateCastDeviceInput {
            enabled: Some(Some(false)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        validate_cast_grant_authority(&f.services, &f.claims)
            .await
            .is_err()
    );
    assert!(f.checkpoint("PLAYING", 100.0, false).await.is_err());
    assert!(f.progress().await.is_empty());
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn remote_checkpoints_update_the_canonical_owner_file_row_and_cancellation_preserves_resume()
{
    let f = Fixture::new().await;
    assert!(!f.checkpoint("PLAYING", 2400.0, false).await.unwrap());
    assert!(!f.checkpoint("PAUSED", 2600.0, false).await.unwrap());
    let rows = f.progress().await;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].user_id, f.claims.user_id);
    assert_eq!(rows[0].current_position, 2600.0);
    assert!(!rows[0].is_watched);
    // CANCELLED/INTERRUPTED/ERROR are IDLE without FINISHED. Zero currentTime
    // must not erase the last good position or mark the title watched.
    assert!(f.checkpoint("IDLE", 0.0, false).await.unwrap());
    let progress = f.progress().await.remove(0);
    assert_eq!(progress.current_position, 2600.0);
    assert!(!progress.is_watched);
    assert!(progress.watched_at.is_none());
    assert!(
        validate_cast_grant_authority(&f.services, &f.claims)
            .await
            .is_err()
    );
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn explicit_receiver_finished_marks_canonical_completion_and_ends_access() {
    let f = Fixture::new().await;
    f.checkpoint("PLAYING", 10790.0, false).await.unwrap();
    assert!(f.checkpoint("IDLE", 0.0, true).await.unwrap());
    let progress = f.progress().await.remove(0);
    assert!(progress.is_watched);
    assert!(progress.watched_at.is_some());
    assert_eq!(progress.current_position, 10800.0);
    assert_eq!(progress.progress_percent, 100.0);
    assert!(
        validate_cast_grant_authority(&f.services, &f.claims)
            .await
            .is_err()
    );
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn stale_receiver_status_and_non_finite_values_cannot_corrupt_progress() {
    let f = Fixture::new().await;
    assert!(f.checkpoint("PLAYING", f64::NAN, false).await.is_err());
    assert!(
        checkpoint_cast_status(
            &f.services,
            &f.claims,
            fake_status("PLAYING", 10.0, false),
            "old-transport",
            7
        )
        .await
        .is_err()
    );
    assert!(f.progress().await.is_empty());
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn current_account_demotion_and_refresh_expiry_revoke_cast() {
    let f = Fixture::new().await;
    User::update_by_id(
        &f.db,
        &f.claims.user_id,
        UpdateUserInput {
            role: Some("member".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        validate_cast_grant_authority(&f.services, &f.claims)
            .await
            .is_err()
    );
    assert!(f.checkpoint("PLAYING", 100.0, false).await.is_err());
    assert!(f.progress().await.is_empty());
    f.services.stop_all().await.unwrap();

    let f = Fixture::new().await;
    let token = RefreshToken::query(f.db.pool())
        .filter(RefreshTokenWhereInput {
            session_id: Some(cast_eq(&f.claims.issuer_session_id)),
            ..Default::default()
        })
        .fetch_all()
        .await
        .unwrap()
        .remove(0);
    RefreshToken::update_by_id(
        &f.db,
        &token.id,
        UpdateRefreshTokenInput {
            expires_at: Some((chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert!(
        validate_cast_grant_authority(&f.services, &f.claims)
            .await
            .is_err()
    );
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn controls_deny_disabled_receivers_before_transport() {
    let f = Fixture::new().await;
    let (_, calls) = f.record_controls().await;
    CastDevice::update_by_id(
        &f.db,
        &f.claims.receiver_device_id,
        crate::graphql::entities::UpdateCastDeviceInput {
            enabled: Some(Some(false)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    f.assert_all_controls_denied().await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(f.progress().await.is_empty());
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn controls_deny_ended_sessions_with_retained_receiver_ids_before_transport() {
    let f = Fixture::new().await;
    let (_, calls) = f.record_controls().await;
    CastSession::update_by_id(
        &f.db,
        &f.claims.session_id,
        UpdateCastSessionInput {
            ended_at: Some(Some(chrono::Utc::now().to_rfc3339())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    f.assert_all_controls_denied().await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn controls_deny_changed_receiver_address_or_port_before_transport() {
    let f = Fixture::new().await;
    let (_, calls) = f.record_controls().await;
    CastDevice::update_by_id(
        &f.db,
        &f.claims.receiver_device_id,
        crate::graphql::entities::UpdateCastDeviceInput {
            address: Some("192.168.1.51".into()),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    f.assert_all_controls_denied().await;
    CastDevice::update_by_id(
        &f.db,
        &f.claims.receiver_device_id,
        crate::graphql::entities::UpdateCastDeviceInput {
            address: Some(f.claims.receiver_address.clone()),
            port: Some(8010),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    f.assert_all_controls_denied().await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn controls_deny_expired_grants_and_rebound_receiver_sessions_before_transport() {
    let f = Fixture::new().await;
    let (_, calls) = f.record_controls().await;
    CastSession::update_by_id(
        &f.db,
        &f.claims.session_id,
        UpdateCastSessionInput {
            grant_expires_at: Some(Some(
                (chrono::Utc::now() - chrono::Duration::seconds(1)).to_rfc3339(),
            )),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    f.assert_all_controls_denied().await;
    CastSession::update_by_id(
        &f.db,
        &f.claims.session_id,
        UpdateCastSessionInput {
            grant_expires_at: Some(Some(
                chrono::DateTime::from_timestamp(f.claims.expires_at, 0)
                    .unwrap()
                    .to_rfc3339(),
            )),
            receiver_session_id: Some(Some("replacement-receiver-session".into())),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    f.assert_all_controls_denied().await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn controls_revalidate_the_actual_caller_session_before_transport() {
    let f = Fixture::new().await;
    let (_, calls) = f.record_controls().await;
    f.services
        .get_auth()
        .await
        .unwrap()
        .logout(&f.refresh_token)
        .await
        .unwrap();
    f.assert_all_controls_denied().await;
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn historical_controls_and_monitor_cleanup_cannot_touch_replacement_playback() {
    let f = Fixture::new().await;
    let (cast, calls) = f.record_controls().await;
    let replacement_id = create(&f.schema, "CastSession", json!({
        "userId":f.claims.user_id, "mediaFileId":f.claims.media_file_id,
        "deviceId":f.claims.receiver_device_id,
        "streamUrl":format!("/api/media/{}/stream", f.claims.media_file_id),
        "receiverAddress":f.claims.receiver_address,
        "grantExpiresAt":chrono::DateTime::from_timestamp(f.claims.expires_at, 0).unwrap().to_rfc3339(),
        "receiverTransportId":"transport-2","receiverSessionId":"receiver-session-2","mediaSessionId":8,
        "playerState":"PLAYING","currentPosition":40.0,"duration":10800.0,
        "volume":0.5,"isMuted":false,"startedAt":chrono::Utc::now().to_rfc3339()
    })).await;
    // Exercise the same receiver handoff used by castMedia. Leave the old row
    // active deliberately: its historical IDs cannot authorize any hardware.
    let operation = cast
        .lock_session_operations(&f.claims.receiver_address, f.claims.receiver_port)
        .await
        .unwrap();
    let released = cast.release_receiver(
        &f.claims.receiver_device_id,
        &f.claims.receiver_address,
        f.claims.receiver_port,
    );
    assert_eq!(released, vec![f.claims.clone()]);
    let mut replacement = f.claims.clone();
    replacement.session_id = replacement_id.clone();
    cast.manage_session(
        replacement,
        "transport-2".into(),
        "receiver-session-2".into(),
        8,
    );
    drop(operation);
    f.assert_all_controls_denied().await;
    tokio::time::timeout(
        std::time::Duration::from_secs(2),
        monitor_cast_session(
            Arc::clone(&f.services),
            Arc::clone(&cast),
            f.claims.session_id.clone(),
            f.claims.receiver_address.clone(),
            f.claims.receiver_port,
            "transport-1".into(),
            7,
            f.claims.clone(),
        ),
    )
    .await
    .expect("historical monitor must retire before touching transport");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let active = CastSession::get(f.db.pool(), &replacement_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(active.player_state, "PLAYING");
    assert!(active.ended_at.is_none());
    let result = control_cast_session(
        &f.services,
        &AuthUser::from(f.authority.0.clone()),
        &f.authority,
        &replacement_id,
        SessionCommand::SetVolume(0.75),
    )
    .await
    .unwrap();
    assert!(result.success);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn confirmed_seek_then_stop_persists_progress_without_waiting_for_monitor() {
    let f = Fixture::new().await;
    let (_, calls) = f.record_controls().await;
    f.checkpoint("PLAYING", 100.0, false).await.unwrap();
    let seek = f.command("castSeek", ",position:3600").await;
    assert!(seek.errors.is_empty(), "{seek:?}");
    assert_eq!(f.progress().await[0].current_position, 3600.0);
    let stop = f.command("castStop", "").await;
    assert!(stop.errors.is_empty(), "{stop:?}");
    let progress = f.progress().await.remove(0);
    assert_eq!(progress.current_position, 3600.0);
    assert!(!progress.is_watched);
    let session = CastSession::get(f.db.pool(), &f.claims.session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session.current_position, 3600.0);
    assert_eq!(session.last_position, Some(3600.0));
    assert!(session.ended_at.is_some());
    assert!(f.checkpoint("PLAYING", 101.0, false).await.is_err());
    assert_eq!(f.progress().await[0].current_position, 3600.0);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn older_poll_cannot_overwrite_a_confirmed_seek_even_before_stop() {
    let f = Fixture::new().await;
    let (_, calls) = f.record_controls().await;
    f.checkpoint("PLAYING", 100.0, false).await.unwrap();
    let observed = validate_cast_grant_authority(&f.services, &f.claims)
        .await
        .unwrap();
    let older_status = fake_status("PLAYING", 101.0, false);
    let seek = f.command("castSeek", ",position:3600").await;
    assert!(seek.errors.is_empty(), "{seek:?}");
    assert!(
        checkpoint_cast_status_for_caller(
            &f.services,
            &f.claims,
            older_status,
            "transport-1",
            7,
            None,
            Some(&observed)
        )
        .await
        .is_err()
    );
    assert_eq!(f.progress().await[0].current_position, 3600.0);
    let session = CastSession::get(f.db.pool(), &f.claims.session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session.current_position, 3600.0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn control_admission_waits_for_the_monitor_or_launch_operation_to_finish() {
    let f = Fixture::new().await;
    let (cast, calls) = f.record_controls().await;
    let operation = cast
        .lock_session_operations(&f.claims.receiver_address, f.claims.receiver_port)
        .await
        .unwrap();
    let mut seek = Box::pin(f.command("castSeek", ",position:3600"));
    assert!(futures::poll!(seek.as_mut()).is_pending());
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    // A monitor checkpoint must finish before the pending command is admitted.
    f.checkpoint("PLAYING", 100.0, false).await.unwrap();
    drop(operation);
    let result = seek.await;
    assert!(result.errors.is_empty(), "{result:?}");
    assert_eq!(f.progress().await[0].current_position, 3600.0);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    f.services.stop_all().await.unwrap();
}

#[tokio::test]
async fn rejected_transport_seek_leaves_both_session_and_canonical_checkpoint_unchanged() {
    let f = Fixture::new().await;
    f.checkpoint("PLAYING", 100.0, false).await.unwrap();
    f.services
        .get_cast()
        .await
        .unwrap()
        .set_test_control_transport(Arc::new(|_, _, _| {
            anyhow::bail!("fake receiver rejected command")
        }));
    assert!(
        !f.command("castSeek", ",position:3600")
            .await
            .errors
            .is_empty()
    );
    assert_eq!(f.progress().await[0].current_position, 100.0);
    let session = CastSession::get(f.db.pool(), &f.claims.session_id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(session.current_position, 100.0);
    f.services.stop_all().await.unwrap();
}
