//! Syntax/expansion checks only. These do not qualify generated runtime code.
use super::*;
use quote::{ToTokens, quote};
use syn::visit_mut::VisitMut;

fn fixture() -> TokenStream {
    quote! {
        #[derive(::graphql_orm::async_graphql::InputObject)]
        pub struct QueryInput {
            #[graphql(name = "limit", default = 20)]
            pub limit: i32,
        }
        #[derive(::graphql_orm::async_graphql::SimpleObject)]
        pub struct Changed { pub id: String }
        #[::graphql_orm::async_graphql::Object(name = "FixtureQuery")]
        impl Queries {
            /// Preserve docs, public field name, default and type exactly.
            #[graphql(name = "fixtures", visible = true)]
            async fn list<'a>(&self, ctx: &Context<'a>, #[graphql(default = 20)] limit: i32) -> Result<Vec<Fixture>> {
                heavy::query_and_authorize(ctx, limit).await
            }
        }
        #[::graphql_orm::async_graphql::Subscription]
        impl Subscriptions {
            #[graphql(name = "fixtureChanged")]
            async fn on_changed(&self, ctx: &Context<'_>) -> ::graphql_orm::async_graphql::Result<impl ::graphql_orm::futures::Stream<Item = ::graphql_orm::async_graphql::Result<Changed>>> {
                heavy::authorize_and_subscribe(ctx).await
            }
        }
        impl Fixture {
            const TABLE: &'static str = "fixtures";
            pub fn metadata() -> &'static Metadata {
                static META: Metadata = Metadata { read_policy: Some("member.read"), write_policy: Some("admin.write") };
                &META
            }
            pub fn new() -> Self { Self::default() }
            pub fn write<'a>(ctx: &'a Context<'a>) -> ::graphql_orm::futures::future::BoxFuture<'a, Result<Self>> {
                Box::pin(async move { heavy::write_and_emit(ctx).await })
            }
            pub fn future<'a>(ctx: &'a Context<'a>) -> impl ::core::future::Future<Output = Result<Fixture>> + Send + 'a {
                async move { heavy::load(ctx).await }
            }
        }
    }
}

fn erased_bodies(tokens: TokenStream) -> String {
    struct Erase;
    impl VisitMut for Erase {
        fn visit_impl_item_fn_mut(&mut self, method: &mut syn::ImplItemFn) {
            method.block = syn::parse_quote!({});
        }
        fn visit_item_fn_mut(&mut self, function: &mut syn::ItemFn) {
            function.block = Box::new(syn::parse_quote!({}));
        }
        fn visit_trait_item_fn_mut(&mut self, method: &mut syn::TraitItemFn) {
            if method.default.is_some() {
                method.default = Some(syn::parse_quote!({}));
            }
        }
    }
    let mut file: syn::File = syn::parse2(tokens).unwrap();
    Erase.visit_file_mut(&mut file);
    file.to_token_stream().to_string()
}
fn sync_bodies(tokens: TokenStream) -> Vec<String> {
    struct Gather(Vec<String>);
    impl VisitMut for Gather {
        fn visit_impl_item_fn_mut(&mut self, method: &mut syn::ImplItemFn) {
            let runtime = match &method.sig.output {
                ReturnType::Default => false,
                ReturnType::Type(_, ty) => runtime_type(ty),
            };
            if method.sig.asyncness.is_none() && !runtime {
                self.0.push(method.to_token_stream().to_string());
            }
        }
        fn visit_item_fn_mut(&mut self, function: &mut syn::ItemFn) {
            let runtime = match &function.sig.output {
                ReturnType::Default => false,
                ReturnType::Type(_, ty) => runtime_type(ty),
            };
            if function.sig.asyncness.is_none() && !runtime {
                self.0.push(function.to_token_stream().to_string());
            }
        }
    }
    let mut file: syn::File = syn::parse2(tokens).unwrap();
    let mut gather = Gather(vec![]);
    gather.visit_file_mut(&mut file);
    gather.0
}

#[test]
fn feature_gate_preserves_the_default_expansion_verbatim() {
    let input = fixture();
    let expected = if cfg!(feature = "schema-export-only") {
        stub_expansion(input.clone()).unwrap()
    } else {
        input.clone()
    };
    assert_eq!(finish(input).unwrap().to_string(), expected.to_string());
}

