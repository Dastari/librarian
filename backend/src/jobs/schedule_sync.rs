//! Daily public TV guide cache. All writes use generated entity mutations.
use crate::{
    db::Database,
    graphql::{
        AuthUser, LibrarianSchema,
        entities::{
            Episode, EpisodeWhereInput, ScheduleCache, ScheduleCacheWhereInput, ScheduleSyncState,
            Show,
        },
    },
    services::{
        ServicesManager,
        manager::{Service, ServiceHealth},
        metadata::tvmaze::{TvMazeClient, TvMazeScheduleEntry},
    },
};
use anyhow::{Result, anyhow};
use async_graphql::{Request, Variables};
use async_trait::async_trait;
use chrono::{DateTime, NaiveDate, Utc};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub struct ScheduleSyncConfig {
    pub countries: Vec<String>,
}
impl ScheduleSyncConfig {
    pub fn from_env() -> Result<Self> {
        let value = std::env::var("LIBRARIAN_SCHEDULE_COUNTRIES").unwrap_or_else(|_| "US".into());
        let mut countries: Vec<String> = value
            .split(',')
            .map(|s| s.trim().to_ascii_uppercase())
            .collect();
        countries.sort();
        countries.dedup();
        if countries.is_empty()
            || countries.len() > 8
            || countries
                .iter()
                .any(|s| s.len() != 2 || !s.bytes().all(|b| b.is_ascii_uppercase()))
        {
            return Err(anyhow!(
                "LIBRARIAN_SCHEDULE_COUNTRIES requires 1–8 comma-separated two-letter country codes"
            ));
        }
        Ok(Self { countries })
    }
}

pub struct ScheduleSyncService {
    manager: Arc<ServicesManager>,
    config: ScheduleSyncConfig,
    runtime: tokio::sync::Mutex<Option<(CancellationToken, tokio::task::JoinHandle<()>)>>,
    error: Arc<tokio::sync::RwLock<Option<String>>>,
}
impl ScheduleSyncService {
    pub fn new(manager: Arc<ServicesManager>, config: ScheduleSyncConfig) -> Self {
        Self {
            manager,
            config,
            runtime: Default::default(),
            error: Default::default(),
        }
    }
}
#[async_trait]
impl Service for ScheduleSyncService {
    fn name(&self) -> &str {
        "schedule_sync"
    }
    fn dependencies(&self) -> Vec<String> {
        vec!["database".into(), "graphql".into()]
    }
    async fn start(&self) -> Result<()> {
        let mut runtime = self.runtime.lock().await;
        if runtime.is_some() {
            return Ok(());
        }
        let cancel = CancellationToken::new();
        let token = cancel.clone();
        let manager = self.manager.clone();
        let config = self.config.clone();
        let error = self.error.clone();
        let task = tokio::spawn(async move {
            let client = TvMazeClient::new();
            let mut last_backfill = None;
            let mut interval = tokio::time::interval(Duration::from_secs(3600));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! { _ = token.cancelled() => break, _ = interval.tick() => {} }
                let result = tokio::select! {
                    _ = token.cancelled() => break,
                    result = run_sync(&manager, &config, &client, last_backfill != Some(Utc::now().date_naive())) => result,
                };
                if result.is_ok() {
                    last_backfill = Some(Utc::now().date_naive());
                }
                *error.write().await = result.err().map(|e| {
                    tracing::warn!(service="schedule_sync", error=%e, "TV guide sync failed; keeping cached entries and retrying in one hour"); e.to_string()
                });
            }
        });
        *runtime = Some((cancel, task));
        tracing::info!(service="schedule_sync", countries=?self.config.countries, "Started daily seven-day TV guide sync");
        Ok(())
    }
    async fn stop(&self) -> Result<()> {
        if let Some((cancel, task)) = self.runtime.lock().await.take() {
            cancel.cancel();
            let _ = task.await;
        }
        Ok(())
    }
    async fn health(&self) -> Result<ServiceHealth> {
        if let Some(error) = self.error.read().await.as_ref() {
            return Ok(ServiceHealth::degraded(error));
        }
        let runtime = self.runtime.lock().await;
        Ok(
            if runtime
                .as_ref()
                .is_some_and(|(_, task)| !task.is_finished())
            {
                ServiceHealth::healthy()
            } else {
                ServiceHealth::degraded("schedule_sync worker is not running")
            },
        )
    }
}

