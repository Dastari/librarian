//! Who may read whose statistics (design.md Q136).
//!
//! Your own, always. Administrators: everyone's. Anyone else's only while that person shares
//! them with you or with everyone, and while both accounts are active. Shared statistics are
//! read-only and never include the raw history rows, devices' addresses or user agents.

use std::collections::{BTreeMap, HashMap};

use graphql_orm::graphql::filters::StringFilter;

use crate::db::Database;
use crate::graphql::AuthUser;
use crate::graphql::entities::*;
use crate::services::graphql::auth::Role;

use super::{AUDIENCE_EVERYONE, AUDIENCE_USER, user_names};

fn eq(value: &str) -> StringFilter {
    StringFilter {
        eq: Some(value.to_owned()),
        ..Default::default()
    }
}

/// The shares `owner` has given, all of them (a person has a handful at most).
async fn shares_of(db: &Database, owner: Option<&str>) -> anyhow::Result<Vec<StatsShare>> {
    let mut rows = Vec::new();
    loop {
        let page = StatsShare::query(db.pool())
            .filter(StatsShareWhereInput {
                user_id: owner.map(eq),
                ..Default::default()
            })
            .limit(100)
            .offset(rows.len() as i64)
            .fetch_all()
            .await?;
        let len = page.len();
        rows.extend(page);
        if len < 100 {
            return Ok(rows);
        }
    }
}

pub fn share_grants(share: &StatsShare, viewer_user_id: &str) -> bool {
    share.user_id != viewer_user_id
        && (share.audience == AUDIENCE_EVERYONE
            || (share.audience == AUDIENCE_USER
                && share.viewer_user_id.as_deref() == Some(viewer_user_id)))
}

/// Whether `viewer` may read `target`'s statistics.
pub async fn can_view(db: &Database, viewer: &AuthUser, target: &str) -> anyhow::Result<bool> {
    if !viewer.has_role(Role::Member) {
        return Ok(false);
    }
    if viewer.user_id == target || viewer.is_admin() {
        return Ok(true);
    }
    let active = |id: &str| {
        let id = id.to_owned();
        async move {
            anyhow::Ok(
                User::get(db.pool(), &id)
                    .await?
                    .is_some_and(|user| user.is_active),
            )
        }
    };
    if !active(target).await? || !active(&viewer.user_id).await? {
        return Ok(false);
    }
    Ok(shares_of(db, Some(target))
        .await?
        .iter()
        .any(|share| share_grants(share, &viewer.user_id)))
}

/// A person whose statistics the viewer can open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Viewable {
    pub user_id: String,
    pub name: String,
    pub is_self: bool,
    /// Visible through a share (not just because the viewer is an administrator).
    pub shared: bool,
}

/// Everyone the viewer can open: themselves first, then by name.
pub async fn viewable_users(db: &Database, viewer: &AuthUser) -> anyhow::Result<Vec<Viewable>> {
    let names = user_names(db).await?;
    let mut shared: BTreeMap<String, bool> = BTreeMap::new();
    for share in shares_of(db, None).await? {
        if share_grants(&share, &viewer.user_id) {
            shared.insert(share.user_id.clone(), true);
        }
    }
    let mut people: Vec<Viewable> = names
        .iter()
        .filter(|(id, (_, active))| {
            *id == &viewer.user_id || (*active && (viewer.is_admin() || shared.contains_key(*id)))
        })
        .map(|(id, (name, _))| Viewable {
            user_id: id.clone(),
            name: name.clone(),
            is_self: id == &viewer.user_id,
            shared: shared.contains_key(id),
        })
        .collect();
    people.sort_by(|a, b| {
        b.is_self
            .cmp(&a.is_self)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(people)
}

/// Active users the viewer can share with: everyone but themselves.
pub async fn share_candidates(
    db: &Database,
    viewer: &AuthUser,
) -> anyhow::Result<Vec<(String, String)>> {
    let names: HashMap<String, (String, bool)> = user_names(db).await?;
    let mut people: Vec<(String, String)> = names
        .into_iter()
        .filter(|(id, (_, active))| *active && id != &viewer.user_id)
        .map(|(id, (name, _))| (id, name))
        .collect();
    people.sort_by_key(|a| a.1.to_lowercase());
    Ok(people)
}