#[test]
fn only_runtime_bodies_change_and_opaque_stream_future_signatures_stay_exact() {
    let input = fixture();
    let output = stub_expansion(input.clone()).unwrap();
    assert_eq!(erased_bodies(input.clone()), erased_bodies(output.clone()));
    assert_eq!(sync_bodies(input), sync_bodies(output.clone()));
    let text = output.to_string();
    assert!(!text.contains("heavy ::"));
    assert!(text.contains(UNAVAILABLE));
    // Compare the actual Item type, not printed turbofish punctuation (which
    // rustfmt may legitimately change by inserting a trailing generic comma).
    struct StreamItems(Vec<Type>);
    impl VisitMut for StreamItems {
        fn visit_expr_call_mut(&mut self, call: &mut syn::ExprCall) {
            if let Expr::Path(path) = call.func.as_ref()
                && let Some(segment) = path.path.segments.last()
                && segment.ident == "empty"
                && let Some(arguments) = arguments(&segment.arguments)
            {
                for argument in &arguments.args {
                    if let GenericArgument::Type(item) = argument {
                        self.0.push(item.clone());
                    }
                }
            }
            syn::visit_mut::visit_expr_call_mut(self, call);
        }
    }
    let mut parsed = syn::parse2::<syn::File>(output).unwrap();
    let mut items = StreamItems(vec![]);
    items.visit_file_mut(&mut parsed);
    assert_eq!(items.0.len(), 1);
    let expected: Type = syn::parse_quote!(::graphql_orm::async_graphql::Result<Changed>);
    assert_eq!(
        items.0[0].to_token_stream().to_string(),
        expected.to_token_stream().to_string()
    );
    // BoxFuture retains its declared trait-object type; impl Future gets an
    // unreachable typed async witness instead of incorrectly inferring Never.
    assert!(text.contains(&quote!(let output: Result<Fixture>).to_string()));
}

#[test]
fn generated_stub_blocks_round_trip_without_unrepresentable_inner_attributes() {
    // This is the exact former proc-macro panic: syn::Block does not own an
    // attribute list, unlike ExprBlock/ExprAsync and ItemFn/ImplItemFn.
    let invalid = syn::parse2::<Block>(quote!(
        {
            #![allow(unreachable_code)]
            ::core::panic!("old stub")
        }
    ))
    .unwrap_err();
    assert!(invalid.to_string().contains("square brackets"));

    struct RoundTrip(usize);
    impl VisitMut for RoundTrip {
        fn visit_block_mut(&mut self, block: &mut Block) {
            syn::parse2::<Block>(block.to_token_stream())
                .expect("stub block must parse independently");
            self.0 += 1;
            syn::visit_mut::visit_block_mut(self, block);
        }
        fn visit_attribute_mut(&mut self, attribute: &mut syn::Attribute) {
            assert!(matches!(attribute.style, syn::AttrStyle::Outer));
        }
    }
    let transformed = stub_expansion(fixture()).unwrap();
    let mut file: syn::File = syn::parse2(transformed).unwrap();
    let mut round_trip = RoundTrip(0);
    round_trip.visit_file_mut(&mut file);
    assert!(round_trip.0 > 5);
}

#[test]
fn actual_operation_and_relation_expansions_retain_all_non_body_tokens() {
    let entity: syn::DeriveInput = syn::parse_quote! {
        #[graphql(complex)]
        #[graphql(rename_fields = "camelCase")]
        #[graphql_entity(table = "schema_export_fixtures", plural = "SchemaExportFixtures", default_sort = "created_at", read_policy = "member.read", write_policy = "admin.write", aggregate = true, keyset = "created_at desc, id desc")]
        pub struct Fixture {
            #[primary_key]
            #[filterable(type = "string")]
            pub id: String,
            #[filterable(type = "string")]
            pub owner_id: String,
            #[filterable(type = "date")]
            #[sortable]
            pub created_at: String,
            #[filterable(type = "number")]
            #[sortable]
            pub score: i32,
            #[graphql(skip)]
            #[serde(skip)]
            #[skip_db]
            #[relation(target = "Parent", from = "owner_id", to = "id", emit_fk = false, propagate_change = "up")]
            pub parent: Option<Box<Parent>>,
        }
    };
    let operations = crate::operations::generate_graphql_operations(&entity).unwrap();
    let relations = crate::relations::generate_graphql_relations(&entity).unwrap();
    for original in [operations, relations] {
        let transformed = stub_expansion(original.clone()).unwrap();
        assert_eq!(
            erased_bodies(original.clone()),
            erased_bodies(transformed.clone())
        );
        assert_eq!(sync_bodies(original), sync_bodies(transformed));
    }
    // Entity metadata generation is explicitly outside this transformer.
    let entity_metadata = crate::entity::generate_graphql_entity(&entity).unwrap();
    assert!(!entity_metadata.to_string().contains(UNAVAILABLE));
}

#[test]
fn unsupported_opaque_shapes_fail_instead_of_inventing_a_schema_return_type() {
    let input = quote! { impl Export { fn stream() -> Option<impl ::graphql_orm::futures::Stream<Item = Changed>> { heavy::stream() } } };
    let error = stub_expansion(input).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("unsupported schema-export-only opaque return wrapper")
    );
}
