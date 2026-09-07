use crate::graphql::entities::*;
use async_graphql::SimpleObject;
use graphql_orm::{GraphQLEntity, GraphQLOperations, GraphQLRelations};
use serde::{Deserialize, Serialize};

#[derive(
    GraphQLEntity, GraphQLRelations, GraphQLOperations, Clone, Debug, Serialize, Deserialize,
)]
#[graphql(rename_fields = "camelCase")]
#[serde(rename_all = "PascalCase")]
#[graphql_entity(
    table = "schedule_cache",
    plural = "ScheduleCaches",
    default_sort = "air_date",
    read_policy = "member.read",
    write_policy = "admin.write"
)]
pub struct ScheduleCache {
    #[graphql(name = "id")]
    #[primary_key]
    #[filterable(type = "string")]
    pub id: String,

    #[graphql(name = "tvmazeEpisodeId")]
    #[filterable(type = "number")]
    pub tvmaze_episode_id: i32,

    #[graphql(name = "episodeName")]
    #[filterable(type = "string")]
    #[sortable]
    pub episode_name: String,

    #[graphql(name = "season")]
    #[filterable(type = "number")]
    #[sortable]
    pub season: i32,

    #[graphql(name = "episodeNumber")]
    #[filterable(type = "number")]
    #[sortable]
    pub episode_number: i32,

    #[graphql(name = "episodeType")]
    #[filterable(type = "string")]
    pub episode_type: Option<String>,

    #[graphql(name = "airDate")]
    #[filterable(type = "date")]
    #[sortable]
    pub air_date: String,

    #[graphql(name = "airTime")]
    pub air_time: Option<String>,

    #[graphql(name = "airStamp")]
    #[filterable(type = "date")]
    #[sortable]
    pub air_stamp: Option<String>,

    #[graphql(name = "runtime")]
    #[filterable(type = "number")]
    pub runtime: Option<i32>,

    #[graphql(name = "episodeImageUrl")]
    pub episode_image_url: Option<String>,

    #[graphql(name = "summary")]
    pub summary: Option<String>,

    #[graphql(name = "tvmazeShowId")]
    #[filterable(type = "number")]
    pub tvmaze_show_id: i32,

    #[graphql(name = "showName")]
    #[filterable(type = "string")]
    #[sortable]
    pub show_name: String,

    #[graphql(name = "showNetwork")]
    #[filterable(type = "string")]
    pub show_network: Option<String>,

    #[graphql(name = "showPosterUrl")]
    pub show_poster_url: Option<String>,

    #[graphql(name = "showGenres")]
    #[json_field]
    pub show_genres: Vec<String>,

    #[graphql(name = "countryCode")]
    #[filterable(type = "string")]
    pub country_code: String,

    #[graphql(name = "createdAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub created_at: String,

    #[graphql(name = "updatedAt")]
    #[filterable(type = "date")]
    #[sortable]
    pub updated_at: String,
}

#[derive(Default)]
pub struct ScheduleCacheCustomOperations;

/// Public catalog posters only. No personal library or user data is exposed.
#[derive(async_graphql::SimpleObject)]
pub struct ShowcaseArtwork {
    pub title: String,
    pub poster_url: String,
}

#[async_graphql::Object]
impl ScheduleCacheCustomOperations {
    /// Small public poster selection from the TV guide for the sign-in scene.
    #[graphql(name = "showcaseArtwork")]
    async fn showcase_artwork(
        &self,
        ctx: &async_graphql::Context<'_>,
        #[graphql(default = 12)] limit: i32,
    ) -> async_graphql::Result<Vec<ShowcaseArtwork>> {
        let db = ctx.data::<crate::db::Database>()?;
        let rows = ScheduleCache::query(db.pool())
            .limit(200)
            .fetch_all()
            .await?;
        let mut seen = std::collections::HashSet::new();
        Ok(rows
            .into_iter()
            .filter_map(|row| {
                let url = row.show_poster_url?;
                let parsed = reqwest::Url::parse(&url).ok()?;
                if parsed.scheme() != "https"
                    || parsed.host_str() != Some("static.tvmaze.com")
                    || !parsed.path().contains("/medium_")
                    || !seen.insert(url.clone())
                {
                    return None;
                }
                Some(ShowcaseArtwork {
                    title: row.show_name,
                    poster_url: url,
                })
            })
            .take(limit.clamp(1, 24) as usize)
            .collect())
    }
}
