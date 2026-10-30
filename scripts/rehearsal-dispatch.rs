//! Staging-only replacement for the copied macro package's schema_export module.
//! Never installed in the repository's production macro package. Typed operations
//! remain intact; a successful build qualifies only the paths actually exercised.
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{
    AngleBracketedGenericArguments, Block, Expr, GenericArgument, ImplItem, Item, ItemImpl,
    PathArguments, ReturnType, Type, TypeParamBound, visit_mut::VisitMut,
};

#[cfg(feature = "schema-export-only")]
compile_error!("Dispatch-omitted recovery cannot use schema-export-only");

const MARKER: &str = "typed-runtime-only-v1";
const UNAVAILABLE: &str = "recovery rehearsal: generated public GraphQL dispatch omitted";
const AUDIT_PREFIX: &str = "LIBRARIAN_REHEARSAL_DISPATCH_V1";

#[derive(Debug, PartialEq, Eq)]
struct SelectedImpl {
    self_type: String,
    attribute: String,
    methods: Vec<String>,
}

fn require_marker(marker: Option<&str>) -> syn::Result<()> {
    if marker != Some(MARKER) {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Staged dispatch transform requires the explicit typed-runtime-only-v1 rehearsal marker",
        ));
    }
    Ok(())
}

pub(crate) fn finish(tokens: TokenStream) -> syn::Result<TokenStream> {
    let marker = std::env::var("LIBRARIAN_REHEARSAL_DISPATCH_MARKER").ok();
    require_marker(marker.as_deref())?;
    let (output, selected) = transform(tokens)?;
    // stderr is retained in the parent-owned build log. No record is emitted
    // until the invariant has passed for the entire expansion.
    for selected in selected {
        eprintln!(
            "{AUDIT_PREFIX}\t{}\t{}\t{}\tunselected-normalized-tokens=identical",
            selected.self_type,
            selected.attribute,
            selected.methods.join(",")
        );
    }
    Ok(output)
}

/// Exact namespaces, not suffix matching or Rust visibility. Generated public
/// GraphQL fields are usually Rust-private async methods.
fn target_attribute(item: &ItemImpl) -> syn::Result<Option<String>> {
    let attributes = item
        .attrs
        .iter()
        .filter_map(|attribute| {
            let segments = attribute
                .path()
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            let kind = match segments.as_slice() {
                [namespace, kind] if namespace == "async_graphql" => kind,
                [orm, namespace, kind] if orm == "graphql_orm" && namespace == "async_graphql" => {
                    kind
                }
                _ => return None,
            };
            matches!(kind.as_str(), "Object" | "Subscription" | "ComplexObject")
                .then(|| kind.clone())
        })
        .collect::<Vec<_>>();
    if attributes.is_empty() {
        return Ok(None);
    }
    if item.trait_.is_some() || attributes.len() != 1 {
        return Err(syn::Error::new_spanned(
            item,
            "Dispatch rehearsal accepts exactly one allowlisted attribute on an inherent impl",
        ));
    }
    Ok(attributes.into_iter().next())
}

fn validate_method(method: &syn::ImplItemFn) -> syn::Result<()> {
    if method.sig.asyncness.is_none() {
        return Err(syn::Error::new_spanned(
            &method.sig,
            "Unexpected synchronous method in generated public dispatch impl",
        ));
    }
    for attribute in &method.attrs {
        if attribute.path().is_ident("graphql") {
            let options = attribute.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            )?;
            if options.iter().any(|option| option.path().is_ident("skip")) {
                return Err(syn::Error::new_spanned(
                    attribute,
                    "Skipped helpers in public dispatch impls require explicit review",
                ));
            }
        }
    }
    Ok(())
}

