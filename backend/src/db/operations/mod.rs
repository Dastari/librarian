//! Database operations module
//!
//! This module contains complex database operations that cannot be expressed
//! using the entity query system:
//! - Aggregate queries (COUNT, SUM, AVG with GROUP BY)
//! - Cross-table JOINs
//! - Batch operations with transactions
//! - Cleanup/maintenance operations
//!
//! For simple CRUD and filtered queries, use the entity query system instead.
//!
//! Split by concern (DB only; GraphQL types live in services/graphql):
//! - [app_log]: app log batch insert
//! - [user_auth]: admin check, user create, password/last_login update
//! - [refresh_tokens]: token create/delete/update/cleanup
//! - [user_library_access]: grant/revoke/check access

pub mod app_log;
pub mod refresh_tokens;
pub mod user_auth;
pub mod user_library_access;

pub use app_log::insert_app_logs_batch;
pub use refresh_tokens::{
    cleanup_expired_refresh_tokens,
    create_refresh_token,
    delete_refresh_token,
    delete_user_refresh_tokens,
    update_refresh_token_used,
};
pub use user_auth::{
    create_user,
    has_admin_user,
    update_user_last_login,
    update_user_password,
    CreateUserParams,
};
pub use user_library_access::{
    grant_library_access,
    has_library_access,
    revoke_library_access,
};
