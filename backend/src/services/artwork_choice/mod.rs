//! Choosing artwork for anything (design notes Q126–Q128).
//!
//! Every picture a title shows is a *slot*: a movie's poster, backdrop and logo, a show's poster
//! and backdrop, a season's poster, an episode's still, an album's cover, an artist's picture, an
//! audiobook's cover and a playlist's cover. A slot can be filled from the provider's own list
//! (`options`), from a file someone uploads (`upload`), or put back to what a metadata refresh
//! would choose (`revert`).
//!
//! Whatever someone chooses is locked: the slot's field (`posterUrl`, `coverUrl`…) goes into the
//! entity's `lockedFields`, and from then on
//! - an update that writes the field without also saying what is locked leaves it alone
//!   ([`keep_locked_artwork`], run by the write transform for every generated or repository
//!   update), so no refresh path can put the provider's picture back, and
//! - the artwork cache keeps the chosen bytes when a refresh asks it to cache another picture
//!   for the slot (`ArtworkService::cache_image`).
//!
//! Unlocking is the entity's own generated update of `lockedFields`.

pub mod options;
pub mod upload;

use std::any::Any;

use anyhow::{Context, Result};
use graphql_orm::graphql::filters::StringFilter;
use tracing::info;

use crate::db::Database;
use crate::graphql::AuthUser;
use crate::graphql::entities::{
    Album, Artist, ArtworkCache, ArtworkCacheWhereInput, Audiobook, Episode, Movie, PersonalList,
    Season, Show, UpdateAlbumInput, UpdateArtistInput, UpdateAudiobookInput, UpdateEpisodeInput,
    UpdateMovieInput, UpdatePersonalListInput, UpdateSeasonInput, UpdateShowInput, UpdateUserInput,
    User,
};
use crate::services::ArtworkService;

pub use options::{ArtworkOption, ProviderOptions};
pub use upload::{PreparedUpload, UploadError, prepare_upload};

/// What a picture belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, async_graphql::Enum)]
pub enum ArtworkEntityKind {
    Movie,
    Show,
    Season,
    Episode,
    Album,
    Artist,
    Audiobook,
    Playlist,
    /// A person's photo (design.md Q147).
    User,
}

/// Which picture of it. An artist's picture is stored as its poster, an episode's still as its
/// thumbnail, the names the artwork cache already uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, async_graphql::Enum)]
pub enum ArtworkKind {
    Poster,
    Backdrop,
    Logo,
    Thumbnail,
    Cover,
    Avatar,
}

/// One picture of one kind of entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArtworkSlot {
    pub entity: ArtworkEntityKind,
    pub kind: ArtworkKind,
}

/// Every slot there is, in the order a picker shows its tabs.
pub const SLOTS: &[ArtworkSlot] = &[
    ArtworkSlot::of(ArtworkEntityKind::Movie, ArtworkKind::Poster),
    ArtworkSlot::of(ArtworkEntityKind::Movie, ArtworkKind::Backdrop),
    ArtworkSlot::of(ArtworkEntityKind::Movie, ArtworkKind::Logo),
    ArtworkSlot::of(ArtworkEntityKind::Show, ArtworkKind::Poster),
    ArtworkSlot::of(ArtworkEntityKind::Show, ArtworkKind::Backdrop),
    ArtworkSlot::of(ArtworkEntityKind::Season, ArtworkKind::Poster),
    ArtworkSlot::of(ArtworkEntityKind::Episode, ArtworkKind::Thumbnail),
    ArtworkSlot::of(ArtworkEntityKind::Album, ArtworkKind::Cover),
    ArtworkSlot::of(ArtworkEntityKind::Artist, ArtworkKind::Poster),
    ArtworkSlot::of(ArtworkEntityKind::Audiobook, ArtworkKind::Cover),
    ArtworkSlot::of(ArtworkEntityKind::Playlist, ArtworkKind::Cover),
    ArtworkSlot::of(ArtworkEntityKind::User, ArtworkKind::Avatar),
];