/// Visit only file/inline-module item lists. Never enter unmarked impl bodies,
/// functions, trait methods, closures or macro token payloads.
fn selected_impls(
    items: &mut [Item],
    erase: bool,
    audit: &mut Vec<SelectedImpl>,
) -> syn::Result<()> {
    for item in items {
        match item {
            Item::Impl(item) => {
                let Some(attribute) = target_attribute(item)? else {
                    continue;
                };
                let mut methods = Vec::new();
                for member in &mut item.items {
                    let ImplItem::Fn(method) = member else {
                        return Err(syn::Error::new_spanned(
                            &*member,
                            "Unexpected non-method item in generated public dispatch impl",
                        ));
                    };
                    validate_method(method)?;
                    methods.push(method.sig.ident.to_string());
                    method.block = if erase {
                        syn::parse_quote!({})
                    } else {
                        stub_body(&method.sig)?
                    };
                }
                if methods.is_empty() {
                    return Err(syn::Error::new_spanned(
                        &*item,
                        "Empty public dispatch impl requires review",
                    ));
                }
                audit.push(SelectedImpl {
                    self_type: item.self_ty.to_token_stream().to_string(),
                    attribute,
                    methods,
                });
            }
            Item::Mod(module) => {
                if let Some((_, items)) = &mut module.content {
                    selected_impls(items, erase, audit)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// Mask only allowlisted resolver bodies, then compare the entire normalized
/// expansion. This preserves every unselected item/body and every signature,
/// attribute, type and declaration, including declarations of selected methods.
fn verify_invariant(before: &syn::File, after: &syn::File) -> syn::Result<()> {
    let mut before = before.clone();
    let mut after = after.clone();
    let mut before_selection = Vec::new();
    let mut after_selection = Vec::new();
    selected_impls(&mut before.items, true, &mut before_selection)?;
    selected_impls(&mut after.items, true, &mut after_selection)?;
    if before_selection != after_selection
        || before.to_token_stream().to_string() != after.to_token_stream().to_string()
    {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "Dispatch rehearsal invariant failed: tokens outside selected resolver bodies changed",
        ));
    }
    Ok(())
}

fn transform(tokens: TokenStream) -> syn::Result<(TokenStream, Vec<SelectedImpl>)> {
    let original: syn::File = syn::parse2(tokens.clone())?;
    let mut transformed = original.clone();
    let mut audit = Vec::new();
    selected_impls(&mut transformed.items, false, &mut audit)?;
    let output = if audit.is_empty() {
        tokens
    } else {
        transformed.into_token_stream()
    };
    // Check the actual emitted token stream, not merely an intermediate AST.
    verify_invariant(&original, &syn::parse2(output.clone())?)?;
    Ok((output, audit))
}

fn stub_body(signature: &syn::Signature) -> syn::Result<Block> {
    let concrete = match &signature.output {
        ReturnType::Default => None,
        ReturnType::Type(_, ty) => concrete_opaque_return(ty)?,
    };
    match concrete {
        Some(concrete) => syn::parse2(quote!({
            #[allow(unreachable_code)]
            { ::core::panic!(#UNAVAILABLE); #concrete }
        })),
        None => syn::parse2(quote!({ ::core::panic!(#UNAVAILABLE) })),
    }
}

// Opaque-return witnesses follow the existing tooling helper; no runtime body
// or signature is borrowed from the generated repository implementation.
fn arguments(arguments: &PathArguments) -> Option<&AngleBracketedGenericArguments> {
    match arguments {
        PathArguments::AngleBracketed(arguments) => Some(arguments),
        _ => None,
    }
}

fn concrete_opaque_return(ty: &Type) -> syn::Result<Option<Expr>> {
    match ty {
        Type::ImplTrait(opaque) => {
            for bound in &opaque.bounds {
                let TypeParamBound::Trait(bound) = bound else {
                    continue;
                };
                let Some(segment) = bound.path.segments.last() else {
                    continue;
                };
                let (associated, stream) = match segment.ident.to_string().as_str() {
                    "Stream" => ("Item", true),
                    "Future" => ("Output", false),
                    _ => continue,
                };
                let output = arguments(&segment.arguments)
                    .and_then(|arguments| {
                        arguments.args.iter().find_map(|argument| match argument {
                            GenericArgument::AssocType(binding) if binding.ident == associated => {
                                Some(&binding.ty)
                            }
                            _ => None,
                        })
                    })
                    .ok_or_else(|| {
                        syn::Error::new_spanned(
                            ty,
                            "dispatch-omitted rehearsal requires an explicit Stream Item or Future Output",
                        )
                    })?;
                return if stream {
                    syn::parse2(quote!(::graphql_orm::futures::stream::empty::<#output>()))
                        .map(Some)
                } else {
                    syn::parse2(quote!(async {
                        #[allow(unreachable_code)]
                        {
                            let output: #output = ::core::panic!(#UNAVAILABLE);
                            output
                        }
                    }))
                    .map(Some)
                };
            }
            Err(syn::Error::new_spanned(
                ty,
                "dispatch-omitted rehearsal cannot infer this opaque runtime return type",
            ))
        }
        Type::Path(path) => {
            // Generated subscriptions use Result<impl Stream<Item = ...>>.
            if let Some(segment) = path.path.segments.last()
                && segment.ident == "Result"
                && let Some(inner) = arguments(&segment.arguments).and_then(|arguments| {
                    arguments.args.iter().find_map(|argument| match argument {
                        GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    })
                })
                && let Some(concrete) = concrete_opaque_return(inner)?
            {
                return syn::parse2(quote!(::core::result::Result::Ok(#concrete))).map(Some);
            }
            // Concrete boxed futures/streams accept a divergent body without
            // inventing a replacement type. Other nested opaque shapes do not.
            if contains_opaque(ty) {
                return Err(syn::Error::new_spanned(
                    ty,
                    "unsupported dispatch-omitted rehearsal opaque return wrapper",
                ));
            }
            Ok(None)
        }
        Type::Paren(paren) => concrete_opaque_return(&paren.elem),
        Type::Group(group) => concrete_opaque_return(&group.elem),
        _ if contains_opaque(ty) => Err(syn::Error::new_spanned(
            ty,
            "unsupported dispatch-omitted rehearsal opaque return shape",
        )),
        _ => Ok(None),
    }
}

fn contains_opaque(ty: &Type) -> bool {
    struct Find(bool);
    impl VisitMut for Find {
        fn visit_type_impl_trait_mut(&mut self, _: &mut syn::TypeImplTrait) {
            self.0 = true;
        }
    }
    let mut find = Find(false);
    find.visit_type_mut(&mut ty.clone());
    find.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> TokenStream {
        quote! {
            #[derive(InputObject)]
            struct Input { #[graphql(default = 7)] limit: i32 }
            #[::graphql_orm::async_graphql::Object(name = "FixtureQuery")]
            impl Queries {
                #[graphql(name = "fixtures")]
                async fn list(&self, ctx: &Context<'_>) -> Result<Vec<Fixture>> {
                    dispatch_only::query(ctx).await
                }
            }
            #[async_graphql::Subscription]
            impl Subscriptions {
                async fn changed(&self) -> Result<impl Stream<Item = Result<Changed>>> {
                    dispatch_only::subscribe().await
                }
            }
            #[::graphql_orm::async_graphql::ComplexObject]
            impl Fixture {
                async fn child(&self) -> Result<Option<Child>> { dispatch_only::child(self).await }
            }
            mod nested {
                #[async_graphql::Object(extends)]
                impl<'a> Relations<'a> {
                    async fn related(&self) -> Result<Vec<Child>> { dispatch_only::related().await }
                }
            }
            impl Fixture {
                const TABLE: &'static str = "fixtures";
                pub fn metadata() -> &'static Metadata { production::metadata() }
                pub async fn insert(db: &Database, input: Input) -> Result<Self> {
                    production::insert_with_hooks(db, input).await
                }
                async fn __gom_insert_with_mutation_context(tx: &mut MutationContext, input: Input) -> Result<Self> {
                    production::transaction(tx, input).await
                }
                pub fn write<'a>(tx: &'a mut MutationContext) -> BoxFuture<'a, Result<Self>> {
                    Box::pin(async move { production::write(tx).await })
                }
                fn future(&self) -> impl Future<Output = Result<Self>> {
                    async move { production::future(self).await }
                }
            }
            impl MutationContextInsert for Fixture {
                fn insert_in_mutation_context<'a>(tx: &'a mut MutationContext) -> BoxFuture<'a, Result<Self>> {
                    Box::pin(async move { Self::__gom_insert_with_mutation_context(tx, Input::default()).await })
                }
            }
            impl RelationLoader for Fixture {
                async fn bulk_load_relations(&mut self) -> Result<()> { production::load(self).await }
            }
            impl GeneratedRelationsObject for Fixture {
                fn generated_relations_object(&self) -> Relations<'_> { Relations(self) }
            }
            trait KeepDefault {
                async fn default_method(&self) -> Result<()> { production::default_method(self).await }
            }
            async fn helper() -> Result<()> { production::helper().await }
            fn boxed() -> BoxFuture<'static, Result<()>> { Box::pin(production::boxed()) }
            preserve_macro! { async fn nested_tokens() { production::nested().await } }
        }
    }

    #[test]
    fn explicit_marker_is_required_without_environment_mutation() {
        assert!(require_marker(Some("typed-runtime-only-v1")).is_ok());
        for marker in [None, Some(""), Some("true"), Some("typed-runtime-only-v2")] {
            assert!(require_marker(marker).is_err());
        }
    }

    #[test]
    fn removes_only_marked_dispatch_and_preserves_all_typed_runtime_tokens() {
        let input = fixture();
        let (output, selected) = transform(input.clone()).unwrap();
        verify_invariant(
            &syn::parse2(input).unwrap(),
            &syn::parse2(output.clone()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            selected
                .iter()
                .map(|item| item.attribute.as_str())
                .collect::<Vec<_>>(),
            ["Object", "Subscription", "ComplexObject", "Object"]
        );
        assert_eq!(
            selected
                .iter()
                .map(|item| item.self_type.as_str())
                .collect::<Vec<_>>(),
            ["Queries", "Subscriptions", "Fixture", "Relations < 'a >"]
        );
        assert_eq!(
            selected
                .iter()
                .flat_map(|item| item.methods.iter().map(String::as_str))
                .collect::<Vec<_>>(),
            ["list", "changed", "child", "related"]
        );
        let text = output.to_string();
        assert!(!text.contains("dispatch_only"));
        assert!(text.contains(UNAVAILABLE));
        for required in [
            "insert_with_hooks",
            "__gom_insert_with_mutation_context",
            "production :: transaction",
            "production :: write",
            "production :: future",
            "production :: load",
            "generated_relations_object",
            "production :: default_method",
            "production :: helper",
            "production :: boxed",
            "production :: nested",
        ] {
            assert!(
                text.contains(required),
                "lost typed runtime fragment {required}"
            );
        }
    }

    #[test]
    fn unselected_expansion_returns_the_original_token_stream() {
        let input = quote! {
            #[other::Object] impl Fixture { pub async fn insert(&self) { actual::write().await } }
            #[async_graphql::ObjectExtra] impl Other { async fn typed(&self) { actual::typed().await } }
            impl MutationContextUpdateById for Fixture {
                fn update(&self) -> BoxFuture<'_, ()> { Box::pin(actual::update()) }
            }
        };
        let (output, selected) = transform(input.clone()).unwrap();
        assert!(selected.is_empty());
        assert_eq!(input.to_string(), output.to_string());
    }

    #[test]
    fn invariant_rejects_changes_to_an_unselected_typed_body() {
        let before: syn::File = syn::parse2(fixture()).unwrap();
        let mut after = before.clone();
        let Item::Fn(function) = after
            .items
            .iter_mut()
            .find(|item| matches!(item, Item::Fn(function) if function.sig.ident == "helper"))
            .unwrap()
        else {
            unreachable!()
        };
        function.block = Box::new(syn::parse_quote!({ panic!("typed body was lost") }));
        assert!(verify_invariant(&before, &after).is_err());
    }

    #[test]
    fn invariant_rejects_changes_to_transaction_trait_and_selected_signature() {
        let before: syn::File = syn::parse2(fixture()).unwrap();
        let mut after = before.clone();
        let Item::Impl(item) = after
            .items
            .iter_mut()
            .find(|item| matches!(item, Item::Impl(item) if item.trait_.is_some()))
            .unwrap()
        else {
            unreachable!()
        };
        let ImplItem::Fn(method) = &mut item.items[0] else {
            unreachable!()
        };
        method.block = syn::parse_quote!({ panic!("typed transaction was lost") });
        assert!(verify_invariant(&before, &after).is_err());

        let mut after = before.clone();
        let Item::Impl(item) = &mut after.items[1] else {
            unreachable!()
        };
        let ImplItem::Fn(method) = &mut item.items[0] else {
            unreachable!()
        };
        method.sig.output = syn::parse_quote!(-> Result<u64>);
        assert!(verify_invariant(&before, &after).is_err());
    }

    #[test]
    fn future_and_result_stream_witnesses_preserve_declared_signatures() {
        let input = quote! {
            #[async_graphql::Object]
            impl Queries {
                async fn future(&self) -> impl Future<Output = Result<Fixture>> + Send {
                    dispatch_only::future().await
                }
            }
            #[async_graphql::Subscription]
            impl Subscriptions {
                async fn changed(&self) -> Result<impl Stream<Item = Result<Changed>> + Send + '_> {
                    dispatch_only::stream().await
                }
            }
        };
        let (output, _) = transform(input.clone()).unwrap();
        verify_invariant(
            &syn::parse2(input).unwrap(),
            &syn::parse2(output.clone()).unwrap(),
        )
        .unwrap();
        let text = output.to_string();
        assert!(text.contains("empty :: < Result < Changed > >"), "{text}");
        assert!(!text.contains("dispatch_only"));
    }

    #[test]
    fn unexpected_public_impl_shapes_fail_closed() {
        for input in [
            quote!(
                #[async_graphql::Object]
                impl Queries {
                    fn sync(&self) -> bool {
                        true
                    }
                }
            ),
            quote!(
                #[async_graphql::Object]
                impl Queries {
                    #[graphql(skip)]
                    async fn helper(&self) {}
                }
            ),
            quote!(
                #[async_graphql::Object]
                impl Queries {
                    const C: bool = true;
                }
            ),
            quote!(
                #[async_graphql::Object]
                impl Queries {
                    methods!();
                }
            ),
            quote!(
                #[async_graphql::Object]
                impl Trait for Queries {
                    async fn field(&self) {}
                }
            ),
            quote!(
                #[async_graphql::Object]
                #[async_graphql::Subscription]
                impl Queries {
                    async fn field(&self) {}
                }
            ),
            quote!(
                #[async_graphql::Object]
                impl Queries {
                    async fn field(&self) -> Option<impl Stream<Item = bool>> {
                        None
                    }
                }
            ),
        ] {
            assert!(
                transform(input.clone()).is_err(),
                "accepted unexpected shape: {input}"
            );
        }
    }
}
