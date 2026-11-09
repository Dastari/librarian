//! What each picture of a title shows, and the provider pictures it could show instead (the
//! artwork picker; design notes Q126–Q128).

use std::sync::Arc;

use async_graphql::{Context, ErrorExtensions, Object, Result, SimpleObject};

use crate::db::Database;
use crate::graphql::auth::AuthExt;
use crate::services::artwork_choice::{
    self, ArtworkEntityKind, ArtworkKind, ArtworkOption, ArtworkSlot, ArtworkSlotState,
};
use crate::services::metadata::providers::MetadataService;

/// A slot and the pictures its providers list for it.
#[derive(SimpleObject)]
#[graphql(rename_fields = "camelCase")]
pub struct ArtworkOptions {
    pub slot: ArtworkSlotState,
    /// The default (what a refresh would choose) first, then the providers' own order.
    pub options: Vec<ArtworkOption>,
    /// Providers that couldn't be asked, or aren't set up, by name.
    pub unavailable: Vec<String>,
}

fn not_found() -> async_graphql::Error {
    async_graphql::Error::new("Not found")
        .extend_with(|_, extensions| extensions.set("code", "NOT_FOUND"))
}

fn slot_of(entity: ArtworkEntityKind, kind: ArtworkKind) -> Result<ArtworkSlot> {
    ArtworkSlot::new(entity, kind).ok_or_else(|| {
        async_graphql::Error::new("That kind of title has no picture of that kind")
            .extend_with(|_, extensions| extensions.set("code", "BAD_REQUEST"))
    })
}

#[derive(Default)]
pub struct ArtworkQueries;

#[Object]
impl ArtworkQueries {
    /// Every picture a title has and what each shows, without asking any provider. Readable by
    /// the title's owner and administrators.
    #[graphql(name = "artworkSlots")]
    async fn artwork_slots(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "entityType")] entity_type: ArtworkEntityKind,
        #[graphql(name = "entityId")] entity_id: String,
    ) -> Result<Vec<ArtworkSlotState>> {
        let user = ctx.require_member()?;
        let db = ctx.data_unchecked::<Database>();
        let mut slots = Vec::new();
        for slot in artwork_choice::slots_of(entity_type) {
            let state = artwork_choice::slot_state(db, slot, &entity_id)
                .await?
                .ok_or_else(not_found)?;
            if !user.is_admin() && state.owner_id != user.user_id {
                return Err(not_found());
            }
            slots.push(artwork_choice::describe_slot(db, slot, &entity_id, &state).await?);
        }
        Ok(slots)
    }

    /// The pictures a title's providers list for one of its pictures, asked for when the picker
    /// opens and kept for a few minutes. For whoever may change it.
    #[graphql(name = "artworkOptions")]
    async fn artwork_options(
        &self,
        ctx: &Context<'_>,
        #[graphql(name = "entityType")] entity_type: ArtworkEntityKind,
        #[graphql(name = "entityId")] entity_id: String,
        kind: ArtworkKind,
    ) -> Result<ArtworkOptions> {
        let user = ctx.require_member()?;
        let slot = slot_of(entity_type, kind)?;
        let db = ctx.data_unchecked::<Database>();
        let state = artwork_choice::slot_state(db, slot, &entity_id)
            .await?
            .ok_or_else(not_found)?;
        if !artwork_choice::may_change(user, slot, &state) {
            return Err(not_found());
        }
        let described = artwork_choice::describe_slot(db, slot, &entity_id, &state).await?;
        let providers = if slot.has_provider() {
            let metadata = ctx.data_unchecked::<Arc<MetadataService>>();
            artwork_choice::options::provider_options(metadata, db, slot, &entity_id).await?
        } else {
            Default::default()
        };
        let current = match slot.field() {
            Some(_) => state.source.clone(),
            None => artwork_choice::cached_original(db, slot, &entity_id)
                .await?
                .and_then(|row| row.source_url),
        };
        let providers = providers.with_current(current.as_deref());
        Ok(ArtworkOptions {
            slot: described,
            options: providers.options,
            unavailable: providers.unavailable,
        })
    }
}
