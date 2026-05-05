//! AppSetting Entity

use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

/// AppSetting Entity - application settings
#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "app_settings", plural = "AppSettings", default_sort = "key")]
pub struct AppSetting {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "Key")]
    #[filterable(type = "string")]
    #[sortable]
    #[unique]
    pub key: String,

    #[graphql(name = "Value")]
    pub value: String,

    #[graphql(name = "Description")]
    pub description: Option<String>,

    #[graphql(name = "Category")]
    #[filterable(type = "string")]
    #[sortable]
    pub category: String,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

#[derive(Default)]
pub struct AppSettingCustomOperations;
