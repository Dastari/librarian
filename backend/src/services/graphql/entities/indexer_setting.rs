//! IndexerSetting Entity

use async_graphql::SimpleObject;
use librarian_macros::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

/// IndexerSetting Entity - per-indexer settings
#[derive(
    GraphQLEntity,
    GraphQLRelations,
    GraphQLOperations,
    SimpleObject,
    Clone,
    Debug,
    Serialize,
    Deserialize,
)]
#[graphql(name = "IndexerSetting")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "indexer_settings",
    plural = "IndexerSettings",
    default_sort = "setting_key"
)]
pub struct IndexerSetting {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "IndexerConfigId")]
    #[filterable(type = "string")]
    pub indexer_config_id: String,

    #[graphql(name = "SettingKey")]
    #[filterable(type = "string")]
    #[sortable]
    pub setting_key: String,

    #[graphql(name = "SettingValue")]
    pub setting_value: String,

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
pub struct IndexerSettingCustomOperations;
