//! Host-only, opt-in postprocessing for exporting the real merged Rust schema.
//!
//! Types, attributes, defaults, synchronous metadata and registrations remain
//! intact. Only runtime function bodies from operations/relations are replaced.
//! This deliberately does not touch GraphQLEntity or schema_roots expansion.
use proc_macro2::TokenStream;

pub(crate) fn finish(tokens: TokenStream) -> syn::Result<TokenStream> {
    #[cfg(feature = "schema-export-only")]
    {
        stub_expansion(tokens)
    }
    #[cfg(not(feature = "schema-export-only"))]
    {
        // Do not even parse/reprint normal runtime expansion. In particular,
        // preserve the independent subscription-lag patch exactly as emitted.
        Ok(tokens)
    }
}

#[cfg(any(feature = "schema-export-only", test))]
use quote::{ToTokens, quote};
#[cfg(any(feature = "schema-export-only", test))]
use syn::{
    AngleBracketedGenericArguments, Block, Expr, GenericArgument, PathArguments, ReturnType,
    Signature, Type, TypeParamBound, visit_mut::VisitMut,
};

#[cfg(any(feature = "schema-export-only", test))]
const UNAVAILABLE: &str =
    "graphql-orm schema-export-only build: generated runtime bodies are unavailable";

#[cfg(any(feature = "schema-export-only", test))]
fn stub_expansion(tokens: TokenStream) -> syn::Result<TokenStream> {
    let mut file: syn::File = syn::parse2(tokens)?;
    let mut transform = StubRuntime { error: None };
    transform.visit_file_mut(&mut file);
    if let Some(error) = transform.error {
        return Err(error);
    }
    Ok(file.into_token_stream())
}

#[cfg(any(feature = "schema-export-only", test))]
struct StubRuntime {
    error: Option<syn::Error>,
}

#[cfg(any(feature = "schema-export-only", test))]
impl StubRuntime {
    fn body(&mut self, signature: &Signature, body: &mut Block) -> bool {
        let returns_runtime = match &signature.output {
            ReturnType::Default => false,
            ReturnType::Type(_, ty) => runtime_type(ty),
        };
        if signature.asyncness.is_none() && !returns_runtime {
            return false;
        }
        let concrete = match &signature.output {
            ReturnType::Default => Ok(None),
            ReturnType::Type(_, ty) => concrete_opaque_return(ty),
        };
        // syn::Block cannot contain inner attributes: it has only braces and
        // statements. An outer attribute on a nested expression block is valid
        // both for Block::parse_within and when emitted as a method body.
        let replacement = concrete.and_then(|concrete| match concrete {
            Some(concrete) => syn::parse2(quote!({
                #[allow(unreachable_code)]
                {
                    ::core::panic!(#UNAVAILABLE);
                    #concrete
                }
            })),
            None => syn::parse2(quote!({ ::core::panic!(#UNAVAILABLE) })),
        });
        match replacement {
            Ok(replacement) => *body = replacement,
            Err(error) => {
                self.error = Some(syn::Error::new_spanned(
                    signature,
                    format!("schema-export-only runtime body: {error}"),
                ));
            }
        }
        true
    }
}

#[cfg(any(feature = "schema-export-only", test))]
impl VisitMut for StubRuntime {
    fn visit_impl_item_fn_mut(&mut self, method: &mut syn::ImplItemFn) {
        if !self.body(&method.sig, &mut method.block) {
            syn::visit_mut::visit_impl_item_fn_mut(self, method);
        }
    }
    fn visit_item_fn_mut(&mut self, function: &mut syn::ItemFn) {
        if !self.body(&function.sig, &mut function.block) {
            syn::visit_mut::visit_item_fn_mut(self, function);
        }
    }
    fn visit_trait_item_fn_mut(&mut self, method: &mut syn::TraitItemFn) {
        if let Some(body) = &mut method.default {
            if self.body(&method.sig, body) {
                return;
            }
        }
        syn::visit_mut::visit_trait_item_fn_mut(self, method);
    }
}

#[cfg(any(feature = "schema-export-only", test))]
fn arguments(arguments: &PathArguments) -> Option<&AngleBracketedGenericArguments> {
    match arguments {
        PathArguments::AngleBracketed(arguments) => Some(arguments),
        _ => None,
    }
}

#[cfg(any(feature = "schema-export-only", test))]
fn runtime_type(ty: &Type) -> bool {
    match ty {
        Type::ImplTrait(opaque) => opaque.bounds.iter().any(|bound| {
            matches!(bound, TypeParamBound::Trait(bound) if bound.path.segments.last().is_some_and(|segment| matches!(segment.ident.to_string().as_str(), "Future" | "Stream")))
        }),
        Type::Path(path) => path.path.segments.iter().any(|segment| {
            matches!(segment.ident.to_string().as_str(), "BoxFuture" | "LocalBoxFuture" | "BoxStream" | "LocalBoxStream" | "Future" | "Stream")
                || arguments(&segment.arguments).is_some_and(|arguments| arguments.args.iter().any(|argument| match argument {
                    GenericArgument::Type(ty) => runtime_type(ty),
                    GenericArgument::AssocType(binding) => runtime_type(&binding.ty),
                    _ => false,
                }))
        }),
        Type::Reference(reference) => runtime_type(&reference.elem),
        Type::Paren(paren) => runtime_type(&paren.elem),
        Type::Group(group) => runtime_type(&group.elem),
        Type::TraitObject(object) => object.bounds.iter().any(|bound| {
            matches!(bound, TypeParamBound::Trait(bound) if bound.path.segments.last().is_some_and(|segment| matches!(segment.ident.to_string().as_str(), "Future" | "Stream")))
        }),
        _ => false,
    }
}

/// `impl Stream`/`impl Future` cannot use Never as their inferred hidden type:
/// Never implements neither trait. Supply an unreachable concrete witness with
/// the original Item/Output, preserving the *declared* return type verbatim.
/// Unknown opaque wrappers fail closed instead of changing a schema signature.
#[cfg(any(feature = "schema-export-only", test))]
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
                            "schema-export-only requires an explicit Stream Item or Future Output",
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
                "schema-export-only cannot infer this opaque runtime return type",
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
                    "unsupported schema-export-only opaque return wrapper",
                ));
            }
            Ok(None)
        }
        Type::Paren(paren) => concrete_opaque_return(&paren.elem),
        Type::Group(group) => concrete_opaque_return(&group.elem),
        _ if contains_opaque(ty) => Err(syn::Error::new_spanned(
            ty,
            "unsupported schema-export-only opaque return shape",
        )),
        _ => Ok(None),
    }
}

#[cfg(any(feature = "schema-export-only", test))]
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
#[path = "schema_export_tests.rs"]
mod tests;
