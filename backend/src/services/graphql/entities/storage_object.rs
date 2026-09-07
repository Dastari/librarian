use graphql_orm::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

#[derive(GraphQLEntity, GraphQLOperations, Clone, Debug, Serialize, Deserialize)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "storage_objects",
    plural = "StorageObjects",
    default_sort = "created_at"
)]
pub struct StorageObject {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "objectId")]
    #[filterable(type = "string")]
    pub object_id: String,

    #[graphql(name = "namespace")]
    #[filterable(type = "string")]
    pub namespace: String,

    #[graphql(name = "backend")]
    #[filterable(type = "string")]
    pub backend: String,

    #[graphql(name = "storageKey")]
    #[filterable(type = "string")]
    #[sortable]
    pub storage_key: String,

    #[graphql(name = "originalFileName")]
    pub original_file_name: Option<String>,

    #[graphql(name = "mimeType")]
    pub mime_type: Option<String>,

    #[graphql(name = "sizeBytes")]
    #[filterable(type = "number")]
    pub size_bytes: i64,

    #[graphql(name = "sha256Hex")]
    #[filterable(type = "string")]
    pub sha256_hex: String,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}
