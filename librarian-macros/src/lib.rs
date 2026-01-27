//! Procedural macros for Librarian
//!
//! This crate provides macros to reduce boilerplate in the Librarian backend:
//!
//! - `mutation_result!` - Generate GraphQL mutation result types
//! - `#[derive(GraphQLEntity)]` - Generate GraphQL types, filters, and SQL helpers from a struct
//! - `#[derive(GraphQLRelations)]` - Generate relation loading with look_ahead support
//! - `#[derive(GraphQLOperations)]` - Generate Query/Mutation/Subscription structs

use convert_case::{Case, Casing};
use proc_macro::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, Data, DeriveInput, Field, Fields, Ident, Meta, Token, parse::Parse, parse::ParseStream};

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

// ============================================================================
// GraphQLEntity Derive Macro
// ============================================================================

/// Derive macro for generating GraphQL types, filters, and SQL helpers.
///
/// # Attributes
///
/// ## Struct-level:
/// - `#[graphql_entity(table = "...", plural = "...", default_sort = "...")]`
///
/// ## Field-level:
/// - `#[primary_key]` - Mark as primary key column
/// - `#[filterable(type = "string|number|boolean|date")]` - Enable filtering
/// - `#[sortable]` - Enable sorting by this field
/// - `#[db_column = "..."]` - Map to different column name
/// - `#[relation(...)]` - Define a relation (handled by GraphQLRelations)
/// - `#[skip_db]` - Skip this field in database operations
///
/// # Generated Code
///
/// For a struct `Library`, generates:
/// - `LibraryWhereInput` - GraphQL input for filtering
/// - `LibraryOrderByInput` - GraphQL input for sorting  
/// - `impl DatabaseEntity for Library`
/// - `impl FromSqlRow for Library`
/// - `impl DatabaseFilter for LibraryWhereInput`
#[proc_macro_derive(
    GraphQLEntity,
    attributes(
        graphql_entity,
        graphql,
        primary_key,
        filterable,
        sortable,
        db_column,
        relation,
        skip_db,
        date_field,
        boolean_field,
        json_field
    )
)]
pub fn derive_graphql_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match generate_graphql_entity(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for generating relation loading with look_ahead support.
#[proc_macro_derive(GraphQLRelations, attributes(graphql_entity, graphql, relation))]
pub fn derive_graphql_relations(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match generate_graphql_relations(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

// ============================================================================
// Entity Metadata Parsing
// ============================================================================

#[derive(Default)]
struct EntityMetadata {
    table_name: Option<String>,
    plural_name: Option<String>,
    default_sort: Option<String>,
    /// Notify another table when this entity changes (e.g., "libraries" to trigger LibraryChangedEvent)
    notify: Option<String>,
}

fn parse_entity_metadata(attrs: &[syn::Attribute]) -> syn::Result<EntityMetadata> {
    let mut metadata = EntityMetadata::default();
    
    for attr in attrs {
        if attr.path().is_ident("graphql_entity") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("table") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    metadata.table_name = Some(lit.value());
                } else if meta.path.is_ident("plural") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    metadata.plural_name = Some(lit.value());
                } else if meta.path.is_ident("default_sort") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    metadata.default_sort = Some(lit.value());
                } else if meta.path.is_ident("notify") {
                    let value = meta.value()?;
                    let lit: syn::LitStr = value.parse()?;
                    metadata.notify = Some(lit.value());
                }
                Ok(())
            })?;
        }
    }
    
    Ok(metadata)
}

// ============================================================================
// Field Metadata Parsing
// ============================================================================

#[derive(Default, Clone)]
struct FieldMetadata {
    graphql_name: Option<String>,
    db_column: Option<String>,
    filterable: Option<String>,
    sortable: bool,
    is_primary_key: bool,
    is_relation: bool,
    relation_target: Option<String>,
    relation_from: Option<String>,
    relation_to: Option<String>,
    relation_multiple: bool,
    skip_db: bool,
    /// Skip from Create/Update inputs only (e.g. password_hash); field remains in DB and struct
    skip_input: bool,
    is_date_field: bool,
    is_boolean_field: bool,
    is_json_field: bool,
}

fn parse_field_metadata(field: &Field) -> syn::Result<FieldMetadata> {
    let mut meta = FieldMetadata::default();
    
    for attr in &field.attrs {
        if let Some(ident) = attr.path().get_ident() {
            match ident.to_string().as_str() {
                "graphql" => {
                    let _ = attr.parse_nested_meta(|nested| {
                        if nested.path.is_ident("name") {
                            let value = nested.value()?;
                            let lit: syn::LitStr = value.parse()?;
                            meta.graphql_name = Some(lit.value());
                        } else if nested.path.is_ident("skip") {
                            // Skip from GraphQL schema and from Create/Update inputs; keep in DB
                            meta.skip_input = true;
                        }
                        Ok(())
                    });
                }
                "primary_key" => {
                    meta.is_primary_key = true;
                }
                "filterable" => {
                    if let Meta::List(_) = &attr.meta {
                        let _ = attr.parse_nested_meta(|nested| {
                            if nested.path.is_ident("type") {
                                let value = nested.value()?;
                                let lit: syn::LitStr = value.parse()?;
                                meta.filterable = Some(lit.value());
                            }
                            Ok(())
                        });
                    } else {
                        meta.filterable = Some("string".to_string());
                    }
                }
                "sortable" => {
                    meta.sortable = true;
                }
                "db_column" => {
                    if let Meta::NameValue(nv) = &attr.meta {
                        if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit), .. }) = &nv.value {
                            meta.db_column = Some(lit.value());
                        }
                    }
                }
                "relation" => {
                    meta.is_relation = true;
                    let _ = attr.parse_nested_meta(|nested| {
                        if nested.path.is_ident("target") {
                            let value = nested.value()?;
                            let lit: syn::LitStr = value.parse()?;
                            meta.relation_target = Some(lit.value());
                        } else if nested.path.is_ident("from") {
                            let value = nested.value()?;
                            let lit: syn::LitStr = value.parse()?;
                            meta.relation_from = Some(lit.value());
                        } else if nested.path.is_ident("to") {
                            let value = nested.value()?;
                            let lit: syn::LitStr = value.parse()?;
                            meta.relation_to = Some(lit.value());
                        } else if nested.path.is_ident("multiple") {
                            meta.relation_multiple = true;
                        }
                        Ok(())
                    });
                }
                "skip_db" => {
                    meta.skip_db = true;
                }
                "date_field" => {
                    meta.is_date_field = true;
                }
                "boolean_field" => {
                    meta.is_boolean_field = true;
                }
                "json_field" => {
                    meta.is_json_field = true;
                }
                _ => {}
            }
        }
    }
    
    Ok(meta)
}

fn to_pascal_case(s: &str) -> String {
    s.to_case(Case::Pascal)
}

fn to_snake_case(s: &str) -> String {
    s.to_case(Case::Snake)
}

// ============================================================================
// GraphQL Entity Code Generation
// ============================================================================

