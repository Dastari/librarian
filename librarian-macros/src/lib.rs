//! Procedural macros for Librarian
//!
//! This crate provides macros to reduce boilerplate in the Librarian backend:
//!
//! - `mutation_result!` - Generate GraphQL mutation result types

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Ident, Token, parse::Parse, parse::ParseStream};

/// Generate a GraphQL mutation result type with success, error, and optional entity field.
///
/// # Usage
///
/// ```ignore
/// // Simple result (success + error only)
/// mutation_result!(MutationResult);
///
/// // With entity field
/// mutation_result!(LibraryResult, library: Library);
/// mutation_result!(MovieResult, movie: Movie);
/// ```
///
/// # Generated Code
///
/// For `mutation_result!(LibraryResult, library: Library)`:
///
/// ```ignore
/// #[derive(Debug, Clone, async_graphql::SimpleObject)]
/// pub struct LibraryResult {
///     pub success: bool,
///     pub error: Option<String>,
///     pub library: Option<Library>,
/// }
///
/// impl LibraryResult {
///     pub fn success(library: Library) -> Self {
///         Self { success: true, error: None, library: Some(library) }
///     }
///     pub fn error(msg: impl Into<String>) -> Self {
///         Self { success: false, error: Some(msg.into()), library: None }
///     }
/// }
/// ```
#[proc_macro]
pub fn mutation_result(input: TokenStream) -> TokenStream {
    let parsed = parse_macro_input!(input as MutationResultInput);
    
    let struct_name = &parsed.name;
    
    if let Some((field_name, field_type)) = parsed.field {
        // Result with entity field
        let output = quote! {
            #[derive(Debug, Clone, async_graphql::SimpleObject)]
            pub struct #struct_name {
                pub success: bool,
                pub error: Option<String>,
                pub #field_name: Option<#field_type>,
            }
            
            impl #struct_name {
                pub fn success(#field_name: #field_type) -> Self {
                    Self {
                        success: true,
                        error: None,
                        #field_name: Some(#field_name),
                    }
                }
                
                pub fn error(msg: impl Into<String>) -> Self {
                    Self {
                        success: false,
                        error: Some(msg.into()),
                        #field_name: None,
                    }
                }
            }
        };
        output.into()
    } else {
        // Simple result (no entity field)
        let output = quote! {
            #[derive(Debug, Clone, async_graphql::SimpleObject)]
            pub struct #struct_name {
                pub success: bool,
                pub error: Option<String>,
            }
            
            impl #struct_name {
                pub fn success() -> Self {
                    Self {
                        success: true,
                        error: None,
                    }
                }
                
                pub fn error(msg: impl Into<String>) -> Self {
                    Self {
                        success: false,
                        error: Some(msg.into()),
                    }
                }
            }
        };
        output.into()
    }
}

/// Input for mutation_result! macro
struct MutationResultInput {
    name: Ident,
    field: Option<(Ident, Ident)>,
}

impl Parse for MutationResultInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: Ident = input.parse()?;
        
        let field = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            let field_name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let field_type: Ident = input.parse()?;
            Some((field_name, field_type))
        } else {
            None
        };
        
        Ok(MutationResultInput { name, field })
    }
}