/// What an uploaded picture's source is recorded as: a marker, never a URL, so nothing tries to
/// download it again and a provider URL can never equal it.
pub const UPLOAD_SOURCE_PREFIX: &str = "upload:";

impl ArtworkSlot {
    pub const fn of(entity: ArtworkEntityKind, kind: ArtworkKind) -> Self {
        Self { entity, kind }
    }

    /// The slot, when this kind of entity has a picture of this kind.
    pub fn new(entity: ArtworkEntityKind, kind: ArtworkKind) -> Option<Self> {
        SLOTS
            .iter()
            .copied()
            .find(|slot| slot.entity == entity && slot.kind == kind)
    }

    /// The slot an `/api/artwork/{entity}/{id}/{kind}` path names.
    pub fn from_path(entity_type: &str, artwork_type: &str) -> Option<Self> {
        SLOTS.iter().copied().find(|slot| {
            slot.entity_type() == entity_type.to_ascii_lowercase()
                && slot.artwork_type() == artwork_type.to_ascii_lowercase()
        })
    }

    /// The artwork cache's entity type.
    pub fn entity_type(self) -> &'static str {
        match self.entity {
            ArtworkEntityKind::Movie => "movie",
            ArtworkEntityKind::Show => "show",
            ArtworkEntityKind::Season => "season",
            ArtworkEntityKind::Episode => "episode",
            ArtworkEntityKind::Album => "album",
            ArtworkEntityKind::Artist => "artist",
            ArtworkEntityKind::Audiobook => "audiobook",
            ArtworkEntityKind::Playlist => "playlist",
            ArtworkEntityKind::User => "user",
        }
    }

    /// The artwork cache's artwork type.
    pub fn artwork_type(self) -> &'static str {
        match self.kind {
            ArtworkKind::Poster => "poster",
            ArtworkKind::Backdrop => "backdrop",
            ArtworkKind::Logo => "logo",
            ArtworkKind::Thumbnail => "thumbnail",
            ArtworkKind::Cover => "cover",
            ArtworkKind::Avatar => "avatar",
        }
    }

    /// The entity field that holds the slot's source, by its GraphQL name, which is also its lock
    /// name. A movie's logo has no field: nothing but a choice ever fills it, so the artwork
    /// cache's row is the whole record of it.
    pub fn field(self) -> Option<&'static str> {
        match (self.entity, self.kind) {
            (ArtworkEntityKind::Movie | ArtworkEntityKind::Show, ArtworkKind::Poster)
            | (ArtworkEntityKind::Season, ArtworkKind::Poster) => Some("posterUrl"),
            (ArtworkEntityKind::Movie | ArtworkEntityKind::Show, ArtworkKind::Backdrop) => {
                Some("backdropUrl")
            }
            (ArtworkEntityKind::Episode, ArtworkKind::Thumbnail) => Some("stillUrl"),
            (
                ArtworkEntityKind::Album
                | ArtworkEntityKind::Audiobook
                | ArtworkEntityKind::Playlist,
                ArtworkKind::Cover,
            ) => Some("coverUrl"),
            (ArtworkEntityKind::Artist, ArtworkKind::Poster) => Some("imageUrl"),
            (ArtworkEntityKind::User, ArtworkKind::Avatar) => Some("avatarUrl"),
            _ => None,
        }
    }

    /// Whether a provider lists pictures for this slot. A playlist's cover and a person's photo
    /// are only ever uploaded.
    pub fn has_provider(self) -> bool {
        !matches!(
            self.entity,
            ArtworkEntityKind::Playlist | ArtworkEntityKind::User
        )
    }

    /// Whether a metadata refresh can write this slot, so a choice needs a lock to stick.
    pub fn refreshable(self) -> bool {
        self.has_provider() && self.field().is_some()
    }
}

/// Artwork field names, by entity, that a lock protects.
fn artwork_fields(entity_name: &str) -> &'static [&'static str] {
    match entity_name {
        "Movie" | "Show" => &["posterUrl", "backdropUrl"],
        "Season" => &["posterUrl"],
        "Episode" => &["stillUrl"],
        "Album" | "Audiobook" => &["coverUrl"],
        "Artist" => &["imageUrl"],
        _ => &[],
    }
}

