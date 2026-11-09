//! Saving what `credits` parsed: people, credits and title details, through the generated
//! entity mutations (`docs/design.md` Q129–Q131).
//!
//! Reads go through the typed entity queries; every write is a generated `create`/`update`/
//! `delete` mutation, batched as aliased fields of one document so a show's few hundred credits
//! are a handful of requests rather than hundreds.

use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow};
use async_graphql::{Request, Variables};
use graphql_orm::graphql::filters::{IntFilter, StringFilter};
use serde_json::{Value, json};
use tracing::{debug, warn};

use crate::db::Database;
use crate::graphql::{AuthUser, LibrarianSchema};
use crate::services::graphql::entities::{
    Credit, CreditWhereInput, Person, PersonWhereInput, TitleDetail, TitleDetailWhereInput,
};

use super::credits::{
    CreditEntry, PersonDetails, PersonRef, TitleFacts, name_key, person_stand_in_key,
    stand_in_tmdb_id,
};

/// How many mutation fields go in one request.
const BATCH: usize = 40;

/// Lock names (`lockedFields`, by GraphQL name) that keep a refresh off credits or details.
pub const CREDIT_LOCKS: &[&str] = &["credits", "castNames"];

/// The title a credit or detail row belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetKind {
    Movie,
    Show,
    Season,
    Episode,
    Album,
    Track,
    Audiobook,
}

impl TargetKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Show => "show",
            Self::Season => "season",
            Self::Episode => "episode",
            Self::Album => "album",
            Self::Track => "track",
            Self::Audiobook => "audiobook",
        }
    }

    /// The GraphQL name of the target's id field on `Credit` and `TitleDetail`.
    pub fn id_field(self) -> &'static str {
        match self {
            Self::Movie => "movieId",
            Self::Show => "showId",
            Self::Season => "seasonId",
            Self::Episode => "episodeId",
            Self::Album => "albumId",
            Self::Track => "trackId",
            Self::Audiobook => "audiobookId",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TitleTarget {
    pub kind: TargetKind,
    pub id: String,
    /// The title's owner, copied onto every row so the row policy can scope it directly.
    pub user_id: String,
}

impl TitleTarget {
    pub fn new(kind: TargetKind, id: impl Into<String>, user_id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
            user_id: user_id.into(),
        }
    }

    fn eq(&self) -> Option<StringFilter> {
        Some(StringFilter {
            eq: Some(self.id.clone()),
            ..Default::default()
        })
    }

    fn credit_filter(&self, source: &str) -> CreditWhereInput {
        let mut filter = CreditWhereInput {
            source: Some(string_eq(source)),
            ..Default::default()
        };
        match self.kind {
            TargetKind::Movie => filter.movie_id = self.eq(),
            TargetKind::Show => filter.show_id = self.eq(),
            TargetKind::Season => filter.season_id = self.eq(),
            TargetKind::Episode => filter.episode_id = self.eq(),
            TargetKind::Album => filter.album_id = self.eq(),
            TargetKind::Track => filter.track_id = self.eq(),
            TargetKind::Audiobook => filter.audiobook_id = self.eq(),
        }
        filter
    }

    fn detail_filter(&self) -> TitleDetailWhereInput {
        let mut filter = TitleDetailWhereInput::default();
        match self.kind {
            TargetKind::Movie => filter.movie_id = self.eq(),
            TargetKind::Show => filter.show_id = self.eq(),
            TargetKind::Season => filter.season_id = self.eq(),
            TargetKind::Episode => filter.episode_id = self.eq(),
            TargetKind::Album => filter.album_id = self.eq(),
            TargetKind::Track => filter.track_id = self.eq(),
            TargetKind::Audiobook => filter.audiobook_id = self.eq(),
        }
        filter
    }
}

fn string_eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_string()),
        ..Default::default()
    }
}

