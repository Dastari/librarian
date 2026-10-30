//! Private current-session invalidation, backed by generated commit witnesses.
//! Members do not gain access to the admin-only User/RefreshToken entity streams.
use std::{pin::Pin, sync::Arc};

use async_graphql::{Context, Result, SimpleObject, Subscription};
use futures::{Stream, StreamExt};
use graphql_orm::graphql::subscriptions::SubscriptionEvent;
use time::OffsetDateTime;

use crate::db::Database;
use crate::graphql::entities::{RefreshTokenChangedEvent, UserChangedEvent};
use crate::services::graphql::auth::AuthExt;
use crate::services::graphql::session_authority::SessionAuthority;
use crate::services::manager::ServicesManager;

#[derive(SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct SessionInvalidation {
    /// ready, changed or resync. No account/credential payload is broadcast.
    pub kind: String,
}

#[derive(Default)]
pub struct SessionSubscriptions;

#[Subscription]
impl SessionSubscriptions {
    async fn session_changed(
        &self,
        ctx: &Context<'_>,
    ) -> Result<Pin<Box<dyn Stream<Item = SessionInvalidation> + Send>>> {
        ctx.require_member()?;
        let authority = ctx.data::<SessionAuthority>()?.clone();
        let auth = ctx
            .data::<Arc<ServicesManager>>()?
            .get_auth()
            .await
            .ok_or_else(|| async_graphql::Error::new("Authentication service unavailable"))?;
        auth.validate_session_authority(&authority.0)
            .await
            .map_err(|_| async_graphql::Error::new("Session is no longer authorized"))?;
        let db = ctx.data::<Database>()?;
        let users = db
            .ensure_event_sender::<SubscriptionEvent<UserChangedEvent>>()
            .subscribe();
        let tokens = db
            .ensure_event_sender::<SubscriptionEvent<RefreshTokenChangedEvent>>()
            .subscribe();
        let remaining = authority
            .0
            .token_claims
            .expires_at
            .unwrap_or_else(OffsetDateTime::now_utc)
            - OffsetDateTime::now_utc();
        let deadline =
            tokio::time::Instant::now() + remaining.try_into().unwrap_or(std::time::Duration::ZERO);
        let user_id = authority.0.user_id;
        let session_id = authority.0.session_id.to_string();
        Ok(futures::stream::unfold((users, tokens, user_id, session_id, true, deadline, false),
            |(mut users, mut tokens, user_id, session_id, ready, deadline, done)| async move {
                if done { return None; }
                if ready {
                    return Some((SessionInvalidation { kind: "ready".into() }, (users, tokens, user_id, session_id, false, deadline, false)));
                }
                loop {
                    let (kind, done) = tokio::select! {
                        result = users.recv() => match result {
                            Ok(witness) if witness.event.id == user_id => ("changed", false),
                            Ok(_) => continue,
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => ("resync", false),
                            Err(_) => ("resync", true),
                        },
                        result = tokens.recv() => match result {
                            Ok(witness) if witness.event.entity.as_ref().is_some_and(|row| row.user_id == user_id && row.session_id == session_id) => ("changed", false),
                            Ok(_) => continue,
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => ("resync", false),
                            Err(_) => ("resync", true),
                        },
                        _ = tokio::time::sleep_until(deadline) => ("changed", true),
                    };
                    return Some((SessionInvalidation { kind: kind.into() }, (users, tokens, user_id, session_id, false, deadline, done)));
                }
            }).boxed())
    }
}