/// Whether a field name is an artwork lock (the web keeps these when it saves other locks).
pub fn is_artwork_lock(field: &str) -> bool {
    matches!(
        field,
        "posterUrl" | "backdropUrl" | "stillUrl" | "coverUrl" | "imageUrl"
    )
}

/// The write transform's half of a lock: an update that writes a locked artwork field without
/// also writing `lockedFields` is a refresh or a backfill, not someone choosing, so the field is
/// left out of it. Choosing and unlocking write `lockedFields` in the same update and pass.
pub fn keep_locked_artwork(
    entity_name: &str,
    existing_row: Option<&(dyn Any + Send + Sync)>,
    input: &mut (dyn Any + Send + Sync),
) {
    if artwork_fields(entity_name).is_empty() {
        return;
    }
    macro_rules! keep {
        ($entity:ty, $input:ty, $( $name:literal => $field:ident ),+ ) => {{
            let Some(existing) = existing_row.and_then(|row| row.downcast_ref::<$entity>()) else {
                return;
            };
            let Some(input) = input.downcast_mut::<$input>() else {
                return;
            };
            if input.locked_fields.is_some() {
                return;
            }
            let locked = existing.locked_fields.as_deref().unwrap_or_default();
            $(
                if input.$field.is_some() && locked.iter().any(|name| name == $name) {
                    input.$field = None;
                }
            )+
        }};
    }
    match entity_name {
        "Movie" => {
            keep!(Movie, UpdateMovieInput, "posterUrl" => poster_url, "backdropUrl" => backdrop_url)
        }
        "Show" => {
            keep!(Show, UpdateShowInput, "posterUrl" => poster_url, "backdropUrl" => backdrop_url)
        }
        "Season" => keep!(Season, UpdateSeasonInput, "posterUrl" => poster_url),
        "Episode" => keep!(Episode, UpdateEpisodeInput, "stillUrl" => still_url),
        "Album" => keep!(Album, UpdateAlbumInput, "coverUrl" => cover_url),
        "Artist" => keep!(Artist, UpdateArtistInput, "imageUrl" => image_url),
        "Audiobook" => keep!(Audiobook, UpdateAudiobookInput, "coverUrl" => cover_url),
        _ => {}
    }
}

/// A slot's record on its entity.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SlotState {
    /// The entity's owner (a playlist's owner may change its cover).
    pub owner_id: String,
    /// What the slot's field holds: a provider URL, an upload marker, or nothing.
    pub source: Option<String>,
    /// The entity's locks, all of them.
    pub locked_fields: Vec<String>,
}

impl SlotState {
    pub fn locked(&self, slot: ArtworkSlot) -> bool {
        slot.field()
            .is_some_and(|field| self.locked_fields.iter().any(|name| name == field))
    }
}