/// What one replacement did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReplaceSummary {
    pub created: usize,
    pub updated: usize,
    pub deleted: usize,
    pub unchanged: usize,
}

/// One mutation field of a batch.
enum Op {
    Create(&'static str, Value),
    Update(&'static str, String, Value),
    Delete(&'static str, String),
}

pub struct CreditStore {
    schema: LibrarianSchema,
    db: Database,
    /// Person lookups already made in this run, by every key the person was found under.
    people: HashMap<String, String>,
}

impl CreditStore {
    pub fn new(schema: LibrarianSchema, db: Database) -> Self {
        Self {
            schema,
            db,
            people: HashMap::new(),
        }
    }

    async fn run(&self, document: &str, variables: Value) -> Result<Value> {
        let user = AuthUser::system_admin();
        let request = Request::new(document)
            .variables(Variables::from_json(variables))
            .data(user.clone())
            .data(graphql_orm::graphql::auth::AuthSubject::from_parts(
                user.user_id.clone(),
                user.role.iter().cloned().collect(),
                Vec::new(),
                None,
            ))
            .data(user.user_id.clone());
        let response = self.schema.execute(request).await;
        if !response.errors.is_empty() {
            return Err(anyhow!(
                response
                    .errors
                    .iter()
                    .map(|e| e.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            ));
        }
        Ok(serde_json::to_value(&response.data)?)
    }

    /// Run mutations as aliased fields of as few documents as possible. Each field is its own
    /// generated mutation with its own hooks; a failed one is reported and the rest still run.
    async fn apply(&self, ops: Vec<Op>, what: &str) -> Result<usize> {
        let mut failed = 0usize;
        let mut first_error = None;
        for chunk in ops.chunks(BATCH) {
            let mut declarations = Vec::new();
            let mut fields = Vec::new();
            let mut variables = serde_json::Map::new();
            for (index, op) in chunk.iter().enumerate() {
                match op {
                    Op::Create(entity, input) => {
                        declarations.push(format!("$i{index}: Create{entity}Input!"));
                        fields.push(format!(
                            "m{index}: create{entity}(input: $i{index}) {{ success error }}"
                        ));
                        variables.insert(format!("i{index}"), input.clone());
                    }
                    Op::Update(entity, id, input) => {
                        declarations.push(format!(
                            "$k{index}: String!, $i{index}: Update{entity}Input!"
                        ));
                        fields.push(format!(
                            "m{index}: update{entity}(id: $k{index}, input: $i{index}) {{ success error }}"
                        ));
                        variables.insert(format!("k{index}"), json!(id));
                        variables.insert(format!("i{index}"), input.clone());
                    }
                    Op::Delete(entity, id) => {
                        declarations.push(format!("$k{index}: String!"));
                        fields.push(format!(
                            "m{index}: delete{entity}(id: $k{index}) {{ success error }}"
                        ));
                        variables.insert(format!("k{index}"), json!(id));
                    }
                }
            }
            let document = format!(
                "mutation CreditStoreBatch({}) {{ {} }}",
                declarations.join(", "),
                fields.join(" ")
            );
            let data = self.run(&document, Value::Object(variables)).await?;
            for index in 0..chunk.len() {
                let payload = &data[format!("m{index}")];
                if payload["success"].as_bool() != Some(true) {
                    failed += 1;
                    if first_error.is_none() {
                        first_error = payload["error"].as_str().map(str::to_string);
                    }
                }
            }
        }
        if failed > 0 {
            warn!(
                failed,
                what,
                error = first_error.as_deref().unwrap_or("unknown"),
                "Some {what} writes failed: {}",
                first_error.as_deref().unwrap_or("unknown")
            );
        }
        Ok(failed)
    }

    // -----------------------------------------------------------------------------------------
    // People

    async fn people_where(&self, filter: PersonWhereInput) -> Result<Vec<Person>> {
        Ok(Person::query(self.db.pool())
            .filter(filter)
            .limit(20)
            .fetch_all()
            .await?)
    }

    /// The existing person this reference names, by the strongest id they share.
    pub async fn find_person(&self, person: &PersonRef) -> Result<Option<Person>> {
        if let Some(id) = person.tmdb_id.filter(|id| *id > 0)
            && let Some(found) = self
                .people_where(PersonWhereInput {
                    tmdb_person_id: Some(IntFilter {
                        eq: Some(id),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
                .await?
                .into_iter()
                .next()
        {
            return Ok(Some(found));
        }
        let by_string = [
            (person.imdb_id.as_deref(), 0u8),
            (person.musicbrainz_id.as_deref(), 1),
            (person.wikidata_id.as_deref(), 2),
        ];
        for (value, which) in by_string {
            let Some(value) = value.filter(|v| !v.is_empty()) else {
                continue;
            };
            let mut filter = PersonWhereInput::default();
            match which {
                0 => filter.imdb_id = Some(string_eq(value)),
                1 => filter.musicbrainz_artist_id = Some(string_eq(value)),
                _ => filter.wikidata_id = Some(string_eq(value)),
            }
            if let Some(found) = self.people_where(filter).await?.into_iter().next() {
                return Ok(Some(found));
            }
        }
        if person.has_external_id() {
            return Ok(None);
        }
        // Known by name only: the same as another person known by that name only.
        let key = name_key(&person.name);
        if key.is_empty() {
            return Ok(None);
        }
        Ok(self
            .people_where(PersonWhereInput {
                name_key: Some(string_eq(&key)),
                ..Default::default()
            })
            .await?
            .into_iter()
            .find(|row| {
                row.tmdb_person_id <= 0
                    && row.imdb_id.is_none()
                    && row.musicbrainz_artist_id.is_none()
                    && row.wikidata_id.is_none()
            }))
    }

    fn cache_keys(person: &PersonRef) -> Vec<String> {
        let mut keys = Vec::new();
        if let Some(id) = person.tmdb_id.filter(|id| *id > 0) {
            keys.push(format!("tmdb:{id}"));
        }
        if let Some(id) = &person.imdb_id {
            keys.push(format!("imdb:{id}"));
        }
        if let Some(id) = &person.musicbrainz_id {
            keys.push(format!("mb:{id}"));
        }
        if let Some(id) = &person.wikidata_id {
            keys.push(format!("wd:{id}"));
        }
        if keys.is_empty() {
            keys.push(format!("name:{}", name_key(&person.name)));
        }
        keys
    }

    /// The person row for this reference: found by id and given any ids it lacked, or created.
    pub async fn resolve_person(&mut self, person: &PersonRef) -> Result<String> {
        let keys = Self::cache_keys(person);
        if let Some(id) = keys.iter().find_map(|key| self.people.get(key)) {
            return Ok(id.clone());
        }
        let id = match self.find_person(person).await? {
            Some(existing) => {
                self.merge_person(&existing, person).await?;
                existing.id
            }
            None => self.create_person(person).await?,
        };
        for key in keys {
            self.people.insert(key, id.clone());
        }
        Ok(id)
    }

    async fn merge_person(&self, existing: &Person, person: &PersonRef) -> Result<()> {
        let mut input = serde_json::Map::new();
        if let Some(id) = person.tmdb_id.filter(|id| *id > 0)
            && existing.tmdb_person_id != id
            && existing.tmdb_person_id <= 0
        {
            input.insert("tmdbPersonId".into(), json!(id));
        }
        let mut fill = |field: &str, current: &Option<String>, new: &Option<String>| {
            if current.is_none()
                && let Some(value) = new
            {
                input.insert(field.into(), json!(value));
            }
        };
        fill("imdbId", &existing.imdb_id, &person.imdb_id);
        fill(
            "musicbrainzArtistId",
            &existing.musicbrainz_artist_id,
            &person.musicbrainz_id,
        );
        fill("wikidataId", &existing.wikidata_id, &person.wikidata_id);
        fill(
            "knownForDepartment",
            &existing.known_for_department,
            &person.known_for_department,
        );
        // TMDB's name and picture are the ones shown; another source only fills a gap.
        let from_tmdb = person.tmdb_id.is_some_and(|id| id > 0);
        if person.profile_url.is_some()
            && (existing.profile_url.is_none() || from_tmdb)
            && existing.profile_url != person.profile_url
        {
            input.insert("profileUrl".into(), json!(person.profile_url));
        }
        if from_tmdb && !person.name.is_empty() && existing.name != person.name {
            input.insert("name".into(), json!(person.name));
            input.insert("nameKey".into(), json!(name_key(&person.name)));
        }
        if existing.name_key.is_none() {
            input.insert("nameKey".into(), json!(name_key(&existing.name)));
        }
        if input.is_empty() {
            return Ok(());
        }
        self.apply(
            vec![Op::Update(
                "Person",
                existing.id.clone(),
                Value::Object(input),
            )],
            "person",
        )
        .await?;
        Ok(())
    }

    async fn create_person(&self, person: &PersonRef) -> Result<String> {
        let stand_in_key = person_stand_in_key(person);
        for attempt in 0..5u32 {
            let tmdb_id = match person.tmdb_id.filter(|id| *id > 0) {
                Some(id) => id,
                None => stand_in_tmdb_id(&stand_in_key, attempt),
            };
            let data = self
                .run(
                    r#"mutation CreateCreditedPerson($input: CreatePersonInput!) {
                        createPerson(input: $input) { success error person { id } }
                    }"#,
                    json!({ "input": {
                        "tmdbPersonId": tmdb_id,
                        "name": person.name,
                        "nameKey": name_key(&person.name),
                        "profileUrl": person.profile_url,
                        "imdbId": person.imdb_id,
                        "musicbrainzArtistId": person.musicbrainz_id,
                        "wikidataId": person.wikidata_id,
                        "knownForDepartment": person.known_for_department,
                    }}),
                )
                .await?;
            let payload = &data["createPerson"];
            if let Some(id) = payload["person"]["id"].as_str() {
                return Ok(id.to_string());
            }
            let error = payload["error"].as_str().unwrap_or("createPerson failed");
            // Two titles crediting the same new person at once, or a stand-in collision.
            if let Some(found) = self.find_person(person).await? {
                return Ok(found.id);
            }
            if person.tmdb_id.is_some_and(|id| id > 0) || !error.to_lowercase().contains("unique") {
                return Err(anyhow!("Could not save person '{}': {error}", person.name));
            }
        }
        Err(anyhow!(
            "Could not find a free stand-in id for person '{}'",
            person.name
        ))
    }

    /// A person's own record (biography, dates, pictures, ids), as read from TMDB.
    pub async fn save_person_details(
        &self,
        person: &Person,
        details: &PersonDetails,
        fetched_at: &str,
    ) -> Result<()> {
        let mut input = json!({
            "name": details.person.name,
            "nameKey": name_key(&details.person.name),
            "biography": details.biography,
            "birthday": details.birthday,
            "deathday": details.deathday,
            "placeOfBirth": details.place_of_birth,
            "knownForDepartment": details.person.known_for_department,
            "alsoKnownAs": details.also_known_as,
            "imageUrls": details.image_urls,
            "detailsFetchedAt": fetched_at,
        });
        let object = input.as_object_mut().expect("object");
        if details.person.profile_url.is_some() {
            object.insert("profileUrl".into(), json!(details.person.profile_url));
        }
        // Ids are only ever added: another person may already hold one (then it stays there).
        for (field, current, new) in [
            ("imdbId", &person.imdb_id, &details.person.imdb_id),
            (
                "wikidataId",
                &person.wikidata_id,
                &details.person.wikidata_id,
            ),
        ] {
            if current.is_none()
                && let Some(value) = new
                && self
                    .find_person(&PersonRef {
                        name: String::new(),
                        imdb_id: (field == "imdbId").then(|| value.clone()),
                        wikidata_id: (field == "wikidataId").then(|| value.clone()),
                        ..Default::default()
                    })
                    .await?
                    .is_none()
            {
                object.insert(field.into(), json!(value));
            }
        }
        self.apply(
            vec![Op::Update("Person", person.id.clone(), input)],
            "person details",
        )
        .await?;
        Ok(())
    }

    /// Mark a person read even when the provider had nothing, so the fill moves on.
    pub async fn mark_person_fetched(&self, person_id: &str, fetched_at: &str) -> Result<()> {
        self.apply(
            vec![Op::Update(
                "Person",
                person_id.to_string(),
                json!({ "detailsFetchedAt": fetched_at }),
            )],
            "person details",
        )
        .await?;
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Credits

    /// Make the title's credits from `source` exactly `credits`: matching credits are updated
    /// in place, new ones created and the rest deleted. Running it twice changes nothing.
    pub async fn replace_credits(
        &mut self,
        target: &TitleTarget,
        source: &str,
        credits: &[CreditEntry],
    ) -> Result<ReplaceSummary> {
        let existing = Credit::query(self.db.pool())
            .filter(target.credit_filter(source))
            .fetch_all()
            .await?;
        let mut pool: HashMap<(String, String, bool, String, String), Vec<Credit>> = HashMap::new();
        for credit in existing {
            let key = (
                credit.person_id.clone(),
                credit.kind.clone(),
                credit.guest,
                credit.job.clone().unwrap_or_default().to_lowercase(),
                credit
                    .character_name
                    .clone()
                    .unwrap_or_default()
                    .to_lowercase(),
            );
            pool.entry(key).or_default().push(credit);
        }
        let mut summary = ReplaceSummary::default();
        let mut ops = Vec::new();
        let mut seen = HashSet::new();
        for entry in credits {
            let person_id = match self.resolve_person(&entry.person).await {
                Ok(id) => id,
                Err(error) => {
                    warn!(
                        target = target.kind.as_str(),
                        target_id = %target.id,
                        person = %entry.person.name,
                        error = %error,
                        "Skipped a credit of '{}' on {} {}: {error}",
                        entry.person.name,
                        target.kind.as_str(),
                        target.id
                    );
                    continue;
                }
            };
            let (kind, guest, job, character) = entry.identity();
            let key = (
                person_id.clone(),
                kind.as_str().to_string(),
                guest,
                job,
                character,
            );
            if !seen.insert(key.clone()) {
                continue;
            }
            let fields = json!({
                "department": entry.department,
                "job": entry.job,
                "characterName": entry.character,
                "creditOrder": entry.order,
                "episodeCount": entry.episode_count,
                "providerCreditId": entry.provider_credit_id,
            });
            match pool.get_mut(&key).and_then(|rows| rows.pop()) {
                Some(row) => {
                    let same = row.department == entry.department
                        && row.job == entry.job
                        && row.character_name == entry.character
                        && row.credit_order == entry.order
                        && row.episode_count == entry.episode_count
                        && row.provider_credit_id == entry.provider_credit_id
                        && row.user_id == target.user_id;
                    if same {
                        summary.unchanged += 1;
                    } else {
                        let mut input = fields;
                        input["userId"] = json!(target.user_id);
                        ops.push(Op::Update("Credit", row.id, input));
                        summary.updated += 1;
                    }
                }
                None => {
                    let mut input = fields;
                    let object = input.as_object_mut().expect("object");
                    object.insert("userId".into(), json!(target.user_id));
                    object.insert("personId".into(), json!(person_id));
                    object.insert("targetKind".into(), json!(target.kind.as_str()));
                    object.insert(target.kind.id_field().into(), json!(target.id));
                    object.insert("kind".into(), json!(kind.as_str()));
                    object.insert("guest".into(), json!(guest));
                    object.insert("source".into(), json!(source));
                    ops.push(Op::Create("Credit", input));
                    summary.created += 1;
                }
            }
        }
        for row in pool.into_values().flatten() {
            ops.push(Op::Delete("Credit", row.id));
            summary.deleted += 1;
        }
        self.apply(ops, "credit").await?;
        debug!(
            target = target.kind.as_str(),
            target_id = %target.id,
            source,
            created = summary.created,
            updated = summary.updated,
            deleted = summary.deleted,
            unchanged = summary.unchanged,
            "Replaced {} credits of {} {}",
            source,
            target.kind.as_str(),
            target.id
        );
        Ok(summary)
    }

    // -----------------------------------------------------------------------------------------
    // Title details

    pub async fn title_detail(&self, target: &TitleTarget) -> Result<Option<TitleDetail>> {
        Ok(TitleDetail::query(self.db.pool())
            .filter(target.detail_filter())
            .limit(1)
            .fetch_all()
            .await?
            .into_iter()
            .next())
    }

    /// Save the title's details, one row per title. A field whose lock name is in `locked`
    /// (`studios`, `networks`, `keywords`, `contentRatings`, `alternativeTitles`, `videos`) keeps
    /// what it had.
    pub async fn save_title_detail(
        &self,
        target: &TitleTarget,
        source: &str,
        facts: &TitleFacts,
        locked: &[String],
        fetched_at: &str,
    ) -> Result<()> {
        let json_text = |value: &dyn erased_json::Json| value.text();
        let mut input = json!({
            "userId": target.user_id,
            "source": source,
            "providerId": facts.provider_id,
            "studios": facts.studios,
            "networks": facts.networks,
            "keywords": facts.keywords,
            "originCountries": facts.origin_countries,
            "contentRatings": json_text(&facts.content_ratings),
            "alternativeTitles": json_text(&facts.alternative_titles),
            "videos": json_text(&facts.videos),
            "externalIds": json_text(&facts.external_ids),
            "homepage": facts.homepage,
            "fetchedAt": fetched_at,
        });
        let object = input.as_object_mut().expect("object");
        for lock in locked {
            if [
                "studios",
                "networks",
                "keywords",
                "originCountries",
                "contentRatings",
                "alternativeTitles",
                "videos",
                "externalIds",
                "homepage",
            ]
            .contains(&lock.as_str())
            {
                object.remove(lock.as_str());
            }
        }
        let op = match self.title_detail(target).await? {
            Some(existing) => Op::Update("TitleDetail", existing.id, input),
            None => {
                object.insert("targetKind".into(), json!(target.kind.as_str()));
                object.insert(target.kind.id_field().into(), json!(target.id));
                Op::Create("TitleDetail", input)
            }
        };
        if self.apply(vec![op], "title detail").await? > 0 {
            return Err(anyhow!(
                "Could not save details of {} {}",
                target.kind.as_str(),
                target.id
            ));
        }
        Ok(())
    }
}

/// JSON text for the structured detail fields; empty lists and maps are stored as null.
mod erased_json {
    pub trait Json {
        fn text(&self) -> Option<String>;
    }
    impl<T: serde::Serialize> Json for Vec<T> {
        fn text(&self) -> Option<String> {
            (!self.is_empty())
                .then(|| serde_json::to_string(self).ok())
                .flatten()
        }
    }
    impl<K: serde::Serialize + Ord, V: serde::Serialize> Json for std::collections::BTreeMap<K, V> {
        fn text(&self) -> Option<String> {
            (!self.is_empty())
                .then(|| serde_json::to_string(self).ok())
                .flatten()
        }
    }
}

#[cfg(test)]
#[path = "credit_store_tests.rs"]
mod tests;
