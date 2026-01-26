use super::media_file::MediaFile;
use async_graphql::SimpleObject;
use librarian_macros::{GraphQLEntity, GraphQLOperations};
use serde::{Deserialize, Serialize};

#[derive(GraphQLEntity, GraphQLOperations, SimpleObject, Clone, Debug, Serialize, Deserialize)]
#[graphql(name = "Episode")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(table = "episodes", plural = "Episodes", default_sort = "season")]
pub struct Episode {
    #[graphql(name = "Id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "ShowId")]
    #[filterable(type = "string")]
    pub show_id: String,

    #[graphql(name = "Season")]
    #[filterable(type = "number")]
    #[sortable]
    pub season: i32,

    #[graphql(name = "Episode")]
    #[filterable(type = "number")]
    #[sortable]
    pub episode: i32,

    #[graphql(name = "AbsoluteNumber")]
    #[filterable(type = "number")]
    pub absolute_number: Option<i32>,

    #[graphql(name = "Title")]
    #[filterable(type = "string")]
    #[sortable]
    pub title: Option<String>,

    #[graphql(name = "Overview")]
    pub overview: Option<String>,

    #[graphql(name = "AirDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub air_date: Option<String>,

    #[graphql(name = "Runtime")]
    #[filterable(type = "number")]
    pub runtime: Option<i32>,

    #[graphql(name = "TvmazeId")]
    #[filterable(type = "number")]
    pub tvmaze_id: Option<i32>,

    #[graphql(name = "TmdbId")]
    #[filterable(type = "number")]
    pub tmdb_id: Option<i32>,

    #[graphql(name = "TvdbId")]
    #[filterable(type = "number")]
    pub tvdb_id: Option<i32>,

    #[graphql(name = "MediaFileId")]
    #[filterable(type = "string")]
    pub media_file_id: Option<String>,

    #[graphql(name = "CreatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "UpdatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,

    #[graphql(name = "MediaFile")]
    #[relation(target = "MediaFile", from = "media_file_id", to = "id")]
    pub media_file: Option<MediaFile>,
}

#[derive(Default)]
pub struct EpisodeCustomOperations;

// // ============================================================================
// // ComplexObject Resolvers (relations + computed fields)
// // ============================================================================

//     /// Download progress (0.0 - 1.0) for episodes that are currently downloading
//     ///
//     /// Returns None if the episode already has a file or is not being downloaded.
//     #[graphql(name = "DownloadProgress")]
//     async fn download_progress(&self, ctx: &async_graphql::Context<'_>) -> Option<f32> {
//         if self.media_file_id.is_some() {
//             return None;
//         }

//         let db = ctx.data_unchecked::<Database>();

//         // Find pending file matches for this episode
//         let matches = EntityQuery::<PendingFileMatchEntity>::new()
//             .filter(&PendingFileMatchEntityWhereInput {
//                 episode_id: Some(StringFilter::eq(&self.id)),
//                 ..Default::default()
//             })
//             .fetch_all(db.pool())
//             .await
//             .ok()?;

//         if matches.is_empty() {
//             return None;
//         }

//         // Get the torrent progress for these matches
//         let mut total_progress = 0.0f32;
//         let mut count = 0;

//         for m in &matches {
//             if m.source_type == "torrent" {
//                 if let Some(ref source_id) = m.source_id {
//                     // Get torrent files for this source
//                     let files = EntityQuery::<TorrentFileEntity>::new()
//                         .filter(&TorrentFileEntityWhereInput {
//                             torrent_id: Some(StringFilter::eq(source_id)),
//                             ..Default::default()
//                         })
//                         .fetch_all(db.pool())
//                         .await
//                         .ok()
//                         .unwrap_or_default();

//                     if let Some(file_index) = m.source_file_index {
//                         if let Some(file) = files.iter().find(|f| f.file_index == file_index) {
//                             total_progress += file.progress as f32;
//                             count += 1;
//                         }
//                     }
//                 }
//             }
//         }

//         if count > 0 {
//             Some(total_progress / count as f32)
//         } else {
//             None
//         }
//     }
// }