/// Read a slot's entity. `None` when the entity does not exist.
pub async fn slot_state(db: &Database, slot: ArtworkSlot, id: &str) -> Result<Option<SlotState>> {
    let pool = db.pool();
    let id = id.to_string();
    let state = |owner_id: String, source: Option<String>, locks: Option<Vec<String>>| SlotState {
        owner_id,
        source: source.filter(|source| !source.trim().is_empty()),
        locked_fields: locks.unwrap_or_default(),
    };
    Ok(match (slot.entity, slot.kind) {
        (ArtworkEntityKind::Movie, kind) => Movie::get(pool, &id).await?.map(|movie| {
            let source = match kind {
                ArtworkKind::Poster => movie.poster_url,
                ArtworkKind::Backdrop => movie.backdrop_url,
                _ => None,
            };
            state(movie.user_id, source, movie.locked_fields)
        }),
        (ArtworkEntityKind::Show, kind) => Show::get(pool, &id).await?.map(|show| {
            let source = if kind == ArtworkKind::Backdrop {
                show.backdrop_url
            } else {
                show.poster_url
            };
            state(show.user_id, source, show.locked_fields)
        }),
        (ArtworkEntityKind::Season, _) => match Season::get(pool, &id).await? {
            Some(season) => Show::get(pool, &season.show_id)
                .await?
                .map(|show| state(show.user_id, season.poster_url, season.locked_fields)),
            None => None,
        },
        (ArtworkEntityKind::Episode, _) => match Episode::get(pool, &id).await? {
            Some(episode) => Show::get(pool, &episode.show_id)
                .await?
                .map(|show| state(show.user_id, episode.still_url, episode.locked_fields)),
            None => None,
        },
        (ArtworkEntityKind::Album, _) => Album::get(pool, &id)
            .await?
            .map(|album| state(album.user_id, album.cover_url, album.locked_fields)),
        (ArtworkEntityKind::Artist, _) => Artist::get(pool, &id)
            .await?
            .map(|artist| state(artist.user_id, artist.image_url, artist.locked_fields)),
        (ArtworkEntityKind::Audiobook, _) => Audiobook::get(pool, &id)
            .await?
            .map(|book| state(book.user_id, book.cover_url, book.locked_fields)),
        (ArtworkEntityKind::Playlist, _) => PersonalList::get(pool, &id)
            .await?
            .map(|list| state(list.user_id, list.cover_url, None)),
        (ArtworkEntityKind::User, _) => User::get(pool, &id)
            .await?
            .map(|user| state(user.id, user.avatar_url, None)),
    })
}

/// Who may change a slot: an admin, as for every other metadata edit; a playlist's owner for its
/// cover, since a playlist is personal; a person for their own photo.
pub fn may_change(user: &AuthUser, slot: ArtworkSlot, state: &SlotState) -> bool {
    user.is_admin()
        || (matches!(
            slot.entity,
            ArtworkEntityKind::Playlist | ArtworkEntityKind::User
        ) && state.owner_id == user.user_id)
}

/// The locks after a slot is locked or unlocked.
fn relocked(current: &[String], field: &str, lock: bool) -> Vec<String> {
    let mut next = current
        .iter()
        .filter(|name| name.as_str() != field)
        .cloned()
        .collect::<Vec<_>>();
    if lock {
        next.push(field.to_string());
    }
    next
}

/// Write a slot's field and its lock in one update, through the entity layer (its hooks and
/// change events run). Writing `lockedFields` too is what lets the write past a lock.
pub async fn write_slot(
    db: &Database,
    slot: ArtworkSlot,
    id: &str,
    source: Option<String>,
    lock: bool,
) -> Result<()> {
    let Some(field) = slot.field() else {
        return Ok(());
    };
    let state = slot_state(db, slot, id)
        .await?
        .with_context(|| format!("{} {id} not found", slot.entity_type()))?;
    let locks = Some(Some(relocked(&state.locked_fields, field, lock)));
    let value = Some(source);
    let id = id.to_string();
    let written = match (slot.entity, slot.kind) {
        (ArtworkEntityKind::Movie, kind) => {
            let mut input = UpdateMovieInput {
                locked_fields: locks,
                ..Default::default()
            };
            if kind == ArtworkKind::Backdrop {
                input.backdrop_url = value;
            } else {
                input.poster_url = value;
            }
            Movie::update_by_id(db, &id, input).await?.is_some()
        }
        (ArtworkEntityKind::Show, kind) => {
            let mut input = UpdateShowInput {
                locked_fields: locks,
                ..Default::default()
            };
            if kind == ArtworkKind::Backdrop {
                input.backdrop_url = value;
            } else {
                input.poster_url = value;
            }
            Show::update_by_id(db, &id, input).await?.is_some()
        }
        (ArtworkEntityKind::Season, _) => Season::update_by_id(
            db,
            &id,
            UpdateSeasonInput {
                poster_url: value,
                locked_fields: locks,
                ..Default::default()
            },
        )
        .await?
        .is_some(),
        (ArtworkEntityKind::Episode, _) => Episode::update_by_id(
            db,
            &id,
            UpdateEpisodeInput {
                still_url: value,
                locked_fields: locks,
                ..Default::default()
            },
        )
        .await?
        .is_some(),
        (ArtworkEntityKind::Album, _) => Album::update_by_id(
            db,
            &id,
            UpdateAlbumInput {
                cover_url: value,
                locked_fields: locks,
                ..Default::default()
            },
        )
        .await?
        .is_some(),
        (ArtworkEntityKind::Artist, _) => Artist::update_by_id(
            db,
            &id,
            UpdateArtistInput {
                image_url: value,
                locked_fields: locks,
                ..Default::default()
            },
        )
        .await?
        .is_some(),
        (ArtworkEntityKind::Audiobook, _) => Audiobook::update_by_id(
            db,
            &id,
            UpdateAudiobookInput {
                cover_url: value,
                locked_fields: locks,
                ..Default::default()
            },
        )
        .await?
        .is_some(),
        // Nothing refreshes a playlist, so its cover needs no lock.
        (ArtworkEntityKind::Playlist, _) => PersonalList::update_by_id(
            db,
            &id,
            UpdatePersonalListInput {
                cover_url: value,
                ..Default::default()
            },
        )
        .await?
        .is_some(),
        // Nor a person's photo.
        (ArtworkEntityKind::User, _) => User::update_by_id(
            db,
            &id,
            UpdateUserInput {
                avatar_url: value,
                ..Default::default()
            },
        )
        .await?
        .is_some(),
    };
    anyhow::ensure!(written, "{} {id} not found", slot.entity_type());
    Ok(())
}