fn generate_graphql_entity(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = &input.ident;
    
    let data = match &input.data {
        Data::Struct(data) => data,
        _ => return Err(syn::Error::new_spanned(input, "GraphQLEntity can only be derived for structs")),
    };
    
    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => return Err(syn::Error::new_spanned(input, "GraphQLEntity requires named fields")),
    };
    
    let entity_meta = parse_entity_metadata(&input.attrs)?;
    let table_name = entity_meta.table_name.as_deref().unwrap_or("unknown");
    let plural_name = entity_meta.plural_name.as_deref()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{}s", struct_name));
    let default_sort = entity_meta.default_sort.as_deref().unwrap_or("id");
    
    // Collect field info
    let mut column_names: Vec<String> = Vec::new();
    let mut column_defs: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut primary_key_col: Option<String> = None;
    let mut where_input_fields = Vec::new();
    let mut order_by_fields = Vec::new();
    let mut filter_to_sql = Vec::new();
    let mut from_row_fields = Vec::new();
    let mut sortable_columns: Vec<String> = Vec::new();
    
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let field_meta = parse_field_metadata(field)?;
        
        // Skip relation fields for column list
        if field_meta.is_relation || field_meta.skip_db {
            // Initialize relation fields to empty
            if is_vec_type(field_type) {
                from_row_fields.push(quote! { #field_name: Vec::new(), });
            } else if is_option_type(field_type) {
                from_row_fields.push(quote! { #field_name: None, });
            } else {
                from_row_fields.push(quote! { #field_name: Default::default(), });
            }
            continue;
        }
        
        let rust_name = field_name.to_string();
        let graphql_name = field_meta.graphql_name.clone()
            .unwrap_or_else(|| to_pascal_case(&rust_name));
        let db_col = field_meta.db_column.clone()
            .unwrap_or_else(|| rust_name.clone());
        
        column_names.push(db_col.clone());
        
        // Determine SQL type and nullability
        let is_nullable = is_option_type(field_type);
        let is_pk = field_meta.is_primary_key;
        let sql_type = rust_type_to_sql_type(field_type, &field_meta);
        let default_val = if rust_name == "created_at" || rust_name == "updated_at" {
            Some("(datetime('now'))")
        } else {
            None
        };
        
        // Build column definition
        let default_expr = match default_val {
            Some(d) => quote! { Some(#d) },
            None => quote! { None },
        };
        
        column_defs.push(quote! {
            crate::graphql::orm::ColumnDef {
                name: #db_col,
                sql_type: #sql_type,
                nullable: #is_nullable,
                is_primary_key: #is_pk,
                default: #default_expr,
            }
        });
        
        if field_meta.is_primary_key {
            primary_key_col = Some(db_col.clone());
        }
        
        // Generate WhereInput field for filterable fields
        if let Some(ref filter_type) = field_meta.filterable {
            let (input_field, sql_gen) = generate_filter_field(
                field_name,
                &graphql_name,
                &db_col,
                filter_type,
            )?;
            where_input_fields.push(input_field);
            filter_to_sql.push(sql_gen);
        }
        
        // Generate OrderByInput field for sortable fields
        if field_meta.sortable {
            sortable_columns.push(db_col.clone());
            let order_field_name = syn::Ident::new(&to_snake_case(&graphql_name), field_name.span());
            order_by_fields.push(quote! {
                #[graphql(name = #graphql_name)]
                pub #order_field_name: Option<crate::graphql::orm::OrderDirection>,
            });
        }
        
        // Generate FromSqlRow field assignment
        let row_assignment = generate_row_field_assignment(
            field_name,
            field_type,
            &db_col,
            &field_meta,
        )?;
        from_row_fields.push(row_assignment);
    }
    
    let primary_key = primary_key_col.as_deref().unwrap_or("id");
    let columns_array: Vec<&str> = column_names.iter().map(|s| s.as_str()).collect();
    
    // Generate type names (as strings for #[graphql(name = "...")] and as idents for struct names)
    let where_input_name_str = format!("{}WhereInput", struct_name);
    let order_by_name_str = format!("{}OrderByInput", struct_name);
    let where_input_name = syn::Ident::new(&where_input_name_str, struct_name.span());
    let order_by_name = syn::Ident::new(&order_by_name_str, struct_name.span());
    
    // Generate order_by to_sql_order implementation
    let order_by_match_arms: Vec<_> = sortable_columns.iter().map(|col| {
        let field_name = syn::Ident::new(&to_snake_case(col), struct_name.span());
        quote! {
            if let Some(dir) = &self.#field_name {
                parts.push(format!("{} {}", #col, dir.to_sql()));
            }
        }
    }).collect();
    
    Ok(quote! {
        // WhereInput for filtering
        #[derive(async_graphql::InputObject, Default, Clone, Debug)]
        #[graphql(name = #where_input_name_str)]
        pub struct #where_input_name {
            #(#where_input_fields)*
            
            /// Logical AND of conditions
            #[graphql(name = "And")]
            pub and: Option<Vec<#where_input_name>>,
            
            /// Logical OR of conditions
            #[graphql(name = "Or")]
            pub or: Option<Vec<#where_input_name>>,
            
            /// Logical NOT of condition
            #[graphql(name = "Not")]
            pub not: Option<Box<#where_input_name>>,
        }
        
        // OrderByInput for sorting
        #[derive(async_graphql::InputObject, Default, Clone, Debug)]
        #[graphql(name = #order_by_name_str)]
        pub struct #order_by_name {
            #(#order_by_fields)*
        }
        
        impl crate::graphql::orm::DatabaseOrderBy for #order_by_name {
            fn to_sql_order(&self) -> Option<String> {
                let mut parts = Vec::new();
                #(#order_by_match_arms)*
                if parts.is_empty() {
                    None
                } else {
                    Some(parts.join(", "))
                }
            }
        }
        
        impl crate::graphql::orm::DatabaseFilter for #where_input_name {
            fn to_sql_conditions(&self) -> (Vec<String>, Vec<crate::graphql::orm::SqlValue>) {
                let mut conditions = Vec::new();
                let mut values = Vec::new();
                
                #(#filter_to_sql)*
                
                // Handle And
                if let Some(ref and_filters) = self.and {
                    for filter in and_filters {
                        let (sub_conds, sub_vals) = filter.to_sql_conditions();
                        conditions.extend(sub_conds);
                        values.extend(sub_vals);
                    }
                }
                
                // Handle Or
                if let Some(ref or_filters) = self.or {
                    let mut or_parts = Vec::new();
                    for filter in or_filters {
                        let (sub_conds, sub_vals) = filter.to_sql_conditions();
                        if !sub_conds.is_empty() {
                            or_parts.push(format!("({})", sub_conds.join(" AND ")));
                            values.extend(sub_vals);
                        }
                    }
                    if !or_parts.is_empty() {
                        conditions.push(format!("({})", or_parts.join(" OR ")));
                    }
                }
                
                // Handle Not
                if let Some(ref not_filter) = self.not {
                    let (sub_conds, sub_vals) = not_filter.to_sql_conditions();
                    if !sub_conds.is_empty() {
                        conditions.push(format!("NOT ({})", sub_conds.join(" AND ")));
                        values.extend(sub_vals);
                    }
                }
                
                (conditions, values)
            }
            
            fn is_empty(&self) -> bool {
                // Check if all filter fields are None/empty
                let (conds, _) = self.to_sql_conditions();
                conds.is_empty()
            }
        }
        
        impl crate::graphql::orm::DatabaseEntity for #struct_name {
            const TABLE_NAME: &'static str = #table_name;
            const PLURAL_NAME: &'static str = #plural_name;
            const PRIMARY_KEY: &'static str = #primary_key;
            const DEFAULT_SORT: &'static str = #default_sort;
            
            fn column_names() -> &'static [&'static str] {
                &[#(#columns_array),*]
            }
        }
        
        impl crate::graphql::orm::DatabaseSchema for #struct_name {
            fn columns() -> &'static [crate::graphql::orm::ColumnDef] {
                static COLUMNS: &[crate::graphql::orm::ColumnDef] = &[
                    #(#column_defs),*
                ];
                COLUMNS
            }
        }
        
        impl crate::graphql::orm::FromSqlRow for #struct_name {
            fn from_row(row: &sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
                use sqlx::Row;
                use crate::db::sqlite_helpers::*;
                
                Ok(Self {
                    #(#from_row_fields)*
                })
            }
        }
    })
}

// ============================================================================
// Filter Field Generation
// ============================================================================