fn synced_today(state: &ScheduleSyncState, today: NaiveDate) -> bool {
    state.sync_error.is_none()
        && state.last_sync_days == 7
        && parse_stamp(&state.last_synced_at).is_some_and(|d| d.date_naive() == today)
}
fn parse_stamp(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|d| d.with_timezone(&Utc))
        .or_else(|| {
            value
                .parse::<i64>()
                .ok()
                .and_then(|n| DateTime::from_timestamp(n, 0))
        })
}

fn authenticated_request(operation: &str, user: AuthUser) -> Request {
    let subject = graphql_orm::graphql::auth::AuthSubject::from_parts(
        user.user_id.clone(),
        user.role.iter().cloned().collect(),
        Vec::new(),
        None,
    );
    Request::new(operation).data(user).data(subject)
}

async fn mutate(schema: &LibrarianSchema, operation: &str, variables: Value) -> Result<Value> {
    let response = schema
        .execute(
            authenticated_request(operation, AuthUser::system_admin_for("system-internal"))
                .variables(Variables::from_json(variables)),
        )
        .await;
    if !response.errors.is_empty() {
        return Err(anyhow!(
            "Guide entity mutation failed: {:?}",
            response.errors
        ));
    }
    let data = response.data.into_json()?;
    if data["result"]["success"] != true {
        return Err(anyhow!(
            "Guide entity mutation failed: {}",
            data["result"]["error"]
        ));
    }
    Ok(data["result"].clone())
}

pub(crate) async fn run_sync(
    manager: &Arc<ServicesManager>,
    config: &ScheduleSyncConfig,
    client: &TvMazeClient,
    backfill: bool,
) -> Result<()> {
    let service = manager
        .get_database()
        .await
        .ok_or_else(|| anyhow!("Schedule database unavailable"))?;
    let db = service.pool();
    let schema = manager
        .get_graphql()
        .await
        .ok_or_else(|| anyhow!("Schedule GraphQL unavailable"))?
        .schema()
        .await
        .ok_or_else(|| anyhow!("Schedule schema unavailable"))?;
    let today = Utc::now().date_naive();
    let states = ScheduleSyncState::query(db.pool()).fetch_all().await?;
    let mut failures = Vec::new();
    for country in &config.countries {
        let state = states.iter().find(|s| &s.country_code == country);
        if state.is_some_and(|s| synced_today(s, today)) {
            continue;
        }
        let result = sync_country(db, &schema, client, country, today).await;
        let message = result.as_ref().err().map(ToString::to_string);
        let input = json!({"countryCode": country, "lastSyncedAt": if result.is_ok() { Utc::now().to_rfc3339() } else { state.map(|s| s.last_synced_at.clone()).unwrap_or_else(|| "1970-01-01T00:00:00Z".into()) }, "lastSyncDays": if result.is_ok() {7} else {state.map(|s| s.last_sync_days).unwrap_or(0)}, "syncError":message});
        if let Some(state) = state {
            mutate(&schema, "mutation($id:String!,$input:UpdateScheduleSyncStateInput!){result:updateScheduleSyncState(id:$id,input:$input){success error}}", json!({"id":state.id,"input":input})).await?;
        } else {
            mutate(&schema, "mutation($input:CreateScheduleSyncStateInput!){result:createScheduleSyncState(input:$input){success error}}", json!({"input":input})).await?;
        }
        if let Err(error) = result {
            failures.push(format!("{country}: {error}"));
        }
    }
    // Existing libraries get timestamps without re-importing episodes or changing wanted flags.
    if backfill && let Err(error) = backfill_air_stamps(db, &schema, client).await {
        failures.push(error.to_string());
    }
    if !failures.is_empty() {
        return Err(anyhow!(failures.join("; ")));
    }
    Ok(())
}

async fn sync_country(
    db: &Database,
    schema: &LibrarianSchema,
    client: &TvMazeClient,
    country: &str,
    today: NaiveDate,
) -> Result<()> {
    let mut entries = HashMap::new();
    // Fetch the complete window before altering the cache. A provider failure preserves the previous window.
    for offset in 0..7 {
        let date = today + chrono::Duration::days(offset);
        for (region, streaming) in [(country, false), ("", true)] {
            for entry in client.schedule(region, date, streaming).await? {
                if entry.show().is_some() {
                    entries.insert(entry.episode.id, entry);
                }
            }
        }
    }
    persist_entries(db, schema, country, today, &entries).await?;
    tracing::info!(service="schedule_sync", country, count=entries.len(), start_date=%today, "Synced seven-day broadcast and global streaming TV guide");
    Ok(())
}