/// Whether the artwork cache holds an original for the slot.
pub async fn cached_original(
    db: &Database,
    slot: ArtworkSlot,
    id: &str,
) -> Result<Option<ArtworkCache>> {
    let string = |value: &str| {
        Some(StringFilter {
            eq: Some(value.to_string()),
            ..Default::default()
        })
    };
    Ok(ArtworkCache::query(db.pool())
        .filter(ArtworkCacheWhereInput {
            entity_type: string(slot.entity_type()),
            entity_id: string(id),
            artwork_type: string(slot.artwork_type()),
            ..Default::default()
        })
        .limit(1)
        .fetch_all()
        .await?
        .into_iter()
        .next())
}

/// What a slot shows, for the picker: its source, whether it is locked, whether it was uploaded.
#[derive(Debug, Clone, async_graphql::SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct ArtworkSlotState {
    pub entity_type: ArtworkEntityKind,
    pub entity_id: String,
    pub kind: ArtworkKind,
    /// Something is shown: a cached picture, or a source the cache will download on first use.
    pub has_artwork: bool,
    /// A metadata refresh leaves the picture alone.
    pub locked: bool,
    /// Someone uploaded the picture.
    pub uploaded: bool,
    /// A provider lists pictures for this slot (a playlist's cover is only ever uploaded).
    pub has_provider: bool,
    /// All of the entity's locks, for an unlock through its generated update.
    pub locked_fields: Vec<String>,
}

/// The state of one slot, without asking any provider.
pub async fn describe_slot(
    db: &Database,
    slot: ArtworkSlot,
    id: &str,
    state: &SlotState,
) -> Result<ArtworkSlotState> {
    let cached = cached_original(db, slot, id).await?;
    let source = match slot.field() {
        Some(_) => state.source.clone(),
        None => cached.as_ref().and_then(|row| row.source_url.clone()),
    };
    Ok(ArtworkSlotState {
        entity_type: slot.entity,
        entity_id: id.to_string(),
        kind: slot.kind,
        has_artwork: cached.is_some() || source.is_some(),
        // A movie's logo is only ever chosen; nothing refreshes it.
        locked: state.locked(slot) || (slot.field().is_none() && cached.is_some()),
        uploaded: source
            .as_deref()
            .is_some_and(|source| source.starts_with(UPLOAD_SOURCE_PREFIX)),
        has_provider: slot.has_provider(),
        locked_fields: state.locked_fields.clone(),
    })
}

/// The slots one entity has.
pub fn slots_of(entity: ArtworkEntityKind) -> Vec<ArtworkSlot> {
    SLOTS
        .iter()
        .copied()
        .filter(|slot| slot.entity == entity)
        .collect()
}