fn generate_filter_field(
    field_name: &syn::Ident,
    graphql_name: &str,
    db_col: &str,
    filter_type: &str,
) -> syn::Result<(proc_macro2::TokenStream, proc_macro2::TokenStream)> {
    let filter_field_name = syn::Ident::new(&to_snake_case(graphql_name), field_name.span());
    
    match filter_type {
        "string" => {
            let input = quote! {
                #[graphql(name = #graphql_name)]
                pub #filter_field_name: Option<crate::graphql::filters::StringFilter>,
            };
            let sql = quote! {
                if let Some(ref f) = self.#filter_field_name {
                    if let Some(ref v) = f.eq {
                        conditions.push(format!("{} = ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(v.clone()));
                    }
                    if let Some(ref v) = f.ne {
                        conditions.push(format!("{} != ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(v.clone()));
                    }
                    if let Some(ref v) = f.contains {
                        // Case-insensitive contains using LOWER()
                        conditions.push(format!("LOWER({}) LIKE LOWER(?)", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(format!("%{}%", v)));
                    }
                    if let Some(ref v) = f.starts_with {
                        conditions.push(format!("LOWER({}) LIKE LOWER(?)", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(format!("{}%", v)));
                    }
                    if let Some(ref v) = f.ends_with {
                        conditions.push(format!("LOWER({}) LIKE LOWER(?)", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(format!("%{}", v)));
                    }
                    if let Some(ref list) = f.in_list {
                        if !list.is_empty() {
                            let placeholders: Vec<&str> = list.iter().map(|_| "?").collect();
                            conditions.push(format!("{} IN ({})", #db_col, placeholders.join(", ")));
                            for v in list {
                                values.push(crate::graphql::orm::SqlValue::String(v.clone()));
                            }
                        }
                    }
                    if let Some(ref list) = f.not_in {
                        if !list.is_empty() {
                            let placeholders: Vec<&str> = list.iter().map(|_| "?").collect();
                            conditions.push(format!("{} NOT IN ({})", #db_col, placeholders.join(", ")));
                            for v in list {
                                values.push(crate::graphql::orm::SqlValue::String(v.clone()));
                            }
                        }
                    }
                    // IsNull / IsNotNull
                    if let Some(is_null) = f.is_null {
                        if is_null {
                            conditions.push(format!("{} IS NULL", #db_col));
                        } else {
                            conditions.push(format!("{} IS NOT NULL", #db_col));
                        }
                    }
                    // Similar/fuzzy matching - use LIKE for candidate filtering
                    // Actual scoring happens in Rust post-processing
                    if let Some(ref sim) = f.similar {
                        // Use a broad LIKE pattern to get candidates
                        // Fuzzy scoring with strsim happens after fetch
                        let pattern = crate::graphql::orm::generate_candidate_pattern(&sim.value);
                        conditions.push(format!("LOWER({}) LIKE LOWER(?)", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(pattern));
                    }
                }
            };
            Ok((input, sql))
        }
        "number" => {
            let input = quote! {
                #[graphql(name = #graphql_name)]
                pub #filter_field_name: Option<crate::graphql::filters::IntFilter>,
            };
            let sql = quote! {
                if let Some(ref f) = self.#filter_field_name {
                    if let Some(v) = f.eq {
                        conditions.push(format!("{} = ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::Int(v as i64));
                    }
                    if let Some(v) = f.ne {
                        conditions.push(format!("{} != ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::Int(v as i64));
                    }
                    if let Some(v) = f.lt {
                        conditions.push(format!("{} < ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::Int(v as i64));
                    }
                    if let Some(v) = f.lte {
                        conditions.push(format!("{} <= ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::Int(v as i64));
                    }
                    if let Some(v) = f.gt {
                        conditions.push(format!("{} > ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::Int(v as i64));
                    }
                    if let Some(v) = f.gte {
                        conditions.push(format!("{} >= ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::Int(v as i64));
                    }
                    if let Some(ref list) = f.in_list {
                        if !list.is_empty() {
                            let placeholders: Vec<&str> = list.iter().map(|_| "?").collect();
                            conditions.push(format!("{} IN ({})", #db_col, placeholders.join(", ")));
                            for v in list {
                                values.push(crate::graphql::orm::SqlValue::Int(*v as i64));
                            }
                        }
                    }
                    if let Some(ref list) = f.not_in {
                        if !list.is_empty() {
                            let placeholders: Vec<&str> = list.iter().map(|_| "?").collect();
                            conditions.push(format!("{} NOT IN ({})", #db_col, placeholders.join(", ")));
                            for v in list {
                                values.push(crate::graphql::orm::SqlValue::Int(*v as i64));
                            }
                        }
                    }
                    // IsNull / IsNotNull
                    if let Some(is_null) = f.is_null {
                        if is_null {
                            conditions.push(format!("{} IS NULL", #db_col));
                        } else {
                            conditions.push(format!("{} IS NOT NULL", #db_col));
                        }
                    }
                }
            };
            Ok((input, sql))
        }
        "boolean" | "bool" => {
            let input = quote! {
                #[graphql(name = #graphql_name)]
                pub #filter_field_name: Option<crate::graphql::filters::BoolFilter>,
            };
            let sql = quote! {
                if let Some(ref f) = self.#filter_field_name {
                    if let Some(v) = f.eq {
                        conditions.push(format!("{} = ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::Int(if v { 1 } else { 0 }));
                    }
                    if let Some(v) = f.ne {
                        conditions.push(format!("{} != ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::Int(if v { 1 } else { 0 }));
                    }
                    // IsNull / IsNotNull
                    if let Some(is_null) = f.is_null {
                        if is_null {
                            conditions.push(format!("{} IS NULL", #db_col));
                        } else {
                            conditions.push(format!("{} IS NOT NULL", #db_col));
                        }
                    }
                }
            };
            Ok((input, sql))
        }
        "date" => {
            let input = quote! {
                #[graphql(name = #graphql_name)]
                pub #filter_field_name: Option<crate::graphql::filters::DateFilter>,
            };
            let sql = quote! {
                if let Some(ref f) = self.#filter_field_name {
                    if let Some(ref v) = f.eq {
                        conditions.push(format!("{} = ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(v.clone()));
                    }
                    if let Some(ref v) = f.ne {
                        conditions.push(format!("{} != ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(v.clone()));
                    }
                    if let Some(ref v) = f.lt {
                        conditions.push(format!("{} < ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(v.clone()));
                    }
                    if let Some(ref v) = f.lte {
                        conditions.push(format!("{} <= ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(v.clone()));
                    }
                    if let Some(ref v) = f.gt {
                        conditions.push(format!("{} > ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(v.clone()));
                    }
                    if let Some(ref v) = f.gte {
                        conditions.push(format!("{} >= ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(v.clone()));
                    }
                    if let Some(ref range) = f.between {
                        if let (Some(ref start), Some(ref end)) = (&range.start, &range.end) {
                            conditions.push(format!("{} BETWEEN ? AND ?", #db_col));
                            values.push(crate::graphql::orm::SqlValue::String(start.clone()));
                            values.push(crate::graphql::orm::SqlValue::String(end.clone()));
                        }
                    }
                    // IsNull / IsNotNull
                    if let Some(is_null) = f.is_null {
                        if is_null {
                            conditions.push(format!("{} IS NULL", #db_col));
                        } else {
                            conditions.push(format!("{} IS NOT NULL", #db_col));
                        }
                    }
                    // Date arithmetic operators
                    if f.in_past == Some(true) {
                        conditions.push(format!("{} < date('now')", #db_col));
                    }
                    if f.in_future == Some(true) {
                        conditions.push(format!("{} > date('now')", #db_col));
                    }
                    if f.is_today == Some(true) {
                        conditions.push(format!("{} = date('now')", #db_col));
                    }
                    if let Some(days) = f.recent_days {
                        // Within the last N days (inclusive of today)
                        conditions.push(format!("{} >= date('now', '-{} days') AND {} <= date('now')", #db_col, days, #db_col));
                    }
                    if let Some(days) = f.within_days {
                        // Within the next N days (inclusive of today)
                        conditions.push(format!("{} >= date('now') AND {} <= date('now', '+{} days')", #db_col, #db_col, days));
                    }
                    if let Some(ref rel) = f.gte_relative {
                        let expr = rel.to_sql_expr();
                        conditions.push(format!("{} >= {}", #db_col, expr));
                    }
                    if let Some(ref rel) = f.lte_relative {
                        let expr = rel.to_sql_expr();
                        conditions.push(format!("{} <= {}", #db_col, expr));
                    }
                }
            };
            Ok((input, sql))
        }
        _ => Err(syn::Error::new(
            field_name.span(),
            format!("Unsupported filter type: {}", filter_type),
        )),
    }
}

// ============================================================================
// Row Field Assignment Generation
// ============================================================================

fn generate_row_field_assignment(
    field_name: &syn::Ident,
    field_type: &syn::Type,
    db_col: &str,
    meta: &FieldMetadata,
) -> syn::Result<proc_macro2::TokenStream> {
    // Handle special field types
    if meta.is_date_field {
        return Ok(quote! {
            #field_name: {
                let s: Option<String> = row.try_get(#db_col)?;
                s.and_then(|s| str_to_datetime(&s).ok())
            },
        });
    }
    
    if meta.is_boolean_field {
        return Ok(quote! {
            #field_name: {
                let i: i32 = row.try_get(#db_col)?;
                int_to_bool(i)
            },
        });
    }
    
    if meta.is_json_field {
        return Ok(quote! {
            #field_name: {
                let s: String = row.try_get(#db_col)?;
                json_to_vec(&s)
            },
        });
    }
    
    // Check type and generate appropriate code
    if let syn::Type::Path(type_path) = field_type {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = segment.ident.to_string();
            
            match type_name.as_str() {
                "String" => {
                    return Ok(quote! {
                        #field_name: row.try_get(#db_col)?,
                    });
                }
                "i32" | "i64" => {
                    return Ok(quote! {
                        #field_name: row.try_get(#db_col)?,
                    });
                }
                "f32" | "f64" => {
                    return Ok(quote! {
                        #field_name: row.try_get(#db_col)?,
                    });
                }
                "bool" => {
                    return Ok(quote! {
                        #field_name: {
                            let i: i32 = row.try_get(#db_col)?;
                            int_to_bool(i)
                        },
                    });
                }
                "Uuid" => {
                    return Ok(quote! {
                        #field_name: {
                            let s: String = row.try_get(#db_col)?;
                            str_to_uuid(&s).map_err(|e| sqlx::Error::Decode(e.into()))?
                        },
                    });
                }
                "Option" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_type)) = args.args.first() {
                            return generate_option_row_field_assignment(field_name, inner_type, db_col);
                        }
                    }
                }
                "Vec" => {
                    // JSON array field
                    return Ok(quote! {
                        #field_name: {
                            let s: String = row.try_get(#db_col)?;
                            json_to_vec(&s)
                        },
                    });
                }
                "DateTime" => {
                    return Ok(quote! {
                        #field_name: {
                            let s: String = row.try_get(#db_col)?;
                            str_to_datetime(&s).map_err(|e| sqlx::Error::Decode(e.into()))?
                        },
                    });
                }
                _ => {}
            }
        }
    }
    
    // Default: try direct get
    Ok(quote! {
        #field_name: row.try_get(#db_col)?,
    })
}

fn generate_option_row_field_assignment(
    field_name: &syn::Ident,
    inner_type: &syn::Type,
    db_col: &str,
) -> syn::Result<proc_macro2::TokenStream> {
    if let syn::Type::Path(inner_path) = inner_type {
        if let Some(segment) = inner_path.path.segments.last() {
            let inner_name = segment.ident.to_string();
            
            match inner_name.as_str() {
                "String" => {
                    return Ok(quote! {
                        #field_name: row.try_get(#db_col)?,
                    });
                }
                "i32" | "i64" => {
                    return Ok(quote! {
                        #field_name: row.try_get(#db_col)?,
                    });
                }
                "f32" | "f64" => {
                    return Ok(quote! {
                        #field_name: row.try_get(#db_col)?,
                    });
                }
                "bool" => {
                    return Ok(quote! {
                        #field_name: {
                            let i: Option<i32> = row.try_get(#db_col)?;
                            i.map(int_to_bool)
                        },
                    });
                }
                "Uuid" => {
                    return Ok(quote! {
                        #field_name: {
                            let s: Option<String> = row.try_get(#db_col)?;
                            s.map(|s| str_to_uuid(&s))
                                .transpose()
                                .map_err(|e| sqlx::Error::Decode(e.into()))?
                        },
                    });
                }
                "DateTime" => {
                    return Ok(quote! {
                        #field_name: {
                            let s: Option<String> = row.try_get(#db_col)?;
                            s.map(|s| str_to_datetime(&s))
                                .transpose()
                                .map_err(|e| sqlx::Error::Decode(e.into()))?
                        },
                    });
                }
                "Vec" => {
                    return Ok(quote! {
                        #field_name: {
                            let s: Option<String> = row.try_get(#db_col)?;
                            s.map(|s| json_to_vec(&s))
                        },
                    });
                }
                _ => {}
            }
        }
    }
    
    Ok(quote! {
        #field_name: row.try_get(#db_col)?,
    })
}

fn is_vec_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Vec";
        }
    }
    false
}

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn is_bool_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "bool" {
                return true;
            }
            // Check for Option<bool>
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return is_bool_type(inner);
                    }
                }
            }
        }
    }
    false
}

/// Convert Rust type to SQLite type string
fn rust_type_to_sql_type(ty: &syn::Type, meta: &FieldMetadata) -> &'static str {
    // Handle Option<T> by unwrapping
    let inner_type = if is_option_type(ty) {
        if let syn::Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        inner
                    } else {
                        ty
                    }
                } else {
                    ty
                }
            } else {
                ty
            }
        } else {
            ty
        }
    } else {
        ty
    };
    
    // Check field metadata first
    if meta.is_boolean_field {
        return "INTEGER";
    }
    if meta.is_json_field {
        return "TEXT";
    }
    if meta.is_date_field {
        return "TEXT";
    }
    
    // Infer from Rust type first (so f64 becomes REAL not INTEGER for "number" filter)
    if let syn::Type::Path(type_path) = inner_type {
        if let Some(segment) = type_path.path.segments.last() {
            let type_name = segment.ident.to_string();
            match type_name.as_str() {
                "String" | "str" => return "TEXT",
                "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize" => return "INTEGER",
                "f32" | "f64" => return "REAL",
                "bool" => return "INTEGER",
                "Vec" => return "TEXT", // JSON array
                _ => return "TEXT", // Default to TEXT for unknown types
            }
        }
    }
    
    "TEXT"
}

// ============================================================================
// GraphQL Relations Code Generation
// ============================================================================

/// Relation definition parsed from attributes
#[derive(Clone)]
struct RelationDef {
    field_name: syn::Ident,
    graphql_name: String,
    target_type_str: String,
    fk_column: String,
    is_multiple: bool,
}

fn generate_graphql_relations(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = &input.ident;
    
    let data = match &input.data {
        Data::Struct(data) => data,
        _ => return Err(syn::Error::new_spanned(input, "GraphQLRelations can only be derived for structs")),
    };
    
    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => return Err(syn::Error::new_spanned(input, "GraphQLRelations requires named fields")),
    };
    
    let _entity_meta = parse_entity_metadata(&input.attrs)?;
    
    // Find primary key field
    let mut pk_field_name: Option<syn::Ident> = None;
    for field in fields {
        let meta = parse_field_metadata(field)?;
        if meta.is_primary_key {
            pk_field_name = Some(field.ident.clone().unwrap());
            break;
        }
    }
    let pk_field = pk_field_name.unwrap_or_else(|| syn::Ident::new("id", struct_name.span()));
    
    // Collect relations
    let mut relations: Vec<RelationDef> = Vec::new();
    
    for field in fields {
        let meta = parse_field_metadata(field)?;
        if !meta.is_relation {
            continue;
        }
        
        let field_name = field.ident.clone().unwrap();
        let rust_name = field_name.to_string();
        let graphql_name = meta.graphql_name.clone()
            .unwrap_or_else(|| to_pascal_case(&rust_name));
        
        let target_type = meta.relation_target.clone()
            .unwrap_or_else(|| "Unknown".to_string());
        let to_col = meta.relation_to.clone()
            .unwrap_or_else(|| "unknown_id".to_string());
        let is_multiple = meta.relation_multiple;
        
        relations.push(RelationDef {
            field_name,
            graphql_name,
            target_type_str: target_type,
            fk_column: to_col,
            is_multiple,
        });
    }
    
    // Generate relation metadata
    let relation_metadata: Vec<_> = relations.iter().map(|r| {
        let graphql_name = &r.graphql_name;
        let target_type = &r.target_type_str;
        let is_multiple = r.is_multiple;
        quote! {
            crate::graphql::orm::RelationMetadata {
                field_name: #graphql_name,
                target_type: #target_type,
                is_multiple: #is_multiple,
            }
        }
    }).collect();
    
    // Generate ComplexObject resolver methods for relations with filtering/sorting/pagination
    //
    // Strategy:
    // - When NO filter/sort/pagination args provided: Use DataLoader for batching (N+1 free)
    // - When args ARE provided: Use direct database query (supports full SQL filtering)
    //
    // This gives optimal performance for simple relation traversal while keeping
    // full filter/sort/pagination support for complex queries.
    let relation_resolvers: Vec<_> = relations.iter().map(|r| {
        let field_name = &r.field_name;
        let graphql_name = &r.graphql_name;
        let fk_column = &r.fk_column;
        
        // Generate type name strings for use in fully-qualified paths
        let target_type_str = &r.target_type_str;
        let where_input_str = format!("{}WhereInput", r.target_type_str);
        let order_by_input_str = format!("{}OrderByInput", r.target_type_str);
        let connection_type_str = format!("{}Connection", r.target_type_str);
        let edge_type_str = format!("{}Edge", r.target_type_str);
        
        // Create idents for local use in the macro
        let target_type = syn::Ident::new(target_type_str, struct_name.span());
        let where_input = syn::Ident::new(&where_input_str, struct_name.span());
        let order_by_input = syn::Ident::new(&order_by_input_str, struct_name.span());
        let connection_type = syn::Ident::new(&connection_type_str, struct_name.span());
        let edge_type = syn::Ident::new(&edge_type_str, struct_name.span());
        
        if r.is_multiple {
            // One-to-many relation with smart batching
            quote! {
                /// Get related #graphql_name with optional filtering, sorting, and pagination.
                ///
                /// When no arguments are provided, uses DataLoader to batch queries and
                /// avoid N+1 when loading relations for multiple parent entities.
                /// When filter/sort/pagination arguments are provided, uses direct
                /// database query for full SQL support.
                #[graphql(name = #graphql_name)]
                async fn #field_name(
                    &self,
                    ctx: &async_graphql::Context<'_>,
                    #[graphql(name = "Where")] where_input: Option<crate::graphql::entities::#where_input>,
                    #[graphql(name = "OrderBy")] order_by: Option<crate::graphql::entities::#order_by_input>,
                    #[graphql(name = "Page")] page: Option<crate::graphql::orm::PageInput>,
                ) -> async_graphql::Result<crate::graphql::entities::#connection_type> {
                    use crate::graphql::entities::#target_type;
                    use crate::graphql::entities::#connection_type;
                    use crate::graphql::entities::#edge_type;
                    use crate::graphql::orm::{DatabaseEntity, DatabaseFilter, DatabaseOrderBy, EntityQuery, SqlValue};
                    
                    let db = ctx.data_unchecked::<crate::db::Database>();
                    
                    // Check if we can use DataLoader (no filter/sort/pagination args)
                    let use_dataloader = where_input.is_none() && order_by.is_none() && page.is_none();
                    
                    let entities: Vec<#target_type> = if use_dataloader {
                        // Fast path: Use DataLoader for batched loading
                        use crate::graphql::loaders::RelationLoader;
                        use async_graphql::dataloader::DataLoader;
                        
                        let loader = ctx.data_unchecked::<DataLoader<RelationLoader<#target_type>>>();
                        loader
                            .load_one(self.id.clone())
                            .await
                            .map_err(|e| async_graphql::Error::new(e.to_string()))?
                            .unwrap_or_default()
                    } else {
                        // Slow path: Use direct query with full SQL support
                        let mut query = EntityQuery::<#target_type>::new()
                            .where_clause(
                                &format!("{} = ?", #fk_column),
                                SqlValue::String(self.id.clone())
                            );
                        
                        if let Some(ref filter) = where_input {
                            query = query.filter(filter);
                        }
                        
                        if let Some(ref order) = order_by {
                            query = query.order_by(order);
                        }
                        
                        if query.order_clauses.is_empty() {
                            query = query.default_order();
                        }
                        
                        if let Some(ref p) = page {
                            query = query.paginate(p);
                        }
                        
                        query.fetch_all(db)
                            .await
                            .map_err(|e| async_graphql::Error::new(e.to_string()))?
                    };
                    
                    // Build connection response
                    let total = entities.len() as i64;
                    let offset = page.as_ref().map(|p| p.offset()).unwrap_or(0) as usize;
                    
                    // For DataLoader path, we already have all results (no server-side pagination)
                    // For direct query path, pagination was applied server-side
                    let has_next_page = if use_dataloader {
                        false // We loaded all results
                    } else {
                        // This is an approximation; true count would need a separate query
                        page.as_ref().map(|p| entities.len() as i64 >= p.limit()).unwrap_or(false)
                    };
                    let has_previous_page = offset > 0;
                    
                    let edges: Vec<#edge_type> = entities
                        .into_iter()
                        .enumerate()
                        .map(|(i, entity)| #edge_type {
                            cursor: crate::graphql::pagination::encode_cursor((offset + i) as i64),
                            node: entity,
                        })
                        .collect();
                    
                    let page_info = crate::graphql::pagination::PageInfo {
                        has_next_page,
                        has_previous_page,
                        start_cursor: edges.first().map(|e| e.cursor.clone()),
                        end_cursor: edges.last().map(|e| e.cursor.clone()),
                        total_count: Some(total),
                    };
                    
                    Ok(#connection_type { edges, page_info })
                }
            }
        } else {
            // Single relation (many-to-one) - uses direct query
            // TODO: Could batch these too with a reverse lookup pattern
            quote! {
                /// Get related #graphql_name
                #[graphql(name = #graphql_name)]
                async fn #field_name(
                    &self,
                    ctx: &async_graphql::Context<'_>,
                ) -> async_graphql::Result<Option<crate::graphql::entities::#target_type>> {
                    use crate::graphql::orm::{DatabaseEntity, EntityQuery, SqlValue};
                    use crate::graphql::entities::#target_type;
                    
                    let db = ctx.data_unchecked::<crate::db::Database>();
                    
                    let result = EntityQuery::<#target_type>::new()
                        .where_clause(
                            &format!("{} = ?", #target_type::PRIMARY_KEY),
                            SqlValue::String(self.id.clone())
                        )
                        .fetch_one(db)
                        .await
                        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                    
                    Ok(result)
                }
            }
        }
    }).collect();
    
    // Generate simple RelationLoader impl (for backward compatibility)
    let single_load_blocks: Vec<proc_macro2::TokenStream> = Vec::new();
    let bulk_load_blocks: Vec<proc_macro2::TokenStream> = Vec::new();
    
    let struct_name_str = struct_name.to_string();
    let has_relations = !relations.is_empty();
    
    // Only generate ComplexObject impl if there are relations
    let complex_object_impl = if has_relations {
        quote! {
            #[async_graphql::ComplexObject]
            impl #struct_name {
                #(#relation_resolvers)*
            }
        }
    } else {
        quote! {}
    };
    
    Ok(quote! {
        impl crate::graphql::orm::RelationLoader for #struct_name {
            async fn load_relations(
                &mut self,
                pool: &sqlx::SqlitePool,
                selection: &[async_graphql::context::SelectionField<'_>],
            ) -> Result<(), sqlx::Error> {
                #(#single_load_blocks)*
                Ok(())
            }
            
            async fn bulk_load_relations(
                entities: &mut [Self],
                pool: &sqlx::SqlitePool,
                selection: &[async_graphql::context::SelectionField<'_>],
            ) -> Result<(), sqlx::Error> {
                #(#bulk_load_blocks)*
                Ok(())
            }
        }
        
        impl #struct_name {
            /// Get relation metadata for look_ahead traversal
            pub fn relation_metadata() -> &'static [crate::graphql::orm::RelationMetadata] {
                static RELATIONS: &[crate::graphql::orm::RelationMetadata] = &[
                    #(#relation_metadata),*
                ];
                RELATIONS
            }
            
            /// Get entity name for relation registry
            pub fn entity_name() -> &'static str {
                #struct_name_str
            }
        }
        
        #complex_object_impl
    })
}

fn get_inner_type(ty: &syn::Type) -> syn::Result<proc_macro2::TokenStream> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Vec" || segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return Ok(quote! { #inner });
                    }
                }
            }
        }
    }
    Ok(quote! { #ty })
}

// ============================================================================
// GraphQLOperations Derive Macro
// ============================================================================

/// Derive macro for generating Query, Mutation, and Subscription structs.
///
/// This macro generates complete CRUD operations for a GraphQL entity:
///
/// # Generated Queries
/// - `{PluralName}(Where, OrderBy, Page)` - List with filtering, sorting, pagination
/// - `{EntityName}(Id)` - Get single entity by ID
///
/// # Generated Mutations  
/// - `Create{EntityName}(Input)` - Create new entity
/// - `Update{EntityName}(Id, Input)` - Update existing entity
/// - `Delete{EntityName}(Id)` - Delete single entity
/// - `Delete{PluralName}(Where)` - Delete multiple entities matching Where filter
///
/// # Generated Subscriptions
/// - `{EntityName}Changed(Filter)` - Real-time updates
///
/// # Usage
///
/// ```ignore
/// #[derive(GraphQLEntity, GraphQLRelations, GraphQLOperations)]
/// #[graphql_entity(table = "libraries", plural = "Libraries")]
/// pub struct Library { ... }
/// ```
///
/// Then merge into your schema:
/// ```ignore
/// #[derive(MergedObject)]
/// pub struct QueryRoot(LibraryQueries, MovieQueries, ...);
/// ```
#[proc_macro_derive(GraphQLOperations, attributes(graphql_entity, graphql, primary_key))]
pub fn derive_graphql_operations(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match generate_graphql_operations(&input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn generate_graphql_operations(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();
    
    let data = match &input.data {
        Data::Struct(data) => data,
        _ => return Err(syn::Error::new_spanned(input, "GraphQLOperations can only be derived for structs")),
    };
    
    let fields = match &data.fields {
        Fields::Named(fields) => &fields.named,
        _ => return Err(syn::Error::new_spanned(input, "GraphQLOperations requires named fields")),
    };
    
    let entity_meta = parse_entity_metadata(&input.attrs)?;
    let table_name = entity_meta.table_name.as_deref().unwrap_or("unknown");
    let plural_name = entity_meta.plural_name.clone()
        .unwrap_or_else(|| format!("{}s", struct_name));
    
    // Generate notification code if notify attribute is set
    // Currently supports notify="libraries" for entities with library_id field
    let library_id_field = fields.iter().find(|field| {
        field.ident.as_ref().map(|ident| ident == "library_id").unwrap_or(false)
    });
    let library_id_is_option = library_id_field.map(|field| is_option_type(&field.ty)).unwrap_or(false);
    let notify_on_change = if let Some(ref notify_target) = entity_meta.notify {
        if notify_target == "libraries" && library_id_field.is_some() {
            let library_id_expr = if library_id_is_option {
                quote! { entity.library_id.clone() }
            } else {
                quote! { Some(entity.library_id.clone()) }
            };
            // Generate code to broadcast LibraryChangedEvent
            quote! {
                // Notify library of changes (if entity has library_id)
                if let Some(ref lib_id) = #library_id_expr {
                    if let Ok(tx) = ctx.data::<tokio::sync::broadcast::Sender<crate::graphql::entities::LibraryChangedEvent>>() {
                        // Fetch the library entity to include in the event
                        if let Ok(Some(lib)) = crate::graphql::entities::Library::get(pool, lib_id.as_str()).await {
                            let _ = tx.send(crate::graphql::entities::LibraryChangedEvent {
                                action: crate::graphql::orm::ChangeAction::Updated,
                                id: lib_id.clone(),
                                entity: Some(lib),
                            });
                        }
                    }
                }
            }
        } else {
            // Unsupported notify target or missing library_id, generate no-op
            quote! {}
        }
    } else {
        // No notify attribute
        quote! {}
    };
    
    // Find primary key field
    let mut pk_field_name: Option<syn::Ident> = None;
    let mut pk_type: Option<proc_macro2::TokenStream> = None;
    for field in fields {
        let meta = parse_field_metadata(field)?;
        if meta.is_primary_key {
            pk_field_name = Some(field.ident.clone().unwrap());
            let ty = &field.ty;
            pk_type = Some(quote! { #ty });
            break;
        }
    }
    let pk_field = pk_field_name.clone().unwrap_or_else(|| syn::Ident::new("id", struct_name.span()));
    let pk_type = pk_type.unwrap_or_else(|| quote! { String });
    
    // Generate type names
    let queries_struct = syn::Ident::new(&format!("{}Queries", struct_name), struct_name.span());
    let mutations_struct = syn::Ident::new(&format!("{}Mutations", struct_name), struct_name.span());
    let subscriptions_struct = syn::Ident::new(&format!("{}Subscriptions", struct_name), struct_name.span());
    let where_input = syn::Ident::new(&format!("{}WhereInput", struct_name), struct_name.span());
    let order_by_input = syn::Ident::new(&format!("{}OrderByInput", struct_name), struct_name.span());
    let create_input = syn::Ident::new(&format!("Create{}Input", struct_name), struct_name.span());
    let update_input = syn::Ident::new(&format!("Update{}Input", struct_name), struct_name.span());
    let result_type = syn::Ident::new(&format!("{}Result", struct_name), struct_name.span());
    let changed_event = syn::Ident::new(&format!("{}ChangedEvent", struct_name), struct_name.span());
    
    // GraphQL operation names (PascalCase)
    let list_query_name = &plural_name;
    let single_query_name = &struct_name_str;
    let create_mutation_name = format!("Create{}", struct_name);
    let update_mutation_name = format!("Update{}", struct_name);
    let delete_mutation_name = format!("Delete{}", struct_name);
    let delete_many_mutation_name = format!("Delete{}", plural_name);
    let subscription_name = format!("{}Changed", struct_name);
    let delete_many_result_type = syn::Ident::new(&format!("Delete{}Result", plural_name), struct_name.span());
    let delete_many_result_type_str = format!("Delete{}Result", plural_name);
    
    // Generate input fields (excluding primary key for create, all optional for update)
    let mut create_input_fields: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut update_input_fields: Vec<proc_macro2::TokenStream> = Vec::new();
    
    // For SQL generation
    let mut insert_columns: Vec<String> = Vec::new();
    let mut insert_binds: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut update_field_checks: Vec<proc_macro2::TokenStream> = Vec::new();
    
    // Track string-filterable fields for search_similar
    let mut string_filterable_fields: Vec<(syn::Ident, bool)> = Vec::new(); // (field_name, is_option)
    
    for field in fields {
        let field_name = field.ident.as_ref().unwrap();
        let field_type = &field.ty;
        let meta = parse_field_metadata(field)?;
        
        // Skip relations and computed fields
        if meta.is_relation || meta.skip_db {
            continue;
        }
        
        let rust_name = field_name.to_string();
        let graphql_name = meta.graphql_name.clone()
            .unwrap_or_else(|| to_pascal_case(&rust_name));
        let db_col = meta.db_column.clone().unwrap_or_else(|| rust_name.clone());
        
        // Track string-filterable fields for fuzzy search
        if meta.filterable.as_deref() == Some("string") {
            string_filterable_fields.push((field_name.clone(), is_option_type(field_type)));
        }
        
        // Skip primary key and skip_input fields (e.g. password_hash) for create input
        if !meta.is_primary_key && !meta.skip_input {
            // For create: use the field type directly (required fields stay required)
            create_input_fields.push(quote! {
                #[graphql(name = #graphql_name)]
                pub #field_name: #field_type,
            });
            
            // Track columns for INSERT
            insert_columns.push(db_col.clone());
            
            // Generate bind value push based on field type
            // We push to bind_values vector to avoid lifetime issues with sqlx::query
            if meta.is_boolean_field || is_bool_type(field_type) {
                if is_option_type(field_type) {
                    insert_binds.push(quote! {
                        match input.#field_name {
                            Some(b) => bind_values.push(crate::graphql::orm::SqlValue::Int(if b { 1 } else { 0 })),
                            None => bind_values.push(crate::graphql::orm::SqlValue::Null),
                        }
                    });
                } else {
                    insert_binds.push(quote! {
                        bind_values.push(crate::graphql::orm::SqlValue::Int(if input.#field_name { 1 } else { 0 }));
                    });
                }
            } else if meta.is_json_field || is_vec_type(field_type) {
                insert_binds.push(quote! {
                    bind_values.push(crate::graphql::orm::SqlValue::String(
                        serde_json::to_string(&input.#field_name).unwrap_or_else(|_| "[]".to_string())
                    ));
                });
            } else if is_option_type(field_type) {
                insert_binds.push(quote! {
                    match &input.#field_name {
                        Some(v) => bind_values.push(crate::graphql::orm::SqlValue::String(v.to_string())),
                        None => bind_values.push(crate::graphql::orm::SqlValue::Null),
                    }
                });
            } else {
                insert_binds.push(quote! {
                    bind_values.push(crate::graphql::orm::SqlValue::String(input.#field_name.to_string()));
                });
            }
        }
        
        // For update: wrap in Option to make all fields optional (skip PK, timestamps, skip_input)
        let is_timestamp = rust_name == "created_at" || rust_name == "updated_at";
        if !meta.is_primary_key && !is_timestamp && !meta.skip_input {
            // All update fields are wrapped in Option (even if already optional)
            // This allows distinguishing between "not provided" and "set to null"
            let update_type = quote! { Option<#field_type> };
            
            update_input_fields.push(quote! {
                #[graphql(name = #graphql_name)]
                pub #field_name: #update_type,
            });
            
            // Generate update field check
            let is_already_optional = is_option_type(field_type);
            
            if meta.is_boolean_field || is_bool_type(field_type) {
                if is_already_optional {
                    // Option<Option<bool>> case
                    update_field_checks.push(quote! {
                        if let Some(ref val) = input.#field_name {
                            set_clauses.push(format!("{} = ?", #db_col));
                            match val {
                                Some(b) => values.push(crate::graphql::orm::SqlValue::Int(if *b { 1 } else { 0 })),
                                None => values.push(crate::graphql::orm::SqlValue::Null),
                            }
                        }
                    });
                } else {
                    // Option<bool> case
                    update_field_checks.push(quote! {
                        if let Some(ref val) = input.#field_name {
                            set_clauses.push(format!("{} = ?", #db_col));
                            values.push(crate::graphql::orm::SqlValue::Int(if *val { 1 } else { 0 }));
                        }
                    });
                }
            } else if meta.is_json_field || is_vec_type(field_type) {
                update_field_checks.push(quote! {
                    if let Some(ref val) = input.#field_name {
                        set_clauses.push(format!("{} = ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(
                            serde_json::to_string(val).unwrap_or_else(|_| "[]".to_string())
                        ));
                    }
                });
            } else if is_already_optional {
                // Field type is already Option<T>, update type is Option<Option<T>>
                update_field_checks.push(quote! {
                    if let Some(ref val) = input.#field_name {
                        set_clauses.push(format!("{} = ?", #db_col));
                        match val {
                            Some(v) => values.push(crate::graphql::orm::SqlValue::String(v.to_string())),
                            None => values.push(crate::graphql::orm::SqlValue::Null),
                        }
                    }
                });
            } else {
                // Field type is T, update type is Option<T>
                update_field_checks.push(quote! {
                    if let Some(ref val) = input.#field_name {
                        set_clauses.push(format!("{} = ?", #db_col));
                        values.push(crate::graphql::orm::SqlValue::String(val.to_string()));
                    }
                });
            }
        }
    }
    
    // Build INSERT SQL template
    let insert_cols_str = insert_columns.join(", ");
    let insert_placeholders: Vec<&str> = insert_columns.iter().map(|_| "?").collect();
    let insert_placeholders_str = insert_placeholders.join(", ");
    
    // Column list for SQL (unused now but kept for reference)
    let column_names: Vec<String> = fields.iter()
        .filter_map(|f| {
            let meta = parse_field_metadata(f).ok()?;
            if meta.is_relation || meta.skip_db {
                return None;
            }
            let name = f.ident.as_ref()?.to_string();
            Some(meta.db_column.unwrap_or(name))
        })
        .collect();
    let _columns_str = column_names.join(", ");
    
    // Generate additional type names
    let edge_type = syn::Ident::new(&format!("{}Edge", struct_name), struct_name.span());
    let connection_type = syn::Ident::new(&format!("{}Connection", struct_name), struct_name.span());
    let edge_type_str = format!("{}Edge", struct_name);
    let connection_type_str = format!("{}Connection", struct_name);
    let create_input_str = format!("Create{}Input", struct_name);
    let update_input_str = format!("Update{}Input", struct_name);
    let result_type_str = format!("{}Result", struct_name);
    let changed_event_str = format!("{}ChangedEvent", struct_name);
    
    // Generate match arms for searchable fields (used in search_similar)
    let searchable_field_arms: Vec<proc_macro2::TokenStream> = string_filterable_fields
        .iter()
        .map(|(field_name, is_option)| {
            let field_str = field_name.to_string();
            if *is_option {
                quote! {
                    #field_str => entity.#field_name.as_deref(),
                }
            } else {
                quote! {
                    #field_str => Some(entity.#field_name.as_str()),
                }
            }
        })
        .collect();
    
    let searchable_field_match = if searchable_field_arms.is_empty() {
        quote! { None }
    } else {
        quote! {
            match field {
                #(#searchable_field_arms)*
                _ => None,
            }
        }
    };
    
    Ok(quote! {
        // ============================================================================
        // Connection/Edge Types (for pagination)
        // ============================================================================
        
        /// Edge containing a node and cursor
        #[derive(async_graphql::SimpleObject, Debug, Clone)]
        #[graphql(name = #edge_type_str)]
        pub struct #edge_type {
            /// The item at the end of the edge
            #[graphql(name = "Node")]
            pub node: #struct_name,
            /// A cursor for pagination
            #[graphql(name = "Cursor")]
            pub cursor: String,
        }
        
        /// Connection containing edges and page info
        #[derive(async_graphql::SimpleObject, Debug, Clone)]
        #[graphql(name = #connection_type_str)]
        pub struct #connection_type {
            /// The edges in this connection
            #[graphql(name = "Edges")]
            pub edges: Vec<#edge_type>,
            /// Pagination information
            #[graphql(name = "PageInfo")]
            pub page_info: crate::graphql::pagination::PageInfo,
        }
        
        impl #connection_type {
            /// Create from a generic Connection
            pub fn from_generic(conn: crate::graphql::pagination::Connection<#struct_name>) -> Self {
                Self {
                    edges: conn.edges.into_iter().map(|e| #edge_type {
                        node: e.node,
                        cursor: e.cursor,
                    }).collect(),
                    page_info: conn.page_info,
                }
            }
            
            /// Create an empty connection
            pub fn empty() -> Self {
                Self {
                    edges: Vec::new(),
                    page_info: crate::graphql::pagination::PageInfo::default(),
                }
            }
        }
        
        // ============================================================================
        // Create/Update Input Types
        // ============================================================================
        
        /// Input for creating a new #struct_name
        #[derive(async_graphql::InputObject, Clone, Debug)]
        #[graphql(name = #create_input_str)]
        pub struct #create_input {
            #(#create_input_fields)*
        }
        
        /// Input for updating an existing #struct_name
        #[derive(async_graphql::InputObject, Clone, Debug, Default)]
        #[graphql(name = #update_input_str)]
        pub struct #update_input {
            #(#update_input_fields)*
        }
        
        /// Result type for #struct_name mutations
        #[derive(Debug, Clone, async_graphql::SimpleObject)]
        #[graphql(name = #result_type_str)]
        pub struct #result_type {
            #[graphql(name = "Success")]
            pub success: bool,
            #[graphql(name = "Error")]
            pub error: Option<String>,
            #[graphql(name = #struct_name_str)]
            pub entity: Option<#struct_name>,
        }
        
        impl #result_type {
            /// Create a successful result with the entity
            pub fn ok(entity: #struct_name) -> Self {
                Self { success: true, error: None, entity: Some(entity) }
            }
            /// Create an error result
            pub fn err(msg: impl Into<String>) -> Self {
                Self { success: false, error: Some(msg.into()), entity: None }
            }
        }
        
        /// Event for #struct_name changes (subscriptions)
        #[derive(Debug, Clone, async_graphql::SimpleObject, serde::Serialize, serde::Deserialize)]
        #[graphql(name = #changed_event_str)]
        pub struct #changed_event {
            #[graphql(name = "Action")]
            pub action: crate::graphql::orm::ChangeAction,
            #[graphql(name = "Id")]
            pub id: #pk_type,
            #[graphql(name = #struct_name_str)]
            pub entity: Option<#struct_name>,
        }
        
        /// Result of bulk delete by Where filter
        #[derive(Debug, Clone, async_graphql::SimpleObject)]
        #[graphql(name = #delete_many_result_type_str)]
        pub struct #delete_many_result_type {
            pub success: bool,
            pub error: Option<String>,
            #[graphql(name = "DeletedCount")]
            pub deleted_count: i64,
        }
        
        impl #delete_many_result_type {
            pub fn ok(deleted_count: i64) -> Self {
                Self { success: true, error: None, deleted_count }
            }
            pub fn err(msg: impl Into<String>) -> Self {
                Self { success: false, error: Some(msg.into()), deleted_count: 0 }
            }
        }
        
        // ============================================================================
        // Query Struct
        // ============================================================================
        
        /// Generated queries for #struct_name
        #[derive(Default)]
        pub struct #queries_struct;
        
        #[async_graphql::Object]
        impl #queries_struct {
            /// Get a list of #plural_name with optional filtering, sorting, and pagination
            #[graphql(name = #list_query_name)]
            async fn list(
                &self,
                ctx: &async_graphql::Context<'_>,
                #[graphql(name = "Where")] where_input: Option<#where_input>,
                #[graphql(name = "OrderBy")] order_by: Option<Vec<#order_by_input>>,
                #[graphql(name = "Page")] page: Option<crate::graphql::orm::PageInput>,
            ) -> async_graphql::Result<#connection_type> {
                use crate::graphql::orm::{DatabaseEntity, DatabaseFilter, DatabaseOrderBy, EntityQuery, FromSqlRow};
                use crate::graphql::auth::AuthExt;
                
                let _user = ctx.auth_user()?;
                let db = ctx.data_unchecked::<crate::db::Database>();
                let pool = db;
                
                let mut query = EntityQuery::<#struct_name>::new();
                
                if let Some(ref filter) = where_input {
                    query = query.filter(filter);
                }
                
                if let Some(ref orders) = order_by {
                    for order in orders {
                        query = query.order_by(order);
                    }
                }
                
                if query.order_clauses.is_empty() {
                    query = query.default_order();
                }
                
                if let Some(ref p) = page {
                    query = query.paginate(p);
                }
                
                let generic_conn = query.fetch_connection(pool).await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                
                Ok(#connection_type::from_generic(generic_conn))
            }
            
            /// Get a single #struct_name_str by ID
            #[graphql(name = #single_query_name)]
            async fn get_by_id(
                &self,
                ctx: &async_graphql::Context<'_>,
                #[graphql(name = "Id")] id: #pk_type,
            ) -> async_graphql::Result<Option<#struct_name>> {
                use crate::graphql::orm::{DatabaseEntity, EntityQuery, FromSqlRow, SqlValue};
                use crate::graphql::auth::AuthExt;
                
                let _user = ctx.auth_user()?;
                let db = ctx.data_unchecked::<crate::db::Database>();
                let pool = db;
                
                let pk_col = #struct_name::PRIMARY_KEY;
                let entity = EntityQuery::<#struct_name>::new()
                    .where_clause(&format!("{} = ?", pk_col), SqlValue::String(id.to_string()))
                    .fetch_one(pool)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                
                Ok(entity)
            }
        }
        
        // ============================================================================
        // Mutation Struct
        // ============================================================================
        
        /// Generated mutations for #struct_name
        #[derive(Default)]
        pub struct #mutations_struct;
        
        #[async_graphql::Object]
        impl #mutations_struct {
            /// Create a new #struct_name_str
            #[graphql(name = #create_mutation_name)]
            async fn create(
                &self,
                ctx: &async_graphql::Context<'_>,
                #[graphql(name = "Input")] input: #create_input,
            ) -> async_graphql::Result<#result_type> {
                use crate::graphql::auth::AuthExt;
                use crate::graphql::orm::{DatabaseEntity, EntityQuery, FromSqlRow, SqlValue};
                
                let _user = ctx.auth_user()?;
                let db = ctx.data_unchecked::<crate::db::Database>();
                let pool = db;
                
                // Generate new UUID for primary key
                let new_id = uuid::Uuid::new_v4().to_string();
                
                // Build INSERT SQL
                let sql: String = format!(
                    "INSERT INTO {} (id, {}) VALUES (?, {})",
                    #table_name,
                    #insert_cols_str,
                    #insert_placeholders_str
                );
                
                // Collect all values first
                let mut bind_values: Vec<crate::graphql::orm::SqlValue> = Vec::new();
                bind_values.push(crate::graphql::orm::SqlValue::String(new_id.clone()));
                #(#insert_binds)*
                
                // Execute using our helper that handles lifetimes properly
                let result = crate::graphql::orm::execute_with_binds(&sql, &bind_values, pool).await;
                
                match result {
                    Ok(_) => {
                        // Fetch the created entity
                        let entity = EntityQuery::<#struct_name>::new()
                            .where_clause("id = ?", SqlValue::String(new_id))
                            .fetch_one(pool)
                            .await
                            .map_err(|e| async_graphql::Error::new(e.to_string()))?
                            .ok_or_else(|| async_graphql::Error::new("Entity not found after creation"))?;
                        
                        // Notify related tables if configured
                        #notify_on_change
                        
                        Ok(#result_type::ok(entity))
                    }
                    Err(e) => Ok(#result_type::err(e.to_string())),
                }
            }
            
            /// Update an existing #struct_name_str
            #[graphql(name = #update_mutation_name)]
            async fn update(
                &self,
                ctx: &async_graphql::Context<'_>,
                #[graphql(name = "Id")] id: #pk_type,
                #[graphql(name = "Input")] input: #update_input,
            ) -> async_graphql::Result<#result_type> {
                use crate::graphql::auth::AuthExt;
                use crate::graphql::orm::{DatabaseEntity, EntityQuery, FromSqlRow, SqlValue};
                
                let _user = ctx.auth_user()?;
                let db = ctx.data_unchecked::<crate::db::Database>();
                let pool = db;
                
                // Build dynamic UPDATE SQL based on provided fields
                let mut set_clauses: Vec<String> = Vec::new();
                let mut values: Vec<crate::graphql::orm::SqlValue> = Vec::new();
                
                #(#update_field_checks)*
                
                // Always update updated_at
                set_clauses.push("updated_at = datetime('now')".to_string());
                
                if set_clauses.is_empty() {
                    return Ok(#result_type::err("No fields to update"));
                }
                
                let sql = format!(
                    "UPDATE {} SET {} WHERE {} = ?",
                    #table_name,
                    set_clauses.join(", "),
                    #struct_name::PRIMARY_KEY
                );
                
                // Add the ID to the values for the WHERE clause
                values.push(SqlValue::String(id.to_string()));
                
                let result = crate::graphql::orm::execute_with_binds(&sql, &values, pool).await;
                
                match result {
                    Ok(r) if r.rows_affected() > 0 => {
                        // Fetch the updated entity
                        let entity = EntityQuery::<#struct_name>::new()
                            .where_clause(&format!("{} = ?", #struct_name::PRIMARY_KEY), SqlValue::String(id.to_string()))
                            .fetch_one(pool)
                            .await
                            .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                        
                        match entity {
                            Some(entity) => {
                                // Notify related tables if configured
                                #notify_on_change
                                
                                Ok(#result_type::ok(entity))
                            },
                            None => Ok(#result_type::err("Entity not found after update")),
                        }
                    }
                    Ok(_) => Ok(#result_type::err("Entity not found")),
                    Err(e) => Ok(#result_type::err(e.to_string())),
                }
            }
            
            /// Delete a #struct_name_str
            #[graphql(name = #delete_mutation_name)]
            async fn delete(
                &self,
                ctx: &async_graphql::Context<'_>,
                #[graphql(name = "Id")] id: #pk_type,
            ) -> async_graphql::Result<#result_type> {
                use crate::graphql::auth::AuthExt;
                use crate::graphql::orm::{DatabaseEntity, EntityQuery, SqlValue};
                
                let _user = ctx.auth_user()?;
                let db = ctx.data_unchecked::<crate::db::Database>();
                let pool = db;
                
                // Fetch entity before deletion for notification purposes
                let entity = EntityQuery::<#struct_name>::new()
                    .where_clause(&format!("{} = ?", #struct_name::PRIMARY_KEY), SqlValue::String(id.to_string()))
                    .fetch_one(pool)
                    .await
                    .map_err(|e| async_graphql::Error::new(e.to_string()))?;
                
                if entity.is_none() {
                    return Ok(#result_type::err("Entity not found"));
                }
                let entity = entity.unwrap();
                
                let sql = format!("DELETE FROM {} WHERE {} = ?", #table_name, #struct_name::PRIMARY_KEY);
                
                let result = sqlx::query(&sql)
                    .bind(&id.to_string())
                    .execute(pool)
                    .await;
                
                match result {
                    Ok(r) if r.rows_affected() > 0 => {
                        // Notify related tables if configured
                        #notify_on_change
                        
                        Ok(#result_type { 
                            success: true, 
                            error: None, 
                            entity: None 
                        })
                    },
                    Ok(_) => Ok(#result_type::err("Entity not found")),
                    Err(e) => Ok(#result_type::err(e.to_string())),
                }
            }
            
            /// Delete multiple #plural_name matching the given Where filter
            #[graphql(name = #delete_many_mutation_name)]
            async fn delete_many(
                &self,
                ctx: &async_graphql::Context<'_>,
                #[graphql(name = "Where")] where_input: Option<#where_input>,
            ) -> async_graphql::Result<#delete_many_result_type> {
                use crate::graphql::auth::AuthExt;
                use crate::graphql::orm::{DatabaseEntity, DatabaseFilter, EntityQuery, FromSqlRow};
                
                let _user = ctx.auth_user()?;
                let db = ctx.data_unchecked::<crate::db::Database>();
                let pool = db;
                
                let filter = match where_input {
                    Some(ref f) if !f.is_empty() => f,
                    _ => return Ok(#delete_many_result_type::err("Where filter is required for bulk delete and must not be empty")),
                };
                
                let mut query = EntityQuery::<#struct_name>::new().filter(filter);
                let (sql, values) = query.build_delete_sql();
                
                let result = crate::graphql::orm::execute_with_binds(&sql, &values, pool).await;
                
                match result {
                    Ok(r) => Ok(#delete_many_result_type::ok(r.rows_affected() as i64)),
                    Err(e) => Ok(#delete_many_result_type::err(e.to_string())),
                }
            }
        }
        
        // ============================================================================
        // Subscription Struct
        // ============================================================================
        
        /// Generated subscriptions for #struct_name
        #[derive(Default)]
        pub struct #subscriptions_struct;
        
        #[async_graphql::Subscription]
        impl #subscriptions_struct {
            /// Subscribe to #struct_name_str changes
            #[graphql(name = #subscription_name)]
            async fn on_changed(
                &self,
                ctx: &async_graphql::Context<'_>,
                #[graphql(name = "Filter")] _filter: Option<crate::graphql::orm::SubscriptionFilterInput>,
            ) -> impl futures::Stream<Item = #changed_event> {
                use futures::stream::{self, StreamExt};
                
                // Try to get the broadcast channel for this entity type
                // If not available, return an empty stream (subscription not enabled)
                let maybe_events = ctx.data_opt::<tokio::sync::broadcast::Sender<#changed_event>>();
                
                match maybe_events {
                    None => {
                        // Return empty stream if no broadcast channel is configured
                        stream::empty().left_stream()
                    }
                    Some(events) => {
                        let rx = events.subscribe();
                        
                        use tokio_stream::wrappers::BroadcastStream;
                        
                        BroadcastStream::new(rx)
                            .filter_map(move |result| async move {
                                match result {
                                    Ok(event) => Some(event),
                                    Err(_) => None,
                                }
                            })
                            .right_stream()
                    }
                }
            }
        }
        
        // ============================================================================
        // Repository Trait Implementation
        // ============================================================================
        
        /// Repository implementation for #struct_name
        /// 
        /// Provides static async methods for common database operations.
        impl #struct_name {
            /// Find all entities matching the given filter
            pub fn query<'a>(pool: &'a sqlx::SqlitePool) -> crate::graphql::orm::FindQuery<'a, Self, #where_input, #order_by_input> {
                crate::graphql::orm::FindQuery::new(pool)
            }
            
            /// Find entity by ID
            pub async fn get(pool: &sqlx::SqlitePool, id: &str) -> Result<Option<Self>, sqlx::Error> {
                use crate::graphql::orm::{DatabaseEntity, EntityQuery, FromSqlRow, SqlValue};
                
                EntityQuery::<Self>::new()
                    .where_clause(&format!("{} = ?", <Self as DatabaseEntity>::PRIMARY_KEY), SqlValue::String(id.to_string()))
                    .fetch_one(pool)
                    .await
            }
            
            /// Count entities matching the given filter
            pub fn count_query<'a>(pool: &'a sqlx::SqlitePool) -> crate::graphql::orm::CountQuery<'a, #where_input> {
                use crate::graphql::orm::DatabaseEntity;
                crate::graphql::orm::CountQuery::new(pool, <Self as DatabaseEntity>::TABLE_NAME)
            }
            
            /// Search entities with fuzzy/similar text matching
            /// 
            /// # Arguments
            /// * `pool` - Database connection pool
            /// * `field` - Name of the field to search (snake_case)
            /// * `query` - The search query text
            /// * `threshold` - Minimum similarity score (0.0-1.0, recommended: 0.5-0.7)
            /// * `filter` - Optional additional filter to apply
            /// * `limit` - Maximum number of results to return
            /// 
            /// # Returns
            /// Vector of (entity, score) tuples, sorted by score descending
            /// 
            /// # Example
            /// ```rust,ignore
            /// let matches = MovieEntity::search_similar(
            ///     &pool,
            ///     "title",
            ///     "red october",
            ///     0.6,
            ///     Some(MovieEntityWhereInput {
            ///         library_id: Some(StringFilter::eq(library_id)),
            ///         ..Default::default()
            ///     }),
            ///     Some(25),
            /// ).await?;
            /// 
            /// for (movie, score) in matches {
            ///     println!("{}: {:.2}", movie.title, score);
            /// }
            /// ```
            pub async fn search_similar(
                pool: &sqlx::SqlitePool,
                field: &str,
                query: &str,
                threshold: f64,
                filter: Option<#where_input>,
                limit: Option<i64>,
            ) -> Result<Vec<(Self, f64)>, sqlx::Error> {
                use crate::graphql::orm::FuzzyMatcher;
                
                // Fetch candidates (optionally filtered)
                let mut q = Self::query(pool);
                if let Some(f) = filter {
                    q = q.filter(f);
                }
                // Fetch more than limit to account for fuzzy filtering
                if let Some(l) = limit {
                    q = q.limit(l * 5);
                }
                let candidates = q.fetch_all().await?;
                
                // Score with fuzzy matcher
                let matcher = FuzzyMatcher::new(query).with_threshold(threshold);
                let mut results = matcher.filter_and_score(candidates, |entity| {
                    Self::get_searchable_field(entity, field)
                });
                
                // Apply limit
                if let Some(l) = limit {
                    results.truncate(l as usize);
                }
                
                Ok(results.into_iter().map(|m| (m.entity, m.score)).collect())
            }
            
            /// Get a searchable field value by name (for fuzzy matching)
            #[doc(hidden)]
            fn get_searchable_field<'a>(entity: &'a Self, field: &str) -> Option<&'a str> {
                #searchable_field_match
            }
        }
    })
}

// ============================================================================
// schema_roots! procedural macro
// ============================================================================

/// Build QueryRoot and MutationRoot from entity names and optional custom-ops list.
///
/// Generated types per entity: `XQueries`, `XMutations`, and optionally `XCustomOperations`.
/// The single source of truth is the entity list; no need to list each *Queries/*Mutations type.
///
/// # Usage
///
/// ```ignore
/// librarian_macros::schema_roots! {
///     query_custom_ops: [Movie, Library, Show, Album, Audiobook, Torrent, IndexerConfig, RssFeed],
///     entities: [
///         Library, Movie, Show, Episode, MediaFile, Artist, Album, Track,
///         Audiobook, Chapter, Torrent, TorrentFile, RssFeed, RssFeedItem,
///         PendingFileMatch, IndexerConfig, IndexerSetting, IndexerSearchCache,
///         User, InviteToken, RefreshToken, AppSetting, AppLog,
///         VideoStream, AudioStream, Subtitle, MediaChapter,
///         PlaybackSession, PlaybackProgress, CastDevice, CastSession, CastSetting,
///         UsenetServer, UsenetDownload, ScheduleCache, ScheduleSyncState,
///         NamingPattern, SourcePriorityRule, Notification, ArtworkCache, TorznabCategory,
///     ],
/// }
/// ```
///
/// Expands to:
/// - `#[derive(MergedObject, Default)] pub struct QueryRoot(MovieCustomOperations, ..., LibraryQueries, ...);`
/// - `#[derive(MergedObject, Default)] pub struct MutationRoot(LibraryMutations, ...);`
/// - `#[derive(MergedSubscription, Default)] pub struct SubscriptionRoot(LibrarySubscriptions, ...);`
#[proc_macro]
pub fn schema_roots(input: TokenStream) -> TokenStream {
    let args = match syn::parse::<SchemaRootsArgs>(input) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error().into(),
    };

    let query_custom_ops = &args.query_custom_ops;
    let entities = &args.entities;

    // Use mixed_site so generated idents don't trigger "proc-macro map is missing error entry for crate"
    let span = proc_macro2::Span::mixed_site();
    // Types for QueryRoot: XCustomOperations for each in query_custom_ops, then extra_query_types, then XQueries for each entity
    let custom_op_types: Vec<proc_macro2::TokenStream> = query_custom_ops
        .iter()
        .map(|e| {
            let name = syn::Ident::new(&format!("{}CustomOperations", e), span);
            quote! { #name }
        })
        .chain(args.extra_query_types.iter().map(|e| quote! { #e }))
        .collect();
    let query_types: Vec<proc_macro2::TokenStream> = entities
        .iter()
        .map(|e| {
            let name = syn::Ident::new(&format!("{}Queries", e), span);
            quote! { #name }
        })
        .collect();

    // Types for MutationRoot: extra_mutation_types first, then XMutations for each entity
    let extra_mutation_type_streams: Vec<proc_macro2::TokenStream> = args
        .extra_mutation_types
        .iter()
        .map(|e| quote! { #e })
        .collect();
    let mutation_custom_ops = if extra_mutation_type_streams.is_empty() {
        None
    } else {
        Some(extra_mutation_type_streams.as_slice())
    };
    let mutation_types: Vec<proc_macro2::TokenStream> = entities
        .iter()
        .map(|e| {
            let name = syn::Ident::new(&format!("{}Mutations", e), span);
            quote! { #name }
        })
        .collect();

    // Types for SubscriptionRoot: extra_subscription_types first, then XSubscriptions for each entity
    let extra_subscription_type_streams: Vec<proc_macro2::TokenStream> = args
        .extra_subscription_types
        .iter()
        .map(|e| quote! { #e })
        .collect();
    let subscription_custom_ops = if extra_subscription_type_streams.is_empty() {
        None
    } else {
        Some(extra_subscription_type_streams.as_slice())
    };
    let subscription_types: Vec<proc_macro2::TokenStream> = entities
        .iter()
        .map(|e| {
            let name = syn::Ident::new(&format!("{}Subscriptions", e), span);
            quote! { #name }
        })
        .collect();

    // Chunk types to avoid exceeding async-graphql's MergedObject recursion limit (~15–20 deep).
    let query_custom_chunk = if custom_op_types.is_empty() {
        None
    } else {
        Some(custom_op_types.as_slice())
    };
    let query_root = emit_chunked_merged(
        "Query",
        query_custom_chunk.as_deref(),
        &query_types,
        async_graphql_merged_object_derive(),
    );
    let mutation_root = emit_chunked_merged(
        "Mutation",
        mutation_custom_ops,
        &mutation_types,
        async_graphql_merged_object_derive(),
    );
    let subscription_root = emit_chunked_merged_subscription(
        "Subscription",
        subscription_custom_ops,
        &subscription_types,
    );

    quote! {
        #query_root
        #mutation_root
        #subscription_root
    }
    .into()
}

fn async_graphql_merged_object_derive() -> proc_macro2::TokenStream {
    quote! { async_graphql::MergedObject }
}

fn emit_chunked_merged(
    name: &str,
    custom_ops: Option<&[proc_macro2::TokenStream]>,
    types: &[proc_macro2::TokenStream],
    derive_macro: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let chunk_size = 12usize;
    let mut chunk_defs = Vec::new();
    let mut root_chunk_idents = Vec::new();

    // Custom ops chunk (Query only)
    if let Some(ops) = custom_ops {
        if !ops.is_empty() {
            let chunk_name = syn::Ident::new(&format!("{}RootCustomOpsChunk", name), proc_macro2::Span::mixed_site());
            let def = quote! {
                #[derive(#derive_macro, Default)]
                pub struct #chunk_name(
                    #(#ops),*
                );
            };
            chunk_defs.push(def);
            root_chunk_idents.push(chunk_name);
        }
    }

    // Entity type chunks
    for (i, chunk_types) in types.chunks(chunk_size).enumerate() {
        let chunk_name = syn::Ident::new(&format!("{}RootChunk{}", name, i), proc_macro2::Span::mixed_site());
        let def = quote! {
            #[derive(#derive_macro, Default)]
            pub struct #chunk_name(
                #(#chunk_types),*
            );
        };
        chunk_defs.push(def);
        root_chunk_idents.push(chunk_name);
    }

    let root_name = syn::Ident::new(&format!("{}Root", name), proc_macro2::Span::mixed_site());
    let root_def = quote! {
        #[derive(#derive_macro, Default)]
        pub struct #root_name(
            #(#root_chunk_idents),*
        );
    };

    quote! {
        #(#chunk_defs)*
        #root_def
    }
}

fn emit_chunked_merged_subscription(
    name: &str,
    custom_ops: Option<&[proc_macro2::TokenStream]>,
    types: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    let derive_macro = quote! { async_graphql::MergedSubscription };
    emit_chunked_merged(name, custom_ops, types, derive_macro)
}

struct SchemaRootsArgs {
    query_custom_ops: Vec<Ident>,
    entities: Vec<Ident>,
    extra_mutation_types: Vec<Ident>,
    extra_query_types: Vec<Ident>,
    extra_subscription_types: Vec<Ident>,
}

impl Parse for SchemaRootsArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        fn parse_list(content: ParseStream) -> syn::Result<Vec<Ident>> {
            let list = Punctuated::<Ident, Token![,]>::parse_terminated(content)?;
            Ok(list.into_iter().collect())
        }

        // query_custom_ops: [ ... ],
        let label: Ident = input.parse()?;
        if label.to_string() != "query_custom_ops" {
            return Err(syn::Error::new(label.span(), "expected `query_custom_ops`"));
        }
        input.parse::<Token![:]>()?;
        let content;
        syn::bracketed!(content in input);
        let query_custom_ops = parse_list(&content)?;
        let _: Option<Token![,]> = input.parse().ok();

        // entities: [ ... ]
        let label: Ident = input.parse()?;
        if label.to_string() != "entities" {
            return Err(syn::Error::new(label.span(), "expected `entities`"));
        }
        input.parse::<Token![:]>()?;
        let content;
        syn::bracketed!(content in input);
        let entities = parse_list(&content)?;
        let _: Option<Token![,]> = input.parse().ok();

        // optional extra_mutation_types, extra_query_types, extra_subscription_types
        let mut extra_mutation_types = Vec::new();
        let mut extra_query_types = Vec::new();
        let mut extra_subscription_types = Vec::new();
        while input.peek(Ident) {
            let label: Ident = input.parse()?;
            if label.to_string() == "extra_mutation_types" {
                input.parse::<Token![:]>()?;
                let content;
                syn::bracketed!(content in input);
                extra_mutation_types = parse_list(&content)?;
                let _: Option<Token![,]> = input.parse().ok();
            } else if label.to_string() == "extra_query_types" {
                input.parse::<Token![:]>()?;
                let content;
                syn::bracketed!(content in input);
                extra_query_types = parse_list(&content)?;
                let _: Option<Token![,]> = input.parse().ok();
            } else if label.to_string() == "extra_subscription_types" {
                input.parse::<Token![:]>()?;
                let content;
                syn::bracketed!(content in input);
                extra_subscription_types = parse_list(&content)?;
                let _: Option<Token![,]> = input.parse().ok();
            } else {
                return Err(syn::Error::new(label.span(), "expected `extra_mutation_types`, `extra_query_types`, or `extra_subscription_types`"));
            }
        }

        Ok(SchemaRootsArgs {
            query_custom_ops,
            entities,
            extra_mutation_types,
            extra_query_types,
            extra_subscription_types,
        })
    }
}
