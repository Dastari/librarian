//! Session-bound delivery guard. The credential is server-only request data;
//! no bearer token or credential fields enter the GraphQL output schema.
#[cfg(test)]
#[path = "session_authority_tests.rs"]
mod tests;
use std::sync::Arc;

use async_graphql::extensions::{
    Extension, ExtensionContext, ExtensionFactory, NextExecute, NextPrepareRequest, NextSubscribe,
};
use async_graphql::{ErrorExtensions, Request, Response, ServerResult};
use futures::{StreamExt, stream::BoxStream};

use crate::services::auth::AuthService;
use crate::services::manager::ServicesManager;

#[derive(Clone)]
pub(crate) struct SessionAuthority(pub agql_auth::AuthUser);

impl SessionAuthority {
    pub(crate) fn subject(&self) -> graphql_orm::graphql::auth::AuthSubject {
        let user = &self.0;
        let mut subject = graphql_orm::graphql::auth::AuthSubject::from_parts(
            user.user_id.clone(),
            user.roles.clone(),
            user.scopes.clone(),
            user.token_claims.tenant_id.clone(),
        );
        subject.user_id = Some(user.user_id.clone());
        subject.session_id = Some(user.session_id.to_string());
        subject.token_id = user.token_claims.jti.clone();
        subject.correlation_id = user.token_claims.correlation_id.clone();
        subject.organization_id = user.token_claims.organization_id.clone();
        subject
    }
}

pub(crate) struct SessionDeliveryGuard;

async fn authority_valid(ctx: &ExtensionContext<'_>) -> bool {
    let Some(authority) = ctx.data_opt::<SessionAuthority>() else {
        return true;
    };
    let Some(services) = ctx.data_opt::<Arc<ServicesManager>>() else {
        return false;
    };
    let Some(auth) = services.get_auth().await else {
        return false;
    };
    auth.validate_session_authority(&authority.0).await.is_ok()
}

fn authority_error() -> async_graphql::ServerError {
    async_graphql::Error::new("Session authority changed")
        .extend_with(|_, ext| ext.set("code", "UNAUTHORIZED"))
        .into_server_error(async_graphql::Pos::default())
}

impl ExtensionFactory for SessionDeliveryGuard {
    fn create(&self) -> Arc<dyn Extension> {
        Arc::new(SessionDeliveryGuard)
    }
}

#[async_trait::async_trait]
impl Extension for SessionDeliveryGuard {
    // Socket queries/mutations also use execute_stream. Checking only after
    // polling the response stream is too late: effects may already have run.
    async fn prepare_request(
        &self,
        ctx: &ExtensionContext<'_>,
        request: Request,
        next: NextPrepareRequest<'_>,
    ) -> ServerResult<Request> {
        if !authority_valid(ctx).await {
            return Err(authority_error());
        }
        next.run(ctx, request).await
    }

    async fn execute(
        &self,
        ctx: &ExtensionContext<'_>,
        operation_name: Option<&str>,
        next: NextExecute<'_>,
    ) -> Response {
        if !authority_valid(ctx).await {
            return Response::from_errors(vec![authority_error()]);
        }
        next.run(ctx, operation_name).await
    }

    fn subscribe<'s>(
        &self,
        ctx: &ExtensionContext<'_>,
        stream: BoxStream<'s, Response>,
        next: NextSubscribe<'_>,
    ) -> BoxStream<'s, Response> {
        let stream = next.run(ctx, stream);
        let Some(authority) = ctx.data_opt::<SessionAuthority>().cloned() else {
            return stream;
        };
        let services = ctx.data_opt::<Arc<ServicesManager>>().cloned();
        // Validate once per response, rather than independently for every selected
        // metadata field. Stop before releasing any payload under stale authority.
        futures::stream::unfold(
            (stream, services, authority, false),
            |(mut stream, services, authority, stopped)| async move {
                if stopped {
                    return None;
                }
                let response = stream.next().await?;
                let auth: Option<Arc<AuthService>> = match &services {
                    Some(services) => services.get_auth().await,
                    None => None,
                };
                let valid = match auth {
                    Some(auth) => auth.validate_session_authority(&authority.0).await.is_ok(),
                    None => false,
                };
                let response = if valid {
                    response
                } else {
                    Response::from_errors(vec![authority_error()])
                };
                Some((response, (stream, services, authority, !valid)))
            },
        )
        .boxed()
    }
}
