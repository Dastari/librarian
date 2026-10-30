//! Inspect real feature-off generator bodies, never schema-export stubs.
//! These AST checks do not exercise broadcast overflow or GraphQL execution.
use quote::{ToTokens, quote};
use syn::visit_mut::VisitMut;

fn runtime_subscription(append_only: bool) -> syn::ImplItemFn {
    let input: syn::DeriveInput = syn::parse2(quote! {
        #[graphql_entity(table = "lag_fixtures", plural = "LagFixtures", append_only = #append_only, read_policy = "member.read", write_policy = "admin.write")]
        pub struct LagFixture {
            #[primary_key]
            #[filterable(type = "string")]
            pub id: String,
            pub name: String,
        }
    }).unwrap();
    let original = crate::operations::generate_graphql_operations(&input).unwrap();
    // This module is disabled in export mode; finish must be the identity here.
    assert_eq!(
        original.to_string(),
        crate::schema_export::finish(original.clone())
            .unwrap()
            .to_string()
    );
    assert!(!original.to_string().contains("schema-export-only"));
    let file: syn::File = syn::parse2(original).unwrap();
    let mut subscriptions = file
        .items
        .into_iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item) => Some(item.items),
            _ => None,
        })
        .flatten()
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method) if method.sig.ident == "on_changed" => Some(method),
            _ => None,
        });
    let method = subscriptions.next().expect("generated subscription");
    assert!(subscriptions.next().is_none());
    method
}

#[derive(Default)]
struct SubscriptionBody {
    receives: Vec<syn::ExprMatch>,
    authorized: Vec<syn::ExprMatch>,
    calls: std::collections::BTreeMap<String, usize>,
}

impl VisitMut for SubscriptionBody {
    fn visit_expr_match_mut(&mut self, expression: &mut syn::ExprMatch) {
        if let syn::Expr::Path(path) = expression.expr.as_ref() {
            if path.path.is_ident("result") {
                self.receives.push(expression.clone());
            } else if path.path.is_ident("authorized") {
                self.authorized.push(expression.clone());
            }
        }
        syn::visit_mut::visit_expr_match_mut(self, expression);
    }

    fn visit_expr_call_mut(&mut self, expression: &mut syn::ExprCall) {
        if let syn::Expr::Path(path) = expression.func.as_ref() {
            if let Some(segment) = path.path.segments.last() {
                *self.calls.entry(segment.ident.to_string()).or_default() += 1;
            }
        }
        syn::visit_mut::visit_expr_call_mut(self, expression);
    }

    fn visit_expr_method_call_mut(&mut self, expression: &mut syn::ExprMethodCall) {
        *self.calls.entry(expression.method.to_string()).or_default() += 1;
        syn::visit_mut::visit_expr_method_call_mut(self, expression);
    }
}

fn assert_runtime_lag_mapping(append_only: bool) {
    let mut method = runtime_subscription(append_only);
    let expected_output: syn::ReturnType = syn::parse_quote! {
        -> ::graphql_orm::async_graphql::Result<impl ::graphql_orm::futures::Stream<Item = ::graphql_orm::async_graphql::Result<LagFixtureChangedEvent>>>
    };
    assert_eq!(
        method.sig.output.to_token_stream().to_string(),
        expected_output.to_token_stream().to_string()
    );
    let mut body = SubscriptionBody::default();
    body.visit_block_mut(&mut method.block);
    // A receive error must be an emitted GraphQL error with a resync code.
    // In particular, result.ok()? would silently discard the lag notification.
    let expected_receive: syn::ExprMatch = syn::parse_quote! {
        match result {
            Ok(witness) => witness,
            Err(_) => return Some(Err(::graphql_orm::async_graphql::ErrorExtensions::extend_with(
                ::graphql_orm::async_graphql::Error::new("Subscription events were missed; refresh the authorized snapshot"),
                |_, extensions| extensions.set("code", "LIVE_RESYNC_REQUIRED"),
            ))),
        }
    };
    assert_eq!(body.receives.len(), 1);
    assert_eq!(
        body.receives[0].to_token_stream().to_string(),
        expected_receive.to_token_stream().to_string()
    );
    let expected_delivery: syn::ExprMatch = syn::parse_quote! {
        match authorized {
            Ok(row) => Some(Ok(LagFixtureChangedEvent {
                action: event.action,
                change_kind: event.change_kind,
                id: event.id.clone(),
                source_entity: event.source_entity.clone(),
                source_id: event.source_id.clone(),
                path: event.path.clone(),
                entity: Some(row),
            })),
            Err(error) => { let _ = ::graphql_orm::graphql::loaders::relation_authorization_error(error); None }
        }
    };
    assert_eq!(body.authorized.len(), 1);
    assert_eq!(
        body.authorized[0].to_token_stream().to_string(),
        expected_delivery.to_token_stream().to_string()
    );
    // Preserve entry authorization and post-await rechecks; denied rows stay
    // filtered above instead of being returned as successful changed events.
    for (name, count) in [
        ("enforce_resolver_auth", 3),
        ("enforce_resolver_assurance", 3),
        ("ensure_entity_access", 2),
        ("authorize_provenance", 1),
        ("authorize_subscription_row", 1),
        ("authorize_subscription_fields", 1),
    ] {
        assert_eq!(
            body.calls.get(name).copied().unwrap_or_default(),
            count,
            "{name}"
        );
    }
}

#[test]
fn ordinary_single_pk_subscription_emits_resync_error_on_lag() {
    assert_runtime_lag_mapping(false);
}

#[test]
fn append_only_single_pk_subscription_emits_the_same_resync_error_on_lag() {
    assert_runtime_lag_mapping(true);
}