fn schedule_input(entry: &TvMazeScheduleEntry, country: &str) -> Result<Value> {
    let show = entry
        .show()
        .ok_or_else(|| anyhow!("TVmaze episode {} has no show", entry.episode.id))?;
    let ep = &entry.episode;
    let date = ep
        .airdate
        .as_deref()
        .filter(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok())
        .ok_or_else(|| anyhow!("TVmaze schedule episode {} has no valid air date", ep.id))?;
    Ok(
        json!({"tvmazeEpisodeId":ep.id,"episodeName":ep.name,"season":ep.season,"episodeNumber":ep.number,
        "episodeType":ep.episode_type,"airDate":date,"airTime":ep.airtime,"airStamp":ep.air_stamp,
        "runtime":ep.runtime,"episodeImageUrl":ep.image.as_ref().and_then(|image| image.medium.as_ref()),
        "summary":ep.clean_summary(),"tvmazeShowId":show.id,"showName":show.name,
        "showNetwork":show.network.as_ref().map(|n| &n.name).or_else(|| show.web_channel.as_ref().map(|n| &n.name)),
        "showPosterUrl":show.image.as_ref().and_then(|image| image.medium.as_ref()),"showGenres":show.genres,"countryCode":country}),
    )
}

async fn persist_entries(
    db: &Database,
    schema: &LibrarianSchema,
    country: &str,
    today: NaiveDate,
    entries: &HashMap<u32, TvMazeScheduleEntry>,
) -> Result<()> {
    let existing: Vec<_> = ScheduleCache::query(db.pool())
        .filter(ScheduleCacheWhereInput {
            country_code: Some(graphql_orm::graphql::filters::StringFilter {
                eq: Some(country.into()),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?
        .into_iter()
        .filter(|e| e.country_code == country)
        .collect();
    for entry in entries.values() {
        let input = schedule_input(entry, country)?;
        if let Some(old) = existing
            .iter()
            .find(|e| e.tvmaze_episode_id == entry.episode.id as i32)
        {
            mutate(schema, "mutation($id:String!,$input:UpdateScheduleCacheInput!){result:updateScheduleCache(id:$id,input:$input){success error}}", json!({"id":old.id,"input":input})).await?;
        } else {
            mutate(schema, "mutation($input:CreateScheduleCacheInput!){result:createScheduleCache(input:$input){success error}}", json!({"input":input})).await?;
        }
    }
    let keep_from = today - chrono::Duration::days(14);
    let end = today + chrono::Duration::days(6);
    for old in existing {
        let date = NaiveDate::parse_from_str(&old.air_date, "%Y-%m-%d")
            .ok()
            .or_else(|| parse_stamp(&old.air_date).map(|stamp| stamp.date_naive()));
        if date.is_some_and(|d| {
            d < keep_from
                || (d >= today
                    && d <= end
                    && !entries.contains_key(&(old.tvmaze_episode_id as u32)))
        }) {
            mutate(
                schema,
                "mutation($id:String!){result:deleteScheduleCache(id:$id){success error}}",
                json!({"id":old.id}),
            )
            .await?;
        }
    }
    Ok(())
}

async fn backfill_air_stamps(
    db: &Database,
    schema: &LibrarianSchema,
    client: &TvMazeClient,
) -> Result<()> {
    let episodes = Episode::query(db.pool())
        .filter(EpisodeWhereInput {
            air_stamp: Some(graphql_orm::graphql::filters::DateFilter {
                is_null: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        })
        .fetch_all()
        .await?;
    let missing: HashSet<_> = episodes
        .iter()
        .filter(|e| e.air_stamp.is_none() && e.tvmaze_id.is_some())
        .map(|e| e.show_id.as_str())
        .collect();
    for show in Show::query(db.pool())
        .fetch_all()
        .await?
        .into_iter()
        .filter(|show| missing.contains(show.id.as_str()))
    {
        let Some(tvmaze_id) = show.tvmaze_id else {
            continue;
        };
        for ep in client.get_episodes(tvmaze_id as u32).await? {
            let Some(stamp) = ep.air_stamp else {
                continue;
            };
            for existing in episodes.iter().filter(|e| {
                e.show_id == show.id && e.tvmaze_id == Some(ep.id as i32) && e.air_stamp.is_none()
            }) {
                mutate(schema, "mutation($id:String!,$input:UpdateEpisodeInput!){result:updateEpisode(id:$id,input:$input){success error}}", json!({"id":existing.id,"input":{"airStamp":stamp}})).await?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "schedule_sync_tests.rs"]
mod tests;