/// Fill a slot with a provider picture and lock it. The picture must be one the provider lists
/// for it, so a choice can only ever be something the picker showed.
pub async fn choose(
    db: &Database,
    artwork: &ArtworkService,
    providers: &ProviderOptions,
    slot: ArtworkSlot,
    id: &str,
    url: &str,
) -> Result<()> {
    anyhow::ensure!(
        providers
            .options
            .iter()
            .any(|option| options::same_image(&option.url, url)),
        "That picture isn't one the provider lists for this {}",
        slot.entity_type()
    );
    artwork
        .replace_from_url(url, slot.entity_type(), id, slot.artwork_type())
        .await?;
    write_slot(db, slot, id, Some(url.to_string()), true).await?;
    info!(
        entity_type = slot.entity_type(),
        entity_id = %id,
        artwork_type = slot.artwork_type(),
        source = %url,
        "Chose and locked {} {} artwork from the provider's list",
        slot.entity_type(),
        slot.artwork_type()
    );
    Ok(())
}

/// Fill a slot with an uploaded picture and lock it. Returns the stored picture's size.
pub async fn upload(
    db: &Database,
    artwork: &ArtworkService,
    slot: ArtworkSlot,
    id: &str,
    bytes: &[u8],
) -> Result<(i32, i32), UploadFailure> {
    let prepared = prepare_upload(bytes).map_err(UploadFailure::Invalid)?;
    let marker = format!(
        "{UPLOAD_SOURCE_PREFIX}{}",
        &prepared.sha256_hex[..16.min(prepared.sha256_hex.len())]
    );
    let size = (prepared.width, prepared.height);
    artwork
        .store_upload(
            slot.entity_type(),
            id,
            slot.artwork_type(),
            prepared,
            &marker,
        )
        .await
        .map_err(UploadFailure::Failed)?;
    write_slot(db, slot, id, Some(marker), true)
        .await
        .map_err(UploadFailure::Failed)?;
    info!(
        entity_type = slot.entity_type(),
        entity_id = %id,
        artwork_type = slot.artwork_type(),
        width = size.0,
        height = size.1,
        "Stored and locked an uploaded {} {} picture",
        slot.entity_type(),
        slot.artwork_type()
    );
    Ok(size)
}

/// Why an upload was not stored.
#[derive(Debug)]
pub enum UploadFailure {
    /// The file itself: the person can fix it.
    Invalid(UploadError),
    /// Storage or the database.
    Failed(anyhow::Error),
}

/// Unlock a slot and put back what a metadata refresh would choose. With nothing to go back to
/// (a playlist, a movie's logo, a provider without a picture) the slot is emptied. When the
/// provider couldn't be asked nothing changes, so a choice is never lost to an outage.
pub async fn revert(
    db: &Database,
    artwork: &ArtworkService,
    providers: Option<&ProviderOptions>,
    slot: ArtworkSlot,
    id: &str,
) -> Result<()> {
    let default = providers.and_then(|list| list.options.iter().find(|option| option.is_default));
    if let Some(default) = default {
        artwork
            .replace_from_url(&default.url, slot.entity_type(), id, slot.artwork_type())
            .await?;
        write_slot(db, slot, id, Some(default.url.clone()), false).await?;
    } else {
        if let Some(list) = providers
            && !list.unavailable.is_empty()
        {
            anyhow::bail!(
                "Couldn't reach {}, so nothing was changed. Try again later.",
                list.unavailable.join(", ")
            );
        }
        artwork
            .remove_artwork(slot.entity_type(), id, slot.artwork_type())
            .await?;
        write_slot(db, slot, id, None, false).await?;
    }
    info!(
        entity_type = slot.entity_type(),
        entity_id = %id,
        artwork_type = slot.artwork_type(),
        restored = default.is_some(),
        "Unlocked {} {} artwork and put back the provider's choice",
        slot.entity_type(),
        slot.artwork_type()
    );
    Ok(())
}

#[cfg(test)]
mod tests;
